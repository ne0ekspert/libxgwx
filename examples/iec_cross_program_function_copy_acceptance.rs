//! Copy a complete function network between smart home IEC programs.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn payloads(document: &XgwxDocument) -> Result<Vec<Vec<u8>>, xgwx::XgwxError> {
    document
        .ladder_programs()
        .into_iter()
        .map(|program| program.map(|program| program.data))
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_cross_program_function_copy_acceptance SOURCE OUTPUT".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    let original = payloads(&document)?;
    // Program 3 already has an automatic UDINT local named 변환, matching the
    // source WORD_TO_UDINT output. Clearing its L1-L3 network creates room.
    document
        .delete_iec_ld_group(3, 1, 1)
        .map_err(|error| format!("clear destination: {error:?}"))?;
    let cleared = payloads(&document)?;
    document
        .copy_iec_ld_group_to_program(2, 1, 1, 3, 1)
        .map_err(|error| format!("copy function group: {error:?}"))?;
    let copied = document.ladder_programs().remove(3)?;
    let group = copied
        .iec_row_frames()
        .ok_or("copied row framing failed")?
        .into_iter()
        .find(|row| row.row_index == 1)
        .ok_or("destination L1 missing")?
        .group_index;
    if !copied
        .iec_function_blocks()
        .ok_or("copied function parsing failed")?
        .iter()
        .any(|block| block.group_index == group && block.name.value == "WORD_TO_UDINT")
    {
        return Err("destination WORD_TO_UDINT was not copied".into());
    }
    if copied.iec_circuit_graph().is_none() {
        return Err("copied circuit graph is invalid".into());
    }
    let generated = payloads(&document)?;
    for index in 0..generated.len() {
        if index != 3 && generated[index] != original[index] {
            return Err(format!("unrelated program {index} changed").into());
        }
    }
    document.write_to(output)?;
    document.delete_iec_ld_group(3, group, 1)?;
    if payloads(&document)? != cleared {
        return Err("deleting the function copy did not restore the cleared destination".into());
    }
    println!("PASS program 2 WORD_TO_UDINT L1-L3 copied to program 3 L1-L3");
    Ok(())
}
