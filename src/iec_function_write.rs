//! Placement of native scalar MOVE, conversion, arithmetic and comparison bodies.
use crate::{IecCircuitAreaKind, IecCircuitEdgeKind, IecRecordKind, LadderProgramData, XgwxError};
use std::collections::BTreeMap;

pub(crate) fn spec(name: &str) -> Option<(u8, u16, u8)> {
    Some(match name {
        "MOVE" => (0x20, 0x76, 2),
        "WORD_TO_UDINT" => (0x20, 0x183, 2),
        "INT_TO_UDINT" => (0x20, 0x1c3, 2),
        "UDINT_TO_TIME" => (0x20, 0x218, 2),
        "TIME_TO_UDINT" => (0x20, 0x245, 2),
        "UDINT_TO_INT" => (0x20, 0x210, 2),
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

pub(crate) fn conversion_types(name: &str) -> Option<(u32, u32)> {
    Some(match name {
        "WORD_TO_UDINT" => (4, 0x800),
        "INT_TO_UDINT" => (0x40, 0x800),
        "UDINT_TO_TIME" => (0x800, 0x8000),
        "TIME_TO_UDINT" => (0x8000, 0x800),
        "UDINT_TO_INT" => (0x800, 0x40),
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

pub(crate) fn body(name: &str, row: u16, x: u8) -> Vec<u8> {
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
        let input_flags = if let Some((source, _)) = conversion_types(name) {
            0x0020_0000 | source
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
            if name == "MOVE" || conversion_types(name).is_some() {
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
                if let Some((_, destination)) = conversion_types(name) {
                    destination
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

/// Canonical TON records for the captured contact/short-feed terminal layout.
pub(crate) fn terminal_timer_tail(
    row: u16,
    instance: &str,
    preset: &str,
    elapsed: &str,
) -> Vec<u8> {
    let y = (row * 4).to_le_bytes();
    let mut out = vec![0xff, 1, 0, 0, 0, 4, y[0], y[1], 0, 0, 0, 0, 0, 0, 0];
    out.extend_from_slice(&[
        0, 0x67, 0, 0, 0, 7, y[0], y[1], 0, 1, 0, 0, 2, 0, 0, 0x21, 0x51, 0, 0, 0, 2, 0, 0, 0, 3,
        0, 7, y[0], y[1], 0, 1, 0, 2, 0, 0, 0, 2, 0,
    ]);
    pin(&mut out, "", 0x0020_0001, "IN", 1);
    pin(&mut out, "", 1, "Q", 3);
    text(&mut out, "TON");
    text(&mut out, instance);
    for ordinal in 1..=2 {
        let pin_y = ((row + ordinal) * 4).to_le_bytes();
        out.extend_from_slice(&[
            7,
            pin_y[0],
            pin_y[1],
            0,
            0,
            0,
            2,
            0,
            0,
            0,
            if ordinal == 1 { 2 } else { 0 },
            0,
        ]);
        if ordinal == 1 {
            pin(&mut out, "", 0x0020_8000, "PT", 1);
            pin(&mut out, "", 0x8000, "ET", 3);
        }
    }
    let mut input = row_header(row + 1);
    input[17..21].copy_from_slice(&56u32.to_le_bytes());
    input[29] = 10;
    input[33..35].copy_from_slice(&3u16.to_le_bytes());
    out.extend(input);
    out.extend(expression(row + 1, 4, preset));
    out.extend_from_slice(&[1, 0x68, 0, 0, 0, 7, y[0], y[1], 0]);
    out.extend(expression(row + 1, 10, elapsed));
    let mut last = row_header(row + 2);
    last[29] = 7;
    last[33..35].copy_from_slice(&1u16.to_le_bytes());
    out.extend(last);
    out.extend_from_slice(&[2, 0x69, 0, 0, 0, 7, y[0], y[1], 0]);
    out
}

pub(crate) fn wire(row: u16, start: u8, end: u8) -> Vec<u8> {
    let y = (row * 4).to_le_bytes();
    vec![
        0xff, 2, 0, 0, 0, start, y[0], y[1], 0, 0, 0, 0, 0, 0, 0, end, y[0], y[1], 0,
    ]
}

pub(crate) fn expression(row: u16, x: u8, value: &str) -> Vec<u8> {
    let y = (row * 4).to_le_bytes();
    let mut out = vec![0xff, 0x46, 0, 0, 0, x, y[0], y[1], 0, 0, 0, 0, 0, 0, 0];
    text(&mut out, value);
    out
}

pub(crate) fn row_header(row: u16) -> Vec<u8> {
    let y = (row * 4).to_le_bytes();
    let mut out = u32::from(row).to_le_bytes().to_vec();
    out.extend_from_slice(&[
        0xff, 0x43, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x27, 0, 0, 0, 94, y[0], y[1], 0, 94, y[0],
        y[1], 0, 1, y[0], y[1], 0, 0, 0,
    ]);
    out
}

/// Captured normal-mode R_TRIG feed, before its optional terminal scalar.
fn trigger_prefix(program: &LadderProgramData, prefix: &[&crate::IecRecordFrame]) -> bool {
    let [contact, short, long, trigger] = prefix else {
        return false;
    };
    contact.kind == IecRecordKind::Contact(6)
        && short.kind == IecRecordKind::ShortWire
        && long.kind == IecRecordKind::LongWire
        && trigger.kind == IecRecordKind::FunctionBlock
        && [contact, short, long].iter().zip([1, 4, 7]).all(|(r, x)| {
            program.data[r.offset + 5] == x && program.data[r.offset + 11..r.offset + 15] == [0; 4]
        })
        && program.data[trigger.offset + 5] == 10
        && program.data[trigger.offset + 11] == 0
        && matches!(program.data[trigger.offset + 12], 0 | 2)
        && program.data[trigger.offset + 13..trigger.offset + 15] == [0; 2]
        && program.data[long.offset + 15] == 7
        && program.data[short.offset + 9..short.offset + 15] == [0; 6]
        && program.data[long.offset + 9..long.offset + 15] == [0; 6]
        && program.iec_function_blocks().is_some_and(|blocks| {
            blocks.iter().any(|b| {
                b.record_offset == trigger.offset
                    && b.name.value == "R_TRIG"
                    && b.opcode_family == 0x21
                    && b.opcode == 0x1f
                    && b.pin_count == 1
                    && b.instance.is_some()
            })
        })
}

/// Retained MOVE and its open result feed after native comparison deletion.
pub(crate) fn wired_comparison_scaffold(
    program: &LadderProgramData,
    row: u16,
    x: u8,
) -> Option<usize> {
    if let Some(group) = crate::iec_coil_comparison_write::scaffold(program, row, x) {
        return Some(group);
    }
    if let Some(group) = crate::iec_paired_comparison_write::scaffold(program, row, x) {
        return Some(group);
    }
    if x != 10 || row == 0 || row > u16::MAX / 4 - 3 {
        return None;
    }
    let rows = program.iec_row_frames()?;
    if rows.iter().any(|r| r.row_index == row) {
        return None;
    }
    let first = rows.iter().find(|r| r.row_index == row + 1)?;
    let group = rows
        .iter()
        .filter(|r| r.group_index == first.group_index)
        .collect::<Vec<_>>();
    if group.len() != 3
        || group
            .iter()
            .enumerate()
            .any(|(i, r)| r.row_index != row + 1 + i as u16)
        || program.data[first.start - 6..first.start - 2] != [0; 4]
    {
        return None;
    }
    let blocks = program.iec_function_blocks()?;
    let mut bodies = blocks.iter().filter(|b| b.group_index == first.group_index);
    let mov = bodies.next()?;
    if bodies.next().is_some()
        || mov.raw_x != 19
        || mov.row_index != row + 1
        || mov.name.value != "MOVE"
        || mov.opcode_family != 0x20
        || mov.opcode != 0x76
        || mov.pin_count != 2
        || mov.instance.is_some()
        || program.data[mov.record_offset + 11..mov.record_offset + 15] != [0, 2, 0, 0]
    {
        return None;
    }
    let records = program.iec_record_frames()?;
    for (i, expected) in [
        vec![
            (IecRecordKind::LongWire, 13),
            (IecRecordKind::FunctionBlock, 19),
        ],
        vec![
            (IecRecordKind::FunctionOperand, 16),
            (IecRecordKind::LinkReference(0x68), 19),
            (IecRecordKind::FunctionOperand, 22),
        ],
        vec![(IecRecordKind::LinkReference(0x69), 19)],
    ]
    .iter()
    .enumerate()
    {
        let parts = records
            .iter()
            .filter(|r| r.group_index == first.group_index && r.row_index == row + 1 + i as u16)
            .collect::<Vec<_>>();
        if parts.len() != expected.len()
            || parts
                .iter()
                .zip(expected)
                .any(|(r, (kind, x))| r.kind != *kind || program.data[r.offset + 5] != *x)
        {
            return None;
        }
        if i == 0
            && (program.data[parts[0].offset + 15] != 16
                || program.data[parts[0].offset + 9..parts[0].offset + 15] != [0; 6])
        {
            return None;
        }
    }
    let refs = program.iec_function_references()?;
    let links = program.iec_function_operand_links()?;
    if refs
        .iter()
        .filter(|r| r.target_record_offset == mov.record_offset)
        .count()
        != 2
        || links
            .iter()
            .filter(|r| r.target_record_offset == mov.record_offset)
            .count()
            != 2
    {
        return None;
    }
    Some(first.group_index)
}

fn wired_comparison_prefix(program: &LadderProgramData, prefix: &[&crate::IecRecordFrame]) -> bool {
    let (feed, body) = match prefix {
        [wire, body]
            if wire.kind == IecRecordKind::LongWire
                && program.data[wire.offset + 5] == 1
                && program.data[wire.offset + 15] == 7 =>
        {
            (&prefix[..1], body)
        }
        [a, b, c, body]
            if [a, b, c]
                .iter()
                .zip([
                    (IecRecordKind::ShortWire, 1),
                    (IecRecordKind::ShortWire, 4),
                    (IecRecordKind::LongWire, 7),
                ])
                .all(|(r, (kind, x))| r.kind == kind && program.data[r.offset + 5] == x)
                && program.data[c.offset + 15] == 7 =>
        {
            (&prefix[..3], body)
        }
        _ => return false,
    };
    body.kind == IecRecordKind::FunctionBlock
        && program.data[body.offset + 5] == 10
        && feed
            .iter()
            .all(|r| program.data[r.offset + 9..r.offset + 15] == [0; 6])
}

/// Native comparison result row after deleting its staggered MOVE consumer.
fn comparison_result_scaffold(program: &LadderProgramData, row: u16, x: u8) -> Option<usize> {
    if x != 19 || row == 0 {
        return None;
    }
    let rows = program.iec_row_frames()?;
    let top = rows.iter().find(|r| r.row_index == row - 1)?;
    let group = rows
        .iter()
        .filter(|r| r.group_index == top.group_index)
        .collect::<Vec<_>>();
    if group.len() != 4
        || group
            .iter()
            .enumerate()
            .any(|(i, r)| r.row_index != row - 1 + i as u16)
        || program.data[top.start - 6..top.start - 2] != [0; 4]
    {
        return None;
    }
    let blocks = program.iec_function_blocks()?;
    let mut bodies = blocks.iter().filter(|b| b.group_index == top.group_index);
    let eq = bodies.next()?;
    if bodies.next().is_some()
        || eq.row_index != row - 1
        || eq.raw_x != 10
        || eq.opcode_family != 0x28
        || spec(&eq.name.value).is_none_or(|(f, op, n)| f != 0x28 || op != eq.opcode || n != 3)
        || eq.pin_count != 3
        || eq.instance.is_some()
        || program.data[eq.record_offset + 11..eq.record_offset + 15] != [0, 2, 0, 0]
    {
        return None;
    }
    let records = program.iec_record_frames()?;
    let prefix = records
        .iter()
        .filter(|r| r.group_index == top.group_index && r.row_index == row - 1)
        .collect::<Vec<_>>();
    if !wired_comparison_prefix(program, &prefix) {
        return None;
    }
    for (index, expected) in [
        vec![
            (IecRecordKind::FunctionOperand, 7),
            (IecRecordKind::LinkReference(0x68), 10),
        ],
        vec![
            (IecRecordKind::FunctionOperand, 7),
            (IecRecordKind::LinkReference(0x68), 10),
        ],
        vec![(IecRecordKind::LinkReference(0x69), 10)],
    ]
    .iter()
    .enumerate()
    {
        let parts = records
            .iter()
            .filter(|r| r.group_index == top.group_index && r.row_index == row + index as u16)
            .collect::<Vec<_>>();
        if parts.len() != expected.len()
            || parts
                .iter()
                .zip(expected)
                .any(|(r, (kind, x))| r.kind != *kind || program.data[r.offset + 5] != *x)
        {
            return None;
        }
    }
    let refs = program.iec_function_references()?;
    let links = program.iec_function_operand_links()?;
    if refs
        .iter()
        .filter(|r| r.target_record_offset == eq.record_offset)
        .count()
        != 3
        || links
            .iter()
            .filter(|r| r.target_record_offset == eq.record_offset)
            .count()
            != 2
        || links
            .iter()
            .any(|r| r.target_record_offset == eq.record_offset && r.is_output)
    {
        return None;
    }
    Some(eq.record_offset)
}

fn trigger_scaffold(program: &LadderProgramData, row: u16, x: u8) -> Result<bool, XgwxError> {
    if x != 19 {
        return Ok(false);
    }
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let Some(top) = rows.iter().find(|r| r.row_index == row) else {
        return Ok(false);
    };
    let group = rows
        .iter()
        .filter(|r| r.group_index == top.group_index)
        .collect::<Vec<_>>();
    let prefix = records
        .iter()
        .filter(|r| r.group_index == top.group_index && r.row_index == row)
        .collect::<Vec<_>>();
    if group.len() != 2
        || group[0].row_index != row
        || group[1].row_index != row + 1
        || program.data[group[0].start - 6..group[0].start - 2] != [0; 4]
        || !trigger_prefix(program, &prefix)
    {
        return Ok(false);
    }
    let bottom = records
        .iter()
        .filter(|r| r.group_index == top.group_index && r.row_index == row + 1)
        .collect::<Vec<_>>();
    Ok(
        matches!(bottom.as_slice(), [reference] if reference.kind == IecRecordKind::LinkReference(0x69)
        && program.data[reference.offset] == 1
        && program.data[reference.offset + 5] == 10
        && program.data[reference.offset + 6..reference.offset + 8] == (row * 4).to_le_bytes()),
    )
}

/// A block may extend the final branch-only row without merging the next network.
pub(crate) fn branch_tail(
    program: &LadderProgramData,
    row: u16,
    x: u8,
) -> Result<Option<u8>, XgwxError> {
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let Some(frame) = rows.iter().find(|frame| frame.row_index == row) else {
        return Ok(None);
    };
    let at_row = records
        .iter()
        .filter(|record| record.row_index == row)
        .collect::<Vec<_>>();
    if at_row.len() != 1
        || at_row[0].kind != IecRecordKind::BranchEnd
        || rows
            .iter()
            .any(|other| other.group_index == frame.group_index && other.row_index > row)
    {
        return Ok(None);
    }
    let boundary = program.data[at_row[0].offset + 5];
    if x < boundary + 4 {
        return Ok(None);
    }
    if !matches!(
        (boundary, x),
        (3, 7) | (6, 13) | (9, 13) | (9, 16) | (12, 16) | (15, 19)
    ) {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "function placement on this branch boundary is not yet native-validated",
        });
    }
    if boundary != 3 {
        let first = rows
            .iter()
            .find(|r| r.group_index == frame.group_index)
            .unwrap();
        let mode = u32::from_le_bytes(
            program.data[first.start - 6..first.start - 2]
                .try_into()
                .unwrap(),
        );
        if !matches!(
            (boundary, x, mode),
            (6, 13, 0) | (9, 13, 0) | (9, 16, 1) | (12, 16, 1) | (15, 19, 0)
        ) {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "completed branch function placement in this execution mode is not native-validated",
            });
        }
    }
    let layout = program
        .iec_circuit_layout()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    Ok(layout
        .open_branch_endpoints
        .iter()
        .any(|point| {
            point.group_index == frame.group_index && point.row_index == row && point.x == boundary
        })
        .then_some(boundary))
}

/// Contact rows retained by native scalar deletion can feed a new block.
/// Keep placement restricted to the captured contact, boundary and mode shapes.
pub(crate) fn contact_tail(
    program: &LadderProgramData,
    row: u16,
    x: u8,
) -> Result<Option<u8>, XgwxError> {
    if crate::iec_staggered_move_write::scaffold(program, row, x) {
        return Ok(Some(1));
    }
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let Some(top) = rows.iter().find(|r| r.row_index == row) else {
        return Ok(None);
    };
    if rows
        .iter()
        .any(|r| r.group_index == top.group_index && r.row_index > row)
    {
        return Ok(None);
    }
    let first = rows
        .iter()
        .find(|r| r.group_index == top.group_index)
        .unwrap();
    let mode = u32::from_le_bytes(
        program.data[first.start - 6..first.start - 2]
            .try_into()
            .unwrap(),
    );
    let at_top = records
        .iter()
        .filter(|r| r.group_index == top.group_index && r.row_index == row)
        .collect::<Vec<_>>();
    let (contact, branch) = match at_top.as_slice() {
        [contact] if contact.kind == IecRecordKind::Contact(6) => (*contact, false),
        [end, contact]
            if end.kind == IecRecordKind::BranchEnd
                && program.data[end.offset + 5] == 3
                && contact.kind == IecRecordKind::Contact(7) =>
        {
            (*contact, true)
        }
        _ => return Ok(None),
    };
    let c = program.data[contact.offset + 5];
    let enabled_move = c == 1
        && matches!(x, 16 | 19)
        && mode == 0
        && !branch
        && first.row_index == row
        && program.data[top.start + 17..top.start + 21]
            == (if x == 19 { 50u32 } else { 39u32 }).to_le_bytes()
        && rows
            .iter()
            .find(|r| r.row_index > row)
            .is_some_and(|next| next.row_index >= row.saturating_add(3));
    Ok(
        (enabled_move || matches!((c, x, mode, branch), (4, 13, 0, true) | (1, 19, 1, false)))
            .then_some(c),
    )
}

/// Native contact-fed scalar branches retain both contacts and their branch records.
/// Return the branch boundary and the first feed cell after the contacts.
fn continuing_contact_prefix(
    program: &LadderProgramData,
    records: &[&crate::IecRecordFrame],
) -> Option<(u8, u8)> {
    if let [end, left, right, start] = records {
        return (end.kind == IecRecordKind::BranchEnd
            && program.data[end.offset + 5] == 3
            && left.kind == IecRecordKind::Contact(6)
            && program.data[left.offset + 5] == 4
            && right.kind == IecRecordKind::Contact(7)
            && program.data[right.offset + 5] == 7
            && program.data[left.offset + 11..left.offset + 15] == [0; 4]
            && program.data[right.offset + 11..right.offset + 15] == [0; 4]
            && start.kind == IecRecordKind::BranchStart
            && program.data[start.offset + 7] == 9)
            .then_some((9, 10));
    }
    let (left, right, start) = match records {
        [left, start, right] if start.kind == IecRecordKind::BranchStart => (*left, *right, *start),
        [left, right, start] if start.kind == IecRecordKind::BranchStart => (*left, *right, *start),
        _ => return None,
    };
    if left.kind != IecRecordKind::Contact(6)
        || right.kind != IecRecordKind::Contact(6)
        || program.data[left.offset + 5] != 1
        || program.data[right.offset + 5] != 4
        || program.data[left.offset + 11..left.offset + 15] != [0; 4]
        || program.data[right.offset + 11..right.offset + 15] != [0; 4]
    {
        return None;
    }
    let boundary = program.data[start.offset + 7];
    if (records[1].kind == IecRecordKind::BranchStart && boundary == 3)
        || (records[2].kind == IecRecordKind::BranchStart && boundary == 6)
    {
        Some((boundary, 7))
    } else {
        None
    }
}

fn parallel_contact_prefix(
    program: &LadderProgramData,
    records: &[&crate::IecRecordFrame],
    row: u16,
) -> bool {
    let [wire, left, contact, right] = records else {
        return false;
    };
    wire.kind == IecRecordKind::ShortWire
        && program.data[wire.offset + 5] == 1
        && program.data[wire.offset + 9..wire.offset + 15] == [0; 6]
        && contact.kind == IecRecordKind::Contact(6)
        && program.data[contact.offset + 5] == 4
        && program.data[contact.offset + 11..contact.offset + 15] == [0; 4]
        && [(*left, 3u8), (*right, 6u8)]
            .into_iter()
            .all(|(r, boundary)| {
                r.kind == IecRecordKind::BranchStart
                    && program.data[r.offset + 7] == boundary
                    && program.data[r.offset + 18..r.offset + 20] == ((row + 1) * 4).to_le_bytes()
                    && program.data[r.offset + 11..r.offset + 17] == [0; 6]
            })
}

fn parallel_contact_bottom(
    program: &LadderProgramData,
    records: &[&crate::IecRecordFrame],
    row: u16,
) -> bool {
    let [left, contact, right] = records else {
        return false;
    };
    contact.kind == IecRecordKind::Contact(6)
        && program.data[contact.offset + 5] == 4
        && program.data[contact.offset + 11..contact.offset + 15] == [0; 4]
        && [(*left, 3u8), (*right, 6u8)]
            .into_iter()
            .all(|(r, boundary)| {
                r.kind == IecRecordKind::BranchEnd
                    && program.data[r.offset + 5] == boundary
                    && program.data[r.offset + 6..r.offset + 8] == (row * 4).to_le_bytes()
            })
}

/// Native MOVE Delete retains the two contacts between branch boundaries 3 and 6.
fn parallel_contact_scaffold(program: &LadderProgramData, row: u16, x: u8, count: u8) -> bool {
    if x != 19 || count != 2 || row > u16::MAX / 4 - 2 {
        return false;
    }
    let Some(rows) = program.iec_row_frames() else {
        return false;
    };
    let Some(records) = program.iec_record_frames() else {
        return false;
    };
    let Some(first) = rows.iter().find(|r| r.row_index == row) else {
        return false;
    };
    let group = rows
        .iter()
        .filter(|r| r.group_index == first.group_index)
        .collect::<Vec<_>>();
    if group.len() != 2
        || group[0].row_index != row
        || group[1].row_index != row + 1
        || program.data[group[0].start - 6..group[0].start - 2] != [0; 4]
        || rows
            .iter()
            .find(|r| r.row_index > row + 1)
            .is_none_or(|r| r.row_index < row + 3)
    {
        return false;
    }
    let at = |y| {
        records
            .iter()
            .filter(|r| r.group_index == first.group_index && r.row_index == y)
            .collect::<Vec<_>>()
    };
    parallel_contact_prefix(program, &at(row), row)
        && parallel_contact_bottom(program, &at(row + 1), row)
}

/// A scalar body deleted from a continuing branch leaves its spine rows.
pub(crate) fn continuing_scaffold(
    program: &LadderProgramData,
    row: u16,
    x: u8,
    count: u8,
) -> Result<Option<u8>, XgwxError> {
    if count == 3 {
        if let Some(boundary) = crate::iec_chain_comparison_write::feed_boundary(program, row, x) {
            return Ok(Some(boundary));
        }
    }
    if parallel_contact_scaffold(program, row, x, count) {
        return Ok(Some(6));
    }
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let Some(top) = rows.iter().find(|r| r.row_index == row) else {
        return Ok(None);
    };
    let first = rows
        .iter()
        .find(|r| r.group_index == top.group_index)
        .unwrap();
    let mode = u32::from_le_bytes(
        program.data[first.start - 6..first.start - 2]
            .try_into()
            .unwrap(),
    );
    let last = row
        .checked_add(u16::from(count))
        .filter(|last| *last < u16::MAX / 4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let at_top = records
        .iter()
        .filter(|r| r.group_index == top.group_index && r.row_index == row)
        .collect::<Vec<_>>();
    let upper = continuing_contact_prefix(program, &at_top)
        .filter(|_| mode == 0 && x == 13 && (first.row_index == row || at_top.len() == 4));
    let mut boundary = upper.map(|p| p.0);
    if upper.is_some() {
        let start = at_top
            .iter()
            .find(|r| r.kind == IecRecordKind::BranchStart)
            .unwrap();
        if program.data[start.offset + 18..start.offset + 20] != ((row + 1) * 4).to_le_bytes() {
            return Ok(None);
        }
    }
    for y in row + u16::from(upper.is_some())..=last {
        let at_row = records
            .iter()
            .filter(|r| r.group_index == top.group_index && r.row_index == y)
            .collect::<Vec<_>>();
        let [end, start] = at_row.as_slice() else {
            return Ok(None);
        };
        if end.kind != IecRecordKind::BranchEnd || start.kind != IecRecordKind::BranchStart {
            return Ok(None);
        }
        let b = program.data[end.offset + 5];
        if boundary.is_some_and(|old| old != b)
            || program.data[start.offset + 7] != b
            || program.data[start.offset + 18..start.offset + 20] != ((y + 1) * 4).to_le_bytes()
        {
            return Ok(None);
        }
        boundary = Some(b);
    }
    let b = boundary.unwrap();
    if upper.is_none() && !matches!((b, x, mode), (9, 13, 0) | (12, 16, 1) | (15, 19, 0)) {
        return Ok(None);
    }
    let following = last + 1;
    Ok(records
        .iter()
        .any(|r| {
            r.group_index == top.group_index
                && r.row_index == following
                && r.kind == IecRecordKind::BranchEnd
                && program.data[r.offset + 5] == b
        })
        .then_some(b))
}

/// Materialize the branch row inserted above the final pin row of an upper MOVE.
/// The caller has shifted rows with the native blank-row operation already.
pub(crate) fn materialize_upper_move_row(
    program: &LadderProgramData,
    row: u16,
    x: u8,
) -> Result<Vec<u8>, XgwxError> {
    let unsupported = || XgwxError::UnsupportedLadderLayout;
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let top = rows
        .iter()
        .find(|r| r.row_index == row)
        .ok_or_else(unsupported)?;
    let prefix = records
        .iter()
        .filter(|r| r.group_index == top.group_index && r.row_index == row)
        .collect::<Vec<_>>();
    if x != 13
        || continuing_contact_prefix(program, &prefix) != Some((3, 7))
        || rows.iter().any(|r| r.row_index == row + 2)
    {
        return Err(unsupported());
    }
    let upper = records
        .iter()
        .find(|r| {
            r.group_index == top.group_index
                && r.row_index == row + 1
                && r.kind == IecRecordKind::BranchStart
                && program.data[r.offset + 7] == 3
        })
        .ok_or_else(unsupported)?;
    let lower = records
        .iter()
        .find(|r| {
            r.group_index == top.group_index
                && r.row_index == row + 3
                && r.kind == IecRecordKind::BranchEnd
                && program.data[r.offset + 5] == 3
        })
        .ok_or_else(unsupported)?;
    if program.data[upper.offset + 18..upper.offset + 20] != ((row + 3) * 4).to_le_bytes()
        || program.data[lower.offset + 6..lower.offset + 8] != ((row + 1) * 4).to_le_bytes()
    {
        return Err(unsupported());
    }
    let insertion = rows
        .iter()
        .find(|r| r.group_index == top.group_index && r.row_index == row + 3)
        .ok_or_else(unsupported)?
        .start;
    let first = rows
        .iter()
        .find(|r| r.group_index == top.group_index)
        .ok_or_else(unsupported)?;
    let mut updated = program.data.clone();
    updated[upper.offset + 18..upper.offset + 20].copy_from_slice(&((row + 2) * 4).to_le_bytes());
    updated[lower.offset + 6..lower.offset + 8].copy_from_slice(&((row + 2) * 4).to_le_bytes());
    let count_offset = first.start - 2;
    let count = u16::from_le_bytes(updated[count_offset..count_offset + 2].try_into().unwrap());
    updated[count_offset..count_offset + 2]
        .copy_from_slice(&count.checked_add(1).ok_or_else(unsupported)?.to_le_bytes());
    let mut frame = row_header(row + 2);
    frame[29] = 3;
    frame[33..35].copy_from_slice(&2u16.to_le_bytes());
    frame.extend(branch_end(row + 2, 3));
    frame.extend(branch_start(row + 2, 3, 0));
    updated.splice(insertion..insertion, frame);
    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    if continuing_scaffold(&verified, row, x, 3)? != Some(3)
        || verified.iec_circuit_graph().is_none()
    {
        return Err(unsupported());
    }
    Ok(updated)
}

/// The branch-only rows retained by native block deletion can be reused.
pub(crate) fn branch_scaffold(
    program: &LadderProgramData,
    row: u16,
    x: u8,
    count: u8,
) -> Result<Option<u8>, XgwxError> {
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let Some(top) = rows.iter().find(|frame| frame.row_index == row) else {
        return Ok(None);
    };
    let group_tail = rows
        .iter()
        .filter(|frame| frame.group_index == top.group_index && frame.row_index >= row)
        .collect::<Vec<_>>();
    // Native insertion keeps the surplus branch row when MOVE replaces a
    // three-pin block. Only the captured two- and three-pin scaffolds qualify.
    if !(3..=4).contains(&group_tail.len()) || group_tail.len() < usize::from(count) + 1 {
        return Ok(None);
    }
    let last = group_tail.last().unwrap().row_index;
    let mut boundary = None;
    for (ordinal, frame) in group_tail.iter().enumerate() {
        if frame.row_index != row + ordinal as u16 {
            return Ok(None);
        }
        let at_row = records
            .iter()
            .filter(|record| {
                record.group_index == top.group_index && record.row_index == frame.row_index
            })
            .collect::<Vec<_>>();
        let final_row = frame.row_index == last;
        if at_row.len() != if final_row { 1 } else { 2 }
            || at_row[0].kind != IecRecordKind::BranchEnd
            || (!final_row && at_row[1].kind != IecRecordKind::BranchStart)
        {
            return Ok(None);
        }
        let current = program.data[at_row[0].offset + 5];
        if boundary.is_some_and(|expected| expected != current) {
            return Ok(None);
        }
        boundary = Some(current);
        if !final_row {
            let bytes = &program.data[at_row[1].offset..at_row[1].end];
            if bytes[7] != current
                || u16::from_le_bytes([bytes[18], bytes[19]]) != (frame.row_index + 1) * 4
            {
                return Ok(None);
            }
        }
    }
    let boundary = boundary.unwrap();
    if x < boundary + 4 {
        return Ok(None);
    }
    if boundary != 3 || x != 7 {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "function placement on this branch scaffold is not yet native-validated",
        });
    }
    let layout = program
        .iec_circuit_layout()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    Ok(layout
        .open_branch_endpoints
        .iter()
        .any(|point| {
            point.group_index == top.group_index && point.row_index == last && point.x == boundary
        })
        .then_some(boundary))
}

pub(crate) fn branch_start(row: u16, boundary: u8, flags: u8) -> Vec<u8> {
    let y = (row * 4).to_le_bytes();
    let next = ((row + 1) * 4).to_le_bytes();
    vec![
        0,
        0,
        0,
        0,
        0,
        2,
        0,
        boundary,
        y[0],
        y[1],
        0,
        0,
        0,
        flags,
        0,
        0,
        0,
        boundary - 1,
        next[0],
        next[1],
        0,
        0,
        0,
        flags,
        0,
        0,
        0,
    ]
}

pub(crate) fn branch_end(row: u16, boundary: u8) -> Vec<u8> {
    let previous = ((row - 1) * 4).to_le_bytes();
    vec![1, 0, 0, 0, 0, boundary, previous[0], previous[1], 0]
}

pub(crate) fn body_with_flags(name: &str, row: u16, x: u8, flags: u8) -> Vec<u8> {
    let mut bytes = body(name, row, x);
    if flags == 0 {
        return bytes;
    }
    bytes[11] = flags;
    bytes[32] = flags;
    // Pin coordinate headers follow the six leading marker strings.
    let mut cursor = 38;
    for descriptor in 0..6 {
        let length = usize::from(bytes[cursor + 3]);
        cursor += 4 + length * 2;
        if descriptor == 0 || descriptor == 2 {
            cursor += 4;
        }
        if descriptor == 1 || descriptor == 3 {
            cursor += 5;
        }
    }
    let count = spec(name).unwrap().2;
    for ordinal in 1..=count {
        bytes[cursor + 6] = flags;
        cursor += 12;
        if ordinal == count {
            break;
        }
        for _ in 0..2 {
            cursor += 4 + usize::from(bytes[cursor + 3]) * 2;
            cursor += 4;
            cursor += 4 + usize::from(bytes[cursor + 3]) * 2;
            cursor += 5;
        }
    }
    bytes
}

/// Recreate the native wired comparison, preserving its retained MOVE rows.
fn insert_wired_comparison(
    program: &LadderProgramData,
    row: u16,
    x: u8,
    name: &str,
    operands: &[String],
) -> Result<Vec<u8>, XgwxError> {
    if crate::iec_coil_comparison_write::scaffold(program, row, x).is_some() {
        return crate::iec_coil_comparison_write::insert(program, row, x, name, operands);
    }
    if crate::iec_paired_comparison_write::scaffold(program, row, x).is_some() {
        return crate::iec_paired_comparison_write::insert(program, row, x, name, operands);
    }
    let unsupported = || XgwxError::UnsupportedLadderLayout;
    let group_index = wired_comparison_scaffold(program, row, x).ok_or_else(unsupported)?;
    if spec(name).is_none_or(|(f, _, n)| f != 0x28 || n != 3)
        || operands.len() != 2
        || operands.iter().any(|v| {
            v.is_empty() || v.encode_utf16().count() > 255 || v.chars().any(char::is_control)
        })
    {
        return Err(unsupported());
    }
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    let group = rows
        .iter()
        .filter(|r| r.group_index == group_index)
        .collect::<Vec<_>>();
    let first = group[0];
    let last = group[2];
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let mut replacement = program.data[first.start - 10..first.start].to_vec();
    replacement[8..10].copy_from_slice(&4u16.to_le_bytes());
    let mut header = row_header(row);
    header[26..28].copy_from_slice(&program.data[first.start + 26..first.start + 28]);
    header[29] = x;
    header[33..35].copy_from_slice(&2u16.to_le_bytes());
    replacement.extend(header);
    replacement.extend(wire(row, 1, x - 3));
    let mut comparison = body(name, row, x);
    comparison[12] = 0;
    replacement.extend(comparison);
    for (index, frame) in group.iter().enumerate() {
        let mut header = program.data[frame.start..frame.records_start].to_vec();
        let mut parts = records
            .iter()
            .filter(|r| r.group_index == group_index && r.row_index == frame.row_index)
            .map(|r| program.data[r.offset..r.end].to_vec())
            .collect::<Vec<_>>();
        if index < 2 {
            parts.push(expression(frame.row_index, x - 3, &operands[index]));
        }
        let y = (row * 4).to_le_bytes();
        parts.push(vec![
            index as u8 + 1,
            if index == 2 { 0x69 } else { 0x68 },
            0,
            0,
            0,
            x,
            y[0],
            y[1],
            0,
        ]);
        parts.sort_by_key(|r| r[5]);
        header[29] = parts.iter().map(|r| r[5]).max().unwrap();
        header[33..35].copy_from_slice(&(parts.len() as u16).to_le_bytes());
        replacement.extend(header);
        for part in parts {
            replacement.extend(part);
        }
    }
    let mut verified = program.clone();
    verified
        .data
        .splice(first.start - 10..last.end, replacement);
    verified.decoded_len = verified.data.len();
    let block = verified
        .iec_function_blocks()
        .ok_or_else(unsupported)?
        .into_iter()
        .find(|b| b.row_index == row && b.raw_x == x)
        .ok_or_else(unsupported)?;
    if (verified.iec_circuit_graph().is_none()
        && !crate::writer::iec_preserves_groups_around_edit(
            program,
            &verified,
            block.group_index..block.group_index + 1,
            1,
        ))
        || remove_wired_comparison(&verified, block.record_offset)? != program.data
    {
        return Err(unsupported());
    }
    Ok(verified.data)
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
    if crate::iec_coil_comparison_write::scaffold(program, row, x).is_some() {
        return crate::iec_coil_comparison_write::insert(program, row, x, name, operands);
    }
    if operands.len() == 2
        && spec(name).is_some_and(|(f, _, _)| f == 0x28)
        && wired_comparison_scaffold(program, row, x).is_some()
    {
        return insert_wired_comparison(program, row, x, name, operands);
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
    let result_fed = comparison_result_scaffold(program, row, x);
    if result_fed.is_some() && name != "MOVE" {
        return Err(invalid(
            "only MOVE refill on an EQ result is native-validated",
        ));
    }
    let shared_power_row = result_fed.is_some()
        || records
            .iter()
            .any(|record| record.row_index == row && record.kind == IecRecordKind::FunctionBlock);
    let continuing = continuing_scaffold(program, row, x, count)?;
    let chain_comparison = crate::iec_chain_comparison_write::scaffold(program, row, x);
    if chain_comparison && spec(name).is_none_or(|(family, _, _)| family != 0x28) {
        return Err(invalid(
            "only comparison refill in this contact-fed chain is native-validated",
        ));
    }
    let parallel_contact = parallel_contact_scaffold(program, row, x, count);
    if parallel_contact && name != "MOVE" {
        return Err(invalid(
            "only MOVE refill on this parallel contact feed is native-validated",
        ));
    }
    let trigger_fed = trigger_scaffold(program, row, x)?;
    if trigger_fed && conversion_types(name).is_some() {
        return Err(invalid(
            "conversion placement after R_TRIG is not yet native-validated",
        ));
    }
    let top_prefix = records
        .iter()
        .filter(|r| r.row_index == row)
        .collect::<Vec<_>>();
    let upper_contact =
        continuing.is_some() && continuing_contact_prefix(program, &top_prefix).is_some();
    if continuing.is_some() && conversion_types(name).is_some() {
        return Err(invalid(
            "conversion placement on a continuing branch is not yet native-validated",
        ));
    }
    let scaffold = continuing.or(branch_scaffold(program, row, x, count)?);
    let branch = branch_tail(program, row, x)?.or(scaffold);
    // Completed tails stop at EN. Continuing scaffolds retain their spine.
    // Both refill using the retained EN row's cached end-Y sequence.
    let contact = contact_tail(program, row, x)?;
    let staggered_move = crate::iec_staggered_move_write::scaffold(program, row, x);
    if contact == Some(1)
        && rows
            .iter()
            .find(|r| r.row_index == row)
            .is_some_and(|top| program.data[top.start - 6..top.start - 2] == [0; 4])
        && name != "MOVE"
    {
        return Err(invalid(
            "only MOVE refill on this enabled contact feed is native-validated",
        ));
    }
    let completed_tail = (branch.is_some_and(|boundary| boundary != 3) && scaffold.is_none())
        || contact.is_some()
        || continuing.is_some();
    let graph = program
        .iec_circuit_layout()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let independent_fresh = rows.iter().all(|frame| frame.row_index < row);
    let total = u16::from_le_bytes(program.data[4..6].try_into().unwrap());
    // XG5000 materializes implicit blank rows below the stored program.
    // Fresh MOVE L87/x4, ADD L91/x16 and EQ L96/x10 captures share this
    // envelope, preserving gaps and all earlier row coordinates. Conversion
    // placement retains its separate native acceptance boundary.
    if row > total && conversion_types(name).is_some() {
        return Err(invalid(
            "fresh conversion placement beyond the final stored row is not native-validated",
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
            && !(branch == Some(edge.start.x) && (edge.end.row_index == row || scaffold.is_some()))
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
    let mut mode = 0u32;
    for group in group_rows.iter().take(end).skip(first) {
        let start = group[0].start - 10;
        let value = u32::from_le_bytes(program.data[start + 4..start + 8].try_into().unwrap());
        if value > 1 {
            return Err(invalid("unsupported IEC network execution mode"));
        }
        mode |= value;
    }
    let flags = if mode == 1 { 4 } else { 0 };
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
    if trigger_fed {
        let at = rows.iter().find(|r| r.row_index == row).unwrap().start + 26;
        let cached: [u8; 2] = program.data[at..at + 2].try_into().unwrap();
        for r in row + 1..=last {
            let header = &mut merged.get_mut(&r).unwrap().0;
            header[29] = if r == row + 1 { x + 3 } else { x };
            let end_y = if r == last {
                ((last - 1) * 4).to_le_bytes()
            } else {
                cached
            };
            header[26..28].copy_from_slice(&end_y);
        }
    }
    if let Some(boundary) = branch.or(contact) {
        merged.get_mut(&row).unwrap().0[29] = x;
        for r in row + 1..=last {
            let (header, records) = merged.get_mut(&r).unwrap();
            if parallel_contact && r == last {
                // This row was discarded by Delete. Reopening the saved file
                // creates it with the actual coordinate, retaining the lower
                // contact row's existing cache.
                header[26..28].copy_from_slice(&(r * 4).to_le_bytes());
            } else if upper_contact || chain_comparison {
                // Native refill preserves the retained contact-fed branch headers,
                // including the end-Y cache of a pin row shifted by Ctrl+L.
            } else if staggered_move {
                // Both operand rows already belong to the lower MOVE. Native
                // refill merges the groups without changing their end-Y caches.
            } else if contact == Some(1) && x == 16 {
                // Reopening the saved enabled contact tail discards native
                // hidden pin-row caches. Fresh x16 pins use their actual rows.
                header[26..28].copy_from_slice(&(r * 4).to_le_bytes());
            } else if completed_tail {
                // Native scalar refill continues the retained EN row's
                // cached end Y, which can differ from the actual row in a
                // disabled network. Pin and reference coordinates remain real.
                let top = rows.iter().find(|frame| frame.row_index == row).unwrap();
                let cached = u16::from_le_bytes(
                    program.data[top.start + 26..top.start + 28]
                        .try_into()
                        .unwrap(),
                );
                let cached = cached
                    .checked_add((r - row) * 4)
                    .ok_or(XgwxError::UnsupportedLadderLayout)?;
                header[26..28].copy_from_slice(&cached.to_le_bytes());
            } else {
                header[26..28].copy_from_slice(&(row * 4).to_le_bytes());
            }
            header[29] = if r == row + 1 { x + 3 } else { x };
            if scaffold.is_none() && !completed_tail {
                records.push(branch_end(r, boundary));
                if r < last {
                    records.push(branch_start(r, boundary, 0));
                }
            }
        }
    }
    if conversion_types(name).is_some() {
        // The conversion's native envelope has a taller EN row and caches
        // the output cell as the end cell of its shared operand row.
        let top_header = &mut merged.get_mut(&row).unwrap().0;
        let height = u32::from_le_bytes(top_header[17..21].try_into().unwrap());
        top_header[17..21].copy_from_slice(&height.max(50).to_le_bytes());
        let operand_header = &mut merged.get_mut(&(row + 1)).unwrap().0;
        operand_header[29] = operand_header[29].max(x + 3);
    }
    if let Some(eq_offset) = result_fed {
        let eq_x = program.data[eq_offset + 5];
        merged
            .get_mut(&(row - 1))
            .unwrap()
            .1
            .iter_mut()
            .find(|r| r[1] == 0x67 && r[5] == eq_x)
            .unwrap()[12] = 0;
        merged
            .get_mut(&row)
            .unwrap()
            .1
            .push(wire(row, eq_x + 3, x - 3));
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
    if let Some(boundary) = branch {
        if scaffold.is_none() && !completed_tail {
            retained.push(branch_start(row, boundary, flags));
        }
        let prefix = records
            .iter()
            .filter(|r| r.row_index == row)
            .collect::<Vec<_>>();
        let feed_start = continuing_contact_prefix(program, &prefix).map_or(boundary + 1, |p| p.1);
        let mut feed = wire(row, feed_start, x - 3);
        feed[11] = flags;
        retained.push(feed);
    } else if let Some(contact) = contact {
        let mut feed = wire(row, contact + 3, x - 3);
        feed[11] = flags;
        retained.push(feed);
    } else if retained.is_empty() && x > 1 {
        retained.push(wire(row, 1, x - 3));
    }
    // Native placement after a chain tail reconnects the previous ENO to EN.
    // Existing feeds (including short wires) already end at the new input.
    if branch.is_none() {
        let input_connected = retained.iter().any(|record| {
            (record[1] == 1 && record[5] == x - 3) || (record[1] == 2 && record[15] == x - 3)
        });
        if let Some(previous) = retained
            .iter()
            .filter(|record| record[1] == 0x67 && record[5] < x)
            .map(|record| record[5])
            .max()
            .filter(|_| !input_connected)
        {
            let start = previous + 3;
            if start <= x - 3
                && !retained
                    .iter()
                    .any(|record| record[5] >= start && record[5] < x)
            {
                if trigger_fed {
                    retained.push(wire(row, start, x - 3));
                } else {
                    // Preserve the captured chain's one-cell feeds. Each short
                    // wire reaches the following cell, including the final EN.
                    for cell in (start..x).step_by(3) {
                        let y = (row * 4).to_le_bytes();
                        let mut feed =
                            vec![0xff, 1, 0, 0, 0, cell, y[0], y[1], 0, 0, 0, 0, 0, 0, 0];
                        feed[11] = flags;
                        retained.push(feed);
                    }
                }
            }
        }
    }
    if let Some(previous) = retained
        .iter()
        .filter(|record| record[1] == 0x67 && record[5] < x)
        .map(|record| record[5])
        .max()
        .filter(|previous| {
            retained.iter().all(|record| {
                record[5] <= *previous || record[5] >= x || matches!(record[1], 1 | 2)
            })
        })
    {
        retained
            .iter_mut()
            .find(|record| record[1] == 0x67 && record[5] == previous)
            .unwrap()[12] = 0;
    }
    let mut new_body = body_with_flags(
        name,
        row,
        x,
        if completed_tail && continuing.is_none() {
            0
        } else {
            flags
        },
    );
    if retained
        .iter()
        .any(|record| record[1] == 0x67 && record[5] > x)
    {
        new_body[12] = 0;
    }
    retained.push(new_body);
    *top = retained;
    for (index, operand) in operands.iter().enumerate() {
        let output = index + 1 == operands.len();
        let expr_row = row + if output { 1 } else { index as u16 + 1 };
        let mut expr = expression(expr_row, if output { x + 3 } else { x - 3 }, operand);
        expr[11] = flags;
        merged.get_mut(&expr_row).unwrap().1.push(expr);
        if output && !rows.iter().any(|existing| existing.row_index == expr_row) {
            // A fresh operand row ends at OUT, not at the function body.
            merged.get_mut(&expr_row).unwrap().0[29] = x + 3;
        }
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
            out.extend_from_slice(&mode.to_le_bytes());
            out.extend_from_slice(&(merged.len() as u16).to_le_bytes());
            for (_, (mut header, mut records)) in merged.clone() {
                if shared_power_row {
                    records.sort_by_key(|record| record[5]);
                    if let Some(last) = records.iter().map(|record| record[5]).max() {
                        header[29] = last;
                    }
                }
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
    let result = verified
        .iec_circuit_layout()
        .ok_or_else(|| invalid("IEC function placement did not preserve the circuit graph"))?;
    if branch.is_none()
        && !independent_fresh
        && !graph.open_branch_endpoints.is_empty()
        && !crate::writer::iec_preserves_groups_around_edit(program, &verified, first..end, 1)
    {
        return Err(invalid(
            "IEC trigger-fed placement changed unrelated networks",
        ));
    }
    if independent_fresh && !graph.open_branch_endpoints.is_empty() {
        // Appending a separate network must not rewrite earlier incomplete
        // wiring. Its old groups, record offsets and pin bindings stay exact.
        if out.get(8..program.data.len()) != Some(&program.data[8..])
            || result.open_branch_endpoints != graph.open_branch_endpoints
            || result.function_bindings.len() != graph.function_bindings.len() + usize::from(count)
            || result
                .function_bindings
                .get(..graph.function_bindings.len())
                != Some(graph.function_bindings.as_slice())
        {
            return Err(invalid(
                "independent IEC function placement changed earlier wiring or pins",
            ));
        }
    }
    if let Some(boundary) = branch {
        let mut expected_open = graph.open_branch_endpoints.clone();
        if chain_comparison {
            // Adding the EN feed connects the retained spine even when its
            // immediately preceding contact was deleted during editing.
            expected_open.retain(|point| {
                !(point.group_index == first && point.row_index == row && point.x == boundary)
            });
        }
        if continuing.is_none() {
            let point = expected_open
                .iter_mut()
                .find(|point| {
                    point.group_index == first
                        && (scaffold.is_some() || point.row_index == row)
                        && point.x == boundary
                })
                .ok_or(XgwxError::UnsupportedLadderLayout)?;
            if completed_tail {
                expected_open
                    .retain(|p| !(p.group_index == first && p.row_index == row && p.x == boundary));
            } else if scaffold.is_none() {
                point.row_index = last;
            }
        }
        expected_open.sort_by_key(|point| (point.group_index, point.row_index, point.x));
        let normalize = |mut binding: crate::IecFunctionBinding| {
            binding.block_record_offset = 0;
            binding.reference_record_offset = 0;
            binding.expression_record_offset = None;
            binding
        };
        let retained = result
            .function_bindings
            .iter()
            .filter(|binding| {
                binding.pin_point.row_index < row || binding.pin_point.row_index > last
            })
            .cloned()
            .map(normalize)
            .collect::<Vec<_>>();
        let before = graph
            .function_bindings
            .iter()
            .cloned()
            .map(normalize)
            .collect::<Vec<_>>();
        if result.open_branch_endpoints != expected_open
            || before != retained
            || result.function_bindings.len() != graph.function_bindings.len() + usize::from(count)
            || new_count != group_count
        {
            return Err(invalid(
                "IEC branch function placement changed unrelated wiring or pins",
            ));
        }
    }
    Ok(out)
}

/// Native single-cell Delete of a terminal scalar after a completed branch
/// or a captured contact prefix. The prefix survives; feeds and empty pin rows
/// disappear.
fn remove_completed_branch_tail(
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
        || !matches!(block.opcode_family, 0x20 | 0x28)
        || block.instance.is_some()
        || !matches!(block.pin_count, 2 | 3)
        || !matches!(block.raw_x, 13 | 16 | 19)
        || spec(expected).is_none_or(|(family, opcode, count)| {
            family != block.opcode_family || opcode != block.opcode || count != block.pin_count
        })
        || program.data[offset + 12] != 2
    {
        return Err(unsupported());
    }
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let group_rows = rows
        .iter()
        .filter(|r| r.group_index == block.group_index)
        .collect::<Vec<_>>();
    let first = group_rows.first().ok_or_else(unsupported)?;
    let mode = u32::from_le_bytes(
        program.data[first.start - 6..first.start - 2]
            .try_into()
            .unwrap(),
    );
    if mode > 1 {
        return Err(unsupported());
    }
    let top = group_rows
        .iter()
        .find(|r| r.row_index == block.row_index)
        .ok_or_else(unsupported)?;
    let last = group_rows.last().ok_or_else(unsupported)?;
    if last.row_index != block.row_index + u16::from(block.pin_count) {
        return Err(unsupported());
    }
    let at_top = records
        .iter()
        .filter(|r| r.group_index == block.group_index && r.row_index == block.row_index)
        .collect::<Vec<_>>();
    if at_top.last().is_none_or(|r| r.offset != offset) {
        return Err(unsupported());
    }
    let (prefix, contact) = match at_top.as_slice() {
        [end, c, ..]
            if end.kind == IecRecordKind::BranchEnd
                && program.data[end.offset + 5] == 3
                && c.kind == IecRecordKind::Contact(7)
                && program.data[c.offset + 5] == 4
                && block.raw_x == 13
                && mode == 0 =>
        {
            (2, Some(4))
        }
        [c, ..]
            if c.kind == IecRecordKind::Contact(6)
                && program.data[c.offset + 5] == 1
                && matches!(block.raw_x, 16 | 19)
                && ((mode == 1 && block.raw_x == 19)
                    || (mode == 0
                        && expected == "MOVE"
                        && first.row_index == block.row_index
                        && group_rows.len() == 3
                        && blocks
                            .iter()
                            .filter(|b| b.group_index == block.group_index)
                            .count()
                            == 1
                        && program.data.get(offset..block.record_end)
                            == Some(body("MOVE", block.row_index, block.raw_x).as_slice()))) =>
        {
            (1, Some(1))
        }
        [end, ..] if end.kind == IecRecordKind::BranchEnd => (1, None),
        _ => return Err(unsupported()),
    };
    let boundary = contact.unwrap_or(program.data[at_top[0].offset + 5]);
    if contact.is_none()
        && !matches!(
            (boundary, block.raw_x, mode),
            (6, 13, 0) | (9, 13, 0) | (9, 16, 1) | (12, 16, 1) | (15, 19, 0)
        )
    {
        return Err(unsupported());
    }
    let feeds = &at_top[prefix..at_top.len() - 1];
    if contact.is_none()
        && !(feeds.len() == 1 && feeds[0].kind == IecRecordKind::LongWire)
        && !feeds
            .iter()
            .all(|feed| feed.kind == IecRecordKind::ShortWire)
    {
        return Err(unsupported());
    }
    let mut next = boundary + if contact.is_some() { 3 } else { 1 };
    for feed in feeds {
        if program.data[feed.offset + 5] != next
            || !(program.data[feed.offset + 9..feed.offset + 15] == [0; 6]
                || (mode == 1
                    && program.data[feed.offset + 9..feed.offset + 15] == [0, 0, 4, 0, 0, 0]))
        {
            return Err(unsupported());
        }
        next = match feed.kind {
            IecRecordKind::ShortWire => next.checked_add(3),
            IecRecordKind::LongWire
                if program.data[feed.offset + 15] >= next
                    && (program.data[feed.offset + 15] - next).is_multiple_of(3) =>
            {
                program.data[feed.offset + 15].checked_add(3)
            }
            _ => None,
        }
        .ok_or_else(unsupported)?;
    }
    if feeds.is_empty() || next != block.raw_x {
        return Err(unsupported());
    }
    let links = program
        .iec_function_operand_links()
        .ok_or_else(unsupported)?;
    let refs = program.iec_function_references().ok_or_else(unsupported)?;
    let owned_links = links
        .iter()
        .filter(|l| l.target_record_offset == offset)
        .collect::<Vec<_>>();
    let owned_refs = refs
        .iter()
        .filter(|r| r.target_record_offset == offset)
        .collect::<Vec<_>>();
    if owned_links.len() != usize::from(block.pin_count)
        || owned_refs.len() != usize::from(block.pin_count)
    {
        return Err(unsupported());
    }
    let mut owned = vec![offset];
    owned.extend(feeds.iter().map(|r| r.offset));
    owned.extend(owned_links.iter().map(|l| l.record_offset));
    owned.extend(owned_refs.iter().map(|r| r.record_offset));
    if records.iter().any(|r| {
        r.group_index == block.group_index
            && r.row_index > block.row_index
            && !owned.contains(&r.offset)
    }) || group_rows
        .iter()
        .filter(|r| r.row_index >= block.row_index)
        .enumerate()
        .any(|(i, r)| r.row_index != block.row_index + i as u16)
    {
        return Err(unsupported());
    }
    let mut retained = program.data[top.start..top.records_start].to_vec();
    retained[17..21].copy_from_slice(
        &(if contact == Some(1) && mode == 0 && block.raw_x == 19 {
            50u32
        } else {
            39u32
        })
        .to_le_bytes(),
    );
    retained[29] = if contact.is_some() {
        boundary
    } else {
        boundary - 1
    };
    retained[33..35].copy_from_slice(&(prefix as u16).to_le_bytes());
    for record in &at_top[..prefix] {
        retained.extend_from_slice(&program.data[record.offset..record.end]);
    }
    let mut updated = program.data.clone();
    let header = group_rows[0].start - 10;
    updated[header + 8..header + 10]
        .copy_from_slice(&((group_rows.len() - usize::from(block.pin_count)) as u16).to_le_bytes());
    updated.splice(top.start..last.end, retained);
    let total = u16::from_le_bytes(program.data[4..6].try_into().unwrap());
    if block.group_index + 1
        == usize::from(u16::from_le_bytes(program.data[6..8].try_into().unwrap()))
        && last.row_index.checked_add(1) == Some(total)
    {
        updated[4..6].copy_from_slice(&(block.row_index + 1).to_le_bytes());
    }
    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    let layout = verified.iec_circuit_layout().ok_or_else(unsupported)?;
    if verified
        .iec_function_blocks()
        .is_none_or(|b| b.len() + 1 != blocks.len())
        || verified
            .iec_function_operand_links()
            .is_none_or(|l| l.len() + usize::from(block.pin_count) != links.len())
        || verified
            .iec_function_references()
            .is_none_or(|r| r.len() + usize::from(block.pin_count) != refs.len())
        || (contact.is_none()
            && !layout.open_branch_endpoints.iter().any(|p| {
                p.group_index == block.group_index
                    && p.row_index == block.row_index
                    && p.x == boundary
            }))
    {
        return Err(unsupported());
    }
    Ok(updated)
}

/// Remove only a scalar body's owned records beside a continuing spine.
fn remove_continuing_scalar(
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
    if conversion_types(expected).is_some()
        || block.name.value != expected
        || block.instance.is_some()
        || !matches!(block.pin_count, 2 | 3)
        || !matches!(block.opcode_family, 0x20 | 0x28)
        || program.data[offset + 12] != 2
        || spec(expected).is_none_or(|(f, op, n)| {
            f != block.opcode_family || op != block.opcode || n != block.pin_count
        })
    {
        return Err(unsupported());
    }
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let links = program
        .iec_function_operand_links()
        .ok_or_else(unsupported)?;
    let refs = program.iec_function_references().ok_or_else(unsupported)?;
    let owned_links = links
        .iter()
        .filter(|l| l.target_record_offset == offset)
        .collect::<Vec<_>>();
    let owned_refs = refs
        .iter()
        .filter(|r| r.target_record_offset == offset)
        .collect::<Vec<_>>();
    if owned_links.len() != usize::from(block.pin_count)
        || owned_refs.len() != usize::from(block.pin_count)
    {
        return Err(unsupported());
    }
    let at_top = records
        .iter()
        .filter(|r| r.group_index == block.group_index && r.row_index == block.row_index)
        .collect::<Vec<_>>();
    let contact_prefix = [4, 3].into_iter().find_map(|n| {
        at_top.get(..n).and_then(|prefix| {
            continuing_contact_prefix(program, prefix).map(|(b, feed)| (n, b, feed))
        })
    });
    let upper = contact_prefix.map(|(_, b, feed)| (b, feed));
    let (prefix_count, b, feed_start) = if let Some((b, feed_start)) = upper {
        (contact_prefix.unwrap().0, b, feed_start)
    } else if let [end, start, _, _] = at_top.as_slice() {
        if end.kind != IecRecordKind::BranchEnd || start.kind != IecRecordKind::BranchStart {
            return Err(unsupported());
        }
        let b = program.data[end.offset + 5];
        (2, b, b + 1)
    } else {
        return Err(unsupported());
    };
    if at_top.last().is_none_or(|body| body.offset != offset) {
        return Err(unsupported());
    }
    let feeds = &at_top[prefix_count..at_top.len() - 1];
    let first = rows
        .iter()
        .find(|r| r.group_index == block.group_index)
        .ok_or_else(unsupported)?;
    let mode = u32::from_le_bytes(
        program.data[first.start - 6..first.start - 2]
            .try_into()
            .unwrap(),
    );
    if upper.is_some()
        && (mode != 0
            || (first.row_index != block.row_index && prefix_count != 4)
            || block.raw_x != 13)
    {
        return Err(unsupported());
    }
    let mut next = feed_start;
    for feed in feeds {
        if program.data[feed.offset + 5] != next
            || !(program.data[feed.offset + 9..feed.offset + 15] == [0; 6]
                || (mode == 1
                    && program.data[feed.offset + 9..feed.offset + 15] == [0, 0, 4, 0, 0, 0]))
        {
            return Err(unsupported());
        }
        next = match feed.kind {
            IecRecordKind::ShortWire if upper.is_some() => next.checked_add(3),
            IecRecordKind::LongWire
                if program.data[feed.offset + 15] >= next
                    && (program.data[feed.offset + 15] - next).is_multiple_of(3) =>
            {
                program.data[feed.offset + 15].checked_add(3)
            }
            _ => None,
        }
        .ok_or_else(unsupported)?;
    }
    if feeds.is_empty() || next != block.raw_x {
        return Err(unsupported());
    }
    let mut removed = vec![offset];
    removed.extend(feeds.iter().map(|feed| feed.offset));
    removed.extend(owned_links.iter().map(|l| l.record_offset));
    removed.extend(owned_refs.iter().map(|r| r.record_offset));
    let mut updated = program.data.clone();
    for y in block.row_index..=block.row_index + u16::from(block.pin_count) {
        let frame = rows
            .iter()
            .find(|r| r.group_index == block.group_index && r.row_index == y)
            .ok_or_else(unsupported)?;
        let retained = records
            .iter()
            .filter(|r| {
                r.group_index == block.group_index
                    && r.row_index == y
                    && !removed.contains(&r.offset)
            })
            .collect::<Vec<_>>();
        let upper_row = upper.is_some() && y == block.row_index;
        if if upper_row {
            continuing_contact_prefix(program, &retained) != upper
        } else {
            retained.len() != 2
                || retained[0].kind != IecRecordKind::BranchEnd
                || retained[1].kind != IecRecordKind::BranchStart
        } {
            return Err(unsupported());
        }
        updated[frame.start + 17..frame.start + 21].copy_from_slice(&39u32.to_le_bytes());
        updated[frame.start + 29] = if upper_row { b.max(4) } else { b };
        updated[frame.start + 33..frame.start + 35]
            .copy_from_slice(&(retained.len() as u16).to_le_bytes());
    }
    for record in records.iter().rev().filter(|r| removed.contains(&r.offset)) {
        updated.drain(record.offset..record.end);
    }
    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    if continuing_scaffold(&verified, block.row_index, block.raw_x, block.pin_count)? != Some(b)
        || (verified.iec_circuit_graph().is_none()
            && !crate::writer::iec_preserves_outside_function_rows(
                program,
                &verified,
                block.group_index,
                block.row_index..=block.row_index + u16::from(block.pin_count),
            ))
        || verified
            .iec_function_blocks()
            .is_none_or(|v| v.len() + 1 != blocks.len())
        || verified
            .iec_function_operand_links()
            .is_none_or(|v| v.len() + usize::from(block.pin_count) != links.len())
        || verified
            .iec_function_references()
            .is_none_or(|v| v.len() + usize::from(block.pin_count) != refs.len())
    {
        return Err(unsupported());
    }
    Ok(updated)
}

/// Remove a scalar block while retaining its native branch scaffold and rows.
pub(crate) fn remove_branch(
    program: &LadderProgramData,
    offset: usize,
    expected_name: &str,
) -> Result<Vec<u8>, XgwxError> {
    if let Ok(data) = crate::iec_open_spine_comparison_write::remove(program, offset, expected_name)
    {
        return Ok(data);
    }
    let invalid = |reason| XgwxError::InvalidLadderEdit { reason };
    let block = program
        .iec_function_blocks()
        .ok_or(XgwxError::UnsupportedLadderLayout)?
        .into_iter()
        .find(|block| block.record_offset == offset)
        .ok_or_else(|| invalid("IEC branch function position changed"))?;
    if block.name.value != expected_name {
        return Err(invalid("IEC branch function name changed"));
    }
    let (_, _, count) = spec(expected_name)
        .filter(|_| expected_name != "WORD_TO_UDINT")
        .ok_or_else(|| invalid("unsupported IEC branch function"))?;
    if block.pin_count != count {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let at_top = records
        .iter()
        .filter(|r| r.group_index == block.group_index && r.row_index == block.row_index)
        .collect::<Vec<_>>();
    if (matches!(at_top.as_slice(), [end, start, _, _] if end.kind == IecRecordKind::BranchEnd && start.kind == IecRecordKind::BranchStart)
        || [4, 3].into_iter().any(|n| {
            at_top
                .get(..n)
                .is_some_and(|prefix| continuing_contact_prefix(program, prefix).is_some())
        }))
        && matches!(block.raw_x, 13 | 16 | 19)
    {
        return remove_continuing_scalar(program, offset, expected_name);
    }
    if matches!(block.raw_x, 13 | 16 | 19)
        && records.iter().any(|r| {
            r.group_index == block.group_index
                && r.row_index == block.row_index
                && matches!(
                    r.kind,
                    IecRecordKind::BranchEnd
                        | IecRecordKind::Contact(6)
                        | IecRecordKind::Contact(7)
                )
        })
    {
        return remove_completed_branch_tail(program, offset, expected_name);
    }
    let feed = records
        .iter()
        .find(|record| {
            record.kind == IecRecordKind::LongWire
                && record.row_index == block.row_index
                && record.group_index == block.group_index
                && record.end == block.record_offset
        })
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let boundary = program.data[feed.offset + 5]
        .checked_sub(1)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if !(3..=90).contains(&boundary)
        || !boundary.is_multiple_of(3)
        || program.data[feed.offset + 15].checked_add(3) != Some(block.raw_x)
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let links = program
        .iec_function_operand_links()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let refs = program
        .iec_function_references()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let mut removed = vec![block.record_offset, feed.offset];
    let mut operands = BTreeMap::new();
    for link in links
        .iter()
        .filter(|link| link.target_record_offset == block.record_offset)
    {
        let record = records
            .iter()
            .find(|record| record.offset == link.record_offset)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let strings = crate::internal::extract_utf16_marker_strings(
            &program.data[record.offset..record.end],
            false,
            true,
        );
        if strings.len() != 1 {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        operands.insert(link.ordinal, strings[0].value.clone());
        removed.push(record.offset);
    }
    removed.extend(
        refs.iter()
            .filter(|reference| reference.target_record_offset == block.record_offset)
            .map(|reference| reference.record_offset),
    );
    if operands.len() != usize::from(count)
        || removed.len() != 2 + 2 * usize::from(count)
        || operands.keys().copied().ne(1..=count)
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let last = block
        .row_index
        .checked_add(u16::from(count))
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let mut updated = program.data.clone();
    for row in rows.iter().filter(|row| {
        row.group_index == block.group_index && (block.row_index..=last).contains(&row.row_index)
    }) {
        let deleted = records
            .iter()
            .filter(|record| record.row_index == row.row_index && removed.contains(&record.offset))
            .count();
        let remaining = row
            .record_count
            .checked_sub(deleted as u16)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        updated[row.records_start - 2..row.records_start].copy_from_slice(&remaining.to_le_bytes());
        let continues = records.iter().any(|record| {
            record.group_index == row.group_index
                && record.row_index == row.row_index
                && record.kind == IecRecordKind::BranchStart
                && !removed.contains(&record.offset)
        });
        updated[row.start + 29] = if continues { boundary } else { boundary - 1 };
    }
    removed.sort_unstable();
    for offset in removed.into_iter().rev() {
        let record = records
            .iter()
            .find(|record| record.offset == offset)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        updated.drain(record.offset..record.end);
    }
    let mut scaffold = program.clone();
    scaffold.decoded_len = updated.len();
    scaffold.data = updated.clone();
    if branch_scaffold(&scaffold, block.row_index, block.raw_x, count)? != Some(boundary) {
        return Err(invalid(
            "IEC branch function shares rows with unrelated records",
        ));
    }
    // Reconstruction proves that only the block, its feed and its own pins
    // were removed; unrelated bytes and the group execution mode are unchanged.
    let rebuilt = insert(
        &scaffold,
        block.row_index,
        block.raw_x,
        expected_name,
        &operands.into_values().collect::<Vec<_>>(),
    )?;
    if rebuilt != program.data {
        return Err(invalid(
            "IEC branch function does not match the native removable shape",
        ));
    }
    Ok(updated)
}

/// Remove MOVE from the captured EQ result row, including its feed wire.
fn remove_result_move(
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
    if expected != "MOVE"
        || block.name.value != expected
        || block.raw_x != 19
        || block.opcode_family != 0x20
        || block.opcode != 0x76
        || block.pin_count != 2
        || block.instance.is_some()
        || block.row_index == 0
        || program.data[offset + 11..offset + 15] != [0, 2, 0, 0]
    {
        return Err(unsupported());
    }
    let eq = blocks
        .iter()
        .find(|b| {
            b.group_index == block.group_index
                && b.row_index + 1 == block.row_index
                && b.raw_x == 10
        })
        .ok_or_else(unsupported)?;
    if program.data[eq.record_offset + 11..eq.record_offset + 15] != [0; 4] {
        return Err(unsupported());
    }
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let feed = records
        .iter()
        .find(|r| {
            r.group_index == block.group_index
                && r.row_index == block.row_index
                && r.end == offset
                && r.kind == IecRecordKind::LongWire
        })
        .ok_or_else(unsupported)?;
    if program.data[feed.offset + 5] != 13
        || program.data[feed.offset + 15] != 16
        || program.data[feed.offset + 9..feed.offset + 15] != [0; 6]
    {
        return Err(unsupported());
    }
    let links = program
        .iec_function_operand_links()
        .ok_or_else(unsupported)?;
    let refs = program.iec_function_references().ok_or_else(unsupported)?;
    let owned_links = links
        .iter()
        .filter(|r| r.target_record_offset == offset)
        .collect::<Vec<_>>();
    let owned_refs = refs
        .iter()
        .filter(|r| r.target_record_offset == offset)
        .collect::<Vec<_>>();
    if owned_links.len() != 2 || owned_refs.len() != 2 {
        return Err(unsupported());
    }
    let mut removed = vec![offset, feed.offset];
    removed.extend(owned_links.iter().map(|r| r.record_offset));
    removed.extend(owned_refs.iter().map(|r| r.record_offset));
    let group = rows
        .iter()
        .filter(|r| r.group_index == block.group_index)
        .collect::<Vec<_>>();
    let first = group.first().ok_or_else(unsupported)?;
    let last = group.last().ok_or_else(unsupported)?;
    let mut replacement = program.data[first.start - 10..first.start].to_vec();
    for row in &group {
        let retained = records
            .iter()
            .filter(|r| {
                r.group_index == block.group_index
                    && r.row_index == row.row_index
                    && !removed.contains(&r.offset)
            })
            .collect::<Vec<_>>();
        let mut header = program.data[row.start..row.records_start].to_vec();
        header[33..35].copy_from_slice(&(retained.len() as u16).to_le_bytes());
        if row.row_index >= block.row_index {
            header[17..21].copy_from_slice(&39u32.to_le_bytes());
            header[29] = 10;
        }
        replacement.extend(header);
        for r in retained {
            let mut bytes = program.data[r.offset..r.end].to_vec();
            if r.offset == eq.record_offset {
                bytes[12] = 2;
            }
            replacement.extend(bytes);
        }
    }
    let mut verified = program.clone();
    verified
        .data
        .splice(first.start - 10..last.end, replacement);
    verified.decoded_len = verified.data.len();
    if comparison_result_scaffold(&verified, block.row_index, block.raw_x).is_none()
        || (verified.iec_circuit_graph().is_none()
            && !crate::writer::iec_preserves_groups_around_edit(
                program,
                &verified,
                block.group_index..block.group_index + 1,
                1,
            ))
        || verified
            .iec_function_blocks()
            .is_none_or(|b| b.len() + 1 != blocks.len())
    {
        return Err(unsupported());
    }
    Ok(verified.data)
}

/// Remove a staggered MOVE whose operand rows overlap an earlier function.
/// Native Delete retains the one-cell EN feed and every neighboring pin.
fn remove_staggered_move(
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
    if block.raw_x == 19 {
        return remove_result_move(program, offset, expected);
    }
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let group_rows = rows
        .iter()
        .filter(|r| r.group_index == block.group_index)
        .collect::<Vec<_>>();
    let first = group_rows.first().ok_or_else(unsupported)?;
    let last = group_rows.last().ok_or_else(unsupported)?;
    if expected != "MOVE"
        || block.name.value != expected
        || block.opcode_family != 0x20
        || block.opcode != 0x76
        || block.pin_count != 2
        || block.instance.is_some()
        || block.raw_x != 4
        || program.data[offset + 12] != 2
        || !matches!(block.row_index.checked_sub(first.row_index), Some(1 | 2))
        || last.row_index != block.row_index + 2
        || group_rows
            .iter()
            .enumerate()
            .any(|(i, r)| r.row_index != first.row_index + i as u16)
    {
        return Err(unsupported());
    }
    let neighbors = blocks
        .iter()
        .filter(|b| b.group_index == block.group_index && b.record_offset != offset)
        .collect::<Vec<_>>();
    if neighbors.is_empty()
        || neighbors
            .iter()
            .any(|b| b.row_index != first.row_index || b.raw_x < 7)
        || !neighbors
            .iter()
            .any(|b| b.raw_x == 16 && b.opcode_family == 0x20 && matches!(b.pin_count, 2 | 3))
        || records
            .iter()
            .filter(|r| r.group_index == block.group_index)
            .any(|r| {
                !matches!(
                    r.kind,
                    IecRecordKind::Contact(_)
                        | IecRecordKind::LongWire
                        | IecRecordKind::ShortWire
                        | IecRecordKind::FunctionBlock
                        | IecRecordKind::FunctionOperand
                        | IecRecordKind::LinkReference(_)
                )
            })
    {
        return Err(unsupported());
    }
    let feed = records
        .iter()
        .find(|r| {
            r.group_index == block.group_index
                && r.row_index == block.row_index
                && r.end == offset
                && r.kind == IecRecordKind::LongWire
        })
        .ok_or_else(unsupported)?;
    if program.data[feed.offset + 5] != 1
        || program.data[feed.offset + 15] != 1
        || program.data[feed.offset + 9..feed.offset + 15] != [0; 6]
    {
        return Err(unsupported());
    }
    let links = program
        .iec_function_operand_links()
        .ok_or_else(unsupported)?;
    let refs = program.iec_function_references().ok_or_else(unsupported)?;
    let mut removed = vec![offset];
    let owned_links = links
        .iter()
        .filter(|l| l.target_record_offset == offset)
        .collect::<Vec<_>>();
    let owned_refs = refs
        .iter()
        .filter(|r| r.target_record_offset == offset)
        .collect::<Vec<_>>();
    if owned_links.len() != 2 || owned_refs.len() != 2 {
        return Err(unsupported());
    }
    removed.extend(owned_links.iter().map(|l| l.record_offset));
    removed.extend(owned_refs.iter().map(|r| r.record_offset));
    let mut replacement = program.data[first.start - 10..first.start].to_vec();
    let mut retained_count = 0u16;
    let mut empty_tail = false;
    for row in &group_rows {
        let retained = records
            .iter()
            .filter(|r| {
                r.group_index == block.group_index
                    && r.row_index == row.row_index
                    && !removed.contains(&r.offset)
            })
            .collect::<Vec<_>>();
        if retained.is_empty() {
            empty_tail = true;
            continue;
        }
        if empty_tail {
            return Err(unsupported());
        }
        let mut header = program.data[row.start..row.records_start].to_vec();
        header[33..35].copy_from_slice(&(retained.len() as u16).to_le_bytes());
        if row.row_index >= block.row_index {
            header[17..21].copy_from_slice(&39u32.to_le_bytes());
            header[29] = retained
                .iter()
                .map(|r| program.data[r.offset + 5])
                .max()
                .unwrap();
        }
        replacement.extend(header);
        for record in retained {
            replacement.extend_from_slice(&program.data[record.offset..record.end]);
        }
        retained_count += 1;
    }
    replacement[8..10].copy_from_slice(&retained_count.to_le_bytes());
    let mut updated = program.data.clone();
    updated.splice(first.start - 10..last.end, replacement);
    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    if verified
        .iec_function_blocks()
        .is_none_or(|b| b.len() + 1 != blocks.len())
        || verified
            .iec_function_operand_links()
            .is_none_or(|l| l.len() + 2 != links.len())
        || verified
            .iec_function_references()
            .is_none_or(|r| r.len() + 2 != refs.len())
        || (verified.iec_circuit_graph().is_none()
            && !crate::writer::iec_preserves_groups_around_edit(
                program,
                &verified,
                block.group_index..block.group_index + 1,
                1,
            ))
    {
        return Err(unsupported());
    }
    Ok(updated)
}

/// Delete a comparison while retaining the staggered MOVE and its result feed.
fn remove_wired_comparison(
    program: &LadderProgramData,
    offset: usize,
) -> Result<Vec<u8>, XgwxError> {
    let unsupported = || XgwxError::UnsupportedLadderLayout;
    let blocks = program.iec_function_blocks().ok_or_else(unsupported)?;
    let block = blocks
        .iter()
        .find(|b| b.record_offset == offset)
        .ok_or_else(unsupported)?;
    if block.raw_x != 10
        || block.opcode_family != 0x28
        || block.pin_count != 3
        || block.instance.is_some()
        || spec(&block.name.value)
            .is_none_or(|(f, op, n)| f != 0x28 || op != block.opcode || n != 3)
        || program.data[offset + 11..offset + 15] != [0; 4]
    {
        return Err(unsupported());
    }
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    let group = rows
        .iter()
        .filter(|r| r.group_index == block.group_index)
        .collect::<Vec<_>>();
    let first = group.first().ok_or_else(unsupported)?;
    let last = group.last().ok_or_else(unsupported)?;
    if first.row_index != block.row_index || group.len() != 4 {
        return Err(unsupported());
    }
    let prefix = records
        .iter()
        .filter(|r| r.group_index == block.group_index && r.row_index == block.row_index)
        .collect::<Vec<_>>();
    if !wired_comparison_prefix(program, &prefix)
        || prefix.last().is_none_or(|r| r.offset != offset)
    {
        return Err(unsupported());
    }
    let links = program
        .iec_function_operand_links()
        .ok_or_else(unsupported)?;
    let refs = program.iec_function_references().ok_or_else(unsupported)?;
    let owned = links
        .iter()
        .filter(|l| l.target_record_offset == offset)
        .collect::<Vec<_>>();
    let references = refs
        .iter()
        .filter(|r| r.target_record_offset == offset)
        .collect::<Vec<_>>();
    if owned.len() != 2 || owned.iter().any(|l| l.is_output) || references.len() != 3 {
        return Err(unsupported());
    }
    let mut removed = prefix.iter().map(|r| r.offset).collect::<Vec<_>>();
    removed.extend(owned.iter().map(|l| l.record_offset));
    removed.extend(references.iter().map(|r| r.record_offset));
    let mut replacement = program.data[first.start - 10..first.start].to_vec();
    replacement[8..10].copy_from_slice(&3u16.to_le_bytes());
    for row in group.iter().skip(1) {
        let retained = records
            .iter()
            .filter(|r| {
                r.group_index == block.group_index
                    && r.row_index == row.row_index
                    && !removed.contains(&r.offset)
            })
            .collect::<Vec<_>>();
        if retained.is_empty() {
            return Err(unsupported());
        }
        let mut header = program.data[row.start..row.records_start].to_vec();
        header[17..21].copy_from_slice(&39u32.to_le_bytes());
        header[29] = retained
            .iter()
            .map(|r| program.data[r.offset + 5])
            .max()
            .unwrap();
        header[33..35].copy_from_slice(&(retained.len() as u16).to_le_bytes());
        replacement.extend(header);
        for r in retained {
            replacement.extend_from_slice(&program.data[r.offset..r.end]);
        }
    }
    let mut verified = program.clone();
    verified
        .data
        .splice(first.start - 10..last.end, replacement);
    verified.decoded_len = verified.data.len();
    if wired_comparison_scaffold(&verified, block.row_index, block.raw_x).is_none()
        || (verified.iec_circuit_graph().is_none()
            && !crate::writer::iec_preserves_groups_around_edit(
                program,
                &verified,
                block.group_index..block.group_index + 1,
                1,
            ))
        || verified
            .iec_function_blocks()
            .is_none_or(|b| b.len() + 1 != blocks.len())
    {
        return Err(unsupported());
    }
    Ok(verified.data)
}

/// Remove a MOVE fed by two parallel contacts, keeping both branch boundaries.
fn remove_parallel_contact_move(
    program: &LadderProgramData,
    offset: usize,
) -> Result<Vec<u8>, XgwxError> {
    let unsupported = || XgwxError::UnsupportedLadderLayout;
    let blocks = program.iec_function_blocks().ok_or_else(unsupported)?;
    let block = blocks
        .iter()
        .find(|b| b.record_offset == offset)
        .ok_or_else(unsupported)?;
    if block.name.value != "MOVE"
        || block.raw_x != 19
        || program.data.get(offset..block.record_end)
            != Some(body("MOVE", block.row_index, 19).as_slice())
        || blocks
            .iter()
            .filter(|b| b.group_index == block.group_index)
            .count()
            != 1
    {
        return Err(unsupported());
    }
    let row = block.row_index;
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let group = rows
        .iter()
        .filter(|r| r.group_index == block.group_index)
        .collect::<Vec<_>>();
    if group.len() != 3
        || group
            .iter()
            .enumerate()
            .any(|(i, r)| r.row_index != row + i as u16)
        || program.data[group[0].start - 6..group[0].start - 2] != [0; 4]
    {
        return Err(unsupported());
    }
    let top = records
        .iter()
        .filter(|r| r.group_index == block.group_index && r.row_index == row)
        .collect::<Vec<_>>();
    if top.len() < 6
        || top.last().is_none_or(|r| r.offset != offset)
        || !parallel_contact_prefix(program, &top[..4], row)
    {
        return Err(unsupported());
    }
    let mut next = 7u8;
    let feeds = &top[4..top.len() - 1];
    for feed in feeds {
        if program.data[feed.offset + 5] != next
            || program.data[feed.offset + 9..feed.offset + 15] != [0; 6]
        {
            return Err(unsupported());
        }
        next = match feed.kind {
            IecRecordKind::ShortWire => next.checked_add(3),
            IecRecordKind::LongWire
                if program.data[feed.offset + 15] >= next
                    && (program.data[feed.offset + 15] - next).is_multiple_of(3) =>
            {
                program.data[feed.offset + 15].checked_add(3)
            }
            _ => None,
        }
        .ok_or_else(unsupported)?;
    }
    if next != 19 {
        return Err(unsupported());
    }
    let links = program
        .iec_function_operand_links()
        .ok_or_else(unsupported)?;
    let references = program.iec_function_references().ok_or_else(unsupported)?;
    let owned_links = links
        .iter()
        .filter(|l| l.target_record_offset == offset)
        .collect::<Vec<_>>();
    let owned_refs = references
        .iter()
        .filter(|r| r.target_record_offset == offset)
        .collect::<Vec<_>>();
    if owned_links.len() != 2 || owned_refs.len() != 2 {
        return Err(unsupported());
    }
    let mut removed = vec![offset];
    removed.extend(feeds.iter().map(|r| r.offset));
    removed.extend(owned_links.iter().map(|l| l.record_offset));
    removed.extend(owned_refs.iter().map(|r| r.record_offset));
    let bottom = records
        .iter()
        .filter(|r| {
            r.group_index == block.group_index
                && r.row_index == row + 1
                && !removed.contains(&r.offset)
        })
        .collect::<Vec<_>>();
    if !parallel_contact_bottom(program, &bottom, row)
        || records.iter().any(|r| {
            r.group_index == block.group_index
                && r.row_index == row + 2
                && !removed.contains(&r.offset)
        })
    {
        return Err(unsupported());
    }
    let mut replacement = program.data[group[0].start - 10..group[0].start].to_vec();
    replacement[8..10].copy_from_slice(&2u16.to_le_bytes());
    for (index, retained) in [&top[..4], bottom.as_slice()].into_iter().enumerate() {
        let mut header = program.data[group[index].start..group[index].records_start].to_vec();
        header[17..21].copy_from_slice(&39u32.to_le_bytes());
        header[29] = if index == 0 { 6 } else { 5 };
        header[33..35].copy_from_slice(&(retained.len() as u16).to_le_bytes());
        replacement.extend(header);
        for r in retained {
            replacement.extend_from_slice(&program.data[r.offset..r.end]);
        }
    }
    let mut verified = program.clone();
    verified
        .data
        .splice(group[0].start - 10..group[2].end, replacement);
    verified.decoded_len = verified.data.len();
    if !parallel_contact_scaffold(&verified, row, 19, 2)
        || (verified.iec_circuit_graph().is_none()
            && !crate::writer::iec_preserves_groups_around_edit(
                program,
                &verified,
                block.group_index..block.group_index + 1,
                1,
            ))
        || verified
            .iec_function_blocks()
            .is_none_or(|b| b.len() + 1 != blocks.len())
    {
        return Err(unsupported());
    }
    Ok(verified.data)
}

/// Delete one body from a horizontal scalar chain with shared pin rows.
/// Native Delete retains intermediate wires and trims wires beyond the new tail.
pub(crate) fn remove_chain(
    program: &LadderProgramData,
    offset: usize,
    expected: &str,
) -> Result<Vec<u8>, XgwxError> {
    let invalid = |reason| XgwxError::InvalidLadderEdit { reason };
    let blocks = program
        .iec_function_blocks()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let block = blocks
        .iter()
        .find(|b| b.record_offset == offset)
        .ok_or_else(|| invalid("scalar chain target changed"))?;
    if block.name.value != expected {
        return Err(invalid("scalar chain function name changed"));
    }
    if let Ok(data) = crate::iec_chain_comparison_write::remove(program, offset, expected) {
        return Ok(data);
    }
    if expected == "MOVE" && block.raw_x == 16 {
        if let Ok(data) = crate::iec_upper_contact_move_write::remove(program, offset) {
            return Ok(data);
        }
        if let Ok(data) = crate::iec_staggered_move_write::remove_upper(program, offset) {
            return Ok(data);
        }
    }
    if expected == "MOVE" && block.raw_x == 19 {
        if let Ok(data) = crate::iec_contact_mesh_move_write::remove(program, offset) {
            return Ok(data);
        }
        if let Ok(data) = remove_parallel_contact_move(program, offset) {
            return Ok(data);
        }
    }
    if conversion_types(expected).is_some() && [10, 19].contains(&block.raw_x) {
        if let Ok(data) = crate::iec_conversion_pair_write::remove(program, offset) {
            return Ok(data);
        }
    }
    if expected == "TON" {
        return if block.raw_x == 10 {
            crate::iec_conversion_pair_write::remove_timer(program, offset)
        } else if block.raw_x == 22 {
            crate::iec_long_feed_timer_write::remove(program, offset)
        } else {
            crate::iec_connected_timer_write::remove(program, offset)
        };
    }
    if expected == "MOVE"
        && matches!(block.raw_x, 16 | 19)
        && blocks
            .iter()
            .filter(|b| b.group_index == block.group_index)
            .count()
            == 1
    {
        if let Ok(data) = remove_completed_branch_tail(program, offset, expected) {
            return Ok(data);
        }
    }
    if block.opcode_family == 0x28 && block.raw_x == 7 {
        if let Ok(data) = crate::iec_coil_comparison_write::remove(program, offset) {
            return Ok(data);
        }
    }
    if block.opcode_family == 0x28 && [4, 7, 13, 16].contains(&block.raw_x) {
        if let Ok(data) = crate::iec_paired_comparison_write::remove(program, offset) {
            return Ok(data);
        }
    }
    if block.opcode_family == 0x28
        && block.raw_x == 10
        && blocks.iter().any(|b| {
            b.group_index == block.group_index
                && b.row_index == block.row_index + 1
                && b.raw_x == 19
        })
    {
        return remove_wired_comparison(program, offset);
    }
    if blocks
        .iter()
        .any(|b| b.group_index == block.group_index && b.row_index < block.row_index)
    {
        return remove_staggered_move(program, offset, expected);
    }
    let group = blocks
        .iter()
        .filter(|b| b.group_index == block.group_index)
        .collect::<Vec<_>>();
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let group_rows = rows
        .iter()
        .filter(|r| r.group_index == block.group_index)
        .collect::<Vec<_>>();
    let top_records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let prefix = top_records
        .iter()
        .filter(|r| r.group_index == block.group_index && r.row_index == block.row_index)
        .collect::<Vec<_>>();
    let trigger_tail = group.len() == 2
        && block.raw_x == 19
        && program.data[offset + 11..offset + 15] == [0, 2, 0, 0]
        && block.instance.is_none()
        && matches!(block.pin_count, 2 | 3)
        && conversion_types(expected).is_none()
        && spec(expected).is_some_and(|(f, op, count)| {
            f == block.opcode_family && op == block.opcode && count == block.pin_count
        })
        && prefix.len() == 6
        && trigger_prefix(program, &prefix[..4])
        && prefix[4].kind == IecRecordKind::LongWire
        && program.data[prefix[4].offset + 5] == 13
        && program.data[prefix[4].offset + 15] == 16
        && program.data[prefix[4].offset + 9..prefix[4].offset + 15] == [0; 6]
        && prefix[5].offset == offset
        && program.data[group_rows[0].start - 6..group_rows[0].start - 2] == [0; 4];
    if group.len() < 2
        || !(3..=4).contains(&group_rows.len())
        || (!trigger_tail
            && group.iter().any(|b| {
                b.opcode_family != 0x20
                    || b.instance.is_some()
                    || !matches!(b.pin_count, 2 | 3)
                    || b.row_index != block.row_index
            }))
        || group_rows
            .iter()
            .enumerate()
            .any(|(i, r)| r.row_index != block.row_index + i as u16)
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if records
        .iter()
        .filter(|r| r.group_index == block.group_index)
        .any(|r| {
            if r.row_index == block.row_index {
                !matches!(
                    r.kind,
                    IecRecordKind::ShortWire
                        | IecRecordKind::LongWire
                        | IecRecordKind::FunctionBlock
                ) && !(trigger_tail && r.offset == prefix[0].offset)
            } else {
                !matches!(
                    r.kind,
                    IecRecordKind::FunctionOperand | IecRecordKind::LinkReference(_)
                )
            }
        })
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let links = program
        .iec_function_operand_links()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let refs = program
        .iec_function_references()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let mut removed = vec![offset];
    removed.extend(
        links
            .iter()
            .filter(|l| l.target_record_offset == offset)
            .map(|l| l.record_offset),
    );
    removed.extend(
        refs.iter()
            .filter(|r| r.target_record_offset == offset)
            .map(|r| r.record_offset),
    );
    let owned_links = links
        .iter()
        .filter(|l| l.target_record_offset == offset)
        .count();
    let owned_refs = refs
        .iter()
        .filter(|r| r.target_record_offset == offset)
        .count();
    if owned_links != usize::from(block.pin_count)
        || owned_refs != usize::from(block.pin_count)
        || removed.len() != 1 + owned_links + owned_refs
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let last = group
        .iter()
        .filter(|b| b.record_offset != offset)
        .map(|b| b.raw_x)
        .max()
        .unwrap();
    removed.extend(
        records
            .iter()
            .filter(|r| {
                r.group_index == block.group_index
                    && r.row_index == block.row_index
                    && matches!(r.kind, IecRecordKind::ShortWire | IecRecordKind::LongWire)
                    && program.data[r.offset + 5] > last
            })
            .map(|r| r.offset),
    );
    let mut updated = program.data.clone();
    // Native Delete marks the surviving chain tail as terminal.
    let tail = group
        .iter()
        .find(|b| b.record_offset != offset && b.raw_x == last)
        .unwrap();
    if !matches!(updated[tail.record_offset + 12], 0 | 2) {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    updated[tail.record_offset + 12] = 2;
    let mut retained_rows = Vec::new();
    let mut empty_tail = false;
    for row in &group_rows {
        let retained = records
            .iter()
            .filter(|r| {
                r.group_index == row.group_index
                    && r.row_index == row.row_index
                    && !removed.contains(&r.offset)
            })
            .collect::<Vec<_>>();
        if retained.is_empty() {
            empty_tail = true;
            continue;
        }
        // Only trailing empty body rows disappear; never compact an interior gap.
        if empty_tail {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let mut header = updated[row.start..row.records_start].to_vec();
        if trigger_tail {
            header[17..21].copy_from_slice(
                &(if row.row_index == block.row_index {
                    50u32
                } else {
                    39
                })
                .to_le_bytes(),
            );
        }
        header[33..35].copy_from_slice(&(retained.len() as u16).to_le_bytes());
        header[29] = retained
            .iter()
            .map(|r| program.data[r.offset + 5])
            .max()
            .unwrap();
        for record in retained {
            header.extend_from_slice(&updated[record.offset..record.end]);
        }
        retained_rows.push(header);
    }
    let first = group_rows[0].start - 10;
    let end = group_rows.last().unwrap().end;
    let mut replacement = updated[first..first + 10].to_vec();
    replacement[8..10].copy_from_slice(&(retained_rows.len() as u16).to_le_bytes());
    for row in retained_rows {
        replacement.extend(row);
    }
    updated.splice(first..end, replacement);
    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    if verified
        .iec_function_blocks()
        .is_none_or(|b| b.len() + 1 != blocks.len())
        || verified
            .iec_function_operand_links()
            .is_none_or(|l| l.len() + owned_links != links.len())
        || verified
            .iec_function_references()
            .is_none_or(|r| r.len() + owned_refs != refs.len())
        || (verified.iec_circuit_graph().is_none()
            && !crate::writer::iec_preserves_groups_around_edit(
                program,
                &verified,
                block.group_index..block.group_index + 1,
                1,
            ))
        || (trigger_tail && !trigger_scaffold(&verified, block.row_index, block.raw_x)?)
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(updated)
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
            (
                "INT_TO_UDINT",
                include_bytes!("../fixtures/function-bodies/iec_int_to_udint.bin").as_slice(),
            ),
            (
                "UDINT_TO_TIME",
                include_bytes!("../fixtures/function-bodies/iec_udint_to_time.bin").as_slice(),
            ),
            (
                "TIME_TO_UDINT",
                include_bytes!("../fixtures/function-bodies/iec_time_to_udint.bin").as_slice(),
            ),
            (
                "UDINT_TO_INT",
                include_bytes!("../fixtures/function-bodies/iec_udint_to_int.bin").as_slice(),
            ),
        ] {
            let row = u16::from_le_bytes([native[6], native[7]]) / 4;
            let mut generated = body(name, row, native[5]);
            // Tail state is assigned by placement, independently of pin descriptors.
            assert_eq!(generated[12], 2);
            assert!(matches!(native[12], 0 | 2));
            generated[12] = native[12];
            assert_eq!(generated, native, "{name}");
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
            assert!(remove_branch(&program, blocks[0].record_offset, name).is_err());
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

    #[test]
    fn disabled_scalar_bodies_and_expressions_decode_without_enabling_the_network() {
        for name in [
            "MOVE", "ADD", "SUB", "MUL", "DIV", "EQ", "GT", "GE", "LT", "LE",
        ] {
            let mut program = empty_program();
            program.data[12..16].copy_from_slice(&1u32.to_le_bytes());
            let count = spec(name).unwrap().2;
            let operands = (0..count)
                .map(|ordinal| {
                    if ordinal + 1 == count {
                        "%MW100".to_string()
                    } else {
                        "1".to_string()
                    }
                })
                .collect::<Vec<_>>();
            program.data = insert(&program, 0, 7, name, &operands).unwrap();
            assert_eq!(&program.data[12..16], &1u32.to_le_bytes());
            let blocks = program.iec_function_blocks().unwrap();
            assert_eq!(blocks.len(), 1, "{name}");
            assert_eq!(program.data[blocks[0].record_offset + 11], 4);
            assert_eq!(
                program.iec_function_operand_links().unwrap().len(),
                usize::from(count)
            );
            assert!(program.iec_circuit_graph().is_some());
            for record in program.iec_record_frames().unwrap() {
                if record.kind == IecRecordKind::FunctionOperand {
                    assert_eq!(program.data[record.offset + 11], 4);
                }
            }
        }
    }
}
