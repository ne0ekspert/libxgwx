//! Native staggered comparison pairs feeding a three- or four-segment branch.
use crate::iec_function_write::{body, expression, row_header, spec, wire};
use crate::{IecRecordKind as K, LadderProgramData, XgwxError};
use std::collections::BTreeMap;

type Row = (Vec<u8>, Vec<Vec<u8>>);
struct Pair {
    row: u16,
    x: u8,
    first_group: usize,
    last_group: usize,
    rows: BTreeMap<u16, Row>,
    head: bool,
    tail: bool,
}

fn parse(program: &LadderProgramData, row: u16, x: u8) -> Option<Pair> {
    if ![4, 7].contains(&x) || row > u16::MAX / 4 - 6 {
        return None;
    }
    let frames = program.iec_row_frames()?;
    let records = program.iec_record_frames()?;
    let blocks = program.iec_function_blocks()?;
    let first = frames.iter().find(|r| r.row_index == row)?;
    let head = blocks.iter().find(|b| b.row_index == row && b.raw_x == x);
    let last_group = first.group_index + usize::from(head.is_none());
    let last = frames
        .iter()
        .filter(|r| r.group_index == last_group)
        .last()?;
    let depth = last.row_index.checked_sub(row)?;
    if ![5, 6].contains(&depth) {
        return None;
    }
    let tail = blocks
        .iter()
        .find(|b| b.row_index == row + 1 && b.raw_x == x + 9);
    if last.group_index != first.group_index + usize::from(head.is_none()) {
        return None;
    }
    let owned = frames
        .iter()
        .filter(|r| (first.group_index..=last.group_index).contains(&r.group_index))
        .collect::<Vec<_>>();
    let expected_rows = (row..=row + depth)
        .filter(|r| *r != row + 1 || head.is_some() || tail.is_some())
        .collect::<Vec<_>>();
    if owned.iter().map(|r| r.row_index).collect::<Vec<_>>() != expected_rows {
        return None;
    }
    for f in [first, last] {
        if program.data.get(f.start - 6..f.start - 2)? != [0; 4] {
            return None;
        }
    }
    for (b, expected_row, expected_x, flags) in [
        (head, row, x, if tail.is_some() { 0 } else { 2 }),
        (tail, row + 1, x + 9, 0),
    ] {
        if let Some(b) = b {
            if spec(&b.name.value).is_none_or(|(family, opcode, pins)| {
                family != 0x28
                    || b.opcode_family != family
                    || b.opcode != opcode
                    || b.pin_count != pins
            }) || b.instance.is_some()
            {
                return None;
            }
            let mut canonical = body(&b.name.value, expected_row, expected_x);
            canonical[12] = flags;
            let record = records.iter().find(|r| r.offset == b.record_offset)?;
            if program.data.get(record.offset..record.end)? != canonical {
                return None;
            }
            if program
                .iec_function_references()?
                .iter()
                .filter(|r| r.target_record_offset == b.record_offset)
                .count()
                != 3
                || program
                    .iec_function_operand_links()?
                    .iter()
                    .filter(|r| r.target_record_offset == b.record_offset && !r.is_output)
                    .count()
                    != 2
            {
                return None;
            }
        }
    }
    if blocks
        .iter()
        .filter(|b| (first.group_index..=last.group_index).contains(&b.group_index))
        .count()
        != usize::from(head.is_some()) + usize::from(tail.is_some())
    {
        return None;
    }
    let mut rows = BTreeMap::new();
    for f in &owned {
        let rr = records
            .iter()
            .filter(|r| r.group_index == f.group_index && r.row_index == f.row_index)
            .collect::<Vec<_>>();
        let i = f.row_index - row;
        let mut shape = Vec::new();
        if i == 0 {
            let contact = rr.first()?;
            if !matches!(contact.kind, K::Contact(6) | K::Contact(7)) {
                return None;
            }
            shape.push((contact.kind, 1));
            if head.is_some() {
                if x == 7 {
                    shape.push((K::LongWire, 4));
                }
                shape.push((K::FunctionBlock, x));
            }
        }
        if head.is_some() && (1..=3).contains(&i) {
            if i < 3 {
                shape.push((K::FunctionOperand, x - 3));
            }
            shape.push((K::LinkReference(if i == 3 { 0x69 } else { 0x68 }), x));
        }
        if tail.is_some() && i == 1 {
            shape.extend([(K::LongWire, x + 3), (K::FunctionBlock, x + 9)]);
        }
        if tail.is_some() && (2..=4).contains(&i) {
            if i < 4 {
                shape.push((K::FunctionOperand, x + 6));
            }
            shape.push((K::LinkReference(if i == 4 { 0x69 } else { 0x68 }), x + 9));
        }
        if i == 2 {
            shape.extend([
                (K::ShortWire, x + 12),
                (K::BranchStart, 2),
                (K::Contact(7), x + 15),
                (K::LongWire, x + 18),
                (K::Coil(14), 94),
            ]);
        }
        if (3..depth).contains(&i) {
            shape.extend([(K::BranchEnd, x + 14), (K::BranchStart, 2)]);
        }
        if i == depth {
            shape.push((K::Contact(6), 1));
            for at in (4..=x + 12).step_by(3) {
                shape.push((if x == 7 { K::LongWire } else { K::ShortWire }, at));
            }
            shape.push((K::BranchEnd, x + 14));
        }
        if rr.len() != shape.len()
            || rr
                .iter()
                .zip(&shape)
                .any(|(r, (k, at))| r.kind != *k || program.data[r.offset + 5] != *at)
        {
            return None;
        }
        for r in &rr {
            if r.kind == K::LongWire {
                let at = program.data[r.offset + 5];
                let end = if i == 0 {
                    4
                } else if i == 1 {
                    x + 6
                } else if i == 2 {
                    91
                } else {
                    at
                };
                if program.data[r.offset..r.end] != wire(f.row_index, at, end) {
                    return None;
                }
            }
            if r.kind == K::FunctionOperand && program.data[r.offset + 9..r.offset + 15] != [0; 6] {
                return None;
            }
        }
        rows.insert(
            f.row_index,
            (
                program.data[f.start..f.records_start].to_vec(),
                rr.iter()
                    .map(|r| program.data[r.offset..r.end].to_vec())
                    .collect(),
            ),
        );
    }
    let vertical = program
        .iec_geometry()?
        .vertical
        .into_iter()
        .filter(|v| (first.group_index..=last.group_index).contains(&v.group_index))
        .collect::<Vec<_>>();
    if vertical.len() != usize::from(depth - 2)
        || (0..depth - 2).any(|i| {
            !vertical.iter().any(|v| {
                v.start_row_index == row + 2 + i && v.end_row_index == row + 3 + i && v.x == x + 14
            })
        })
    {
        return None;
    }
    if program.iec_circuit_graph().is_none() {
        return None;
    }
    Some(Pair {
        row,
        x,
        first_group: first.group_index,
        last_group: last.group_index,
        rows,
        head: head.is_some(),
        tail: tail.is_some(),
    })
}

pub(crate) fn scaffold(program: &LadderProgramData, row: u16, x: u8) -> Option<usize> {
    let head = [4, 7].contains(&x);
    let r = if head { row } else { row.checked_sub(1)? };
    let at = if head { x } else { x.checked_sub(9)? };
    let pair = parse(program, r, at)?;
    if !(if head { pair.head } else { pair.tail }) {
        return Some(pair.first_group);
    }

    None
}

pub(crate) fn sites(
    program: &LadderProgramData,
) -> Option<Vec<crate::IecWiredComparisonInsertionSite>> {
    let mut candidates = std::collections::BTreeSet::new();
    for b in program.iec_function_blocks()? {
        if b.opcode_family == 0x28 {
            if [4, 7].contains(&b.raw_x) {
                candidates.insert((b.row_index, b.raw_x));
            }
            if [13, 16].contains(&b.raw_x) {
                if let Some(row) = b.row_index.checked_sub(1) {
                    candidates.insert((row, b.raw_x - 9));
                }
            }
        }
    }
    let vertical = program.iec_geometry()?.vertical;
    for v in &vertical {
        if [18, 21].contains(&v.x)
            && !vertical.iter().any(|p| {
                p.group_index == v.group_index && p.end_row_index == v.start_row_index && p.x == v.x
            })
        {
            if let Some(row) = v.start_row_index.checked_sub(2) {
                candidates.insert((row, v.x - 14));
            }
        }
    }
    let mut sites = Vec::new();
    for (row, x) in candidates {
        if let Some(pair) = parse(program, row, x) {
            if !pair.head {
                sites.push(crate::IecWiredComparisonInsertionSite {
                    group_index: pair.first_group,
                    row_index: row,
                    raw_x: x,
                });
            }
            if !pair.tail {
                sites.push(crate::IecWiredComparisonInsertionSite {
                    group_index: pair.first_group,
                    row_index: row + 1,
                    raw_x: x + 9,
                });
            }
        }
    }

    Some(sites)
}

fn serialize(program: &LadderProgramData, pair: Pair) -> Result<Vec<u8>, XgwxError> {
    let fail = || XgwxError::UnsupportedLadderLayout;
    let frames = program.iec_row_frames().ok_or_else(fail)?;
    let count = usize::from(u16::from_le_bytes(program.data[6..8].try_into().unwrap()));
    let mut groups = Vec::new();
    let mut g = 0;
    while g < count {
        if g == pair.first_group {
            let partitions = if pair.head {
                vec![pair.rows.keys().copied().collect::<Vec<_>>()]
            } else {
                vec![
                    vec![pair.row],
                    pair.rows
                        .keys()
                        .copied()
                        .filter(|r| *r != pair.row)
                        .collect(),
                ]
            };
            for keys in partitions {
                let mut bytes = vec![0; 10];
                bytes[8..10].copy_from_slice(&(keys.len() as u16).to_le_bytes());
                for key in keys {
                    let (header, records) = &pair.rows[&key];
                    bytes.extend(header);
                    for record in records {
                        bytes.extend(record);
                    }
                }
                groups.push(bytes);
            }
            g = pair.last_group + 1;
        } else {
            let owned = frames
                .iter()
                .filter(|r| r.group_index == g)
                .collect::<Vec<_>>();
            groups.push(
                program.data[owned[0].start - 10..owned.last().ok_or_else(fail)?.end].to_vec(),
            );
            g += 1;
        }
    }
    let mut out = program.data[..8].to_vec();
    out[6..8].copy_from_slice(&(groups.len() as u16).to_le_bytes());
    for (index, mut bytes) in groups.into_iter().enumerate() {
        bytes[..4].copy_from_slice(&(index as u32).to_le_bytes());
        out.extend(bytes);
    }
    out.extend(&program.data[frames.last().ok_or_else(fail)?.end..]);
    let mut verified = program.clone();
    verified.data = out;
    verified.decoded_len = verified.data.len();
    if parse(&verified, pair.row, pair.x).is_none() {
        return Err(fail());
    }
    Ok(verified.data)
}

fn refresh(pair: &mut Pair, change_head: bool) {
    let base = pair.row;
    let x = pair.x;
    let last = *pair.rows.keys().last().unwrap();
    for (r, (h, records)) in &mut pair.rows {
        let i = *r - base;
        if i == 0 && change_head {
            h[17..21].copy_from_slice(&50u32.to_le_bytes());
        }
        if i == 1 {
            h[17..21].copy_from_slice(&39u32.to_le_bytes());
        }
        if i == 2 {
            h[17..21].copy_from_slice(&50u32.to_le_bytes());
        }
        h[29] = match i {
            0 => {
                if pair.head {
                    x
                } else {
                    1
                }
            }
            1 => {
                if pair.tail {
                    x + 9
                } else {
                    x
                }
            }
            2 => x + 15,
            _ if *r != last => x + 14,
            _ => x + 13,
        };
        h[33..35].copy_from_slice(&(records.len() as u16).to_le_bytes());
        if i == 0 && pair.head {
            if let Some(b) = records.iter_mut().find(|b| b[1] == 0x67) {
                b[12] = if pair.tail { 0 } else { 2 };
            }
        }
    }
}

pub(crate) fn remove(program: &LadderProgramData, offset: usize) -> Result<Vec<u8>, XgwxError> {
    let fail = || XgwxError::UnsupportedLadderLayout;
    let b = program
        .iec_function_blocks()
        .ok_or_else(fail)?
        .into_iter()
        .find(|b| b.record_offset == offset)
        .ok_or_else(fail)?;
    let head = [4, 7].contains(&b.raw_x);
    let row = if head {
        b.row_index
    } else {
        b.row_index.checked_sub(1).ok_or_else(fail)?
    };
    let x = if head {
        b.raw_x
    } else {
        b.raw_x.checked_sub(9).ok_or_else(fail)?
    };
    let mut pair = parse(program, row, x).ok_or_else(fail)?;
    let target_row = b.row_index;
    let target_x = b.raw_x;
    for (r, (_, records)) in &mut pair.rows {
        records.retain(|record| {
            let at = record[5];
            !((*r == target_row
                && ((record[1] == 0x67 && at == target_x)
                    || (record[1] == 2 && at == if head { 4 } else { x + 3 })))
                || ((*r == target_row + 1 || *r == target_row + 2)
                    && record[1] == 0x46
                    && at == target_x - 3)
                || matches!(record[1], 0x68 | 0x69) && at == target_x)
        });
    }
    if head {
        pair.head = false;
    } else {
        pair.tail = false;
    }
    pair.rows.retain(|_, (_, records)| !records.is_empty());
    refresh(&mut pair, head);
    serialize(program, pair)
}

pub(crate) fn insert(
    program: &LadderProgramData,
    row: u16,
    x: u8,
    name: &str,
    operands: &[String],
) -> Result<Vec<u8>, XgwxError> {
    let fail = || XgwxError::UnsupportedLadderLayout;
    if spec(name).is_none_or(|(family, _, pins)| family != 0x28 || pins != 3)
        || operands.len() != 2
        || operands.iter().any(|v| {
            v.is_empty() || v.encode_utf16().count() > 255 || v.chars().any(char::is_control)
        })
    {
        return Err(fail());
    }
    let head = [4, 7].contains(&x);
    let base = if head {
        row
    } else {
        row.checked_sub(1).ok_or_else(fail)?
    };
    let at = if head {
        x
    } else {
        x.checked_sub(9).ok_or_else(fail)?
    };
    let mut pair = parse(program, base, at).ok_or_else(fail)?;
    if if head { pair.head } else { pair.tail } {
        return Err(fail());
    }
    if !pair.rows.contains_key(&(base + 1)) {
        let mut h = row_header(base + 1);
        if pair.rows[&(base + 2)].0[26..28] == [0, 0] {
            h[26..28].fill(0);
        }
        pair.rows.insert(base + 1, (h, vec![]));
    }
    let mut b = body(name, row, x);
    b[12] = if head && !pair.tail { 2 } else { 0 };
    let start = pair.rows.get_mut(&row).ok_or_else(fail)?;
    if head {
        if x == 7 {
            start.1.push(wire(row, 4, 4));
        }
    } else {
        start.1.push(wire(row, at + 3, at + 6));
    }
    start.1.push(b);
    for i in 1..=3u16 {
        let mut added = Vec::new();
        if i < 3 {
            added.push(expression(row + i, x - 3, &operands[usize::from(i - 1)]));
        }
        let y = (row * 4).to_le_bytes();
        added.push(vec![
            i as u8,
            if i == 3 { 0x69 } else { 0x68 },
            0,
            0,
            0,
            x,
            y[0],
            y[1],
            0,
        ]);
        let records = &mut pair.rows.get_mut(&(row + i)).ok_or_else(fail)?.1;
        let position = if head {
            0
        } else {
            records
                .iter()
                .take_while(|r| matches!(r[1], 0x46 | 0x68 | 0x69) && r[5] <= at)
                .count()
        };
        records.splice(position..position, added);
    }
    if head {
        pair.head = true;
    } else {
        pair.tail = true;
    }
    refresh(&mut pair, head);
    serialize(program, pair)
}
