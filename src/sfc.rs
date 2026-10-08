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
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
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
                        SfcBlock {
                            block_index,
                            name: block.attribute("PragramName").unwrap_or("").to_owned(),
                            main: block.attribute("MainBlock") == Some("1"),
                            language_type: number(block, "LanguageType"),
                            language: number(block, "Language"),
                            rows: grid.and_then(|g| number(g, "RowSize")).unwrap_or(0),
                            columns: grid.and_then(|g| number(g, "ColSize")).unwrap_or(0),
                            entities,
                        }
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
