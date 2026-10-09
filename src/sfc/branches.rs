use super::*;

fn branch_kind(row: &SfcRow) -> Option<(u32, bool)> {
    Some(match row.kind.as_str() {
        "alternative_split" => (0, false),
        "alternative_join" => (0, true),
        "parallel_split" => (2, false),
        "parallel_join" => (2, true),
        _ => return None,
    })
}

// Captured balanced, rectangular branches. Reject gaps, crossing/nested ranges,
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
        return if rows.len() > 512
            || rows
                .iter()
                .any(|r| r.branch_end.is_some() || branch_kind(r).is_some())
        {
            Err(fail())
        } else {
            Ok(())
        };
    }
    let mut cells = BTreeMap::new();
    let mut regions = Vec::new();
    let mut open: Option<(&SfcRow, u32)> = None;
    for row in rows {
        let p = row.position.as_ref().ok_or_else(fail)?;
        if p.row >= 512
            || p.column > 14
            || p.column % 2 != 0
            || cells.insert((p.row, p.column), row).is_some()
        {
            return Err(fail());
        }
        if let Some((kind, join)) = branch_kind(row) {
            let end = row.branch_end.ok_or_else(fail)?;
            if p.column != 0 || end < 2 || end > 14 || end % 2 != 0 {
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
    let max_row = cells.keys().map(|(r, _)| *r).max().unwrap_or(0);
    // One main path continues through every row; side lanes exist only inside
    // their matching split/join. Each path has the same captured row height.
    for r in 0..=max_row {
        let main = cells.get(&(r, 0)).ok_or_else(fail)?;
        if let Some(&(start, finish, end, kind)) =
            regions.iter().find(|(s, e, _, _)| *s < r && r < *e)
        {
            let meaningful = (start + 1..r)
                .filter(|n| {
                    cells
                        .get(&(*n, 0))
                        .is_some_and(|v| v.kind != "continuation")
                })
                .count();
            let expected = if main.kind == "continuation" {
                "continuation"
            } else if meaningful % 2 == 0 {
                if kind == 0 {
                    "transition"
                } else {
                    "step"
                }
            } else if kind == 0 {
                "step"
            } else {
                "transition"
            };
            for c in (0..=end).step_by(2) {
                let node = cells.get(&(r, c)).ok_or_else(fail)?;
                if node.kind != expected || node.initial {
                    return Err(fail());
                }
            }
            let count = (start + 1..finish)
                .filter(|n| {
                    cells
                        .get(&(*n, 0))
                        .is_some_and(|v| v.kind != "continuation")
                })
                .count();
            if count % 2 == 0 {
                return Err(fail());
            }
        } else if main.position.as_ref().unwrap().column != 0 {
            return Err(fail());
        }
    }
    for (start, finish, _, kind) in &regions {
        let before = rows
            .iter()
            .rev()
            .find(|r| {
                r.position
                    .as_ref()
                    .is_some_and(|p| p.column == 0 && p.row < *start)
                    && r.kind != "continuation"
            })
            .ok_or_else(fail)?;
        let after = rows
            .iter()
            .find(|r| {
                r.position
                    .as_ref()
                    .is_some_and(|p| p.column == 0 && p.row > *finish)
                    && r.kind != "continuation"
            })
            .ok_or_else(fail)?;
        let expected = if *kind == 0 { "step" } else { "transition" };
        if before.kind != expected || after.kind != expected {
            return Err(fail());
        }
    }
    for (&(r, c), node) in &cells {
        if c != 0
            && !regions
                .iter()
                .any(|(s, e, end, _)| *s < r && r < *e && c <= *end)
        {
            return Err(fail());
        }
        if branch_kind(node).is_some() && !regions.iter().any(|(s, e, _, _)| r == *s || r == *e) {
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
                if value.action.is_some() {
                    2
                } else {
                    10
                }
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
        || block.rows > 512
        || block.columns < 4
        || block.columns > 16
        || block.columns % 2 != 0
        || block.entities.len() != (block.rows * block.columns) as usize
    {
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
        let mut other = block
            .entities
            .iter()
            .find(|e| e.row == node.row && e.column == Some(node.column.unwrap() + 1))?
            .clone();
        let mut first = node.clone();
        first.row = Some(i as u32);
        first.column = Some(0);
        other.row = Some(i as u32);
        other.column = Some(1);
        projected.entities.extend([first, other]);
    }
    let mut rows = projected.linear_rows(sources, bool_names)?;
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
    validate_rows(&rows, bool_names).ok()?;
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
