//! Add a parallel contact to a native SET-coil rung in the smart home project.
use std::{env, error::Error};
use xgwx::{IecRecordKind, XgwxDocument};

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_parallel_set_acceptance SOURCE OUTPUT".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    document.update_iec_ld_coil_kind(5, 555, "OUTPUT", "SET")?;
    let set_only = document.to_bytes()?;
    document.insert_iec_ld_parallel_contact_kind(5, 4, "%MX761", "%MX762", "NC", "%MX760")?;
    let program = document.ladder_programs().remove(5)?;
    let records = program
        .iec_record_frames()
        .ok_or("record framing failed")?
        .into_iter()
        .filter(|record| record.group_index == 4)
        .map(|record| record.kind)
        .collect::<Vec<_>>();
    if records
        != [
            IecRecordKind::Contact(6),
            IecRecordKind::BranchStart,
            IecRecordKind::LongWire,
            IecRecordKind::Coil(16),
            IecRecordKind::Contact(7),
            IecRecordKind::BranchEnd,
        ]
        || program.iec_circuit_graph().is_none()
    {
        return Err(format!("SET parallel network did not validate: {records:?}").into());
    }
    let mut restored = document.clone();
    restored.edit_iec_ld_branch_segment(5, 4, 4, 5, 3, true, false)?;
    if restored.to_bytes()? != set_only {
        return Err("removing the new branch did not restore the SET-coil rung".into());
    }
    document.write_to(output)?;
    println!("PASS program 5 SET-coil rung L4 gained an NC parallel contact at L5");
    Ok(())
}
