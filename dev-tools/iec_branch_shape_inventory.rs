//! Inspect IEC branch groups without modifying the supplied workspace.
use std::{env, error::Error};
use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn Error>> {
    let source = env::args()
        .nth(1)
        .ok_or("usage: iec_branch_shape_inventory SOURCE")?;
    let doc = XgwxDocument::from_path(source)?;
    for (p, program) in doc.ladder_programs().into_iter().enumerate() {
        let program = program?;
        let rows = program.iec_row_frames().ok_or("invalid rows")?;
        let records = program.iec_record_frames().ok_or("invalid records")?;
        let geometry = program.iec_geometry().ok_or("invalid geometry")?;
        println!(
            "program {p}: functions {:?}, references {:?}, operand links {:?}, strict graph {}, layout {:?}",
            program.iec_function_blocks().map(|x| x.len()),
            program.iec_function_references().map(|x| x.len()),
            program.iec_function_operand_links().map(|x| x.len()),
            program.iec_circuit_graph().is_some(),
            program
                .iec_circuit_layout()
                .map(|x| x.open_branch_endpoints)
        );
        for g in 0..usize::from(u16::from_le_bytes(program.data[6..8].try_into()?)) {
            let branches = geometry
                .vertical
                .iter()
                .filter(|b| b.group_index == g)
                .collect::<Vec<_>>();
            if branches.is_empty() {
                continue;
            }
            println!(
                "program {p} group {g}: {:?}",
                branches
                    .iter()
                    .map(|b| (b.start_row_index, b.end_row_index, b.x))
                    .collect::<Vec<_>>()
            );
            for row in rows.iter().filter(|r| r.group_index == g) {
                println!(
                    " L{} cache {}: {:?}",
                    row.row_index,
                    program.data[row.start + 29],
                    records
                        .iter()
                        .filter(|r| r.group_index == g && r.row_index == row.row_index)
                        .map(|r| (
                            r.kind,
                            program.data[r.offset
                                + if r.kind == xgwx::IecRecordKind::BranchStart {
                                    7
                                } else {
                                    5
                                }]
                        ))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    Ok(())
}
