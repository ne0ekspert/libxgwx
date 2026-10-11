//! Reproduce the native L47 Delete Line of the first comparison block.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_heating_chain_head_acceptance SOURCE OUTPUT".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    let block = document
        .program()
        .ladder_programs()
        .remove(6)?
        .iec_function_blocks()
        .ok_or("invalid IEC blocks")?
        .into_iter()
        .find(|block| block.group_index == 13 && block.row_index == 46)
        .ok_or("first group-13 block missing")?;
    document
        .program_mut()
        .get(6)
        .and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .delete_heating_chain_head(block.record_offset, &block.name.value)
        })?;
    fs::write(output, document.to_bytes()?)?;
    Ok(())
}
