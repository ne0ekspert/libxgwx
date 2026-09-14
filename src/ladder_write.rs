//! Structural editing for decoded LD rows and branch references.
use crate::{XgwxError, ladder_records::*};

/// Optimistic edit at a physical cell. `expected: None` means no element;
/// `replacement: None` removes the actual record, leaving a wiring gap.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct LadderCellEdit {
    pub raw_y: u8,
    pub column: u8,
    pub expected: Option<LadderEditElement>,
    pub replacement: Option<LadderEditElement>,
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
    let original_parts: Vec<_> = original.split(',').collect();
    if original != expected || original_parts.len() != count {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "instruction text does not match its stored operands",
        });
    }
    let parts: Vec<_> = replacement.split(',').map(str::trim).collect();
    let type_changed = parts.first() != original_parts.first();
    if count < 2 {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "terminal instructions cannot be replaced",
        });
    }
    let opcode = if type_changed {
        let definition = crate::ladder_instruction_catalog()
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
        .any(|part| part.is_empty() || !part.chars().all(|c| c.is_ascii_graphic()))
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "instruction operands must be nonempty printable ASCII tokens separated by commas",
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
    let new_x = right
        .checked_sub((parts.len() - 1) * 3)
        .filter(|x| *x >= 1)
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "not enough space for the replacement instruction",
        })? as u8;
    if type_changed
        && argument_headers
            .iter()
            .enumerate()
            .any(|(i, h)| h[4..] != [u8::from(i == 0), 0, 32, 0, 0, 0])
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
                program.rows[row].y,
                0,
                0,
                u8::from(index == 0),
                0,
                32,
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
                    row.y,
                    0,
                    0,
                ],
                x: new_x,
                wire_end: None,
                element: None,
            });
        }
        rebuilt.sort_by_key(|r| r.x);
        row.records = rebuilt;
        merge_wires(row);
        if row.prefix[21] == record.x {
            row.prefix[21] = new_x;
        }
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

fn wire(x: u8, end: u8, y: u8) -> Record {
    let mut bytes = vec![255, 2, 0, 0, 0, x, y, 0, 0, 0, 0, 0, 0, 0, 0];
    bytes.extend([end, y, 0, 0]);
    Record {
        bytes,
        x,
        wire_end: Some(end),
        element: None,
    }
}
fn element_record(x: u8, y: u8, element: &LadderEditElement) -> Record {
    let mut bytes = vec![
        255,
        element.kind.marker(),
        0,
        0,
        0,
        x,
        y,
        0,
        0,
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

/// Physical rows, including sparse blanks, excluding comment-only rows.
#[cfg(feature = "wasm")]
pub(crate) fn editable_ladder_rows(bytes: &[u8]) -> Result<Vec<u8>, XgwxError> {
    let program = EditableProgram::parse(bytes)?;
    Ok((0..u16_at(&program.header, 4)?)
        .map(|index| (index * 4) as u8)
        .filter(|y| !program.rows.iter().any(|r| r.y == *y && r.prefix[13] != 0))
        .collect())
}

#[cfg(any(feature = "wasm", test))]
pub(crate) fn editable_ladder_supported(bytes: &[u8]) -> bool {
    EditableProgram::parse(bytes).is_ok()
}

/// Exact vertical reference segments; unlike rendered lines these are not merged.
#[cfg(feature = "wasm")]
pub(crate) fn ladder_connections(bytes: &[u8]) -> Result<Vec<(u8, u8, u8)>, XgwxError> {
    let program = EditableProgram::parse(bytes)?;
    Ok(program
        .rows
        .iter()
        .flat_map(|row| {
            row.records
                .iter()
                .filter(|r| r.bytes.starts_with(&[0, 0]))
                .map(|r| (r.x, row.y, r.bytes[18]))
        })
        .collect())
}

pub(crate) fn edit_ladder_cell(bytes: &[u8], edit: &LadderCellEdit) -> Result<Vec<u8>, XgwxError> {
    if edit.column > 9 || !edit.raw_y.is_multiple_of(4) {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "invalid row or column",
        });
    }
    if let Some(element) = &edit.replacement {
        if element.kind.has_operand() {
            let address = element.operand.as_bytes();
            if address.len() < 2
                || address.len() > 32
                || !matches!(address[0], b'P' | b'M' | b'K' | b'F' | b'L' | b'T' | b'C')
                || !address[1..].iter().all(u8::is_ascii_digit)
            {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "use an uppercase P/M/K/F/L/T/C device address",
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
    if edit.expected.is_none()
        && edit.replacement.is_some()
        && usize::from(edit.raw_y) / 4 < u16_at(&program.header, 4)?
    {
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
            .any(|r| r.bytes.starts_with(&[0, 34]) && r.x <= x)
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
                .map(|r| r.wire_end.unwrap_or(r.x) + 3)
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
    pub raw_y: u8,
    pub boundary: u8,
    pub expected: bool,
    pub present: bool,
}

fn blank_row(y: u8) -> Row {
    let mut prefix = vec![0; 35];
    prefix[..4].copy_from_slice(&(u32::from(y) / 4).to_le_bytes());
    prefix[4..6].copy_from_slice(&[255, 67]);
    prefix[17] = 39;
    prefix[21..25].copy_from_slice(&[94, y, 0, 0]);
    prefix[25..29].copy_from_slice(&[94, y, 0, 0]);
    prefix[29..33].copy_from_slice(&[1, y, 0, 0]);
    Row {
        prefix,
        y,
        records: Vec::new(),
    }
}
fn branch_start(x: u8, y: u8, target_y: u8) -> Record {
    let bytes = vec![
        0,
        0,
        0,
        0,
        0,
        2,
        0,
        x,
        y,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        x - 1,
        target_y,
        0,
        0,
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
fn branch_end(x: u8, source_y: u8) -> Record {
    Record {
        bytes: vec![1, 0, 0, 0, 0, x, source_y, 0, 0],
        x,
        wire_end: None,
        element: None,
    }
}
impl EditableProgram {
    fn materialize_row(&mut self, y: u8) -> usize {
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
                    && let Some(last) = self.rows.iter().position(|r| r.y == record.bytes[18])
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
        || usize::from(y) / 4 + 1 >= u16_at(&program.header, 4)?
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
                .any(|r| r.bytes.starts_with(&[0, 0]) && r.x == x && r.bytes[18] == target_y)
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
                .any(|r| r.bytes.starts_with(&[0, 34]) && r.x <= x + 1)
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
                && r.bytes[18] > y
                && (row.y != y || r.bytes[18] != target_y)
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
                !(row.y == y && r.bytes.starts_with(&[0, 0]) && r.x == x && r.bytes[18] == target_y)
                    && !(row.y == target_y
                        && r.bytes.starts_with(&[1, 0])
                        && r.x == x
                        && r.bytes[6] == y)
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
pub(crate) fn insert_ladder_row(bytes: &[u8], raw_y: u8) -> Result<Vec<u8>, XgwxError> {
    let mut program = EditableProgram::parse(bytes)?;
    let count = u16_at(&program.header, 4)?;
    if !raw_y.is_multiple_of(4) || usize::from(raw_y) / 4 > count || count >= 61 {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "invalid insertion row or row limit reached",
        });
    }
    let mut crossings = Vec::new();
    for row in &program.rows {
        for r in &row.records {
            if r.bytes.starts_with(&[0, 0]) && row.y < raw_y && r.bytes[18] >= raw_y {
                crossings.push((r.x, row.y, r.bytes[18] + 4));
            }
        }
    }
    for row in &mut program.rows {
        let old_y = row.y;
        if old_y >= raw_y {
            row.y += 4;
            row.prefix[..4].copy_from_slice(&(u32::from(row.y) / 4).to_le_bytes());
            row.prefix[22] = row.y;
            row.prefix[30] = row.y;
        }
        for r in &mut row.records {
            if r.bytes.starts_with(&[0, 0]) {
                if r.bytes[8] >= raw_y {
                    r.bytes[8] += 4;
                }
                if r.bytes[18] >= raw_y {
                    r.bytes[18] += 4;
                }
            } else if r.bytes.starts_with(&[1, 0]) {
                if r.bytes[6] >= raw_y {
                    r.bytes[6] += 4;
                }
            } else if old_y >= raw_y {
                r.bytes[6] = row.y;
                if r.wire_end.is_some() {
                    r.bytes[16] = row.y;
                }
                if r.bytes.starts_with(&[0, 34]) {
                    let (_, end) = string_at(&r.bytes, 19)?;
                    let count = u16_at(&r.bytes, end)?;
                    let mut next = end + 2;
                    for _ in 0..count {
                        r.bytes[next + 1] = row.y;
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
                        && r.bytes[18] == target_y
                    {
                        r.bytes[18] = raw_y;
                    }
                    if row.y == target_y
                        && r.bytes.starts_with(&[1, 0])
                        && r.x == x
                        && r.bytes[6] == source_y
                    {
                        r.bytes[6] = raw_y;
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
    program.header[4..6].copy_from_slice(&((count + 1) as u16).to_le_bytes());
    program.rebuild_groups();
    let output = program.encode();
    EditableProgram::parse(&output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
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
            let text = format!(
                "{},{}",
                spec.mnemonic,
                vec!["0"; spec.operand_count].join(",")
            );
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
            (8, 2, element(LadderEditKind::NormallyOpen, "M1")),
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
