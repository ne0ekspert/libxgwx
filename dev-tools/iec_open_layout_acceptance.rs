//! Generate a combined wire-gap and nonstructural edit for native acceptance.
use std::{env, error::Error};
use xgwx::{IecRecordKind, XgwxDocument};
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: iec_open_layout_acceptance SOURCE OUTPUT".into());
    }
    let mut doc = XgwxDocument::from_path(&args[0])?;
    doc.program_mut().get(0).and_then(|mut selected_program| {
        selected_program
            .iec_ld()?
            .edit_vertical_wire(33, 69, 70, 12, true, false)
    })?;
    let program = doc.program().ladder_programs().remove(0)?;
    let record = program
        .iec_record_frames()
        .unwrap()
        .into_iter()
        .find(|r| {
            r.group_index == 33 && r.row_index == 69 && r.kind == IecRecordKind::FunctionOperand
        })
        .unwrap();
    let operand = program
        .strings
        .iter()
        .find(|s| s.offset >= record.offset && s.offset < record.end && s.value == "0")
        .unwrap();
    doc.program_mut().get(0).and_then(|mut selected_program| {
        selected_program
            .iec_ld()?
            .update_function_operand(operand.offset, "0", "123")
    })?;
    let program = doc.program().ladder_programs().remove(0)?;
    let block = program
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.group_index == 33 && b.name.value == "EQ")
        .unwrap();
    doc.program_mut().get(0).and_then(|mut selected_program| {
        selected_program
            .iec_ld()?
            .update_comparison_function(block.name.offset, "EQ", "GE")
    })?;
    doc.write_to(&args[1])?;
    assert_eq!(
        doc.program()
            .ladder_programs()
            .remove(0)?
            .iec_circuit_layout()
            .unwrap()
            .open_branch_endpoints
            .len(),
        2
    );
    println!("PASS generated open-layout operand and comparison edits");
    Ok(())
}
