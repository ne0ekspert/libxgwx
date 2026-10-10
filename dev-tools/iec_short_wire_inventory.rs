//! Find short IEC wires bracketed by long wires in the smart-home project.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("usage: iec_short_wire_inventory <file.xgwx>")?;
    let doc = XgwxDocument::from_path(path)?;
    for (program_index, program) in doc.ladder_programs().into_iter().enumerate() {
        let program = program?;
        let Some(sites) = program.iec_horizontal_wire_deletion_sites() else {
            continue;
        };
        for site in sites {
            println!(
                "program {program_index} group {} L{} short offset 0x{:X} raw x {}",
                site.group_index, site.row_index, site.wire_offset, site.raw_x
            );
        }
    }
    Ok(())
}
