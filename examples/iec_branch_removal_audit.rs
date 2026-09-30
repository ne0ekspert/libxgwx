//! Count supported removals of existing IEC vertical branch segments.
use std::{cmp::Reverse, collections::BTreeMap, env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let source = env::args()
        .nth(1)
        .ok_or("usage: iec_branch_removal_audit SOURCE")?;
    let by_group = env::args().any(|arg| arg == "--by-group");
    let document = XgwxDocument::from_path(source)?;
    let mut total = 0;
    let mut editable = 0;
    let mut rejected = BTreeMap::<String, (usize, Vec<String>)>::new();
    let mut group_counts = BTreeMap::<(usize, usize, String), usize>::new();
    for (program_index, result) in document.ladder_programs().into_iter().enumerate() {
        let program = result?;
        let geometry = program.iec_geometry().ok_or("invalid IEC geometry")?;
        for segment in geometry.vertical {
            total += 1;
            let mut trial = document.clone();
            match trial.edit_iec_ld_branch_segment(
                program_index,
                segment.group_index,
                segment.start_row_index,
                segment.end_row_index,
                segment.x,
                true,
                false,
            ) {
                Ok(()) => editable += 1,
                Err(error) => {
                    let reason = error.to_string();
                    *group_counts
                        .entry((program_index, segment.group_index, reason.clone()))
                        .or_default() += 1;
                    let sample = format!(
                        "program {program_index}, group {}, L{}–L{}, x{}",
                        segment.group_index,
                        segment.start_row_index,
                        segment.end_row_index,
                        segment.x
                    );
                    let entry = rejected.entry(reason).or_default();
                    entry.0 += 1;
                    if entry.1.len() < 10 {
                        entry.1.push(sample);
                    }
                }
            }
        }
    }
    println!("{editable}/{total} removable vertical segments");
    for (reason, (count, samples)) in rejected {
        println!("{count}: {reason}");
        for sample in samples {
            println!("  {sample}");
        }
    }
    if by_group {
        let mut groups = group_counts.into_iter().collect::<Vec<_>>();
        groups.sort_by_key(|((program, group, reason), count)| {
            (Reverse(*count), *program, *group, reason.clone())
        });
        println!("rejected segments by program and group:");
        for ((program, group, reason), count) in groups {
            println!("  {count:>2} program {program} group {group}: {reason}");
        }
    }
    Ok(())
}
