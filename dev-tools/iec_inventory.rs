//! Inspect captured IEC LD record and function coverage in an XGWX workspace.
use std::collections::BTreeMap;
use std::error::Error;
use xgwx::{IecRecordKind, XgwxDocument};

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: iec_inventory <file.xgwx>")?;
    let document = XgwxDocument::from_path(path)?;
    let detail = std::env::args().any(|arg| arg == "--detail");
    let record_detail = std::env::args().any(|arg| arg == "--records");
    let row_detail = std::env::args().any(|arg| arg == "--rows");
    let site_detail = std::env::args().any(|arg| arg == "--sites");
    let mut seen_functions = std::collections::BTreeSet::new();
    for (index, program) in document.ladder_programs().into_iter().enumerate() {
        let program = program?;
        if program.project_type != Some(2) {
            continue;
        }
        let rows = program.iec_row_frames().ok_or("IEC row framing failed")?;
        let records = program
            .iec_record_frames()
            .ok_or("IEC record framing failed")?;
        let blocks = program
            .iec_function_blocks()
            .ok_or("IEC function parsing failed")?;
        let references = program
            .iec_function_references()
            .ok_or("IEC function link parsing failed")?;
        let operands = program
            .iec_function_operand_links()
            .ok_or("IEC operand link parsing failed")?;
        let geometry = program
            .iec_geometry()
            .ok_or("IEC geometry parsing failed")?;
        let graph = program
            .iec_circuit_graph()
            .ok_or("IEC circuit graph validation failed")?;
        let mut kinds = BTreeMap::<String, usize>::new();
        for record in &records {
            let kind = match record.kind {
                IecRecordKind::Contact(code) => format!("Contact {code:02x}"),
                IecRecordKind::Coil(code) => format!("Coil {code:02x}"),
                IecRecordKind::LinkReference(code) => format!("Link {code:02x}"),
                other => format!("{other:?}"),
            };
            *kinds.entry(kind).or_default() += 1;
        }
        let mut functions = BTreeMap::<String, usize>::new();
        for block in &blocks {
            *functions.entry(block.name.value.clone()).or_default() += 1;
            if detail && seen_functions.insert(block.name.value.clone()) {
                let bytes = &program.data[block.record_offset..block.record_end];
                println!(
                    "  function {} at 0x{:x}: family {:02x}, opcode {:04x}, pins {}, bytes {}, header {:02x?}",
                    block.name.value,
                    block.record_offset,
                    block.opcode_family,
                    block.opcode,
                    block.pin_count,
                    bytes.len(),
                    &bytes[..bytes.len().min(40)]
                );
                println!(
                    "    strings: {:?}",
                    block
                        .field_strings
                        .iter()
                        .map(|item| (item.offset - block.record_offset, &item.value))
                        .collect::<Vec<_>>()
                );
                let mut marker = 0;
                let mut markers = Vec::new();
                while marker + 4 <= bytes.len() {
                    if bytes[marker..].starts_with(&[0xff, 0xfe, 0xff]) {
                        let len = bytes[marker + 3] as usize;
                        let end = marker + 4 + len * 2;
                        if end <= bytes.len() {
                            let units = bytes[marker + 4..end]
                                .chunks_exact(2)
                                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                                .collect::<Vec<_>>();
                            if let Ok(value) = String::from_utf16(&units) {
                                markers.push((marker, end, value));
                                marker = end;
                                continue;
                            }
                        }
                    }
                    marker += 1;
                }
                println!("    markers: {markers:?}");
                for (line, chunk) in bytes.chunks(16).enumerate() {
                    println!(
                        "    {:04x}: {}",
                        line * 16,
                        chunk
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    );
                }
            }
            if detail {
                let block_references = references
                    .iter()
                    .filter(|reference| reference.target_record_offset == block.record_offset)
                    .map(|reference| {
                        let operand = operands
                            .iter()
                            .find(|operand| {
                                operand.target_record_offset == block.record_offset
                                    && operand.ordinal == reference.ordinal
                            })
                            .and_then(|operand| {
                                program.strings.iter().find(|string| {
                                    string.offset >= operand.record_offset
                                        && string.offset
                                            < records
                                                .iter()
                                                .find(|record| {
                                                    record.offset == operand.record_offset
                                                })
                                                .map_or(operand.record_offset, |record| record.end)
                                })
                            })
                            .map(|string| string.value.as_str());
                        (
                            reference.ordinal,
                            reference.row_index,
                            reference.raw_x,
                            reference.is_output,
                            operand,
                        )
                    })
                    .collect::<Vec<_>>();
                println!(
                    "    instance {} at group {} L{} x{}: {:?}",
                    block.name.value,
                    block.group_index,
                    block.row_index,
                    block.raw_x,
                    block_references
                );
            }
        }
        println!(
            "program {index}: {} rows, {} records",
            rows.len(),
            records.len()
        );
        if row_detail {
            for row in &rows {
                let header = &program.data[row.start..row.records_start];
                println!(
                    "  row-header group {} L{}: cache={} primary=({},L{}) secondary=({},L{}) tail=({},L{}) records={}",
                    row.group_index,
                    row.row_index,
                    header[17],
                    header[21],
                    u16::from_le_bytes([header[22], header[23]]) / 4,
                    header[25],
                    u16::from_le_bytes([header[26], header[27]]) / 4,
                    header[29],
                    u16::from_le_bytes([header[30], header[31]]) / 4,
                    row.record_count,
                );
            }
        }
        println!("  records: {kinds:?}");
        println!("  functions: {functions:?}");
        if site_detail {
            for site in program
                .iec_terminal_function_deletion_sites()
                .unwrap_or_default()
            {
                let block = blocks
                    .iter()
                    .find(|block| block.record_offset == site.block_offset)
                    .ok_or("terminal function block missing")?;
                println!(
                    "  terminal-delete group {} L{} x{} {} block={} wire={}",
                    site.group_index,
                    site.row_index,
                    site.raw_x,
                    block.name.value,
                    site.block_offset,
                    site.wire_offset
                );
            }
            for site in program
                .iec_terminal_function_insertion_sites()
                .unwrap_or_default()
            {
                println!(
                    "  terminal-insert group {} L{} x{} contact={} insertion={}",
                    site.group_index,
                    site.row_index,
                    site.raw_x,
                    site.contact_offset,
                    site.insertion_offset
                );
            }
            for site in program
                .iec_standalone_function_deletion_sites()
                .unwrap_or_default()
            {
                let block = blocks
                    .iter()
                    .find(|block| block.record_offset == site.block_offset)
                    .ok_or("standalone function block missing")?;
                println!(
                    "  standalone-delete group {} L{} x{} {} block={} wire={}",
                    site.group_index,
                    site.row_index,
                    site.raw_x,
                    block.name.value,
                    site.block_offset,
                    site.wire_offset
                );
            }
            for site in program
                .iec_function_cell_deletion_sites()
                .unwrap_or_default()
            {
                let block = blocks
                    .iter()
                    .find(|block| block.record_offset == site.block_offset)
                    .ok_or("function cell block missing")?;
                println!(
                    "  function-cell-delete group {} L{} x{} {} block={}",
                    site.group_index,
                    site.row_index,
                    site.raw_x,
                    block.name.value,
                    site.block_offset
                );
            }
            for site in program
                .iec_connected_arithmetic_deletion_sites()
                .unwrap_or_default()
            {
                let name = blocks
                    .iter()
                    .find(|block| block.record_offset == site.block_offset)
                    .ok_or("connected arithmetic block missing")?
                    .name
                    .value
                    .as_str();
                println!(
                    "  connected-arithmetic-delete group {} L{} x{} {} block={} wire={}",
                    site.group_index,
                    site.row_index,
                    site.raw_x,
                    name,
                    site.block_offset,
                    site.wire_offset
                );
            }
        }
        if detail {
            for row in &rows {
                let parts = records
                    .iter()
                    .filter(|record| {
                        record.group_index == row.group_index && record.row_index == row.row_index
                    })
                    .collect::<Vec<_>>();
                if parts.len() >= 5
                    && parts.len() % 2 == 1
                    && parts.iter().enumerate().all(|(index, part)| {
                        if index == parts.len() - 1 {
                            matches!(part.kind, IecRecordKind::Coil(_))
                        } else if index % 2 == 1 {
                            part.kind == IecRecordKind::LongWire
                        } else {
                            matches!(part.kind, IecRecordKind::Contact(_))
                        }
                    })
                {
                    println!(
                        "  linear row L{}: header x {}, contacts {:?}, wires {:?}",
                        row.row_index,
                        program.data[row.start + 29],
                        parts
                            .iter()
                            .filter(|part| matches!(part.kind, IecRecordKind::Contact(_)))
                            .map(|part| program.data[part.offset + 5])
                            .collect::<Vec<_>>(),
                        parts
                            .iter()
                            .filter(|part| part.kind == IecRecordKind::LongWire)
                            .map(|part| (
                                program.data[part.offset + 5],
                                program.data[part.offset + 15]
                            ))
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
        println!(
            "  links: {} references, {} operands; geometry: {} wires, {} branches; graph: {} edges, {} components",
            references.len(),
            operands.len(),
            geometry.horizontal.len(),
            geometry.vertical.len(),
            graph.edges.len(),
            graph.power_components.len(),
        );
        if record_detail {
            for row in &rows {
                let parts = records
                    .iter()
                    .filter(|record| {
                        record.group_index == row.group_index && record.row_index == row.row_index
                    })
                    .map(|record| {
                        let bytes = &program.data[record.offset..record.end];
                        match record.kind {
                            IecRecordKind::LongWire => {
                                format!("LongWire({}..{})", bytes[5], bytes[15])
                            }
                            IecRecordKind::BranchStart => format!("BranchStart({})", bytes[7]),
                            IecRecordKind::BranchEnd => format!("BranchEnd({})", bytes[5]),
                            IecRecordKind::FunctionBlock => format!("FunctionBlock({})", bytes[5]),
                            IecRecordKind::LinkReference(code) => format!(
                                "Link{code:02x}(ordinal={},x={},target=L{})",
                                bytes[0],
                                bytes[5],
                                u16::from_le_bytes([bytes[6], bytes[7]]) / 4
                            ),
                            kind => format!("{kind:?}({})", bytes[5]),
                        }
                    })
                    .collect::<Vec<_>>();
                println!(
                    "  group {} L{}: {}",
                    row.group_index,
                    row.row_index,
                    parts.join(" ")
                );
            }
        }
        if detail {
            for connection in &geometry.vertical {
                println!(
                    "  branch group {} L{}..L{} x {} records 0x{:x}/0x{:x}",
                    connection.group_index,
                    connection.start_row_index,
                    connection.end_row_index,
                    connection.x,
                    connection.start_offset,
                    connection.end_offset,
                );
            }
        }
    }
    Ok(())
}
