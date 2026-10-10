//! Remove the captured program 0 L42-L43 contact branch through the guarded writer.
use std::{env, error::Error, fs};
use xgwx::{IecRecordKind, XgwxDocument};

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_branch_wire_delete_acceptance <source> <output>".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    document.edit_iec_ld_branch_segment(0, 22, 42, 43, 3, true, false)?;
    fs::write(output, document.to_bytes()?)?;
    let parsed = XgwxDocument::from_path(output)?;
    let program = parsed.ladder_programs().remove(0)?;
    let row = program
        .iec_row_frames()
        .ok_or("row framing failed")?
        .into_iter()
        .find(|row| row.group_index == 22 && row.row_index == 42)
        .ok_or("edited row missing")?;
    let kinds = program
        .iec_record_frames()
        .ok_or("record framing failed")?
        .into_iter()
        .filter(|record| record.group_index == 22)
        .map(|record| record.kind)
        .collect::<Vec<_>>();
    if row.record_count != 5
        || kinds
            != [
                IecRecordKind::Contact(6),
                IecRecordKind::LongWire,
                IecRecordKind::Contact(7),
                IecRecordKind::LongWire,
                IecRecordKind::Coil(14),
            ]
    {
        return Err(format!("unexpected edited row: {kinds:?}").into());
    }
    program.iec_circuit_graph().ok_or("invalid circuit graph")?;
    println!("PASS program 0 L42-L43 branch removal");
    Ok(())
}
