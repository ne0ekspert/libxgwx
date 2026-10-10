//! Delete the L58 EQ and remove its orphan x15 feed in one saved project.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_heating_x3_repaired_acceptance SOURCE OUTPUT".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    let block = document
        .ladder_programs()
        .remove(6)?
        .iec_function_blocks()
        .ok_or("invalid IEC blocks")?
        .into_iter()
        .find(|block| block.group_index == 13 && block.row_index == 58)
        .ok_or("L58 EQ missing")?;
    document.delete_iec_ld_heating_chain_x3_eq_repaired(
        6,
        block.record_offset,
        &block.name.value,
    )?;
    fs::write(output, document.to_bytes()?)?;
    Ok(())
}
