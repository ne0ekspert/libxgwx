use crate::{XgwxDocument, XmlElement};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct SfcProgram {
    pub program_index: usize,
    pub blocks: Vec<SfcBlock>,
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
                        if list.descendants_named("SFC_ProgramProperty").count() == 1
                            && linear_container_supported(block)
                        {
                            result.editable_rows = result.linear_rows();
                        }
                        result
                    })
                    .collect();
                Some(SfcProgram {
                    program_index,
                    blocks,
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

/// One row in the captured two-column, linear SFC layout.
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
}

impl SfcBlock {
    fn linear_rows(&self) -> Option<Vec<SfcRow>> {
        if !self.main || self.language_type != Some(3) || self.language != Some(2) {
            return None;
        }
        if self.entities.is_empty() {
            return (self.rows == 0 && self.columns == 0).then(Vec::new);
        }
        if self.rows > 512 || self.columns != 2 || self.entities.len() != self.rows as usize * 2 {
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
            if e.type_code == Some(10) {
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
            if ["Bookmark", "BreakPoint", "PropertyProgram", "StepVariable"]
                .iter()
                .any(|k| p[*k] != "0")
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
                _ => return None,
            };
            let p = &e.properties["EntityStep"];
            let adjacent = positions.get(&(row, 1))?;
            let action = if kind == "step" {
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
                kind: kind.into(),
                title: p["Title"].clone(),
                comment: p["Comment"].clone(),
                initial: p["InitialStep"] == "1",
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
        validate_rows(&rows).ok()?;
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
                .all(|(i, c)| !e.children[..i].iter().any(|n| n.name == c.name))
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

fn validate_rows(rows: &[SfcRow]) -> Result<(), crate::XgwxError> {
    let fail = |s: &str| crate::XgwxError::SfcEdit(s.into());
    if rows.len() > 512 {
        return Err(fail("linear charts support at most 512 rows"));
    }
    let identifier = |s: &str| {
        !s.is_empty()
            && s.len() <= 32
            && s.bytes()
                .enumerate()
                .all(|(i, c)| c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))
    };
    let bool_address = |s: &str| {
        s.strip_prefix("%MX").is_some_and(|n| {
            !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()) && n.parse::<u16>().is_ok()
        })
    };
    let mut steps = std::collections::BTreeSet::new();
    let mut labels = std::collections::BTreeSet::new();
    let mut initial_count = 0;
    for row in rows {
        if row.comment.len() > 65536 {
            return Err(fail("comment is too long"));
        }
        match row.kind.as_str() {
            "step" => {
                if !identifier(&row.title) || !steps.insert(&row.title) {
                    return Err(fail(
                        "step names must be unique identifiers of at most 32 characters",
                    ));
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
                if row.action.as_deref().is_some_and(|a| !bool_address(a)) {
                    return Err(fail("actions require a direct %MX BOOL address"));
                }
            }
            "transition" if bool_address(&row.title) => {}
            "label" if identifier(&row.title) && labels.insert(&row.title) => {}
            "jump" if identifier(&row.title) => {}
            _ => {
                return Err(fail(
                    "use unique label identifiers and direct %MX BOOL transitions",
                ));
            }
        }
        if row.action.is_none() && (row.action_qualifier.is_some() || row.action_time.is_some()) {
            return Err(fail("action properties require an action operand"));
        }
        if row.kind != "step" && (row.initial || row.action.is_some() || !row.comment.is_empty()) {
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
        if block.entities != patch.expected_entities {
            return Err(fail("stale SFC chart"));
        }
        if block.editable_rows.is_none() {
            return Err(fail(
                "only captured linear main charts with direct BOOL operands and supported actions support structural editing",
            ));
        }
        validate_rows(&patch.rows)?;
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
            if kind != 10 {
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
                _ => 6,
            };
            grid_xml.push_str(&entity(kind, row, 0, &r.title, &r.comment, r.initial));
        }
        for (row, r) in patch.rows.iter().enumerate() {
            let (kind, title) = if r.kind == "step" {
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
            grid_xml.push_str(&action_xml);
        }
        grid_xml.push_str("</EntityPropertyList></EntityGrid>");
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
        let mut candidate = self.clone();
        candidate.apply_xml_replacements(replacements)?;
        candidate.to_verified_bytes()?;
        *self = candidate;
        Ok(())
    }
}
