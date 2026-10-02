//! Placement of native scalar MOVE, conversion, arithmetic and comparison bodies.
use crate::{IecCircuitAreaKind, IecCircuitEdgeKind, IecRecordKind, LadderProgramData, XgwxError};
use std::collections::BTreeMap;

pub(crate) fn spec(name: &str) -> Option<(u8, u16, u8)> {
    Some(match name {
        "MOVE" => (0x20, 0x76, 2),
        "WORD_TO_UDINT" => (0x20, 0x183, 2),
        "ADD" => (0x20, 0x47, 3),
        "SUB" => (0x20, 0x7f, 3),
        "MUL" => (0x20, 0x48, 3),
        "DIV" => (0x20, 0x63, 3),
        "EQ" => (0x28, 0x4c, 3),
        "GT" => (0x28, 0x44, 3),
        "GE" => (0x28, 0x4f, 3),
        "LT" => (0x28, 0x4d, 3),
        "LE" => (0x28, 0x50, 3),
        _ => return None,
    })
}

fn text(out: &mut Vec<u8>, value: &str) {
    let units = value.encode_utf16().collect::<Vec<_>>();
    out.extend_from_slice(&[0xff, 0xfe, 0xff, units.len() as u8]);
    for unit in units {
        out.extend_from_slice(&unit.to_le_bytes());
    }
}

fn pin(out: &mut Vec<u8>, type_text: &str, flags: u32, name: &str, role: u8) {
    text(out, type_text);
    out.extend_from_slice(&flags.to_le_bytes());
    text(out, name);
    out.extend_from_slice(&[role, 0, 0, 0, 0]);
}

fn body(name: &str, row: u16, x: u8) -> Vec<u8> {
    let (family, opcode, count) = spec(name).unwrap();
    let y = (row * 4).to_le_bytes();
    let mut out = vec![0, 0x67, 0, 0, 0, x, y[0], y[1], 0, 1, 0, 0, 2, 0, 0, family];
    out.extend_from_slice(&opcode.to_le_bytes());
    out.extend_from_slice(&[
        0,
        0,
        count,
        0,
        0,
        0,
        count + 1,
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
    ]);
    pin(&mut out, "", 0x0020_0001, "EN", 1);
    pin(&mut out, "", 1, "ENO", 3);
    text(&mut out, name);
    text(&mut out, "");
    for ordinal in 1..=count {
        let y = ((row + u16::from(ordinal)) * 4).to_le_bytes();
        out.extend_from_slice(&[
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
            if ordinal < count { 2 } else { 0 },
            0,
        ]);
        if ordinal == count {
            continue;
        }
        let array = if name == "MOVE" {
            "ARRAY[0..-1] OF ANY"
        } else {
            ""
        };
        let input_flags = if name == "WORD_TO_UDINT" {
            0x0020_0004
        } else if name == "MOVE" {
            0x022f_ffff
        } else if family == 0x28 {
            0x002f_ffff
        } else {
            0x0020_7fe0
        };
        pin(
            &mut out,
            array,
            input_flags,
            if matches!(name, "MOVE" | "WORD_TO_UDINT") {
                "IN"
            } else if ordinal == 1 {
                "IN1"
            } else {
                "IN2"
            },
            1,
        );
        if ordinal == 1 {
            pin(
                &mut out,
                array,
                if name == "WORD_TO_UDINT" {
                    0x800
                } else if name == "MOVE" {
                    0x020f_ffff
                } else if family == 0x28 {
                    1
                } else {
                    0x7fe0
                },
                "OUT",
                3,
            );
        } else {
            pin(&mut out, "", 0, "", 0);
        }
    }
    out
}

fn wire(row: u16, start: u8, end: u8) -> Vec<u8> {
    let y = (row * 4).to_le_bytes();
    vec![
        0xff, 2, 0, 0, 0, start, y[0], y[1], 0, 0, 0, 0, 0, 0, 0, end, y[0], y[1], 0,
    ]
}

fn expression(row: u16, x: u8, value: &str) -> Vec<u8> {
    let y = (row * 4).to_le_bytes();
    let mut out = vec![0xff, 0x46, 0, 0, 0, x, y[0], y[1], 0, 0, 0, 0, 0, 0, 0];
    text(&mut out, value);
    out
}

fn row_header(row: u16) -> Vec<u8> {
    let y = (row * 4).to_le_bytes();
    let mut out = u32::from(row).to_le_bytes().to_vec();
    out.extend_from_slice(&[
        0xff, 0x43, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x27, 0, 0, 0, 94, y[0], y[1], 0, 94, y[0],
        y[1], 0, 1, y[0], y[1], 0, 0, 0,
    ]);
    out
}

/// Preserve existing groups and records; merge only groups touched by the body.
pub(crate) fn insert(
    program: &LadderProgramData,
    row: u16,
    x: u8,
    name: &str,
    operands: &[String],
) -> Result<Vec<u8>, XgwxError> {
    let invalid = |reason| XgwxError::InvalidLadderEdit { reason };
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let (_, _, count) = spec(name).ok_or_else(|| invalid("unknown IEC function"))?;
    if !(4..=91).contains(&x) || !(x - 1).is_multiple_of(3) || row > u16::MAX / 4 - u16::from(count)
    {
        return Err(invalid(
            "IEC function requires room for its input and output cells",
        ));
    }
    if operands.len() != usize::from(count)
        || operands.iter().any(|value| {
            value.is_empty()
                || value.encode_utf16().count() > 255
                || value.chars().any(char::is_control)
        })
    {
        return Err(invalid("IEC function operand count or text is invalid"));
    }
    let last = row + u16::from(count);
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let graph = program
        .iec_circuit_graph()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let total = u16::from_le_bytes(program.data[4..6].try_into().unwrap());
    if row > total {
        return Err(invalid(
            "IEC function cannot skip past the final editable row",
        ));
    }
    // The final body row occupies only the function cell; data rows need the
    // adjacent expression cells. Horizontal wires may be replaced only at EN.
    let footprint = |r: u16| {
        if r == row || r == last {
            (x, x)
        } else {
            (x - 3, x + 3)
        }
    };
    for area in &graph.occupied_areas {
        for r in area.start_row_index.max(row)..=area.end_row_index.min(last) {
            let (left, right) = footprint(r);
            if area.start_x <= right
                && area.end_x >= left
                && !(r == row && area.kind == IecCircuitAreaKind::HorizontalWire)
            {
                return Err(invalid(
                    "IEC function body or operand would overlap an existing cell",
                ));
            }
        }
    }
    if records.iter().any(|record| {
        record.kind == IecRecordKind::Comment && (row..=last).contains(&record.row_index)
    }) || graph.edges.iter().any(|edge| {
        edge.kind == IecCircuitEdgeKind::VerticalBranch
            && edge.start.row_index <= last
            && edge.end.row_index >= row
            && edge.start.x >= x - 4
            && edge.start.x <= x + 5
    }) {
        return Err(invalid("IEC function cannot overlap a comment or branch"));
    }
    let group_count = usize::from(u16::from_le_bytes(program.data[6..8].try_into().unwrap()));
    let group_rows = (0..group_count)
        .map(|g| {
            rows.iter()
                .filter(|r| r.group_index == g)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if group_rows.iter().any(Vec::is_empty) {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let affected = group_rows
        .iter()
        .enumerate()
        .filter(|(_, rs)| rs[0].row_index <= last && rs.last().unwrap().row_index >= row)
        .map(|(g, _)| g)
        .collect::<Vec<_>>();
    let first = affected.first().copied().unwrap_or_else(|| {
        group_rows
            .iter()
            .position(|rs| rs[0].row_index > row)
            .unwrap_or(group_count)
    });
    let end = affected.last().map_or(first, |g| g + 1);
    let mut merged = BTreeMap::<u16, (Vec<u8>, Vec<Vec<u8>>)>::new();
    for (g, group) in group_rows.iter().enumerate().take(end).skip(first) {
        for r in group {
            merged.insert(
                r.row_index,
                (
                    program.data[r.start..r.records_start].to_vec(),
                    records
                        .iter()
                        .filter(|v| v.group_index == g && v.row_index == r.row_index)
                        .map(|v| program.data[v.offset..v.end].to_vec())
                        .collect(),
                ),
            );
        }
    }
    for r in row..=last {
        merged.entry(r).or_insert_with(|| {
            let mut header = row_header(r);
            header[29] = x;
            (header, vec![])
        });
    }
    if name == "WORD_TO_UDINT" {
        // The conversion's native envelope has a taller EN row and caches
        // the output cell as the end cell of its shared operand row.
        let top_header = &mut merged.get_mut(&row).unwrap().0;
        let height = u32::from_le_bytes(top_header[17..21].try_into().unwrap());
        top_header[17..21].copy_from_slice(&height.max(50).to_le_bytes());
        let operand_header = &mut merged.get_mut(&(row + 1)).unwrap().0;
        operand_header[29] = operand_header[29].max(x + 3);
    }
    let top = &mut merged.get_mut(&row).unwrap().1;
    let mut retained = Vec::new();
    for record in top.drain(..) {
        if record[1] == 2 && record[5] <= x && record[15] >= x {
            if record[5] < x {
                let mut fragment = wire(row, record[5], x - 3);
                fragment[9..15].copy_from_slice(&record[9..15]);
                retained.push(fragment);
            }
            if record[15] > x {
                let mut fragment = wire(row, x + 3, record[15]);
                fragment[9..15].copy_from_slice(&record[9..15]);
                retained.push(fragment);
            }
        } else if record[1] == 1 && record[5] == x {
            // One-cell wire replaced by EN.
        } else {
            retained.push(record);
        }
    }
    if retained.is_empty() && x > 1 {
        retained.push(wire(row, 1, x - 3));
    }
    retained.push(body(name, row, x));
    *top = retained;
    for (index, operand) in operands.iter().enumerate() {
        let output = index + 1 == operands.len();
        let expr_row = row + if output { 1 } else { index as u16 + 1 };
        merged.get_mut(&expr_row).unwrap().1.push(expression(
            expr_row,
            if output { x + 3 } else { x - 3 },
            operand,
        ));
        let reference_row = row
            + if output {
                u16::from(count)
            } else {
                index as u16 + 1
            };
        let y = (row * 4).to_le_bytes();
        merged.get_mut(&reference_row).unwrap().1.push(vec![
            index as u8 + 1,
            if output { 0x69 } else { 0x68 },
            0,
            0,
            0,
            x,
            y[0],
            y[1],
            0,
        ]);
    }
    let new_count = group_count + 1 - (end - first);
    if new_count > usize::from(u16::MAX)
        || merged.len() > usize::from(u16::MAX)
        || merged
            .values()
            .any(|(_, records)| records.len() > usize::from(u16::MAX))
    {
        return Err(invalid(
            "IEC function placement exceeds the native row or group count",
        ));
    }
    let mut out = program.data[..8].to_vec();
    out[4..6].copy_from_slice(&total.max(last + 1).to_le_bytes());
    out[6..8].copy_from_slice(&(new_count as u16).to_le_bytes());
    for g in 0..new_count {
        if g == first {
            out.extend_from_slice(&(g as u32).to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&(merged.len() as u16).to_le_bytes());
            for (_, (mut header, records)) in merged.clone() {
                let n = header.len();
                header[n - 2..].copy_from_slice(&(records.len() as u16).to_le_bytes());
                out.extend(header);
                for record in records {
                    out.extend(record);
                }
            }
        } else {
            let old_g = if g < first { g } else { g + end - first - 1 };
            let rs = &group_rows[old_g];
            let start = rs[0].start - 10;
            let tail = rs.last().unwrap().end;
            out.extend_from_slice(&(g as u32).to_le_bytes());
            out.extend_from_slice(&program.data[start + 4..tail]);
        }
    }
    let mut verified = program.clone();
    verified.decoded_len = out.len();
    verified.data = out.clone();
    if verified.iec_circuit_graph().is_none() {
        return Err(invalid(
            "IEC function placement did not preserve the circuit graph",
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::XgwxDocument;

    fn empty_program() -> LadderProgramData {
        let doc = XgwxDocument::parse(include_bytes!("../fixtures/elements.xgwx")).unwrap();
        let mut program = doc.ladder_programs().remove(0).unwrap();
        program.project_type = Some(2);
        program.data = vec![0, 0, 0, 0, 1, 0, 1, 0];
        program
            .data
            .extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 1, 0]);
        program.data.extend(row_header(0));
        program.decoded_len = program.data.len();
        program
    }

    #[test]
    fn bodies_match_captured_native_pin_descriptors() {
        for (name, native) in [
            (
                "MOVE",
                include_bytes!("../fixtures/function-bodies/iec_move.bin").as_slice(),
            ),
            (
                "ADD",
                include_bytes!("../fixtures/function-bodies/iec_add.bin").as_slice(),
            ),
            (
                "EQ",
                include_bytes!("../fixtures/function-bodies/iec_eq.bin").as_slice(),
            ),
            (
                "WORD_TO_UDINT",
                include_bytes!("../fixtures/function-bodies/iec_word_to_udint.bin").as_slice(),
            ),
        ] {
            let row = u16::from_le_bytes([native[6], native[7]]) / 4;
            assert_eq!(body(name, row, native[5]), native, "{name}");
        }
    }

    #[test]
    fn conversion_group_matches_native_envelope_and_reference_order() {
        let program = empty_program();
        let bytes = insert(
            &program,
            1,
            4,
            "WORD_TO_UDINT",
            &["%MW301".into(), "변환".into()],
        )
        .unwrap();
        let actual = &bytes[program.data.len()..];
        let native = include_bytes!("iec_word_to_udint_group.bin");
        let differences = (0..actual.len().max(native.len()))
            .filter(|&i| actual.get(i) != native.get(i))
            .collect::<Vec<_>>();
        assert!(
            differences.is_empty(),
            "native envelope differences: {differences:?}"
        );
    }

    #[test]
    fn places_every_scalar_function_and_rejects_overlap() {
        for name in [
            "MOVE",
            "WORD_TO_UDINT",
            "ADD",
            "SUB",
            "MUL",
            "DIV",
            "EQ",
            "GT",
            "GE",
            "LT",
            "LE",
        ] {
            let mut program = empty_program();
            let count = spec(name).unwrap().2;
            let operands = (0..count)
                .map(|i| {
                    if i == count - 1 {
                        if spec(name).unwrap().0 == 0x28 {
                            "%MX100"
                        } else {
                            "%MW100"
                        }
                    } else {
                        "1"
                    }
                    .to_string()
                })
                .collect::<Vec<_>>();
            program.data = insert(&program, 0, 10, name, &operands).unwrap();
            let blocks = program.iec_function_blocks().unwrap();
            assert_eq!(blocks.len(), 1);
            assert_eq!(blocks[0].name.value, name);
            assert_eq!(blocks[0].raw_x, 10);
            assert_eq!(
                program.iec_function_operand_links().unwrap().len(),
                usize::from(count)
            );
            assert!(insert(&program, 0, 10, name, &operands).is_err());
            assert!(insert(&program, 1, 7, name, &operands).is_err());
            // Place beside the existing body; merge their group without moving it.
            let data = insert(&program, 0, 22, name, &operands).unwrap();
            program.data = data;
            assert_eq!(program.iec_function_blocks().unwrap().len(), 2);
            assert_eq!(
                program.iec_function_operand_links().unwrap().len(),
                2 * usize::from(count)
            );
        }
    }
}
