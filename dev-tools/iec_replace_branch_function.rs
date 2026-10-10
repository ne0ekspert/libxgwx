//! Replace a captured branch scalar function using its decoded coordinates.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 9 {
        return Err("usage: iec_replace_branch_function SOURCE OUTPUT PROGRAM ROW X EXPECTED REPLACEMENT OPERAND...".into());
    }
    let mut doc = XgwxDocument::from_path(&args[0])?;
    let program_index = args[2].parse::<usize>()?;
    let row = args[3].parse::<u16>()?;
    let x = args[4].parse::<u8>()?;
    let program = doc
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("missing program")??;
    let block = program
        .iec_function_blocks()
        .ok_or("invalid function records")?
        .into_iter()
        .find(|block| block.row_index == row && block.raw_x == x)
        .ok_or("missing function at selected coordinates")?;
    doc.replace_iec_ld_branch_function(
        program_index,
        block.record_offset,
        &args[5],
        &args[6],
        &args[7..],
    )?;
    std::fs::write(&args[1], doc.to_bytes()?)?;
    Ok(())
}
