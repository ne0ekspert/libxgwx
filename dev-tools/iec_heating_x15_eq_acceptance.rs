//! Delete either x15-fed comparison block from the original smart home file.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, row, output] = args.as_slice() else {
        return Err("usage: iec_heating_x15_eq_acceptance SOURCE ROW OUTPUT".into());
    };
    let row = row.parse::<u16>()?;
    let mut document = XgwxDocument::from_path(source)?;
    let block = document
        .ladder_programs()
        .remove(6)?
        .iec_function_blocks()
        .ok_or("invalid IEC blocks")?
        .into_iter()
        .find(|block| block.group_index == 13 && block.row_index == row)
        .ok_or("comparison missing")?;
    document.delete_iec_ld_heating_chain_x15_eq(6, block.record_offset, &block.name.value)?;
    fs::write(output, document.to_bytes()?)?;
    Ok(())
}
