//! Captured IEC program-local `PB50` symbol table records.
use crate::*;

/// Primitive IEC type IDs captured from XG5000's ordered local-variable
/// selector. BOOL, INT, UDINT, TIME, WORD and DINT also occur in native files.
pub const IEC_PRIMITIVE_TYPES: &[(&str, u32)] = &[
    ("BOOL", 1),
    ("BYTE", 2),
    ("WORD", 3),
    ("DWORD", 4),
    ("LWORD", 5),
    ("SINT", 6),
    ("INT", 7),
    ("DINT", 8),
    ("LINT", 9),
    ("USINT", 10),
    ("UINT", 11),
    ("UDINT", 12),
    ("ULINT", 13),
    ("REAL", 14),
    ("LREAL", 15),
    ("TIME", 16),
    ("DATE", 17),
    ("TIME_OF_DAY", 18),
    ("DATE_AND_TIME", 19),
];

pub fn iec_primitive_type_name(code: u32) -> Option<&'static str> {
    IEC_PRIMITIVE_TYPES
        .iter()
        .find_map(|&(name, id)| (id == code).then_some(name))
}

pub fn iec_primitive_type_code(name: &str) -> Option<u32> {
    IEC_PRIMITIVE_TYPES
        .iter()
        .find_map(|&(label, id)| (label == name).then_some(id))
}

/// One symbol stored under a program's `<LocalVar><Symbols>` payload.
/// The binary type metadata for automatic variables is still opaque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IecLocalSymbol {
    pub record_offset: usize,
    pub name: String,
    pub address: Option<String>,
    pub type_reference: Option<String>,
    pub data_type_code: u32,
    pub data_type: Option<String>,
    pub storage_class: String,
    pub allocation_number: Option<u32>,
    pub allocation_width: Option<u32>,
    pub description: Option<String>,
    pub is_instance: bool,
}

impl IecLocalSymbol {
    pub(crate) fn from_symbols_element(symbols: &XmlElement) -> Result<Vec<Self>, XgwxError> {
        let expected_count = attr_u32(symbols, "Count")
            .ok_or(XgwxError::InvalidVariableRecord { index: 0 })?
            as usize;
        let compressed = attr_bool(symbols, "Compressed").unwrap_or(false);
        if expected_count == 0 {
            return if symbols.text.trim().is_empty()
                || decode_base64_payload(&symbols.text, compressed)?
                    .data
                    .is_empty()
            {
                Ok(Vec::new())
            } else {
                Err(XgwxError::InvalidVariableRecord { index: 0 })
            };
        }
        let data = decode_base64_payload(&symbols.text, compressed)?.data;
        let strings = extract_utf16_marker_strings(&data, false, true);
        let starts = strings
            .iter()
            .enumerate()
            .filter_map(|(index, item)| (item.value == "PB50").then_some(index))
            .collect::<Vec<_>>();
        if starts.len() != expected_count
            || starts.first() != Some(&0)
            || strings.first().is_none_or(|item| item.offset != 0)
            || strings
                .last()
                .is_none_or(|item| item.end_offset != data.len())
        {
            return Err(XgwxError::InvalidVariableRecord { index: 0 });
        }
        let mut symbols_out = Vec::with_capacity(expected_count);
        for (record_index, &start) in starts.iter().enumerate() {
            let end = starts
                .get(record_index + 1)
                .copied()
                .unwrap_or(strings.len());
            let fields = &strings[start..end];
            if !matches!(fields.len(), 8 | 9) || fields[1].value.is_empty() {
                return Err(XgwxError::InvalidVariableRecord {
                    index: record_index,
                });
            }
            let is_instance = fields.len() == 9;
            let type_header = data.get(fields[1].end_offset..fields[2].offset).ok_or(
                XgwxError::InvalidVariableRecord {
                    index: record_index,
                },
            )?;
            if type_header.len() != if is_instance { 8 } else { 12 }
                || type_header[..4] != 1_u32.to_le_bytes()
                || (!is_instance && type_header[8..12] != [0; 4])
            {
                return Err(XgwxError::InvalidVariableRecord {
                    index: record_index,
                });
            }
            let data_type_code = u32::from_le_bytes(type_header[4..8].try_into().unwrap());
            let class_index = fields.len() - 3;
            let storage_class = fields[class_index].value.as_str();
            if !matches!(storage_class, "" | "A" | "M" | "I" | "Q") {
                return Err(XgwxError::InvalidVariableRecord {
                    index: record_index,
                });
            }
            let allocation = data
                .get(fields[class_index].end_offset..fields[class_index + 1].offset)
                .filter(|bytes| bytes.len() == 36)
                .ok_or(XgwxError::InvalidVariableRecord {
                    index: record_index,
                })?;
            let allocation_number = u32::from_le_bytes(allocation[..4].try_into().unwrap());
            let allocation_width = u32::from_le_bytes(allocation[4..8].try_into().unwrap());
            if storage_class.is_empty() && (allocation_number != u32::MAX || allocation_width != 0)
            {
                return Err(XgwxError::InvalidVariableRecord {
                    index: record_index,
                });
            }
            let type_or_address = fields[2].value.as_str();
            let address = type_or_address
                .starts_with('%')
                .then(|| type_or_address.to_owned());
            let type_reference =
                (is_instance && !type_or_address.is_empty()).then(|| type_or_address.to_owned());
            let description = fields[class_index - 1].value.as_str();
            symbols_out.push(Self {
                record_offset: fields[0].offset,
                name: fields[1].value.clone(),
                address,
                type_reference,
                data_type_code,
                data_type: iec_primitive_type_name(data_type_code).map(str::to_owned),
                storage_class: storage_class.to_owned(),
                allocation_number: (allocation_number != u32::MAX).then_some(allocation_number),
                allocation_width: (allocation_width != 0).then_some(allocation_width),
                description: (!description.is_empty()).then(|| description.to_owned()),
                is_instance,
            });
        }
        Ok(symbols_out)
    }
}
