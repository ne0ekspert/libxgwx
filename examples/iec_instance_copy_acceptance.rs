//! Generate a copied IEC function network with an independent R_TRIG instance.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_instance_copy_acceptance <copied.xgwx> <output.xgwx>".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    let program = document
        .ladder_programs()
        .into_iter()
        .next()
        .ok_or("program 0 missing")??;
    let block = program
        .iec_function_blocks()
        .ok_or("IEC function decode failed")?
        .into_iter()
        .find(|block| block.group_index == 33 && block.name.value == "R_TRIG")
        .ok_or("copied R_TRIG missing")?;
    let original = block.instance.as_ref().ok_or("R_TRIG instance missing")?;
    document.duplicate_iec_ld_function_instance(
        0,
        block.record_offset,
        &original.value,
        "INST4",
    )?;
    fs::write(output, document.to_bytes()?)?;
    Ok(())
}
