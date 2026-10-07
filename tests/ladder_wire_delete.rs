#![cfg(feature = "write")]
use xgwx::{LadderBranchEdit, LadderCellEdit, LadderEditElement, LadderEditKind, XgwxDocument};
fn xgk() -> XgwxDocument {
    let mut doc =
        XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgk.xgwx")).unwrap();
    for row in [0, 4] {
        for (column, kind, operand) in [
            (0, LadderEditKind::NormallyOpen, "M00000"),
            (9, LadderEditKind::Output, "M00001"),
        ] {
            doc.edit_ladder_cell(
                0,
                &LadderCellEdit {
                    raw_y: row,
                    column,
                    expected: None,
                    replacement: Some(LadderEditElement {
                        kind,
                        operand: operand.into(),
                    }),
                },
            )
            .unwrap();
        }
    }
    doc
}
fn cells(doc: &xgwx::LadderProgramData) -> Vec<(u32, u8, String)> {
    doc.structure
        .rungs
        .iter()
        .flat_map(|row| {
            row.cells
                .iter()
                .map(|cell| (cell.raw_y, cell.raw_x, cell.value.clone()))
        })
        .collect()
}
fn native_payload(bytes: &[u8]) -> Vec<u8> {
    XgwxDocument::parse(bytes)
        .unwrap()
        .ladder_programs()
        .remove(0)
        .unwrap()
        .data
}
fn capture(name: &str, doc: &XgwxDocument) {
    if let Ok(dir) = std::env::var("WIRE_DELETE_CAPTURE_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            std::path::Path::new(&dir).join(format!("{name}.xgwx")),
            doc.to_verified_bytes().unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn xgk_horizontal_delete_preserves_elements_rows_and_rejects_stale_selection() {
    let mut doc = xgk();
    let before = doc.ladder_programs().remove(0).unwrap();
    let line = before.structure.horizontal_lines[0].clone();
    doc.delete_ladder_horizontal_wire(0, line.raw_y, line.raw_x_start, line.raw_x_end)
        .unwrap();
    let after = doc.ladder_programs().remove(0).unwrap();
    assert_eq!(cells(&before), cells(&after));
    capture("DELXGH", &doc);
    assert_eq!(
        after.data,
        native_payload(include_bytes!(
            "../fixtures/ladder-wire-delete/xgk-horizontal-native.xgwx"
        ))
    );
    assert_eq!(
        after.structure.horizontal_lines.len() + 1,
        before.structure.horizontal_lines.len()
    );
    let saved = doc.to_verified_bytes().unwrap();
    assert!(
        doc.delete_ladder_horizontal_wire(0, line.raw_y, line.raw_x_start, line.raw_x_end)
            .is_err()
    );
    assert_eq!(doc.to_verified_bytes().unwrap(), saved);
}
#[test]
fn xgk_vertical_delete_preserves_other_branches_and_shared_rows() {
    let mut doc = xgk();
    for boundary in [1, 2] {
        doc.edit_ladder_branch(
            0,
            &LadderBranchEdit {
                raw_y: 0,
                boundary,
                expected: false,
                present: true,
            },
        )
        .unwrap();
    }
    let before = doc.ladder_programs().remove(0).unwrap();
    doc.delete_ladder_vertical_wire(0, 3, 0, 4).unwrap();
    let after = doc.ladder_programs().remove(0).unwrap();
    assert_eq!(cells(&after), cells(&before));
    capture("DELXGV", &doc);
    assert_eq!(
        after.data,
        native_payload(include_bytes!(
            "../fixtures/ladder-wire-delete/xgk-vertical-native.xgwx"
        ))
    );
    assert_eq!(after.structure.vertical_lines.len(), 1);
    assert_eq!(after.structure.vertical_lines[0].raw_x, 6);
    let saved = doc.to_verified_bytes().unwrap();
    assert!(doc.delete_ladder_vertical_wire(0, 3, 0, 4).is_err());
    assert_eq!(doc.to_verified_bytes().unwrap(), saved);
}
#[test]
fn iec_horizontal_delete_preserves_contacts_coils_and_row_framing() {
    let mut doc =
        XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgi.xgwx")).unwrap();
    doc.insert_iec_ld_single_element(0, 0, 1, "contact", "NO", "%MX0")
        .unwrap();
    doc.insert_iec_ld_single_element(0, 0, 94, "coil", "OUTPUT", "%MX1")
        .unwrap();
    let before = doc.ladder_programs().remove(0).unwrap();
    let line = before.iec_geometry().unwrap().horizontal[0].clone();
    let stable: Vec<_> = before
        .iec_record_frames()
        .unwrap()
        .into_iter()
        .filter(|r| r.offset != line.offset)
        .map(|r| before.data[r.offset..r.end].to_vec())
        .collect();
    doc.delete_iec_ld_horizontal_wire_record(0, line.offset, line.start_x, line.end_x)
        .unwrap();
    let after = doc.ladder_programs().remove(0).unwrap();
    capture("DELIEH", &doc);
    assert_eq!(
        after.data,
        native_payload(include_bytes!(
            "../fixtures/ladder-wire-delete/iec-horizontal-native.xgwx"
        ))
    );
    assert!(after.iec_circuit_graph().is_some());
    assert!(after.iec_geometry().unwrap().horizontal.is_empty());
    assert_eq!(
        stable,
        after
            .iec_record_frames()
            .unwrap()
            .into_iter()
            .map(|r| after.data[r.offset..r.end].to_vec())
            .collect::<Vec<_>>()
    );
    let saved = doc.to_verified_bytes().unwrap();
    assert!(
        doc.delete_iec_ld_horizontal_wire_record(0, line.offset, line.start_x, line.end_x)
            .is_err()
    );
    assert_eq!(doc.to_verified_bytes().unwrap(), saved);
}

#[test]
fn isolated_iec_contact_and_coil_delete_leave_empty_rows_and_reject_stale_text() {
    for (name, kind, operand, raw_x) in
        [("contact", "NO", "%MX0", 1), ("coil", "OUTPUT", "%MX1", 94)]
    {
        let mut doc =
            XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgi.xgwx")).unwrap();
        doc.insert_iec_ld_single_element(0, 0, raw_x, name, kind, operand)
            .unwrap();
        let before = doc.ladder_programs().remove(0).unwrap();
        let offset = before.iec_record_frames().unwrap()[0].offset;
        let source = doc.to_verified_bytes().unwrap();
        assert!(
            doc.delete_iec_ld_isolated_element(0, offset, "stale")
                .is_err()
        );
        assert_eq!(doc.to_verified_bytes().unwrap(), source);
        doc.delete_iec_ld_isolated_element(0, offset, operand)
            .unwrap();
        let after = doc.ladder_programs().remove(0).unwrap();
        assert!(after.iec_record_frames().unwrap().is_empty());
        assert_eq!(after.iec_row_frames().unwrap()[0].row_index, 0);
        assert!(after.iec_circuit_graph().is_some());
        capture(
            if name == "contact" {
                "DELIEC"
            } else {
                "DELIEO"
            },
            &doc,
        );
    }
}
