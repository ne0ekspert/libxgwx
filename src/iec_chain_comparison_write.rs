//! Refill captured contact-fed comparison heads with one or two branch spines.
use crate::iec_function_write::{
    body_with_flags, branch_end, branch_start, row_header, spec, wire,
};
use crate::{IecRecordKind as K, LadderProgramData, XgwxError};

fn prefix(program: &LadderProgramData, row: u16, x: u8, gap: u16, split: bool) -> Option<usize> {
    if x == 19 {
        return enabled_prefix(program, row, gap, split);
    }
    if x != 16 || row > u16::MAX / 4 - 5 || ![3, 4].contains(&gap) {
        return None;
    }
    let rows = program.iec_row_frames()?;
    let records = program.iec_record_frames()?;
    let top = rows.iter().find(|r| r.row_index == row)?;
    let first = rows.iter().find(|r| r.group_index == top.group_index)?;
    if first.row_index != row || program.data.get(first.start - 6..first.start - 2)? != [1, 0, 0, 0]
    {
        return None;
    }
    let expected = [
        vec![
            (K::Contact(6), 1),
            (K::BranchStart, 3),
            (K::ShortWire, 4),
            (K::LongWire, 7),
            (K::BranchStart, 12),
        ],
        vec![
            (K::Contact(7), 1),
            (K::BranchEnd, 3),
            (K::BranchEnd, 12),
            (K::BranchStart, 12),
        ],
        vec![(K::BranchEnd, 12), (K::BranchStart, 12)],
    ];
    for (index, expected) in expected.iter().enumerate() {
        let y = row + index as u16;
        let frame = rows
            .iter()
            .find(|r| r.group_index == top.group_index && r.row_index == y)?;
        let at = records
            .iter()
            .filter(|r| r.group_index == top.group_index && r.row_index == y)
            .collect::<Vec<_>>();
        if at.len() != expected.len() {
            return None;
        }
        for (record, &(kind, position)) in at.iter().zip(expected) {
            let bytes = &program.data[record.offset..record.end];
            let stored_x = bytes[if kind == K::BranchStart { 7 } else { 5 }];
            if record.kind != kind || stored_x != position {
                return None;
            }
            match kind {
                K::BranchStart => {
                    if bytes[8..10] != (y * 4).to_le_bytes()
                        || bytes[18..20]
                            != ((y + if index == 2 && !split { gap - 2 } else { 1 }) * 4)
                                .to_le_bytes()
                        || !matches!(bytes[13], 0 | 4)
                        || bytes[23] != bytes[13]
                    {
                        return None;
                    }
                }
                K::BranchEnd => {
                    if bytes[6..8] != ((y - 1) * 4).to_le_bytes() {
                        return None;
                    }
                }
                K::LongWire => {
                    if bytes[15] != 10 || bytes[9..15] != [0; 6] {
                        return None;
                    }
                }
                K::ShortWire => {
                    if bytes[9..15] != [0; 6] {
                        return None;
                    }
                }
                K::Contact(_) => {
                    if bytes[9..15] != [1, 0, 0, 0, 0, 0] {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        if program.data[frame.start + 17..frame.start + 21] != 39u32.to_le_bytes() {
            return None;
        }
    }
    let next = program
        .iec_function_blocks()?
        .into_iter()
        .find(|b| b.group_index == top.group_index && b.row_index == row + gap && b.raw_x == x)?;
    if next.opcode_family != 0x28 || next.pin_count != 3 || next.instance.is_some() {
        return None;
    }
    Some(top.group_index)
}

// Native enabled comparison head or middle with outer x3 and inner x15 feeds.
fn enabled_prefix(program: &LadderProgramData, row: u16, gap: u16, split: bool) -> Option<usize> {
    if row > u16::MAX / 4 - 5 || ![3, 4].contains(&gap) {
        return None;
    }
    let rows = program.iec_row_frames()?;
    let records = program.iec_record_frames()?;
    let top = rows.iter().find(|r| r.row_index == row)?;
    let first = rows.iter().find(|r| r.group_index == top.group_index)?;
    if program.data.get(first.start - 6..first.start - 2)? != [0; 4] {
        return None;
    }
    let middle = first.row_index != row;
    if middle {
        let previous_row = row.checked_sub(4)?;
        let previous = program.iec_function_blocks()?.into_iter().find(|b| {
            b.group_index == top.group_index && b.row_index == previous_row && b.raw_x == 19
        })?;
        if previous.opcode_family != 0x28 || previous.pin_count != 3 || previous.instance.is_some()
        {
            return None;
        }
    }
    let top_records = records
        .iter()
        .filter(|r| r.group_index == top.group_index && r.row_index == row)
        .collect::<Vec<_>>();
    // Contact edits leave gaps in this feed; recognize the surviving records
    // instead of requiring the original three contacts and their original kinds.
    let contact_fed = middle
        && top_records.len() >= 3
        && top_records[0].kind == K::BranchEnd
        && program.data[top_records[0].offset + 5] == 3
        && top_records[1].kind == K::ShortWire
        && program.data[top_records[1].offset + 5] == 4
        && top_records.last()?.kind == K::BranchStart
        && program.data[top_records.last()?.offset + 7] == 15
        && top_records[2..top_records.len() - 1].iter().all(|r| {
            matches!(r.kind, K::Contact(6..=11))
                && [7, 10, 13].contains(&program.data[r.offset + 5])
        })
        && top_records[2..top_records.len() - 1]
            .windows(2)
            .all(|pair| program.data[pair[0].offset + 5] < program.data[pair[1].offset + 5]);
    let depleted_contact_feed = middle
        && top_records.len() == 2
        && top_records[0].kind == K::BranchEnd
        && program.data[top_records[0].offset + 5] == 3
        && top_records[1].kind == K::BranchStart
        && program.data[top_records[1].offset + 7] == 15;
    let outer_only = middle && top_records.len() == 2 && !depleted_contact_feed;
    let mut pairs = vec![
        (K::BranchEnd, 3),
        (K::BranchStart, 3),
        (K::BranchEnd, 15),
        (K::BranchStart, 15),
    ];
    if outer_only {
        pairs.truncate(2);
    } else if contact_fed || depleted_contact_feed {
        pairs.drain(..2);
    }
    let head = vec![
        (K::Contact(6), 1),
        (K::BranchStart, 3),
        (K::ShortWire, 4),
        (K::Contact(6), 7),
        (K::Contact(7), 10),
        (K::Contact(7), 13),
        (K::BranchStart, 15),
    ];
    let expected = [
        if contact_fed || depleted_contact_feed {
            top_records
                .iter()
                .map(|r| {
                    (
                        r.kind,
                        program.data[r.offset + if r.kind == K::BranchStart { 7 } else { 5 }],
                    )
                })
                .collect()
        } else if middle {
            pairs.clone()
        } else {
            head
        },
        pairs.clone(),
        pairs,
    ];
    for (index, shape) in expected.iter().enumerate() {
        let y = row + index as u16;
        let frame = rows
            .iter()
            .find(|r| r.group_index == top.group_index && r.row_index == y)?;
        let at = records
            .iter()
            .filter(|r| r.group_index == top.group_index && r.row_index == y)
            .collect::<Vec<_>>();
        let height = &program.data[frame.start + 17..frame.start + 21];
        let native_middle_height = middle && index == 0 && height == 73u32.to_le_bytes();
        if at.len() != shape.len() || (height != 39u32.to_le_bytes() && !native_middle_height) {
            return None;
        }
        for (record, &(kind, x)) in at.iter().zip(shape) {
            let bytes = &program.data[record.offset..record.end];
            if record.kind != kind || bytes[if kind == K::BranchStart { 7 } else { 5 }] != x {
                return None;
            }
            match kind {
                K::BranchStart => {
                    if bytes[8..10] != (y * 4).to_le_bytes()
                        || bytes[18..20]
                            != ((y + if index == 2 && !split { gap - 2 } else { 1 }) * 4)
                                .to_le_bytes()
                        || bytes[13] != 0
                        || bytes[23] != 0
                        || bytes[17] != x - 1
                    {
                        return None;
                    }
                }
                K::BranchEnd => {
                    if bytes[6..8] != ((y - 1) * 4).to_le_bytes() {
                        return None;
                    }
                }
                K::ShortWire => {
                    if bytes[9..15] != [0; 6] {
                        return None;
                    }
                }
                K::Contact(_) => {
                    if bytes[9..15] != [1, 0, 0, 0, 0, 0] {
                        return None;
                    }
                }
                _ => return None,
            }
        }
    }
    let next = program
        .iec_function_blocks()?
        .into_iter()
        .find(|b| b.group_index == top.group_index && b.row_index == row + gap && b.raw_x == 19)?;
    (next.opcode_family == 0x28 && next.pin_count == 3 && next.instance.is_none())
        .then_some(top.group_index)
}

fn boundaries(program: &LadderProgramData, row: u16, x: u8) -> Vec<u8> {
    if x == 16 {
        return vec![12];
    }
    program
        .iec_record_frames()
        .unwrap_or_default()
        .iter()
        .filter(|r| r.row_index == row && r.kind == K::BranchStart)
        .map(|r| program.data[r.offset + 7])
        .collect()
}

pub(crate) fn feed_boundary(program: &LadderProgramData, row: u16, x: u8) -> Option<u8> {
    scaffold(program, row, x).then(|| *boundaries(program, row, x).last().unwrap())
}

pub(crate) fn needs_room(program: &LadderProgramData, row: u16, x: u8) -> bool {
    prefix(program, row, x, 3, false).is_some()
}

pub(crate) fn scaffold(program: &LadderProgramData, row: u16, x: u8) -> bool {
    if ![16, 19].contains(&x) || row > u16::MAX / 4 - 5 {
        return false;
    }
    let Some(records) = program.iec_record_frames() else {
        return false;
    };
    let Some(group) = prefix(program, row, x, 4, true) else {
        return false;
    };
    let at = records
        .iter()
        .filter(|r| r.group_index == group && r.row_index == row + 3)
        .collect::<Vec<_>>();
    let boundaries = boundaries(program, row, x);
    let flags = if x == 19 { 0 } else { 4 };
    if at.len() != boundaries.len() * 2 {
        return false;
    }
    boundaries
        .iter()
        .zip(at.chunks_exact(2))
        .all(|(&boundary, pair)| {
            let [end, start] = pair else {
                return false;
            };
            let Some(next_end) = records.iter().find(|r| {
                r.row_index == row + 4
                    && r.group_index == group
                    && r.kind == K::BranchEnd
                    && program.data[r.offset + 5] == boundary
            }) else {
                return false;
            };
            end.kind == K::BranchEnd
                && start.kind == K::BranchStart
                && program.data[end.offset + 5] == boundary
                && program.data[end.offset + 6..end.offset + 8] == ((row + 2) * 4).to_le_bytes()
                && program.data[start.offset + 7] == boundary
                && program.data[start.offset + 18..start.offset + 20]
                    == ((row + 4) * 4).to_le_bytes()
                && program.data[start.offset + 13] == flags
                && program.data[start.offset + 23] == flags
                && program.data[next_end.offset + 6..next_end.offset + 8]
                    == ((row + 3) * 4).to_le_bytes()
        })
}

/// Materialize the implicit row after native Ctrl+L and split its branch feed.
pub(crate) fn materialize(
    program: &LadderProgramData,
    row: u16,
    x: u8,
) -> Result<Vec<u8>, XgwxError> {
    let unsupported = || XgwxError::UnsupportedLadderLayout;
    let group = prefix(program, row, x, 4, false).ok_or_else(unsupported)?;
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    if rows.iter().any(|r| r.row_index == row + 3) {
        return Err(unsupported());
    }
    let first = rows
        .iter()
        .find(|r| r.group_index == group)
        .ok_or_else(unsupported)?;
    let next = rows
        .iter()
        .find(|r| r.group_index == group && r.row_index == row + 4)
        .ok_or_else(unsupported)?;
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let boundaries = boundaries(program, row, x);
    let flags = if x == 19 { 0 } else { 4 };
    let mut out = program.data.clone();
    for &boundary in &boundaries {
        let start = records
            .iter()
            .find(|r| {
                r.group_index == group
                    && r.row_index == row + 2
                    && r.kind == K::BranchStart
                    && program.data[r.offset + 7] == boundary
            })
            .ok_or_else(unsupported)?;
        let end = records
            .iter()
            .find(|r| {
                r.group_index == group
                    && r.row_index == row + 4
                    && r.kind == K::BranchEnd
                    && program.data[r.offset + 5] == boundary
            })
            .ok_or_else(unsupported)?;
        out[start.offset + 18..start.offset + 20].copy_from_slice(&((row + 3) * 4).to_le_bytes());
        out[end.offset + 6..end.offset + 8].copy_from_slice(&((row + 3) * 4).to_le_bytes());
    }
    let count = u16::from_le_bytes(out[first.start - 2..first.start].try_into().unwrap());
    out[first.start - 2..first.start]
        .copy_from_slice(&count.checked_add(1).ok_or_else(unsupported)?.to_le_bytes());
    let mut added = row_header(row + 3);
    added[29] = *boundaries.last().unwrap();
    added[33..35].copy_from_slice(&(boundaries.len() as u16 * 2).to_le_bytes());
    for &boundary in &boundaries {
        added.extend(branch_end(row + 3, boundary));
        added.extend(branch_start(row + 3, boundary, flags));
    }
    out.splice(next.start..next.start, added);
    // Replacing the comparison restores its x4 contact feed. This allows the
    // missing contacts to be restored using ordinary cell insertion afterward.
    let top = rows
        .iter()
        .find(|r| r.group_index == group && r.row_index == row)
        .ok_or_else(unsupported)?;
    let at = records
        .iter()
        .filter(|r| r.group_index == group && r.row_index == row)
        .collect::<Vec<_>>();
    if x == 19
        && at.len() == 2
        && at[0].kind == K::BranchEnd
        && program.data[at[0].offset + 5] == 3
        && at[1].kind == K::BranchStart
        && program.data[at[1].offset + 7] == 15
    {
        let y = (row * 4).to_le_bytes();
        out[top.start + 33..top.start + 35].copy_from_slice(&3u16.to_le_bytes());
        out.splice(
            at[0].end..at[0].end,
            [255, 1, 0, 0, 0, 4, y[0], y[1], 0, 0, 0, 0, 0, 0, 0],
        );
    }
    let mut verified = program.clone();
    verified.decoded_len = out.len();
    verified.data = out;
    if !scaffold(&verified, row, x) || verified.iec_circuit_layout().is_none() {
        return Err(unsupported());
    }
    Ok(verified.data)
}

/// Retain the restored branch rows when removing a newly placed comparison.
pub(crate) fn remove(
    program: &LadderProgramData,
    offset: usize,
    expected: &str,
) -> Result<Vec<u8>, XgwxError> {
    let unsupported = || XgwxError::UnsupportedLadderLayout;
    let blocks = program.iec_function_blocks().ok_or_else(unsupported)?;
    let block = blocks
        .iter()
        .find(|b| b.record_offset == offset)
        .ok_or_else(unsupported)?;
    if block.name.value != expected
        || ![16, 19].contains(&block.raw_x)
        || block.row_index > u16::MAX / 4 - 5
        || spec(expected).is_none_or(|(family, _, count)| family != 0x28 || count != 3)
        || program.data.get(block.record_offset..block.record_end)
            != Some(
                body_with_flags(
                    expected,
                    block.row_index,
                    block.raw_x,
                    if block.raw_x == 19 { 0 } else { 4 },
                )
                .as_slice(),
            )
    {
        return Err(unsupported());
    }
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let refs = program.iec_function_references().ok_or_else(unsupported)?;
    let links = program
        .iec_function_operand_links()
        .ok_or_else(unsupported)?;
    let owned_refs = refs
        .iter()
        .filter(|r| r.target_record_offset == offset)
        .collect::<Vec<_>>();
    let owned_links = links
        .iter()
        .filter(|r| r.target_record_offset == offset)
        .collect::<Vec<_>>();
    if owned_refs.len() != 3 || owned_links.len() != 3 {
        return Err(unsupported());
    }
    let boundary = records
        .iter()
        .filter(|r| {
            r.group_index == block.group_index
                && r.row_index == block.row_index
                && r.kind == K::BranchStart
        })
        .map(|r| program.data[r.offset + 7])
        .max()
        .ok_or_else(unsupported)?;
    let mut feed = wire(block.row_index, boundary + 1, block.raw_x - 3);
    feed[11] = if block.raw_x == 19 { 0 } else { 4 };
    let feed = records
        .iter()
        .find(|r| {
            r.row_index == block.row_index
                && r.group_index == block.group_index
                && program.data.get(r.offset..r.end) == Some(feed.as_slice())
        })
        .ok_or_else(unsupported)?;
    let removed = records
        .iter()
        .filter(|r| {
            r.offset == offset
                || r.offset == feed.offset
                || owned_refs
                    .iter()
                    .any(|reference| reference.record_offset == r.offset)
                || owned_links
                    .iter()
                    .any(|link| link.record_offset == r.offset)
        })
        .collect::<Vec<_>>();
    if removed.len() != 8 {
        return Err(unsupported());
    }
    let mut out = program.data.clone();
    for row in rows.iter().filter(|r| {
        r.group_index == block.group_index
            && (block.row_index..=block.row_index + 3).contains(&r.row_index)
    }) {
        let count = removed
            .iter()
            .filter(|r| r.row_index == row.row_index)
            .count() as u16;
        out[row.start + 33..row.start + 35].copy_from_slice(
            &row.record_count
                .checked_sub(count)
                .ok_or_else(unsupported)?
                .to_le_bytes(),
        );
        out[row.start + 29] = boundary;
    }
    for record in removed.iter().rev() {
        out.drain(record.offset..record.end);
    }
    let mut verified = program.clone();
    verified.decoded_len = out.len();
    verified.data = out;
    if !scaffold(&verified, block.row_index, block.raw_x)
        || verified.iec_circuit_layout().is_none()
        || verified
            .iec_function_blocks()
            .is_none_or(|b| b.len() + 1 != blocks.len())
    {
        return Err(unsupported());
    }
    Ok(verified.data)
}
