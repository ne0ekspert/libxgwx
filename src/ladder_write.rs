//! Structural editing for decoded LD rows and branch references.
use crate::{XgwxError, ladder_records::*};

/// Optimistic edit at a physical cell. `expected: None` means no element;
/// `replacement: None` removes the actual record, leaving a wiring gap.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct LadderCellEdit {
    pub raw_y: u32,
    pub column: u8,
    pub expected: Option<LadderEditElement>,
    pub replacement: Option<LadderEditElement>,
}

/// Kind of native comment record shown in an LD diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
pub enum LadderCommentKind {
    Rung,
    Output,
}

/// Create or replace a native LD comment. `expected` is `None` only when
/// creating a comment; edits fail if the stored text changed after selection.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct LadderCommentEdit {
    pub kind: LadderCommentKind,
    pub raw_y: u32,
    pub expected: Option<String>,
    pub replacement: String,
}

/// Insert one catalog application instruction at the output end of a row.
pub(crate) fn insert_ladder_instruction(
    bytes: &[u8],
    raw_y: u32,
    mnemonic: &str,
    operands: &[String],
) -> Result<Vec<u8>, XgwxError> {
    insert_ladder_function(bytes, raw_y, mnemonic, operands, None)
}

pub(crate) fn insert_ladder_comparison(
    bytes: &[u8],
    raw_y: u32,
    column: u8,
    mnemonic: &str,
    operands: &[String],
) -> Result<Vec<u8>, XgwxError> {
    let operand_count = crate::ladder_comparison_catalog()
        .iter()
        .find(|spec| spec.mnemonic == mnemonic)
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "unknown comparison instruction",
        })?
        .operand_count;
    if usize::from(column) + operand_count + 1 > 9 {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "comparison operands must fit before the output column",
        });
    }
    insert_ladder_function(bytes, raw_y, mnemonic, operands, Some(column))
}

// Branch records use grid boundaries (3, 6, ...), rather than element
// positions (1, 4, ...). The next wire cell starts immediately after a boundary.
fn trailing_wire_start(record: &Record) -> u8 {
    if record.bytes.starts_with(&[0, 0]) || record.bytes.starts_with(&[1, 0]) {
        record.x + 1
    } else {
        record_end_x(record) + 3
    }
}

fn valid_instruction_operand_text(text: &str) -> bool {
    if let Some(body) = text.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')) {
        return body.chars().all(|c| c.is_ascii() && !c.is_ascii_control() && c != '\'');
    }
    !text.is_empty() && text.chars().all(|c| c.is_ascii_graphic() && !matches!(c, ',' | '\'' | '"'))
}

fn insert_ladder_function(
    bytes: &[u8],
    raw_y: u32,
    mnemonic: &str,
    operands: &[String],
    contact_column: Option<u8>,
) -> Result<Vec<u8>, XgwxError> {
    let catalog = if contact_column.is_some() {
        crate::ladder_comparison_catalog()
    } else {
        crate::ladder_instruction_catalog()
    };
    let spec = catalog
        .iter()
        .find(|spec| spec.mnemonic == mnemonic)
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "unknown application instruction",
        })?;
    if operands.len() != spec.operand_count
        || operands.iter().any(|operand| {
            !valid_instruction_operand_text(operand)
        })
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "instruction operand count or text is invalid",
        });
    }
    crate::instruction_operands::validate_raw_operands(
        mnemonic,
        &operands.iter().map(String::as_str).collect::<Vec<_>>(),
    )?;
    let parts = std::iter::once(mnemonic)
        .chain(operands.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let text = parts.join(",");
    if text.len() > 255 {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "instruction text exceeds 255 units",
        });
    }
    let mut program = EditableProgram::parse(bytes)?;
    if !raw_y.is_multiple_of(4)
        || raw_y >= XGK_MAX_ROWS as u32 * 4
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "instruction row is outside the editable range",
        });
    }
    let row_index = program.materialize_row(raw_y);
    let row = &mut program.rows[row_index];
    let x = contact_column.map_or(94 - (spec.operand_count as u8) * 3, |column| 1 + column * 3);
    let right = x + spec.operand_count as u8 * 3;
    let flags = if contact_column.is_some() { 0 } else { 32 };
    if row.prefix[13] != 0
        || row.records.iter().any(|record| {
            record.wire_end.is_none()
                && !matches!(record.bytes[1], 0x23 | 0x24)
                && record.x <= right
                && record_end_x(record) >= x
        })
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "instruction would overlap an existing element or branch",
        });
    }
    let mut records = Vec::new();
    for record in &row.records {
        if record.wire_end.is_some_and(|end| end >= x) && record.x <= right {
            if record.x < x {
                records.push(wire(record.x, x - 3, raw_y));
            }
            if record.wire_end.unwrap() > right {
                records.push(wire(right + 3, record.wire_end.unwrap(), raw_y));
            }
        } else {
            records.push(record.clone());
        }
    }
    let wire_start = records
        .iter()
        .map(trailing_wire_start)
        .max()
        .unwrap_or(1);
    if contact_column.is_none() && wire_start < x {
        records.push(wire(wire_start, x - 3, raw_y));
    }
    let mut record = vec![0, 34, 0, 0, 0, x, raw_y.to_le_bytes()[0], raw_y.to_le_bytes()[1], raw_y.to_le_bytes()[2], 1, 0, flags, 0, 0, 0];
    record.extend_from_slice(&spec.opcode.to_le_bytes());
    append_string(&mut record, &text);
    record.extend_from_slice(&(parts.len() as u16).to_le_bytes());
    for (index, part) in parts.iter().enumerate() {
        record.extend_from_slice(&[
            x + index as u8 * 3,
            raw_y.to_le_bytes()[0], raw_y.to_le_bytes()[1], raw_y.to_le_bytes()[2],
            u8::from(index == 0),
            0,
            flags,
            0,
            0,
            0,
        ]);
        append_string(&mut record, part);
    }
    records.push(Record {
        bytes: record,
        x,
        wire_end: None,
        element: None,
    });
    for index in 1..parts.len() {
        records.push(Record {
            bytes: vec![
                index as u8,
                if index == parts.len() - 1 { 0x24 } else { 0x23 },
                0,
                0,
                0,
                x,
                raw_y.to_le_bytes()[0], raw_y.to_le_bytes()[1], raw_y.to_le_bytes()[2],
            ],
            x,
            wire_end: None,
            element: None,
        });
    }
    records.sort_by_key(|record| record.x);
    row.records = records;
    if contact_column.is_none() {
        row.prefix[21] = x;
    } else if matches!(mnemonic, "B" | "BN") {
        // Native indexed-bit insertion remembers the index operand's position.
        // Other comparison families retain their existing row metadata.
        row.prefix[29] = right;
    }
    if operands.iter().any(|operand| operand.starts_with('\'')) {
        // Native $MOV/$MOVP captures use this row-prefix variant for literals.
        row.prefix[17] = 40;
    }
    merge_wires(row);
    let output = program.encode();
    EditableProgram::parse(&output)?;
    Ok(output)
}

fn record_end_x(record: &Record) -> u8 {
    if let Some(end) = record.wire_end {
        return end;
    }
    if record.bytes.starts_with(&[0, 34]) {
        let (_, end) = string_at(&record.bytes, 19).unwrap();
        return record.x + ((u16_at(&record.bytes, end).unwrap() - 1) * 3) as u8;
    }
    record.x
}

/// Remove a verified comparison and its operand references, leaving a wiring gap.
pub(crate) fn delete_ladder_comparison(
    bytes: &[u8],
    offset: usize,
    expected: &str,
) -> Result<Vec<u8>, XgwxError> {
    let mut program = EditableProgram::parse(bytes)?;
    let mut position = program.header.len();
    let mut target = None;
    for (row_index, row) in program.rows.iter().enumerate() {
        if let Some((_, header)) = program
            .group_headers
            .iter()
            .find(|(first, _)| *first == row_index)
        {
            position += header.len();
        }
        position += row.prefix.len();
        for (index, record) in row.records.iter().enumerate() {
            if position.checked_add(19) == Some(offset) && record.bytes.starts_with(&[0, 34]) {
                target = Some((row_index, index));
            }
            position += record.bytes.len();
        }
    }
    let (row_index, index) = target.ok_or(XgwxError::InvalidLadderEdit {
        reason: "comparison record not found",
    })?;
    let row = &mut program.rows[row_index];
    let record = &row.records[index];
    let (text, end) = string_at(&record.bytes, 19)?;
    let opcode = u32::from_le_bytes(record.bytes[15..19].try_into().unwrap());
    let spec = crate::ladder_comparison_catalog()
        .iter()
        .find(|spec| spec.opcode == opcode && text.split(',').next() == Some(spec.mnemonic));
    let count = spec.map(|spec| spec.operand_count + 1);
    if record.bytes[9..15] != [1, 0, 0, 0, 0, 0]
        || spec.is_none()
        || text != expected
        || Some(u16_at(&record.bytes, end)?) != count
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "comparison changed or is not supported",
        });
    }
    // Parsing already checked the adjacent operand-reference records.
    row.records.drain(index..index + count.unwrap());
    let output = program.encode();
    EditableProgram::parse(&output)?;
    Ok(output)
}

/// Remove a catalog application at the output end of an unbranched row.
/// Native deletion also removes the immediately preceding output feed wire.
pub(crate) fn delete_ladder_instruction(
    bytes: &[u8],
    offset: usize,
    expected: &str,
) -> Result<Vec<u8>, XgwxError> {
    let mut program = EditableProgram::parse(bytes)?;
    let mut position = program.header.len();
    let mut target = None;
    for (row_index, row) in program.rows.iter().enumerate() {
        if let Some((_, header)) = program
            .group_headers
            .iter()
            .find(|(first, _)| *first == row_index)
        {
            position += header.len();
        }
        position += row.prefix.len();
        for (index, record) in row.records.iter().enumerate() {
            if position.checked_add(19) == Some(offset) && record.bytes.starts_with(&[0, 34]) {
                target = Some((row_index, index));
            }
            position += record.bytes.len();
        }
    }
    let (row_index, index) = target.ok_or(XgwxError::InvalidLadderEdit {
        reason: "application record not found",
    })?;
    if !program
        .group_headers
        .iter()
        .any(|(first, header)| *first == row_index && u16_at(header, 8).ok() == Some(1))
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "application deletion requires an unbranched row group",
        });
    }
    let row = &mut program.rows[row_index];
    let record = &row.records[index];
    let (text, end) = string_at(&record.bytes, 19)?;
    let opcode = u32::from_le_bytes(record.bytes[15..19].try_into().unwrap());
    let spec = crate::ladder_instruction_catalog()
        .iter()
        .find(|spec| spec.opcode == opcode && text.split(',').next() == Some(spec.mnemonic))
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "application is not supported",
        })?;
    let count = spec.operand_count + 1;
    if text != expected
        || usize::from(u16_at(&record.bytes, end)?) != count
        || record.bytes[9..15] != [1, 0, 32, 0, 0, 0]
        || record_end_x(record) != 94
        || row.prefix[13] != 0
        || row
            .records
            .iter()
            .any(|r| r.bytes.starts_with(&[0, 0]) || matches!(r.bytes[1], 0x3f | 0x40))
        || index + count != row.records.len()
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "application changed or has an unsupported output layout",
        });
    }
    let x = record.x;
    row.records.drain(index..index + count);
    if row
        .records
        .last()
        .is_some_and(|r| r.wire_end == x.checked_sub(3))
    {
        row.records.pop();
    }
    row.prefix[21] = 94;
    row.prefix[29] = row
        .records
        .iter()
        .filter(|r| !matches!(r.bytes[1], 0x23 | 0x24))
        .map(record_end_x)
        .max()
        .unwrap_or(1);
    let output = program.encode();
    EditableProgram::parse(&output)?;
    Ok(output)
}

/// Replace a recognized instruction's combined text and all decomposed tokens.
/// Returns None for a target outside an instruction in a supported program.
/// Unknown layouts retain the older bounded text API.
pub(crate) fn update_instruction_text(
    bytes: &[u8],
    offset: usize,
    expected: &str,
    replacement: &str,
) -> Result<Option<Vec<u8>>, XgwxError> {
    let Ok(mut program) = EditableProgram::parse(bytes) else {
        return Ok(None);
    };
    let mut position = program.header.len();
    let mut target = None;
    for (row_index, row) in program.rows.iter().enumerate() {
        if let Some((_, header)) = program
            .group_headers
            .iter()
            .find(|(first, _)| *first == row_index)
        {
            position += header.len();
        }
        position += row.prefix.len();
        for (record_index, record) in row.records.iter().enumerate() {
            if record.bytes.starts_with(&[0, 34])
                && (position..position + record.bytes.len()).contains(&offset)
            {
                if offset != position + 19 {
                    return Err(XgwxError::InvalidLadderEdit {
                        reason: "edit the complete instruction, not an internal operand copy",
                    });
                }
                target = Some((row_index, record_index));
            }
            position += record.bytes.len();
        }
    }
    let Some((row, record)) = target else {
        return Ok(None);
    };
    let record_index = record;
    let record = program.rows[row].records[record_index].clone();
    let (original, end) = string_at(&record.bytes, 19)?;
    let count = u16_at(&record.bytes, end)?;
    let original_parts = crate::internal::ladder_instruction_parts(&original, false)
        .ok_or(XgwxError::InvalidLadderEdit { reason: "unclosed instruction string literal" })?;
    if original != expected || original_parts.len() != count {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "instruction text does not match its stored operands",
        });
    }
    let parts = crate::internal::ladder_instruction_parts(replacement, false)
        .ok_or(XgwxError::InvalidLadderEdit { reason: "unclosed instruction string literal" })?
        .into_iter().map(str::trim).collect::<Vec<_>>();
    crate::instruction_operands::validate_raw_operands(parts[0], &parts[1..])?;
    let type_changed = parts.first() != original_parts.first();
    let known_operandless_application = count == 1
        && record.bytes[11] == 32
        && crate::ladder_instruction_catalog().iter().any(|spec| {
            spec.operand_count == 0
                && spec.mnemonic == original_parts[0]
                && spec.opcode == u32::from_le_bytes(record.bytes[15..19].try_into().unwrap())
        });
    if count < 2 && !known_operandless_application {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "terminal instructions cannot be replaced",
        });
    }
    let opcode = if type_changed {
        let catalog = if record.bytes[11] == 0 {
            crate::ladder_comparison_catalog()
        } else {
            crate::ladder_instruction_catalog()
        };
        let definition = catalog
            .iter()
            .find(|spec| spec.mnemonic == parts[0])
            .ok_or(XgwxError::InvalidLadderEdit {
                reason: "instruction replacement requires a supported instruction mnemonic",
            })?;
        if parts.len() != definition.operand_count + 1 {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "operand count does not match the selected instruction",
            });
        }
        definition.opcode
    } else {
        if parts.len() != count {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "operand count does not match the existing instruction",
            });
        }
        u32::from_le_bytes(record.bytes[15..19].try_into().unwrap())
    };
    if parts
        .iter()
        .any(|part| !valid_instruction_operand_text(part))
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "instruction operands require printable ASCII tokens or single-quoted strings",
        });
    }
    let text = parts.join(",");
    if text.encode_utf16().count() > 255 {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "instruction text exceeds the 255 UTF-16 unit record limit",
        });
    }
    // Validate every redundant original token before rebuilding either copy.
    let mut next = end + 2;
    let mut argument_headers = Vec::new();
    for part in &original_parts {
        let (stored, stop) = string_at(&record.bytes, next + 10)?;
        if stored != *part {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "instruction text does not match its stored operands",
            });
        }
        argument_headers.push(record.bytes[next..next + 10].to_vec());
        next = stop;
    }
    let right = usize::from(record.x) + (count - 1) * 3;
    // Native comparison contacts keep their left edge when their arity changes.
    // Output applications keep their right edge at the output column.
    let comparison = record.bytes[11] == 0;
    let new_x = if comparison {
        record.x
    } else {
        right
            .checked_sub((parts.len() - 1) * 3)
            .filter(|x| *x >= 1)
            .ok_or(XgwxError::InvalidLadderEdit {
                reason: "not enough space for the replacement instruction",
            })? as u8
    };
    let new_right = usize::from(new_x) + (parts.len() - 1) * 3;
    if comparison && new_right > 25 {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "comparison operands must fit before the output column",
        });
    }
    if type_changed
        && argument_headers
            .iter()
            .enumerate()
            .any(|(i, h)| h[4..] != [u8::from(i == 0), 0, record.bytes[11], 0, 0, 0])
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "instruction has unsupported operand flags",
        });
    }
    let mut updated = record.bytes[..19].to_vec();
    updated[5] = new_x;
    updated[15..19].copy_from_slice(&opcode.to_le_bytes());
    append_string(&mut updated, &text);
    updated.extend_from_slice(&(parts.len() as u16).to_le_bytes());
    for (index, part) in parts.iter().enumerate() {
        let header = if type_changed {
            vec![
                new_x + index as u8 * 3,
                program.rows[row].y.to_le_bytes()[0], program.rows[row].y.to_le_bytes()[1], program.rows[row].y.to_le_bytes()[2],
                u8::from(index == 0),
                0,
                record.bytes[11],
                0,
                0,
                0,
            ]
        } else {
            argument_headers[index].clone()
        };
        updated.extend(header);
        append_string(&mut updated, part);
    }
    if !type_changed {
        program.rows[row].records[record_index].bytes = updated;
    } else {
        let row = &mut program.rows[row];
        let mut rebuilt = Vec::new();
        for (index, other) in row.records.iter().enumerate() {
            if (record_index..record_index + count).contains(&index) {
                continue;
            }
            if comparison && new_right > right {
                let growth_start = right as u8 + 3;
                if let Some(end) = other.wire_end {
                    if other.x <= new_right as u8 && end >= growth_start {
                        if other.x < growth_start {
                            rebuilt.push(wire(other.x, growth_start - 3, row.y));
                        }
                        if end > new_right as u8 {
                            rebuilt.push(wire(new_right as u8 + 3, end, row.y));
                        }
                        continue;
                    }
                } else if other.x <= new_right as u8 && record_end_x(other) >= growth_start {
                    return Err(XgwxError::InvalidLadderEdit {
                        reason: "replacement instruction would overlap an element or branch connection",
                    });
                }
            }
            if new_x < record.x && other.x < record.x {
                if let Some(end) = other.wire_end {
                    if end >= new_x {
                        if other.x < new_x {
                            rebuilt.push(wire(other.x, new_x - 3, row.y));
                        }
                        continue;
                    }
                } else if other.x >= new_x {
                    return Err(XgwxError::InvalidLadderEdit {
                        reason: "replacement instruction would overlap an element or branch connection",
                    });
                }
            }
            rebuilt.push(other.clone());
        }
        if new_x > record.x {
            rebuilt.push(wire(record.x, new_x - 3, row.y));
        }
        if comparison && new_right < right {
            rebuilt.push(wire(new_right as u8 + 3, right as u8, row.y));
        }
        rebuilt.push(Record {
            bytes: updated,
            x: new_x,
            wire_end: None,
            element: None,
        });
        for index in 1..parts.len() {
            rebuilt.push(Record {
                bytes: vec![
                    index as u8,
                    if index == parts.len() - 1 { 0x24 } else { 0x23 },
                    0,
                    0,
                    0,
                    new_x,
                    row.y.to_le_bytes()[0], row.y.to_le_bytes()[1], row.y.to_le_bytes()[2],
                ],
                x: new_x,
                wire_end: None,
                element: None,
            });
        }
        rebuilt.sort_by_key(|r| r.x);
        row.records = rebuilt;
        merge_wires(row);
        if comparison && usize::from(row.prefix[29]) == right {
            row.prefix[29] = new_right as u8;
        }
        if row.prefix[21] == record.x {
            row.prefix[21] = new_x;
        }
    }
    if parts.iter().skip(1).any(|operand| operand.starts_with('\'')) {
        program.rows[row].prefix[17] = 40;
    }
    let output = program.encode();
    EditableProgram::parse(&output)?;
    Ok(Some(output))
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend([255, 254, 255, value.encode_utf16().count() as u8]);
    for unit in value.encode_utf16() {
        bytes.extend(unit.to_le_bytes());
    }
}

fn wire(x: u8, end: u8, y: u32) -> Record {
    let mut bytes = vec![255, 2, 0, 0, 0, x, y.to_le_bytes()[0], y.to_le_bytes()[1], y.to_le_bytes()[2], 0, 0, 0, 0, 0, 0];
    bytes.extend([end, y.to_le_bytes()[0], y.to_le_bytes()[1], y.to_le_bytes()[2]]);
    Record {
        bytes,
        x,
        wire_end: Some(end),
        element: None,
    }
}
fn element_record(x: u8, y: u32, element: &LadderEditElement) -> Record {
    let mut bytes = vec![
        255,
        element.kind.marker(),
        0,
        0,
        0,
        x,
        y.to_le_bytes()[0], y.to_le_bytes()[1], y.to_le_bytes()[2],
        1,
        0,
        if element.kind.is_coil() { 32 } else { 0 },
        0,
        0,
        0,
    ];
    if element.kind.has_operand() {
        bytes.extend([255, 254, 255, element.operand.encode_utf16().count() as u8]);
        for unit in element.operand.encode_utf16() {
            bytes.extend(unit.to_le_bytes());
        }
    }
    Record {
        bytes,
        x,
        wire_end: None,
        element: Some(element.clone()),
    }
}

fn comment_marker(kind: LadderCommentKind) -> u8 {
    match kind {
        LadderCommentKind::Rung => 0x3f,
        LadderCommentKind::Output => 0x40,
    }
}

fn comment_record(kind: LadderCommentKind, y: u32, text: &str) -> Record {
    let rung = kind == LadderCommentKind::Rung;
    let x = if rung { 1 } else { 97 };
    let mut bytes = vec![
        255,
        comment_marker(kind),
        0,
        0,
        0,
        x,
        y.to_le_bytes()[0], y.to_le_bytes()[1], y.to_le_bytes()[2],
        0,
        0,
        if rung { 32 } else { 0 },
        0,
        0,
        0,
    ];
    append_string(&mut bytes, text);
    let native_width = if rung { 13_u32 } else { 12_u32 };
    bytes.extend(native_width.to_le_bytes());
    bytes.extend(native_width.to_le_bytes());
    Record {
        bytes,
        x,
        wire_end: None,
        element: None,
    }
}

fn replace_comment_text(
    record: &Record,
    expected: &str,
    replacement: &str,
) -> Result<Record, XgwxError> {
    let (actual, text_end) = string_at(&record.bytes, 15)?;
    if actual != expected {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "comment changed since selection",
        });
    }
    let trailer = record.bytes.get(text_end..).ok_or_else(unsupported)?;
    if trailer.len() != 8 {
        return Err(unsupported());
    }
    let mut bytes = record.bytes[..15].to_vec();
    append_string(&mut bytes, replacement);
    bytes.extend(trailer);
    Ok(Record {
        bytes,
        x: record.x,
        wire_end: None,
        element: None,
    })
}

/// Create or edit a native rung/output comment record.
pub(crate) fn edit_ladder_comment(
    bytes: &[u8],
    edit: &LadderCommentEdit,
) -> Result<Vec<u8>, XgwxError> {
    let units = edit.replacement.encode_utf16().count();
    if !edit.raw_y.is_multiple_of(4)
        || edit.raw_y >= XGK_MAX_ROWS as u32 * 4
        || edit.replacement.trim().is_empty()
        || units > u8::MAX as usize
        || edit.replacement.chars().any(|character| {
            character == '\0'
                || (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        })
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "comment must contain 1 to 255 UTF-16 units of text",
        });
    }

    let marker = [255, comment_marker(edit.kind)];
    if let Some(expected) = &edit.expected {
        let mut program = EditableProgram::parse(bytes)?;
        let row = program
            .rows
            .iter_mut()
            .find(|row| row.y == edit.raw_y)
            .ok_or(XgwxError::InvalidLadderEdit {
                reason: "comment row does not exist",
            })?;
        let record = row
            .records
            .iter_mut()
            .find(|record| record.bytes.starts_with(&marker))
            .ok_or(XgwxError::InvalidLadderEdit {
                reason: "comment changed since selection",
            })?;
        *record = replace_comment_text(record, expected, &edit.replacement)?;
        return Ok(program.encode());
    }

    let mut program = if edit.kind == LadderCommentKind::Rung {
        let mut parsed = EditableProgram::parse(bytes)?;
        if parsed.rows.iter().any(|row| {
            row.records.iter().any(|record| {
                record.bytes.starts_with(&[0, 0])
                    && row.y < edit.raw_y
                    && y_at(&record.bytes, 18) >= edit.raw_y
            })
        }) {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "rung comments cannot be inserted inside a branch span",
            });
        }
        let requested = edit.raw_y as usize / 4;
        if requested > row_count(&parsed.header)? {
            set_row_count(&mut parsed.header, requested);
        }
        EditableProgram::parse(&insert_ladder_row(&parsed.encode(), edit.raw_y)?)?
    } else {
        EditableProgram::parse(bytes)?
    };

    let count = row_count(&program.header)?;
    if edit.kind == LadderCommentKind::Output && edit.raw_y as usize / 4 >= count {
        set_row_count(&mut program.header, edit.raw_y as usize / 4 + 1);
    } else if (edit.raw_y as usize) / 4 >= row_count(&program.header)? {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "comment row does not exist",
        });
    }
    let index = program.materialize_row(edit.raw_y);
    let row = &mut program.rows[index];
    if row
        .records
        .iter()
        .any(|record| record.bytes.starts_with(&marker))
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "comment already exists at this row",
        });
    }
    if edit.kind == LadderCommentKind::Rung {
        if !row.records.is_empty() {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "rung comment row is not empty",
            });
        }
        row.prefix[13] = 1;
    } else if row.prefix[13] != 0 {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "output comments require a ladder row",
        });
    }
    row.records
        .push(comment_record(edit.kind, edit.raw_y, &edit.replacement));
    row.records.sort_by_key(|record| record.x);
    program.rebuild_groups();
    let output = program.encode();
    EditableProgram::parse(&output)?;
    Ok(output)
}

pub(crate) fn delete_ladder_rung_comment(
    bytes: &[u8],
    raw_y: u32,
    expected: &str,
) -> Result<Vec<u8>, XgwxError> {
    if !raw_y.is_multiple_of(4) {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "invalid rung comment row",
        });
    }
    let mut program = EditableProgram::parse(bytes)?;
    let count = row_count(&program.header)?;
    let row_index =
        program
            .rows
            .iter()
            .position(|row| row.y == raw_y)
            .ok_or(XgwxError::InvalidLadderEdit {
                reason: "comment row does not exist",
            })?;
    let row = &program.rows[row_index];
    let record = row
        .records
        .iter()
        .find(|record| record.bytes.starts_with(&[255, 0x3f]))
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "comment changed since selection",
        })?;
    let (actual, _) = string_at(&record.bytes, 15)?;
    if actual != expected {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "comment changed since selection",
        });
    }
    if row.prefix[13] != 1 || row.records.len() != 1 || count == 0 {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "rung comment row contains unsupported records",
        });
    }
    if program.rows.iter().any(|row| {
        row.y < raw_y
            && row
                .records
                .iter()
                .any(|record| record.bytes.starts_with(&[0, 0]) && y_at(&record.bytes, 18) > raw_y)
    }) {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "rung comment belongs to a branch span",
        });
    }

    program.rows.remove(row_index);
    for row in &mut program.rows {
        let old_y = row.y;
        if old_y > raw_y {
            row.y -= 4;
            row.prefix[..4].copy_from_slice(&(u32::from(row.y) / 4).to_le_bytes());
            put_y(&mut row.prefix, 22, row.y);
            put_y(&mut row.prefix, 30, row.y);
        }
        for record in &mut row.records {
            if record.bytes.starts_with(&[0, 0]) {
                if y_at(&record.bytes, 8) > raw_y {
                    shift_y(&mut record.bytes, 8, -4);
                }
                if y_at(&record.bytes, 18) > raw_y {
                    shift_y(&mut record.bytes, 18, -4);
                }
            } else if record.bytes.starts_with(&[1, 0]) {
                if y_at(&record.bytes, 6) > raw_y {
                    shift_y(&mut record.bytes, 6, -4);
                }
            } else if old_y > raw_y {
                put_y(&mut record.bytes, 6, row.y);
                if record.wire_end.is_some() {
                    put_y(&mut record.bytes, 16, row.y);
                }
                if record.bytes.starts_with(&[0, 34]) {
                    let (_, end) = string_at(&record.bytes, 19)?;
                    let operand_count = u16_at(&record.bytes, end)?;
                    let mut next = end + 2;
                    for _ in 0..operand_count {
                        put_y(&mut record.bytes, next + 1, row.y);
                        let (_, end) = string_at(&record.bytes, next + 10)?;
                        next = end;
                    }
                }
            }
        }
    }
    set_row_count(&mut program.header, count - 1);
    program.rebuild_groups();
    let output = program.encode();
    EditableProgram::parse(&output)?;
    Ok(output)
}

/// Physical rows, including sparse blanks, excluding comment-only rows.
#[cfg(feature = "wasm")]
pub(crate) fn editable_ladder_rows(bytes: &[u8]) -> Result<Vec<u32>, XgwxError> {
    let program = EditableProgram::parse(bytes)?;
    Ok((0..row_count(&program.header)?)
        .map(|index| (index * 4) as u32)
        .filter(|y| !program.rows.iter().any(|r| r.y == *y && r.prefix[13] != 0))
        .collect())
}

#[cfg(any(feature = "wasm", test))]
pub(crate) fn editable_ladder_supported(bytes: &[u8]) -> bool {
    EditableProgram::parse(bytes).is_ok()
}

/// Exact vertical reference segments; unlike rendered lines these are not merged.
#[cfg(feature = "wasm")]
pub(crate) fn ladder_connections(bytes: &[u8]) -> Result<Vec<(u8, u32, u32)>, XgwxError> {
    let program = EditableProgram::parse(bytes)?;
    Ok(program
        .rows
        .iter()
        .flat_map(|row| {
            row.records
                .iter()
                .filter(|r| r.bytes.starts_with(&[0, 0]))
                .map(|r| (r.x, row.y, y_at(&r.bytes, 18)))
        })
        .collect())
}

fn valid_bit_device_address(operand: &str) -> bool {
    let address = operand.as_bytes();
    if !(2..=32).contains(&address.len()) {
        return false;
    }
    if address[0] == b'D' {
        return operand[1..].split_once('.').is_some_and(|(word, bit)| {
            !word.is_empty()
                && word.bytes().all(|c| c.is_ascii_digit())
                && bit.len() == 1
                && bit
                    .bytes()
                    .all(|c| c.is_ascii_digit() || matches!(c, b'A'..=b'F'))
        });
    }
    matches!(address[0], b'P' | b'M' | b'K' | b'F' | b'L' | b'T' | b'C')
        && address[1..].iter().all(u8::is_ascii_digit)
}

pub(crate) fn edit_ladder_cell(bytes: &[u8], edit: &LadderCellEdit) -> Result<Vec<u8>, XgwxError> {
    if edit.column > 9 || !edit.raw_y.is_multiple_of(4) || edit.raw_y >= XGK_MAX_ROWS as u32 * 4 {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "invalid row or column",
        });
    }
    if let Some(element) = &edit.replacement {
        if element.kind.has_operand() {
            if !valid_bit_device_address(&element.operand) {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "use an uppercase P/M/K/F/L/T/C device address or D register bit D0000.0 through D0000.F",
                });
            }
        } else if !element.operand.is_empty() {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "this operation does not accept a device address",
            });
        }
        if element.kind.is_coil() != (edit.column == 9) {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "coils require column 10; contacts require columns 1 through 9",
            });
        }
    }
    let mut program = EditableProgram::parse(bytes)?;
    if program.rows.is_empty()
        && edit.raw_y == 0
        && edit.expected.is_none()
        && edit.replacement.is_some()
    {
        let mut prefix = vec![0; 35];
        prefix[4..6].copy_from_slice(&[255, 67]);
        prefix[17] = 39;
        prefix[21] = 94;
        prefix[25] = 94;
        prefix[29] = 1;
        program
            .group_headers
            .push((0, vec![0, 0, 0, 0, 0, 0, 0, 0, 1, 0]));
        program.rows.push(Row {
            prefix,
            y: 0,
            records: Vec::new(),
        });
    }
    if edit.expected.is_none() && edit.replacement.is_some() {
        program.materialize_row(edit.raw_y);
    }
    let row = program
        .rows
        .iter_mut()
        .find(|row| row.y == edit.raw_y)
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "row does not exist",
        })?;
    let x = if edit.column == 9 {
        94
    } else {
        1 + edit.column * 3
    };
    if row.prefix[13] != 0
        || row
            .records
            .iter()
            .any(|r| r.bytes.starts_with(&[0, 34]) && r.x <= x && x <= record_end_x(r))
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "this cell belongs to a comment or application instruction",
        });
    }
    let existing = row
        .records
        .iter()
        .find(|record| record.x == x && record.wire_end.is_none());
    if existing.is_some_and(|record| record.element.is_none()) {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "this instruction is not structurally editable",
        });
    }
    if existing.and_then(|record| record.element.as_ref()) != edit.expected.as_ref() {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "cell changed since selection",
        });
    }
    let mut records = Vec::new();
    for record in &row.records {
        if record.x == x && record.wire_end.is_none() {
            continue;
        }
        if edit.replacement.is_some()
            && let Some(end) = record.wire_end
            && record.x <= x
            && end >= x
        {
            if record.x < x {
                records.push(wire(record.x, x - 3, row.y));
            }
            if end > x {
                records.push(wire(x + 3, end, row.y));
            }
        } else {
            records.push(record.clone());
        }
    }
    if let Some(element) = &edit.replacement {
        // Native coil insertion fills only the trailing wire after the last
        // existing contact or wire; it does not bridge earlier deleted cells.
        if element.kind.is_coil() && edit.expected.is_none() {
            let start = records
                .iter()
                .map(trailing_wire_start)
                .max()
                .unwrap_or(1);
            if start <= 91 {
                records.push(wire(start, 91, row.y));
            }
        }
        records.push(element_record(x, row.y, element));
    }
    records.sort_by_key(|record| record.x);
    row.prefix[29] = row.prefix[29].max(
        records
            .iter()
            .filter(|r| r.element.as_ref().is_some_and(|e| !e.kind.is_coil()))
            .map(|r| r.x)
            .max()
            .unwrap_or(1),
    );
    row.records = records;
    Ok(program.encode())
}

/// Toggle a vertical connection from this physical row to the next one.
/// `boundary` is the grid boundary after columns 1 through 9.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct LadderBranchEdit {
    pub raw_y: u32,
    pub boundary: u8,
    pub expected: bool,
    pub present: bool,
}

fn blank_row(y: u32) -> Row {
    let mut prefix = vec![0; 35];
    prefix[..4].copy_from_slice(&(u32::from(y) / 4).to_le_bytes());
    prefix[4..6].copy_from_slice(&[255, 67]);
    prefix[17] = 39;
    prefix[21..25].copy_from_slice(&xy(94, y));
    prefix[25..29].copy_from_slice(&xy(94, y));
    prefix[29..33].copy_from_slice(&xy(1, y));
    Row {
        prefix,
        y,
        records: Vec::new(),
    }
}
fn branch_start(x: u8, y: u32, target_y: u32) -> Record {
    let bytes = vec![
        0,
        0,
        0,
        0,
        0,
        2,
        0,
        x,
        y.to_le_bytes()[0], y.to_le_bytes()[1], y.to_le_bytes()[2],
        0,
        0,
        0,
        0,
        0,
        0,
        x - 1,
        target_y.to_le_bytes()[0], target_y.to_le_bytes()[1], target_y.to_le_bytes()[2],
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    Record {
        bytes,
        x,
        wire_end: None,
        element: None,
    }
}
fn branch_end(x: u8, source_y: u32) -> Record {
    Record {
        bytes: vec![1, 0, 0, 0, 0, x, source_y.to_le_bytes()[0], source_y.to_le_bytes()[1], source_y.to_le_bytes()[2]],
        x,
        wire_end: None,
        element: None,
    }
}
impl EditableProgram {
    fn materialize_row(&mut self, y: u32) -> usize {
        if let Some(i) = self.rows.iter().position(|row| row.y == y) {
            return i;
        }
        let i = self.rows.partition_point(|row| row.y < y);
        self.rows.insert(i, blank_row(y));
        self.rebuild_groups();
        i
    }
    fn rebuild_groups(&mut self) {
        let mut connected = vec![false; self.rows.len().saturating_sub(1)];
        for (i, row) in self.rows.iter().enumerate() {
            for record in &row.records {
                if record.bytes.starts_with(&[0, 0])
                    && let Some(last) = self.rows.iter().position(|r| r.y == y_at(&record.bytes, 18))
                {
                    for edge in &mut connected[i..last] {
                        *edge = true;
                    }
                }
            }
        }
        self.group_headers.clear();
        let mut first = 0;
        while first < self.rows.len() {
            let mut last = first + 1;
            while last < self.rows.len() && connected[last - 1] {
                last += 1;
            }
            let mut header = vec![0; 10];
            header[..4].copy_from_slice(&(self.group_headers.len() as u32).to_le_bytes());
            header[8..10].copy_from_slice(&((last - first) as u16).to_le_bytes());
            self.group_headers.push((first, header));
            first = last;
        }
    }
}
fn split_wire_at_boundary(row: &mut Row, x: u8) {
    let mut records = Vec::new();
    for r in &row.records {
        if let Some(end) = r.wire_end
            && r.x < x
            && end > x
        {
            records.push(wire(r.x, x - 2, row.y));
            records.push(wire(x + 1, end, row.y));
        } else {
            records.push(r.clone());
        }
    }
    row.records = records;
}
fn merge_wires(row: &mut Row) {
    let mut records: Vec<Record> = Vec::new();
    for r in &row.records {
        if let Some(previous) = records.last_mut()
            && let (Some(end), Some(next_end)) = (previous.wire_end, r.wire_end)
            && end + 3 == r.x
        {
            *previous = wire(previous.x, next_end, row.y);
        } else {
            records.push(r.clone());
        }
    }
    row.records = records;
}
pub(crate) fn edit_ladder_branch(
    bytes: &[u8],
    edit: &LadderBranchEdit,
) -> Result<Vec<u8>, XgwxError> {
    let mut program = EditableProgram::parse(bytes)?;
    let y = edit.raw_y;
    if !(1..=9).contains(&edit.boundary)
        || !y.is_multiple_of(4)
        || (y as usize) / 4 + 1 >= row_count(&program.header)?
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "branch needs two existing adjacent rows and boundary 1 through 9",
        });
    }
    let x = edit.boundary * 3;
    let target_y = y + 4;
    let exists = program.rows.iter().any(|r| {
        r.y == y
            && r.records
                .iter()
                .any(|r| r.bytes.starts_with(&[0, 0]) && r.x == x && y_at(&r.bytes, 18) == target_y)
    });
    if exists != edit.expected {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "branch changed since selection",
        });
    }
    if exists == edit.present {
        return Ok(bytes.to_vec());
    }
    for row in program.rows.iter().filter(|r| r.y == y || r.y == target_y) {
        if row.prefix[13] != 0
            || row
                .records
                .iter()
                .any(|r| r.bytes.starts_with(&[0, 34]) && r.x <= x + 1 && x < record_end_x(r))
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "branch boundary belongs to an unsupported record or connection span",
            });
        }
    }
    if program.rows.iter().any(|row| {
        row.records.iter().any(|r| {
            r.bytes.starts_with(&[0, 0])
                && r.x == x
                && row.y < target_y
                && y_at(&r.bytes, 18) > y
                && (row.y != y || y_at(&r.bytes, 18) != target_y)
        })
    }) {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "split the existing connection span before editing this segment",
        });
    }
    let upper_new = !program.rows.iter().any(|r| r.y == y);
    let lower_new = !program.rows.iter().any(|r| r.y == target_y);
    program.materialize_row(y);
    program.materialize_row(target_y);
    if edit.present {
        for row in program
            .rows
            .iter_mut()
            .filter(|r| r.y == y || r.y == target_y)
        {
            split_wire_at_boundary(row, x);
            if row.y == y {
                row.records.push(branch_start(x, y, target_y));
                if upper_new {
                    row.prefix[29] = x;
                }
            } else {
                row.records.push(branch_end(x, y));
                if lower_new {
                    row.prefix[29] = x - 1;
                }
            }
            row.records
                .sort_by_key(|r| (r.x, r.bytes.starts_with(&[0, 0])));
        }
    } else {
        for row in program
            .rows
            .iter_mut()
            .filter(|r| r.y == y || r.y == target_y)
        {
            row.records.retain(|r| {
                !(row.y == y && r.bytes.starts_with(&[0, 0]) && r.x == x && y_at(&r.bytes, 18) == target_y)
                    && !(row.y == target_y
                        && r.bytes.starts_with(&[1, 0])
                        && r.x == x
                        && y_at(&r.bytes, 6) == y)
            });
            merge_wires(row);
        }
    }
    program.rebuild_groups();
    let output = program.encode();
    EditableProgram::parse(&output)?;
    Ok(output)
}

/// Insert one physical blank row before `raw_y`, preserving native sparse rows.
pub(crate) fn insert_ladder_row(bytes: &[u8], raw_y: u32) -> Result<Vec<u8>, XgwxError> {
    let mut program = EditableProgram::parse(bytes)?;
    let count = row_count(&program.header)?;
    if !raw_y.is_multiple_of(4) || (raw_y as usize) / 4 > count || count >= XGK_MAX_ROWS {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "invalid insertion row or row limit reached",
        });
    }
    let mut crossings = Vec::new();
    for row in &program.rows {
        for r in &row.records {
            if r.bytes.starts_with(&[0, 0]) && row.y < raw_y && y_at(&r.bytes, 18) >= raw_y {
                crossings.push((r.x, row.y, y_at(&r.bytes, 18) + 4));
            }
        }
    }
    for row in &mut program.rows {
        let old_y = row.y;
        if old_y >= raw_y {
            row.y += 4;
            row.prefix[..4].copy_from_slice(&(u32::from(row.y) / 4).to_le_bytes());
            put_y(&mut row.prefix, 22, row.y);
            put_y(&mut row.prefix, 30, row.y);
        }
        for r in &mut row.records {
            if r.bytes.starts_with(&[0, 0]) {
                if y_at(&r.bytes, 8) >= raw_y {
                    shift_y(&mut r.bytes, 8, 4);
                }
                if y_at(&r.bytes, 18) >= raw_y {
                    shift_y(&mut r.bytes, 18, 4);
                }
            } else if r.bytes.starts_with(&[1, 0]) {
                if y_at(&r.bytes, 6) >= raw_y {
                    shift_y(&mut r.bytes, 6, 4);
                }
            } else if old_y >= raw_y {
                put_y(&mut r.bytes, 6, row.y);
                if r.wire_end.is_some() {
                    put_y(&mut r.bytes, 16, row.y);
                }
                if r.bytes.starts_with(&[0, 34]) {
                    let (_, end) = string_at(&r.bytes, 19)?;
                    let count = u16_at(&r.bytes, end)?;
                    let mut next = end + 2;
                    for _ in 0..count {
                        put_y(&mut r.bytes, next + 1, row.y);
                        let (_, end) = string_at(&r.bytes, next + 10)?;
                        next = end;
                    }
                }
            }
        }
    }
    if !crossings.is_empty() {
        let i = program.materialize_row(raw_y);
        for (x, source_y, target_y) in crossings {
            for row in &mut program.rows {
                for r in &mut row.records {
                    if row.y == source_y
                        && r.bytes.starts_with(&[0, 0])
                        && r.x == x
                        && y_at(&r.bytes, 18) == target_y
                    {
                        put_y(&mut r.bytes, 18, raw_y);
                    }
                    if row.y == target_y
                        && r.bytes.starts_with(&[1, 0])
                        && r.x == x
                        && y_at(&r.bytes, 6) == source_y
                    {
                        put_y(&mut r.bytes, 6, raw_y);
                    }
                }
            }
            program.rows[i].records.push(branch_end(x, source_y));
            program.rows[i]
                .records
                .push(branch_start(x, raw_y, target_y));
            program.rows[i].prefix[29] = program.rows[i].prefix[29].max(x);
        }
        program.rows[i].records.sort_by_key(|r| r.x);
    }
    set_row_count(&mut program.header, count + 1);
    program.rebuild_groups();
    let output = program.encode();
    EditableProgram::parse(&output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wide_rows_preserve_function_comment_and_branch_coordinates_across_shifts() {
        let mut bytes =
            insert_ladder_instruction(&[0; 8], 65532 * 4, "MOV", &["1".into(), "D100".into()])
                .unwrap();
        bytes =
            insert_ladder_comparison(&bytes, 65533 * 4, 0, ">=", &["D100".into(), "D102".into()])
                .unwrap();
        bytes = edit_ladder_branch(
            &bytes,
            &LadderBranchEdit {
                raw_y: 65532 * 4,
                boundary: 4,
                expected: false,
                present: true,
            },
        )
        .unwrap();
        bytes = edit_ladder_comment(
            &bytes,
            &LadderCommentEdit {
                kind: LadderCommentKind::Output,
                raw_y: 65532 * 4,
                expected: None,
                replacement: "Wide output".into(),
            },
        )
        .unwrap();
        let original = bytes.clone();
        bytes = edit_ladder_comment(
            &bytes,
            &LadderCommentEdit {
                kind: LadderCommentKind::Rung,
                raw_y: 0,
                expected: None,
                replacement: "Wide rung".into(),
            },
        )
        .unwrap();
        let shifted = EditableProgram::parse(&bytes).unwrap();
        assert_eq!(row_count(&shifted.header).unwrap(), 65535);
        assert_eq!(header_size(&shifted.header).unwrap(), 12);
        assert_eq!(shifted.rows.last().unwrap().y, 65534 * 4);
        assert!(insert_ladder_row(&bytes, 0).is_err());
        let restored = delete_ladder_rung_comment(&bytes, 0, "Wide rung").unwrap();
        assert_eq!(restored, original);
        assert_eq!(header_size(&restored).unwrap(), 8);
    }

    #[test]
    fn xgk_last_row_is_sparse_and_rejects_overflow() {
        let edit = LadderCellEdit {
            raw_y: 65534 * 4,
            column: 0,
            expected: None,
            replacement: Some(LadderEditElement {
                kind: LadderEditKind::NormallyOpen,
                operand: "M00000".into(),
            }),
        };
        let bytes = edit_ladder_cell(&[0; 8], &edit).unwrap();
        let parsed = EditableProgram::parse(&bytes).unwrap();
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.rows[0].y, 65534 * 4);
        assert_eq!(row_count(&parsed.header).unwrap(), 65535);
        assert!(
            edit_ladder_cell(
                &bytes,
                &LadderCellEdit {
                    raw_y: 65535 * 4,
                    ..edit.clone()
                }
            )
            .is_err()
        );
        assert!(edit_ladder_cell(&bytes, &edit).is_err());
        assert!(
            insert_ladder_instruction(&bytes, 65535 * 4, "MOV", &["1".into(), "D100".into()])
                .is_err()
        );
        assert_eq!(parsed.encode(), bytes);
    }
    #[test]
    fn d_register_bits_use_one_hexadecimal_index() {
        for bit in "0123456789ABCDEF".chars() {
            assert!(valid_bit_device_address(&format!("D0000.{bit}")));
        }
        for bad in ["D0000", "D0000.10", "D0000.G", "D.0", "DA.0", "D0000..0"] {
            assert!(!valid_bit_device_address(bad), "{bad}");
        }
    }

    #[test]
    fn inserting_application_instruction_reproduces_native_mov_records() {
        let document = crate::XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
        let data = document.ladder_programs().remove(0).unwrap();
        let mut program = EditableProgram::parse(&data.data).unwrap();
        let row = program
            .rows
            .iter_mut()
            .find(|row| {
                row.records.iter().any(|record| {
                    record.bytes.starts_with(&[0, 34])
                        && string_at(&record.bytes, 19).unwrap().0 == "MOV,0,D000000"
                })
            })
            .unwrap();
        let native = row.records.clone();
        let y = row.y;
        let x = native
            .iter()
            .find(|record| record.bytes.starts_with(&[0, 34]))
            .unwrap()
            .x;
        row.records.retain(|record| record.x < x);
        let edited =
            insert_ladder_instruction(&program.encode(), y, "MOV", &["0".into(), "D000000".into()])
                .unwrap();
        let parsed = EditableProgram::parse(&edited).unwrap();
        let row = parsed.rows.iter().find(|row| row.y == y).unwrap();
        assert_eq!(
            row.records.iter().map(|r| &r.bytes).collect::<Vec<_>>(),
            native.iter().map(|r| &r.bytes).collect::<Vec<_>>()
        );
        assert!(
            insert_ladder_instruction(&edited, y, "I2R", &["D100".into(), "D200".into()]).is_err()
        );
    }

    #[test]
    fn comparison_records_match_native_xg5000_captures() {
        for (name, captured) in [
            (
                "=",
                include_bytes!("../fixtures/function-bodies/xgk_eq.bin").as_slice(),
            ),
            (
                ">=",
                include_bytes!("../fixtures/function-bodies/xgk_ge.bin").as_slice(),
            ),
            (
                ">",
                include_bytes!("../fixtures/function-bodies/xgk_gt.bin").as_slice(),
            ),
            (
                "<",
                include_bytes!("../fixtures/function-bodies/xgk_lt.bin").as_slice(),
            ),
            (
                "<=",
                include_bytes!("../fixtures/function-bodies/xgk_le.bin").as_slice(),
            ),
            (
                "<>",
                include_bytes!("../fixtures/function-bodies/xgk_ne.bin").as_slice(),
            ),
        ] {
            let mut native = captured.to_vec();
            let delta = i16::from(native[5]) - 4;
            let operands = string_at(&native, 19)
                .unwrap()
                .0
                .split(',')
                .skip(1)
                .map(String::from)
                .collect::<Vec<_>>();
            native[5] = 4;
            native[6] = 0;
            let (_, text_end) = string_at(&native, 19).unwrap();
            let count = u16_at(&native, text_end).unwrap();
            let mut cursor = text_end + 2;
            for _ in 0..count {
                native[cursor] = u8::try_from(i16::from(native[cursor]) - delta).unwrap();
                native[cursor + 1] = 0;
                cursor = string_at(&native, cursor + 10).unwrap().1;
            }
            let bytes = insert_ladder_comparison(
                include_bytes!("../fixtures/ladder-edit/R10.bin"),
                0,
                1,
                name,
                &operands,
            )
            .unwrap();
            let parsed = EditableProgram::parse(&bytes).unwrap();
            let record = parsed.rows[0]
                .records
                .iter()
                .find(|r| r.bytes.starts_with(&[0, 34]))
                .unwrap();
            assert_eq!(record.bytes, native);
        }
    }

    #[test]
    fn comparison_deletion_removes_operand_references_and_preserves_other_records() {
        for spec in crate::ladder_comparison_catalog() {
            let operands = catalog_test_operands(spec);
            let source = insert_ladder_comparison(
                include_bytes!("../fixtures/ladder-edit/R10.bin"),
                0,
                1,
                spec.mnemonic,
                &operands,
            )
            .unwrap();
            let text = format!("{},{}", spec.mnemonic, operands.join(","));
            let encoded = text
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>();
            let offset = source
                .windows(encoded.len())
                .position(|b| b == encoded)
                .unwrap()
                - 4;
            assert!(delete_ladder_comparison(&source, offset, "stale").is_err());
            assert!(delete_ladder_comparison(&source, offset + 1, &text).is_err());
            let deleted = delete_ladder_comparison(&source, offset, &text).unwrap();
            let before = EditableProgram::parse(&source).unwrap();
            let after = EditableProgram::parse(&deleted).unwrap();
            assert_eq!(
                before.rows[0].records.len(),
                after.rows[0].records.len() + spec.operand_count + 1
            );
            assert!(
                after.rows[0]
                    .records
                    .iter()
                    .all(|r| !r.bytes.starts_with(&[0, 34]) && !matches!(r.bytes[1], 0x23 | 0x24))
            );
            assert!(delete_ladder_comparison(&deleted, offset, &text).is_err());
            let restored =
                insert_ladder_comparison(&deleted, 0, 1, spec.mnemonic, &operands).unwrap();
            assert_eq!(source, restored);
        }
        let source = insert_ladder_instruction(
            include_bytes!("../fixtures/ladder-edit/R10.bin"),
            0,
            "MOV",
            &["0".into(), "D100".into()],
        )
        .unwrap();
        let text = "MOV,0,D100";
        let encoded = text
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        let offset = source
            .windows(encoded.len())
            .position(|b| b == encoded)
            .unwrap()
            - 4;
        assert!(delete_ladder_comparison(&source, offset, text).is_err());
    }

    #[test]
    fn comparison_arity_changes_keep_native_left_edge_and_wiring() {
        let binary = include_bytes!("../fixtures/ladder-edit/instructions/comparison_dword.bin");
        let group =
            include_bytes!("../fixtures/ladder-edit/instructions/comparison_dword_group.bin");
        let grown =
            update_instruction_text(binary, 72, "D=,D000100,D000104", "DG=,D000100,D000104,1")
                .unwrap()
                .unwrap();
        assert_eq!(grown, group.as_slice());
        let shrunk =
            update_instruction_text(group, 72, "DG=,D000100,D000104,1", "D=,D000100,D000104")
                .unwrap()
                .unwrap();
        assert_eq!(shrunk, binary.as_slice());

        let empty = include_bytes!("../fixtures/ladder-edit/R10.bin");
        let source =
            insert_ladder_comparison(empty, 0, 0, "D=", &["D100".into(), "D104".into()]).unwrap();
        let source = edit_ladder_cell(
            &source,
            &crate::LadderCellEdit {
                raw_y: 0,
                column: 3,
                expected: None,
                replacement: Some(LadderEditElement {
                    kind: LadderEditKind::NormallyOpen,
                    operand: "M00000".into(),
                }),
            },
        )
        .unwrap();
        let offset = source
            .windows(4)
            .position(|b| b == [b'D', 0, b'=', 0])
            .unwrap()
            - 4;
        assert!(
            update_instruction_text(&source, offset, "D=,D100,D104", "DG=,D100,D104,1").is_err()
        );
    }

    #[test]
    fn all_comparison_families_reproduce_complete_native_saved_programs() {
        for (batch, expected) in [
            include_bytes!("../fixtures/ladder-edit/instructions/comparisons_first_half.bin").as_slice(),
            include_bytes!("../fixtures/ladder-edit/instructions/comparison_output.bin").as_slice(),
        ].into_iter().enumerate() {
            let mut bytes = include_bytes!("../fixtures/ladder-edit/instructions/operandless.bin").to_vec();
            for (index, spec) in crate::ladder_comparison_catalog().chunks(39).nth(batch).unwrap().iter().enumerate() {
                let y = 8 + index as u32 * 4;
                bytes = insert_ladder_row(&bytes, y).unwrap();
                let bit = spec.mnemonic.starts_with('4') || spec.mnemonic.starts_with('8');
                let mut operands = if bit { vec!["D100.0".into(), "D102.0".into()] } else { vec!["D100".into(), "D104".into()] };
                if spec.operand_count == 3 {
                    operands.push(if spec.mnemonic.starts_with('G') || spec.mnemonic.starts_with("DG") { "1" } else { "D108" }.into());
                }
                bytes = insert_ladder_comparison(&bytes, y, 0, spec.mnemonic, &operands).unwrap();
                bytes = insert_ladder_instruction(&bytes, y, "MOV", &["1".into(), format!("D{}", 200 + index)]).unwrap();
            }
            assert_eq!(bytes, expected);
        }
    }

    #[test]
    fn application_deletion_and_basic_brst_match_complete_native_payloads() {
        let before = include_bytes!("../fixtures/ladder-edit/instructions/comparison_output.bin");
        let deleted =
            include_bytes!("../fixtures/ladder-edit/instructions/comparison_output_deleted.bin");
        let brst = include_bytes!("../fixtures/ladder-edit/instructions/basic_brst.bin");
        let encoded = "MOV,1,D200"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        let offset = before
            .windows(encoded.len())
            .position(|w| w == encoded)
            .unwrap()
            - 4;
        let terminal = include_bytes!("../fixtures/ladder-edit/instructions/operandless.bin");
        assert!(delete_ladder_instruction(terminal, 387, "END").is_err());
        assert_eq!(
            delete_ladder_instruction(before, offset, "MOV,1,D200").unwrap(),
            deleted.as_slice()
        );
        assert!(delete_ladder_instruction(before, offset, "MOV,2,D200").is_err());
        assert!(delete_ladder_instruction(before, offset + 1, "MOV,1,D200").is_err());
        assert_eq!(
            insert_ladder_instruction(deleted, 8, "BRST", &["M00020".into(), "8".into()]).unwrap(),
            brst.as_slice()
        );
    }

    #[test]
    fn string_literal_applications_reproduce_complete_native_saved_program() {
        let mut bytes = include_bytes!("../fixtures/ladder-edit/instructions/operandless.bin").to_vec();
        for (old, new) in [("STOP", "$MOVP,'Room B, off',D000200"), ("WDT", "$MOV,'Room A, on',D000100")] {
            let encoded = old.encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>();
            let offset = bytes.windows(encoded.len()).position(|b| b == encoded).unwrap() - 4;
            bytes = update_instruction_text(&bytes, offset, old, new).unwrap().unwrap();
        }
        assert_eq!(bytes, include_bytes!("../fixtures/ladder-edit/instructions/string_literals.bin").as_slice());
    }

    #[test]
    fn quoted_string_operands_remain_single_native_tokens() {
        let empty = include_bytes!("../fixtures/ladder-edit/R10.bin");
        let bytes = insert_ladder_instruction(empty, 0, "$MOV", &["'Room A, on'".into(), "D100".into()]).unwrap();
        let text = "$MOV,'Room A, on',D100";
        let encoded = text.encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>();
        let offset = bytes.windows(encoded.len()).position(|b| b == encoded).unwrap() - 4;
        let edited = update_instruction_text(&bytes, offset, text, "$MOV,'Room B, off',D104").unwrap().unwrap();
        let restored = update_instruction_text(&edited, offset, "$MOV,'Room B, off',D104", text).unwrap().unwrap();
        assert_eq!(restored, bytes);
        assert_eq!(crate::internal::parse_ladder_operation_call(text).unwrap().1, ["'Room A, on'", "D100"]);
        assert_eq!(crate::internal::parse_ladder_operation_call("$MOV 'Room A, on' D100").unwrap().1, ["'Room A, on'", "D100"]);
        assert!(crate::internal::parse_ladder_operation_call("$MOV,'unclosed,D100").is_none());
        assert!(insert_ladder_instruction(empty, 0, "$MOV", &["1".into(), "D100".into()]).is_err());
        assert!(insert_ladder_instruction(empty, 0, "MOV", &["'room'".into(), "D100".into()]).is_err());
        assert!(insert_ladder_instruction(empty, 0, "$MOV", &["D100".into(), "'destination'".into()]).is_err());
        assert!(insert_ladder_instruction(empty, 0, "$MOV", &[format!("'{}'", "a".repeat(32)), "D100".into()]).is_err());
        assert!(insert_ladder_instruction(empty, 0, "$MOV", &["'room\n'".into(), "D100".into()]).is_err());
    }

    #[test]
    fn indexed_bits_and_ff_reproduce_complete_native_saved_program() {
        let mut bytes = include_bytes!("../fixtures/ladder-edit/instructions/operandless.bin").to_vec();
        let offset = bytes.windows(8).position(|w| w == b"S\0T\0O\0P\0").unwrap() - 4;
        bytes = update_instruction_text(&bytes, offset, "STOP", "FF,M00030")
            .unwrap().unwrap();
        for (y, operand, mnemonic, operands) in [
            (0, "M00000", "B", ["D000100", "4"]),
            (4, "M00001", "BN", ["D000104", "D000108"]),
        ] {
            bytes = edit_ladder_cell(&bytes, &crate::LadderCellEdit {
                raw_y: y, column: 0,
                expected: Some(LadderEditElement { kind: LadderEditKind::NormallyOpen, operand: operand.into() }),
                replacement: None,
            }).unwrap();
            bytes = insert_ladder_comparison(&bytes, y, 0, mnemonic, &operands.map(String::from)).unwrap();
        }
        assert_eq!(bytes, include_bytes!("../fixtures/ladder-edit/instructions/indexed_bits_ff.bin").as_slice());
    }

    #[test]
    fn all_comparisons_preserve_contact_flags_and_native_opcodes() {
        let empty = include_bytes!("../fixtures/ladder-edit/R10.bin");
        for spec in crate::ladder_comparison_catalog() {
            let operands = catalog_test_operands(spec);
            let mut bytes =
                insert_ladder_comparison(empty, 0, 2, spec.mnemonic, &operands).unwrap();
            assert!(insert_ladder_comparison(&bytes, 0, 1, spec.mnemonic, &operands).is_err());
            for replacement in crate::ladder_comparison_catalog() {
                let parsed = EditableProgram::parse(&bytes).unwrap();
                let record = parsed.rows[0]
                    .records
                    .iter()
                    .find(|r| r.bytes.starts_with(&[0, 34]))
                    .unwrap();
                let text = string_at(&record.bytes, 19).unwrap().0;
                let encoded = text
                    .encode_utf16()
                    .flat_map(u16::to_le_bytes)
                    .collect::<Vec<_>>();
                let offset = bytes
                    .windows(encoded.len())
                    .position(|window| window == encoded)
                    .unwrap()
                    - 4;
                bytes = update_instruction_text(
                    &bytes,
                    offset,
                    &text,
                    &format!(
                        "{},{}",
                        replacement.mnemonic,
                        catalog_test_operands(replacement).join(",")
                    ),
                )
                .unwrap()
                .unwrap();
                let parsed = EditableProgram::parse(&bytes).unwrap();
                let record = parsed.rows[0]
                    .records
                    .iter()
                    .find(|r| r.bytes.starts_with(&[0, 34]))
                    .unwrap();
                assert_eq!(&record.bytes[9..15], &[1, 0, 0, 0, 0, 0]);
                assert_eq!(
                    u32::from_le_bytes(record.bytes[15..19].try_into().unwrap()),
                    replacement.opcode
                );
            }
        }
    }

    #[test]
    fn comparison_and_output_instruction_preserve_their_disjoint_spans() {
        let empty = include_bytes!("../fixtures/ladder-edit/R10.bin");
        let bytes =
            insert_ladder_comparison(empty, 0, 0, "=", &["D100".into(), "D102".into()]).unwrap();
        let bytes =
            insert_ladder_instruction(&bytes, 0, "MOV", &["0".into(), "D104".into()]).unwrap();
        let parsed = EditableProgram::parse(&bytes).unwrap();
        let instructions = parsed.rows[0]
            .records
            .iter()
            .filter(|r| r.bytes.starts_with(&[0, 34]))
            .collect::<Vec<_>>();
        assert_eq!(instructions.len(), 2);
        assert_eq!(instructions[0].bytes[11], 0);
        assert_eq!(instructions[1].bytes[11], 32);
        assert_eq!(
            parsed.rows[0]
                .records
                .iter()
                .find(|r| r.wire_end.is_some())
                .unwrap()
                .x,
            10
        );
        assert!(
            insert_ladder_comparison(&bytes, 0, 1, ">=", &["D100".into(), "D102".into()]).is_err()
        );
        let text = string_at(&instructions[0].bytes, 19).unwrap().0;
        let offset = bytes
            .windows(text.len() * 2)
            .position(|window| {
                window
                    == text
                        .encode_utf16()
                        .flat_map(u16::to_le_bytes)
                        .collect::<Vec<_>>()
            })
            .unwrap()
            - 4;
        let updated = update_instruction_text(&bytes, offset, &text, ">=,D100,D102")
            .unwrap()
            .unwrap();
        assert_eq!(
            EditableProgram::parse(&updated).unwrap().rows[0]
                .records
                .iter()
                .find(|r| r.bytes.starts_with(&[0, 34]))
                .unwrap()
                .bytes[11],
            0
        );
    }

    // These catalog-wide tests verify encoding, using device operands admitted by
    // the manual instead of constants in writable operands.
    fn catalog_test_operands(spec: &crate::LadderInstructionSpec) -> Vec<String> {
        let rules = crate::ladder_instruction_operand_rules(spec.mnemonic);
        (0..spec.operand_count)
            .map(|i| {
                let Some(rule) = rules.get(i) else {
                    return "D100".to_owned();
                };
                let Some(areas) = rule.device_areas else {
                    return "D100".to_owned();
                };
                if areas.contains(&"D") {
                    return "D100".to_owned();
                }
                if areas.contains(&"D.x") {
                    return "D100.0".to_owned();
                }
                if areas.contains(&"PMK") {
                    return "M100".to_owned();
                }
                if let Some(area) = areas.first() {
                    return if *area == "U" {
                        "U00.00".to_owned()
                    } else {
                        format!("{area}100")
                    };
                }
                "0".to_owned()
            })
            .collect()
    }

    #[test]
    fn operandless_applications_match_native_capture_and_keep_end_protected() {
        let native = include_bytes!("../fixtures/ladder-edit/instructions/operandless.bin");
        for (row, name) in [(0, "STOP"), (1, "WDT")] {
            let mut source = EditableProgram::parse(native).unwrap();
            source.rows[row]
                .records
                .retain(|record| !record.bytes.starts_with(&[0, 34]));
            let actual =
                insert_ladder_instruction(&source.encode(), row as u32 * 4, name, &[]).unwrap();
            assert_eq!(actual, native.as_slice(), "{name} native payload differs");
        }
        let offset = 122;
        let enlarged = update_instruction_text(native, offset, "STOP", "MOV,1,D100")
            .unwrap()
            .unwrap();
        let restored = update_instruction_text(&enlarged, offset, "MOV,1,D100", "STOP")
            .unwrap()
            .unwrap();
        assert_eq!(restored, native.as_slice());
        assert!(update_instruction_text(native, 387, "END", "STOP").is_err());
    }

    #[test]
    fn instruction_insertion_validates_catalog_and_operand_count() {
        let empty = include_bytes!("../fixtures/ladder-edit/R10.bin");
        for spec in crate::ladder_instruction_catalog() {
            let operands = catalog_test_operands(spec);
            let bytes = insert_ladder_instruction(empty, 0, spec.mnemonic, &operands).unwrap();
            let parsed = EditableProgram::parse(&bytes).unwrap();
            let instruction = parsed.rows[0]
                .records
                .iter()
                .find(|record| record.bytes.starts_with(&[0, 34]))
                .unwrap();
            assert_eq!(
                u32::from_le_bytes(instruction.bytes[15..19].try_into().unwrap()),
                spec.opcode
            );
            assert_eq!(
                string_at(&instruction.bytes, 19).unwrap().0,
                std::iter::once(spec.mnemonic)
                    .chain(operands.iter().map(String::as_str))
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
        for (name, operands) in [
            ("UNKNOWN", vec!["0".into()]),
            ("MOV", vec!["0".into()]),
            ("MOV", vec!["0".into(), "D 1".into()]),
        ] {
            assert!(insert_ladder_instruction(empty, 0, name, &operands).is_err());
        }
    }

    fn element(kind: LadderEditKind, operand: &str) -> Option<LadderEditElement> {
        Some(LadderEditElement {
            kind,
            operand: operand.into(),
        })
    }
    #[test]
    fn instruction_edits_synchronize_tokens_and_preserve_other_records() {
        let original = crate::XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
        let data = original.ladder_programs().remove(0).unwrap();
        let before = EditableProgram::parse(&data.data).unwrap();
        let old = "MOV,0,D000000";
        let offset = data.strings.iter().find(|s| s.value == old).unwrap().offset;
        for replacement in [
            "MOV,1,D1",
            "MOV,12345,D000042",
            "MOV,12345,D1",
            "MOV,12345,D42",
            "MOV,1,D000042",
        ] {
            let mut doc = original.clone();
            doc.update_ladder_cell_text(0, offset, old, replacement)
                .unwrap();
            let doc = crate::XgwxDocument::parse(&doc.to_bytes().unwrap()).unwrap();
            let edited = doc.ladder_programs().remove(0).unwrap();
            let after = EditableProgram::parse(&edited.data).unwrap();
            assert_eq!(before.header, after.header);
            assert_eq!(before.group_headers, after.group_headers);
            let mut changed = 0;
            for (a, b) in before.rows.iter().zip(&after.rows) {
                assert_eq!(a.prefix, b.prefix);
                assert_eq!(a.records.len(), b.records.len());
                for (a, b) in a.records.iter().zip(&b.records) {
                    if a.bytes == b.bytes {
                        continue;
                    }
                    changed += 1;
                    assert_eq!(&a.bytes[..19], &b.bytes[..19]);
                    let (combined, end) = string_at(&b.bytes, 19).unwrap();
                    assert_eq!(combined, replacement);
                    let mut next = end + 2;
                    for part in replacement.split(',') {
                        let (stored, end) = string_at(&b.bytes, next + 10).unwrap();
                        assert_eq!(stored, part);
                        next = end;
                    }
                    assert_eq!(next, b.bytes.len());
                }
            }
            assert_eq!(changed, 1);
            let new_offset = edited
                .strings
                .iter()
                .find(|s| s.value == replacement)
                .unwrap()
                .offset;
            let mut restored = doc;
            restored
                .update_ladder_cell_text(0, new_offset, replacement, old)
                .unwrap();
            assert_eq!(
                restored.ladder_programs().remove(0).unwrap().data,
                data.data
            );
        }
    }

    #[test]
    fn native_mov_to_add_instruction_replacement_matches() {
        let mut doc = crate::XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
        let data = doc.ladder_programs().remove(0).unwrap();
        let original = "MOV,0,D000000";
        let offset = data
            .strings
            .iter()
            .find(|s| s.value == original)
            .unwrap()
            .offset;
        doc.update_ladder_cell_text(0, offset, original, "ADD,1,2,D000000")
            .unwrap();
        let actual = doc.ladder_programs().remove(0).unwrap().data;
        let source = EditableProgram::parse(&data.data).unwrap();
        let mut native = EditableProgram::parse(include_bytes!(
            "../fixtures/ladder-edit/instructions/R70.bin"
        ))
        .unwrap();
        // Native interactive editing recalculates display heights for all rows.
        // The writer deliberately preserves those unrelated presentation values.
        for (row, original) in native.rows.iter_mut().zip(&source.rows) {
            row.prefix[17..21].copy_from_slice(&original.prefix[17..21]);
        }
        assert_eq!(actual, native.encode());
    }

    #[test]
    fn instruction_catalog_replacements_resize_and_restore() {
        let doc = crate::XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
        let data = doc.ladder_programs().remove(0).unwrap();
        let old = "MOV,0,D000000";
        let offset = data.strings.iter().find(|s| s.value == old).unwrap().offset;
        let catalog = crate::ladder_instruction_catalog();
        assert!(catalog.len() > 800);
        for spec in catalog {
            let operands = catalog_test_operands(spec);
            let text = std::iter::once(spec.mnemonic)
                .chain(operands.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join(",");
            let edited = update_instruction_text(&data.data, offset, old, &text)
                .unwrap_or_else(|e| panic!("{}: {e}", spec.mnemonic))
                .unwrap();
            let parsed = EditableProgram::parse(&edited).unwrap();
            let record = parsed
                .rows
                .iter()
                .flat_map(|r| &r.records)
                .find(|r| {
                    r.bytes.starts_with(&[0, 34]) && string_at(&r.bytes, 19).unwrap().0 == text
                })
                .unwrap();
            assert_eq!(
                u32::from_le_bytes(record.bytes[15..19].try_into().unwrap()),
                spec.opcode
            );
            let new_offset = edited
                .windows(19)
                .position(|w| w == &record.bytes[..19])
                .unwrap()
                + 19;
            let restored = update_instruction_text(&edited, new_offset, &text, old)
                .unwrap()
                .unwrap();
            assert_eq!(restored, data.data, "{}", spec.mnemonic);
        }
    }

    #[test]
    fn native_add_to_ton_instruction_replacement_matches() {
        let source = include_bytes!("../fixtures/ladder-edit/instructions/R70.bin");
        let old = "ADD,1,2,D000000";
        let mut encoded = Vec::new();
        append_string(&mut encoded, old);
        let offset = source
            .windows(encoded.len())
            .position(|w| w == encoded)
            .unwrap();
        let actual = update_instruction_text(source, offset, old, "TON,T0000,100")
            .unwrap()
            .unwrap();
        assert_eq!(
            actual,
            include_bytes!("../fixtures/ladder-edit/instructions/R71.bin")
        );
    }

    #[test]
    fn invalid_instruction_edits_are_atomic() {
        let mut doc = crate::XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
        let original = doc.to_bytes().unwrap();
        let data = doc.ladder_programs().remove(0).unwrap();
        let old = "MOV,0,D000000";
        let offset = data.strings.iter().find(|s| s.value == old).unwrap().offset;
        for text in [
            "ADD,0,D000000",
            "MOV,0",
            "MOV,0,D1,D2",
            "MOV,,D1",
            "MOV,0,D 1",
            "MOV,0,한글",
            &format!("MOV,0,{}", "D".repeat(250)),
        ] {
            assert!(
                doc.update_ladder_cell_text(0, offset, old, text).is_err(),
                "{text}"
            );
            assert_eq!(doc.to_bytes().unwrap(), original);
        }
        assert!(
            doc.update_ladder_cell_text(0, offset, "MOV,9,D000000", "MOV,1,D1")
                .is_err()
        );
        let inner = data
            .strings
            .iter()
            .find(|s| s.offset > offset && s.value == "D000000")
            .unwrap();
        assert!(
            doc.update_ladder_cell_text(0, inner.offset, &inner.value, "D000042")
                .is_err()
        );
        assert_eq!(doc.to_bytes().unwrap(), original);
    }

    #[test]
    fn structural_edits_match_native_xg5000_records() {
        let empty = include_bytes!("../fixtures/ladder-edit/R10.bin");
        let contact = include_bytes!("../fixtures/ladder-edit/R11.bin");
        let coil = include_bytes!("../fixtures/ladder-edit/R12.bin");
        let deleted = include_bytes!("../fixtures/ladder-edit/R14.bin");
        let nc = include_bytes!("../fixtures/ladder-edit/R15.bin");
        let restored = include_bytes!("../fixtures/ladder-edit/R16.bin");
        for (source, column, expected, replacement, target) in [
            (
                empty.as_slice(),
                0,
                None,
                element(LadderEditKind::NormallyOpen, "M00000"),
                contact.as_slice(),
            ),
            (
                contact.as_slice(),
                9,
                None,
                element(LadderEditKind::Output, "M00010"),
                coil.as_slice(),
            ),
            (
                coil.as_slice(),
                0,
                element(LadderEditKind::NormallyOpen, "M00000"),
                None,
                deleted.as_slice(),
            ),
            (
                deleted.as_slice(),
                1,
                None,
                element(LadderEditKind::NormallyClosed, "M00001"),
                nc.as_slice(),
            ),
            (
                nc.as_slice(),
                0,
                None,
                element(LadderEditKind::NormallyOpen, "M00000"),
                restored.as_slice(),
            ),
        ] {
            assert_eq!(
                edit_ladder_cell(
                    source,
                    &LadderCellEdit {
                        raw_y: 0,
                        column,
                        expected,
                        replacement
                    }
                )
                .unwrap(),
                target
            );
        }
    }

    #[test]
    fn every_decoded_structural_kind_round_trips_its_native_record() {
        let doc = crate::XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
        let data = doc.ladder_programs().remove(0).unwrap().data;
        let parsed = EditableProgram::parse(&data).unwrap();
        let elements = parsed
            .rows
            .iter()
            .flat_map(|row| {
                row.records.iter().filter_map(move |record| {
                    record.element.clone().map(|element| {
                        let column = if element.kind.is_coil() {
                            9
                        } else {
                            (record.x - 1) / 3
                        };
                        (row.y, column, element)
                    })
                })
            })
            .collect::<Vec<_>>();

        for kind in [
            LadderEditKind::NormallyOpen,
            LadderEditKind::NormallyClosed,
            LadderEditKind::AddressedRisingPulse,
            LadderEditKind::AddressedFallingPulse,
            LadderEditKind::AddressedRisingPulseNot,
            LadderEditKind::AddressedFallingPulseNot,
            LadderEditKind::Inverse,
            LadderEditKind::RisingPulse,
            LadderEditKind::FallingPulse,
            LadderEditKind::Output,
            LadderEditKind::InverseOutput,
            LadderEditKind::Set,
            LadderEditKind::Reset,
            LadderEditKind::RisingPulseOutput,
            LadderEditKind::FallingPulseOutput,
        ] {
            assert!(elements.iter().any(|(_, _, element)| element.kind == kind));
        }

        for (raw_y, column, element) in elements {
            assert_eq!(
                edit_ladder_cell(
                    &data,
                    &LadderCellEdit {
                        raw_y,
                        column,
                        expected: Some(element.clone()),
                        replacement: Some(element),
                    },
                )
                .unwrap(),
                data,
            );
        }
    }

    #[test]
    fn comments_edit_and_create_native_records() {
        let doc = crate::XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
        let data = doc.ladder_programs().remove(0).unwrap().data;
        for (kind, raw_y, text) in [
            (LadderCommentKind::Rung, 0, "렁 설명문 1"),
            (LadderCommentKind::Output, 4, "출력 설명문 1"),
        ] {
            assert_eq!(
                edit_ladder_comment(
                    &data,
                    &LadderCommentEdit {
                        kind,
                        raw_y,
                        expected: Some(text.to_owned()),
                        replacement: text.to_owned(),
                    },
                )
                .unwrap(),
                data,
            );
        }
        let without_rung = delete_ladder_rung_comment(&data, 0, "렁 설명문 1").unwrap();
        let restored = edit_ladder_comment(
            &without_rung,
            &LadderCommentEdit {
                kind: LadderCommentKind::Rung,
                raw_y: 0,
                expected: None,
                replacement: "렁 설명문 1".to_owned(),
            },
        )
        .unwrap();
        assert_eq!(restored, data);
        assert!(delete_ladder_rung_comment(&data, 0, "stale").is_err());
        assert!(delete_ladder_rung_comment(&data, 1, "렁 설명문 1").is_err());

        let edited = edit_ladder_comment(
            &data,
            &LadderCommentEdit {
                kind: LadderCommentKind::Output,
                raw_y: 4,
                expected: Some("출력 설명문 1".to_owned()),
                replacement: "Updated output comment".to_owned(),
            },
        )
        .unwrap();
        let parsed = EditableProgram::parse(&edited).unwrap();
        let output = parsed
            .rows
            .iter()
            .find(|row| row.y == 4)
            .unwrap()
            .records
            .iter()
            .find(|record| record.bytes.starts_with(&[255, 0x40]))
            .unwrap();
        assert_eq!(
            string_at(&output.bytes, 15).unwrap().0,
            "Updated output comment"
        );

        let empty = include_bytes!("../fixtures/ladder-edit/R10.bin");
        let rung = edit_ladder_comment(
            empty,
            &LadderCommentEdit {
                kind: LadderCommentKind::Rung,
                raw_y: 0,
                expected: None,
                replacement: "New rung comment".to_owned(),
            },
        )
        .unwrap();
        let parsed = EditableProgram::parse(&rung).unwrap();
        assert_eq!(u16_at(&parsed.header, 4).unwrap(), 1);
        assert_eq!(parsed.rows[0].prefix[13], 1);
        assert_eq!(
            string_at(&parsed.rows[0].records[0].bytes, 15).unwrap().0,
            "New rung comment"
        );
        let deleted = delete_ladder_rung_comment(&rung, 0, "New rung comment").unwrap();
        assert_eq!(deleted, empty);

        let output = edit_ladder_comment(
            empty,
            &LadderCommentEdit {
                kind: LadderCommentKind::Output,
                raw_y: 0,
                expected: None,
                replacement: "New output comment".to_owned(),
            },
        )
        .unwrap();
        let parsed = EditableProgram::parse(&output).unwrap();
        assert_eq!(parsed.rows[0].prefix[13], 0);
        assert_eq!(
            string_at(&parsed.rows[0].records[0].bytes, 15).unwrap().0,
            "New output comment"
        );
    }

    #[test]
    fn comment_edits_reject_stale_empty_and_unsafe_creations() {
        let doc = crate::XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
        let data = doc.ladder_programs().remove(0).unwrap().data;
        for edit in [
            LadderCommentEdit {
                kind: LadderCommentKind::Rung,
                raw_y: 0,
                expected: Some("stale".to_owned()),
                replacement: "Updated".to_owned(),
            },
            LadderCommentEdit {
                kind: LadderCommentKind::Output,
                raw_y: 4,
                expected: None,
                replacement: "Duplicate".to_owned(),
            },
            LadderCommentEdit {
                kind: LadderCommentKind::Output,
                raw_y: 4,
                expected: Some("출력 설명문 1".to_owned()),
                replacement: "  ".to_owned(),
            },
            LadderCommentEdit {
                kind: LadderCommentKind::Rung,
                raw_y: 24,
                expected: None,
                replacement: "Inside branch".to_owned(),
            },
        ] {
            assert!(edit_ladder_comment(&data, &edit).is_err());
        }
    }
    #[test]
    fn deletion_preserves_native_high_water_coordinate() {
        assert_eq!(
            edit_ladder_cell(
                include_bytes!("../fixtures/ladder-edit/R17.bin"),
                &LadderCellEdit {
                    raw_y: 0,
                    column: 1,
                    expected: element(LadderEditKind::NormallyClosed, "M00001"),
                    replacement: None,
                }
            )
            .unwrap(),
            include_bytes!("../fixtures/ladder-edit/R18.bin")
        );
    }

    #[test]
    fn invalid_positions_operands_and_terminal_edits_are_rejected() {
        let source = include_bytes!("../fixtures/ladder-edit/R17.bin");
        for (raw_y, column, replacement) in [
            (1, 2, element(LadderEditKind::NormallyOpen, "M1")),
            (65535 * 4, 2, element(LadderEditKind::NormallyOpen, "M1")),
            (0, 10, element(LadderEditKind::NormallyOpen, "M1")),
            (0, 2, element(LadderEditKind::Output, "M1")),
            (0, 2, element(LadderEditKind::NormallyOpen, "MOV M1 M2")),
            (4, 9, element(LadderEditKind::Output, "M1")),
        ] {
            assert!(
                edit_ladder_cell(
                    source,
                    &LadderCellEdit {
                        raw_y,
                        column,
                        expected: None,
                        replacement,
                    }
                )
                .is_err()
            );
        }
    }

    #[test]
    fn branched_fixture_round_trip_preserves_unedited_records() {
        let doc = crate::XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
        let data = doc.ladder_programs().remove(0).unwrap().data;
        let parsed = EditableProgram::parse(&data).unwrap();
        assert_eq!(parsed.encode(), data);
        for (y, operand) in [
            (4, "M00000"),
            (8, "P00000"),
            (12, "P00001"),
            (16, "P00002"),
            (20, "M00010"),
            (44, "F00091"),
        ] {
            let out = edit_ladder_cell(
                &data,
                &LadderCellEdit {
                    raw_y: y,
                    column: 0,
                    expected: element(LadderEditKind::NormallyOpen, operand),
                    replacement: element(LadderEditKind::NormallyClosed, "M42"),
                },
            )
            .unwrap();
            let after = EditableProgram::parse(&out).unwrap();
            for (before, row) in parsed.rows.iter().zip(after.rows.iter()) {
                if row.y != y {
                    assert_eq!(before.prefix, row.prefix);
                    assert_eq!(
                        before.records.iter().map(|r| &r.bytes).collect::<Vec<_>>(),
                        row.records.iter().map(|r| &r.bytes).collect::<Vec<_>>()
                    );
                } else {
                    assert_eq!(
                        before
                            .records
                            .iter()
                            .skip(1)
                            .map(|r| &r.bytes)
                            .collect::<Vec<_>>(),
                        row.records
                            .iter()
                            .skip(1)
                            .map(|r| &r.bytes)
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
    }

    #[test]
    fn native_branch_connections_and_row_insertions_match() {
        let linear = include_bytes!("../fixtures/ladder-edit/R17.bin");
        let connected = include_bytes!("../fixtures/ladder-edit/branches/R40.bin");
        let stretched = include_bytes!("../fixtures/ladder-edit/branches/R41.bin");
        let blank = include_bytes!("../fixtures/ladder-edit/branches/R42.bin");
        let branch = include_bytes!("../fixtures/ladder-edit/branches/R43.bin");
        let contact = include_bytes!("../fixtures/ladder-edit/branches/R44.bin");
        for data in [
            connected.as_slice(),
            stretched.as_slice(),
            blank.as_slice(),
            branch.as_slice(),
            contact.as_slice(),
        ] {
            assert_eq!(EditableProgram::parse(data).unwrap().encode(), data);
        }
        let edit = LadderBranchEdit {
            raw_y: 0,
            boundary: 1,
            expected: false,
            present: true,
        };
        assert_eq!(edit_ladder_branch(linear, &edit).unwrap(), connected);
        assert_eq!(insert_ladder_row(connected, 4).unwrap(), stretched);
        assert_eq!(insert_ladder_row(linear, 4).unwrap(), blank);
        assert_eq!(edit_ladder_branch(blank, &edit).unwrap(), branch);
        assert_eq!(
            edit_ladder_cell(
                branch,
                &LadderCellEdit {
                    raw_y: 4,
                    column: 0,
                    expected: None,
                    replacement: element(LadderEditKind::NormallyOpen, "M00002")
                }
            )
            .unwrap(),
            contact
        );
    }

    #[test]
    fn branch_rejections_leave_document_unchanged() {
        let mut doc = crate::XgwxDocument::from_path("fixtures/ladder-edit/linear.xgwx").unwrap();
        let original = doc.to_bytes().unwrap();
        for edit in [
            LadderBranchEdit {
                raw_y: 0,
                boundary: 0,
                expected: false,
                present: true,
            },
            LadderBranchEdit {
                raw_y: 4,
                boundary: 1,
                expected: false,
                present: true,
            },
            LadderBranchEdit {
                raw_y: 0,
                boundary: 1,
                expected: true,
                present: false,
            },
        ] {
            assert!(doc.edit_ladder_branch(0, &edit).is_err());
            assert_eq!(doc.to_bytes().unwrap(), original);
        }
        assert!(doc.insert_ladder_row(0, 3).is_err());
        assert_eq!(doc.to_bytes().unwrap(), original);
        let mut malformed = include_bytes!("../fixtures/ladder-edit/branches/R44.bin").to_vec();
        let start = malformed
            .windows(7)
            .position(|w| w == [0, 0, 0, 0, 0, 2, 0])
            .unwrap();
        malformed[start + 18] = 8;
        assert!(EditableProgram::parse(&malformed).is_err());
    }

    #[test]
    fn connection_segment_can_be_removed_after_row_insertion() {
        let source = include_bytes!("../fixtures/ladder-edit/branches/R44.bin");
        let stretched = insert_ladder_row(source, 4).unwrap();
        let edit = LadderBranchEdit {
            raw_y: 0,
            boundary: 1,
            expected: true,
            present: false,
        };
        let removed = edit_ladder_branch(&stretched, &edit).unwrap();
        let program = EditableProgram::parse(&removed).unwrap();
        assert!(
            !program
                .rows
                .iter()
                .any(|row| row.y == 0 && row.records.iter().any(|r| r.bytes.starts_with(&[0, 0])))
        );
        assert!(program.rows.iter().any(|row| {
            row.y == 4
                && row
                    .records
                    .iter()
                    .any(|r| r.bytes.starts_with(&[0, 0]) && r.bytes[18] == 8)
        }));
        let restored = edit_ladder_branch(
            &removed,
            &LadderBranchEdit {
                expected: false,
                present: true,
                ..edit
            },
        )
        .unwrap();
        assert_eq!(restored, stretched);
    }

    #[test]
    fn removing_a_connection_removes_its_rendered_wire() {
        let doc = crate::XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
        let data = doc.ladder_programs().remove(0).unwrap().data;
        let bytes = edit_ladder_branch(
            &data,
            &LadderBranchEdit {
                raw_y: 8,
                boundary: 1,
                expected: true,
                present: false,
            },
        )
        .unwrap();
        let (_, vertical) = crate::ladder_records::exact_geometry(&bytes).unwrap();
        assert!(
            !vertical
                .iter()
                .any(|line| line.raw_x == 3 && line.raw_y_start == 8)
        );
        assert!(
            vertical
                .iter()
                .any(|line| line.raw_x == 6 && line.raw_y_start == 8)
        );
        let blank = include_bytes!("../fixtures/ladder-edit/branches/R44.bin");
        let (horizontal, _) = crate::ladder_records::exact_geometry(blank).unwrap();
        assert!(!horizontal.iter().any(|line| line.raw_y == 4));
    }

    #[test]
    fn outputs_on_lower_branch_rows_match_native_saved_program() {
        let document = crate::XgwxDocument::parse(include_bytes!("../fixtures/elements.xgwx")).unwrap();
        let source = document.ladder_programs().remove(0).unwrap().data;
        let coil = edit_ladder_cell(&source, &LadderCellEdit {
            raw_y: 12, column: 9, expected: None,
            replacement: element(LadderEditKind::Output, "M00030"),
        }).unwrap();
        let actual = insert_ladder_instruction(&coil, 16, "MOV", &["1".into(), "D100".into()]).unwrap();
        let native = include_bytes!("../fixtures/ladder-edit/branches/branch_outputs.bin");
        assert_eq!(actual, native.as_slice());
        assert!(editable_ladder_supported(&actual));
    }

    #[test]
    fn rejects_truncated_and_unknown_layouts() {
        let source = include_bytes!("../fixtures/ladder-edit/R16.bin");
        for end in 0..source.len() {
            assert!(!editable_ladder_supported(&source[..end]));
        }
        let mut unknown = source.to_vec();
        unknown[54] = 0x7f;
        assert!(!editable_ladder_supported(&unknown));
        let mut trailing = source.to_vec();
        trailing.push(0);
        assert!(!editable_ladder_supported(&trailing));
    }
}
