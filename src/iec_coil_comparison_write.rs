//! Native two-input comparison with its result wired to a retained terminal coil.
use crate::iec_function_write::{body, expression, row_header, spec, wire};
use crate::{IecRecordKind as K, LadderProgramData, XgwxError};

fn result_records(program: &LadderProgramData, group: usize, row: u16) -> Option<Vec<Vec<u8>>> {
    let records = program.iec_record_frames()?;
    let kept = records
        .iter()
        .filter(|r| {
            r.group_index == group
                && r.row_index == row
                && matches!(r.kind, K::LongWire | K::Coil(14))
        })
        .collect::<Vec<_>>();
    let [feed, coil] = kept.as_slice() else {
        return None;
    };
    if program.data[feed.offset..feed.end] != wire(row, 10, 91)
        || program.data[coil.offset + 5] != 94
        || program.data[coil.offset + 9..coil.offset + 15] != [1, 0, 0x20, 0, 0, 0]
    {
        return None;
    }
    Some(
        kept.into_iter()
            .map(|r| program.data[r.offset..r.end].to_vec())
            .collect(),
    )
}

pub(crate) fn scaffold(program: &LadderProgramData, row: u16, x: u8) -> Option<usize> {
    if x != 7 || row > u16::MAX / 4 - 3 {
        return None;
    }
    let rows = program.iec_row_frames()?;
    let result = rows.iter().find(|r| r.row_index == row + 1)?;
    if rows
        .iter()
        .any(|r| r.group_index == result.group_index && r.row_index != row + 1)
        || rows
            .iter()
            .any(|r| (row..=row + 3).contains(&r.row_index) && r.row_index != row + 1)
        || program.data[result.start - 6..result.start - 2] != [0; 4]
        || program.data[result.start + 17..result.start + 21] != 50u32.to_le_bytes()
        || program.data[result.start + 29] != 7
        || result.record_count != 2
    {
        return None;
    }
    result_records(program, result.group_index, row + 1)?;
    Some(result.group_index)
}

pub(crate) fn sites(
    program: &LadderProgramData,
) -> Option<Vec<crate::IecWiredComparisonInsertionSite>> {
    Some(
        program
            .iec_row_frames()?
            .iter()
            .filter_map(|r| {
                let row = r.row_index.checked_sub(1)?;
                Some(crate::IecWiredComparisonInsertionSite {
                    group_index: scaffold(program, row, 7)?,
                    row_index: row,
                    raw_x: 7,
                })
            })
            .collect(),
    )
}

pub(crate) fn remove(program: &LadderProgramData, offset: usize) -> Result<Vec<u8>, XgwxError> {
    let unsupported = || XgwxError::UnsupportedLadderLayout;
    let blocks = program.iec_function_blocks().ok_or_else(unsupported)?;
    let b = blocks
        .iter()
        .find(|b| b.record_offset == offset)
        .ok_or_else(unsupported)?;
    let row = b.row_index;
    if b.raw_x != 7
        || row > u16::MAX / 4 - 3
        || spec(&b.name.value).is_none_or(|(f, op, n)| {
            f != 0x28 || b.opcode_family != f || b.opcode != op || n != 3 || b.pin_count != n
        })
    {
        return Err(unsupported());
    }
    let mut canonical = body(&b.name.value, row, 7);
    canonical[12] = 0;
    if program.data[offset..b.record_end] != canonical {
        return Err(unsupported());
    }
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    let group = rows
        .iter()
        .filter(|r| r.group_index == b.group_index)
        .collect::<Vec<_>>();
    if group.len() != 4
        || group
            .iter()
            .enumerate()
            .any(|(i, r)| r.row_index != row + i as u16)
        || program.data[group[0].start - 6..group[0].start - 2] != [0; 4]
        || blocks
            .iter()
            .filter(|v| v.group_index == b.group_index)
            .count()
            != 1
    {
        return Err(unsupported());
    }
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let shape = records
        .iter()
        .filter(|r| r.group_index == b.group_index)
        .map(|r| (r.row_index - row, r.kind, program.data[r.offset + 5]))
        .collect::<Vec<_>>();
    let mut expected = vec![
        (0, K::LongWire, 1),
        (0, K::FunctionBlock, 7),
        (1, K::FunctionOperand, 4),
        (1, K::LinkReference(0x68), 7),
        (1, K::LongWire, 10),
        (1, K::Coil(14), 94),
        (2, K::FunctionOperand, 4),
        (2, K::LinkReference(0x68), 7),
        (3, K::LinkReference(0x69), 7),
    ];
    if shape.first() == Some(&(0, K::ShortWire, 1)) {
        expected.splice(0..1, [(0, K::ShortWire, 1), (0, K::LongWire, 4)]);
    }
    if shape != expected {
        return Err(unsupported());
    }
    for r in records
        .iter()
        .filter(|r| r.group_index == b.group_index && r.row_index == row && r.offset != offset)
    {
        let bytes = &program.data[r.offset..r.end];
        if r.kind == K::ShortWire {
            if bytes[9..15] != [0; 6] {
                return Err(unsupported());
            }
        } else if bytes != wire(row, bytes[5], 4) {
            return Err(unsupported());
        }
    }
    let links = program
        .iec_function_operand_links()
        .ok_or_else(unsupported)?;
    let refs = program.iec_function_references().ok_or_else(unsupported)?;
    if links
        .iter()
        .filter(|l| l.target_record_offset == offset && !l.is_output)
        .count()
        != 2
        || links
            .iter()
            .filter(|l| l.target_record_offset == offset)
            .count()
            != 2
        || refs
            .iter()
            .filter(|r| r.target_record_offset == offset)
            .count()
            != 3
    {
        return Err(unsupported());
    }
    let result = result_records(program, b.group_index, row + 1).ok_or_else(unsupported)?;
    let first = group[0].start - 10;
    let end = group[3].end;
    let mut replacement = program.data[first..first + 10].to_vec();
    replacement[8..10].copy_from_slice(&1u16.to_le_bytes());
    let mut header = program.data[group[1].start..group[1].records_start].to_vec();
    header[17..21].copy_from_slice(&50u32.to_le_bytes());
    header[33..35].copy_from_slice(&2u16.to_le_bytes());
    replacement.extend(header);
    for record in result {
        replacement.extend(record);
    }
    let mut verified = program.clone();
    verified.data.splice(first..end, replacement);
    verified.decoded_len = verified.data.len();
    if scaffold(&verified, row, 7).is_none() || verified.iec_circuit_graph().is_none() {
        return Err(unsupported());
    }
    Ok(verified.data)
}

pub(crate) fn insert(
    program: &LadderProgramData,
    row: u16,
    x: u8,
    name: &str,
    operands: &[String],
) -> Result<Vec<u8>, XgwxError> {
    let unsupported = || XgwxError::UnsupportedLadderLayout;
    let group = scaffold(program, row, x).ok_or_else(unsupported)?;
    if spec(name).is_none_or(|(f, _, n)| f != 0x28 || n != 3) || operands.len() != 2 {
        return Err(unsupported());
    }
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    let result = rows
        .iter()
        .find(|r| r.group_index == group)
        .ok_or_else(unsupported)?;
    let first = result.start - 10;
    let mut replacement = program.data[first..first + 10].to_vec();
    replacement[8..10].copy_from_slice(&4u16.to_le_bytes());
    let cache = u16::from_le_bytes(
        program.data[result.start + 26..result.start + 28]
            .try_into()
            .unwrap(),
    );
    let last_cache = cache.checked_add(4).ok_or_else(unsupported)?;
    let mut block = body(name, row, 7);
    block[12] = 0;
    let mut parts = vec![vec![wire(row, 1, 4), block], vec![], vec![], vec![]];
    let y = (row * 4).to_le_bytes();
    for i in 1..=3 {
        if i < 3 {
            parts[i].push(expression(row + i as u16, 4, &operands[i - 1]));
        }
        parts[i].push(vec![
            i as u8,
            if i == 3 { 0x69 } else { 0x68 },
            0,
            0,
            0,
            7,
            y[0],
            y[1],
            0,
        ]);
    }
    parts[1].extend(result_records(program, group, row + 1).ok_or_else(unsupported)?);
    for (i, records) in parts.into_iter().enumerate() {
        let mut header = if i == 1 {
            program.data[result.start..result.records_start].to_vec()
        } else {
            row_header(row + i as u16)
        };
        header[26..28].copy_from_slice(&if i == 3 { last_cache } else { cache }.to_le_bytes());
        header[29] = 7;
        header[33..35].copy_from_slice(&(records.len() as u16).to_le_bytes());
        replacement.extend(header);
        for record in records {
            replacement.extend(record);
        }
    }
    let mut verified = program.clone();
    verified.data.splice(first..result.end, replacement);
    verified.decoded_len = verified.data.len();
    let b = verified
        .iec_function_blocks()
        .ok_or_else(unsupported)?
        .into_iter()
        .find(|b| b.row_index == row && b.raw_x == 7)
        .ok_or_else(unsupported)?;
    if verified.iec_circuit_graph().is_none() || remove(&verified, b.record_offset)? != program.data
    {
        return Err(unsupported());
    }
    Ok(verified.data)
}
