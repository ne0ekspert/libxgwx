//! Change one mapped BOOL address across digit lengths for XG5000 acceptance.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, destination] = args.as_slice() else {
        return Err("usage: iec_address_length_acceptance <source.xgwx> <output.xgwx>".into());
    };
    let original = fs::read(source)?;
    let mut document = XgwxDocument::parse(&original)?;
    document.update_iec_local_symbol_address(0, 5, "ON", "%MX8", "%MX100")?;
    let edited = document.to_bytes()?;
    let mut restored = XgwxDocument::parse(&edited)?;
    restored.update_iec_local_symbol_address(0, 5, "ON", "%MX100", "%MX8")?;
    let reparsed = XgwxDocument::parse(&restored.to_bytes()?)?;
    let original_document = XgwxDocument::parse(&original)?;
    for (left, right) in reparsed
        .ladder_programs()
        .into_iter()
        .zip(original_document.ladder_programs())
    {
        if left?.data != right?.data {
            return Err("inverse changed a ProgramData payload".into());
        }
    }
    let restored_symbols = reparsed
        .iec_local_symbols()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let original_symbols = original_document
        .iec_local_symbols()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    if restored_symbols != original_symbols {
        return Err("inverse changed a local symbol".into());
    }
    fs::write(destination, edited)?;
    println!("PASS program 0 ON address %MX8 -> %MX100; inverse restores decoded data");
    Ok(())
}
