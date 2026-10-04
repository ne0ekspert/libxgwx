//! Native conversion pair and TON sharing a retained input branch.
use crate::iec_function_write::{body, expression, row_header, wire};
use crate::{IecRecordKind as K, LadderProgramData, XgwxError};

type Row = (Vec<u8>, Vec<Vec<u8>>);
struct Pair {
    row: u16,
    group: usize,
    rows: Vec<Row>,
    first: bool,
    second: bool,
    timer: bool,
}

fn reference(row: u16, x: u8, ordinal: u8) -> Vec<u8> {
    let y = (row * 4).to_le_bytes();
    vec![
        ordinal,
        if ordinal == 1 { 0x68 } else { 0x69 },
        0,
        0,
        0,
        x,
        y[0],
        y[1],
        0,
    ]
}

// The two native binding encodings are retained without interpreting their meaning.
fn timer_body(row: u16, instance: &str, flags: u8) -> Vec<u8> {
    let mut canonical = crate::iec_connected_timer_write::body_at(row, 10, instance, true);
    canonical[32] = flags;
    let len = canonical.len();
    canonical[len - 60] = flags;
    canonical[len - 6] = flags;
    canonical
}

fn parse(program: &LadderProgramData, row: u16) -> Option<Pair> {
    if row > u16::MAX / 4 - 5 {
        return None;
    }
    let frames = program.iec_row_frames()?;
    let records = program.iec_record_frames()?;
    let blocks = program.iec_function_blocks()?;
    let top = frames.iter().find(|r| r.row_index == row)?;
    let owned = frames
        .iter()
        .filter(|r| r.group_index == top.group_index)
        .collect::<Vec<_>>();
    if ![4, 6].contains(&owned.len())
        || owned
            .iter()
            .enumerate()
            .any(|(i, r)| r.row_index != row + i as u16)
        || program.data.get(top.start - 6..top.start - 2)? != [0; 4]
    {
        return None;
    }
    let first = blocks.iter().find(|b| b.row_index == row && b.raw_x == 10);
    let second = blocks.iter().find(|b| b.row_index == row && b.raw_x == 19);
    for (b, name, x, flags) in [
        (
            first,
            "INT_TO_UDINT",
            10,
            if second.is_some() { 0 } else { 2 },
        ),
        (second, "UDINT_TO_TIME", 19, 2),
    ] {
        if let Some(b) = b {
            let mut canonical = body(name, row, x);
            canonical[12] = flags;
            if b.instance.is_some()
                || program.data.get(b.record_offset..b.record_end)? != canonical
                || program
                    .iec_function_operand_links()?
                    .iter()
                    .filter(|l| l.target_record_offset == b.record_offset)
                    .count()
                    != 2
                || program
                    .iec_function_references()?
                    .iter()
                    .filter(|l| l.target_record_offset == b.record_offset)
                    .count()
                    != 2
            {
                return None;
            }
        }
    }
    let timer = blocks
        .iter()
        .find(|b| b.group_index == top.group_index && b.row_index == row + 3 && b.raw_x == 10);
    if owned.len() != if timer.is_some() { 6 } else { 4 }
        || blocks
            .iter()
            .filter(|b| b.group_index == top.group_index)
            .count()
            != usize::from(first.is_some())
                + usize::from(second.is_some())
                + usize::from(timer.is_some())
        || (timer.is_none()
            && frames
                .iter()
                .any(|r| [row + 4, row + 5].contains(&r.row_index)))
    {
        return None;
    }
    if let Some(timer) = timer {
        if timer.name.value != "TON" {
            return None;
        }
        let flags = *program.data.get(timer.record_offset + 32)?;
        if ![0, 2].contains(&flags) {
            return None;
        }
        let canonical = timer_body(row + 3, &timer.instance.as_ref()?.value, flags);
        if program.data.get(timer.record_offset..timer.record_end)? != canonical
            || program
                .iec_function_operand_links()?
                .iter()
                .filter(|l| l.target_record_offset == timer.record_offset)
                .count()
                != 1
            || program
                .iec_function_references()?
                .iter()
                .filter(|l| l.target_record_offset == timer.record_offset)
                .count()
                != 2
        {
            return None;
        }
    }
    let mut rows = Vec::new();
    for (i, f) in owned.iter().enumerate() {
        let rr = records
            .iter()
            .filter(|r| r.group_index == top.group_index && r.row_index == f.row_index)
            .collect::<Vec<_>>();
        let mut shape = match i {
            0 => vec![(K::Contact(6), 1), (K::Contact(6), 4), (K::BranchStart, 2)],
            1 | 2 => vec![(K::BranchEnd, 6), (K::BranchStart, 2)],
            3 if timer.is_some() => {
                vec![(K::BranchEnd, 6), (K::LongWire, 7), (K::FunctionBlock, 10)]
            }
            3 => vec![(K::BranchEnd, 6)],
            4 => vec![(K::FunctionOperand, 7), (K::LinkReference(0x68), 10)],
            _ => vec![(K::LinkReference(0x69), 10)],
        };
        if i == 0 {
            if first.is_some() || second.is_some() {
                shape.push((K::ShortWire, 7));
            }
            if first.is_some() {
                shape.push((K::FunctionBlock, 10));
            }
            if second.is_some() {
                shape.extend([(K::LongWire, 13), (K::FunctionBlock, 19)]);
            }
        }
        for (present, x) in [(first.is_some(), 10), (second.is_some(), 19)] {
            if present && i == 1 {
                shape.extend([
                    (K::FunctionOperand, x - 3),
                    (K::LinkReference(0x68), x),
                    (K::FunctionOperand, x + 3),
                ]);
            }
            if present && i == 2 {
                shape.push((K::LinkReference(0x69), x));
            }
        }
        if rr.len() != shape.len()
            || rr
                .iter()
                .zip(shape)
                .any(|(r, (k, x))| r.kind != k || program.data[r.offset + 5] != x)
        {
            return None;
        }
        for r in &rr {
            let bytes = &program.data[r.offset..r.end];
            let y = (f.row_index * 4).to_le_bytes();
            match r.kind {
                K::ShortWire if bytes != [0xff, 1, 0, 0, 0, 7, y[0], y[1], 0, 0, 0, 0, 0, 0, 0] => {
                    return None;
                }
                K::LongWire
                    if bytes != wire(f.row_index, bytes[5], if i == 0 { 16 } else { 7 }) =>
                {
                    return None;
                }
                K::FunctionOperand if bytes[9..15] != [0; 6] => return None,
                K::LinkReference(_)
                    if bytes
                        != reference(
                            if i < 3 { row } else { row + 3 },
                            bytes[5],
                            if i == 1 || i == 4 { 1 } else { 2 },
                        ) =>
                {
                    return None;
                }
                K::BranchEnd
                    if bytes
                        != [
                            1,
                            0,
                            0,
                            0,
                            0,
                            6,
                            ((f.row_index - 1) * 4) as u8,
                            (((f.row_index - 1) * 4) >> 8) as u8,
                            0,
                        ] =>
                {
                    return None;
                }
                K::BranchStart => {
                    let next = ((f.row_index + 1) * 4).to_le_bytes();
                    if bytes
                        != [
                            0, 0, 0, 0, 0, 2, 0, 6, y[0], y[1], 0, 0, 0, 0, 0, 0, 0, 5, next[0],
                            next[1], 0, 0, 0, 0, 0, 0, 0,
                        ]
                    {
                        return None;
                    }
                }
                _ => {}
            }
        }
        let mut h = program.data[f.start..f.records_start].to_vec();
        h[17..21].copy_from_slice(&39u32.to_le_bytes());
        h[29] = 1;
        h[33..35].fill(0);
        if h != row_header(f.row_index) {
            return None;
        }
        rows.push((
            program.data[f.start..f.records_start].to_vec(),
            rr.iter()
                .map(|r| program.data[r.offset..r.end].to_vec())
                .collect(),
        ));
    }
    let vertical = program
        .iec_geometry()?
        .vertical
        .into_iter()
        .filter(|v| v.group_index == top.group_index)
        .collect::<Vec<_>>();
    if vertical.len() != 3
        || (0..3).any(|i| {
            !vertical
                .iter()
                .any(|v| v.start_row_index == row + i && v.end_row_index == row + i + 1 && v.x == 6)
        })
    {
        return None;
    }
    let graph = program.iec_circuit_layout()?;
    let expected_open = if timer.is_some() {
        vec![]
    } else {
        vec![crate::IecCircuitPoint {
            group_index: top.group_index,
            row_index: row + 3,
            x: 6,
        }]
    };
    if graph.open_branch_endpoints != expected_open {
        return None;
    }
    Some(Pair {
        row,
        group: top.group_index,
        rows,
        first: first.is_some(),
        second: second.is_some(),
        timer: timer.is_some(),
    })
}

pub(crate) fn scaffold(program: &LadderProgramData, row: u16, x: u8) -> Option<usize> {
    let pair = parse(program, row)?;
    if (x == 10 && !pair.first) || (x == 19 && !pair.second) {
        Some(pair.group)
    } else {
        None
    }
}

fn refresh(pair: &mut Pair, conversions: bool) {
    for (i, (h, rr)) in pair.rows.iter_mut().enumerate() {
        if !conversions && i < 3 {
            // Native editing refreshes the captured 78-pixel top-row cache to 50.
            // Other conversion row caches survive the timer-only operation.
            if i == 0 && h[17..21] == 78u32.to_le_bytes() {
                h[17..21].copy_from_slice(&50u32.to_le_bytes());
            }
            continue;
        }
        let height = match i {
            0 => {
                if pair.first || pair.second {
                    50
                } else {
                    39
                }
            }
            1 => {
                if pair.second {
                    40
                } else {
                    39
                }
            }
            2 => 39,
            3 => {
                if pair.timer {
                    50
                } else {
                    39
                }
            }
            5 if conversions && !pair.first && !pair.second => 39,
            _ => u32::from_le_bytes(h[17..21].try_into().unwrap()),
        };
        h[17..21].copy_from_slice(&height.to_le_bytes());
        h[29] = match i {
            0 | 2 => {
                if pair.second {
                    19
                } else if pair.first {
                    10
                } else {
                    6
                }
            }
            1 => {
                if pair.second {
                    22
                } else if pair.first {
                    13
                } else {
                    6
                }
            }
            3 if !pair.timer => 5,
            _ => 10,
        };
        h[33..35].copy_from_slice(&(rr.len() as u16).to_le_bytes());
        if i == 0 {
            if let Some(b) = rr.iter_mut().find(|b| b[1] == 0x67 && b[5] == 10) {
                b[12] = if pair.second { 0 } else { 2 };
            }
        }
    }
}

fn serialize(
    program: &LadderProgramData,
    mut pair: Pair,
    conversions: bool,
) -> Result<Vec<u8>, XgwxError> {
    refresh(&mut pair, conversions);
    let frames = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let top = frames.iter().find(|r| r.group_index == pair.group).unwrap();
    let last = frames
        .iter()
        .filter(|r| r.group_index == pair.group)
        .last()
        .unwrap();
    let mut data = program.data[..top.start].to_vec();
    data[top.start - 2..top.start].copy_from_slice(&(pair.rows.len() as u16).to_le_bytes());
    for (h, rr) in pair.rows {
        data.extend(h);
        for r in rr {
            data.extend(r);
        }
    }
    data.extend(&program.data[last.end..]);
    let mut verified = program.clone();
    verified.decoded_len = data.len();
    verified.data = data;
    if parse(&verified, pair.row).is_none() {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(verified.data)
}

pub(crate) fn remove(program: &LadderProgramData, offset: usize) -> Result<Vec<u8>, XgwxError> {
    let fail = || XgwxError::UnsupportedLadderLayout;
    let block = program
        .iec_function_blocks()
        .ok_or_else(fail)?
        .into_iter()
        .find(|b| b.record_offset == offset)
        .ok_or_else(fail)?;
    if ![10, 19].contains(&block.raw_x) {
        return Err(fail());
    }
    let mut pair = parse(program, block.row_index).ok_or_else(fail)?;
    let x = block.raw_x;
    pair.rows[0]
        .1
        .retain(|r| !((r[1] == 0x67 && r[5] == x) || (x == 19 && r[1] == 2 && r[5] == 13)));
    pair.rows[1].1.retain(|r| {
        !((r[1] == 0x46 && [x - 3, x + 3].contains(&r[5])) || (r[1] == 0x68 && r[5] == x))
    });
    pair.rows[2].1.retain(|r| !(r[1] == 0x69 && r[5] == x));
    if x == 10 {
        pair.first = false;
    } else {
        pair.second = false;
    }
    if !pair.first && !pair.second {
        pair.rows[0].1.retain(|r| r[1] != 1);
    }
    serialize(program, pair, true)
}

pub(crate) fn insert(
    program: &LadderProgramData,
    row: u16,
    x: u8,
    name: &str,
    operands: &[String],
) -> Result<Vec<u8>, XgwxError> {
    let fail = || XgwxError::UnsupportedLadderLayout;
    if operands.len() != 2
        || !matches!((x, name), (10, "INT_TO_UDINT") | (19, "UDINT_TO_TIME"))
        || scaffold(program, row, x).is_none()
    {
        return Err(fail());
    }
    let mut pair = parse(program, row).ok_or_else(fail)?;
    let y = (row * 4).to_le_bytes();
    if !pair.first && !pair.second {
        pair.rows[0]
            .1
            .push(vec![0xff, 1, 0, 0, 0, 7, y[0], y[1], 0, 0, 0, 0, 0, 0, 0]);
    }
    let index = if x == 10 { 4 } else { pair.rows[0].1.len() };
    let mut added = Vec::new();
    if x == 19 {
        added.push(wire(row, 13, 16));
    }
    added.push(body(name, row, x));
    pair.rows[0].1.splice(index..index, added);
    let index = if x == 10 { 2 } else { pair.rows[1].1.len() };
    pair.rows[1].1.splice(
        index..index,
        [
            expression(row + 1, x - 3, &operands[0]),
            reference(row, x, 1),
            expression(row + 1, x + 3, &operands[1]),
        ],
    );
    let index = if x == 10 { 2 } else { pair.rows[2].1.len() };
    pair.rows[2].1.insert(index, reference(row, x, 2));
    if x == 10 {
        pair.first = true;
    } else {
        pair.second = true;
    }
    serialize(program, pair, true)
}

/// Remove the TON while retaining all three segments of its shared input branch.
pub(crate) fn remove_timer(
    program: &LadderProgramData,
    offset: usize,
) -> Result<Vec<u8>, XgwxError> {
    let fail = || XgwxError::UnsupportedLadderLayout;
    let block = program
        .iec_function_blocks()
        .ok_or_else(fail)?
        .into_iter()
        .find(|b| b.record_offset == offset && b.raw_x == 10 && b.name.value == "TON")
        .ok_or_else(fail)?;
    let mut pair =
        parse(program, block.row_index.checked_sub(3).ok_or_else(fail)?).ok_or_else(fail)?;
    pair.rows[3].1.truncate(1);
    pair.rows.truncate(4);
    pair.timer = false;
    serialize(program, pair, false)
}

pub(crate) fn timer_scaffold(
    program: &LadderProgramData,
    contact_offset: usize,
) -> Option<crate::IecTerminalFunctionInsertionSite> {
    let records = program.iec_record_frames()?;
    let contact = records.iter().find(|r| r.offset == contact_offset)?;
    if contact.kind != K::Contact(6) || program.data[contact.offset + 5] != 1 {
        return None;
    }
    let pair = parse(program, contact.row_index)?;
    if pair.timer {
        return None;
    }
    let last = program
        .iec_row_frames()?
        .into_iter()
        .find(|r| r.group_index == pair.group && r.row_index == pair.row + 3)?;
    Some(crate::IecTerminalFunctionInsertionSite {
        group_index: pair.group,
        row_index: pair.row + 3,
        contact_offset,
        insertion_offset: last.end,
        raw_x: 10,
    })
}

pub(crate) fn insert_timer(
    program: &LadderProgramData,
    contact_offset: usize,
    instance: &str,
    preset: &str,
    literal: bool,
) -> Result<Vec<u8>, XgwxError> {
    let fail = || XgwxError::UnsupportedLadderLayout;
    let site = timer_scaffold(program, contact_offset).ok_or_else(fail)?;
    let mut pair = parse(program, site.row_index - 3).ok_or_else(fail)?;
    let row = site.row_index;
    pair.rows[3]
        .1
        .extend([wire(row, 7, 7), timer_body(row, instance, 0)]);
    let mut pin_header = row_header(row + 1);
    pin_header[17..21].copy_from_slice(&(if literal { 39u32 } else { 40 }).to_le_bytes());
    pair.rows.push((
        pin_header,
        vec![expression(row + 1, 7, preset), reference(row, 10, 1)],
    ));
    pair.rows
        .push((row_header(row + 2), vec![reference(row, 10, 2)]));
    pair.timer = true;
    serialize(program, pair, false)
}

pub(crate) fn owns_function(program: &LadderProgramData, offset: usize) -> bool {
    let Some(block) = program
        .iec_function_blocks()
        .and_then(|v| v.into_iter().find(|b| b.record_offset == offset))
    else {
        return false;
    };
    let base = match (block.name.value.as_str(), block.raw_x) {
        ("TON", 10) => block.row_index.checked_sub(3),
        ("INT_TO_UDINT", 10) | ("UDINT_TO_TIME", 19) => Some(block.row_index),
        _ => None,
    };
    base.is_some_and(|row| parse(program, row).is_some())
}
