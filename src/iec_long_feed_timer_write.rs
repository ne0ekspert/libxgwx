//! Native terminal TON layouts with a single input or three series contacts.
use crate::{IecRecordFrame, IecRecordKind as K, LadderProgramData, XgwxError};

fn matches(program: &LadderProgramData, records: &[&IecRecordFrame], shape: &[(K, u8)]) -> bool {
    records.len() == shape.len()
        && records
            .iter()
            .zip(shape)
            .all(|(r, (kind, x))| r.kind == *kind && program.data.get(r.offset + 5) == Some(x))
}

fn series(program: &LadderProgramData, records: &[&IecRecordFrame]) -> bool {
    matches(
        program,
        records,
        &[
            (K::Contact(6), 1),
            (K::ShortWire, 4),
            (K::ShortWire, 7),
            (K::Contact(7), 10),
            (K::Contact(7), 13),
        ],
    )
}

pub(crate) fn scaffold(
    program: &LadderProgramData,
    contact: usize,
) -> Option<crate::IecTerminalFunctionInsertionSite> {
    let rows = program.iec_row_frames()?;
    let records = program.iec_record_frames()?;
    let first = records.iter().find(|r| r.offset == contact)?;
    let group = rows
        .iter()
        .filter(|r| r.group_index == first.group_index)
        .collect::<Vec<_>>();
    let [top] = group.as_slice() else { return None };
    let owned = records
        .iter()
        .filter(|r| r.group_index == first.group_index)
        .collect::<Vec<_>>();
    let single = matches(program, &owned, &[(K::Contact(6), 1)]);
    if (!single && !series(program, &owned))
        || owned[0].offset != contact
        || top.row_index > u16::MAX / 4 - 2
        || program.data.get(top.start - 6..top.start - 2)? != [0, 0, 0, 0]
        || program.data.get(top.start - 2..top.start)? != [1, 0]
        || program.data[top.start + 29] != if single { 1 } else { 13 }
        || program.data[top.start + 17..top.start + 21]
            != (if single { 50u32 } else { 39u32 }).to_le_bytes()
        || rows
            .iter()
            .find(|r| r.group_index == first.group_index + 1)?
            .row_index
            != top.row_index + 3
        || program.iec_circuit_graph().is_none()
    {
        return None;
    }
    Some(crate::IecTerminalFunctionInsertionSite {
        group_index: first.group_index,
        row_index: top.row_index,
        contact_offset: contact,
        insertion_offset: top.end,
        raw_x: 22,
    })
}

pub(crate) fn remove(program: &LadderProgramData, offset: usize) -> Result<Vec<u8>, XgwxError> {
    let fail = || XgwxError::UnsupportedLadderLayout;
    let blocks = program.iec_function_blocks().ok_or_else(fail)?;
    let b = blocks
        .iter()
        .find(|b| b.record_offset == offset)
        .ok_or_else(fail)?;
    if b.name.value != "TON"
        || b.raw_x != 22
        || b.opcode_family != 0x21
        || b.opcode != 0x51
        || b.pin_count != 2
        || b.instance.is_none()
        || program.data[offset + 11..offset + 15] != [0, 2, 0, 0]
        || blocks
            .iter()
            .filter(|v| v.group_index == b.group_index)
            .count()
            != 1
    {
        return Err(fail());
    }
    let rows = program.iec_row_frames().ok_or_else(fail)?;
    let group = rows
        .iter()
        .filter(|r| r.group_index == b.group_index)
        .collect::<Vec<_>>();
    let [top, input, last] = group.as_slice() else {
        return Err(fail());
    };
    if top.row_index != b.row_index
        || input.row_index != b.row_index + 1
        || last.row_index != b.row_index + 2
        || program.data[top.start - 6..top.start - 2] != [0, 0, 0, 0]
    {
        return Err(fail());
    }
    let records = program.iec_record_frames().ok_or_else(fail)?;
    let prefix = records
        .iter()
        .filter(|r| {
            r.group_index == b.group_index && r.row_index == b.row_index && r.offset < offset
        })
        .collect::<Vec<_>>();
    let single_original = matches(
        program,
        &prefix,
        &[
            (K::Contact(6), 1),
            (K::ShortWire, 4),
            (K::ShortWire, 7),
            (K::ShortWire, 10),
            (K::ShortWire, 13),
            (K::LongWire, 16),
        ],
    );
    let single_refill = matches(program, &prefix, &[(K::Contact(6), 1), (K::LongWire, 4)]);
    let three = prefix.len() == 6
        && series(program, &prefix[..5])
        && matches(program, &prefix[5..], &[(K::LongWire, 16)]);
    if (!single_original && !single_refill && !three)
        || program.data[prefix.last().ok_or_else(fail)?.offset + 15] != 19
        || records
            .iter()
            .filter(|r| r.group_index == b.group_index && r.row_index == b.row_index)
            .count()
            != prefix.len() + 1
    {
        return Err(fail());
    }
    let inputs = records
        .iter()
        .filter(|r| r.group_index == b.group_index && r.row_index == input.row_index)
        .collect::<Vec<_>>();
    let outputs = records
        .iter()
        .filter(|r| r.group_index == b.group_index && r.row_index == last.row_index)
        .collect::<Vec<_>>();
    if !matches(
        program,
        &inputs,
        &[(K::FunctionOperand, 19), (K::LinkReference(0x68), 22)],
    ) || !matches(program, &outputs, &[(K::LinkReference(0x69), 22)])
    {
        return Err(fail());
    }
    let refs = program.iec_function_references().ok_or_else(fail)?;
    if refs
        .iter()
        .filter(|r| r.target_record_offset == offset)
        .count()
        != 2
    {
        return Err(fail());
    }
    let links = program.iec_function_operand_links().ok_or_else(fail)?;
    let owned = links
        .iter()
        .filter(|l| l.target_record_offset == offset)
        .collect::<Vec<_>>();
    if owned.len() != 1 || owned[0].is_output || owned[0].record_offset != inputs[0].offset {
        return Err(fail());
    }
    let count = if three { 5usize } else { 1 };
    let mut replacement = program.data[top.start - 10..top.records_start].to_vec();
    replacement[8..10].copy_from_slice(&1u16.to_le_bytes());
    replacement[27..31].copy_from_slice(&(if three { 39u32 } else { 50u32 }).to_le_bytes());
    replacement[36..38].copy_from_slice(&(b.row_index * 4).to_le_bytes());
    replacement[39] = if three { 13 } else { 1 };
    replacement[43..45].copy_from_slice(&(count as u16).to_le_bytes());
    for r in prefix.iter().take(count) {
        replacement.extend(&program.data[r.offset..r.end]);
    }
    let mut verified = program.clone();
    verified.data.splice(top.start - 10..last.end, replacement);
    verified.decoded_len = verified.data.len();
    if scaffold(&verified, prefix[0].offset).is_none() {
        return Err(fail());
    }
    Ok(verified.data)
}

fn header(row: u16, cache: bool, count: u16) -> Vec<u8> {
    let y = (row * 4).to_le_bytes();
    let mut h = vec![0u8; 35];
    h[..4].copy_from_slice(&u32::from(row).to_le_bytes());
    h[4] = 0xff;
    h[5] = 0x43;
    h[17] = 39;
    h[21] = 94;
    h[25] = 94;
    h[29] = 22;
    for at in [22, 30] {
        h[at..at + 2].copy_from_slice(&y);
    }
    if cache {
        h[26..28].copy_from_slice(&y);
    }
    h[33..35].copy_from_slice(&count.to_le_bytes());
    h
}

pub(crate) fn insert(
    program: &LadderProgramData,
    contact: usize,
    instance: &str,
    preset: &str,
    _literal: bool,
) -> Result<Vec<u8>, XgwxError> {
    let fail = || XgwxError::UnsupportedLadderLayout;
    if instance.is_empty()
        || instance.encode_utf16().count() > 255
        || preset.encode_utf16().count() > 255
    {
        return Err(fail());
    }
    let site = scaffold(program, contact).ok_or_else(fail)?;
    let top = program
        .iec_row_frames()
        .ok_or_else(fail)?
        .into_iter()
        .find(|r| r.group_index == site.group_index)
        .ok_or_else(fail)?;
    let three = top.record_count == 5;
    let mut replacement = program.data[top.start - 10..top.end].to_vec();
    replacement[8..10].copy_from_slice(&3u16.to_le_bytes());
    replacement[27..31].copy_from_slice(&(if three { 50u32 } else { 62u32 }).to_le_bytes());
    replacement[39] = 22;
    replacement[43..45].copy_from_slice(&(top.record_count + 2).to_le_bytes());
    let y = (site.row_index * 4).to_le_bytes();
    replacement.extend([
        0xff,
        2,
        0,
        0,
        0,
        if three { 16 } else { 4 },
        y[0],
        y[1],
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        19,
        y[0],
        y[1],
        0,
    ]);
    let offset = top.start - 10 + replacement.len();
    replacement.extend(crate::iec_connected_timer_write::body_at(
        site.row_index,
        22,
        instance,
        true,
    ));
    replacement.extend(header(site.row_index + 1, three, 2));
    let py = ((site.row_index + 1) * 4).to_le_bytes();
    // These native terminal captures store TIME literals with flags zero.
    replacement.extend([0xff, 0x46, 0, 0, 0, 19, py[0], py[1], 0, 0, 0, 0, 0, 0, 0]);
    let units = preset.encode_utf16().collect::<Vec<_>>();
    replacement.extend([0xff, 0xfe, 0xff, units.len() as u8]);
    for u in units {
        replacement.extend(u.to_le_bytes());
    }
    replacement.extend([1, 0x68, 0, 0, 0, 22, y[0], y[1], 0]);
    replacement.extend(header(site.row_index + 2, three, 1));
    replacement.extend([2, 0x69, 0, 0, 0, 22, y[0], y[1], 0]);
    let mut verified = program.clone();
    verified.data.splice(top.start - 10..top.end, replacement);
    verified.decoded_len = verified.data.len();
    if verified.iec_circuit_graph().is_none() || remove(&verified, offset)? != program.data {
        return Err(fail());
    }
    Ok(verified.data)
}
