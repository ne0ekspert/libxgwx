//! Compare XG5000's program 3 L39 middle short-wire branch deletion.
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
    let [source, native, output] = args.as_slice() else {
        return Err("usage: iec_middle_shortwire_branch_acceptance SOURCE NATIVE OUTPUT".into());
    };
    let mut generated = XgwxDocument::from_path(source)?;
    generated.edit_iec_ld_branch_segment(3, 15, 38, 39, 6, true, false)?;
    fs::write(output, generated.to_bytes()?)?;
    let generated = XgwxDocument::from_path(output)?;
    let native = XgwxDocument::from_path(native)?;
    if group_bytes(&generated, 15)? != group_bytes(&native, 15)? {
        return Err("generated group 15 differs from XG5000 Delete Line".into());
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
    generated
        .ladder_programs()
        .remove(3)?
        .iec_circuit_graph()
        .ok_or("generated circuit graph failed")?;
    println!("PASS native group 15 middle short-wire branch deletion");
    Ok(())
}
