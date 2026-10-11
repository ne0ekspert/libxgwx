//! Remove the two captured smart-home x6 branches for native validation.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, p4_output, both_output] = args.as_slice() else {
        return Err("usage: iec_x6_branch_delete_acceptance SOURCE P4_OUTPUT BOTH_OUTPUT".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    document
        .program_mut()
        .get(4)
        .and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .edit_branch_segment(6, 7, 8, 6, true, false)
        })?;
    fs::write(p4_output, document.to_bytes()?)?;
    document
        .program_mut()
        .get(5)
        .and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .edit_branch_segment(5, 5, 6, 6, true, false)
        })?;
    fs::write(both_output, document.to_bytes()?)?;
    for index in [4, 5] {
        document
            .program()
            .ladder_programs()
            .remove(index)?
            .iec_circuit_graph()
            .ok_or("invalid IEC circuit graph")?;
    }
    println!("PASS program 4 and 5 x6 branch removal");
    Ok(())
}
