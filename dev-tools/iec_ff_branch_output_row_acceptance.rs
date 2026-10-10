//! Compare the captured XG5000 L15 Delete Line with the guarded writer.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn group_bytes(document: &XgwxDocument) -> Result<Vec<u8>, Box<dyn Error>> {
    let program = document.ladder_programs().remove(0)?;
    let rows = program.iec_row_frames().ok_or("IEC row framing failed")?;
    let group = rows
        .iter()
        .filter(|row| row.group_index == 9)
        .collect::<Vec<_>>();
    let first = group.first().ok_or("group 9 is missing")?;
    let last = group.last().ok_or("group 9 is missing")?;
    Ok(program.data[first.start - 10..last.end].to_vec())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, native, output] = args.as_slice() else {
        return Err("usage: iec_ff_branch_output_row_acceptance SOURCE NATIVE OUTPUT".into());
    };
    let mut generated = XgwxDocument::from_path(source)?;
    generated.delete_iec_ld_ff_branch_output_row(0, 9, 15)?;
    fs::write(output, generated.to_bytes()?)?;
    let reparsed = XgwxDocument::from_path(output)?;
    let native = XgwxDocument::from_path(native)?;
    if group_bytes(&reparsed)? != group_bytes(&native)? {
        return Err("generated group 9 differs from XG5000 Delete Line".into());
    }
    let ours = reparsed.ladder_programs();
    let theirs = native.ladder_programs();
    if ours.len() != theirs.len() {
        return Err("program count changed".into());
    }
    for (index, (our_program, their_program)) in ours.into_iter().zip(theirs).enumerate() {
        let our_program = our_program?;
        let their_program = their_program?;
        if our_program.data.len() != their_program.data.len() {
            return Err(format!("program {index} payload length differs").into());
        }
        if index > 0 && our_program.data != their_program.data {
            return Err(format!("unrelated program {index} differs").into());
        }
        our_program
            .iec_circuit_graph()
            .ok_or("generated circuit graph failed")?;
    }
    println!("PASS native FF branch row deletion: group 9 and all other programs match");
    Ok(())
}
