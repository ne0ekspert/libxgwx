//! Audit writer branch-removal preflight without modifying the source workspace.
//! Accepted means the writer validates an edit, not that it passed native XG5000.
use std::{collections::BTreeMap, env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let source = env::args()
        .nth(1)
        .ok_or("usage: iec_branch_edit_coverage SOURCE")?;
    let details = env::args().any(|arg| arg == "--details");
    let document = XgwxDocument::from_path(source)?;
    let mut total = 0;
    let mut accepted = 0;
    let mut row_preserving = 0;
    let mut failures = BTreeMap::<String, usize>::new();
    for (p, program) in document.ladder_programs().into_iter().enumerate() {
        let program = program?;
        if program.project_type != Some(2) {
            continue;
        }
        let geometry = program.iec_geometry().ok_or("invalid IEC geometry")?;
        let functions = program
            .iec_function_blocks()
            .ok_or("invalid IEC functions")?;
        let mut groups = BTreeMap::<usize, (usize, usize)>::new();
        for segment in &geometry.vertical {
            total += 1;
            if document
                .clone()
                .edit_iec_ld_vertical_wire(
                    p,
                    segment.group_index,
                    segment.start_row_index,
                    segment.end_row_index,
                    segment.x,
                    true,
                    false,
                )
                .is_ok()
            {
                row_preserving += 1;
            }
            let group = groups.entry(segment.group_index).or_default();
            group.1 += 1;
            let mut candidate = document.clone();
            match candidate.edit_iec_ld_branch_segment(
                p,
                segment.group_index,
                segment.start_row_index,
                segment.end_row_index,
                segment.x,
                true,
                false,
            ) {
                Ok(()) => {
                    accepted += 1;
                    group.0 += 1;
                    if details {
                        println!(
                            "ACCEPT program {p}, group {}, L{}-L{}, x{}",
                            segment.group_index,
                            segment.start_row_index,
                            segment.end_row_index,
                            segment.x
                        );
                    }
                }
                Err(error) => {
                    *failures.entry(error.to_string()).or_default() += 1;
                }
            }
        }
        for (group, (ok, count)) in groups {
            let names = functions
                .iter()
                .filter(|f| f.group_index == group)
                .map(|f| f.name.value.as_str())
                .collect::<Vec<_>>();
            println!(
                "program {p}, group {group}: {ok}/{count} branch removals pass writer preflight; functions {names:?}"
            );
        }
    }
    println!("TOTAL {accepted}/{total} branch cleanup removals pass writer preflight");
    println!(
        "TOTAL {row_preserving}/{total} row-preserving vertical removals pass writer preflight"
    );
    for (reason, count) in failures {
        println!("REJECTED {count}: {reason}");
    }
    Ok(())
}
