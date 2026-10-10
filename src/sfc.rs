use crate::{XgwxDocument, XmlElement};
use std::collections::BTreeMap;
mod branches;
mod declarations;
pub use declarations::{SfcArrayBound,SfcDeclaration};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct SfcProgram {
    pub program_index: usize,
    pub blocks: Vec<SfcBlock>,
    pub variables: Vec<SfcVariable>,
    pub variables_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct SfcBlock {
    pub block_index: usize,
    pub name: String,
    pub main: bool,
    pub language_type: Option<u32>,
    pub language: Option<u32>,
    pub rows: u32,
    pub columns: u32,
    pub entities: Vec<SfcEntity>,
    pub editable_rows: Option<Vec<SfcRow>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
pub struct SfcEntity {
    pub entity_index: usize,
    pub type_code: Option<u32>,
    pub row: Option<u32>,
    pub column: Option<u32>,
    pub properties: BTreeMap<String, BTreeMap<String, String>>,
}

fn number(node: &XmlElement, key: &str) -> Option<u32> {
    node.attribute(key)?.parse().ok()
}

impl XgwxDocument {
    /// Read native SFC XML independently of binary ladder program bodies.
    /// Unknown entity kinds and property attributes remain available to callers.
    pub fn sfc_programs(&self) -> Vec<SfcProgram> {
        self.root
            .descendants_named("Program")
            .enumerate()
            .filter_map(|(program_index, program)| {
                let list = program.descendants_named("SFC_ProgramList").next()?;
                let variables = self.sfc_variables(program_index);
                let bool_names = variables
                    .as_ref()
                    .map(|vars| {
                        vars.iter()
                            .filter(|v| !v.system && v.data_type == "BOOL" && v.declaration.as_ref().is_none_or(|d|d.dimensions.is_empty()))
                            .map(|v| v.name.to_lowercase())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let sources = st_sources(list);
                let blocks = list
                    .descendants_named("SFC_ProgramProperty")
                    .enumerate()
                    .map(|(block_index, block)| {
                        let grid = block.descendants_named("EntityGrid").next();
                        let entities = grid
                            .into_iter()
                            .flat_map(|g| g.descendants_named("EntityProperty"))
                            .enumerate()
                            .map(|(entity_index, entity)| SfcEntity {
                                entity_index,
                                type_code: number(entity, "Type"),
                                row: number(entity, "Row"),
                                column: number(entity, "Col"),
                                properties: entity
                                    .children
                                    .iter()
                                    .map(|p| {
                                        (
                                            p.name.clone(),
                                            p.attributes
                                                .iter()
                                                .map(|a| (a.name.clone(), a.value.clone()))
                                                .collect(),
                                        )
                                    })
                                    .collect(),
                            })
                            .collect();
                        let mut result = SfcBlock {
                            block_index,
                            name: block.attribute("PragramName").unwrap_or("").to_owned(),
                            main: block.attribute("MainBlock") == Some("1"),
                            language_type: number(block, "LanguageType"),
                            language: number(block, "Language"),
                            rows: grid.and_then(|g| number(g, "RowSize")).unwrap_or(0),
                            columns: grid.and_then(|g| number(g, "ColSize")).unwrap_or(0),
                            entities,
                            editable_rows: None,
                        };
                        if sources.is_some() && linear_container_supported(block) {
                            result.editable_rows =
                                result.linear_rows(sources.as_ref().unwrap(), &bool_names, false).or_else(|| {
                                    branches::decode_rows(&result, sources.as_ref().unwrap(), &bool_names)
                                });
                        }
                        result
                    })
                    .collect();
                Some(SfcProgram {
                    program_index,
                    blocks,
                    variables_error: variables.as_ref().err().map(ToString::to_string),
                    variables: variables.unwrap_or_default(),
                })
            })
            .collect()
    }
}

#[cfg(feature = "write")]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct SfcEntityPatch {
    pub program_index: usize,
    pub block_index: usize,
    pub entity_index: usize,
    pub expected_type: u32,
    pub expected_row: u32,
    pub expected_column: u32,
    pub field: String,
    pub expected_value: String,
    pub replacement: String,
}

#[cfg(feature = "write")]
impl XgwxDocument {
    /// Edit captured SFC step comments or direct BOOL M transitions.
    /// Identity and old-value checks reject stale selections; all other XML is preserved.
    pub fn edit_sfc_entity(&mut self, patch: &SfcEntityPatch) -> Result<(), crate::XgwxError> {
        use crate::XgwxError;
        let fail = |message: &str| XgwxError::SfcEdit(message.to_owned());
        if patch.replacement.len() > 65536 {
            return Err(fail("replacement is too long"));
        }
        let doc = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let program = doc
            .descendants()
            .filter(|n| n.has_tag_name("Program"))
            .nth(patch.program_index)
            .ok_or_else(|| fail("program is absent"))?;
        let block = program
            .descendants()
            .filter(|n| n.has_tag_name("SFC_ProgramProperty"))
            .nth(patch.block_index)
            .ok_or_else(|| fail("SFC block is absent"))?;
        if block.attribute("Language") != Some("2") || block.attribute("LanguageType") != Some("3")
        {
            return Err(fail("only native XGI SFC blocks are validated"));
        }
        let entity = block
            .descendants()
            .filter(|n| n.has_tag_name("EntityProperty"))
            .nth(patch.entity_index)
            .ok_or_else(|| fail("entity is absent"))?;
        for (key, value) in [
            ("Type", patch.expected_type),
            ("Row", patch.expected_row),
            ("Col", patch.expected_column),
        ] {
            if entity.attribute(key).and_then(|v| v.parse::<u32>().ok()) != Some(value) {
                return Err(fail("stale entity position or type"));
            }
        }
        if !matches!(patch.expected_type, 0 | 1) {
            return Err(fail("entity kind is not validated for editing"));
        }
        let target = entity
            .children()
            .find(|n| n.has_tag_name("EntityStep"))
            .ok_or_else(|| fail("step/transition properties are absent"))?;
        let field = match patch.field.as_str() {
            "comment" if patch.expected_type == 0 => "Comment",
            "condition"
                if patch.expected_type == 1 && target.attribute("PropertyProgram") == Some("0") =>
            {
                let valid = |s: &str| {
                    s.strip_prefix("%MX").is_some_and(|v| {
                        !v.is_empty()
                            && v.bytes().all(|c| c.is_ascii_digit())
                            && v.parse::<u16>().is_ok()
                    })
                };
                if !valid(&patch.expected_value) || !valid(&patch.replacement) {
                    return Err(fail(
                        "only existing direct %MX BOOL transition conditions are validated",
                    ));
                }
                "Title"
            }
            _ => return Err(fail("field is not validated for editing")),
        };
        let attribute = target
            .attributes()
            .find(|a| a.name() == field)
            .ok_or_else(|| fail("existing field is absent"))?;
        if attribute.value() != patch.expected_value {
            return Err(fail("stale field value"));
        }
        let range = attribute.range_value();
        let value = patch
            .replacement
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\r', "&#xD;")
            .replace('\n', "&#xA;")
            .replace('\t', "&#x9;");
        let mut candidate = self.clone();
        let mut replacements = vec![(range, value.clone())];
        // XG5000 mirrors transition text into the adjacent annotation cell.
        if patch.expected_type == 1 {
            let mirrors = block
                .descendants()
                .filter(|n| {
                    n.has_tag_name("EntityProperty")
                        && n.attribute("Type") == Some("9")
                        && n.attribute("Row").and_then(|v| v.parse::<u32>().ok())
                            == Some(patch.expected_row)
                        && n.attribute("Col").and_then(|v| v.parse::<u32>().ok())
                            == patch.expected_column.checked_add(1)
                })
                .collect::<Vec<_>>();
            if mirrors.len() != 1 {
                return Err(fail("transition annotation is absent or ambiguous"));
            }
            let mirror = mirrors[0]
                .children()
                .find(|n| n.has_tag_name("EntityStep"))
                .and_then(|n| n.attributes().find(|a| a.name() == field))
                .ok_or_else(|| fail("transition annotation field is absent"))?;
            if mirror.value() != patch.expected_value {
                return Err(fail("transition annotation is inconsistent"));
            }
            replacements.push((mirror.range_value(), value));
        }
        candidate.apply_xml_replacements(replacements)?;
        candidate.to_verified_bytes()?;
        *self = candidate;
        Ok(())
    }
}

/// A linear row or positioned node/boundary in a captured balanced SFC chart.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct SfcRow {
    pub kind: String,
    pub title: String,
    pub comment: String,
    pub initial: bool,
    pub action: Option<String>,
    #[cfg_attr(
        feature = "wasm",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub action_qualifier: Option<String>,
    #[cfg_attr(
        feature = "wasm",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub action_time: Option<String>,
    #[cfg_attr(
        feature = "wasm",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub action_code: Option<String>,
    #[cfg_attr(
        feature = "wasm",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub transition_code: Option<String>,
    #[cfg_attr(
        feature = "wasm",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub position: Option<SfcPosition>,
    #[cfg_attr(
        feature = "wasm",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub branch_end: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct SfcPosition {
    pub row: u32,
    pub column: u32,
}

impl SfcBlock {
    fn linear_rows(
        &self,
        sources: &BTreeMap<String, (u32, String)>,
        bool_names: &[String],
        projection: bool,
    ) -> Option<Vec<SfcRow>> {
        if !self.main || self.language_type != Some(3) || self.language != Some(2) {
            return None;
        }
        if self.entities.is_empty() {
            return (self.rows == 0 && self.columns == 0).then(Vec::new);
        }
        if self.rows > if projection { branches::MAX_GRID_CELLS / 2 } else { branches::MAX_ROWS } || self.columns != 2 || self.entities.len() != self.rows as usize * 2 {
            return None;
        }
        let mut rows = Vec::new();
        let mut positions = BTreeMap::new();
        for e in &self.entities {
            if positions.insert((e.row?, e.column?), e).is_some() {
                return None;
            }
            if e.properties.get("EntityBasic")?
                != &BTreeMap::from([("Virtural".into(), "0".into())])
            {
                return None;
            }
            if matches!(e.type_code, Some(7 | 10)) {
                if e.properties.len() != 1 {
                    return None;
                }
                continue;
            }
            let p = e.properties.get("EntityStep")?;
            let keys = [
                "Title",
                "InnerVariableName",
                "Comment",
                "Bookmark",
                "BreakPoint",
                "PropertyProgram",
                "InitialStep",
                "StepVariable",
            ];
            if p.len() != keys.len() || keys.iter().any(|k| !p.contains_key(*k)) {
                return None;
            }
            if ["Bookmark", "BreakPoint", "StepVariable"]
                .iter()
                .any(|k| p[*k] != "0")
            {
                return None;
            }
            if !matches!(p["PropertyProgram"].as_str(), "0" | "1")
                || p["PropertyProgram"] == "1" && !matches!(e.type_code, Some(1 | 2))
            {
                return None;
            }
            if !matches!(p["InitialStep"].as_str(), "0" | "1") {
                return None;
            }
            if e.type_code == Some(2) {
                let a = e.properties.get("EntityAction")?;
                if e.properties.len() != 3
                    || a.len() != 6
                    || ["FBInstanceName", "FBInstanceOut", "ResetVarName"]
                        .iter()
                        .any(|k| a.get(*k).is_none_or(|v| !v.is_empty()))
                    || a.get("TimeIndex").is_none_or(|v| v != "0")
                    || qualifier_name(a.get("Qualifier")?).is_none()
                    || !a.contains_key("Time")
                {
                    return None;
                }
            } else if e.properties.len() != 2 {
                return None;
            }
        }
        for row in 0..self.rows {
            let e = positions.get(&(row, 0))?;
            let kind = match e.type_code? {
                0 => "step",
                1 => "transition",
                5 => "jump",
                6 => "label",
                7 => "continuation",
                _ => return None,
            };
            let empty = BTreeMap::from([
                ("Title".into(), String::new()), ("Comment".into(), String::new()),
                ("InitialStep".into(), "0".into()), ("PropertyProgram".into(), "0".into()),
            ]);
            let p = e.properties.get("EntityStep").unwrap_or(&empty);
            let adjacent = positions.get(&(row, 1))?;
            let action = if matches!(kind, "step" | "continuation") {
                match adjacent.type_code? {
                    10 => None,
                    2 if adjacent.properties["EntityStep"]["InitialStep"] == "0"
                        && adjacent.properties["EntityStep"]["Comment"].is_empty() =>
                    {
                        Some(adjacent.properties["EntityStep"]["Title"].clone())
                    }
                    _ => return None,
                }
            } else {
                if adjacent.type_code != Some(9)
                    || adjacent.properties["EntityStep"]["Title"] != p["Title"]
                    || adjacent.properties["EntityStep"]["Comment"] != p["Comment"]
                    || adjacent.properties["EntityStep"]["InitialStep"] != "0"
                {
                    return None;
                }
                None
            };
            rows.push(SfcRow {
                position: None,
                branch_end: None,
                kind: kind.into(),
                title: p["Title"].clone(),
                comment: p["Comment"].clone(),
                initial: p["InitialStep"] == "1",
                action_code: if adjacent.type_code == Some(2)
                    && adjacent.properties["EntityStep"]["PropertyProgram"] == "1"
                {
                    let (kind, source) =
                        sources.get(&adjacent.properties["EntityStep"]["Title"])?;
                    if *kind != 1 {
                        return None;
                    }
                    Some(source.clone())
                } else {
                    None
                },
                transition_code: if p["PropertyProgram"] == "1" {
                    if kind != "transition" {
                        return None;
                    }
                    let (kind, source) = sources.get(&p["Title"])?;
                    if *kind != 2 {
                        return None;
                    }
                    Some(source.clone())
                } else {
                    None
                },
                action,
                action_qualifier: if adjacent.type_code == Some(2) {
                    let q = qualifier_name(&adjacent.properties["EntityAction"]["Qualifier"])?;
                    (q != "N").then(|| q.to_string())
                } else {
                    None
                },
                action_time: if adjacent.type_code == Some(2) {
                    let t = &adjacent.properties["EntityAction"]["Time"];
                    (!t.is_empty()).then(|| t.clone())
                } else {
                    None
                },
            });
        }
        validate_rows(&rows, bool_names, projection).ok()?;
        Some(rows)
    }
}

fn linear_container_supported(block: &XmlElement) -> bool {
    let Some(container) = block.descendants_named("EntityContainer").next() else {
        return false;
    };
    if container.attribute("BookmarkCount") != Some("0") || container.children.len() != 1 {
        return false;
    }
    let Some(grid) = container
        .children
        .first()
        .filter(|n| n.name == "EntityGrid")
    else {
        return false;
    };
    if grid.attributes.len() != 2
        || grid
            .attributes
            .iter()
            .any(|a| !["RowSize", "ColSize"].contains(&a.name.as_str()))
        || grid.children.len() != 1
    {
        return false;
    }
    let list = &grid.children[0];
    if list.name != "EntityPropertyList" || !list.attributes.is_empty() {
        return false;
    }
    list.children.iter().all(|e| {
        e.name == "EntityProperty"
            && e.attributes.len() == 3
            && e.attributes
                .iter()
                .all(|a| ["Type", "Row", "Col"].contains(&a.name.as_str()))
            && e.children
                .iter()
                .enumerate()
                .all(|(i, c)| {
                    c.children.is_empty()
                        && c.text.trim().is_empty()
                        && !e.children[..i].iter().any(|n| n.name == c.name)
                })
    })
}

fn qualifier_name(code: &str) -> Option<&'static str> {
    match code {
        "1" => Some("N"),
        "2" => Some("R"),
        "4" => Some("S"),
        "8" => Some("L"),
        "16" => Some("D"),
        "32" => Some("P"),
        "64" => Some("SD"),
        "128" => Some("DS"),
        "256" => Some("SL"),
        _ => None,
    }
}
fn qualifier_code(name: &str) -> Option<u16> {
    [1, 2, 4, 8, 16, 32, 64, 128, 256]
        .into_iter()
        .find(|c| qualifier_name(&c.to_string()) == Some(name))
}
fn timed_qualifier(name: &str) -> bool {
    matches!(name, "L" | "D" | "SD" | "DS" | "SL")
}
fn valid_action_time(time: &str) -> bool {
    // A bounded literal subset avoids variables and unverified timer layouts.
    let Some(mut rest) = time.strip_prefix("T#") else {
        return false;
    };
    let mut last_rank = 5;
    let mut total = 0u64;
    let mut parts = 0;
    while !rest.is_empty() {
        let n = rest.bytes().take_while(|c| c.is_ascii_digit()).count();
        if n == 0 {
            return false;
        }
        let Ok(value) = rest[..n].parse::<u64>() else {
            return false;
        };
        rest = &rest[n..];
        let (unit, rank, factor) = if rest.starts_with("ms") {
            ("ms", 0, 1)
        } else if rest.starts_with('s') {
            ("s", 1, 1000)
        } else if rest.starts_with('m') {
            ("m", 2, 60000)
        } else if rest.starts_with('h') {
            ("h", 3, 3600000)
        } else if rest.starts_with('d') {
            ("d", 4, 86400000)
        } else {
            return false;
        };
        if rank >= last_rank {
            return false;
        }
        last_rank = rank;
        let Some(sum) = value.checked_mul(factor).and_then(|v| total.checked_add(v)) else {
            return false;
        };
        total = sum;
        parts += 1;
        rest = &rest[unit.len()..];
    }
    parts > 0 && total <= u32::MAX as u64
}

fn validate_rows(rows: &[SfcRow], bool_names: &[String], projection: bool) -> Result<(), crate::XgwxError> {
    let fail = |s: &str| crate::XgwxError::SfcEdit(s.into());
    if !projection { branches::validate_layout(rows)?; }
    if rows.len() > branches::MAX_GRID_CELLS as usize { return Err(fail("chart exceeds the editor grid size safety limit")); }
    if rows.iter().filter(|r| r.kind == "step").count() > 512 {
        return Err(fail("XG5000 supports at most 512 ordinary steps per SFC program"));
    }
    let identifier = |s: &str| {
        !s.is_empty()
            && s.len() <= 32
            && s.bytes()
                .enumerate()
                .all(|(i, c)| c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))
    };
    let bool_address = |s: &str| {
        bool_names.contains(&s.to_lowercase())
            || s.strip_prefix("%MX").is_some_and(|n| {
                !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()) && n.parse::<u16>().is_ok()
            })
    };
    let mut steps = std::collections::BTreeSet::new();
    let mut labels = std::collections::BTreeSet::new();
    let mut initial_count = 0;
    for row in rows {
        if row.action_code.is_some() && (!matches!(row.kind.as_str(), "step" | "continuation") || row.action.is_none())
            || row.transition_code.is_some() && row.kind != "transition"
        {
            return Err(fail("ST source requires an action or transition program"));
        }
        for code in [&row.action_code, &row.transition_code]
            .into_iter()
            .flatten()
        {
            if code.encode_utf16().count() > 65536
                || code
                    .chars()
                    .any(|c| c == '\0' || (c.is_control() && !matches!(c, '\r' | '\n' | '\t')))
            {
                return Err(fail(
                    "ST source is too long or contains invalid control characters",
                ));
            }
        }
        if row.comment.len() > 65536 {
            return Err(fail("comment is too long"));
        }
        match row.kind.as_str() {
            "alternative_split" | "alternative_join" | "parallel_split" | "parallel_join"
                if row.title.is_empty() => {}
            "step" | "continuation" => {
                if row.kind == "step" && (!identifier(&row.title) || !steps.insert(&row.title)) {
                    return Err(fail(
                        "step names must be unique identifiers of at most 32 characters",
                    ));
                }
                if row.kind == "continuation" && (!row.title.is_empty() || row.initial || !row.comment.is_empty()) {
                    return Err(fail("action continuation rows have no step name, comment, or initial status"));
                }
                initial_count += usize::from(row.initial);
                let qualifier = row.action_qualifier.as_deref().unwrap_or("N");
                if qualifier_code(qualifier).is_none() {
                    return Err(fail("unknown action qualifier"));
                }
                if row.action.is_some() {
                    let time = row.action_time.as_deref().unwrap_or("");
                    if timed_qualifier(qualifier) && !valid_action_time(time) {
                        return Err(fail("timed actions require a TIME literal such as T#2s"));
                    }
                    if !timed_qualifier(qualifier) && !time.is_empty() {
                        return Err(fail("this action qualifier does not use a time"));
                    }
                }
                if row.action.as_deref().is_some_and(|a| {
                    if row.action_code.is_some() {
                        !identifier(a)
                    } else {
                        !bool_address(a)
                    }
                }) {
                    return Err(fail(
                        "actions require a direct %MX address or declared BOOL variable",
                    ));
                }
            }
            "transition"
                if if row.transition_code.is_some() {
                    identifier(&row.title)
                } else {
                    bool_address(&row.title)
                } => {}
            "label" if identifier(&row.title) && labels.insert(&row.title) => {}
            "jump" if identifier(&row.title) => {}
            _ => {
                return Err(fail(
                    "use unique label identifiers and %MX or declared BOOL transitions",
                ));
            }
        }
        if row.action.is_none() && (row.action_qualifier.is_some() || row.action_time.is_some()) {
            return Err(fail("action properties require an action operand"));
        }
        if !matches!(row.kind.as_str(), "step" | "continuation") && (row.initial || row.action.is_some() || !row.comment.is_empty()) {
            return Err(fail(
                "only steps support initial status, comments, and actions",
            ));
        }
    }
    if initial_count > 1 {
        return Err(fail("linear charts support one initial step"));
    }
    if rows
        .iter()
        .any(|r| r.kind == "jump" && !labels.contains(&r.title))
    {
        return Err(fail("jump target must name an existing label"));
    }
    Ok(())
}

#[cfg(feature = "write")]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct SfcSequencePatch {
    pub program_index: usize,
    pub block_index: usize,
    pub expected_entities: Vec<SfcEntity>,
    pub rows: Vec<SfcRow>,
    #[cfg_attr(feature = "wasm", serde(default))]
    pub expected_rows: Option<Vec<SfcRow>>,
}

#[cfg(feature = "write")]
impl XgwxDocument {
    /// Replace a captured linear chart atomically, retaining program identities and local symbols.
    pub fn replace_sfc_sequence(
        &mut self,
        patch: &SfcSequencePatch,
    ) -> Result<(), crate::XgwxError> {
        use crate::XgwxError;
        let fail = |s: &str| XgwxError::SfcEdit(s.into());
        let programs = self.sfc_programs();
        let program = programs
            .iter()
            .find(|p| p.program_index == patch.program_index)
            .ok_or_else(|| fail("SFC program is absent"))?;
        let block = program
            .blocks
            .get(patch.block_index)
            .ok_or_else(|| fail("SFC block is absent"))?;
        if patch
            .expected_rows
            .as_ref()
            .is_some_and(|rows| block.editable_rows.as_ref() != Some(rows))
            || block.editable_rows.as_ref().is_some_and(|rows| {
                rows.iter()
                    .any(|r| r.action_code.is_some() || r.transition_code.is_some())
            }) && patch.expected_rows.is_none()
        {
            return Err(fail("stale or missing SFC source snapshot"));
        }
        if block.entities != patch.expected_entities {
            return Err(fail("stale SFC chart"));
        }
        if block.editable_rows.is_none() {
            return Err(fail(
                "only captured linear or balanced branch charts with supported operands and actions support structural editing",
            ));
        }
        let bool_names = program
            .variables
            .iter()
            .filter(|v| !v.system && v.data_type == "BOOL" && v.declaration.as_ref().is_none_or(|d|d.dimensions.is_empty()))
            .map(|v| v.name.to_lowercase())
            .collect::<Vec<_>>();
        validate_rows(&patch.rows, &bool_names, false)?;
        let xml = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let node = xml
            .descendants()
            .filter(|n| n.has_tag_name("Program"))
            .nth(patch.program_index)
            .unwrap();
        let block_node = node
            .descendants()
            .filter(|n| n.has_tag_name("SFC_ProgramProperty"))
            .nth(patch.block_index)
            .unwrap();
        let grid = block_node
            .descendants()
            .find(|n| n.has_tag_name("EntityGrid"))
            .ok_or_else(|| fail("entity grid is absent"))?;
        if grid
            .attributes()
            .any(|a| !["RowSize", "ColSize"].contains(&a.name()))
            || grid
                .children()
                .filter(|n| n.is_element())
                .any(|n| !n.has_tag_name("EntityPropertyList"))
        {
            return Err(fail("unknown grid data must be preserved"));
        }
        let escape = |s: &str| {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\r', "&#xD;")
                .replace('\n', "&#xA;")
                .replace('\t', "&#x9;")
        };
        let entity = |kind: u8, row: usize, col: u8, title: &str, comment: &str, initial: bool| {
            let mut s = format!(
                "<EntityProperty Type=\"{kind}\" Col=\"{col}\" Row=\"{row}\"><EntityBasic Virtural=\"0\"/>"
            );
            if !matches!(kind, 7 | 10) {
                s.push_str(&format!("<EntityStep Title=\"{}\" InnerVariableName=\"\" Comment=\"{}\" Bookmark=\"0\" BreakPoint=\"0\" PropertyProgram=\"0\" InitialStep=\"{}\" StepVariable=\"0\"/>",escape(title),escape(comment),u8::from(initial)));
            }
            if kind == 2 {
                s.push_str("<EntityAction Time=\"\" Qualifier=\"1\" FBInstanceName=\"\" FBInstanceOut=\"\" ResetVarName=\"\" TimeIndex=\"0\"/>");
            }
            s.push_str("</EntityProperty>");
            s
        };
        let mut grid_xml = format!(
            "<EntityGrid RowSize=\"{}\" ColSize=\"{}\"><EntityPropertyList>",
            patch.rows.len(),
            if patch.rows.is_empty() { 0 } else { 2 }
        );
        for (row, r) in patch.rows.iter().enumerate() {
            let kind = match r.kind.as_str() {
                "step" => 0,
                "transition" => 1,
                "jump" => 5,
                "continuation" => 7,
                _ => 6,
            };
            let mut value = entity(kind, row, 0, &r.title, &r.comment, r.initial);
            if r.transition_code.is_some() {
                value = value.replace("PropertyProgram=\"0\"", "PropertyProgram=\"1\"");
            }
            grid_xml.push_str(&value);
        }
        for (row, r) in patch.rows.iter().enumerate() {
            let (kind, title) = if matches!(r.kind.as_str(), "step" | "continuation") {
                if let Some(a) = &r.action {
                    (2, a.as_str())
                } else {
                    (10, "")
                }
            } else {
                (9, r.title.as_str())
            };
            let mut action_xml = entity(kind, row, 1, title, "", false);
            if kind == 2 {
                let code = qualifier_code(r.action_qualifier.as_deref().unwrap_or("N")).unwrap();
                action_xml = action_xml
                    .replace("Qualifier=\"1\"", &format!("Qualifier=\"{code}\""))
                    .replace(
                        "Time=\"\"",
                        &format!(
                            "Time=\"{}\"",
                            escape(r.action_time.as_deref().unwrap_or(""))
                        ),
                    );
            }
            if r.action_code.is_some() && kind == 2 {
                action_xml = action_xml.replace("PropertyProgram=\"0\"", "PropertyProgram=\"1\"");
            }
            grid_xml.push_str(&action_xml);
        }
        grid_xml.push_str("</EntityPropertyList></EntityGrid>");
        if patch.rows.iter().any(|r| r.position.is_some()) {
            grid_xml = branches::grid_xml(&patch.rows)?;
        }
        let mut replacements = vec![(grid.range(), grid_xml)];
        // XG5000 regenerates these compilation caches after structural edits.
        for tag in ["SFC_ProgramData", "SFC_ProgramDataBlock"] {
            let cache = block_node
                .children()
                .find(|n| n.has_tag_name(tag))
                .ok_or_else(|| fail("SFC compilation metadata is absent"))?;
            let keys: &[&str] = if tag == "SFC_ProgramData" {
                &[
                    "UploadProgramSize",
                    "DocRungTableOffset",
                    "DocRungTableSize",
                    "PreDocStepCount",
                    "PostDocStepCount",
                    "Variable",
                    "InitVariable",
                    "NFE",
                    "NFE1",
                ]
            } else {
                &[
                    "StartEnd",
                    "StepTransition",
                    "StopRestart",
                    "StopMode",
                    "NumberActiveStep",
                    "ContinuousTransition",
                ]
            };
            if cache.attributes().len() != keys.len()
                || cache.attributes().any(|a| !keys.contains(&a.name()))
                || cache
                    .children()
                    .any(|n| n.is_element() || n.text().is_some_and(|t| !t.trim().is_empty()))
            {
                return Err(fail("unknown compilation data must be preserved"));
            }
            let mut value = format!("<{tag}");
            for a in cache.attributes() {
                let v = match a.name() {
                    "UploadProgramSize" | "DocRungTableOffset" | "DocRungTableSize"
                    | "PreDocStepCount" | "PostDocStepCount" => "0",
                    _ => "",
                };
                value.push_str(&format!(" {}=\"{v}\"", a.name()));
            }
            value.push_str("/>");
            replacements.push((cache.range(), value));
        }
        let chart = block_node
            .children()
            .find(|n| n.has_tag_name("SFC_Program"))
            .ok_or_else(|| fail("SFC chart is absent"))?;
        let count = chart
            .attributes()
            .find(|a| a.name() == "StepCount")
            .ok_or_else(|| fail("step count is absent"))?;
        replacements.push((count.range_value(), "0".into()));
        let list = block_node
            .parent()
            .ok_or_else(|| fail("SFC block list is absent"))?;
        let mut desired = BTreeMap::<String, (u32, String)>::new();
        for row in &patch.rows {
            for (name, kind, code) in [
                (row.action.as_deref().unwrap_or(""), 1, &row.action_code),
                (row.title.as_str(), 2, &row.transition_code),
            ] {
                if let Some(code) = code {
                    if name.eq_ignore_ascii_case(&block.name) {
                        return Err(fail("program name conflicts with main block"));
                    }
                    let value = (kind, code.clone());
                    if desired
                        .keys()
                        .any(|k| k.eq_ignore_ascii_case(name) && k != name)
                    {
                        return Err(fail("program names must be unique ignoring case"));
                    }
                    if desired
                        .insert(name.to_string(), value.clone())
                        .is_some_and(|old| old != value)
                    {
                        return Err(fail("shared programs require matching source and kind"));
                    }
                }
            }
        }
        let previous = block.editable_rows.as_ref().unwrap();
        let mut orphan_count = 0;
        let mut extra = String::new();
        // All non-main blocks have been checked as the captured ST layout by st_sources.
        for old in list.children().filter(|n| {
            n.has_tag_name("SFC_ProgramProperty") && n.attribute("MainBlock") == Some("0")
        }) {
            let name = old.attribute("PragramName").unwrap_or("");
            if let Some((kind, code)) = desired.remove(name) {
                replacements.push((old.range(), st_block_xml(name, kind, &code)));
            } else if previous.iter().any(|r| {
                r.action_code.is_some() && r.action.as_deref() == Some(name)
                    || r.transition_code.is_some() && r.title == name
            }) {
                replacements.push((old.range(), String::new()));
            } else {
                orphan_count += 1;
            }
        }
        for (name, (kind, code)) in desired {
            extra.push_str(&st_block_xml(&name, kind, &code));
        }
        if !extra.is_empty() {
            let end = list.range().end - "</SFC_ProgramListData>".len();
            replacements.push((end..end, extra));
        }
        let outer = list
            .parent()
            .ok_or_else(|| fail("SFC program list is absent"))?;
        if let Some(attr) = outer.attributes().find(|a| a.name() == "ProgramCount") {
            let count = patch
                .rows
                .iter()
                .flat_map(|r| {
                    [
                        (r.action.as_deref().unwrap_or(""), &r.action_code),
                        (r.title.as_str(), &r.transition_code),
                    ]
                })
                .filter(|(_, c)| c.is_some())
                .map(|(n, _)| n)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                + 1
                + orphan_count;
            replacements.push((attr.range_value(), count.to_string()));
        }
        let mut candidate = self.clone();
        candidate.apply_xml_replacements(replacements)?;
        candidate.to_verified_bytes()?;
        *self = candidate;
        Ok(())
    }
}

// Native ST CodeList contains exactly CodeCount UTF-16 units, compressed with bzip2.
pub(crate) fn read_st_source(block: &XmlElement) -> Option<String> {
    let st = block.children.iter().find(|n| n.name == "ST_Program")?;
    if st.attribute("Version") != Some("Ver 1.1")
        || st.attributes.len() != 2
        || number(st, "StepCount").is_none()
        || st.children.len() != 3
        || st
            .children
            .iter()
            .any(|n| !["CodeList", "Breakpoints", "Bookmarks"].contains(&n.name.as_str()))
        || st
            .children
            .iter()
            .filter(|n| n.name != "CodeList")
            .any(|n| {
                !n.children.is_empty() || !n.text.trim().is_empty() || !n.attributes.is_empty()
            })
    {
        return None;
    }
    let code = st.children.iter().find(|n| n.name == "CodeList")?;
    if !code.children.is_empty()
        || code
            .attributes
            .iter()
            .any(|a| !["CodeCount", "dt", "Compressed"].contains(&a.name.as_str()))
    {
        return None;
    }
    let count = number(code, "CodeCount")? as usize;
    if count > 65536 {
        return None;
    }
    let raw = if code.text.trim().is_empty() {
        Vec::new()
    } else {
        crate::decode_base64_payload(&code.text, code.attribute("Compressed") == Some("1"))
            .ok()?
            .data
    };
    if raw.len() != count * 2 {
        return None;
    }
    String::from_utf16(
        &raw.chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>(),
    )
    .ok()
}
fn st_sources(list: &XmlElement) -> Option<BTreeMap<String, (u32, String)>> {
    let mut result = BTreeMap::new();
    let mut main_count = 0;
    for block in list.descendants_named("SFC_ProgramProperty") {
        if block.attribute("MainBlock") == Some("1") {
            main_count += 1;
            continue;
        }
        let kind = number(block, "LanguageType")?;
        if !matches!(kind, 1 | 2)
            || number(block, "Language") != Some(4)
            || block.attribute("MainBlock") != Some("0")
            || block.attribute("Comment") != Some("")
            || block.attributes.len() != 5
            || block.children.len() != if kind == 1 { 3 } else { 2 }
            || block.children.iter().any(|n| {
                !["SFC_ProgramData", "SFC_ProgramDataAction", "ST_Program"]
                    .contains(&n.name.as_str())
            })
        {
            return None;
        }
        if let Some(action) = block
            .children
            .iter()
            .find(|n| n.name == "SFC_ProgramDataAction")
        {
            if kind != 1
                || action.attributes.len() != 1
                || action.attribute("ActionPostScan") != Some("0")
                || !action.children.is_empty()
                || !action.text.trim().is_empty()
            {
                return None;
            }
        }
        let cache = block
            .children
            .iter()
            .find(|n| n.name == "SFC_ProgramData")?;
        if !cache.children.is_empty()
            || !cache.text.trim().is_empty()
            || cache.attributes.len() != 9
            || cache.attributes.iter().any(|a| {
                ![
                    "UploadProgramSize",
                    "DocRungTableOffset",
                    "DocRungTableSize",
                    "PreDocStepCount",
                    "PostDocStepCount",
                    "Variable",
                    "InitVariable",
                    "NFE",
                    "NFE1",
                ]
                .contains(&a.name.as_str())
            })
        {
            return None;
        }
        let name = block.attribute("PragramName")?.to_string();
        if result
            .insert(name, (kind, read_st_source(block)?))
            .is_some()
        {
            return None;
        }
    }
    (main_count == 1).then_some(result)
}
#[cfg(feature = "write")]
fn encode_sfc_payload(bytes: &[u8]) -> String {
    use base64::Engine;
    use std::io::Write;
    let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::best());
    encoder.write_all(bytes).expect("writing into a vector");
    base64::engine::general_purpose::STANDARD.encode(encoder.finish().expect("vector compression"))
}
#[cfg(feature = "write")]
fn st_block_xml(name: &str, kind: u32, source: &str) -> String {
    let bytes = source
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    format!(
        "<SFC_ProgramProperty LanguageType=\"{kind}\" Language=\"4\" PragramName=\"{name}\" Comment=\"\" MainBlock=\"0\"><SFC_ProgramData UploadProgramSize=\"0\" DocRungTableOffset=\"0\" DocRungTableSize=\"0\" PreDocStepCount=\"0\" PostDocStepCount=\"0\" Variable=\"\" InitVariable=\"\" NFE=\"\" NFE1=\"\"/>{}<ST_Program Version=\"Ver 1.1\" StepCount=\"0\"><CodeList CodeCount=\"{}\" dt:dt=\"bin.base64\" xmlns:dt=\"urn:schemas-microsoft-com:datatypes\" Compressed=\"1\">{}</CodeList><Breakpoints/><Bookmarks/></ST_Program></SFC_ProgramProperty>",
        if kind == 1 {
            "<SFC_ProgramDataAction ActionPostScan=\"0\"/>"
        } else {
            ""
        },
        bytes.len() / 2,
        encode_sfc_payload(&bytes)
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct SfcVariable {
    pub name: String,
    pub data_type: String,
    pub description: String,
    pub system: bool,
    #[cfg_attr(
        feature = "wasm",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub declaration: Option<SfcDeclaration>,
}
impl XgwxDocument {
    /// Decode regular IEC declarations while retaining the two captured SFC system records.
    pub fn sfc_variables(
        &self,
        program_index: usize,
    ) -> Result<Vec<SfcVariable>, crate::XgwxError> {
        let program = self
            .root
            .descendants_named("Program")
            .nth(program_index)
            .ok_or_else(|| crate::XgwxError::SfcEdit("program is absent".into()))?;
        let table = program
            .descendants_named("LocalVar")
            .next()
            .and_then(|n| n.descendants_named("Symbols").next())
            .ok_or_else(|| crate::XgwxError::SfcEdit("symbols are absent".into()))?;
        let mut bytes =
            crate::decode_base64_payload(&table.text, table.attribute("Compressed") == Some("1"))?
                .data;
        if self.xgk_auto_allocation() { declarations::normalize_xgk_types(&mut bytes)?; }
        let metadata = declarations::metadata(&bytes)?;
        Ok(self
            .sfc_symbols(program_index)?
            .into_iter()
            .map(|s| SfcVariable {
                system: program.descendants_named("SFC_ProgramList").next().is_some() && matches!(s.name.as_str(), "TRANS" | "GOTO_INIT"),
                declaration: metadata
                    .get(&s.name)
                    .map(|(_, v)| v.clone())
                    .filter(|v| *v != SfcDeclaration::default()),
                data_type: metadata
                    .get(&s.name)
                    .map(|(ty, _)| ty.clone())
                    .filter(|s| !s.is_empty())
                    .or(s.type_reference)
                    .or(s.data_type)
                    .unwrap_or_default(),
                name: s.name,
                description: s.description.unwrap_or_default(),
            })
            .collect())
    }
    fn sfc_symbols(
        &self,
        program_index: usize,
    ) -> Result<Vec<crate::IecLocalSymbol>, crate::XgwxError> {
        use base64::Engine;
        let fail = || crate::XgwxError::SfcEdit("unsupported SFC variable table".into());
        let program = self
            .root
            .descendants_named("Program")
            .nth(program_index)
            .ok_or_else(fail)?;
        let is_sfc = program.descendants_named("SFC_ProgramList").next().is_some();
        let is_text = matches!(program.attribute("Kind"), Some("4" | "9"))
            && program.children.iter().find(|n| n.name == "Body").is_some_and(|b| read_st_source(b).is_some());
        if !is_sfc && !is_text { return Err(fail()); }
        let mut table = program
            .descendants_named("LocalVar")
            .next()
            .and_then(|n| n.descendants_named("Symbols").next())
            .ok_or_else(fail)?
            .clone();
        if number(&table, "Count") == Some(0) {
            return crate::IecLocalSymbol::from_symbols_element(&table);
        }
        let mut bytes =
            crate::decode_base64_payload(&table.text, table.attribute("Compressed") == Some("1"))?
                .data;
        // Class 11/12 and the SFC-only flag differ from ordinary VAR declarations.
        for (name, class) in [("GOTO_INIT", 11_u32), ("TRANS", 12_u32)].into_iter().filter(|_| is_sfc) {
            let mut marker = vec![0xff, 0xfe, 0xff, name.len() as u8];
            marker.extend(name.encode_utf16().flat_map(u16::to_le_bytes));
            let hits = bytes
                .windows(marker.len())
                .enumerate()
                .filter_map(|(i, b)| (b == marker).then_some(i))
                .collect::<Vec<_>>();
            if hits.len() != 1 {
                return Err(fail());
            }
            let start = hits[0] + marker.len();
            if bytes.get(start..start + 12)
                != Some(
                    [
                        class.to_le_bytes(),
                        1_u32.to_le_bytes(),
                        4_u32.to_le_bytes(),
                    ]
                    .concat()
                    .as_slice(),
                )
            {
                return Err(fail());
            }
            bytes[start..start + 4].copy_from_slice(&1_u32.to_le_bytes());
            bytes[start + 8..start + 12].fill(0);
        }
        if self.xgk_auto_allocation() { declarations::normalize_xgk_types(&mut bytes)?; }
        let offsets = declarations::normalize(&mut bytes)?;
        table.text = base64::engine::general_purpose::STANDARD.encode(bytes);
        for a in &mut table.attributes {
            if a.name == "Compressed" {
                a.value = "0".into();
            }
        }
        let mut symbols = crate::IecLocalSymbol::from_symbols_element(&table)?;
        for symbol in &mut symbols {
            symbol.record_offset = *offsets.get(&symbol.name).ok_or_else(fail)?;
        }
        Ok(symbols)
    }
}
#[cfg(feature = "write")]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct SfcVariablePatch {
    pub program_index: usize,
    pub expected_variables: Vec<SfcVariable>,
    pub name: String,
    pub data_type: String,
    pub description: String,
    pub remove: bool,
    #[cfg_attr(feature = "wasm", serde(default))]
    pub declaration: Option<SfcDeclaration>,
    #[cfg_attr(feature = "wasm", serde(default))]
    pub update: bool,
}
#[cfg(feature = "write")]
impl XgwxDocument {
    /// Insert, update, or remove captured program-local SFC declarations.
    pub fn edit_sfc_variable(&mut self, patch: &SfcVariablePatch) -> Result<(), crate::XgwxError> {
        let fail = |s: &str| crate::XgwxError::SfcEdit(s.into());
        if self.sfc_variables(patch.program_index)? != patch.expected_variables {
            return Err(fail("stale SFC variable table"));
        }
        if patch.update && patch.remove {
            return Err(fail("choose update or remove"));
        }
        let name = &patch.name;
        if name.is_empty()
            || name.len() > 32
            || !name
                .bytes()
                .enumerate()
                .all(|(i, c)| c == b'_' || c.is_ascii_alphabetic() || i > 0 && c.is_ascii_digit())
            || ["TRANS", "GOTO_INIT"]
                .iter()
                .any(|n| n.eq_ignore_ascii_case(name))
        {
            return Err(fail(
                "use a unique variable identifier of at most 32 characters",
            ));
        }
        let symbols = self.sfc_symbols(patch.program_index)?;
        let xml = roxmltree::Document::parse(&self.xml).map_err(crate::XgwxError::Xml)?;
        let program = xml
            .descendants()
            .filter(|n| n.has_tag_name("Program"))
            .nth(patch.program_index)
            .unwrap();
        let table = program
            .descendants()
            .find(|n| n.has_tag_name("LocalVar"))
            .and_then(|n| n.descendants().find(|n| n.has_tag_name("Symbols")))
            .ok_or_else(|| fail("SFC symbols are absent"))?;
        let mut bytes = crate::decode_base64_payload(
            table.text().unwrap_or_default(),
            table.attribute("Compressed") == Some("1"),
        )?
        .data;
        if patch.remove {
            let index = symbols
                .iter()
                .position(|s| s.name == *name)
                .ok_or_else(|| fail("variable is absent"))?;
            if self.sfc_variable_referenced(patch.program_index, name)? {
                return Err(fail("variable is used by the chart or ST source"));
            }
            bytes.drain(
                symbols[index].record_offset
                    ..symbols
                        .get(index + 1)
                        .map_or(bytes.len(), |s| s.record_offset),
            );
        } else {
            if !patch.update && symbols.iter().any(|s| s.name.eq_ignore_ascii_case(name)) {
                return Err(fail("variable name is already declared"));
            }
            if patch.description.encode_utf16().count() > 255
                || patch.description.chars().any(char::is_control)
            {
                return Err(fail(
                    "variable description is too long or contains control characters",
                ));
            }
            let record = if self.xgk_auto_allocation() { declarations::xgk_record(patch)? } else { declarations::record(patch)? };
            let offset = symbols
                .iter()
                .find(|s| s.name.to_lowercase() > name.to_lowercase())
                .map_or(bytes.len(), |s| s.record_offset);
            if patch.update {
                let index = symbols
                    .iter()
                    .position(|s| s.name == *name)
                    .ok_or_else(|| fail("variable is absent"))?;
                if symbols[index].address.is_some() || !symbols[index].storage_class.is_empty() {
                    return Err(fail("mapped declarations are read only"));
                }
                let before = patch
                    .expected_variables
                    .iter()
                    .find(|v| v.name == *name)
                    .unwrap();
                let old = before.declaration.clone().unwrap_or_default();
                let new = patch.declaration.clone().unwrap_or_default();
                if before.data_type != patch.data_type || old.dimensions != new.dimensions {
                    // Type changes are bounded to unused declarations; initial values,
                    // retention and descriptions may change without rewriting ST.
                    if self.sfc_variable_referenced(patch.program_index, name)? {
                        return Err(fail(
                            "cannot change the type or bounds of a referenced declaration",
                        ));
                    }
                }
                let start = symbols[index].record_offset;
                let end = symbols
                    .get(index + 1)
                    .map_or(bytes.len(), |s| s.record_offset);
                bytes.splice(start..end, record);
            } else {
                bytes.splice(offset..offset, record);
            }
        }
        let count = table.attributes().find(|a| a.name() == "Count").ok_or_else(|| fail("symbol count is absent"))?;
        let text = table.children().find(|n| n.is_text());
        use base64::Engine;
        let encoded = if table.attribute("Compressed") == Some("1") {
            encode_sfc_payload(&bytes)
        } else {
            base64::engine::general_purpose::STANDARD.encode(&bytes)
        };
        let new_count = if patch.remove { symbols.len() - 1 } else { symbols.len() + usize::from(!patch.update) };
        let replacements = if let Some(text) = text {
            vec![(count.range_value(), new_count.to_string()), (text.range(), encoded)]
        } else if symbols.is_empty() && table.attributes().len() == 1 && table.children().next().is_none() {
            vec![(table.range(), format!("<Symbols Count=\"{new_count}\" dt:dt=\"bin.base64\" xmlns:dt=\"urn:schemas-microsoft-com:datatypes\" Compressed=\"0\">{encoded}</Symbols>"))]
        } else { return Err(fail("symbol payload is absent")); };
        let mut candidate = self.clone();
        candidate.apply_xml_replacements(replacements)?;
        candidate.sfc_variables(patch.program_index)?;
        candidate.to_verified_bytes()?;
        *self = candidate;
        Ok(())
    }
    fn sfc_variable_referenced(&self, index: usize, name: &str) -> Result<bool, crate::XgwxError> {
        let fail =
            || crate::XgwxError::SfcEdit("cannot check references in this program language".into());
        if let Some(text) = self.text_programs().into_iter().find(|p| p.program_index == index) {
            if !text.editable { return Err(fail()); }
            let source = text.source.ok_or_else(fail)?;
            return Ok(source.split(|c: char| !c.is_alphanumeric() && c != '_').any(|v| v.eq_ignore_ascii_case(name)));
        }
        let sfc = self
            .sfc_programs()
            .into_iter()
            .find(|p| p.program_index == index)
            .ok_or_else(fail)?;
        if sfc.blocks.iter().any(|b| {
            b.entities.iter().any(|e| {
                e.properties
                    .values()
                    .any(|p| p.values().any(|v| v.eq_ignore_ascii_case(name)))
            })
        }) {
            return Ok(true);
        }
        let program = self
            .root
            .descendants_named("Program")
            .nth(index)
            .ok_or_else(fail)?;
        for block in program
            .descendants_named("SFC_ProgramProperty")
            .filter(|b| b.attribute("MainBlock") == Some("0"))
        {
            let source = read_st_source(block).ok_or_else(fail)?;
            if source
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .any(|v| v.eq_ignore_ascii_case(name))
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
