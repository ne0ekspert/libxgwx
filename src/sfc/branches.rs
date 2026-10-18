use super::*;

pub(super) const MAX_ROWS: u32 = 65_535;
pub(super) const MAX_COLUMNS: u32 = 65_535;
pub(super) const MAX_GRID_CELLS: u32 = 1_048_576;

#[cfg(test)]
std::thread_local! {
    static CELL_LOOKUPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn cell<'a>(cells: &BTreeMap<(u32, u32), &'a SfcRow>, row: u32, column: u32) -> Option<&'a SfcRow> {
    #[cfg(test)]
    CELL_LOOKUPS.with(|count| count.set(count.get() + 1));
    cells.get(&(row, column)).copied()
}

fn branch_kind(row: &SfcRow) -> Option<(u32, bool)> {
    Some(match row.kind.as_str() {
        "alternative_split" => (0, false),
        "alternative_join" => (0, true),
        "parallel_split" => (2, false),
        "parallel_join" => (2, true),
        _ => return None,
    })
}

// Captured rectangular branches with independently padded paths. Reject gaps, crossing/nested ranges,
// custom priorities and extra native records instead of guessing their topology.
pub(super) fn validate_layout(rows: &[SfcRow]) -> Result<(), crate::XgwxError> {
    let fail = || {
        crate::XgwxError::SfcEdit("branches require balanced, nonoverlapping paths with alternating steps and transitions".into())
    };
    let mut previous = BTreeMap::<u32, (u32, String)>::new();
    for (i, row) in rows.iter().enumerate() {
        let column = row.position.as_ref().map_or(0, |p| p.column);
        let index = row.position.as_ref().map_or(i as u32, |p| p.row);
        if row.kind == "continuation" {
            let (last_row, owner) = previous.get_mut(&column).ok_or_else(fail)?;
            if *last_row + 1 != index
                || !matches!(owner.as_str(), "step" | "transition" | "label")
                || row.action.is_some() && owner != "step"
            {
                return Err(fail());
            }
            *last_row = index;
        } else {
            previous.insert(column, (index, row.kind.clone()));
        }
    }
    if rows.iter().all(|r| r.position.is_none()) {
        return if rows.len() > MAX_ROWS as usize
            || rows
                .iter()
                .any(|r| r.branch_end.is_some() || branch_kind(r).is_some())
        {
            Err(fail())
        } else {
            Ok(())
        };
    }
    let height = rows
        .iter()
        .filter_map(|r| r.position.as_ref().and_then(|p| p.row.checked_add(1)))
        .max()
        .unwrap_or(0);
    let width = rows
        .iter()
        .filter_map(|r| {
            r.position
                .as_ref()
                .and_then(|p| r.branch_end.unwrap_or(p.column).checked_add(2))
        })
        .max()
        .unwrap_or(0);
    if height
        .checked_mul(width)
        .is_none_or(|cells| cells > MAX_GRID_CELLS)
    {
        return Err(crate::XgwxError::SfcEdit(
            "chart exceeds the editor safety limit of 1048576 grid cells".into(),
        ));
    }
    let mut cells = BTreeMap::new();
    let mut regions = Vec::new();
    let mut open: Option<(&SfcRow, u32)> = None;
    for row in rows {
        let p = row.position.as_ref().ok_or_else(fail)?;
        if p.row >= MAX_ROWS
            || p.column > MAX_COLUMNS - 2
            || p.column % 2 != 0
            || cells.insert((p.row, p.column), row).is_some()
        {
            return Err(fail());
        }
        if let Some((kind, join)) = branch_kind(row) {
            let end = row.branch_end.ok_or_else(fail)?;
            if p.column != 0 || end < 2 || end > MAX_COLUMNS - 2 || end % 2 != 0 {
                return Err(fail());
            }
            if join {
                let (start, start_kind) = open.take().ok_or_else(fail)?;
                if start_kind != kind
                    || start.branch_end != row.branch_end
                    || start.position.as_ref().unwrap().row + 1 >= p.row
                {
                    return Err(fail());
                }
                regions.push((start.position.as_ref().unwrap().row, p.row, end, kind));
            } else if open.replace((row, kind)).is_some() {
                return Err(fail());
            }
        } else if row.branch_end.is_some() {
            return Err(fail());
        }
    }
    if open.is_some() || regions.is_empty() {
        return Err(fail());
    }
    // Accepted branches cannot overlap: a competing split/join inside a
    // region would fail its step/transition checks. Index their interior rows
    // once rather than searching all regions for every cell.
    regions.sort_unstable_by_key(|region| region.0);
    if regions.windows(2).any(|pair| pair[0].1 >= pair[1].0) {
        return Err(fail());
    }
    let max_row = cells.keys().map(|(r, _)| *r).max().unwrap_or(0);
    let mut owners = vec![None; max_row as usize + 1];
    for (index, &(start, finish, _, _)) in regions.iter().enumerate() {
        for r in start + 1..finish {
            owners[r as usize] = Some(index);
        }
    }
    let mut active = None;
    let mut counts = Vec::new();
    // One main path continues through every row. Each branch lane maintains
    // parity as it advances; continuations do not change meaningful-node count.
    for r in 0..=max_row {
        cell(&cells, r, 0).ok_or_else(fail)?;
        if let Some(index) = owners[r as usize] {
            let (_, finish, end, kind) = regions[index];
            if active != Some(index) {
                counts.resize((end / 2 + 1) as usize, 0usize);
                counts.fill(0);
                active = Some(index);
            }
            for c in (0..=end).step_by(2) {
                let node = cell(&cells, r, c).ok_or_else(fail)?;
                let count = &mut counts[(c / 2) as usize];
                let expected = match (kind, *count % 2) {
                    (0, 0) | (2, 1) => "transition",
                    _ => "step",
                };
                if node.initial || node.kind != "continuation" && node.kind != expected {
                    return Err(fail());
                }
                *count += usize::from(node.kind != "continuation");
            }
            if r + 1 == finish && counts.iter().any(|count| count % 2 == 0) {
                return Err(fail());
            }
        }
    }
    // Neighbor selection historically follows input order, even if callers
    // interleave coordinate ranges. Prefix maxima and suffix minima of input
    // indices preserve the reverse/forward searches without rescanning rows.
    let mut main: Vec<_> = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            let p = row.position.as_ref()?;
            (p.column == 0 && row.kind != "continuation").then_some((p.row, index, row))
        })
        .collect();
    main.sort_unstable_by_key(|entry| entry.0);
    let mut before = vec![0usize; main.len()];
    let mut after = vec![0usize; main.len()];
    for i in 0..main.len() {
        before[i] = if i == 0 || main[i].1 > main[before[i - 1]].1 {
            i
        } else {
            before[i - 1]
        };
    }
    for i in (0..main.len()).rev() {
        after[i] = if i + 1 == main.len() || main[i].1 < main[after[i + 1]].1 {
            i
        } else {
            after[i + 1]
        };
    }
    for (start, finish, _, kind) in &regions {
        let left = main
            .partition_point(|entry| entry.0 < *start)
            .checked_sub(1)
            .ok_or_else(fail)?;
        let right = main.partition_point(|entry| entry.0 <= *finish);
        let next = *after.get(right).ok_or_else(fail)?;
        let expected = if *kind == 0 { "step" } else { "transition" };
        if main[before[left]].2.kind != expected || main[next].2.kind != expected {
            return Err(fail());
        }
    }
    for &(r, c) in cells.keys() {
        if c != 0 && owners[r as usize].is_none_or(|index| c > regions[index].2) {
            return Err(fail());
        }
    }
    Ok(())
}

fn props(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
fn blank(row: u32, column: u32, kind: u32) -> SfcEntity {
    SfcEntity {
        entity_index: 0,
        type_code: Some(kind),
        row: Some(row),
        column: Some(column),
        properties: BTreeMap::from([("EntityBasic".into(), props(&[("Virtural", "0")]))]),
    }
}
fn step(row: u32, column: u32, kind: u32, value: &SfcRow, adjacent: bool) -> SfcEntity {
    let mut e = blank(row, column, kind);
    let title = if kind == 2 {
        value.action.as_deref().unwrap()
    } else {
        &value.title
    };
    let program = if !adjacent {
        value.transition_code.is_some()
    } else {
        kind == 2 && value.action_code.is_some()
    };
    e.properties.insert(
        "EntityStep".into(),
        props(&[
            ("Title", title),
            ("InnerVariableName", ""),
            ("Comment", if !adjacent { &value.comment } else { "" }),
            ("Bookmark", "0"),
            ("BreakPoint", "0"),
            ("PropertyProgram", if program { "1" } else { "0" }),
            (
                "InitialStep",
                if !adjacent && value.initial { "1" } else { "0" },
            ),
            ("StepVariable", "0"),
        ]),
    );
    if kind == 2 {
        e.properties.insert(
            "EntityAction".into(),
            props(&[
                ("Time", value.action_time.as_deref().unwrap_or("")),
                (
                    "Qualifier",
                    &qualifier_code(value.action_qualifier.as_deref().unwrap_or("N"))
                        .unwrap()
                        .to_string(),
                ),
                ("FBInstanceName", ""),
                ("FBInstanceOut", ""),
                ("ResetVarName", ""),
                ("TimeIndex", "0"),
            ]),
        );
    }
    e
}
fn entities(rows: &[SfcRow]) -> Vec<SfcEntity> {
    let height = rows
        .iter()
        .map(|r| r.position.as_ref().unwrap().row + 1)
        .max()
        .unwrap_or(0);
    let width = rows
        .iter()
        .map(|r| r.branch_end.unwrap_or(r.position.as_ref().unwrap().column) + 2)
        .max()
        .unwrap_or(0);
    let mut cells: BTreeMap<_, _> = (0..width)
        .flat_map(|c| (0..height).map(move |r| ((r, c), blank(r, c, 10))))
        .collect();
    for value in rows {
        let p = value.position.as_ref().unwrap();
        if let Some((kind, join)) = branch_kind(value) {
            for c in p.column..=value.branch_end.unwrap() {
                let mut e = blank(p.row, c, if c % 2 == 0 { 4 } else { 8 });
                if c % 2 == 0 {
                    e.properties.insert(
                        "EntityBranch".into(),
                        props(&[
                            ("Priority", "-1"),
                            (
                                "BranchType",
                                match (kind, join) {
                                    (0, false) => "0",
                                    (0, true) => "1",
                                    (2, false) => "2",
                                    _ => "3",
                                },
                            ),
                        ]),
                    );
                }
                cells.insert((p.row, c), e);
            }
        } else {
            let kind = match value.kind.as_str() {
                "step" => 0,
                "transition" => 1,
                "jump" => 5,
                "continuation" => 7,
                _ => 6,
            };
            cells.insert(
                (p.row, p.column),
                if kind == 7 {
                    blank(p.row, p.column, 7)
                } else {
                    step(p.row, p.column, kind, value, false)
                },
            );
            let kind = if matches!(value.kind.as_str(), "step" | "continuation") {
                if value.action.is_some() { 2 } else { 10 }
            } else {
                9
            };
            cells.insert(
                (p.row, p.column + 1),
                if kind == 10 {
                    blank(p.row, p.column + 1, 10)
                } else {
                    step(p.row, p.column + 1, kind, value, true)
                },
            );
        }
    }
    let mut result: Vec<_> = cells.into_values().collect();
    result.sort_by_key(|e| (e.column, e.row));
    for (i, e) in result.iter_mut().enumerate() {
        e.entity_index = i;
    }
    result
}

pub(super) fn decode_rows(
    block: &SfcBlock,
    sources: &BTreeMap<String, (u32, String)>,
    bool_names: &[String],
) -> Option<Vec<SfcRow>> {
    if !block.main
        || block.rows > MAX_ROWS
        || block.columns < 4
        || block.columns > MAX_COLUMNS
        || block
            .rows
            .checked_mul(block.columns)
            .is_none_or(|cells| cells > MAX_GRID_CELLS)
        || block.columns % 2 != 0
        || block.entities.len() != (block.rows * block.columns) as usize
    {
        return None;
    }
    let by_position: BTreeMap<_, _> = block
        .entities
        .iter()
        .map(|e| Some(((e.row?, e.column?), e)))
        .collect::<Option<_>>()?;
    if by_position.len() != block.entities.len() {
        return None;
    }
    let mut ordinary: Vec<_> = block
        .entities
        .iter()
        .filter(|e| matches!(e.type_code, Some(0 | 1 | 5 | 6 | 7)))
        .cloned()
        .collect();
    ordinary.sort_by_key(|e| (e.column, e.row));
    let mut projected = block.clone();
    projected.rows = ordinary.len() as u32;
    projected.columns = 2;
    projected.entities.clear();
    for (i, node) in ordinary.iter().enumerate() {
        if node.column? % 2 != 0 {
            return None;
        }
        let mut other = (**by_position.get(&(node.row?, node.column?.checked_add(1)?))?).clone();
        let mut first = node.clone();
        first.row = Some(i as u32);
        first.column = Some(0);
        other.row = Some(i as u32);
        other.column = Some(1);
        projected.entities.extend([first, other]);
    }
    let mut rows = projected.linear_rows(sources, bool_names, true)?;
    for (row, node) in rows.iter_mut().zip(&ordinary) {
        row.position = Some(SfcPosition {
            row: node.row?,
            column: node.column?,
        });
    }
    let mut branch_rows: BTreeMap<u32, Vec<&SfcEntity>> = BTreeMap::new();
    for e in &block.entities {
        if matches!(e.type_code, Some(4)) {
            branch_rows.entry(e.row?).or_default().push(e);
        }
    }
    for (r, mut endpoints) in branch_rows {
        endpoints.sort_by_key(|e| e.column);
        let first = endpoints[0];
        let p = first.properties.get("EntityBranch")?;
        let kind = match (first.type_code?, p.get("BranchType")?.as_str()) {
            (4, "0") => "alternative_split",
            (4, "1") => "alternative_join",
            (4, "2") => "parallel_split",
            (4, "3") => "parallel_join",
            _ => return None,
        };
        rows.push(SfcRow {
            kind: kind.into(),
            title: String::new(),
            comment: String::new(),
            initial: false,
            action: None,
            action_qualifier: None,
            action_time: None,
            action_code: None,
            transition_code: None,
            position: Some(SfcPosition {
                row: r,
                column: first.column?,
            }),
            branch_end: Some(endpoints.last()?.column?),
        });
    }
    rows.sort_by_key(|r| {
        let p = r.position.as_ref().unwrap();
        (p.row, p.column)
    });
    validate_rows(&rows, bool_names, false).ok()?;
    let expected = entities(&rows);
    let mut actual = block.entities.clone();
    actual.sort_by_key(|e| (e.column, e.row));
    for (i, e) in actual.iter_mut().enumerate() {
        e.entity_index = i;
        if let Some(p) = e.properties.get_mut("EntityStep") {
            p.insert("InnerVariableName".into(), String::new());
        }
    }
    (expected == actual).then_some(rows)
}

#[cfg(feature = "write")]
pub(super) fn grid_xml(rows: &[SfcRow]) -> Result<String, crate::XgwxError> {
    validate_layout(rows)?;
    let list = entities(rows);
    let height = list.iter().map(|e| e.row.unwrap() + 1).max().unwrap_or(0);
    let width = list
        .iter()
        .map(|e| e.column.unwrap() + 1)
        .max()
        .unwrap_or(0);
    let escape = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\r', "&#xD;")
            .replace('\n', "&#xA;")
            .replace('\t', "&#x9;")
    };
    let mut xml =
        format!("<EntityGrid RowSize=\"{height}\" ColSize=\"{width}\"><EntityPropertyList>");
    for e in list {
        xml.push_str(&format!(
            "<EntityProperty Type=\"{}\" Col=\"{}\" Row=\"{}\">",
            e.type_code.unwrap(),
            e.column.unwrap(),
            e.row.unwrap()
        ));
        for (tag, attrs) in e.properties {
            xml.push_str(&format!("<{tag}"));
            for (key, value) in attrs {
                xml.push_str(&format!(" {key}=\"{}\"", escape(&value)));
            }
            xml.push_str("/>");
        }
        xml.push_str("</EntityProperty>");
    }
    xml.push_str("</EntityPropertyList></EntityGrid>");
    Ok(xml)
}

#[cfg(test)]
mod native_limit_tests {
    use super::*;
    fn row(kind: &str, y: u32, x: u32, end: Option<u32>) -> SfcRow {
        SfcRow {
            kind: kind.into(),
            title: String::new(),
            comment: String::new(),
            initial: false,
            action: None,
            action_code: None,
            transition_code: None,
            action_qualifier: None,
            action_time: None,
            position: Some(SfcPosition { row: y, column: x }),
            branch_end: end,
        }
    }
    fn padded_branch(height: u32) -> Vec<SfcRow> {
        let mut rows = vec![
            row("step", 0, 0, None),
            row("alternative_split", 1, 0, Some(2)),
        ];
        for y in 2..height + 2 {
            for x in [0, 2] {
                rows.push(row(
                    if y == 2 { "transition" } else { "continuation" },
                    y,
                    x,
                    None,
                ));
            }
        }
        rows.extend([
            row("alternative_join", height + 2, 0, Some(2)),
            row("step", height + 3, 0, None),
            row("transition", height + 4, 0, None),
        ]);
        rows
    }
    #[test]
    fn padded_branch_validation_uses_linear_cell_lookups() {
        let rows = padded_branch(512);
        CELL_LOOKUPS.with(|count| count.set(0));
        let start = std::time::Instant::now();
        validate_layout(&rows).unwrap();
        let elapsed = start.elapsed();
        let lookups = CELL_LOOKUPS.with(|count| count.get());
        eprintln!("512-row padded branch: {lookups} cell lookups in {elapsed:?}");
        assert!(
            lookups <= rows.len() * 4,
            "path prefixes and full heights must not be rescanned"
        );
    }
    #[test]
    fn largest_native_height_preserves_continuation_padding() {
        let mut rows = padded_branch(MAX_ROWS - 5);
        CELL_LOOKUPS.with(|count| count.set(0));
        validate_layout(&rows).unwrap();
        assert!(CELL_LOOKUPS.with(|count| count.get()) <= rows.len() * 4);
        rows.push(row("transition", MAX_ROWS, 0, None));
        assert!(validate_layout(&rows).is_err());
    }
    #[test]
    fn independently_padded_paths_keep_separate_parity() {
        let mut rows = padded_branch(5);
        let at = |rows: &[SfcRow], y| {
            rows.iter()
                .position(|r| r.position.as_ref().unwrap() == &SfcPosition { row: y, column: 2 })
                .unwrap()
        };
        let second = at(&rows, 3);
        let third = at(&rows, 4);
        rows[second].kind = "step".into();
        rows[third].kind = "transition".into();
        validate_layout(&rows).unwrap();
        rows[third].kind = "continuation".into();
        assert!(validate_layout(&rows).is_err());
    }
    #[test]
    fn main_neighbors_keep_original_input_order() {
        for rows in [
            vec![
                row("transition", 1, 0, None),
                row("step", 0, 0, None),
                row("alternative_split", 2, 0, Some(2)),
                row("transition", 3, 0, None),
                row("transition", 3, 2, None),
                row("alternative_join", 4, 0, Some(2)),
                row("step", 5, 0, None),
            ],
            vec![
                row("step", 0, 0, None),
                row("alternative_split", 1, 0, Some(2)),
                row("transition", 2, 0, None),
                row("transition", 2, 2, None),
                row("alternative_join", 3, 0, Some(2)),
                row("step", 5, 0, None),
                row("transition", 4, 0, None),
            ],
        ] {
            validate_layout(&rows).unwrap();
        }
    }
    #[test]
    fn many_sequential_regions_and_mixed_branch_kinds_remain_supported() {
        let mut rows = vec![row("step", 0, 0, None)];
        for region in 0..256 {
            let base = region * 4;
            rows.extend([
                row("alternative_split", base + 1, 0, Some(2)),
                row("transition", base + 2, 0, None),
                row("transition", base + 2, 2, None),
                row("alternative_join", base + 3, 0, Some(2)),
                row("step", base + 4, 0, None),
            ]);
        }
        validate_layout(&rows).unwrap();
        let rows = vec![
            row("step", 0, 0, None),
            row("alternative_split", 1, 0, Some(2)),
            row("transition", 2, 0, None),
            row("transition", 2, 2, None),
            row("alternative_join", 3, 0, Some(2)),
            row("step", 4, 0, None),
            row("transition", 5, 0, None),
            row("parallel_split", 6, 0, Some(2)),
            row("step", 7, 0, None),
            row("step", 7, 2, None),
            row("parallel_join", 8, 0, Some(2)),
            row("transition", 9, 0, None),
        ];
        validate_layout(&rows).unwrap();
    }
    #[test]
    fn malformed_padding_regions_coordinates_and_ownership_remain_errors() {
        let original = padded_branch(4);
        for case in 0..11 {
            let mut rows = original.clone();
            match case {
                0 => {
                    rows.remove(4);
                }
                1 => rows.push(rows[0].clone()),
                2 => rows[2].initial = true,
                3 => rows.push(row("step", 2, 4, None)),
                4 => rows[10].kind = "parallel_join".into(),
                5 => rows[1].branch_end = Some(4),
                6 => rows[2].position = None,
                7 => rows[4].action = Some("%MX1".into()),
                8 => rows[2].kind = "step".into(),
                9 => rows[1].branch_end = Some(u32::MAX),
                10 => rows[0].branch_end = Some(2),
                _ => unreachable!(),
            }
            assert!(
                validate_layout(&rows).is_err(),
                "accepted malformed case {case}"
            );
        }
    }
    #[test]
    fn geometrically_overlapping_pairs_are_rejected_even_when_paired_in_input_order() {
        let rows = vec![
            row("step", 0, 0, None),
            row("alternative_split", 1, 0, Some(2)),
            row("transition", 2, 0, None),
            row("transition", 2, 2, None),
            row("alternative_join", 5, 0, Some(2)),
            row("alternative_split", 3, 0, Some(2)),
            row("transition", 4, 0, None),
            row("transition", 4, 2, None),
            row("step", 6, 0, None),
            row("alternative_join", 7, 0, Some(2)),
            row("step", 8, 0, None),
        ];
        assert!(validate_layout(&rows).is_err());
    }
    #[cfg(feature = "write")]
    #[test]
    fn compressed_padded_chart_roundtrips_through_public_sfc_inspection() {
        use std::io::Write;
        let mut rows = padded_branch(2048);
        for row in &mut rows {
            if row.kind == "step" {
                row.title = format!("Step{}", row.position.as_ref().unwrap().row);
            } else if row.kind == "transition" {
                row.title = "%MX0".into();
            }
        }
        rows[0].initial = true;
        let grid = grid_xml(&rows).unwrap();
        let xml = format!(
            "<Project><Program><LocalVar><Symbols Count='0'/></LocalVar><SFC_ProgramList><SFC_ProgramProperty MainBlock='1' LanguageType='3' Language='2' PragramName='Main'><EntityContainer BookmarkCount='0'>{grid}</EntityContainer></SFC_ProgramProperty></SFC_ProgramList></Program></Project>"
        );
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gzip.write_all(xml.as_bytes()).unwrap();
        let mut input = b"XG".to_vec();
        input.extend(gzip.finish().unwrap());
        assert!(input.len() < 65536);
        let doc = XgwxDocument::parse(&input).unwrap();
        let programs = doc.program().sfc_programs();
        assert_eq!(programs[0].blocks[0].editable_rows.as_ref(), Some(&rows));
        assert!(programs[0].variables_error.is_none());
    }
    #[test]
    fn largest_two_column_path_layout_fits_native_columns_and_overflow_is_guarded() {
        let end = 65_532;
        let mut rows = vec![
            row("step", 0, 0, None),
            row("alternative_split", 1, 0, Some(end)),
        ];
        for column in (0..=end).step_by(2) {
            rows.push(row("transition", 2, column, None));
        }
        rows.extend([
            row("alternative_join", 3, 0, Some(end)),
            row("step", 4, 0, None),
            row("transition", 5, 0, None),
        ]);
        assert!(validate_layout(&rows).is_ok());
        rows[1].branch_end = Some(65_534);
        assert!(validate_layout(&rows).is_err());
        rows[1].branch_end = Some(u32::MAX);
        assert!(validate_layout(&rows).is_err());
    }
}
