//! Native split of an enabled MOVE above an independently powered staggered MOVE.
use crate::iec_function_write::body;
use crate::{IecRecordKind as K, LadderProgramData, XgwxError};

/// A lone enabled contact immediately above an independent three-row MOVE.
/// This is the native saved result of removing the upper staggered MOVE.
pub(crate) fn scaffold(program: &LadderProgramData, row: u16, x: u8) -> bool {
    let parse = || -> Option<()> {
        if x != 16 || row > u16::MAX / 4 - 3 {
            return None;
        }
        let rows = program.iec_row_frames()?;
        let top = rows.iter().find(|r| r.row_index == row)?;
        let lower_top = rows.iter().find(|r| r.row_index == row + 1)?;
        let lower_rows = rows
            .iter()
            .filter(|r| r.group_index == lower_top.group_index)
            .collect::<Vec<_>>();
        if rows
            .iter()
            .filter(|r| r.group_index == top.group_index)
            .count()
            != 1
            || lower_top.group_index != top.group_index + 1
            || lower_rows.len() != 3
            || lower_rows
                .iter()
                .enumerate()
                .any(|(i, r)| r.row_index != row + 1 + i as u16)
            || [top, lower_top]
                .iter()
                .any(|r| program.data[r.start - 6..r.start - 2] != [0; 4])
            || std::iter::once(top)
                .chain(lower_rows.iter().copied())
                .any(|r| program.data[r.start + 17..r.start + 21] != 39u32.to_le_bytes())
        {
            return None;
        }
        let records = program.iec_record_frames()?;
        let contact = records
            .iter()
            .filter(|r| r.group_index == top.group_index)
            .collect::<Vec<_>>();
        if contact.len() != 1
            || contact[0].kind != K::Contact(6)
            || program.data[contact[0].offset + 5] != 1
            || program.data[contact[0].offset + 11..contact[0].offset + 15] != [0; 4]
        {
            return None;
        }
        let lower_records = records
            .iter()
            .filter(|r| r.group_index == lower_top.group_index)
            .collect::<Vec<_>>();
        let shape = lower_records
            .iter()
            .map(|r| (r.row_index - row, r.kind, program.data[r.offset + 5]))
            .collect::<Vec<_>>();
        if shape
            != vec![
                (1, K::LongWire, 1),
                (1, K::FunctionBlock, 4),
                (2, K::FunctionOperand, 1),
                (2, K::LinkReference(0x68), 4),
                (2, K::FunctionOperand, 7),
                (3, K::LinkReference(0x69), 4),
            ]
            || program.data[lower_records[0].offset + 15] != 1
            || program.data[lower_records[0].offset + 9..lower_records[0].offset + 15] != [0; 6]
        {
            return None;
        }
        let blocks = program.iec_function_blocks()?;
        let lower = blocks
            .iter()
            .find(|b| b.record_offset == lower_records[1].offset)?;
        if program.data.get(lower.record_offset..lower.record_end)? != body("MOVE", row + 1, 4)
            || program
                .iec_function_operand_links()?
                .iter()
                .filter(|l| l.target_record_offset == lower.record_offset)
                .count()
                != 2
            || program
                .iec_function_references()?
                .iter()
                .filter(|l| l.target_record_offset == lower.record_offset)
                .count()
                != 2
        {
            return None;
        }
        Some(())
    };
    parse().is_some()
}

pub(crate) fn remove_upper(
    program: &LadderProgramData,
    offset: usize,
) -> Result<Vec<u8>, XgwxError> {
    let unsupported = || XgwxError::UnsupportedLadderLayout;
    let blocks = program.iec_function_blocks().ok_or_else(unsupported)?;
    let upper = blocks
        .iter()
        .find(|b| b.record_offset == offset)
        .ok_or_else(unsupported)?;
    let row = upper.row_index;
    if row > u16::MAX / 4 - 3
        || upper.raw_x != 16
        || program.data.get(offset..upper.record_end) != Some(body("MOVE", row, 16).as_slice())
    {
        return Err(unsupported());
    }
    let owned_blocks = blocks
        .iter()
        .filter(|b| b.group_index == upper.group_index)
        .collect::<Vec<_>>();
    let lower = owned_blocks
        .iter()
        .find(|b| b.row_index == row + 1 && b.raw_x == 4)
        .ok_or_else(unsupported)?;
    if owned_blocks.len() != 2
        || program.data.get(lower.record_offset..lower.record_end)
            != Some(body("MOVE", row + 1, 4).as_slice())
    {
        return Err(unsupported());
    }
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    let group = rows
        .iter()
        .filter(|r| r.group_index == upper.group_index)
        .collect::<Vec<_>>();
    if group.len() != 4
        || group
            .iter()
            .enumerate()
            .any(|(i, r)| r.row_index != row + i as u16)
        || program.data[group[0].start - 6..group[0].start - 2] != [0; 4]
    {
        return Err(unsupported());
    }
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let at_top = records
        .iter()
        .filter(|r| r.group_index == upper.group_index && r.row_index == row)
        .collect::<Vec<_>>();
    let contact = at_top.first().ok_or_else(unsupported)?;
    if contact.kind != K::Contact(6)
        || program.data[contact.offset + 5] != 1
        || program.data[contact.offset + 11..contact.offset + 15] != [0; 4]
        || at_top.last().is_none_or(|r| r.offset != offset)
    {
        return Err(unsupported());
    }
    // Original projects can store a mixture of long and short feed segments.
    // Refill normalizes them to one long wire. Both must reach exactly EN.
    let mut cell = 4;
    for feed in &at_top[1..at_top.len() - 1] {
        if cell > 13
            || program.data[feed.offset + 5] != cell
            || program.data[feed.offset + 9..feed.offset + 15] != [0; 6]
        {
            return Err(unsupported());
        }
        cell = match feed.kind {
            K::ShortWire => cell + 3,
            K::LongWire
                if program.data[feed.offset + 15] >= cell
                    && program.data[feed.offset + 15] <= 13
                    && (program.data[feed.offset + 15] - cell).is_multiple_of(3) =>
            {
                program.data[feed.offset + 15] + 3
            }
            _ => return Err(unsupported()),
        };
    }
    if cell != 16 {
        return Err(unsupported());
    }
    let links = program
        .iec_function_operand_links()
        .ok_or_else(unsupported)?;
    let references = program.iec_function_references().ok_or_else(unsupported)?;
    for block in [upper, lower] {
        if links
            .iter()
            .filter(|l| l.target_record_offset == block.record_offset)
            .count()
            != 2
            || references
                .iter()
                .filter(|l| l.target_record_offset == block.record_offset)
                .count()
                != 2
        {
            return Err(unsupported());
        }
    }
    let mut removed = at_top.iter().skip(1).map(|r| r.offset).collect::<Vec<_>>();
    removed.extend(
        links
            .iter()
            .filter(|l| l.target_record_offset == offset)
            .map(|l| l.record_offset),
    );
    removed.extend(
        references
            .iter()
            .filter(|r| r.target_record_offset == offset)
            .map(|r| r.record_offset),
    );
    let retained = group
        .iter()
        .map(|r| {
            records
                .iter()
                .filter(|v| {
                    v.group_index == upper.group_index
                        && v.row_index == r.row_index
                        && !removed.contains(&v.offset)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let kinds = retained
        .iter()
        .map(|rs| {
            rs.iter()
                .map(|r| (r.kind, program.data[r.offset + 5]))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if kinds
        != vec![
            vec![(K::Contact(6), 1)],
            vec![(K::LongWire, 1), (K::FunctionBlock, 4)],
            vec![
                (K::FunctionOperand, 1),
                (K::LinkReference(0x68), 4),
                (K::FunctionOperand, 7),
            ],
            vec![(K::LinkReference(0x69), 4)],
        ]
        || program.data[retained[1][0].offset + 15] != 1
        || program.data[retained[1][0].offset + 9..retained[1][0].offset + 15] != [0; 6]
    {
        return Err(unsupported());
    }
    let old_count = u16::from_le_bytes(program.data[6..8].try_into().unwrap());
    let new_count = old_count.checked_add(1).ok_or_else(unsupported)?;
    let mut out = program.data[..group[0].start - 10].to_vec();
    out[6..8].copy_from_slice(&new_count.to_le_bytes());
    for (i, (frame, kept)) in group.iter().zip(retained).enumerate() {
        if i < 2 {
            out.extend_from_slice(&((upper.group_index + i) as u32).to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&(if i == 0 { 1u16 } else { 3 }).to_le_bytes());
        }
        let mut header = program.data[frame.start..frame.records_start].to_vec();
        header[17..21].copy_from_slice(&39u32.to_le_bytes());
        header[29] = kept
            .iter()
            .map(|r| program.data[r.offset + 5])
            .max()
            .unwrap();
        header[33..35].copy_from_slice(&(kept.len() as u16).to_le_bytes());
        out.extend(header);
        for record in kept {
            out.extend_from_slice(&program.data[record.offset..record.end]);
        }
    }
    for g in upper.group_index + 1..usize::from(old_count) {
        let next = rows
            .iter()
            .filter(|r| r.group_index == g)
            .collect::<Vec<_>>();
        let first = next.first().ok_or_else(unsupported)?.start - 10;
        let last = next.last().ok_or_else(unsupported)?.end;
        out.extend_from_slice(&((g + 1) as u32).to_le_bytes());
        out.extend_from_slice(&program.data[first + 4..last]);
    }
    let mut verified = program.clone();
    verified.decoded_len = out.len();
    verified.data = out;
    if (verified.iec_circuit_graph().is_none()
        && !crate::writer::iec_preserves_groups_around_edit(
            program,
            &verified,
            upper.group_index..upper.group_index + 1,
            2,
        ))
        || !scaffold(&verified, row, 16)
        || verified
            .iec_function_blocks()
            .is_none_or(|b| b.len() + 1 != blocks.len())
        || verified
            .iec_function_operand_links()
            .is_none_or(|l| l.len() + 2 != links.len())
        || verified
            .iec_function_references()
            .is_none_or(|r| r.len() + 2 != references.len())
    {
        return Err(unsupported());
    }
    Ok(verified.data)
}
