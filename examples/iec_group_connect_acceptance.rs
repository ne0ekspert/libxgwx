//! Generate adjacent-group connection/splitting and compare native captures.
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
    for (p, (a, b)) in doc
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .enumerate()
    {
        assert!(
            normalized(&a?) == normalized(&b?),
            "program {p} payload differs from native capture"
        );
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if !(2..=6).contains(&args.len()) {
        return Err(
            "usage: iec_group_connect_acceptance SOURCE OUTPUT [NATIVE_JOIN] [NATIVE_SPLIT] [NATIVE_MULTI] [NATIVE_FUNCTION_JOIN]".into(),
        );
    }
    let mut doc = XgwxDocument::from_path(&args[0])?;
    let originals = doc
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    doc.connect_iec_ld_groups(0, 2, 2, 3, 3, 3)?;
    let changed = doc.ladder_programs().remove(0)?;
    assert_eq!(changed.data.len(), originals[0].data.len() + 26);
    assert_eq!(
        changed.iec_row_frames().unwrap().len(),
        originals[0].iec_row_frames().unwrap().len()
    );
    assert_eq!(
        changed.iec_circuit_graph().unwrap().edges.len(),
        originals[0].iec_circuit_graph().unwrap().edges.len() + 1
    );
    for (p, original) in originals.iter().enumerate().skip(1) {
        assert_eq!(doc.ladder_programs().remove(p)?.data, original.data);
    }
    if let Some(path) = args.get(2) {
        compare(&doc, path)?;
    }
    doc.write_to(&args[1])?;
    doc.edit_iec_ld_vertical_wire(0, 2, 2, 3, 3, true, false)?;
    doc.split_iec_ld_group(0, 2, 2, 3)?;
    for (p, original) in originals.iter().enumerate() {
        assert_eq!(doc.ladder_programs().remove(p)?.data, original.data);
    }
    doc.write_to(format!("{}.split.xgwx", &args[1]))?;
    if let Some(path) = args.get(3) {
        // The native range Delete capture removes the lower contact as well
        // as the incoming wire, then separates the groups.
        let program = doc.ladder_programs().remove(0)?;
        let record = program
            .iec_record_frames()
            .unwrap()
            .into_iter()
            .find(|r| r.group_index == 3 && r.row_index == 3 && r.kind == IecRecordKind::Contact(6))
            .unwrap();
        let units = program.data[record.offset + 19..record.end]
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>();
        let operand = String::from_utf16(&units)?;
        doc.delete_iec_ld_contact(0, record.offset, 1, "NO", &operand)?;
        compare(&doc, path)?;
    }
    let mut suite = XgwxDocument::from_path(&args[0])?;
    for (p, upper, start, lower, end, x) in [(0, 2, 2, 3, 3, 3), (3, 7, 18, 8, 19, 3)] {
        suite.connect_iec_ld_groups(p, upper, start, lower, end, x)?;
    }
    suite.write_to(format!("{}.multi.xgwx", &args[1]))?;
    if let Some(path) = args.get(4) {
        compare(&suite, path)?;
    }
    suite.insert_iec_ld_single_element(0, 66, 1, "contact", "NO", "ON")?;
    suite.write_to(format!("{}.function-base.xgwx", &args[1]))?;
    suite.connect_iec_ld_groups(0, 32, 66, 33, 67, 3)?;
    suite.write_to(format!("{}.suite.xgwx", &args[1]))?;
    if let Some(path) = args.get(5) {
        compare(&suite, path)?;
    }
    println!(
        "PASS native group connection/split comparison, exact restoration and unchanged unrelated programs"
    );
    Ok(())
}
