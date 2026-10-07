use crate::*;
use std::collections::{HashMap, HashSet};

struct IoTemplate {
    suffix: &'static str,
    bit: u32,
    direction: &'static str,
    description: &'static str,
}
struct IoSpec {
    id: u32,
    points: u32,
    variables: &'static [IoTemplate],
}
#[path = "io_variable_data.rs"]
mod data;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct IoVariableGenerationRow {
    pub base: u32,
    pub slot: u32,
    pub model: String,
    pub name: String,
    pub address: String,
    pub address_number: u32,
    pub data_type: String,
    pub description: String,
    pub source_ref: String,
    pub action: String,
    pub existing_names: Vec<String>,
    pub(crate) duplicate_indices: Vec<usize>,
}

fn invalid(reason: &'static str) -> XgwxError {
    XgwxError::InvalidLadderEdit { reason }
}

pub(crate) fn plan(doc: &XgwxDocument) -> Result<Vec<IoVariableGenerationRow>, XgwxError> {
    let configurations = doc.configurations();
    if configurations.len() != 1
        || configurations[0]
            .type_code
            .and_then(crate::cpu::cpu_for_type)
            .is_none_or(|c| c.family != "XGK")
    {
        return Err(invalid(
            "I/O variable generation supports XGK global symbols",
        ));
    }
    let basic = doc
        .root
        .descendants_named("XGTBasicParam")
        .next()
        .ok_or_else(|| invalid("missing I/O allocation settings"))?;
    if basic.attribute("IO_POINT_METHOD") != Some("0") {
        return Err(invalid(
            "I/O variable generation requires variable point allocation",
        ));
    }
    let modules = doc.modules();
    let mut positions = HashMap::new();
    for module in &modules {
        let key = module
            .base
            .zip(module.slot)
            .ok_or_else(|| invalid("module position is missing"))?;
        if positions.insert(key, module).is_some() {
            return Err(invalid("duplicate module position"));
        }
    }
    let existing = doc.variables()?;
    let mut bases = doc.bases();
    bases.sort_by_key(|b| b.base);
    let mut seen_bases = HashSet::new();
    let mut address = 0u32;
    let mut rows = Vec::new();
    for base in bases {
        let index = base
            .base
            .ok_or_else(|| invalid("base position is missing"))?;
        let slots = base
            .slot_count
            .filter(|n| [4, 6, 8, 10, 12].contains(n))
            .ok_or_else(|| invalid("unsupported base slot count"))?;
        if !seen_bases.insert(index) {
            return Err(invalid("duplicate base position"));
        }
        for slot in 0..slots {
            let mut points = 16;
            if let Some(module) = positions.get(&(index, slot)) {
                let entry = crate::xgk_module_catalog()
                    .iter()
                    .find(|entry| {
                        Some(entry.id) == module.id && Some(entry.sub_type) == module.sub_type
                    })
                    .ok_or_else(|| invalid("unknown module prevents I/O address allocation"))?;
                if entry.slot_span != 1 {
                    return Err(invalid("multi-slot module I/O allocation is not verified"));
                }
                if let Some(spec) = data::SPECS
                    .iter()
                    .find(|s| Some(s.id) == module.id && module.sub_type == Some(0))
                {
                    points = spec.points;
                    for template in spec.variables {
                        let number = address
                            .checked_add(template.bit)
                            .ok_or_else(|| invalid("I/O address overflow"))?;
                        let name = format!("_{index:02}{slot:02}_{}", template.suffix);
                        let source_ref =
                            format!("IO:{index}:{slot}:{}:{}", template.bit, template.direction);
                        let duplicates = existing
                            .iter()
                            .enumerate()
                            .filter(|(_, v)| {
                                v.name
                                    .as_deref()
                                    .is_some_and(|n| n.eq_ignore_ascii_case(&name))
                                    || v.source_ref.as_deref() == Some(&source_ref)
                                    || variable_covers(v, number)
                            })
                            .map(|(i, _)| i)
                            .collect::<Vec<_>>();
                        let unchanged = duplicates.len() == 1
                            && existing[duplicates[0]].name.as_deref() == Some(&name)
                            && existing[duplicates[0]].address_area.as_deref() == Some("P")
                            && existing[duplicates[0]].address_number == Some(number)
                            && existing[duplicates[0]].data_type.as_deref() == Some("BIT")
                            && existing[duplicates[0]].description.as_deref()
                                == Some(template.description)
                            && existing[duplicates[0]].source_ref.as_deref() == Some(&source_ref);
                        rows.push(IoVariableGenerationRow {
                            base: index,
                            slot,
                            model: entry.model.into(),
                            name,
                            address: format!("P{:04}{:X}", number / 16, number % 16),
                            address_number: number,
                            data_type: "BIT".into(),
                            description: template.description.into(),
                            source_ref,
                            action: if unchanged {
                                "unchanged"
                            } else if duplicates.is_empty() {
                                "create"
                            } else {
                                "overwrite"
                            }
                            .into(),
                            existing_names: duplicates
                                .iter()
                                .filter_map(|&i| existing[i].name.clone())
                                .collect(),
                            duplicate_indices: duplicates,
                        });
                    }
                } else if ["입력 모듈", "출력 모듈", "입출력 모듈"].contains(&entry.category)
                {
                    return Err(invalid("digital module I/O template is not yet verified"));
                }
            }
            address = address
                .checked_add(points)
                .ok_or_else(|| invalid("I/O address overflow"))?;
        }
    }
    if modules.iter().any(|m| {
        !seen_bases.contains(&m.base.unwrap_or(u32::MAX))
            || m.slot
                .zip(
                    doc.bases()
                        .iter()
                        .find(|b| b.base == m.base)
                        .and_then(|b| b.slot_count),
                )
                .is_none_or(|(s, n)| s >= n)
    }) {
        return Err(invalid("module is outside its configured base"));
    }
    Ok(rows)
}

fn variable_covers(v: &VariableSummary, bit: u32) -> bool {
    if v.address_area.as_deref() != Some("P") {
        return false;
    }
    let Some(number) = v.address_number else {
        return false;
    };
    let Some(kind) = v.data_type.as_deref() else {
        return false;
    };
    let width = match kind {
        "BIT" => 1,
        "BYTE" => 8,
        "WORD" => 16,
        "DWORD" => 32,
        "LWORD" => 64,
        _ => return false,
    };
    let start = if width == 1 {
        number
    } else {
        number.saturating_mul(16)
    };
    bit >= start && bit < start.saturating_add(width)
}

fn string(out: &mut Vec<u8>, text: &str) {
    let count = text.encode_utf16().count();
    out.extend_from_slice(&[255, 254, 255, count as u8]);
    for unit in text.encode_utf16() {
        out.extend_from_slice(&unit.to_le_bytes());
    }
}

pub(crate) fn symbol_record(row: &IoVariableGenerationRow) -> Vec<u8> {
    let mut out = Vec::new();
    for value in ["SV5.0", &row.name, "P"] {
        string(&mut out, value);
    }
    out.extend_from_slice(&row.address_number.to_le_bytes());
    for value in ["BIT", &row.description, &row.source_ref] {
        string(&mut out, value);
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    string(&mut out, "[0:0]");
    out
}
