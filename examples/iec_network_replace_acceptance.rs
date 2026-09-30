//! Generate a complete IEC network replacement for XG5000 acceptance.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_network_replace_acceptance SOURCE OUTPUT".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    let before = document.ladder_programs();
    document.replace_iec_ld_group(0, 2, 2, 4, 6)?;
    let after = document.ladder_programs();
    for index in 1..before.len() {
        if before[index].as_ref().expect("source program parses").data
            != after[index].as_ref().expect("edited program parses").data
        {
            return Err(format!("unrelated program {index} changed").into());
        }
    }
    if after[0]
        .as_ref()
        .expect("edited program parses")
        .iec_circuit_graph()
        .is_none()
    {
        return Err("replaced program circuit graph is invalid".into());
    }
    document.write_to(output)?;
    let saved = XgwxDocument::from_path(output)?;
    for (left, right) in saved.ladder_programs().into_iter().zip(after) {
        if left?.data != right?.data {
            return Err("written ProgramData changed on reparse".into());
        }
    }
    println!("PASS replaced program 0 network L6 with a copy of L2");
    Ok(())
}
