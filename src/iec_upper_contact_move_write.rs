//! Native upper MOVE on two continuing contact spines above a lower MOVE.
use crate::iec_function_write::{body_with_flags, branch_end, branch_start, expression, wire};
use crate::{IecRecordKind as K, LadderProgramData, XgwxError};

// Recognize the entire retained network, including the lower block and its pins.
// Coordinates are relative; operands are preserved rather than fixture-specific.
fn layout(program: &LadderProgramData, row: u16, filled: bool) -> Option<usize> {
    if row > u16::MAX / 4 - 5 {
        return None;
    }
    let rows = program.iec_row_frames()?;
    let top = rows.iter().find(|r| r.row_index == row)?;
    let group = rows
        .iter()
        .filter(|r| r.group_index == top.group_index)
        .collect::<Vec<_>>();
    if group.len() != 6
        || group
            .iter()
            .enumerate()
            .any(|(i, r)| r.row_index != row + i as u16)
        || program.data[top.start - 6..top.start - 2] != 1u32.to_le_bytes()
    {
        return None;
    }
    let records = program.iec_record_frames()?;
    let blocks = program.iec_function_blocks()?;
    let links = program.iec_function_operand_links()?;
    let refs = program.iec_function_references()?;
    let mut shapes = vec![
        vec![
            (K::Contact(6), 1),
            (K::BranchStart, 2),
            (K::ShortWire, 4),
            (K::ShortWire, 7),
            (K::BranchStart, 2),
            (K::Contact(6), 10),
        ],
        vec![
            (K::Contact(6), 1),
            (K::BranchEnd, 3),
            (K::BranchStart, 2),
            (K::BranchEnd, 9),
            (K::BranchStart, 2),
        ],
        vec![
            (K::Contact(6), 1),
            (K::BranchEnd, 3),
            (K::BranchEnd, 9),
            (K::BranchStart, 2),
        ],
        vec![
            (K::BranchEnd, 9),
            (K::ShortWire, 10),
            (K::ShortWire, 13),
            (K::FunctionBlock, 16),
        ],
        vec![
            (K::FunctionOperand, 13),
            (K::LinkReference(0x68), 16),
            (K::FunctionOperand, 19),
        ],
        vec![(K::LinkReference(0x69), 16)],
    ];
    if filled {
        shapes[0].extend([(K::LongWire, 13), (K::FunctionBlock, 16)]);
        shapes[1].extend([
            (K::FunctionOperand, 13),
            (K::LinkReference(0x68), 16),
            (K::FunctionOperand, 19),
        ]);
        shapes[2].push((K::LinkReference(0x69), 16));
    }
    for (i, frame) in group.iter().enumerate() {
        let rs = records
            .iter()
            .filter(|r| r.group_index == top.group_index && r.row_index == frame.row_index)
            .collect::<Vec<_>>();
        let mut actual = rs
            .iter()
            .map(|r| (r.kind, program.data[r.offset + 5]))
            .collect::<Vec<_>>();
        // Original files use a short wire for the upper feed; native refill uses long.
        if filled && actual.get(6) == Some(&(K::ShortWire, 13)) {
            actual[6].0 = K::LongWire;
        }
        if actual != shapes[i] {
            return None;
        }
        for (position, r) in rs.into_iter().enumerate() {
            let bytes = &program.data[r.offset..r.end];
            match r.kind {
                K::BranchStart => {
                    let boundary = match (i, position) {
                        (0, 1) | (1, 2) => 3,
                        (0, 4) | (1, 4) | (2, 3) => 9,
                        _ => return None,
                    };
                    if bytes != branch_start(frame.row_index, boundary, 0) {
                        return None;
                    }
                }
                K::BranchEnd => {
                    if bytes != branch_end(frame.row_index, bytes[5]) {
                        return None;
                    }
                }
                K::ShortWire => {
                    if bytes[9..15] != [0; 6] {
                        return None;
                    }
                }
                K::LongWire => {
                    let mut feed = wire(frame.row_index, 13, 13);
                    feed[11] = 4;
                    if bytes != feed {
                        return None;
                    }
                }
                K::Contact(6) => {
                    if bytes[9..15] != [1, 0, 0, 0, 0, 0] {
                        return None;
                    }
                }
                K::FunctionBlock => {
                    let b = blocks.iter().find(|b| b.record_offset == r.offset)?;
                    let flags = if i == 0 { bytes[11] } else { 0 };
                    if ![0, 4].contains(&flags)
                        || bytes != body_with_flags("MOVE", frame.row_index, 16, flags)
                        || b.name.value != "MOVE"
                        || links
                            .iter()
                            .filter(|l| l.target_record_offset == r.offset)
                            .count()
                            != 2
                        || refs
                            .iter()
                            .filter(|l| l.target_record_offset == r.offset)
                            .count()
                            != 2
                    {
                        return None;
                    }
                }
                K::FunctionOperand => {
                    let target_row = if i <= 2 { row } else { row + 3 };
                    let link = links.iter().find(|l| l.record_offset == r.offset)?;
                    let b = blocks
                        .iter()
                        .find(|b| b.record_offset == link.target_record_offset)?;
                    if b.row_index != target_row || b.raw_x != 16 {
                        return None;
                    }
                }
                K::LinkReference(_) => {
                    let target_row = if i <= 2 { row } else { row + 3 };
                    let reference = refs.iter().find(|l| l.record_offset == r.offset)?;
                    let b = blocks
                        .iter()
                        .find(|b| b.record_offset == reference.target_record_offset)?;
                    if b.row_index != target_row || b.raw_x != 16 {
                        return None;
                    }
                }
                _ => return None,
            }
        }
    }
    Some(top.group_index)
}

pub(crate) fn scaffold(program: &LadderProgramData, row: u16, x: u8) -> bool {
    x == 16 && layout(program, row, false).is_some()
}

pub(crate) fn remove(program: &LadderProgramData, offset: usize) -> Result<Vec<u8>, XgwxError> {
    let unsupported = || XgwxError::UnsupportedLadderLayout;
    let blocks = program.iec_function_blocks().ok_or_else(unsupported)?;
    let upper = blocks
        .iter()
        .find(|b| b.record_offset == offset)
        .ok_or_else(unsupported)?;
    let row = upper.row_index;
    let group = layout(program, row, true).ok_or_else(unsupported)?;
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let links = program
        .iec_function_operand_links()
        .ok_or_else(unsupported)?;
    let refs = program.iec_function_references().ok_or_else(unsupported)?;
    let mut removed = vec![offset];
    removed.extend(
        records
            .iter()
            .filter(|r| {
                r.group_index == group && r.row_index == row && program.data[r.offset + 5] == 13
            })
            .map(|r| r.offset),
    );
    removed.extend(
        links
            .iter()
            .filter(|l| l.target_record_offset == offset)
            .map(|l| l.record_offset),
    );
    removed.extend(
        refs.iter()
            .filter(|l| l.target_record_offset == offset)
            .map(|l| l.record_offset),
    );
    rewrite(program, group, &removed, &[])
}

pub(crate) fn insert(
    program: &LadderProgramData,
    row: u16,
    x: u8,
    name: &str,
    operands: &[String],
) -> Result<Vec<u8>, XgwxError> {
    if x != 16 || name != "MOVE" || operands.len() != 2 {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let group = layout(program, row, false).ok_or(XgwxError::UnsupportedLadderLayout)?;
    let mut feed = wire(row, 13, 13);
    feed[11] = 4;
    let mut input = expression(row + 1, 13, &operands[0]);
    input[11] = 4;
    let mut output = expression(row + 1, 19, &operands[1]);
    output[11] = 4;
    let y = (row * 4).to_le_bytes();
    let added = vec![
        (row, vec![feed, body_with_flags("MOVE", row, 16, 4)]),
        (
            row + 1,
            vec![input, vec![1, 0x68, 0, 0, 0, 16, y[0], y[1], 0], output],
        ),
        (row + 2, vec![vec![2, 0x69, 0, 0, 0, 16, y[0], y[1], 0]]),
    ];
    rewrite(program, group, &[], &added)
}

fn rewrite(
    program: &LadderProgramData,
    group: usize,
    removed: &[usize],
    added: &[(u16, Vec<Vec<u8>>)],
) -> Result<Vec<u8>, XgwxError> {
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let group_rows = rows
        .iter()
        .filter(|r| r.group_index == group)
        .collect::<Vec<_>>();
    let mut out = program.data[..group_rows[0].start].to_vec();
    for frame in &group_rows {
        let mut kept = records
            .iter()
            .filter(|r| {
                r.group_index == group
                    && r.row_index == frame.row_index
                    && !removed.contains(&r.offset)
            })
            .map(|r| program.data[r.offset..r.end].to_vec())
            .collect::<Vec<_>>();
        if let Some((_, rs)) = added.iter().find(|(row, _)| *row == frame.row_index) {
            kept.extend(rs.iter().cloned());
        }
        let mut header = program.data[frame.start..frame.records_start].to_vec();
        header[17..21].copy_from_slice(&39u32.to_le_bytes());
        header[29] = kept
            .iter()
            .map(|r| {
                if r[1] == 0 && r.len() == 27 {
                    r[7]
                } else {
                    r[5]
                }
            })
            .max()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        header[33..35].copy_from_slice(&(kept.len() as u16).to_le_bytes());
        out.extend(header);
        for record in kept {
            out.extend(record);
        }
    }
    out.extend_from_slice(&program.data[group_rows.last().unwrap().end..]);
    let mut result = program.clone();
    result.decoded_len = out.len();
    result.data = out;
    if result.iec_circuit_graph().is_none()
        && !crate::writer::iec_preserves_groups_around_edit(program, &result, group..group + 1, 1)
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(result.data)
}
