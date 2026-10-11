//! Copy a direct-address IEC LD network between the original smart home programs.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn payloads(document: &XgwxDocument) -> Result<Vec<Vec<u8>>, xgwx::XgwxError> {
    document
        .program()
        .ladder_programs()
        .into_iter()
        .map(|program| program.map(|program| program.data))
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_cross_program_copy_acceptance <source.xgwx> <output.xgwx>".into());
    };
    let original_bytes = fs::read(source)?;
    let mut document = XgwxDocument::parse(&original_bytes)?;
    let source_group = document
        .program()
        .ladder_programs()
        .remove(5)?
        .iec_row_frames()
        .ok_or("source row framing failed")?
        .into_iter()
        .find(|row| row.row_index == 4)
        .ok_or("program 5 L4 missing")?
        .group_index;
    document
        .program_mut()
        .get(5)
        .and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .copy_group_to_program(source_group, 4, 6, 8)
        })?;
    fs::write(output, document.to_bytes()?)?;

    let original = XgwxDocument::parse(&original_bytes)?;
    let copied_payloads = payloads(&document)?;
    let original_payloads = payloads(&original)?;
    if copied_payloads
        .iter()
        .zip(&original_payloads)
        .enumerate()
        .any(|(index, (copied, original))| index != 6 && copied != original)
    {
        return Err("cross-program copy changed a program other than its destination".into());
    }
    let copied_row = document
        .program()
        .ladder_programs()
        .remove(6)?
        .iec_row_frames()
        .ok_or("destination row framing failed")?
        .into_iter()
        .find(|row| row.row_index == 8)
        .ok_or("copied L8 missing")?;
    document
        .program_mut()
        .get(6)
        .and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .delete_group(copied_row.group_index, 8)
        })?;
    if payloads(&document)? != original_payloads {
        return Err("deleting the copied network did not restore every program payload".into());
    }
    println!("PASS program 5 L4 copied to program 6 L8 and payload inverse");
    Ok(())
}
