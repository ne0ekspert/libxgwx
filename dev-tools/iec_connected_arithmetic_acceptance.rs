//! Exercise connected arithmetic deletion and optional typed refill.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 4 {
        return Err("usage: iec_connected_arithmetic_acceptance SOURCE OUTPUT PROGRAM ROW [NAME OPERANDS...]".into());
    }
    let p = args[2].parse()?;
    let row = args[3].parse()?;
    let mut doc = XgwxDocument::from_path(&args[0])?;
    let program = doc.program().ladder_programs().remove(p)?;
    let block = program
        .iec_function_blocks()
        .ok_or("invalid blocks")?
        .into_iter()
        .find(|b| b.row_index == row && matches!(b.name.value.as_str(), "ADD" | "SUB"))
        .ok_or("missing arithmetic block")?;
    println!(
        "sites: {:?}",
        program.iec_connected_arithmetic_deletion_sites()
    );
    doc.program_mut().get(p).and_then(|mut selected_program| {
        selected_program
            .iec_ld()?
            .delete_connected_arithmetic(block.record_offset, &block.name.value)
    })?;
    if args.len() > 4 {
        doc.program_mut().get(p).and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .insert_function(row, block.raw_x, &args[4], &args[5..])
        })?;
    }
    doc.write_to(&args[1])?;
    println!("PASS connected arithmetic edit");
    Ok(())
}
