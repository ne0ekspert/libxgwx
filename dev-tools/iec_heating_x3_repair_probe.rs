//! Probe the orphan x15 feed left after native L59 Delete Line.
use std::{env, error::Error};
use xgwx::{IecRecordKind, XgwxDocument};

fn main() -> Result<(), Box<dyn Error>> {
    let source = env::args()
        .nth(1)
        .ok_or("usage: iec_heating_x3_repair_probe SOURCE")?;
    let document = XgwxDocument::from_path(source)?;
    let mut program = document.ladder_programs().remove(6)?;
    let rows = program.iec_row_frames().ok_or("invalid rows")?;
    let records = program.iec_record_frames().ok_or("invalid records")?;
    let mut removed = Vec::new();
    for y in 54..=58u16 {
        let row = rows
            .iter()
            .find(|row| row.group_index == 13 && row.row_index == y)
            .ok_or("row missing")?;
        let row_records = records
            .iter()
            .filter(|record| record.group_index == 13 && record.row_index == y)
            .collect::<Vec<_>>();
        let mut count = 0u16;
        for record in row_records {
            let x = program.data[record.offset + 5];
            if (record.kind == IecRecordKind::BranchStart
                && y < 58
                && program.data[record.offset + 7] == 15)
                || (record.kind == IecRecordKind::BranchEnd && y > 54 && x == 15)
            {
                removed.push((record.offset, record.end));
                count += 1;
            }
        }
        let old = u16::from_le_bytes(program.data[row.start + 33..row.start + 35].try_into()?);
        program.data[row.start + 33..row.start + 35].copy_from_slice(&(old - count).to_le_bytes());
    }
    for (start, end) in removed.into_iter().rev() {
        program.data.drain(start..end);
    }
    program.decoded_len = program.data.len();
    println!(
        "rows {:?}, records {:?}, graph {:?}",
        program.iec_row_frames().map(|items| items.len()),
        program.iec_record_frames().map(|items| items.len()),
        program
            .iec_circuit_graph()
            .as_ref()
            .map(|graph| graph.power_components.len())
    );
    Ok(())
}
