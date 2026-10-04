//! Comparison beside a retained gap in its continuing contact spine.
use crate::iec_function_write::{body_with_flags, expression, spec, wire};
use crate::{IecRecordKind as K, LadderProgramData, XgwxError};

fn layout(p: &LadderProgramData, row: u16, filled: bool) -> Option<usize> {
    let rows = p.iec_row_frames()?;
    let top = rows.iter().find(|r| r.row_index == row)?;
    if rows
        .iter()
        .find(|r| r.group_index == top.group_index)?
        .row_index
        != row
    {
        return None;
    }
    let records = p.iec_record_frames()?;
    let blocks = p.iec_function_blocks()?;
    let mut shapes = vec![
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
        vec![(K::BranchEnd, 12)],
        vec![(K::BranchStart, 12)],
    ];
    if filled {
        shapes[0].extend([(K::LongWire, 13), (K::FunctionBlock, 16)]);
        shapes[1].extend([
            (K::FunctionOperand, 13),
            (K::LinkReference(0x68), 16),
            (K::FunctionOperand, 19),
        ]);
        shapes[2].extend([(K::FunctionOperand, 13), (K::LinkReference(0x68), 16)]);
        shapes[3].push((K::LinkReference(0x69), 16));
    }
    for (i, shape) in shapes.iter().enumerate() {
        let y = row.checked_add(i as u16)?;
        if !rows
            .iter()
            .any(|r| r.group_index == top.group_index && r.row_index == y)
        {
            return None;
        }
        let rs = records
            .iter()
            .filter(|r| r.group_index == top.group_index && r.row_index == y)
            .collect::<Vec<_>>();
        let actual = rs
            .iter()
            .map(|r| {
                (
                    r.kind,
                    p.data[r.offset + if r.kind == K::BranchStart { 7 } else { 5 }],
                )
            })
            .collect::<Vec<_>>();
        if actual != *shape {
            return None;
        }
    }
    let own = blocks
        .iter()
        .filter(|b| b.group_index == top.group_index && (row..=row + 3).contains(&b.row_index))
        .collect::<Vec<_>>();
    if own.len() != usize::from(filled) {
        return None;
    }
    if filled {
        let b = own[0];
        if b.row_index != row
            || b.raw_x != 16
            || spec(&b.name.value).is_none_or(|(f, _, n)| f != 0x28 || n != 3)
            || p.data[b.record_offset..b.record_end] != body_with_flags(&b.name.value, row, 16, 0)
            || p.iec_function_operand_links()?
                .iter()
                .filter(|l| l.target_record_offset == b.record_offset)
                .count()
                != 3
            || p.iec_function_references()?
                .iter()
                .filter(|l| l.target_record_offset == b.record_offset)
                .count()
                != 3
        {
            return None;
        }
    }
    let gaps = p.iec_circuit_layout()?.open_branch_endpoints;
    if !gaps
        .iter()
        .any(|v| v.group_index == top.group_index && v.row_index == row + 2 && v.x == 12)
        || !gaps
            .iter()
            .any(|v| v.group_index == top.group_index && v.row_index == row + 3 && v.x == 12)
    {
        return None;
    }
    Some(top.group_index)
}

pub(crate) fn scaffold(p: &LadderProgramData, row: u16, x: u8) -> bool {
    x == 16 && row <= u16::MAX / 4 - 3 && layout(p, row, false).is_some()
}

pub(crate) fn remove(
    p: &LadderProgramData,
    offset: usize,
    name: &str,
) -> Result<Vec<u8>, XgwxError> {
    let b = p
        .iec_function_blocks()
        .ok_or(XgwxError::UnsupportedLadderLayout)?
        .into_iter()
        .find(|b| b.record_offset == offset && b.name.value == name)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let group = layout(p, b.row_index, true).ok_or(XgwxError::UnsupportedLadderLayout)?;
    rewrite(p, group, b.row_index, None)
}

pub(crate) fn insert(
    p: &LadderProgramData,
    row: u16,
    x: u8,
    name: &str,
    args: &[String],
) -> Result<Vec<u8>, XgwxError> {
    if !scaffold(p, row, x)
        || args.len() != 3
        || spec(name).is_none_or(|(f, _, n)| f != 0x28 || n != 3)
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    rewrite(p, layout(p, row, false).unwrap(), row, Some((name, args)))
}

fn rewrite(
    p: &LadderProgramData,
    group: usize,
    row: u16,
    add: Option<(&str, &[String])>,
) -> Result<Vec<u8>, XgwxError> {
    let rows = p
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = p
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let frames = rows
        .iter()
        .filter(|r| r.group_index == group && (row..=row + 3).contains(&r.row_index))
        .collect::<Vec<_>>();
    let mut out = p.data[..frames[0].start].to_vec();
    for (i, frame) in frames.iter().enumerate() {
        let mut kept = records
            .iter()
            .filter(|r| {
                r.group_index == group
                    && r.row_index == frame.row_index
                    && !matches!(
                        r.kind,
                        K::FunctionBlock | K::FunctionOperand | K::LinkReference(_)
                    )
                    && !(i == 0 && r.kind == K::LongWire && p.data[r.offset + 5] == 13)
            })
            .map(|r| p.data[r.offset..r.end].to_vec())
            .collect::<Vec<_>>();
        if let Some((name, args)) = add {
            let y = (row * 4).to_le_bytes();
            match i {
                0 => kept.extend([wire(row, 13, 13), body_with_flags(name, row, 16, 0)]),
                1 => kept.extend([
                    expression(row + 1, 13, &args[0]),
                    vec![1, 0x68, 0, 0, 0, 16, y[0], y[1], 0],
                    expression(row + 1, 19, &args[2]),
                ]),
                2 => kept.extend([
                    expression(row + 2, 13, &args[1]),
                    vec![2, 0x68, 0, 0, 0, 16, y[0], y[1], 0],
                ]),
                3 => kept.push(vec![3, 0x69, 0, 0, 0, 16, y[0], y[1], 0]),
                _ => unreachable!(),
            }
        }
        let mut header = p.data[frame.start..frame.records_start].to_vec();
        header[17..21].copy_from_slice(
            &(if add.is_some() && i == 0 {
                52u32
            } else {
                39u32
            })
            .to_le_bytes(),
        );
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
        for r in kept {
            out.extend(r);
        }
    }
    out.extend_from_slice(&p.data[frames.last().unwrap().end..]);
    let mut result = p.clone();
    result.decoded_len = out.len();
    result.data = out;
    if layout(&result, row, add.is_some()).is_none()
        || !crate::writer::iec_preserves_outside_function_rows(p, &result, group, row..=row + 3)
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(result.data)
}
