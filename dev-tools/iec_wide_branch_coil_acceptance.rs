//! Compare coil placement and deletion on wider IEC branch rows with native captures.
use std::{env, error::Error};
use xgwx::{IecRecordKind, LadderProgramData, XgwxDocument};
fn normalized(program: &LadderProgramData) -> Vec<u8> {
    let mut bytes = program.data.clone();
    for row in program.iec_row_frames().unwrap() {
        bytes[row.start + 17..row.start + 21].fill(0);
    }
    bytes
}
fn compare(doc: &XgwxDocument, path: &str) -> Result<(), Box<dyn Error>> {
    let native = XgwxDocument::from_path(path)?;
    assert_eq!(doc.ladder_programs().len(), native.ladder_programs().len());
    for (p, (a, b)) in doc
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .enumerate()
    {
        assert!(
            normalized(&a?) == normalized(&b?),
            "program {p} native payload differs"
        );
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if !(2..=3).contains(&args.len()) {
        return Err(
            "usage: iec_wide_branch_coil_acceptance SOURCE OUTPUT [NATIVE_DIRECTORY]".into(),
        );
    }
    let mut doc = XgwxDocument::from_path(&args[0])?;
    doc.extend_iec_ld_vertical_wire(0, 3, 4, 6)?;
    doc.extend_iec_ld_vertical_wire(0, 8, 12, 24)?;
    doc.write_to(format!("{}.wire.xgwx", args[1]))?;
    let before = doc
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    doc.insert_iec_ld_single_element(0, 5, 7, "coil", "OUTPUT", "%MX0")?;
    doc.write_to(format!("{}.first.xgwx", args[1]))?;
    if let Some(dir) = args.get(2) {
        compare(&doc, &format!("{dir}/WIDE6A.xgwx"))?;
    }
    doc.insert_iec_ld_single_element(0, 13, 25, "coil", "OUTPUT", "%MX1")?;
    assert!(
        doc.ladder_programs()
            .remove(0)?
            .iec_circuit_graph()
            .is_some()
    );
    doc.write_to(&args[1])?;
    if let Some(dir) = args.get(2) {
        compare(&doc, &format!("{dir}/WIDE24A.xgwx"))?;
    }
    for (row, operand, stem) in [(5, "%MX0", "WIDE6DEL"), (13, "%MX1", "WIDEDEL")] {
        let program = doc.ladder_programs().remove(0)?;
        let coil = program
            .iec_record_frames()
            .unwrap()
            .into_iter()
            .find(|r| r.row_index == row && r.kind == IecRecordKind::Coil(14))
            .unwrap();
        doc.delete_iec_ld_terminal_coil(0, coil.offset, operand)?;
        if row == 5 {
            doc.write_to(format!("{}.first-deleted.xgwx", args[1]))?;
        }
        if let Some(dir) = args.get(2) {
            compare(&doc, &format!("{dir}/{stem}.xgwx"))?;
        }
    }
    for (p, old) in before.iter().enumerate() {
        assert_eq!(
            doc.ladder_programs().remove(p)?.data,
            old.data,
            "program {p} restoration"
        );
    }
    doc.write_to(format!("{}.deleted.xgwx", args[1]))?;
    println!("PASS wider coil feeds, multiple open endpoints and exact restoration");
    Ok(())
}
