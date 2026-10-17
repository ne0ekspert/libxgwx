#![cfg(feature = "write")]
use xgwx::*;
fn doc() -> XgwxDocument {
    XgwxDocument::parse(include_bytes!(
        "../fixtures/iec-ff/empty-with-instance.xgwx"
    ))
    .unwrap()
}
fn insert(d: &mut XgwxDocument, row: u16, x: u8, instance: &str) -> Result<(), XgwxError> {
    d.program_mut()
        .get(0)?
        .iec_ld()?
        .insert_function(row, x, "FF", &[instance.into()])
}
#[test]
fn fresh_ff_has_native_two_row_layout_and_preserves_instances() {
    let mut d = doc();
    let vars = d.variable().iec_local_symbols().remove(0).unwrap();
    for (row, x) in [(0, 4), (6, 16), (12, 91)] {
        insert(&mut d, row, x, "FF").unwrap();
    }
    let d = XgwxDocument::parse(&d.to_verified_bytes().unwrap()).unwrap();
    let p = d.program().ladder_programs().remove(0).unwrap();
    let blocks = p.iec_function_blocks().unwrap();
    assert_eq!(blocks.len(), 3);
    for (b, (row, x)) in blocks.iter().zip([(0, 4), (6, 16), (12, 91)]) {
        assert_eq!((b.row_index, b.raw_x), (row, x));
        assert_eq!(b.name.value, "FF");
        assert_eq!(b.instance.as_ref().unwrap().value, "FF");
        assert_eq!(b.control_input.name.value, "CLK");
    }
    assert_eq!(p.iec_function_references().unwrap().len(), 3);
    assert!(p.iec_circuit_graph().is_some());
    assert_eq!(d.variable().iec_local_symbols().remove(0).unwrap(), vars);
}
#[test]
fn invalid_instances_coordinates_and_overlap_are_atomic() {
    let mut d = doc();
    insert(&mut d, 3, 16, "FF").unwrap();
    for (row, x, name) in [
        (3, 16, "FF"),
        (4, 16, "FF"),
        (2, 16, "FF"),
        (9, 1, "FF"),
        (9, 5, "FF"),
        (9, 94, "FF"),
        (16383, 4, "FF"),
        (9, 4, "missing"),
    ] {
        let before = d.to_verified_bytes().unwrap();
        assert!(insert(&mut d, row, x, name).is_err(), "{row}/{x}/{name}");
        assert_eq!(d.to_verified_bytes().unwrap(), before);
    }
    let before = d.to_verified_bytes().unwrap();
    assert!(
        d.program_mut()
            .get(0)
            .unwrap()
            .iec_ld()
            .unwrap()
            .insert_function(9, 4, "FF", &[])
            .is_err()
    );
    assert_eq!(d.to_verified_bytes().unwrap(), before);
}

#[test]
fn contact_prefix_is_preserved_and_completed_by_ff() {
    for x in [4, 10, 16] {
        let mut d = doc();
        d.program_mut()
            .get(0)
            .unwrap()
            .iec_ld()
            .unwrap()
            .insert_single_element(3, 1, "contact", "NO", "%MX0")
            .unwrap();
        insert(&mut d, 3, x, "FF").unwrap();
        let p = d.program().ladder_programs().remove(0).unwrap();
        assert_eq!(
            p.iec_record_frames()
                .unwrap()
                .iter()
                .filter(|r| matches!(r.kind, IecRecordKind::Contact(_)))
                .count(),
            1
        );
        assert_eq!(p.iec_function_blocks().unwrap()[0].raw_x, x);
        assert!(p.iec_circuit_graph().is_some());
    }
}

#[test]
fn native_save_as_preserves_fresh_ff_and_all_native_instance_fields() {
    let generated =
        XgwxDocument::parse(include_bytes!("../fixtures/iec-ff/fresh-generated.xgwx")).unwrap();
    let native = XgwxDocument::parse(include_bytes!(
        "../fixtures/iec-ff/fresh-native-roundtrip.xgwx"
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
fn contact_fed_ff_matches_native_insertion_exactly() {
    let mut generated =
        XgwxDocument::parse(include_bytes!("../fixtures/iec-ff/contact-prefix.xgwx")).unwrap();
    let native =
        XgwxDocument::parse(include_bytes!("../fixtures/iec-ff/contact-native.xgwx")).unwrap();
    insert(&mut generated, 20, 10, "FF").unwrap();
    let a = generated
        .program()
        .ladder_programs()
        .remove(0)
        .unwrap()
        .data;
    let b = native.program().ladder_programs().remove(0).unwrap().data;
    let differences = (0..a.len().max(b.len()))
        .filter(|&i| a.get(i) != b.get(i))
        .map(|i| (i, a.get(i), b.get(i)))
        .collect::<Vec<_>>();
    assert!(
        differences.is_empty(),
        "{} differences {:?}",
        differences.len(),
        &differences[..differences.len().min(20)]
    );
}

#[test]
fn q_wiring_matches_native_f5_without_an_output_variable() {
    let mut d =
        XgwxDocument::parse(include_bytes!("../fixtures/iec-ff/contact-native.xgwx")).unwrap();
    let native =
        XgwxDocument::parse(include_bytes!("../fixtures/iec-ff/q-wires-native.xgwx")).unwrap();
    let block = d
        .program()
        .ladder_programs()
        .remove(0)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 20)
        .unwrap();
    for x in [13, 16] {
        d.program_mut()
            .get(0)
            .unwrap()
            .iec_ld()
            .unwrap()
            .insert_function_output_wire(block.record_offset, "FF", "Q", x)
            .unwrap();
    }
    assert_eq!(
        d.program().ladder_programs().remove(0).unwrap().data,
        native.program().ladder_programs().remove(0).unwrap().data
    );
    assert_eq!(
        d.variable().iec_local_symbols().remove(0).unwrap(),
        native.variable().iec_local_symbols().remove(0).unwrap()
    );
}

#[test]
fn q_wires_directly_without_any_output_assignment() {
    let mut d = doc();
    insert(&mut d, 0, 4, "FF").unwrap();
    let vars = d.variable().iec_local_symbols().remove(0).unwrap();
    let block = d
        .program()
        .ladder_programs()
        .remove(0)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .remove(0);
    for x in [7, 10] {
        d.program_mut()
            .get(0)
            .unwrap()
            .iec_ld()
            .unwrap()
            .insert_function_output_wire(block.record_offset, "FF", "Q", x)
            .unwrap();
    }
    let p = d.program().ladder_programs().remove(0).unwrap();
    assert_eq!(
        p.iec_record_frames()
            .unwrap()
            .iter()
            .filter(|r| r.kind == IecRecordKind::ShortWire)
            .count(),
        2
    );
    assert!(p.iec_function_operand_links().unwrap().is_empty());
    assert_eq!(d.variable().iec_local_symbols().remove(0).unwrap(), vars);
    assert!(p.iec_circuit_layout().is_some());
    let before = d.to_verified_bytes().unwrap();
    for (name, pin, x) in [
        ("FF", "CLK", 13),
        ("R_TRIG", "Q", 13),
        ("FF", "Q", 16),
        ("FF", "Q", 7),
    ] {
        assert!(
            d.program_mut()
                .get(0)
                .unwrap()
                .iec_ld()
                .unwrap()
                .insert_function_output_wire(block.record_offset, name, pin, x)
                .is_err()
        );
        assert_eq!(d.to_verified_bytes().unwrap(), before);
    }
}
