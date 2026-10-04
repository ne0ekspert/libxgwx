//! Delete a branch scalar block and optionally refill it with a typed instruction.
use std::{env, error::Error};
use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn Error>> {
    let a = env::args().skip(1).collect::<Vec<_>>();
    if a.len() < 6 {
        return Err("usage: iec_branch_scalar_acceptance SOURCE OUTPUT PROGRAM ROW X EXPECTED [REPLACEMENT OPERANDS...]".into());
    }
    let mut doc = XgwxDocument::from_path(&a[0])?;
    let p = a[2].parse::<usize>()?;
    let row = a[3].parse::<u16>()?;
    let x = a[4].parse::<u8>()?;
    let b = doc
        .ladder_programs()
        .into_iter()
        .nth(p)
        .ok_or("missing program")??
        .iec_function_blocks()
        .ok_or("invalid functions")?
        .into_iter()
        .find(|b| b.row_index == row && b.raw_x == x && b.name.value == a[5])
        .ok_or("missing target")?;
    doc.delete_iec_ld_branch_function(p, b.record_offset, &a[5])?;
    if a.len() > 6 {
        doc.insert_iec_ld_function(p, row, x, &a[6], &a[7..])?;
    }
    std::fs::write(&a[1], doc.to_bytes()?)?;
    Ok(())
}
