//! Remove the captured two-row branches whose lower rows contain a short wire.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, p1_output, both_output] = args.as_slice() else {
        return Err(
            "usage: iec_short_wire_branch_delete_acceptance SOURCE P1_OUTPUT BOTH_OUTPUT".into(),
        );
    };
    let mut document = XgwxDocument::from_path(source)?;
    document.edit_iec_ld_branch_segment(1, 2, 4, 5, 6, true, false)?;
    fs::write(p1_output, document.to_bytes()?)?;
    document.edit_iec_ld_branch_segment(2, 11, 34, 35, 6, true, false)?;
    fs::write(both_output, document.to_bytes()?)?;
    for index in [1, 2] {
        document
            .ladder_programs()
            .remove(index)?
            .iec_circuit_graph()
            .ok_or("invalid IEC circuit graph")?;
    }
    println!("PASS program 1 and 2 short-wire branch removal");
    Ok(())
}
