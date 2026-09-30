//! Compare the captured XG5000 L52 output-branch deletion with the writer.
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
    let [source, native, output, both_output] = args.as_slice() else {
        return Err(
            "usage: iec_output_branch_delete_acceptance SOURCE NATIVE OUTPUT BOTH_OUTPUT".into(),
        );
    };
    let mut generated = XgwxDocument::from_path(source)?;
    generated.edit_iec_ld_branch_segment(3, 18, 51, 52, 24, true, false)?;
    fs::write(output, generated.to_bytes()?)?;
    let reparsed = XgwxDocument::from_path(output)?;
    let native = XgwxDocument::from_path(native)?;
    if group_bytes(&reparsed, 18)? != group_bytes(&native, 18)? {
        return Err("generated group 18 differs from XG5000 Delete Line".into());
    }
    for (index, (ours, theirs)) in reparsed
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
        ours.iec_circuit_graph()
            .ok_or("generated circuit graph failed")?;
    }
    generated.edit_iec_ld_branch_segment(3, 19, 52, 53, 24, true, false)?;
    fs::write(both_output, generated.to_bytes()?)?;
    let both = XgwxDocument::from_path(both_output)?;
    both.ladder_programs()
        .remove(3)?
        .iec_circuit_graph()
        .ok_or("combined circuit graph failed")?;
    println!("PASS program 3 output-branch deletion for groups 18 and 19");
    Ok(())
}
