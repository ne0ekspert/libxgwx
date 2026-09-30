//! Validate a parallel-contact edit on an existing rung in the supplied XGI project.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let (source, output, kind) = match args.as_slice() {
        [source, output] => (source, output, "NO"),
        [source, output, kind] => (source, output, kind.as_str()),
        _ => return Err("usage: iec_original_parallel_acceptance <source> <output> [NO|NC|RISING|FALLING|NEGATED_RISING|NEGATED_FALLING]".into()),
    };
    let source_bytes = fs::read(source)?;
    let mut document = XgwxDocument::parse(&source_bytes)?;
    document.insert_iec_ld_parallel_contact_kind(5, 4, "%MX761", "%MX762", kind, "%MX760")?;
    fs::write(output, document.to_bytes()?)?;

    let mut restored = XgwxDocument::from_path(output)?;
    let program = restored.ladder_programs().remove(5)?;
    let top = program
        .iec_row_frames()
        .ok_or("IEC row framing failed")?
        .into_iter()
        .find(|row| row.row_index == 4)
        .ok_or("L4 missing")?;
    restored.edit_iec_ld_branch_segment(5, top.group_index, 4, 5, 3, true, false)?;
    let original = XgwxDocument::parse(&source_bytes)?;
    let original_programs = original
        .ladder_programs()
        .into_iter()
        .map(|program| program.map(|program| program.data))
        .collect::<Result<Vec<_>, _>>()?;
    let restored_programs = restored
        .ladder_programs()
        .into_iter()
        .map(|program| program.map(|program| program.data))
        .collect::<Result<Vec<_>, _>>()?;
    if restored_programs != original_programs
        || restored
            .iec_local_symbols()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            != original
                .iec_local_symbols()
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?
    {
        return Err("removing the new parallel branch did not restore the project payloads".into());
    }
    println!("PASS original program 5 L4 {kind} parallel contact and payload inverse");
    Ok(())
}
