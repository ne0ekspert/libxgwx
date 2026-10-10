//! Compare XG5000's program 3 L25 first-lower-row deletion and generate matching edits.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn group_bytes(document: &XgwxDocument, group_index: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let program = document.ladder_programs().remove(3)?;
    let rows = program.iec_row_frames().ok_or("IEC row framing failed")?;
    let group = rows
        .iter()
        .filter(|row| row.group_index == group_index)
        .collect::<Vec<_>>();
    let first = group.first().ok_or("group is missing")?;
    let last = group.last().ok_or("group is missing")?;
    Ok(program.data[first.start - 10..last.end].to_vec())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, native, first_output, combined_output] = args.as_slice() else {
        return Err(
            "usage: iec_first_lower_contact_branch_acceptance SOURCE NATIVE FIRST COMBINED".into(),
        );
    };
    let mut first = XgwxDocument::from_path(source)?;
    first.edit_iec_ld_branch_segment(3, 11, 24, 25, 6, true, false)?;
    fs::write(first_output, first.to_bytes()?)?;
    let first = XgwxDocument::from_path(first_output)?;
    let native = XgwxDocument::from_path(native)?;
    if group_bytes(&first, 11)? != group_bytes(&native, 11)? {
        return Err("generated group 11 differs from XG5000 Delete Line".into());
    }
    for (index, (ours, theirs)) in first
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .enumerate()
    {
        let ours = ours?;
        let theirs = theirs?;
        if ours.data.len() != theirs.data.len() || (index != 3 && ours.data != theirs.data) {
            return Err(format!("program {index} differs from native capture").into());
        }
    }
    let mut combined = XgwxDocument::from_path(source)?;
    for (group, start, end) in [
        (17, 46, 47),
        (16, 42, 43),
        (15, 37, 38),
        (14, 33, 34),
        (13, 30, 31),
        (12, 27, 28),
        (11, 24, 25),
    ] {
        combined.edit_iec_ld_branch_segment(3, group, start, end, 6, true, false)?;
    }
    fs::write(combined_output, combined.to_bytes()?)?;
    XgwxDocument::from_path(combined_output)?
        .ladder_programs()
        .remove(3)?
        .iec_circuit_graph()
        .ok_or("combined circuit graph failed")?;
    println!("PASS native group 11 and seven first-lower-row deletions");
    Ok(())
}
