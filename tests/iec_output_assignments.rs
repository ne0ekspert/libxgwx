#![cfg(feature = "write")]
use xgwx::XgwxDocument;
fn base() -> XgwxDocument {
    XgwxDocument::parse(include_bytes!(
        "../fixtures/function-output-wires/base.xgwx"
    ))
    .unwrap()
}
fn output(d: &XgwxDocument, row: u16) -> (usize, String, usize, String, u8) {
    let p = d.ladder_programs().remove(0).unwrap();
    let b = p
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == row)
        .unwrap();
    let pin = b.pins.iter().find(|p| p.name.value == "OUT").unwrap();
    let link = p
        .iec_function_operand_links()
        .unwrap()
        .into_iter()
        .find(|l| l.target_record_offset == b.record_offset && l.is_output)
        .unwrap();
    let s = p
        .strings
        .clone()
        .into_iter()
        .find(|s| s.offset == link.record_offset + 15)
        .unwrap();
    (
        s.offset,
        s.value,
        b.record_offset,
        b.name.value,
        pin.reference_ordinal.unwrap(),
    )
}
#[test]
fn clearing_outputs_keeps_rows_bodies_and_references_and_restores_exactly() {
    let mut d = base();
    let original = d.to_bytes().unwrap();
    for row in [0, 5, 10, 15, 19] {
        let p = d.ladder_programs().remove(0).unwrap();
        let (offset, value, _, _, ordinal) = output(&d, row);
        d.delete_iec_ld_function_output_operand(0, offset, &value)
            .unwrap();
        let after = d.ladder_programs().remove(0).unwrap();
        let old = p
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row)
            .unwrap();
        let new = after
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row)
            .unwrap();
        assert_eq!(
            &p.data[old.record_offset..old.record_end],
            &after.data[new.record_offset..new.record_end]
        );
        assert_eq!(
            p.iec_function_references().unwrap().len(),
            after.iec_function_references().unwrap().len()
        );
        assert_eq!(
            p.iec_row_frames()
                .unwrap()
                .iter()
                .map(|r| (r.group_index, r.row_index))
                .collect::<Vec<_>>(),
            after
                .iec_row_frames()
                .unwrap()
                .iter()
                .map(|r| (r.group_index, r.row_index))
                .collect::<Vec<_>>()
        );
        // Restore by the current body offset; later blocks have moved.
        let b = after
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row)
            .unwrap();
        d.assign_iec_ld_function_output_operand(0, b.record_offset, &b.name.value, ordinal, &value)
            .unwrap();
        assert_eq!(d.to_bytes().unwrap(), original);
    }
}
#[test]
fn input_clear_stale_selection_and_bad_output_assignment_are_atomic() {
    let mut d = base();
    let (offset, value, _, _, ordinal) = output(&d, 0);
    let before = d.to_bytes().unwrap();
    assert!(
        d.delete_iec_ld_function_output_operand(0, offset, "stale")
            .is_err()
    );
    let p = d.ladder_programs().remove(0).unwrap();
    let input = p
        .iec_function_operand_links()
        .unwrap()
        .into_iter()
        .find(|l| !l.is_output)
        .unwrap();
    assert!(
        d.update_iec_ld_function_operand(0, input.record_offset + 15, "%MW0", "")
            .is_err()
    );
    assert_eq!(d.to_bytes().unwrap(), before);
    d.update_iec_ld_function_operand(0, offset, &value, "")
        .unwrap();
    let before = d.to_bytes().unwrap();
    let b = d
        .ladder_programs()
        .remove(0)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .remove(0);
    for value in ["1", "%MX0", "", "%IW0"] {
        assert!(
            d.assign_iec_ld_function_output_operand(0, b.record_offset, "ADD", ordinal, value)
                .is_err()
        );
        assert_eq!(d.to_bytes().unwrap(), before);
    }
    d.assign_iec_ld_function_output_operand(0, b.record_offset, "ADD", ordinal, "%MW2")
        .unwrap();
}
#[test]
fn clear_capture() {
    let mut d = base();
    for row in [0, 5, 10, 15, 19] {
        let (offset, value, ..) = output(&d, row);
        d.delete_iec_ld_function_output_operand(0, offset, &value)
            .unwrap();
    }
    if let Ok(path) = std::env::var("IEC_CLEAR_CAPTURE") {
        d.write_to(path).unwrap();
    }
    let native = XgwxDocument::parse(include_bytes!(
        "../fixtures/function-output-wires/cleared-native.xgwx"
    ))
    .unwrap();
    assert_eq!(
        d.ladder_programs().remove(0).unwrap().data,
        native.ladder_programs().remove(0).unwrap().data
    );
    let b = d
        .ladder_programs()
        .remove(0)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 19)
        .unwrap();
    d.assign_iec_ld_function_output_operand(0, b.record_offset, "INT_TO_UDINT", 2, "%MD10")
        .unwrap();
    let native = XgwxDocument::parse(include_bytes!(
        "../fixtures/function-output-wires/conversion-restored-native.xgwx"
    ))
    .unwrap();
    assert_eq!(
        d.ladder_programs().remove(0).unwrap().data,
        native.ladder_programs().remove(0).unwrap().data
    );
    assert!(
        d.ladder_programs()
            .remove(0)
            .unwrap()
            .iec_circuit_graph()
            .is_some()
    );
}
#[test]
#[ignore = "set LIBXGWX_IEC_FIXTURE for private project coverage"]
fn project_output_clear_and_restore_coverage() {
    let source = XgwxDocument::from_path(std::env::var("LIBXGWX_IEC_FIXTURE").unwrap()).unwrap();
    let programs = source
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let mut cleared = 0;
    let mut restored = 0;
    let mut empty = 0;
    for (index, p) in programs.iter().enumerate() {
        let blocks = p.iec_function_blocks().unwrap();
        for link in p
            .iec_function_operand_links()
            .unwrap()
            .into_iter()
            .filter(|l| l.is_output)
        {
            let block = blocks
                .iter()
                .find(|b| b.record_offset == link.target_record_offset)
                .unwrap();
            if !block
                .pins
                .iter()
                .any(|p| p.name.value == "OUT" && p.reference_ordinal == Some(link.ordinal))
            {
                continue;
            }
            let Some(value) = p
                .strings
                .iter()
                .find(|s| s.offset == link.record_offset + 15)
            else {
                empty += 1;
                continue;
            };
            let mut d = source.clone();
            d.delete_iec_ld_function_output_operand(index, value.offset, &value.value)
                .unwrap();
            cleared += 1;
            if d.assign_iec_ld_function_output_operand(
                index,
                block.record_offset,
                &block.name.value,
                link.ordinal,
                &value.value,
            )
            .is_ok()
            {
                let after = d.ladder_programs().remove(index).unwrap().data;
                let differences = (0..p.data.len().max(after.len()))
                    .filter(|&i| p.data.get(i) != after.get(i))
                    .take(8)
                    .collect::<Vec<_>>();
                assert!(
                    after == p.data,
                    "restore changed program {index} {} row {} at {:?}",
                    block.name.value,
                    block.row_index,
                    differences
                );
                restored += 1;
            }
        }
    }
    println!(
        "output assignments: {cleared} cleared; {restored} restored exactly; {empty} already blank"
    );
    assert!(cleared > 0);
}
#[test]
fn existing_empty_output_record_can_be_reassigned() {
    use base64::Engine;
    use std::io::Write;
    let mut d = base();
    let original = d.ladder_programs().remove(0).unwrap().data;
    let p = d.ladder_programs().remove(0).unwrap();
    let (offset, _, block, _, ordinal) = output(&d, 0);
    let string = p.strings.iter().find(|s| s.offset == offset).unwrap();
    let mut data = p.data.clone();
    data.splice(offset..string.end_offset, [0xff, 0xfe, 0xff, 0]);
    let parsed = roxmltree::Document::parse(&d.xml).unwrap();
    let old = parsed
        .descendants()
        .find(|n| n.has_tag_name("ProgramData"))
        .unwrap()
        .text()
        .unwrap()
        .to_string();
    let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::best());
    encoder.write_all(&data).unwrap();
    let encoded = base64::engine::general_purpose::STANDARD.encode(encoder.finish().unwrap());
    d.xml = d.xml.replacen(&old, &encoded, 1);
    d = XgwxDocument::parse(&d.to_bytes().unwrap()).unwrap();
    d.assign_iec_ld_function_output_operand(0, block, "ADD", ordinal, "%MW2")
        .unwrap();
    assert_eq!(d.ladder_programs().remove(0).unwrap().data, original);
}
