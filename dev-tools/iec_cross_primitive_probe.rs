//! Copy a Unicode UDINT local with the network that writes to it.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_cross_primitive_probe SOURCE OUTPUT".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    let original = document.to_bytes()?;
    if document
        .copy_iec_ld_group_to_program(2, 1, 1, 4, 26)
        .is_ok()
        || document.to_bytes()? != original
    {
        return Err(
            "copy without the Unicode UDINT destination local was not rejected atomically".into(),
        );
    }
    let before_symbols = document.iec_local_symbols().remove(4)?;
    document.copy_iec_ld_group_to_program_with_locals(2, 1, 1, 4, 26)?;
    let after_symbols = document.iec_local_symbols().remove(4)?;
    let local = after_symbols
        .iter()
        .find(|symbol| symbol.name == "변환")
        .ok_or("Unicode UDINT destination local is missing")?;
    if after_symbols.len() != before_symbols.len() + 1
        || local.data_type.as_deref() != Some("UDINT")
        || local.storage_class != "A"
        || local.address.is_some()
    {
        return Err(format!("copied Unicode UDINT local is invalid: {local:?}").into());
    }
    let program = document.ladder_programs().remove(4)?;
    if program.iec_circuit_graph().is_none()
        || !program
            .iec_function_blocks()
            .ok_or("function decode failed")?
            .iter()
            .any(|block| block.row_index == 26 && block.name.value == "WORD_TO_UDINT")
    {
        return Err("copied WORD_TO_UDINT network failed graph validation".into());
    }
    document.write_to(output)?;
    println!(
        "PASS program 2 WORD_TO_UDINT L1-L3 copied to program 4 L26-L28 with Unicode UDINT local"
    );
    Ok(())
}
