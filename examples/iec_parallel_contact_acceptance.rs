//! Generate a native-shaped parallel contact for XG5000 acceptance.
use std::{env, error::Error, fs};
use xgwx::{IecRecordKind, XgwxDocument};

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_parallel_contact_acceptance <source> <output>".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    document.insert_iec_ld_parallel_contact(0, 31, "ON", "OFF", "OFF")?;
    fs::write(output, document.to_bytes()?)?;
    let parsed = XgwxDocument::from_path(output)?;
    let program = parsed.ladder_programs().remove(0)?;
    let rows = program.iec_row_frames().ok_or("row framing failed")?;
    let top = rows
        .iter()
        .find(|row| row.row_index == 31)
        .ok_or("top row missing")?;
    let bottom = rows
        .iter()
        .find(|row| row.row_index == 32 && row.group_index == top.group_index)
        .ok_or("bottom row missing")?;
    if top.record_count != 4 || bottom.record_count != 2 {
        return Err("wrong record counts".into());
    }
    let kinds = program
        .iec_record_frames()
        .ok_or("record framing failed")?
        .into_iter()
        .filter(|record| record.group_index == top.group_index)
        .map(|record| record.kind)
        .collect::<Vec<_>>();
    if kinds
        != [
            IecRecordKind::Contact(6),
            IecRecordKind::BranchStart,
            IecRecordKind::LongWire,
            IecRecordKind::Coil(14),
            IecRecordKind::Contact(6),
            IecRecordKind::BranchEnd,
        ]
    {
        return Err(format!("unexpected records: {kinds:?}").into());
    }
    program.iec_circuit_graph().ok_or("invalid circuit graph")?;
    let mut restored = XgwxDocument::from_path(output)?;
    restored.edit_iec_ld_branch_segment(0, top.group_index, 31, 32, 3, true, false)?;
    if restored.to_bytes()? != fs::read(source)? {
        return Err("removing the generated parallel row did not restore the source".into());
    }
    println!("PASS native-shaped parallel contact in program 0, L31-L32");
    Ok(())
}
