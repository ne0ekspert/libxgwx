//! Generate a combined wire-gap and nonstructural edit for native acceptance.
use std::{env, error::Error};
use xgwx::{IecRecordKind, XgwxDocument};
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: iec_open_layout_acceptance SOURCE OUTPUT".into());
    }
    let mut doc = XgwxDocument::from_path(&args[0])?;
    doc.edit_iec_ld_vertical_wire(0, 33, 69, 70, 12, true, false)?;
    let program = doc.ladder_programs().remove(0)?;
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
    doc.update_iec_ld_function_operand(0, operand.offset, "0", "123")?;
    let program = doc.ladder_programs().remove(0)?;
    let block = program
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.group_index == 33 && b.name.value == "EQ")
        .unwrap();
    doc.update_iec_ld_comparison_function(0, block.name.offset, "EQ", "GE")?;
    doc.write_to(&args[1])?;
    assert_eq!(
        doc.ladder_programs()
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
