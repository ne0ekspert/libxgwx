//! Remove the native program 3 L17-L18 branch with an adjacent upper contact.
use std::{env, error::Error, fs};
use xgwx::{IecRecordKind, XgwxDocument};

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_direct_contact_branch_delete_acceptance <source> <output>".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    document.edit_iec_ld_branch_segment(3, 7, 17, 18, 3, true, false)?;
    fs::write(output, document.to_bytes()?)?;
    let parsed = XgwxDocument::from_path(output)?;
    let program = parsed.ladder_programs().remove(3)?;
    let row = program
        .iec_row_frames()
        .ok_or("row framing failed")?
        .into_iter()
        .find(|row| row.group_index == 7 && row.row_index == 17)
        .ok_or("edited row missing")?;
    let kinds = program
        .iec_record_frames()
        .ok_or("record framing failed")?
        .into_iter()
        .filter(|record| record.group_index == 7)
        .map(|record| record.kind)
        .collect::<Vec<_>>();
    if row.record_count != 4
        || kinds
            != [
                IecRecordKind::Contact(6),
                IecRecordKind::Contact(7),
                IecRecordKind::LongWire,
                IecRecordKind::Coil(14),
            ]
    {
        return Err(format!("unexpected edited row: {kinds:?}").into());
    }
    program.iec_circuit_graph().ok_or("invalid circuit graph")?;
    println!("PASS program 3 L17-L18 branch removal");
    Ok(())
}
