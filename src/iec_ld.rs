//! Captured IEC LD text records in `ProjectType=2` program payloads.
//!
//! XG5000 stores these comments in `FF 3F` records immediately followed by a
//! marker-prefixed UTF-16 string. Element record markers likewise identify a
//! subset of contact and coil operands without including function pin labels.
use crate::{IecRecordKind, LadderProgramData, LadderString};

const COMMENT_PREFIX: [u8; 6] = [0xff, 0x3f, 0, 0, 0, 1];
const FUNCTION_OPERAND_PREFIX: [u8; 2] = [0xff, 0x46];
const STRING_OFFSET_FROM_RECORD: usize = 15;

/// One stored IEC LD row envelope. `records_start..end` contains its records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecRowFrame {
    pub group_index: usize,
    pub row_index: u16,
    pub start: usize,
    pub records_start: usize,
    pub end: usize,
    pub record_count: u16,
}

/// Validated native long and three-grid-unit wire records and paired vertical
/// branches. Use `LadderProgramData::iec_circuit_graph()` to combine these
/// segments with elements and typed function-pin bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IecGeometry {
    pub horizontal: Vec<IecHorizontalSegment>,
    pub vertical: Vec<IecVerticalConnection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecHorizontalSegment {
    pub group_index: usize,
    pub row_index: u16,
    pub offset: usize,
    pub start_x: u8,
    pub end_x: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecVerticalConnection {
    pub group_index: usize,
    pub start_row_index: u16,
    pub end_row_index: u16,
    pub start_offset: usize,
    pub end_offset: usize,
    pub x: u8,
}

impl LadderProgramData {
    /// Locate IEC LD group and row envelopes when the entire header sequence
    /// matches the captured format. Returns `None` for other program layouts.
    pub fn iec_row_frames(&self) -> Option<Vec<IecRowFrame>> {
        row_frames(self)
    }

    /// Decode captured horizontal wires and paired vertical branches.
    pub fn iec_geometry(&self) -> Option<IecGeometry> {
        geometry(self)
    }
}

fn geometry(program: &LadderProgramData) -> Option<IecGeometry> {
    let rows = row_frames(program)?;
    let record_kinds = program
        .iec_record_frames()?
        .into_iter()
        .map(|record| (record.offset, record.kind))
        .collect::<std::collections::HashMap<_, _>>();
    let data = &program.data;
    let mut horizontal = Vec::new();
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    for row in &rows {
        let y = row.row_index * 4;
        for offset in row.records_start..row.end.saturating_sub(14) {
            let Some(kind) = record_kinds.get(&offset) else {
                continue;
            };
            let segment = match kind {
                IecRecordKind::ShortWire => {
                    let bytes = data.get(offset..offset + 15)?;
                    if bytes[..5] != [0xff, 0x01, 0, 0, 0]
                        || !(1..=91).contains(&bytes[5])
                        || bytes[6..8] != y.to_le_bytes()
                        || bytes[8] != 0
                        || !matches!(&bytes[9..15], [0, 0, 0, 0, 0, 0] | [0, 0, 4, 0, 0, 0])
                    {
                        return None;
                    }
                    Some((bytes[5], bytes[5] + 3))
                }
                IecRecordKind::LongWire => {
                    let bytes = data.get(offset..offset + 19)?;
                    let start_x = bytes[5];
                    let end_x = bytes[15];
                    if bytes[..5] != [0xff, 0x02, 0, 0, 0]
                        || !(1..=94).contains(&start_x)
                        || !(start_x..=94).contains(&end_x)
                        || bytes[6..8] != y.to_le_bytes()
                        || bytes[16..18] != y.to_le_bytes()
                        || bytes[8] != 0
                        || bytes[18] != 0
                        || !matches!(&bytes[9..15], [0, 0, 0, 0, 0, 0] | [0, 0, 4, 0, 0, 0])
                    {
                        return None;
                    }
                    Some((start_x, end_x))
                }
                _ => None,
            };
            if let Some((start_x, end_x)) = segment {
                horizontal.push(IecHorizontalSegment {
                    group_index: row.group_index,
                    row_index: row.row_index,
                    offset,
                    start_x,
                    end_x,
                });
            }
        }
        for offset in row.records_start..row.end.saturating_sub(26) {
            if record_kinds.get(&offset) != Some(&IecRecordKind::BranchStart) {
                continue;
            }
            let Some(bytes) = data.get(offset..offset + 27) else {
                continue;
            };
            if bytes[..7] != [0, 0, 0, 0, 0, 2, 0] {
                continue;
            }
            let x = bytes[7];
            let source_y = u16::from_le_bytes([bytes[8], bytes[9]]);
            let target_y = u16::from_le_bytes([bytes[18], bytes[19]]);
            if (1..=93).contains(&x)
                && x % 3 == 0
                && source_y == y
                && target_y > y
                && target_y % 4 == 0
                && matches!(
                    &bytes[10..17],
                    [0, 0, 0, 0, 0, 0, 0] | [0, 0, 0, 4, 0, 0, 0]
                )
                && bytes[17] == x - 1
                && matches!(
                    &bytes[20..27],
                    [0, 0, 0, 0, 0, 0, 0] | [0, 0, 0, 4, 0, 0, 0]
                )
            {
                starts.push((row.group_index, row.row_index, (target_y / 4), offset, x));
            }
        }
        for offset in row.records_start..row.end.saturating_sub(8) {
            if record_kinds.get(&offset) != Some(&IecRecordKind::BranchEnd) {
                continue;
            }
            let Some(bytes) = data.get(offset..offset + 9) else {
                continue;
            };
            if bytes[..5] != [1, 0, 0, 0, 0] {
                continue;
            }
            let x = bytes[5];
            let source_y = u16::from_le_bytes([bytes[6], bytes[7]]);
            if (1..=93).contains(&x)
                && x % 3 == 0
                && source_y < y
                && source_y % 4 == 0
                && bytes[8] == 0
            {
                ends.push((row.group_index, source_y / 4, row.row_index, offset, x));
            }
        }
    }
    let mut vertical = Vec::with_capacity(starts.len());
    let mut used_end_offsets = std::collections::HashSet::new();
    for (group_index, start_row_index, end_row_index, start_offset, x) in starts {
        let matches = ends
            .iter()
            .filter(|&&(group, start, end, _, end_x)| {
                (group, start, end, end_x) == (group_index, start_row_index, end_row_index, x)
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return None;
        }
        if !used_end_offsets.insert(matches[0].3) {
            return None;
        }
        vertical.push(IecVerticalConnection {
            group_index,
            start_row_index,
            end_row_index,
            start_offset,
            end_offset: matches[0].3,
            x,
        });
    }
    if used_end_offsets.len() != ends.len() {
        return None;
    }
    Some(IecGeometry {
        horizontal,
        vertical,
    })
}

fn row_frames(program: &LadderProgramData) -> Option<Vec<IecRowFrame>> {
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return None;
    }
    let data = &program.data;
    // XG5000 stores an untouched IEC ladder as just the zero-count header.
    if data.as_slice() == [0; 8] {
        return Some(Vec::new());
    }
    if data.len() < 43 || data.get(..4)? != [0; 4] {
        return None;
    }
    let max_rows = u16::from_le_bytes(data.get(4..6)?.try_into().ok()?) as usize;
    let group_count = u16::from_le_bytes(data.get(6..8)?.try_into().ok()?) as usize;
    if max_rows == 0 || group_count == 0 || group_count > max_rows {
        return None;
    }

    let mut rows = Vec::new();
    for start in 8..=data.len() - 35 {
        let prefix = &data[start..start + 35];
        if prefix[4..13] != [0xff, 0x43, 0, 0, 0, 0, 0, 0, 0] {
            continue;
        }
        let index = u32::from_le_bytes(prefix[..4].try_into().ok()?) as usize;
        if index >= max_rows || index > u16::MAX as usize / 4 {
            continue;
        }
        let y = (index * 4) as u16;
        let coordinate = |offset: usize| {
            prefix[offset] > 0
                && prefix[offset] <= 94
                && prefix[offset + 1..offset + 3] == y.to_le_bytes()
                && prefix[offset + 3] == 0
        };
        if prefix[13] > 1
            || prefix[14..17] != [0; 3]
            || prefix[21] != 94
            || !coordinate(21)
            || !coordinate(29)
        {
            continue;
        }
        rows.push((
            start,
            index as u16,
            u16::from_le_bytes([prefix[33], prefix[34]]),
        ));
    }
    if rows.is_empty()
        || rows.len() > max_rows
        || rows.windows(2).any(|pair| pair[0].1 >= pair[1].1)
    {
        return None;
    }

    let mut groups = vec![None; group_count];
    for &(start, _, _) in &rows {
        let Some(head_start) = start.checked_sub(10) else {
            continue;
        };
        let head = &data[head_start..start];
        let index = u32::from_le_bytes(head[..4].try_into().ok()?) as usize;
        let count = u16::from_le_bytes(head[8..10].try_into().ok()?) as usize;
        if index < group_count
            && matches!(&head[4..8], [0, 0, 0, 0] | [1, 0, 0, 0])
            && count > 0
            && count <= max_rows
            && groups[index].replace((head_start, start, count)).is_some()
        {
            return None;
        }
    }
    let groups = groups.into_iter().collect::<Option<Vec<_>>>()?;
    if groups[0].0 != 8 || groups.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
        return None;
    }

    let mut frames = Vec::with_capacity(rows.len());
    for (group_index, &(head_start, first_row, count)) in groups.iter().enumerate() {
        let group_end = groups
            .get(group_index + 1)
            .map_or(data.len(), |next| next.0);
        let selected = rows
            .iter()
            .copied()
            .filter(|(start, _, _)| *start >= first_row && *start < group_end)
            .collect::<Vec<_>>();
        if selected.len() != count || selected[0].0 != first_row || head_start + 10 != first_row {
            return None;
        }
        for (index, &(start, row_index, record_count)) in selected.iter().enumerate() {
            let end = selected.get(index + 1).map_or(group_end, |next| next.0);
            let records_start = start + 35;
            if records_start > end || (record_count > 0 && records_start == end) {
                return None;
            }
            frames.push(IecRowFrame {
                group_index,
                row_index,
                start,
                records_start,
                end,
                record_count,
            });
        }
    }
    (frames.len() == rows.len()).then_some(frames)
}

#[derive(Debug, Clone)]
pub(crate) struct ElementOperand {
    pub(crate) string: LadderString,
    #[cfg_attr(not(feature = "wasm"), allow(dead_code))]
    pub(crate) kind: &'static str,
    pub(crate) record_code: u8,
    #[cfg_attr(not(feature = "wasm"), allow(dead_code))]
    pub(crate) raw_x: u8,
    #[cfg_attr(not(feature = "wasm"), allow(dead_code))]
    pub(crate) raw_y: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct FixedFunctionBlock {
    pub(crate) string: LadderString,
    #[cfg_attr(not(feature = "wasm"), allow(dead_code))]
    pub(crate) raw_x: u8,
    #[cfg_attr(not(feature = "wasm"), allow(dead_code))]
    pub(crate) raw_y: u16,
}

pub(crate) fn comments(program: &LadderProgramData) -> Vec<LadderString> {
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return Vec::new();
    }
    crate::internal::extract_utf16_marker_strings(&program.data, false, false)
        .into_iter()
        .filter(|item| {
            !item.value.chars().any(char::is_control)
                && is_record(&program.data, item.offset, &COMMENT_PREFIX)
        })
        .collect()
}

pub(crate) fn rising_contact_operands(program: &LadderProgramData) -> Vec<LadderString> {
    element_operands(program)
        .into_iter()
        .filter(|item| item.record_code == 0x08)
        .map(|item| item.string)
        .collect()
}

pub(crate) fn function_operands(program: &LadderProgramData) -> Vec<LadderString> {
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return Vec::new();
    }
    crate::internal::extract_utf16_marker_strings(&program.data, false, false)
        .into_iter()
        .filter(|item| {
            !item.value.chars().any(char::is_control)
                && is_record(&program.data, item.offset, &FUNCTION_OPERAND_PREFIX)
        })
        .collect()
}

pub(crate) fn arithmetic_function_names(program: &LadderProgramData) -> Vec<LadderString> {
    fixed_function_blocks(program)
        .into_iter()
        .filter(|block| matches!(block.string.value.as_str(), "ADD" | "SUB" | "MUL" | "DIV"))
        .map(|block| block.string)
        .collect()
}

pub(crate) fn comparison_function_names(program: &LadderProgramData) -> Vec<LadderString> {
    fixed_function_blocks(program)
        .into_iter()
        .filter(|block| {
            matches!(
                block.string.value.as_str(),
                "EQ" | "GT" | "GE" | "LT" | "LE"
            )
        })
        .map(|block| block.string)
        .collect()
}

pub(crate) fn fixed_function_blocks(program: &LadderProgramData) -> Vec<FixedFunctionBlock> {
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return Vec::new();
    }
    crate::internal::extract_utf16_marker_strings(&program.data, false, false)
        .into_iter()
        .filter_map(|string| {
            let start = string.offset.checked_sub(67)?;
            let (family, opcode, count) = match string.value.as_str() {
                "ADD" => (0x20, 0x47, 3),
                "SUB" => (0x20, 0x7f, 3),
                "MUL" => (0x20, 0x48, 3),
                "DIV" => (0x20, 0x63, 3),
                "MOVE" => (0x20, 0x76, 2),
                "EQ" => (0x28, 0x4c, 3),
                "GT" => (0x28, 0x44, 3),
                "GE" => (0x28, 0x4f, 3),
                "LT" => (0x28, 0x4d, 3),
                "LE" => (0x28, 0x50, 3),
                _ => return None,
            };
            if program.data.get(start..start + 9)
                != Some(&[family, opcode, 0, 0, 0, count, 0, 0, 0])
            {
                return None;
            }
            let position = program.data.get(start + 11..start + 14)?;
            let raw_x = position[0];
            let raw_y = u16::from_le_bytes([position[1], position[2]]);
            if !(1..=94).contains(&raw_x) || raw_y % 4 != 0 {
                return None;
            }
            Some(FixedFunctionBlock {
                string,
                raw_x,
                raw_y,
            })
        })
        .collect()
}

pub(crate) fn element_operands(program: &LadderProgramData) -> Vec<ElementOperand> {
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return Vec::new();
    }
    crate::internal::extract_utf16_marker_strings(&program.data, false, false)
        .into_iter()
        .filter_map(|string| {
            if string.value.chars().any(char::is_control) {
                return None;
            }
            let start = string.offset.checked_sub(STRING_OFFSET_FROM_RECORD)?;
            let code = match program.data.get(start..start + 2)? {
                [0xff, code] => *code,
                _ => return None,
            };
            let kind = match code {
                0x06 => "Normally open contact variable",
                0x07 => "Normally closed contact variable",
                0x08 => "Rising-edge contact variable",
                0x09 => "Falling-edge contact variable",
                0x0a => "Negated rising-edge contact variable",
                0x0b => "Negated falling-edge contact variable",
                0x0e => "Output coil variable",
                0x0f => "Inverse output coil variable",
                0x10 => "Set coil variable",
                0x11 => "Reset coil variable",
                0x12 => "Rising-edge coil variable",
                0x13 => "Falling-edge coil variable",
                _ => return None,
            };
            Some(ElementOperand {
                string,
                kind,
                record_code: code,
                raw_x: program.data[start + 5],
                raw_y: u16::from_le_bytes([program.data[start + 6], program.data[start + 7]]),
            })
        })
        .collect()
}

fn is_record(data: &[u8], string_offset: usize, prefix: &[u8]) -> bool {
    string_offset
        .checked_sub(STRING_OFFSET_FROM_RECORD)
        .and_then(|start| data.get(start..start + prefix.len()))
        == Some(prefix)
}
