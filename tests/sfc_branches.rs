#[cfg(feature = "write")]
use xgwx::SfcSequencePatch;
use xgwx::XgwxDocument;

#[test]
fn native_balanced_branch_kinds_and_three_paths_are_decoded_with_st() {
    for (file, kind, paths) in [
        ("alternative", "alternative", 2),
        ("parallel", "parallel", 2),
        ("parallel-three", "parallel", 3),
    ] {
        let doc =
            XgwxDocument::from_path(format!("fixtures/sfc/branch-{file}-native.xgwx")).unwrap();
        let program = &doc.sfc_programs()[0];
        let block = &program.blocks[0];
        let rows = block
            .editable_rows
            .as_ref()
            .expect("native branch chart decoded");
        assert_eq!(
            rows.iter()
                .filter(|r| r.kind == format!("{kind}_split"))
                .count(),
            1
        );
        assert_eq!(
            rows.iter()
                .find(|r| r.kind == format!("{kind}_split"))
                .unwrap()
                .branch_end,
            Some((paths - 1) * 2)
        );
        assert!(rows.iter().any(|r| r.action_code.is_some()));
        assert!(rows.iter().any(|r| r.transition_code.is_some()));
        assert_eq!(program.variables_error, None);
    }
}

#[cfg(feature = "write")]
fn apply(doc: &mut XgwxDocument, rows: Vec<xgwx::SfcRow>) -> Result<(), xgwx::XgwxError> {
    let block = doc.sfc_programs()[0].blocks[0].clone();
    doc.replace_sfc_sequence(&SfcSequencePatch {
        program_index: 0,
        block_index: 0,
        expected_entities: block.entities,
        expected_rows: block.editable_rows,
        rows,
    })
}

#[cfg(feature = "write")]
#[test]
fn branch_edits_preserve_existing_st_and_declarations_and_roundtrip() {
    for file in ["alternative", "parallel", "parallel-three"] {
        let mut doc =
            XgwxDocument::from_path(format!("fixtures/sfc/branch-{file}-native.xgwx")).unwrap();
        let original = doc.sfc_programs()[0].clone();
        let mut rows = original.blocks[0].editable_rows.clone().unwrap();
        let index = rows
            .iter()
            .position(|r| {
                r.position.as_ref().unwrap().column == 2
                    && ["step", "transition"].contains(&r.kind.as_str())
            })
            .unwrap();
        if rows[index].kind == "step" {
            rows[index].title = "SideStep".into();
            rows[index].action = Some("%MX12".into());
        } else {
            rows[index].title = "%MX3".into();
        }
        apply(&mut doc, rows.clone()).unwrap();
        let saved = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
        assert_eq!(
            saved.sfc_programs()[0].blocks[0].editable_rows.as_ref(),
            Some(&rows)
        );
        assert_eq!(saved.sfc_programs()[0].variables, original.variables);
        let sources = |rows: &[xgwx::SfcRow]| {
            rows.iter()
                .filter_map(|r| r.action_code.as_ref().or(r.transition_code.as_ref()))
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(
            sources(&rows),
            sources(original.blocks[0].editable_rows.as_ref().unwrap())
        );
    }
}

#[cfg(feature = "write")]
#[test]
fn malformed_branch_layouts_are_rejected_atomically() {
    let source = XgwxDocument::from_path("fixtures/sfc/branch-alternative-native.xgwx").unwrap();
    let original = source.sfc_programs()[0].blocks[0]
        .editable_rows
        .clone()
        .unwrap();
    for case in 0..7 {
        let mut doc = source.clone();
        let mut rows = original.clone();
        let split = rows
            .iter()
            .position(|r| r.kind == "alternative_split")
            .unwrap();
        let join = rows
            .iter()
            .position(|r| r.kind == "alternative_join")
            .unwrap();
        let side = rows
            .iter()
            .position(|r| r.position.as_ref().unwrap().column == 2)
            .unwrap();
        match case {
            0 => {
                rows.remove(join);
            }
            1 => rows[join].kind = "parallel_join".into(),
            2 => rows[split].branch_end = Some(16),
            3 => rows[side].position.as_mut().unwrap().column = 0,
            4 => rows[side].position.as_mut().unwrap().row += 1,
            5 => rows[side].initial = true,
            _ => rows[side].position = None,
        }
        assert!(apply(&mut doc, rows).is_err());
        assert_eq!(doc.xml, source.xml);
    }
    let mut stale = source.clone();
    let mut patch = SfcSequencePatch {
        program_index: 0,
        block_index: 0,
        expected_entities: source.sfc_programs()[0].blocks[0].entities.clone(),
        expected_rows: Some(original.clone()),
        rows: original,
    };
    patch.expected_rows.as_mut().unwrap()[0].title = "stale".into();
    assert!(stale.replace_sfc_sequence(&patch).is_err());
    assert_eq!(stale.xml, source.xml);
}

#[cfg(feature = "write")]
#[test]
fn unknown_branch_priorities_and_connectors_stay_guarded() {
    let source = XgwxDocument::from_path("fixtures/sfc/branch-alternative-native.xgwx").unwrap();
    for xml in [
        source.xml.replacen(
            "<EntityBranch Priority=\"-1\" BranchType=\"0\">",
            "<EntityBranch Priority=\"-1\" BranchType=\"0\"><Extension/>",
            1,
        ),
        source.xml.replacen("Priority=\"-1\"", "Priority=\"0\"", 1),
        source.xml.replacen("Type=\"8\"", "Type=\"7\"", 1),
        source
            .xml
            .replacen("BranchType=\"1\"", "BranchType=\"3\"", 1),
    ] {
        assert_ne!(xml, source.xml);
        let mut seed = source.clone();
        seed.xml = xml;
        let doc = XgwxDocument::parse(&seed.to_bytes().unwrap()).unwrap();
        assert!(doc.sfc_programs()[0].blocks[0].editable_rows.is_none());
    }
}

#[test]
fn xg5000_resaves_retain_branch_topology_st_sources_and_declarations() {
    for kind in [
        "alternative",
        "parallel",
        "alternative-three",
        "parallel-three",
    ] {
        let generated =
            XgwxDocument::from_path(format!("fixtures/sfc/branch-{kind}-generated.xgwx")).unwrap();
        let native =
            XgwxDocument::from_path(format!("fixtures/sfc/branch-{kind}-roundtrip.xgwx")).unwrap();
        let before = &generated.sfc_programs()[0];
        let after = &native.sfc_programs()[0];
        assert_eq!(
            before.blocks[0].editable_rows, after.blocks[0].editable_rows,
            "{kind}"
        );
        assert!(after.blocks[0].editable_rows.is_some());
        assert_eq!(before.variables, after.variables, "{kind}");
        assert_eq!(after.variables_error, None);
    }
}

#[cfg(feature = "write")]
#[test]
fn independent_parallel_paths_retain_native_padding_and_reject_invalid_sequences() {
    let source =
        XgwxDocument::from_path("fixtures/sfc/branch-independent-parallel-generated.xgwx").unwrap();
    let saved =
        XgwxDocument::from_path("fixtures/sfc/branch-independent-parallel-roundtrip.xgwx").unwrap();
    let rows = source.sfc_programs()[0].blocks[0]
        .editable_rows
        .clone()
        .unwrap();
    assert_eq!(
        saved.sfc_programs()[0].blocks[0].editable_rows.as_ref(),
        Some(&rows)
    );
    assert_eq!(
        source.sfc_programs()[0].variables,
        saved.sfc_programs()[0].variables
    );
    assert_eq!(
        rows.iter()
            .filter(|r| r.position.as_ref().unwrap().column == 2
                && matches!(r.kind.as_str(), "step" | "transition"))
            .count(),
        3
    );
    let before = source.to_bytes().unwrap();
    let mut doc = source;
    for initial in [false, true] {
        let mut invalid = rows.clone();
        let row = invalid.iter_mut().find(|r| r.title == "SideNext").unwrap();
        if initial {
            row.initial = true;
        } else {
            row.kind = "transition".into();
            row.title = "%MX21".into();
        }
        assert!(apply(&mut doc, invalid).is_err());
        assert_eq!(doc.to_bytes().unwrap(), before);
    }
}

#[test]
fn independent_alternative_paths_survive_native_save_as() {
    let source =
        XgwxDocument::from_path("fixtures/sfc/branch-independent-alternative-generated.xgwx")
            .unwrap();
    let saved =
        XgwxDocument::from_path("fixtures/sfc/branch-independent-alternative-roundtrip.xgwx")
            .unwrap();
    let a = &source.sfc_programs()[0];
    let b = &saved.sfc_programs()[0];
    assert!(a.blocks[0].editable_rows.is_some());
    assert_eq!(a.blocks[0].editable_rows, b.blocks[0].editable_rows);
    assert_eq!(a.variables, b.variables);
}

#[cfg(feature = "write")]
#[test]
fn native_ordinary_step_limit_is_rejected_atomically() {
    let mut doc = XgwxDocument::from_path("fixtures/sfc/native-loop.xgwx").unwrap();
    let before = doc.to_bytes().unwrap();
    let step = doc.sfc_programs()[0].blocks[0]
        .editable_rows
        .clone()
        .unwrap()
        .into_iter()
        .find(|r| r.kind == "step")
        .unwrap();
    let rows = (0..513)
        .map(|i| {
            let mut r = step.clone();
            r.title = format!("LimitStep{i}");
            r.initial = i == 0;
            r.action = None;
            r.action_code = None;
            r.position = None;
            r
        })
        .collect();
    assert!(
        apply(&mut doc, rows)
            .unwrap_err()
            .to_string()
            .contains("512 ordinary steps")
    );
    assert_eq!(doc.to_bytes().unwrap(), before);
}
