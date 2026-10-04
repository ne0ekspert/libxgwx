//! Native enabled and disabled TON editing with a retained contact branch and coil.
use crate::{IecRecordKind as K, LadderProgramData, XgwxError};

fn text(out: &mut Vec<u8>, value: &str) {
    let units = value.encode_utf16().collect::<Vec<_>>();
    out.extend_from_slice(&[0xff, 0xfe, 0xff, units.len() as u8]);
    for unit in units {
        out.extend(unit.to_le_bytes());
    }
}

fn pin(out: &mut Vec<u8>, flags: u32, name: &str, role: u8) {
    text(out, "");
    out.extend(flags.to_le_bytes());
    text(out, name);
    out.extend([role, 0, 0, 0, 0]);
}

pub(crate) fn body_at(row: u16, x: u8, instance: &str, terminal: bool) -> Vec<u8> {
    let y = (row * 4).to_le_bytes();
    let mut out = vec![
        0,
        0x67,
        0,
        0,
        0,
        x,
        y[0],
        y[1],
        0,
        1,
        0,
        0,
        if terminal { 2 } else { 0 },
        0,
        0,
        0x21,
        0x51,
        0,
        0,
        0,
        2,
        0,
        0,
        0,
        3,
        0,
        x,
        y[0],
        y[1],
        0,
        1,
        0,
        0,
        0,
        0,
        0,
        2,
        0,
    ];
    pin(&mut out, 0x0020_0001, "IN", 1);
    pin(&mut out, 1, "Q", 3);
    text(&mut out, "TON");
    text(&mut out, instance);
    for ordinal in 1..=2 {
        let y = ((row + ordinal) * 4).to_le_bytes();
        out.extend([
            x,
            y[0],
            y[1],
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            if ordinal == 1 { 2 } else { 0 },
            0,
        ]);
        if ordinal == 1 {
            pin(&mut out, 0x0020_8000, "PT", 1);
            pin(&mut out, 0x8000, "ET", 3);
        }
    }
    out
}

/// The captured contact branch survives native deletion with two stored rows.
pub(crate) fn scaffold(
    program: &LadderProgramData,
    contact_offset: usize,
) -> Option<crate::IecTerminalFunctionInsertionSite> {
    let rows = program.iec_row_frames()?;
    let records = program.iec_record_frames()?;
    let contact = records.iter().find(|r| r.offset == contact_offset)?;
    let group = rows
        .iter()
        .filter(|r| r.group_index == contact.group_index)
        .collect::<Vec<_>>();
    let [top, lower] = group.as_slice() else {
        return None;
    };
    let row = top.row_index;
    if row > u16::MAX / 4 - 2
        || lower.row_index != row + 1
        || !matches!(
            program.data.get(top.start - 6..top.start - 2)?,
            [0, 0, 0, 0] | [1, 0, 0, 0]
        )
        || rows.iter().any(|r| r.row_index == row + 2)
        || rows
            .iter()
            .find(|r| r.group_index == contact.group_index + 1)?
            .row_index
            != row + 3
        || program
            .iec_function_blocks()?
            .iter()
            .any(|b| b.group_index == contact.group_index)
    {
        return None;
    }
    let expected = [
        (row, K::Contact(6), 1),
        (row, K::ShortWire, 4),
        (row, K::BranchStart, 2),
        (row, K::LongWire, 7),
        (row, K::Contact(7), 10),
        (row, K::Contact(7), 13),
        (row, K::LongWire, 16),
        (row, K::LongWire, 22),
        (row, K::Coil(14), 94),
        (row + 1, K::Contact(6), 1),
        (row + 1, K::ShortWire, 4),
        (row + 1, K::BranchEnd, 6),
    ];
    let owned = records
        .iter()
        .filter(|r| r.group_index == contact.group_index)
        .collect::<Vec<_>>();
    let branches = program
        .iec_geometry()?
        .vertical
        .into_iter()
        .filter(|b| b.group_index == contact.group_index)
        .collect::<Vec<_>>();
    if owned.len() != expected.len()
        || owned.iter().zip(expected).any(|(r, (y, kind, x))| {
            r.row_index != y || r.kind != kind || program.data[r.offset + 5] != x
        })
        || owned[0].offset != contact_offset
        || [owned[3], owned[6], owned[7]]
            .iter()
            .zip([7, 16, 91])
            .any(|(r, x)| program.data[r.offset + 15] != x)
        || program.iec_circuit_graph().is_none()
        || branches.len() != 1
        || branches[0].start_row_index != row
        || branches[0].end_row_index != row + 1
        || branches[0].x != 6
    {
        return None;
    }
    Some(crate::IecTerminalFunctionInsertionSite {
        group_index: contact.group_index,
        row_index: row,
        contact_offset,
        insertion_offset: owned[7].offset,
        raw_x: 19,
    })
}

pub(crate) fn remove(program: &LadderProgramData, offset: usize) -> Result<Vec<u8>, XgwxError> {
    let fail = || XgwxError::UnsupportedLadderLayout;
    let blocks = program.iec_function_blocks().ok_or_else(fail)?;
    let block = blocks
        .iter()
        .find(|b| b.record_offset == offset)
        .ok_or_else(fail)?;
    if block.name.value != "TON"
        || block.raw_x != 19
        || block.opcode_family != 0x21
        || block.opcode != 0x51
        || block.pin_count != 2
        || block.instance.is_none()
        || blocks
            .iter()
            .filter(|b| b.group_index == block.group_index)
            .count()
            != 1
    {
        return Err(fail());
    }
    // Both captured binding encodings occur in native source files; fresh
    // insertion uses zero. Reject other encodings rather than infer semantics.
    let flags = *program.data.get(offset + 32).ok_or_else(fail)?;
    let mut canonical = body_at(
        block.row_index,
        19,
        &block.instance.as_ref().unwrap().value,
        false,
    );
    let len = canonical.len();
    canonical[32] = flags;
    canonical[len - 60] = flags;
    canonical[len - 6] = flags;
    if ![0, 2].contains(&flags)
        || program.data.get(offset..block.record_end) != Some(canonical.as_slice())
    {
        return Err(fail());
    }
    let rows = program.iec_row_frames().ok_or_else(fail)?;
    let records = program.iec_record_frames().ok_or_else(fail)?;
    let group = rows
        .iter()
        .filter(|r| r.group_index == block.group_index)
        .collect::<Vec<_>>();
    let [top, lower, last] = group.as_slice() else {
        return Err(fail());
    };
    if top.row_index != block.row_index
        || lower.row_index != block.row_index + 1
        || last.row_index != block.row_index + 2
    {
        return Err(fail());
    }
    let links = program.iec_function_operand_links().ok_or_else(fail)?;
    let owned = links
        .iter()
        .filter(|l| l.target_record_offset == offset)
        .collect::<Vec<_>>();
    let refs = program.iec_function_references().ok_or_else(fail)?;
    let refs = refs
        .iter()
        .filter(|r| r.target_record_offset == offset)
        .collect::<Vec<_>>();
    if owned.len() != 1
        || owned[0].is_output
        || owned[0].row_index != block.row_index + 1
        || refs.len() != 2
    {
        return Err(fail());
    }
    let removed = [
        offset,
        owned[0].record_offset,
        refs[0].record_offset,
        refs[1].record_offset,
    ];
    let mut replacement = program.data[top.start - 10..top.start].to_vec();
    replacement[8..10].copy_from_slice(&2u16.to_le_bytes());
    let enabled = program.data.get(top.start - 6..top.start - 2) == Some(&[0, 0, 0, 0]);
    for (row, height, xmax) in [
        (top, 50u32, 19u8),
        (lower, if enabled { 50 } else { 39 }, 5),
    ] {
        let retained = records
            .iter()
            .filter(|r| {
                r.group_index == block.group_index
                    && r.row_index == row.row_index
                    && !removed.contains(&r.offset)
            })
            .collect::<Vec<_>>();
        let mut header = program.data[row.start..row.records_start].to_vec();
        header[17..21].copy_from_slice(&height.to_le_bytes());
        header[29] = xmax;
        header[33..35].copy_from_slice(&(retained.len() as u16).to_le_bytes());
        replacement.extend(header);
        for r in retained {
            replacement.extend(&program.data[r.offset..r.end]);
        }
    }
    if records.iter().any(|r| {
        r.group_index == block.group_index
            && r.row_index == last.row_index
            && !removed.contains(&r.offset)
    }) {
        return Err(fail());
    }
    let mut verified = program.clone();
    verified.data.splice(top.start - 10..last.end, replacement);
    verified.decoded_len = verified.data.len();
    let contact = records
        .iter()
        .find(|r| r.group_index == block.group_index)
        .ok_or_else(fail)?;
    if scaffold(&verified, contact.offset).is_none() {
        return Err(fail());
    }
    Ok(verified.data)
}

pub(crate) fn insert(
    program: &LadderProgramData,
    contact: usize,
    instance: &str,
    preset: &str,
    literal: bool,
) -> Result<Vec<u8>, XgwxError> {
    let fail = || XgwxError::UnsupportedLadderLayout;
    if instance.is_empty()
        || instance.encode_utf16().count() > 255
        || preset.encode_utf16().count() > 255
    {
        return Err(fail());
    }
    let site = scaffold(program, contact).ok_or_else(fail)?;
    let rows = program.iec_row_frames().ok_or_else(fail)?;
    let group = rows
        .iter()
        .filter(|r| r.group_index == site.group_index)
        .collect::<Vec<_>>();
    let [top, lower] = group.as_slice() else {
        return Err(fail());
    };
    let mut replacement = program.data[top.start - 10..lower.end].to_vec();
    let body = body_at(site.row_index, 19, instance, false);
    let enabled = program.data.get(top.start - 6..top.start - 2) == Some(&[0, 0, 0, 0]);
    let lower_at = lower.start - (top.start - 10);
    replacement[8..10].copy_from_slice(&3u16.to_le_bytes());
    replacement[27..31].copy_from_slice(&62u32.to_le_bytes());
    replacement[43..45].copy_from_slice(&10u16.to_le_bytes());
    replacement[lower_at + 29] = 19;
    replacement[lower_at + 33..lower_at + 35].copy_from_slice(&5u16.to_le_bytes());
    let y = (site.row_index * 4).to_le_bytes();
    let pin_y = ((site.row_index + 1) * 4).to_le_bytes();
    replacement.extend([
        0xff,
        0x46,
        0,
        0,
        0,
        16,
        pin_y[0],
        pin_y[1],
        0,
        0,
        0,
        if literal && !enabled { 4 } else { 0 },
        0,
        0,
        0,
    ]);
    text(&mut replacement, preset);
    replacement.extend([1, 0x68, 0, 0, 0, 19, y[0], y[1], 0]);
    let mut header = program.data[lower.start..lower.records_start].to_vec();
    let last_y = ((site.row_index + 2) * 4).to_le_bytes();
    header[0..4].copy_from_slice(&u32::from(site.row_index + 2).to_le_bytes());
    header[22..24].copy_from_slice(&last_y);
    // Native preserves the cached coordinate origin of the retained pin row.
    let cache_y = u16::from_le_bytes(header[26..28].try_into().unwrap())
        .checked_add(4)
        .ok_or_else(fail)?;
    header[26..28].copy_from_slice(&cache_y.to_le_bytes());
    header[17..21].copy_from_slice(&39u32.to_le_bytes());
    header[29] = 19;
    header[30..32].copy_from_slice(&last_y);
    header[33..35].copy_from_slice(&1u16.to_le_bytes());
    replacement.extend(header);
    replacement.extend([2, 0x69, 0, 0, 0, 19, y[0], y[1], 0]);
    let at = site.insertion_offset - (top.start - 10);
    replacement.splice(at..at, body);
    let mut verified = program.clone();
    verified.data.splice(top.start - 10..lower.end, replacement);
    verified.decoded_len = verified.data.len();
    let offset = site.insertion_offset;
    if verified.iec_circuit_graph().is_none() || remove(&verified, offset)? != program.data {
        return Err(fail());
    }
    Ok(verified.data)
}
