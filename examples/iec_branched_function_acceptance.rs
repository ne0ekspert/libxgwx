//! Generate native Delete Line on the last row of a branched arithmetic block.
use std::{env, error::Error};
use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output, program, row] = args.as_slice() else {
        return Err(
            "usage: iec_branched_function_acceptance SOURCE OUTPUT PROGRAM BLOCK_ROW".into(),
        );
    };
    let p = program.parse()?;
    let row: u16 = row.parse()?;
    let mut doc = XgwxDocument::from_path(source)?;
    let block = doc
        .ladder_programs()
        .remove(p)?
        .iec_function_blocks()
        .ok_or("invalid blocks")?
        .into_iter()
        .find(|b| b.row_index == row)
        .ok_or("missing block")?;
    doc.delete_iec_ld_branched_arithmetic(p, block.record_offset, &block.name.value)?;
    doc.write_to(output)?;
    Ok(())
}
