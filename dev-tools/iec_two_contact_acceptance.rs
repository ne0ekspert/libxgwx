//! Add two serial IEC contacts to the supplied project's program 0 L2 rung.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let source = args
        .next()
        .ok_or("usage: iec_two_contact_acceptance SOURCE OUTPUT")?;
    let output = args
        .next()
        .ok_or("usage: iec_two_contact_acceptance SOURCE OUTPUT")?;
    if args.next().is_some() {
        return Err("usage: iec_two_contact_acceptance SOURCE OUTPUT".into());
    }
    let mut document = XgwxDocument::from_path(source)?;
    let program = document
        .ladder_programs()
        .into_iter()
        .next()
        .ok_or("program 0 missing")??;
    let wire = program
        .iec_no_contact_insertion_sites()
        .ok_or("IEC insertion sites unavailable")?
        .into_iter()
        .find(|site| site.row_index == 2 && site.start_x == 4 && site.end_x == 91)
        .ok_or("program 0 L2 long wire not found")?;
    document.insert_iec_ld_contact(0, wire.wire_offset, 25, 4, 91, "NO", "ON")?;

    let program = document
        .ladder_programs()
        .into_iter()
        .next()
        .ok_or("program 0 missing")??;
    let wire = program
        .iec_no_contact_insertion_sites()
        .ok_or("second IEC insertion sites unavailable")?
        .into_iter()
        .find(|site| site.row_index == 2 && site.start_x == 28 && site.end_x == 91)
        .ok_or("remaining program 0 L2 long wire not found")?;
    document.insert_iec_ld_contact(0, wire.wire_offset, 49, 28, 91, "NC", "스위치_1")?;

    let program = document
        .ladder_programs()
        .into_iter()
        .next()
        .ok_or("program 0 missing")??;
    program
        .iec_circuit_graph()
        .ok_or("generated IEC circuit graph is invalid")?;
    document.write_to(&output)?;
    XgwxDocument::from_path(&output)?;
    println!("PASS program 0 L2 serial ON and 스위치_1 contacts: {output}");
    Ok(())
}
