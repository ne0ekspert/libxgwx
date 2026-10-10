//! Dump one decoded IEC LD row and its enclosing native group header.
use std::error::Error;
use xgwx::XgwxDocument;

fn print_hex(label: &str, bytes: &[u8]) {
    println!("{label} ({} bytes)", bytes.len());
    for (line, chunk) in bytes.chunks(16).enumerate() {
        println!(
            "  {:04x}: {}",
            line * 16,
            chunk
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: iec_row_dump <file.xgwx> <program-index> <row-index>")?;
    let program_index = args
        .next()
        .ok_or("missing program index")?
        .parse::<usize>()?;
    let row_index = args.next().ok_or("missing row index")?.parse::<u16>()?;
    let document = XgwxDocument::from_path(path)?;
    let programs = document.ladder_programs();
    let program = programs
        .get(program_index)
        .ok_or("program index is out of range")?
        .as_ref()
        .map_err(|error| error.to_string())?;
    let rows = program.iec_row_frames().ok_or("IEC row framing failed")?;
    let row = rows
        .iter()
        .find(|row| row.row_index == row_index)
        .ok_or("row is not stored")?;
    let first_in_group = rows
        .iter()
        .find(|candidate| candidate.group_index == row.group_index)
        .ok_or("group is missing")?;
    println!(
        "program {program_index}, group {}, row L{row_index}, start 0x{:X}, records {}",
        row.group_index, row.start, row.record_count
    );
    if first_in_group.start == row.start {
        print_hex("group header", &program.data[row.start - 10..row.start]);
    }
    print_hex("row header", &program.data[row.start..row.records_start]);
    let Some(records) = program.iec_record_frames() else {
        print_hex(
            "unframed row records",
            &program.data[row.records_start..row.end],
        );
        return Ok(());
    };
    for (index, record) in records
        .iter()
        .filter(|record| record.group_index == row.group_index && record.row_index == row.row_index)
        .enumerate()
    {
        print_hex(
            &format!("record {index} {:?}", record.kind),
            &program.data[record.offset..record.end],
        );
    }
    Ok(())
}
