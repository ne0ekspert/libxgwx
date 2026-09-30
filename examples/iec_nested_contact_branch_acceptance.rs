//! Compare a program 6 nested branch Delete Line with a native XG5000 capture.
use std::{collections::BTreeSet, env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let (source, native, output, row_index, group_index) = match args.as_slice() {
        [source, native, output, row_index] => (source, native, output, row_index, 3),
        [source, native, output, row_index, group_index] => (
            source,
            native,
            output,
            row_index,
            group_index.parse::<usize>()?,
        ),
        _ => {
            return Err(
                "usage: iec_nested_contact_branch_acceptance SOURCE NATIVE OUTPUT ROW [GROUP]"
                    .into(),
            );
        }
    };
    let row_index = row_index.parse::<u16>()?;
    let mut document = XgwxDocument::from_path(source)?;
    document.delete_iec_ld_nested_contact_branch_row(6, group_index, row_index)?;
    fs::write(output, document.to_bytes()?)?;
    let generated = XgwxDocument::from_path(output)?;
    let native = XgwxDocument::from_path(native)?;
    for (index, (generated, native)) in generated
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .enumerate()
    {
        let generated = generated?;
        let native = native?;
        if generated.data != native.data {
            let cache_offsets = if index == 6 {
                generated
                    .iec_row_frames()
                    .ok_or("generated row framing failed")?
                    .iter()
                    .map(|row| row.start + 17)
                    .collect::<BTreeSet<_>>()
            } else {
                BTreeSet::new()
            };
            let differences = generated
                .data
                .iter()
                .zip(&native.data)
                .enumerate()
                .filter(|(_, (generated, native))| generated != native)
                .map(|(offset, (generated, native))| {
                    format!("0x{offset:X}: {generated:02X} != {native:02X}")
                })
                .collect::<Vec<_>>();
            let changed_offsets = generated
                .data
                .iter()
                .zip(&native.data)
                .enumerate()
                .filter_map(|(offset, (generated, native))| (generated != native).then_some(offset))
                .collect::<Vec<_>>();
            if generated.data.len() != native.data.len()
                || changed_offsets
                    .iter()
                    .any(|offset| !cache_offsets.contains(offset))
            {
                return Err(format!(
                    "program {index} differs from native Save As: lengths {} vs {}; first differences: {}",
                    generated.data.len(),
                    native.data.len(),
                    differences.iter().take(20).cloned().collect::<Vec<_>>().join(", ")
                )
                .into());
            }
            println!("program {index}: native refreshed row display cache at {changed_offsets:?}");
        }
    }
    println!(
        "PASS native program 6 group {group_index} L{row_index} nested contact branch deletion"
    );
    Ok(())
}
