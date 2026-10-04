//! Native MOVE at the end of a six-row contact mesh.
use crate::iec_function_write::{body, branch_end, branch_start, expression, wire};
use crate::{IecRecordKind as K, LadderProgramData, XgwxError};

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
        || program.data[top.start - 6..top.start - 2] != [0; 4]
    {
        return None;
    }
    let records = program.iec_record_frames()?;
    let blocks = program.iec_function_blocks()?;
    let links = program.iec_function_operand_links()?;
    let refs = program.iec_function_references()?;
    if blocks
        .iter()
        .filter(|b| b.group_index == top.group_index)
        .count()
        != usize::from(filled)
    {
        return None;
    }
    let mut shapes = vec![
        vec![
            (K::Contact(6), 1),
            (K::BranchStart, 3),
            (K::ShortWire, 4),
            (K::BranchStart, 6),
            (K::Contact(7), 7),
            (K::BranchStart, 9),
            (K::ShortWire, 10),
            (K::BranchStart, 12),
            (K::Contact(6), 13),
            (K::BranchStart, 15),
        ],
        vec![
            (K::Contact(6), 1),
            (K::BranchEnd, 3),
            (K::BranchStart, 3),
            (K::BranchEnd, 6),
            (K::Contact(7), 7),
            (K::BranchEnd, 9),
            (K::BranchEnd, 12),
            (K::Contact(6), 13),
            (K::BranchEnd, 15),
        ],
        vec![(K::Contact(6), 1), (K::BranchEnd, 3), (K::BranchStart, 3)],
        vec![(K::Contact(6), 1), (K::BranchEnd, 3), (K::BranchStart, 3)],
        vec![(K::Contact(6), 1), (K::BranchEnd, 3), (K::BranchStart, 3)],
        vec![(K::Contact(6), 1), (K::BranchEnd, 3)],
    ];
    if filled {
        shapes[0].extend([(K::LongWire, 16), (K::FunctionBlock, 19)]);
        shapes[1].extend([
            (K::FunctionOperand, 16),
            (K::LinkReference(0x68), 19),
            (K::FunctionOperand, 22),
        ]);
        shapes[2].push((K::LinkReference(0x69), 19));
    }
    let leading = records.iter().find(|r| r.offset == top.records_start)?;
    let has_leading_contact = matches!(leading.kind, K::Contact(6..=11));
    if !has_leading_contact {
        shapes[0].remove(0);
    }
    for (i, frame) in group.iter().enumerate() {
        let rs = records
            .iter()
            .filter(|r| r.group_index == top.group_index && r.row_index == frame.row_index)
            .collect::<Vec<_>>();
        let actual = rs
            .iter()
            .map(|r| {
                let mut kind = r.kind;
                if i == 0 && r.offset == top.records_start && has_leading_contact {
                    kind = K::Contact(6);
                }
                if filled && i == 0 && program.data[r.offset + 5] == 16 && kind == K::ShortWire {
                    kind = K::LongWire;
                }
                (
                    kind,
                    program.data[r.offset + if r.kind == K::BranchStart { 7 } else { 5 }],
                )
            })
            .collect::<Vec<_>>();
        if actual != shapes[i] {
            return None;
        }
        for r in rs {
            let bytes = &program.data[r.offset..r.end];
            match r.kind {
                K::BranchStart => {
                    if bytes != branch_start(frame.row_index, bytes[7], 0) {
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
                    if bytes != wire(row, 16, 16) {
                        return None;
                    }
                }
                K::Contact(6..=11) => {
                    if bytes[9..15] != [1, 0, 0, 0, 0, 0] {
                        return None;
                    }
                }
                K::FunctionBlock => {
                    if bytes != body("MOVE", row, 19)
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
                    let link = links.iter().find(|l| l.record_offset == r.offset)?;
                    let b = blocks
                        .iter()
                        .find(|b| b.record_offset == link.target_record_offset)?;
                    if b.row_index != row || b.raw_x != 19 || bytes[9..15] != [0; 6] {
                        return None;
                    }
                }
                K::LinkReference(_) => {
                    let reference = refs.iter().find(|l| l.record_offset == r.offset)?;
                    let b = blocks
                        .iter()
                        .find(|b| b.record_offset == reference.target_record_offset)?;
                    if b.row_index != row || b.raw_x != 19 {
                        return None;
                    }
                    let y = (row * 4).to_le_bytes();
                    if bytes
                        != [
                            if i == 1 { 1 } else { 2 },
                            if i == 1 { 0x68 } else { 0x69 },
                            0,
                            0,
                            0,
                            19,
                            y[0],
                            y[1],
                            0,
                        ]
                    {
                        return None;
                    }
                }
                _ => return None,
            }
        }
    }
    Some(top.group_index)
}

pub(crate) fn leading_contact_sites(
    program: &LadderProgramData,
    present: bool,
) -> Option<Vec<(usize, u16, usize, u8)>> {
    let rows = program.iec_row_frames()?;
    let records = program.iec_record_frames()?;
    Some(
        rows.iter()
            .filter_map(|row| {
                let group = layout(program, row.row_index, true)
                    .or_else(|| layout(program, row.row_index, false))?;
                let first = records.iter().find(|r| r.offset == row.records_start)?;
                let code = match first.kind {
                    K::Contact(code @ 6..=11) if present => code,
                    K::BranchStart if !present => 6,
                    _ => return None,
                };
                Some((group, row.row_index, first.offset, code))
            })
            .collect(),
    )
}

pub(crate) fn edit_leading_contact(
    program: &LadderProgramData,
    row: u16,
    added: Option<(u8, &str)>,
) -> Result<Vec<u8>, XgwxError> {
    let unsupported = || XgwxError::UnsupportedLadderLayout;
    let group = layout(program, row, true)
        .or_else(|| layout(program, row, false))
        .ok_or_else(unsupported)?;
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let top = rows
        .iter()
        .find(|r| r.row_index == row)
        .ok_or_else(unsupported)?;
    let first = records
        .iter()
        .find(|r| r.offset == top.records_start)
        .ok_or_else(unsupported)?;
    if matches!(first.kind, K::Contact(_)) != added.is_none() {
        return Err(unsupported());
    }
    let frames = rows
        .iter()
        .filter(|r| r.group_index == group)
        .collect::<Vec<_>>();
    let mut out = program.data[..top.start].to_vec();
    for frame in &frames {
        let mut kept = records
            .iter()
            .filter(|r| {
                r.group_index == group
                    && r.row_index == frame.row_index
                    && !(added.is_none() && r.offset == first.offset)
            })
            .map(|r| program.data[r.offset..r.end].to_vec())
            .collect::<Vec<_>>();
        if frame.row_index == row
            && let Some((code, operand)) = added
        {
            if !(6..=11).contains(&code) {
                return Err(unsupported());
            }
            let mut contact = expression(row, 1, operand);
            contact[1] = code;
            contact[9] = 1;
            kept.insert(0, contact);
        }
        let mut header = program.data[frame.start..frame.records_start].to_vec();
        header[17..21].copy_from_slice(&39u32.to_le_bytes());
        header[33..35].copy_from_slice(&(kept.len() as u16).to_le_bytes());
        out.extend(header);
        for record in kept {
            out.extend(record);
        }
    }
    out.extend_from_slice(&program.data[frames.last().unwrap().end..]);
    let mut verified = program.clone();
    verified.decoded_len = out.len();
    verified.data = out;
    if verified.iec_circuit_graph().is_none()
        || !(layout(&verified, row, true).is_some() || layout(&verified, row, false).is_some())
    {
        return Err(unsupported());
    }
    Ok(verified.data)
}

pub(crate) fn scaffold(program: &LadderProgramData, row: u16, x: u8) -> bool {
    x == 19 && layout(program, row, false).is_some()
}

pub(crate) fn remove(program: &LadderProgramData, offset: usize) -> Result<Vec<u8>, XgwxError> {
    let unsupported = || XgwxError::UnsupportedLadderLayout;
    let blocks = program.iec_function_blocks().ok_or_else(unsupported)?;
    let b = blocks
        .iter()
        .find(|b| b.record_offset == offset)
        .ok_or_else(unsupported)?;
    if b.raw_x != 19 || b.name.value != "MOVE" {
        return Err(unsupported());
    }
    let group = layout(program, b.row_index, true).ok_or_else(unsupported)?;
    rewrite(program, group, b.row_index, false, &[])
}

pub(crate) fn insert(
    program: &LadderProgramData,
    row: u16,
    x: u8,
    name: &str,
    operands: &[String],
) -> Result<Vec<u8>, XgwxError> {
    if x != 19 || name != "MOVE" || operands.len() != 2 {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let group = layout(program, row, false).ok_or(XgwxError::UnsupportedLadderLayout)?;
    rewrite(program, group, row, true, operands)
}

fn rewrite(
    program: &LadderProgramData,
    group: usize,
    row: u16,
    filled: bool,
    operands: &[String],
) -> Result<Vec<u8>, XgwxError> {
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let frames = rows
        .iter()
        .filter(|r| r.group_index == group)
        .collect::<Vec<_>>();
    let mut out = program.data[..frames[0].start].to_vec();
    let y = (row * 4).to_le_bytes();
    for frame in &frames {
        let i = usize::from(frame.row_index - row);
        let mut kept = records
            .iter()
            .filter(|r| {
                r.group_index == group
                    && r.row_index == frame.row_index
                    && !matches!(
                        r.kind,
                        K::FunctionBlock | K::FunctionOperand | K::LinkReference(_)
                    )
                    && !(i == 0
                        && program.data[r.offset + 5] == 16
                        && matches!(r.kind, K::ShortWire | K::LongWire))
            })
            .map(|r| program.data[r.offset..r.end].to_vec())
            .collect::<Vec<_>>();
        if filled {
            match i {
                0 => kept.extend([wire(row, 16, 16), body("MOVE", row, 19)]),
                1 => kept.extend([
                    expression(row + 1, 16, &operands[0]),
                    vec![1, 0x68, 0, 0, 0, 19, y[0], y[1], 0],
                    expression(row + 1, 22, &operands[1]),
                ]),
                2 => kept.push(vec![2, 0x69, 0, 0, 0, 19, y[0], y[1], 0]),
                _ => {}
            }
        }
        let mut header = program.data[frame.start..frame.records_start].to_vec();
        header[17..21].copy_from_slice(&39u32.to_le_bytes());
        header[29] = if filled {
            [19, 22, 19, 3, 3, 2][i]
        } else {
            [15, 14, 3, 3, 3, 2][i]
        };
        header[33..35].copy_from_slice(&(kept.len() as u16).to_le_bytes());
        out.extend(header);
        for record in kept {
            out.extend(record);
        }
    }
    out.extend_from_slice(&program.data[frames.last().unwrap().end..]);
    let mut result = program.clone();
    result.decoded_len = out.len();
    result.data = out;
    if result.iec_circuit_graph().is_none() || layout(&result, row, filled).is_none() {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(result.data)
}
