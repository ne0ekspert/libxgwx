//! Native F5 short wires from scalar function BOOL outputs.
use crate::{IecRecordKind as K, LadderProgramData, XgwxError};

pub(crate) fn insert(
    program: &LadderProgramData,
    offset: usize,
    expected_name: &str,
    pin_name: &str,
    start_x: u8,
) -> Result<Vec<u8>, XgwxError> {
    let invalid = |reason| XgwxError::InvalidLadderEdit { reason };
    let unsupported = || XgwxError::UnsupportedLadderLayout;
    let blocks = program.iec_function_blocks().ok_or_else(unsupported)?;
    let block = blocks
        .iter()
        .find(|b| b.record_offset == offset)
        .ok_or_else(unsupported)?;
    if block.name.value != expected_name {
        return Err(invalid("IEC function changed since selection"));
    }
    let (family, opcode, count) =
        crate::iec_function_write::spec(expected_name).ok_or_else(unsupported)?;
    // Instance blocks and noncanonical bodies retain their existing guarded APIs.
    let mut canonical =
        crate::iec_function_write::body(expected_name, block.row_index, block.raw_x);
    canonical[12] = program.data[offset + 12];
    if block.opcode_family != family
        || block.opcode != opcode
        || block.pin_count != count
        || !matches!(canonical[12], 0 | 2)
        || program.data[offset..block.record_end] != canonical
    {
        return Err(unsupported());
    }
    let pin = std::iter::once(&block.control_output)
        .chain(block.pins.iter())
        .find(|p| p.name.value == pin_name && p.direction == crate::IecFunctionPinDirection::Output)
        .ok_or_else(unsupported)?;
    if pin.data_type_mask != 1 || pin.is_array || !matches!(pin_name, "ENO" | "OUT") {
        return Err(invalid(
            "Only BOOL output pins can drive a ladder wire; numeric OUT needs a destination",
        ));
    }
    if !(pin.raw_x..=91).contains(&start_x) || !(start_x - pin.raw_x).is_multiple_of(3) {
        return Err(invalid("IEC output wire requires an adjacent grid cell"));
    }
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    let row = rows
        .iter()
        .find(|r| r.group_index == block.group_index && r.row_index == pin.row_index)
        .ok_or_else(unsupported)?;
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let graph = program.iec_circuit_layout().ok_or_else(unsupported)?;
    // XG5000 forbids using comparison OUT and ENO as power-flow outputs together.
    let other_row = if pin_name == "OUT" {
        block.row_index
    } else {
        block.row_index + 1
    };
    if family == 0x28
        && graph.edges.iter().any(|e| {
            e.start.group_index == block.group_index
                && ((e.start.row_index == other_row && e.start.x == pin.raw_x - 1)
                    || (e.end.row_index == other_row && e.end.x == pin.raw_x - 1))
                && matches!(
                    e.kind,
                    crate::IecCircuitEdgeKind::HorizontalWire
                        | crate::IecCircuitEdgeKind::VerticalBranch
                )
        })
    {
        return Err(invalid(
            "Disconnect the other BOOL output before wiring ENO or OUT",
        ));
    }
    // Extensions must be contiguous short wires from this very pin, not an
    // arbitrary disconnected cell, another function's OUT, or a branch spine.
    for x in (pin.raw_x..start_x).step_by(3) {
        if !records.iter().any(|r| {
            r.group_index == row.group_index
                && r.row_index == row.row_index
                && r.kind == K::ShortWire
                && program.data[r.offset + 5] == x
        }) {
            return Err(invalid("Extend the output wire from its current endpoint"));
        }
    }
    let links = program
        .iec_function_operand_links()
        .ok_or_else(unsupported)?;
    let expression = if start_x == pin.raw_x {
        links
            .iter()
            .find(|l| {
                l.target_record_offset == offset
                    && l.is_output
                    && Some(l.ordinal) == pin.reference_ordinal
            })
            .map(|l| l.record_offset)
    } else {
        None
    };
    if graph.occupied_areas.iter().any(|a| {
        a.start_row_index <= pin.row_index
            && a.end_row_index >= pin.row_index
            && a.start_x <= start_x
            && a.end_x >= start_x
            && Some(a.record_offset) != expression
    }) {
        return Err(invalid("IEC output wire cell is occupied"));
    }
    let old = expression.and_then(|at| records.iter().find(|r| r.offset == at));
    let insertion = old.map_or_else(
        || {
            records
                .iter()
                .find(|r| {
                    r.group_index == row.group_index
                        && r.row_index == row.row_index
                        && program.data[r.offset + 5] > start_x
                })
                .map_or(row.end, |r| r.offset)
        },
        |r| r.offset,
    );
    let y = (pin.row_index * 4).to_le_bytes();
    let wire = [0xff, 1, 0, 0, 0, start_x, y[0], y[1], 0, 0, 0, 0, 0, 0, 0];
    let mut out = program.data.clone();
    out[offset + 12] = 0;
    out[row.start + 29] = start_x;
    let count = row
        .record_count
        .checked_add(u16::from(old.is_none()))
        .ok_or_else(unsupported)?;
    out[row.start + 33..row.start + 35].copy_from_slice(&count.to_le_bytes());
    out.splice(insertion..old.map_or(insertion, |r| r.end), wire);
    let mut verified = program.clone();
    verified.decoded_len = out.len();
    verified.data = out.clone();
    if verified.iec_circuit_layout().is_none() {
        return Err(unsupported());
    }
    Ok(out)
}
