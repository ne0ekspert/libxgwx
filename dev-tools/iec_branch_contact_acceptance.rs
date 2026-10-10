//! Compare contact placement and deletion on a new IEC branch row with native captures.
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
    if !(2..=4).contains(&args.len()) {
        return Err(
            "usage: iec_branch_contact_acceptance SOURCE OUTPUT [NATIVE_INSERT] [NATIVE_DELETE]"
                .into(),
        );
    }
    let mut doc = XgwxDocument::from_path(&args[0])?;
    doc.extend_iec_ld_vertical_wire(0, 3, 4, 3)?;
    let before = doc
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    doc.insert_iec_ld_single_element(0, 5, 1, "contact", "NO", "ON")?;
    assert!(
        doc.ladder_programs()
            .remove(0)?
            .iec_circuit_graph()
            .is_some()
    );
    if let Some(path) = args.get(2) {
        compare(&doc, path)?;
    }
    doc.write_to(&args[1])?;
    let record = doc
        .ladder_programs()
        .remove(0)?
        .iec_record_frames()
        .unwrap()
        .into_iter()
        .find(|r| r.row_index == 5 && r.kind == IecRecordKind::Contact(6))
        .unwrap();
    doc.delete_iec_ld_contact(0, record.offset, 1, "NO", "ON")?;
    for (p, old) in before.iter().enumerate() {
        assert!(
            doc.ladder_programs().remove(p)?.data == old.data,
            "program {p} restoration differs"
        );
    }
    if let Some(path) = args.get(3) {
        compare(&doc, path)?;
    }
    doc.write_to(format!("{}.deleted.xgwx", args[1]))?;
    println!("PASS native branch-row contact insertion/deletion and exact wire-only restoration");
    Ok(())
}
