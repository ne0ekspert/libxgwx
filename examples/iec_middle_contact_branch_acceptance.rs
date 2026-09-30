//! Compare XG5000's program 3 L44 middle-row deletion with the guarded writer.
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
    let [source, native, generated, combined_output] = args.as_slice() else {
        return Err(
            "usage: iec_middle_contact_branch_acceptance SOURCE NATIVE GENERATED COMBINED".into(),
        );
    };
    let mut edited = XgwxDocument::from_path(source)?;
    edited.edit_iec_ld_branch_segment(3, 16, 43, 44, 6, true, false)?;
    fs::write(generated, edited.to_bytes()?)?;
    let generated = XgwxDocument::from_path(generated)?;
    let native = XgwxDocument::from_path(native)?;
    if group_bytes(&generated, 16)? != group_bytes(&native, 16)? {
        return Err("generated group 16 differs from XG5000 Delete Line".into());
    }
    for (index, (ours, theirs)) in generated
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
    combined.edit_iec_ld_branch_segment(3, 17, 47, 48, 6, true, false)?;
    combined.edit_iec_ld_branch_segment(3, 16, 43, 44, 6, true, false)?;
    fs::write(combined_output, combined.to_bytes()?)?;
    XgwxDocument::from_path(combined_output)?
        .ladder_programs()
        .remove(3)?
        .iec_circuit_graph()
        .ok_or("combined circuit graph failed")?;
    println!("PASS native program 3 group 16 L44 middle-row deletion");
    Ok(())
}
