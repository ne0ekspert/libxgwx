//! List source IEC networks that reference automatic primitive locals missing from a target.
use std::{collections::BTreeSet, env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, destination] = args.as_slice() else {
        return Err("usage: iec_cross_local_candidates SOURCE DESTINATION_PROGRAM".into());
    };
    let document = XgwxDocument::from_path(source)?;
    let programs = document
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let locals = document
        .iec_local_symbols()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let destination: usize = destination.parse()?;
    let destination_names = locals[destination]
        .iter()
        .map(|item| item.name.to_lowercase())
        .collect::<BTreeSet<_>>();
    for (index, program) in programs.iter().enumerate() {
        if index == destination {
            continue;
        }
        let symbols = &locals[index];
        let rows = program.iec_row_frames().ok_or("IEC row framing failed")?;
        let groups = rows
            .iter()
            .map(|row| row.group_index)
            .collect::<BTreeSet<_>>();
        for group_index in groups {
            let group = rows
                .iter()
                .filter(|row| row.group_index == group_index)
                .collect::<Vec<_>>();
            let start = group.first().unwrap().start - 10;
            let end = group.last().unwrap().end;
            let missing = program
                .strings
                .iter()
                .filter(|item| item.offset >= start && item.end_offset <= end)
                .filter_map(|item| {
                    symbols
                        .iter()
                        .find(|symbol| symbol.name.eq_ignore_ascii_case(&item.value))
                })
                .filter(|symbol| {
                    !symbol.is_instance
                        && symbol.data_type.is_some()
                        && !destination_names.contains(&symbol.name.to_lowercase())
                })
                .map(|symbol| {
                    format!(
                        "{}:{}:{}",
                        symbol.name,
                        symbol.data_type.as_deref().unwrap(),
                        symbol.storage_class
                    )
                })
                .collect::<BTreeSet<_>>();
            if !missing.is_empty() {
                let all = program
                    .strings
                    .iter()
                    .filter(|item| item.offset >= start && item.end_offset <= end)
                    .map(|item| item.value.as_str())
                    .collect::<Vec<_>>();
                println!(
                    "program {index} group {group_index} L{}-L{}: {missing:?} {all:?}",
                    group.first().unwrap().row_index,
                    group.last().unwrap().row_index
                );
            }
        }
    }
    Ok(())
}
