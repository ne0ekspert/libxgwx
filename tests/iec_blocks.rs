#![cfg(feature = "write")]
use xgwx::*;
fn native() -> XgwxDocument {
    XgwxDocument::parse(include_bytes!(
        "../fixtures/iec-blocks/native-unassigned.xgwx"
    ))
    .unwrap()
}
fn insert(
    d: &mut XgwxDocument,
    row: u16,
    x: u8,
    name: &str,
    args: &[&str],
) -> Result<(), XgwxError> {
    d.program_mut().get(0)?.iec_ld()?.insert_function(
        row,
        x,
        name,
        &args.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
    )
}
const BLOCKS: &[(&str, &[&str])] = &[
    ("R_TRIG", &["Rise"]),
    ("F_TRIG", &["Fall"]),
    ("TON", &["OnTimer", "T#1s"]),
    ("TOF", &["OffTimer", "T#1s"]),
    ("TP", &["PulseTimer", "T#1s"]),
    ("RS", &["ResetLatch", "FALSE"]),
    ("SR", &["SetLatch", "FALSE"]),
    ("CTU_DINT", &["UpCounter", "FALSE", "DINT#10"]),
    ("CTD_DINT", &["DownCounter", "FALSE", "DINT#10"]),
    (
        "CTUD_DINT",
        &["BothCounter", "FALSE", "FALSE", "FALSE", "DINT#10"],
    ),
    ("NE", &["INT#1", "INT#2"]),
    ("MOD", &["INT#5", "INT#2", "%MW100"]),
    ("AND", &["TRUE", "FALSE", "%MX100"]),
    ("OR", &["TRUE", "FALSE", "%MX101"]),
    ("XOR", &["TRUE", "FALSE", "%MX102"]),
    ("NOT", &["TRUE", "%MX103"]),
    ("WORD_TO_UDINT", &["%MW200", "%MD100"]),
];
#[test]
fn fresh_blocks_preserve_native_declarations_and_existing_networks() {
    let mut d = native();
    let vars = d.variable().iec_local_symbols().remove(0).unwrap();
    for (i, (name, args)) in BLOCKS.iter().enumerate() {
        insert(
            &mut d,
            200 + i as u16 * 10,
            if i % 2 == 0 { 4 } else { 16 },
            name,
            args,
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    let parsed = XgwxDocument::parse(&d.to_verified_bytes().unwrap()).unwrap();
    let p = parsed.program().ladder_programs().remove(0).unwrap();
    assert_eq!(
        parsed.variable().iec_local_symbols().remove(0).unwrap(),
        vars
    );
    assert!(p.iec_circuit_layout().is_some());
    assert!(p.iec_function_operand_links().is_some());
    for (i, (name, _)) in BLOCKS.iter().enumerate() {
        assert!(
            p.iec_function_blocks()
                .unwrap()
                .iter()
                .any(|b| b.name.value == *name && b.row_index == 200 + i as u16 * 10)
        );
    }
}
#[test]
fn ctud_bool_outputs_match_native_f5_exactly() {
    let mut d = native();
    let p = d.program().ladder_programs().remove(0).unwrap();
    let b = p
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.name.value == "CTUD_DINT")
        .unwrap();
    for pin in ["QU", "QD"] {
        d.program_mut()
            .get(0)
            .unwrap()
            .iec_ld()
            .unwrap()
            .insert_function_output_wire(b.record_offset, "CTUD_DINT", pin, 7)
            .unwrap();
    }
    let expected = XgwxDocument::parse(include_bytes!(
        "../fixtures/iec-blocks/native-ctud-wires.xgwx"
    ))
    .unwrap();
    assert_eq!(
        d.program().ladder_programs().remove(0).unwrap().data,
        expected.program().ladder_programs().remove(0).unwrap().data
    );
    let b = d
        .program()
        .ladder_programs()
        .remove(0)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.name.value == "AND")
        .unwrap();
    d.program_mut()
        .get(0)
        .unwrap()
        .iec_ld()
        .unwrap()
        .insert_function_output_wire(b.record_offset, "AND", "OUT", 7)
        .unwrap();
    let expected = XgwxDocument::parse(include_bytes!(
        "../fixtures/iec-blocks/native-logic-wire.xgwx"
    ))
    .unwrap();
    assert_eq!(
        d.program().ladder_programs().remove(0).unwrap().data,
        expected.program().ladder_programs().remove(0).unwrap().data
    );
}
#[test]
fn incompatible_types_instances_and_overlap_are_atomic() {
    let mut d = native();
    for (name, args) in [
        ("TOF", vec!["Rise", "T#1s"]),
        ("CTU_DINT", vec!["UpCounter", "INT#1", "DINT#5"]),
        ("TON", vec!["OnTimer", "INT#1"]),
        ("MOD", vec!["REAL#1.0", "REAL#2.0", "%MW100"]),
        ("AND", vec!["TRUE", "%MW200", "%MX100"]),
    ] {
        let before = d.to_verified_bytes().unwrap();
        assert!(insert(&mut d, 200, 4, name, &args).is_err(), "{name}");
        assert_eq!(d.to_verified_bytes().unwrap(), before);
    }
    insert(&mut d, 200, 4, "TON", &["OnTimer", "T#1s"]).unwrap();
    let before = d.to_verified_bytes().unwrap();
    assert!(insert(&mut d, 201, 16, "TOF", &["OffTimer", "T#1s"]).is_err());
    assert_eq!(d.to_verified_bytes().unwrap(), before);
}

#[test]
fn native_strict_check_and_save_as_preserve_the_complete_generated_suite() {
    let generated =
        XgwxDocument::parse(include_bytes!("../fixtures/iec-blocks/generated.xgwx")).unwrap();
    let native = XgwxDocument::parse(include_bytes!(
        "../fixtures/iec-blocks/native-roundtrip.xgwx"
    ))
    .unwrap();
    assert_eq!(
        generated
            .program()
            .ladder_programs()
            .remove(0)
            .unwrap()
            .data,
        native.program().ladder_programs().remove(0).unwrap().data
    );
    assert_eq!(
        generated.variable().iec_local_symbols().remove(0).unwrap(),
        native.variable().iec_local_symbols().remove(0).unwrap()
    );
}

#[test]
fn bitwise_numeric_results_keep_their_destination_and_bool_results_wire() {
    let mut d = native();
    insert(
        &mut d,
        200,
        4,
        "AND",
        &["WORD#16#0001", "WORD#16#0002", "%MW100"],
    )
    .unwrap();
    let b = d
        .program()
        .ladder_programs()
        .remove(0)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 200)
        .unwrap();
    let before = d.to_verified_bytes().unwrap();
    assert!(
        d.program_mut()
            .get(0)
            .unwrap()
            .iec_ld()
            .unwrap()
            .insert_function_output_wire(b.record_offset, "AND", "OUT", 7)
            .is_err()
    );
    assert_eq!(d.to_verified_bytes().unwrap(), before);
    assert!(insert(&mut d, 210, 4, "AND", &["WORD#16#0001", "WORD#16#0002"]).is_err());
    insert(&mut d, 210, 4, "AND", &["TRUE", "FALSE"]).unwrap();
    let b = d
        .program()
        .ladder_programs()
        .remove(0)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 210)
        .unwrap();
    d.program_mut()
        .get(0)
        .unwrap()
        .iec_ld()
        .unwrap()
        .insert_function_output_wire(b.record_offset, "AND", "OUT", 7)
        .unwrap();
}

#[test]
fn fresh_ton_after_a_contact_does_not_require_et_even_at_a_terminal_site() {
    let mut d = XgwxDocument::parse(include_bytes!(
        "../fixtures/iec-blocks/empty-with-instances.xgwx"
    ))
    .unwrap();
    d.program_mut()
        .get(0)
        .unwrap()
        .iec_ld()
        .unwrap()
        .insert_single_element(0, 1, "contact", "NO", "%MX0")
        .unwrap();
    insert(&mut d, 0, 7, "TON", &["OnTimer", "T#1s"]).unwrap();
    assert_eq!(
        d.program()
            .ladder_programs()
            .remove(0)
            .unwrap()
            .iec_function_operand_links()
            .unwrap()
            .len(),
        1
    );
}
