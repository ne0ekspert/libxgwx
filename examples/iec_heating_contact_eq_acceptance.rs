//! Reproduce the native L63 Delete Line on the contact-fed L62 EQ.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_heating_contact_eq_acceptance SOURCE OUTPUT".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    let block = document
        .ladder_programs()
        .remove(6)?
        .iec_function_blocks()
        .ok_or("invalid IEC blocks")?
        .into_iter()
        .find(|block| block.group_index == 13 && block.row_index == 62)
        .ok_or("L62 EQ missing")?;
    document.delete_iec_ld_heating_chain_contact_eq(6, block.record_offset, &block.name.value)?;
    fs::write(output, document.to_bytes()?)?;
    Ok(())
}
