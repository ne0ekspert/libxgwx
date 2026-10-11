//! Find contact-only and branch-only chain rows accepted by guarded writers.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let source = env::args()
        .nth(1)
        .ok_or("usage: iec_chained_contact_branch_inventory SOURCE")?;
    let document = XgwxDocument::from_path(source)?;
    for (program_index, result) in document.program().ladder_programs().into_iter().enumerate() {
        let program = result?;
        let rows = program.iec_row_frames().ok_or("invalid IEC rows")?;
        let mut contact_count = 0;
        let mut empty_count = 0;
        for row in rows {
            let mut trial = document.clone();
            if trial
                .program_mut()
                .get(program_index)
                .and_then(|mut selected_program| {
                    selected_program
                        .iec_ld()?
                        .delete_chained_contact_branch_row(row.group_index, row.row_index)
                })
                .is_ok()
            {
                println!(
                    "contact: program {program_index}, group {}, L{}",
                    row.group_index, row.row_index
                );
                contact_count += 1;
            }
            let mut trial = document.clone();
            if trial
                .program_mut()
                .get(program_index)
                .and_then(|mut selected_program| {
                    selected_program
                        .iec_ld()?
                        .delete_empty_branch_row(row.group_index, row.row_index)
                })
                .is_ok()
            {
                println!(
                    "empty: program {program_index}, group {}, L{}",
                    row.group_index, row.row_index
                );
                empty_count += 1;
            }
        }
        println!(
            "program {program_index}: {contact_count} contact, {empty_count} empty candidate rows"
        );
    }
    Ok(())
}
