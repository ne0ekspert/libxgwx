//! Fresh native FF placement. Captured refill layouts keep their separate writer.
use crate::{IecRecordKind, LadderProgramData, XgwxError};

fn invalid(reason: &'static str) -> XgwxError {
    XgwxError::InvalidLadderEdit { reason }
}
fn text(out: &mut Vec<u8>, value: &str) {
    out.extend([0xff, 0xfe, 0xff, value.encode_utf16().count() as u8]);
    out.extend(value.encode_utf16().flat_map(u16::to_le_bytes));
}
pub(crate) fn body(row: u16, x: u8, instance: &str) -> Vec<u8> {
    let y = (row * 4).to_le_bytes();
    let mut out = vec![
        0, 0x67, 0, 0, 0, x, y[0], y[1], 0, 1, 0, 0, 2, 0, 0, 0x21, 0x21, 0, 0, 0, 1, 0, 0, 0, 2,
        0, x, y[0], y[1], 0, 1, 0, 0, 0, 0, 0, 2, 0,
    ];
    text(&mut out, "");
    out.extend([1, 0, 0x20, 0]);
    text(&mut out, "CLK");
    out.extend([1, 0, 0, 0, 0]);
    text(&mut out, "");
    out.extend([1, 0, 0, 0]);
    text(&mut out, "Q");
    out.extend([3, 0, 0, 0, 0]);
    text(&mut out, "FF");
    text(&mut out, instance);
    let lower = ((row + 1) * 4).to_le_bytes();
    out.extend([x, lower[0], lower[1], 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    out
}
pub(crate) fn insert(
    program: &LadderProgramData,
    row: u16,
    x: u8,
    instance: &str,
) -> Result<Vec<u8>, XgwxError> {
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    if !(4..=91).contains(&x) || !(x - 1).is_multiple_of(3) || row > u16::MAX / 4 - 1 {
        return Err(invalid(
            "FF requires two clear rows and a valid IEC function column",
        ));
    }
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let count = usize::from(u16::from_le_bytes(program.data[6..8].try_into().unwrap()));
    let groups = (0..count)
        .map(|g| {
            rows.iter()
                .filter(|r| r.group_index == g)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if groups.iter().any(Vec::is_empty) {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let affected = groups
        .iter()
        .enumerate()
        .filter(|(_, rs)| rs[0].row_index <= row + 1 && rs.last().unwrap().row_index >= row)
        .map(|(g, _)| g)
        .collect::<Vec<_>>();
    let touched = records
        .iter()
        .filter(|r| affected.contains(&r.group_index))
        .collect::<Vec<_>>();
    let contact = match touched.as_slice() {
        [c] if affected.len() == 1
            && groups[affected[0]].len() == 1
            && c.row_index == row
            && matches!(c.kind, IecRecordKind::Contact(6..=11))
            && program.data[c.offset + 5] == 1 =>
        {
            Some(*c)
        }
        [] => None,
        _ => {
            return Err(invalid(
                "FF placement would overlap an existing network; use clear rows or a single leading contact",
            ));
        }
    };
    let first = affected.first().copied().unwrap_or_else(|| {
        groups
            .iter()
            .position(|rs| rs[0].row_index > row)
            .unwrap_or(count)
    });
    let end = affected.last().map_or(first, |g| g + 1);
    let mut merged = std::collections::BTreeMap::new();
    for rs in groups.iter().take(end).skip(first) {
        let at = rs[0].start - 10;
        if program.data[at + 4..at + 8] != [0; 4] {
            return Err(invalid("FF placement requires a normal empty network"));
        }
        for r in rs {
            merged.insert(
                r.row_index,
                (
                    program.data[r.start..r.records_start].to_vec(),
                    Vec::<Vec<u8>>::new(),
                ),
            );
        }
    }
    for r in row..=row + 1 {
        let mut header = crate::iec_function_write::row_header(r);
        header[17..21].copy_from_slice(&(if r == row { 50u32 } else { 39 }).to_le_bytes());
        header[29] = x;
        let items = if r == row {
            let mut items = vec![];
            if let Some(c) = contact {
                items.push(program.data[c.offset..c.end].to_vec());
            }
            let start = if contact.is_some() { 4 } else { 1 };
            if start <= x - 3 {
                items.push(crate::iec_function_write::wire(row, start, x - 3));
            }
            items.push(body(row, x, instance));
            items
        } else {
            let y = (row * 4).to_le_bytes();
            vec![vec![1, 0x69, 0, 0, 0, x, y[0], y[1], 0]]
        };
        header[33..35].copy_from_slice(&(items.len() as u16).to_le_bytes());
        merged.insert(r, (header, items));
    }
    let new_count = count + 1 - (end - first);
    if new_count > usize::from(u16::MAX) || merged.len() > usize::from(u16::MAX) {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let mut out = program.data[..8].to_vec();
    let total = u16::from_le_bytes(out[4..6].try_into().unwrap());
    out[4..6].copy_from_slice(&total.max(row + 2).to_le_bytes());
    out[6..8].copy_from_slice(&(new_count as u16).to_le_bytes());
    for g in 0..new_count {
        out.extend((g as u32).to_le_bytes());
        if g == first {
            out.extend(0u32.to_le_bytes());
            out.extend((merged.len() as u16).to_le_bytes());
            for (header, items) in merged.values() {
                out.extend(header);
                for item in items {
                    out.extend(item);
                }
            }
        } else {
            let old = if g < first { g } else { g + end - first - 1 };
            let rs = &groups[old];
            out.extend_from_slice(&program.data[rs[0].start - 6..rs.last().unwrap().end]);
        }
    }
    let mut verified = program.clone();
    verified.data = out.clone();
    verified.decoded_len = out.len();
    let blocks = verified
        .iec_function_blocks()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let block = blocks
        .iter()
        .find(|b| {
            b.row_index == row
                && b.raw_x == x
                && b.name.value == "FF"
                && b.instance.as_ref().is_some_and(|i| i.value == instance)
        })
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if verified.iec_record_frames().is_none()
        || verified.iec_circuit_layout().is_none()
        || verified.iec_function_references().is_none_or(|refs| {
            !refs
                .iter()
                .any(|r| r.target_record_offset == block.record_offset)
        })
        || records
            .iter()
            .filter(|r| r.kind == IecRecordKind::FunctionBlock)
            .count()
            + 1
            != blocks.len()
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(out)
}
