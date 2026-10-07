#![cfg(feature = "write")]
use xgwx::XgwxDocument;
fn base() -> XgwxDocument {
    XgwxDocument::parse(include_bytes!(
        "../fixtures/function-output-wires/base.xgwx"
    ))
    .unwrap()
}
fn wire(doc: &mut XgwxDocument, row: u16, pin: &str, x: u8) {
    let b = doc
        .ladder_programs()
        .remove(0)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == row)
        .unwrap();
    doc.insert_iec_ld_function_output_wire(0, b.record_offset, &b.name.value, pin, x)
        .unwrap();
}
fn payload(bytes: &[u8]) -> Vec<u8> {
    XgwxDocument::parse(bytes)
        .unwrap()
        .ladder_programs()
        .remove(0)
        .unwrap()
        .data
}
#[test]
fn output_wires_match_native_f5_and_keep_numeric_destinations() {
    let mut d = base();
    wire(&mut d, 0, "ENO", 10);
    assert_eq!(
        d.ladder_programs().remove(0).unwrap().data,
        payload(include_bytes!(
            "../fixtures/function-output-wires/add-eno-native.xgwx"
        ))
    );
    for row in [5, 10, 19] {
        wire(&mut d, row, "ENO", 10);
    }
    wire(&mut d, 15, "OUT", 10);
    assert_eq!(
        d.ladder_programs().remove(0).unwrap().data,
        payload(include_bytes!(
            "../fixtures/function-output-wires/all-native.xgwx"
        ))
    );
    let p = XgwxDocument::parse(&d.to_verified_bytes().unwrap())
        .unwrap()
        .ladder_programs()
        .remove(0)
        .unwrap();
    assert!(p.iec_circuit_graph().is_some());
    assert_eq!(
        p.iec_function_operand_links()
            .unwrap()
            .iter()
            .filter(|l| l.is_output)
            .count(),
        4
    );
}
#[test]
fn output_wire_guards_are_atomic() {
    let mut d = base();
    let b = d
        .ladder_programs()
        .remove(0)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .remove(0);
    for (name, pin, x) in [
        ("SUB", "ENO", 10),
        ("ADD", "OUT", 10),
        ("ADD", "IN1", 4),
        ("ADD", "ENO", 13),
        ("ADD", "ENO", 94),
    ] {
        let before = d.to_bytes().unwrap();
        assert!(
            d.insert_iec_ld_function_output_wire(0, b.record_offset, name, pin, x)
                .is_err()
        );
        assert_eq!(d.to_bytes().unwrap(), before);
    }
    wire(&mut d, 0, "ENO", 10);
    let before = d.to_bytes().unwrap();
    assert!(
        d.insert_iec_ld_function_output_wire(0, b.record_offset, "ADD", "ENO", 10)
            .is_err()
    );
    assert_eq!(d.to_bytes().unwrap(), before);
    wire(&mut d, 0, "ENO", 13);
    wire(&mut d, 15, "OUT", 10);
    let b = d
        .ladder_programs()
        .remove(0)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 15)
        .unwrap();
    let before = d.to_bytes().unwrap();
    assert!(
        d.insert_iec_ld_function_output_wire(0, b.record_offset, "EQ", "ENO", 10)
            .is_err()
    );
    assert_eq!(d.to_bytes().unwrap(), before);
}
#[test]
fn comparison_without_assignment_retains_out_reference_and_wires_like_native() {
    let mut d =
        XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgi.xgwx")).unwrap();
    for (r, n, ops) in [
        (0, "ADD", vec!["%MW0", "1", "%MW2"]),
        (5, "SUB", vec!["%MW0", "1", "%MW4"]),
        (10, "DIV", vec!["%MW0", "1", "%MW6"]),
        (15, "EQ", vec!["%MW0", "1"]),
        (19, "INT_TO_UDINT", vec!["%MW0", "%MD10"]),
    ] {
        d.insert_iec_ld_function(
            0,
            r,
            7,
            n,
            &ops.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        )
        .unwrap();
    }
    for r in [0, 5, 10, 19] {
        wire(&mut d, r, "ENO", 10);
    }
    wire(&mut d, 15, "OUT", 10);
    assert_eq!(
        d.ladder_programs().remove(0).unwrap().data,
        payload(include_bytes!(
            "../fixtures/function-output-wires/all-native.xgwx"
        ))
    );
    for (row, pin, coil_row) in [
        (0, "ENO", 0),
        (5, "ENO", 5),
        (10, "ENO", 10),
        (15, "OUT", 16),
        (19, "ENO", 19),
    ] {
        d.insert_iec_ld_single_element(
            0,
            coil_row,
            94,
            "coil",
            "OUTPUT",
            &format!("%MX{}", 30 + row),
        )
        .unwrap();
        for x in (13..=91).step_by(3) {
            wire(&mut d, row, pin, x);
        }
    }
    assert_eq!(d.ladder_programs().remove(0).unwrap().data,
        payload(include_bytes!("../fixtures/function-output-wires/full-native.xgwx")));
    if let Ok(path) = std::env::var("IEC_OUTPUT_WIRE_CAPTURE") {
        d.write_to(path).unwrap();
    }
    assert!(
        d.ladder_programs()
            .remove(0)
            .unwrap()
            .iec_circuit_graph()
            .is_some()
    );
}
