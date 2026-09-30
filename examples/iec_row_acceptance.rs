//! Generate and verify IEC implicit blank-row edits for native XG5000 acceptance.
use std::{env, error::Error, path::Path};
use xgwx::XgwxDocument;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn generate(source: &Path, output: &Path, program_index: usize, after_row: u16) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    let before = document
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("program index is out of range")??;
    let before_rows = before.iec_row_frames().ok_or("IEC row framing failed")?;
    let before_max = u16::from_le_bytes(before.data[4..6].try_into()?);

    document.insert_iec_ld_blank_row(program_index, after_row)?;
    document.write_to(output)?;

    let reparsed = XgwxDocument::from_path(output)?;
    let after = reparsed
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("program index is out of range")??;
    let after_rows = after
        .iec_row_frames()
        .ok_or("edited IEC row framing failed")?;
    let after_max = u16::from_le_bytes(after.data[4..6].try_into()?);
    if after_max != before_max + 1 || after_rows.len() != before_rows.len() {
        return Err("blank-row edit changed the wrong row metadata".into());
    }
    for (before, after) in before_rows.iter().zip(&after_rows) {
        let expected = before.row_index + u16::from(before.row_index > after_row);
        if after.row_index != expected
            || after.group_index != before.group_index
            || after.record_count != before.record_count
        {
            return Err(
                format!("row shift mismatch at payload offset 0x{:X}", before.start).into(),
            );
        }
    }
    after
        .iec_circuit_graph()
        .ok_or("edited IEC circuit graph failed validation")?;
    println!(
        "PASS generated program {program_index} blank row after L{after_row}; row high-water mark {before_max} -> {after_max}"
    );
    Ok(())
}

fn verify(generated: &Path, resaved: &Path) -> Result<()> {
    let generated = XgwxDocument::from_path(generated)?;
    let resaved = XgwxDocument::from_path(resaved)?;
    let generated_programs = generated.ladder_programs();
    let resaved_programs = resaved.ladder_programs();
    if generated_programs.len() != resaved_programs.len() {
        return Err("resaved program count changed".into());
    }
    for (index, (generated, resaved)) in generated_programs
        .into_iter()
        .zip(resaved_programs)
        .enumerate()
    {
        if generated?.data != resaved?.data {
            return Err(format!("decoded ProgramData {index} changed during Save As").into());
        }
    }
    println!("PASS native Save As preserved every blank-row ProgramData payload");
    Ok(())
}

fn delete(source: &Path, output: &Path, program_index: usize, blank_row: u16) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    document.delete_iec_ld_blank_row(program_index, blank_row)?;
    document.write_to(output)?;
    let reparsed = XgwxDocument::from_path(output)?;
    reparsed.ladder_programs()[program_index]
        .as_ref()
        .map_err(|error| error.to_string())?
        .iec_circuit_graph()
        .ok_or("edited IEC circuit graph failed validation")?;
    println!("PASS deleted program {program_index} blank row L{blank_row}");
    Ok(())
}

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [command, source, output, program, after_row] if command == "generate" => generate(
            Path::new(source),
            Path::new(output),
            program.parse()?,
            after_row.parse()?,
        ),
        [command, generated, resaved] if command == "verify" => {
            verify(Path::new(generated), Path::new(resaved))
        }
        [command, source, output, program, blank_row] if command == "delete" => delete(
            Path::new(source),
            Path::new(output),
            program.parse()?,
            blank_row.parse()?,
        ),
        _ => Err("usage: iec_row_acceptance generate SOURCE OUTPUT PROGRAM AFTER_ROW | delete SOURCE OUTPUT PROGRAM BLANK_ROW | verify GENERATED RESAVED".into()),
    }
}
