//! Fresh IEC function blocks captured from native XG5000.
use crate::{IecRecordKind as K, LadderProgramData, XgwxError};

fn invalid(reason: &'static str) -> XgwxError {
    XgwxError::InvalidLadderEdit { reason }
}

// Each prototype contains a single normal network at L0/x4, including the
// native hidden references and row headers. No project declarations are stored.
fn template(name: &str) -> Option<&'static [u8]> {
    Some(match name {
        "AND" => include_bytes!("iec_block_templates/and.bin"),
        "CTD_DINT" => include_bytes!("iec_block_templates/ctd_dint.bin"),
        "CTU_DINT" => include_bytes!("iec_block_templates/ctu_dint.bin"),
        "CTUD_DINT" => include_bytes!("iec_block_templates/ctud_dint.bin"),
        "F_TRIG" => include_bytes!("iec_block_templates/f_trig.bin"),
        "MOD" => include_bytes!("iec_block_templates/mod.bin"),
        "NE" => include_bytes!("iec_block_templates/ne.bin"),
        "NOT" => include_bytes!("iec_block_templates/not.bin"),
        "OR" => include_bytes!("iec_block_templates/or.bin"),
        "R_TRIG" => include_bytes!("iec_block_templates/r_trig.bin"),
        "RS" => include_bytes!("iec_block_templates/rs.bin"),
        "SR" => include_bytes!("iec_block_templates/sr.bin"),
        "TOF" => include_bytes!("iec_block_templates/tof.bin"),
        "TON" => include_bytes!("iec_block_templates/ton.bin"),
        "TP" => include_bytes!("iec_block_templates/tp.bin"),
        "WORD_TO_UDINT" => include_bytes!("iec_block_templates/word_to_udint.bin"),
        "XOR" => include_bytes!("iec_block_templates/xor.bin"),
        _ => return None,
    })
}

pub(crate) fn supports(name: &str) -> bool {
    template(name).is_some()
}

fn prototype(program: &LadderProgramData, name: &str) -> Result<LadderProgramData, XgwxError> {
    let mut p = program.clone();
    p.data = template(name)
        .ok_or_else(|| invalid("unknown IEC block"))?
        .to_vec();
    p.decoded_len = p.data.len();
    Ok(p)
}

// Translate all native coordinates before changing variable-length strings.
pub(crate) fn body(
    program: &LadderProgramData,
    name: &str,
    row: u16,
    x: u8,
    instance: &str,
) -> Result<Vec<u8>, XgwxError> {
    let p = prototype(program, name)?;
    let b = p
        .iec_function_blocks()
        .ok_or(XgwxError::UnsupportedLadderLayout)?
        .remove(0);
    let mut out = p.data[b.record_offset..b.record_end].to_vec();
    let y = (row * 4).to_le_bytes();
    out[5] = x;
    out[6..8].copy_from_slice(&y);
    out[26] = x;
    out[27..29].copy_from_slice(&y);
    let mut cursor = 38;
    for descriptor in 0..6 {
        cursor += 4 + usize::from(out[cursor + 3]) * 2;
        if descriptor == 0 || descriptor == 2 {
            cursor += 4;
        }
        if descriptor == 1 || descriptor == 3 {
            cursor += 5;
        }
    }
    for ordinal in 1..=b.pin_count {
        out[cursor] = x;
        out[cursor + 1..cursor + 3]
            .copy_from_slice(&((row + u16::from(ordinal)) * 4).to_le_bytes());
        cursor += 12;
        if ordinal == b.pin_count {
            break;
        }
        for _ in 0..2 {
            cursor += 4 + usize::from(out[cursor + 3]) * 2 + 4;
            cursor += 4 + usize::from(out[cursor + 3]) * 2 + 5;
        }
    }
    if let Some(i) = b.instance {
        let at = i.offset - b.record_offset;
        let mut text = vec![0xff, 0xfe, 0xff, instance.encode_utf16().count() as u8];
        text.extend(instance.encode_utf16().flat_map(u16::to_le_bytes));
        out.splice(at..i.end_offset - b.record_offset, text);
    }
    Ok(out)
}

pub(crate) fn pins(
    program: &LadderProgramData,
    name: &str,
) -> Result<crate::IecFunctionBlock, XgwxError> {
    prototype(program, name)?
        .iec_function_blocks()
        .ok_or(XgwxError::UnsupportedLadderLayout)?
        .into_iter()
        .next()
        .ok_or(XgwxError::UnsupportedLadderLayout)
}

pub(crate) fn insert(
    program: &LadderProgramData,
    row: u16,
    x: u8,
    name: &str,
    instance: &str,
    operands: &[String],
) -> Result<Vec<u8>, XgwxError> {
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let proto = prototype(program, name)?;
    let b = pins(program, name)?;
    let last = row
        .checked_add(u16::from(b.pin_count))
        .filter(|r| *r <= u16::MAX / 4)
        .ok_or_else(|| invalid("IEC block exceeds the row coordinate range"))?;
    if !(4..=91).contains(&x) || !(x - 1).is_multiple_of(3) {
        return Err(invalid("IEC block requires a valid function column"));
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
        .filter(|(_, rs)| rs[0].row_index <= last && rs.last().unwrap().row_index >= row)
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
            && matches!(c.kind, K::Contact(6..=11))
            && program.data[c.offset + 5] == 1 =>
        {
            Some(*c)
        }
        [] => None,
        _ => {
            return Err(invalid(
                "IEC block requires clear rows or a single leading contact",
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
        if program.data[rs[0].start - 6..rs[0].start - 2] != [0; 4] {
            return Err(invalid("IEC block requires a normal network"));
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
    let proto_rows = proto
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let proto_records = proto
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    for pr in &proto_rows {
        let r = row + pr.row_index;
        let mut header = proto.data[pr.start..pr.records_start].to_vec();
        header[..4].copy_from_slice(&u32::from(r).to_le_bytes());
        for at in [22, 26, 30] {
            let relative = u16::from_le_bytes(header[at..at + 2].try_into().unwrap());
            header[at..at + 2].copy_from_slice(&(relative + row * 4).to_le_bytes());
        }
        header[29] = x;
        let mut items = vec![];
        if pr.row_index == 0 {
            if let Some(c) = contact {
                items.push(program.data[c.offset..c.end].to_vec());
            }
            let start = if contact.is_some() { 4 } else { 1 };
            if start <= x - 3 {
                items.push(crate::iec_function_write::wire(row, start, x - 3));
            }
            items.push(body(program, name, row, x, instance)?);
        } else {
            for rec in proto_records
                .iter()
                .filter(|rec| rec.row_index == pr.row_index)
            {
                if matches!(rec.kind, K::LinkReference(_)) {
                    let mut link = proto.data[rec.offset..rec.end].to_vec();
                    link[5] = x;
                    link[6..8].copy_from_slice(&(row * 4).to_le_bytes());
                    items.push(link);
                }
            }
        }
        merged.insert(r, (header, items));
    }
    let mut ordered = b
        .pins
        .iter()
        .filter(|p| p.reference_ordinal.is_some())
        .collect::<Vec<_>>();
    ordered.sort_by_key(|p| p.reference_ordinal);
    for (pin, value) in ordered.iter().zip(operands) {
        if value.is_empty() {
            continue;
        }
        let r = row + pin.row_index;
        let expr_x = if pin.direction == crate::IecFunctionPinDirection::Input {
            x - 3
        } else {
            x + 3
        };
        let (header, items) = merged
            .get_mut(&r)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        header[17..21].copy_from_slice(&56u32.to_le_bytes());
        header[29] = header[29].max(expr_x);
        items.push(crate::iec_function_write::expression(r, expr_x, value));
        items.sort_by_key(|rec| rec[5]);
    }
    for (header, items) in merged.values_mut() {
        header[33..35].copy_from_slice(&(items.len() as u16).to_le_bytes());
    }
    let new_count = count + 1 - (end - first);
    if new_count > usize::from(u16::MAX) || merged.len() > usize::from(u16::MAX) {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let mut out = program.data[..8].to_vec();
    let total = u16::from_le_bytes(out[4..6].try_into().unwrap());
    out[4..6].copy_from_slice(&total.max(last + 1).to_le_bytes());
    out[6..8].copy_from_slice(&(new_count as u16).to_le_bytes());
    for g in 0..new_count {
        out.extend((g as u32).to_le_bytes());
        if g == first {
            out.extend(0u32.to_le_bytes());
            out.extend((merged.len() as u16).to_le_bytes());
            for (h, items) in merged.values() {
                out.extend(h);
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
    if verified.iec_circuit_layout().is_none() || verified.iec_function_operand_links().is_none() {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_fresh_empty_body_reproduces_its_native_network() {
        let doc = crate::create_project("XGI-CPUE", "LD").unwrap();
        let seed = doc.program().ladder_programs().remove(0).unwrap();
        for name in [
            "R_TRIG",
            "F_TRIG",
            "TON",
            "TOF",
            "TP",
            "RS",
            "SR",
            "CTU_DINT",
            "CTD_DINT",
            "CTUD_DINT",
            "NE",
            "MOD",
            "AND",
            "OR",
            "XOR",
            "NOT",
            "WORD_TO_UDINT",
        ] {
            let p = prototype(&seed, name).unwrap();
            let b = pins(&seed, name).unwrap();
            let out = insert(
                &seed,
                0,
                4,
                name,
                b.instance.as_ref().map_or("", |i| i.value.as_str()),
                &[],
            )
            .unwrap();
            assert_eq!(out, p.data, "native prototype {name}");
        }
    }
}
