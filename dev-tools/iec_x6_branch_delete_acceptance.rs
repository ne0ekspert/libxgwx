//! Remove the two captured smart-home x6 branches for native validation.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, p4_output, both_output] = args.as_slice() else {
        return Err("usage: iec_x6_branch_delete_acceptance SOURCE P4_OUTPUT BOTH_OUTPUT".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    document.edit_iec_ld_branch_segment(4, 6, 7, 8, 6, true, false)?;
    fs::write(p4_output, document.to_bytes()?)?;
    document.edit_iec_ld_branch_segment(5, 5, 5, 6, 6, true, false)?;
    fs::write(both_output, document.to_bytes()?)?;
    for index in [4, 5] {
        document
            .ladder_programs()
            .remove(index)?
            .iec_circuit_graph()
            .ok_or("invalid IEC circuit graph")?;
    }
    println!("PASS program 4 and 5 x6 branch removal");
    Ok(())
}
