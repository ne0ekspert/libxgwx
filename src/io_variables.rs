use crate::*;
use std::collections::HashMap;
mod collisions;

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

/// One proposed XGK digital I/O symbol from the guarded generation plan.
/// A preview describes whether the writer will create, overwrite, or retain a symbol
/// without modifying the document.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct IoVariableGenerationRow {
    /// Current rack/base number owning the digital module.
    pub base: u32,
    /// Current module slot within the rack/base.
    pub slot: u32,
    /// Catalog model name of the supported digital I/O module.
    pub model: String,
    /// Generated symbol name derived from base, slot, and I/O point.
    pub name: String,
    /// Formatted XGK P-area bit address.
    pub address: String,
    /// Numeric P-area bit address used by the symbol record.
    pub address_number: u32,
    /// Generated native symbol type; currently `BIT`.
    pub data_type: String,
    /// Description supplied by the validated I/O point template.
    pub description: String,
    /// Generation identity in the form `IO:base:slot:bit:direction`.
    pub source_ref: String,
    /// Planned action: `create`, `overwrite`, or `unchanged`.
    pub action: String,
    /// Existing symbol names selected by name, source reference, or overlapping address.
    pub existing_names: Vec<String>,
    pub(crate) duplicate_indices: Vec<usize>,
}

fn invalid(reason: &'static str) -> XgwxError {
    XgwxError::InvalidLadderEdit { reason }
}

const MAX_BASE_RECORDS: usize = 8;
const MAX_PLAN_ROWS: usize = MAX_BASE_RECORDS * 12 * 64;

fn limit(value: usize, maximum: usize, resource: &'static str) -> Result<(), XgwxError> {
    if value > maximum {
        Err(XgwxError::ResourceLimitExceeded {
            resource,
            limit: maximum,
        })
    } else {
        Ok(())
    }
}

struct PlannedModule {
    base: u32,
    slot: u32,
    address: u32,
    model: &'static str,
    spec: &'static IoSpec,
}

fn bounded_bases(doc: &XgwxDocument) -> Result<Vec<BaseSummary>, XgwxError> {
    let elements: Vec<_> = doc
        .root
        .descendants_named("Base")
        .take(MAX_BASE_RECORDS + 1)
        .collect();
    limit(elements.len(), MAX_BASE_RECORDS, "I/O generation bases")?;
    Ok(elements
        .into_iter()
        .map(BaseSummary::from_element)
        .collect())
}

pub(crate) fn plan(doc: &XgwxDocument) -> Result<Vec<IoVariableGenerationRow>, XgwxError> {
    let configurations: Vec<_> = doc
        .root
        .descendants_named("Configuration")
        .take(2)
        .collect();
    let cpu = match configurations.as_slice() {
        [element] => crate::internal::attr_u32(element, "Type")
            .and_then(crate::cpu::cpu_for_type)
            .filter(|cpu| cpu.family == crate::CpuFamily::Xgk),
        _ => None,
    }
    .ok_or_else(|| invalid("I/O variable generation supports XGK global symbols"))?;
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
    // Native templates retain eight base records even for smaller CPUs. Keep
    // those empty reservations, but enforce CPU limits on populated positions.
    // Validate and cache geometry before generating rows or decoding symbols.
    let mut bases = bounded_bases(doc)?;
    bases.sort_by_key(|base| base.base);
    let mut capacities = HashMap::new();
    for base in &bases {
        let index = base
            .base
            .ok_or_else(|| invalid("base position is missing"))?;
        let slots = base
            .slot_count
            .filter(|n| [4, 6, 8, 10, 12].contains(n) && *n <= cpu.max_slot)
            .ok_or_else(|| invalid("unsupported base slot count"))?;
        if index >= MAX_BASE_RECORDS as u32 {
            return Err(invalid("base exceeds the supported rack range"));
        }
        if capacities.insert(index, slots).is_some() {
            return Err(invalid("duplicate base position"));
        }
    }
    let max_modules = (cpu.max_base * cpu.max_slot) as usize;
    let module_elements: Vec<_> = doc
        .root
        .descendants_named("Module")
        .take(max_modules + 1)
        .collect();
    limit(module_elements.len(), max_modules, "I/O generation modules")?;
    let modules: Vec<_> = module_elements
        .into_iter()
        .map(ModuleSummary::from_element)
        .collect();
    let mut positions = HashMap::new();
    for module in &modules {
        let key = module
            .base
            .zip(module.slot)
            .ok_or_else(|| invalid("module position is missing"))?;
        if positions.insert(key, module).is_some() {
            return Err(invalid("duplicate module position"));
        }
        if key.0 >= cpu.max_base || key.1 >= cpu.max_slot {
            return Err(XgwxError::CpuHardwareLimit {
                model: cpu.model.into(),
                base: key.0,
                slot: key.1,
            });
        }
        if capacities.get(&key.0).is_none_or(|slots| key.1 >= *slots) {
            return Err(invalid("module is outside its configured base"));
        }
    }
    let mut address = 0u32;
    let mut planned = Vec::new();
    let mut row_count = 0usize;
    for base in &bases {
        let index = base.base.unwrap();
        for slot in 0..capacities[&index] {
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
                    row_count = row_count.saturating_add(spec.variables.len());
                    limit(row_count, MAX_PLAN_ROWS, "I/O generation rows")?;
                    planned.push(PlannedModule {
                        base: index,
                        slot,
                        address,
                        model: entry.model,
                        spec,
                    });
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
    let existing = doc.variables()?;
    let index = collisions::Index::new(&existing);
    let mut budget = collisions::Budget::default();
    let mut rows = Vec::with_capacity(row_count);
    for module in planned {
        for template in module.spec.variables {
            let number = module
                .address
                .checked_add(template.bit)
                .ok_or_else(|| invalid("I/O address overflow"))?;
            let base = module.base;
            let slot = module.slot;
            let name = format!("_{base:02}{slot:02}_{}", template.suffix);
            let source_ref = format!("IO:{base}:{slot}:{}:{}", template.bit, template.direction);
            let duplicates = index.matching(&name, &source_ref, number, &mut budget)?;
            let unchanged = duplicates.len() == 1
                && existing[duplicates[0]].name.as_deref() == Some(&name)
                && existing[duplicates[0]].address_area.as_deref() == Some("P")
                && existing[duplicates[0]].address_number == Some(number)
                && existing[duplicates[0]].data_type.as_deref() == Some("BIT")
                && existing[duplicates[0]].description.as_deref() == Some(template.description)
                && existing[duplicates[0]].source_ref.as_deref() == Some(&source_ref);
            rows.push(IoVariableGenerationRow {
                base,
                slot,
                model: module.model.into(),
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
    }
    Ok(rows)
}

#[cfg(test)]
fn variable_covers(v: &VariableSummary, bit: u32) -> bool {
    #[cfg(test)]
    COVER_CHECKS.with(|count| count.set(count.get() + 1));
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

#[cfg(test)]
std::thread_local! {
    static COVER_CHECKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planning_indexes_existing_symbols_once() {
        use base64::Engine;
        use std::io::Write;
        let mut payload = Vec::new();
        for index in 0..256 {
            payload.extend(symbol_record(&IoVariableGenerationRow {
                base: 0,
                slot: 0,
                model: String::new(),
                name: format!("Existing{index}"),
                address: String::new(),
                address_number: 100000,
                data_type: "BIT".into(),
                description: String::new(),
                source_ref: format!("External:{index}"),
                action: String::new(),
                existing_names: vec![],
                duplicate_indices: vec![],
            }));
        }
        let xml = format!(
            "<Project><Configuration Type='0'><XGTBasicParam IO_POINT_METHOD='0'/><Base Base='0' SlotCount='12'><Module Id='42243' SubType='0' Base='0' Slot='0'/></Base></Configuration><Symbols Count='256'>{}</Symbols></Project>",
            base64::engine::general_purpose::STANDARD.encode(payload)
        );
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gzip.write_all(xml.as_bytes()).unwrap();
        let mut bytes = b"XG".to_vec();
        bytes.extend(gzip.finish().unwrap());
        let doc = XgwxDocument::parse(&bytes).unwrap();
        COVER_CHECKS.with(|count| count.set(0));
        let start = std::time::Instant::now();
        let rows = plan(&doc).unwrap();
        let checks = COVER_CHECKS.with(|count| count.get());
        eprintln!(
            "64 rows / 256 existing symbols: {checks} coverage checks in {:?}",
            start.elapsed()
        );
        assert_eq!(rows.len(), 64);
        assert!(rows.iter().all(|row| row.action == "create"));
        assert!(
            checks <= 256 + rows.len(),
            "existing symbols must not be rescanned for every row"
        );
    }
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

// Preserve full-project allocation before selecting rows; scope must not renumber addresses.
pub(crate) fn plan_for(
    doc: &XgwxDocument,
    base: Option<u32>,
    slot: Option<u32>,
) -> Result<Vec<IoVariableGenerationRow>, XgwxError> {
    let bases = bounded_bases(doc)?;
    if base.is_some_and(|selected| !bases.iter().any(|b| b.base == Some(selected))) {
        return Err(invalid("selected I/O base does not exist"));
    }
    if slot.is_some_and(|selected| {
        !bases.iter().any(|b| {
            base.is_none_or(|wanted| b.base == Some(wanted))
                && b.slot_count.is_some_and(|count| selected < count)
        })
    }) {
        return Err(invalid("selected I/O slot is outside the base capacity"));
    }
    Ok(plan(doc)?
        .into_iter()
        .filter(|row| base.is_none_or(|b| row.base == b) && slot.is_none_or(|s| row.slot == s))
        .collect())
}
