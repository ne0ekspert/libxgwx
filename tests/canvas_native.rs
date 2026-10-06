#![cfg(feature = "write")]
use xgwx::{LadderCellEdit, LadderEditElement, LadderEditKind, XgwxDocument};

fn payload(doc: &XgwxDocument) -> Vec<u8> {
    doc.ladder_programs().remove(0).unwrap().data
}

#[test]
fn sparse_iec_canvas_placements_survive_native_save_as_exactly() {
    for (rows, native) in [
        (
            [100, 300],
            include_bytes!("../fixtures/canvas-rows/iec-row300-native.xgwx").as_slice(),
        ),
        (
            [0, 16382],
            include_bytes!("../fixtures/canvas-rows/iec-row16382-native.xgwx").as_slice(),
        ),
    ] {
        let mut doc =
            XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgi.xgwx")).unwrap();
        for row in rows {
            doc.insert_iec_ld_single_element(0, row, 1, "contact", "NO", "%MX0")
                .unwrap();
            doc.insert_iec_ld_single_element(0, row, 94, "coil", "OUTPUT", "%MX1")
                .unwrap();
        }
        let saved = XgwxDocument::parse(native).unwrap();
        assert_eq!(payload(&doc), payload(&saved));
        assert_eq!(
            saved
                .ladder_programs()
                .remove(0)
                .unwrap()
                .iec_row_frames()
                .unwrap()
                .iter()
                .map(|row| row.row_index)
                .collect::<Vec<_>>(),
            rows
        );
        assert!(
            saved
                .ladder_programs()
                .remove(0)
                .unwrap()
                .iec_circuit_graph()
                .is_some()
        );
    }
}

#[test]
fn growing_xgk_canvas_placements_survive_native_save_as_exactly() {
    let mut doc =
        XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgk.xgwx")).unwrap();
    for row in 0..=60 {
        doc.insert_ladder_row(0, row * 4).unwrap();
        if ![0, 60].contains(&row) {
            continue;
        }
        for (column, kind, operand) in [
            (0, LadderEditKind::NormallyOpen, "M00000"),
            (9, LadderEditKind::Output, "M00001"),
        ] {
            doc.edit_ladder_cell(
                0,
                &LadderCellEdit {
                    raw_y: row * 4,
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
    let native = XgwxDocument::parse(include_bytes!(
        "../fixtures/canvas-rows/xgk-row60-native.xgwx"
    ))
    .unwrap();
    assert_eq!(payload(&doc), payload(&native));
}

#[test]
fn wider_iec_coordinates_reject_structural_edits_without_changing_bytes() {
    for native in [
        include_bytes!("../fixtures/canvas-rows/iec-row16384-native.xgwx").as_slice(),
        include_bytes!("../fixtures/canvas-rows/iec-row65535-native.xgwx").as_slice(),
    ] {
        let mut doc = XgwxDocument::parse(native).unwrap();
        let before = doc.to_verified_bytes().unwrap();
        assert!(
            doc.insert_iec_ld_single_element(0, 10, 1, "contact", "NO", "%MX8")
                .is_err()
        );
        assert_eq!(doc.to_verified_bytes().unwrap(), before);
    }
}

#[test]
fn xgk_native_row65_is_decoded_and_editable() {
    let mut doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/canvas-rows/xgk-row65-native.xgwx"
    ))
    .unwrap();
    assert!(
        doc.ladder_programs()[0]
            .as_ref()
            .unwrap()
            .structure
            .rungs
            .iter()
            .any(|r| r.raw_y == 260 && r.cells.iter().any(|c| c.value == "M00002"))
    );
    let before = payload(&doc);
    doc.edit_ladder_comment(
        0,
        &xgwx::LadderCommentEdit {
            kind: xgwx::LadderCommentKind::Rung,
            raw_y: 256,
            expected: None,
            replacement: "Wide comment".into(),
        },
    )
    .unwrap();
    doc.delete_ladder_rung_comment(0, 256, "Wide comment")
        .unwrap();
    assert_eq!(payload(&doc), before);
}

#[test]
fn sparse_xgk_row65_matches_native_records() {
    let mut doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/canvas-rows/xgk-row60-native.xgwx"
    ))
    .unwrap();
    for (column, kind, operand) in [
        (0, LadderEditKind::NormallyOpen, "M00002"),
        (9, LadderEditKind::Output, "M00003"),
    ] {
        doc.edit_ladder_cell(
            0,
            &LadderCellEdit {
                raw_y: 260,
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
    let native = XgwxDocument::parse(include_bytes!(
        "../fixtures/canvas-rows/xgk-row65-native.xgwx"
    ))
    .unwrap();
    assert_eq!(payload(&doc), payload(&native));
}

#[test]
fn wide_xgk_sparse_projects_survive_native_save_as_exactly() {
    for (last, native) in [
        (
            256,
            include_bytes!("../fixtures/canvas-rows/xgk-row256-native.xgwx").as_slice(),
        ),
        (
            65534,
            include_bytes!("../fixtures/canvas-rows/xgk-row65534-native.xgwx").as_slice(),
        ),
    ] {
        let mut doc =
            XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgk.xgwx")).unwrap();
        for (row, input, output) in [(0, "M00000", "M00001"), (last, "M00002", "M00003")] {
            for (column, kind, operand) in [
                (0, LadderEditKind::NormallyOpen, input),
                (9, LadderEditKind::Output, output),
            ] {
                doc.edit_ladder_cell(
                    0,
                    &LadderCellEdit {
                        raw_y: row * 4,
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
        let saved = XgwxDocument::parse(native).unwrap();
        assert_eq!(payload(&doc), payload(&saved));
        assert!(
            saved.ladder_programs()[0]
                .as_ref()
                .unwrap()
                .structure
                .rungs
                .iter()
                .any(|r| r.raw_y == last * 4 && r.cells.iter().any(|c| c.value == "M00002"))
        );
    }
}
