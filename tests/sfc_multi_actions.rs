use xgwx::XgwxDocument;
#[test]
fn native_additional_timed_action_belongs_to_the_preceding_step() {
    let doc = XgwxDocument::from_path("fixtures/sfc/multi-action-two-native.xgwx").unwrap();
    let rows = doc.sfc_programs()[0].blocks[0]
        .editable_rows
        .clone()
        .unwrap();
    assert_eq!(rows[1].kind, "step");
    assert!(rows[1].action_code.is_some());
    assert_eq!(rows[2].kind, "continuation");
    assert_eq!(rows[2].action.as_deref(), Some("%MX12"));
    assert_eq!(rows[2].action_qualifier.as_deref(), Some("L"));
    assert_eq!(rows[2].action_time.as_deref(), Some("T#2s"));
    assert_eq!(rows[3].title, "Ready");
    assert!(rows[3].transition_code.is_some());
}
#[test]
fn native_three_actions_and_branch_continuations_preserve_independent_sources() {
    for file in ["multi-action-three-native", "multi-action-branch-native"] {
        let doc = XgwxDocument::from_path(format!("fixtures/sfc/{file}.xgwx")).unwrap();
        let program = &doc.sfc_programs()[0];
        let rows = program.blocks[0].editable_rows.as_ref().unwrap();
        assert!(rows
            .iter()
            .any(|r| r.kind == "continuation" && r.action.is_some()));
        assert!(rows.iter().any(|r| r.action_code.is_some()));
        assert_eq!(program.variables_error, None);
        if file.contains("three") {
            assert_eq!(rows[3].action_qualifier.as_deref(), Some("P"));
            assert_eq!(rows[3].action_code, rows[1].action_code);
        } else {
            assert_eq!(rows.iter().filter(|r| r.kind == "continuation").count(), 2);
        }
    }
}

#[cfg(feature = "write")]
#[test]
fn additional_actions_cannot_cross_a_transition_or_become_initial_steps() {
    let source = XgwxDocument::from_path("fixtures/sfc/multi-action-two-native.xgwx").unwrap();
    let block = source.sfc_programs()[0].blocks[0].clone();
    for invalid in 0..3 {
        let mut doc = source.clone();
        let mut rows = block.editable_rows.clone().unwrap();
        match invalid {
            0 => rows[2].initial = true,
            1 => rows.swap(2, 3),
            _ => rows[2].action_time = Some("T#bad".into()),
        }
        assert!(doc
            .replace_sfc_sequence(&xgwx::SfcSequencePatch {
                program_index: 0,
                block_index: 0,
                expected_entities: block.entities.clone(),
                expected_rows: block.editable_rows.clone(),
                rows
            })
            .is_err());
        assert_eq!(doc.xml, source.xml);
    }
}

#[test]
fn native_save_as_retains_the_generated_action_stack_and_declarations() {
    for kind in ["linear", "branch"] {
        let generated =
            XgwxDocument::from_path(format!("fixtures/sfc/multi-action-{kind}-generated.xgwx"))
                .unwrap();
        let saved =
            XgwxDocument::from_path(format!("fixtures/sfc/multi-action-{kind}-roundtrip.xgwx"))
                .unwrap();
        let before = &generated.sfc_programs()[0];
        let after = &saved.sfc_programs()[0];
        assert_eq!(
            after.blocks[0].editable_rows,
            before.blocks[0].editable_rows
        );
        assert!(after.blocks[0].editable_rows.is_some());
        assert_eq!(after.variables, before.variables);
    }
}
