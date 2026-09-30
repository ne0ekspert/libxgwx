//! Reproduce native Delete Line of the L50 or L54 heating-chain EQ.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, row, output] = args.as_slice() else {
        return Err("usage: iec_heating_chain_middle_acceptance SOURCE ROW OUTPUT".into());
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
        .ok_or("group-13 block missing")?;
    document.delete_iec_ld_heating_chain_middle(6, block.record_offset, &block.name.value)?;
    fs::write(output, document.to_bytes()?)?;
    Ok(())
}
