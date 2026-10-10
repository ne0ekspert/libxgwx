#![cfg(feature = "write")]
use xgwx::{LadderCellEdit, LadderEditElement, LadderEditKind, XgwxDocument};
#[test]
fn native_empty_iec_accepts_first_contact_and_coil() {
    let mut doc =
        XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgi.xgwx")).unwrap();
    let program = doc.ladder_programs().remove(0).unwrap();
    assert_eq!(program.data, [0; 8]);
    assert!(program.iec_circuit_graph().is_some());
    doc.insert_iec_ld_single_element(0, 0, 1, "contact", "NO", "%MX0")
        .unwrap();
    doc.insert_iec_ld_single_element(0, 0, 94, "coil", "OUTPUT", "%MX1")
        .unwrap();
    let reparsed = XgwxDocument::parse(&doc.to_bytes().unwrap()).unwrap();
    let program = reparsed.ladder_programs().remove(0).unwrap();
    assert_eq!(program.iec_record_frames().unwrap().len(), 3);
    assert!(program.iec_circuit_graph().is_some());
    let mut malformed = program;
    malformed.data = vec![0; 9];
    assert!(malformed.iec_row_frames().is_none());
    malformed.data = vec![0; 8];
    malformed.project_type = Some(1);
    assert!(malformed.iec_row_frames().is_none());
}
#[test]
fn native_empty_xgk_accepts_first_contact_and_coil() {
    let mut doc =
        XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgk.xgwx")).unwrap();
    for (column, kind, operand) in [
        (0, LadderEditKind::NormallyOpen, "M00000"),
        (9, LadderEditKind::Output, "M00001"),
    ] {
        doc.edit_ladder_cell(
            0,
            &LadderCellEdit {
                raw_y: 0,
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
    let reparsed = XgwxDocument::parse(&doc.to_bytes().unwrap()).unwrap();
    let program = reparsed.ladder_programs().remove(0).unwrap();
    assert!(
        program.structure.rungs[0]
            .cells
            .iter()
            .any(|cell| cell.value == "M00000")
    );
    assert!(
        program.structure.rungs[0]
            .cells
            .iter()
            .any(|cell| cell.value == "M00001")
    );
}

#[test]
fn native_blank_projects_accept_network_module_in_first_slot() {
    for source in [
        include_bytes!("../fixtures/empty-projects/new-xgk.xgwx").as_slice(),
        include_bytes!("../fixtures/empty-projects/new-xgi.xgwx").as_slice(),
    ] {
        let mut doc = XgwxDocument::parse(source).unwrap();
        if doc.ladder_programs()[0].as_ref().unwrap().project_type == Some(2) {
            let original = doc.xml.clone();
            assert!(doc.insert_module(0, 0, "XGL-EFMT(B)").is_err());
            assert_eq!(doc.xml, original);
            continue;
        }
        let before = doc.ladder_programs().remove(0).unwrap().data;
        doc.insert_module(0, 0, "XGL-EFMT(B)").unwrap();
        let reparsed = XgwxDocument::parse(&doc.to_bytes().unwrap()).unwrap();
        assert_eq!(reparsed.ladder_programs().remove(0).unwrap().data, before);
        assert_eq!(
            reparsed
                .networks()
                .iter()
                .flat_map(|n| &n.modules)
                .filter(|m| m.base == Some(0) && m.slot == Some(0) && m.id == Some(23041))
                .count(),
            1
        );
        assert_eq!(reparsed.fenet_config_infos().len(), 1);
        doc.delete_module(0, 0).unwrap();
        assert!(doc.fenet_config_infos().is_empty());
        assert!(doc.networks().iter().all(|n| n.modules.is_empty()));
    }
}

#[test]
fn sparse_iec_canvas_contacts_and_coils_preserve_existing_rows() {
    let mut doc =
        XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgi.xgwx")).unwrap();
    doc.insert_iec_ld_single_element(0, 100, 1, "contact", "NO", "%MX0")
        .unwrap();
    let first = doc.ladder_programs().remove(0).unwrap();
    doc.insert_iec_ld_single_element(0, 300, 94, "coil", "OUTPUT", "%MX1")
        .unwrap();
    let program = doc.ladder_programs().remove(0).unwrap();
    assert_eq!(
        program
            .iec_row_frames()
            .unwrap()
            .iter()
            .map(|row| row.row_index)
            .collect::<Vec<_>>(),
        vec![100, 300]
    );
    let original_row = first.iec_row_frames().unwrap().remove(0);
    let preserved_row = program.iec_row_frames().unwrap().remove(0);
    assert_eq!(
        &first.data[original_row.start..original_row.end],
        &program.data[preserved_row.start..preserved_row.end]
    );
    let saved = doc.to_verified_bytes().unwrap();
    assert!(
        doc.insert_iec_ld_single_element(0, 100, 1, "contact", "NO", "%MX2")
            .is_err()
    );
    assert!(
        doc.insert_iec_ld_single_element(0, 16383, 1, "contact", "NO", "%MX2")
            .is_err()
    );
    assert_eq!(doc.to_verified_bytes().unwrap(), saved);
}
