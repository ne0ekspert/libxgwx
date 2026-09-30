//! Delete the captured first EQ cell in the smart-home comparison chain.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_eq_chain_head_acceptance SOURCE OUTPUT".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    let program = document.ladder_programs().remove(0)?;
    let block = program
        .iec_function_blocks()
        .ok_or("invalid blocks")?
        .into_iter()
        .find(|block| block.record_offset == 0x2a5c)
        .ok_or("expected EQ block missing")?;
    println!("block at 0x{:X}: {}", block.record_offset, block.name.value);
    document.delete_iec_ld_eq_chain_head(0, block.record_offset, "EQ")?;
    fs::write(output, document.to_bytes()?)?;
    Ok(())
}
