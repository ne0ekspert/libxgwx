use xgwx::XgwxDocument;
const NATIVE: &[u8] = include_bytes!("../fixtures/sfc/native-loop.xgwx");

#[test]
fn native_blank_sfc_has_one_empty_main_block() {
    let doc = XgwxDocument::parse(include_bytes!("../fixtures/sfc/new-xgi-sfc.xgwx")).unwrap();
    let programs = doc.sfc_programs();
    assert_eq!(programs.len(), 1);
    assert_eq!(programs[0].program_index, 0);
    assert_eq!(programs[0].blocks.len(), 1);
    let block = &programs[0].blocks[0];
    assert_eq!(block.name, "NewProgram");
    assert!(block.main);
    assert_eq!((block.language_type, block.language), (Some(3), Some(2)));
    assert_eq!((block.rows, block.columns), (0, 0));
    assert!(block.entities.is_empty());
    let roundtrip =
        XgwxDocument::parse(include_bytes!("../fixtures/sfc/new-xgi-sfc-roundtrip.xgwx")).unwrap();
    assert_eq!(roundtrip.sfc_programs(), programs);
    let original_locals = doc.root.descendants_named("LocalVar").next().unwrap();
    let saved_locals = roundtrip.root.descendants_named("LocalVar").next().unwrap();
    // XG5000 reindents the parent, while preserving the symbol payloads verbatim.
    assert_eq!(saved_locals.attributes, original_locals.attributes);
    assert_eq!(saved_locals.children, original_locals.children);
}

#[test]
fn native_sfc_has_typed_steps_transitions_and_return_loop() {
    let doc = XgwxDocument::parse(NATIVE).unwrap();
    let programs = doc.sfc_programs();
    assert_eq!(programs.len(), 1);
    let block = &programs[0].blocks[0];
    assert!(block.main);
    assert_eq!(block.name, "NewProgram");
    let kinds: Vec<_> = block
        .entities
        .iter()
        .take(6)
        .map(|e| e.type_code.unwrap())
        .collect();
    assert_eq!(kinds, [6, 0, 1, 0, 1, 5]);
    assert_eq!(
        block.entities[1].properties["EntityStep"]["InitialStep"],
        "1"
    );
    assert_eq!(
        block.entities[3].properties["EntityStep"]["Comment"],
        "Ready for cycle"
    );
}

#[cfg(feature = "write")]
fn condition_patch() -> xgwx::SfcEntityPatch {
    xgwx::SfcEntityPatch {
        program_index: 0,
        block_index: 0,
        entity_index: 2,
        expected_type: 1,
        expected_row: 2,
        expected_column: 0,
        field: "condition".into(),
        expected_value: "%MX0".into(),
        replacement: "%MX2".into(),
    }
}

#[cfg(feature = "write")]
#[test]
fn transition_updates_native_annotation_and_preserves_unrelated_xml() {
    let mut doc = XgwxDocument::parse(NATIVE).unwrap();
    let old = doc.xml.clone();
    doc.edit_sfc_entity(&condition_patch()).unwrap();
    assert_eq!(doc.xml, old.replace("Title=\"%MX0\"", "Title=\"%MX2\""));
    let bytes = doc.to_verified_bytes().unwrap();
    let reparsed = XgwxDocument::parse(&bytes).unwrap();
    assert_eq!(
        reparsed.sfc_programs()[0].blocks[0].entities[2].properties["EntityStep"]["Title"],
        "%MX2"
    );
    assert_eq!(
        reparsed.sfc_programs()[0].blocks[0].entities[8].properties["EntityStep"]["Title"],
        "%MX2"
    );
    let after = doc.xml.clone();
    assert!(doc.edit_sfc_entity(&condition_patch()).is_err());
    assert_eq!(doc.xml, after);
}

#[cfg(feature = "write")]
#[test]
fn stale_position_inconsistent_annotation_and_unknown_fields_are_atomic() {
    for change in 0..6 {
        let mut doc = XgwxDocument::parse(NATIVE).unwrap();
        let mut p = condition_patch();
        match change {
            0 => p.expected_row = 3,
            1 => p.field = "initialStep".into(),
            2 => p.replacement = "%MW2".into(),
            3 => p.entity_index = 0,
            4 => p.field = "comment".into(),
            _ => {
                doc.xml = doc.xml.replacen("Title=\"%MX0\"", "Title=\"%MX3\"", 1);
            }
        }
        let before = doc.xml.clone();
        assert!(doc.edit_sfc_entity(&p).is_err());
        assert_eq!(doc.xml, before);
    }
}

#[cfg(feature = "write")]
#[test]
fn step_comment_roundtrips_xml_metacharacters_and_newlines() {
    let mut doc = XgwxDocument::parse(NATIVE).unwrap();
    let mut p = condition_patch();
    p.entity_index = 3;
    p.expected_type = 0;
    p.expected_row = 3;
    p.field = "comment".into();
    p.expected_value = "Ready for cycle".into();
    p.replacement = "Cycle & <ready> \"OK\"\n다음 단계".into();
    doc.edit_sfc_entity(&p).unwrap();
    let reparsed = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
    assert_eq!(
        reparsed.sfc_programs()[0].blocks[0].entities[3].properties["EntityStep"]["Comment"],
        p.replacement
    );
}

#[test]
fn native_save_as_retains_generated_chart_and_symbol_tables() {
    let roundtrip = XgwxDocument::parse(include_bytes!(
        "../fixtures/sfc/edited-native-roundtrip.xgwx"
    ))
    .unwrap();
    let doc = XgwxDocument::parse(NATIVE).unwrap();
    let entities = &roundtrip.sfc_programs()[0].blocks[0].entities;
    assert_eq!(entities[2].properties["EntityStep"]["Title"], "%MX2");
    assert_eq!(
        entities[3].properties["EntityStep"]["Comment"],
        "Cycle active & ready"
    );
    // SFC locals use a symbol format that the IEC ladder decoder does not support.
    // Compare the native XML tables directly rather than decoding them as LD locals.
    let mut expected = doc.sfc_programs();
    expected[0].blocks[0].entities[2]
        .properties
        .get_mut("EntityStep")
        .unwrap()
        .insert("Title".into(), "%MX2".into());
    expected[0].blocks[0].entities[8]
        .properties
        .get_mut("EntityStep")
        .unwrap()
        .insert("Title".into(), "%MX2".into());
    expected[0].blocks[0].entities[3]
        .properties
        .get_mut("EntityStep")
        .unwrap()
        .insert("Comment".into(), "Cycle active & ready".into());
    let rows = expected[0].blocks[0].editable_rows.as_mut().unwrap();
    rows[2].title = "%MX2".into();
    rows[3].comment = "Cycle active & ready".into();
    assert_eq!(roundtrip.sfc_programs(), expected);
    assert_eq!(
        roundtrip
            .root
            .descendants_named("LocalVar")
            .collect::<Vec<_>>(),
        doc.root.descendants_named("LocalVar").collect::<Vec<_>>()
    );
}

#[cfg(feature = "write")]
#[test]
fn linear_chart_creation_reordering_and_deletion_preserve_symbols() {
    use xgwx::{SfcRow, SfcSequencePatch};
    let mut doc = XgwxDocument::parse(include_bytes!("../fixtures/sfc/new-xgi-sfc.xgwx")).unwrap();
    let original_locals = doc
        .root
        .descendants_named("LocalVar")
        .next()
        .unwrap()
        .clone();
    let rows = vec![
        SfcRow {
            kind: "label".into(),
            title: "Cycle".into(),
            comment: "".into(),
            initial: false,
            action: None,
            action_qualifier: None,
            action_code: None,
            transition_code: None,
            action_time: None,
        },
        SfcRow {
            kind: "step".into(),
            title: "Idle".into(),
            comment: "Ready & <go>\n다음".into(),
            initial: true,
            action: Some("%MX10".into()),
            action_qualifier: None,
            action_code: None,
            transition_code: None,
            action_time: None,
        },
        SfcRow {
            kind: "transition".into(),
            title: "%MX2".into(),
            comment: "".into(),
            initial: false,
            action: None,
            action_qualifier: None,
            action_code: None,
            transition_code: None,
            action_time: None,
        },
        SfcRow {
            kind: "jump".into(),
            title: "Cycle".into(),
            comment: "".into(),
            initial: false,
            action: None,
            action_qualifier: None,
            action_code: None,
            transition_code: None,
            action_time: None,
        },
    ];
    let patch = SfcSequencePatch {
        expected_rows: None,
        program_index: 0,
        block_index: 0,
        expected_entities: vec![],
        rows: rows.clone(),
    };
    doc.replace_sfc_sequence(&patch).unwrap();
    let parsed = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
    let block = &parsed.sfc_programs()[0].blocks[0];
    assert_eq!(block.editable_rows.as_ref().unwrap(), &rows);
    assert_eq!(block.entities.len(), 8);
    assert_eq!(block.entities[5].type_code, Some(2));
    assert_eq!(block.entities[6].properties["EntityStep"]["Title"], "%MX2");
    assert_eq!(
        parsed.root.descendants_named("LocalVar").next().unwrap(),
        &original_locals
    );
    let before = doc.xml.clone();
    assert!(doc.replace_sfc_sequence(&patch).is_err());
    assert_eq!(doc.xml, before);
    let mut rows = rows;
    rows.swap(1, 2);
    doc.replace_sfc_sequence(&SfcSequencePatch {
        expected_rows: None,
        expected_entities: block.entities.clone(),
        rows: rows.clone(),
        ..patch.clone()
    })
    .unwrap();
    assert_eq!(
        doc.sfc_programs()[0].blocks[0]
            .editable_rows
            .as_ref()
            .unwrap(),
        &rows
    );
    let expected = doc.sfc_programs()[0].blocks[0].entities.clone();
    doc.replace_sfc_sequence(&SfcSequencePatch {
        expected_rows: None,
        expected_entities: expected,
        rows: vec![],
        ..patch
    })
    .unwrap();
    assert!(doc.sfc_programs()[0].blocks[0].entities.is_empty());
    assert_eq!(
        doc.root.descendants_named("LocalVar").next().unwrap(),
        &original_locals
    );
}

#[cfg(feature = "write")]
#[test]
fn linear_chart_invalid_names_operands_targets_and_initial_steps_are_atomic() {
    use xgwx::SfcSequencePatch;
    let source = XgwxDocument::parse(NATIVE).unwrap();
    let block = &source.sfc_programs()[0].blocks[0];
    for case in 0..8 {
        let mut doc = source.clone();
        let mut rows = block.editable_rows.clone().unwrap();
        match case {
            0 => rows[3].title = rows[1].title.clone(),
            1 => rows[2].title = "%MW0".into(),
            2 => rows[1].action = Some("symbol".into()),
            3 => rows[3].initial = true,
            4 => rows[5].title = "Missing".into(),
            5 => rows[1].title = "A & B".into(),
            6 => rows[2].comment = "not captured".into(),
            _ => rows[0].kind = "branch".into(),
        }
        assert!(
            doc.replace_sfc_sequence(&SfcSequencePatch {
                expected_rows: None,
                program_index: 0,
                block_index: 0,
                expected_entities: block.entities.clone(),
                rows
            })
            .is_err()
        );
        assert_eq!(doc.xml, source.xml);
    }
}

#[cfg(feature = "write")]
#[test]
fn branches_program_references_and_unknown_native_data_fail_closed() {
    use xgwx::SfcSequencePatch;
    for (before, after) in [
        (
            "Type=\"0\" Col=\"0\" Row=\"1\"",
            "Type=\"3\" Col=\"0\" Row=\"1\"",
        ),
        ("Bookmark=\"0\"", "Bookmark=\"1\""),
        ("PropertyProgram=\"0\"", "PropertyProgram=\"1\""),
        (
            "Type=\"6\" Col=\"0\" Row=\"0\"",
            "Type=\"6\" Col=\"0\" Row=\"0\" Extra=\"keep\"",
        ),
        ("RowSize=\"6\" ColSize=\"2\"", "RowSize=\"6\" ColSize=\"3\""),
        (
            "UploadProgramSize=\"0\"",
            "UploadProgramSize=\"0\" Unknown=\"keep\"",
        ),
    ] {
        let mut source = XgwxDocument::parse(NATIVE).unwrap();
        assert!(source.xml.contains(before));
        source.xml = source.xml.replacen(before, after, 1);
        let mut doc = XgwxDocument::parse(&source.to_bytes().unwrap()).unwrap();
        let block = &doc.sfc_programs()[0].blocks[0];
        let original = doc.xml.clone();
        assert!(
            doc.replace_sfc_sequence(&SfcSequencePatch {
                expected_rows: None,
                program_index: 0,
                block_index: 0,
                expected_entities: block.entities.clone(),
                rows: vec![]
            })
            .is_err()
        );
        assert_eq!(doc.xml, original);
    }
}

#[test]
fn native_structural_save_as_retains_rows_actions_initial_steps_and_symbols() {
    for (source, saved) in [
        (
            include_bytes!("../fixtures/sfc/structural-build.xgwx").as_slice(),
            include_bytes!("../fixtures/sfc/structural-build-native-roundtrip.xgwx").as_slice(),
        ),
        (
            include_bytes!("../fixtures/sfc/structural-delete.xgwx").as_slice(),
            include_bytes!("../fixtures/sfc/structural-delete-native-roundtrip.xgwx").as_slice(),
        ),
        (
            include_bytes!("../fixtures/sfc/structural-rename.xgwx").as_slice(),
            include_bytes!("../fixtures/sfc/structural-rename-native-roundtrip.xgwx").as_slice(),
        ),
    ] {
        let original = XgwxDocument::parse(source).unwrap();
        let roundtrip = XgwxDocument::parse(saved).unwrap();
        let before = original.sfc_programs();
        let after = roundtrip.sfc_programs();
        assert!(after[0].blocks[0].editable_rows.is_some());
        assert_eq!(
            before[0].blocks[0].editable_rows,
            after[0].blocks[0].editable_rows
        );
        let local = |doc: &XgwxDocument| {
            doc.root
                .descendants_named("LocalVar")
                .next()
                .unwrap()
                .children
                .clone()
        };
        assert_eq!(local(&original), local(&roundtrip));
    }
}

#[cfg(feature = "write")]
#[test]
fn action_qualifiers_times_and_invalid_candidates_are_atomic() {
    use xgwx::{SfcRow, SfcSequencePatch};
    let mut doc = XgwxDocument::parse(include_bytes!("../fixtures/sfc/new-xgi-sfc.xgwx")).unwrap();
    for (qualifier, code) in [
        ("N", "1"),
        ("R", "2"),
        ("S", "4"),
        ("L", "8"),
        ("D", "16"),
        ("P", "32"),
        ("SD", "64"),
        ("DS", "128"),
        ("SL", "256"),
    ] {
        let timed = ["L", "D", "SD", "DS", "SL"].contains(&qualifier);
        let rows = vec![SfcRow {
            kind: "step".into(),
            title: "S0".into(),
            comment: "".into(),
            initial: true,
            action: Some("%MX10".into()),
            action_qualifier: (qualifier != "N").then(|| qualifier.into()),
            action_code: None,
            transition_code: None,
            action_time: timed.then(|| "T#1m2s500ms".into()),
        }];
        let patch = SfcSequencePatch {
            expected_rows: None,
            program_index: 0,
            block_index: 0,
            expected_entities: doc.sfc_programs()[0].blocks[0].entities.clone(),
            rows: rows.clone(),
        };
        doc.replace_sfc_sequence(&patch).unwrap();
        let parsed = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
        let programs = parsed.sfc_programs();
        let block = &programs[0].blocks[0];
        assert_eq!(block.editable_rows.as_ref().unwrap(), &rows);
        assert_eq!(
            block.entities[1].properties["EntityAction"]["Qualifier"],
            code
        );
        for (q, t, operand) in [
            ("BAD", "", Some("%MX10")),
            ("L", "", Some("%MX10")),
            ("L", "T#2s1m", Some("%MX10")),
            ("D", "T#999999999d", Some("%MX10")),
            ("N", "T#2s", Some("%MX10")),
            ("S", "", Some("%MW10")),
            ("S", "", None),
        ] {
            let before = doc.clone();
            let mut invalid = rows.clone();
            invalid[0].action = operand.map(String::from);
            invalid[0].action_qualifier = Some(q.into());
            invalid[0].action_time = Some(t.into());
            let patch = SfcSequencePatch {
                expected_rows: None,
                program_index: 0,
                block_index: 0,
                expected_entities: doc.sfc_programs()[0].blocks[0].entities.clone(),
                rows: invalid,
            };
            assert!(doc.replace_sfc_sequence(&patch).is_err());
            assert_eq!(doc, before);
        }
    }
}

#[test]
fn native_action_qualifier_save_as_retains_all_nine_types_times_and_symbols() {
    let original =
        XgwxDocument::parse(include_bytes!("../fixtures/sfc/action-qualifiers.xgwx")).unwrap();
    let saved = XgwxDocument::parse(include_bytes!(
        "../fixtures/sfc/action-qualifiers-native-roundtrip.xgwx"
    ))
    .unwrap();
    let before = original.sfc_programs();
    let after = saved.sfc_programs();
    assert_eq!(
        before[0].blocks[0].editable_rows,
        after[0].blocks[0].editable_rows
    );
    assert!(after[0].blocks[0].editable_rows.is_some());
    let a = original.root.descendants_named("LocalVar").next().unwrap();
    let b = saved.root.descendants_named("LocalVar").next().unwrap();
    assert_eq!(a.attributes, b.attributes);
    assert_eq!(a.children, b.children);
    let actions: Vec<_> = after[0].blocks[0]
        .entities
        .iter()
        .filter(|e| e.type_code == Some(2))
        .collect();
    assert_eq!(actions.len(), 9);
    for (entity, q) in actions
        .iter()
        .zip(["1", "2", "4", "8", "16", "32", "64", "128", "256"])
    {
        assert_eq!(entity.properties["EntityAction"]["Qualifier"], q);
    }
}

#[cfg(feature = "write")]
#[test]
fn native_st_programs_decode_and_generated_sources_roundtrip() {
    use xgwx::SfcSequencePatch;
    let mut doc = XgwxDocument::from_path("fixtures/sfc/native-st-programs.xgwx").unwrap();
    let program = doc.sfc_programs().remove(0);
    assert_eq!(program.blocks.len(), 3);
    assert!(
        program
            .variables
            .iter()
            .any(|v| v.name == "Count" && v.data_type == "DINT")
    );
    assert!(
        program
            .variables
            .iter()
            .any(|v| v.name == "Delay" && v.data_type == "TON")
    );
    assert_eq!(program.variables.iter().filter(|v| v.system).count(), 2);
    let block = &program.blocks[0];
    let mut rows = block
        .editable_rows
        .clone()
        .expect("captured ST chart is editable");
    assert!(
        rows[1]
            .action_code
            .as_ref()
            .unwrap()
            .contains("Delay(IN := TRUE")
    );
    assert!(
        rows[2]
            .transition_code
            .as_ref()
            .unwrap()
            .contains("TRANS := Count")
    );
    rows[1].action_code = Some("Count := ADD(Count, 2);\r\nDelay(IN := TRUE, PT := T#1s);".into());
    doc.replace_sfc_sequence(&SfcSequencePatch {
        expected_rows: block.editable_rows.clone(),
        program_index: 0,
        block_index: 0,
        expected_entities: block.entities.clone(),
        rows: rows.clone(),
    })
    .unwrap();
    let result = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
    assert_eq!(result.sfc_programs()[0].blocks[0].editable_rows, Some(rows));
    assert_eq!(result.sfc_variables(0).unwrap(), program.variables);
}

#[cfg(feature = "write")]
#[test]
fn sfc_typed_declarations_preserve_system_records_and_reject_referenced_deletion() {
    use xgwx::SfcVariablePatch;
    let mut doc = XgwxDocument::from_path("fixtures/sfc/native-st-programs.xgwx").unwrap();
    for ty in [
        "WORD",
        "DWORD",
        "LWORD",
        "INT",
        "DINT",
        "UINT",
        "UDINT",
        "LINT",
        "ULINT",
        "REAL",
        "LREAL",
        "TIME",
        "TOF",
        "TP",
        "CTU_DINT",
        "CTD_DINT",
        "CTUD_DINT",
        "R_TRIG",
        "F_TRIG",
        "RS",
        "SR",
    ] {
        let name = format!("Var_{ty}");
        let expected = doc.sfc_variables(0).unwrap();
        doc.edit_sfc_variable(&SfcVariablePatch {
            program_index: 0,
            expected_variables: expected,
            name: name.clone(),
            data_type: ty.into(),
            description: "typed declaration".into(),
            remove: false,
        })
        .unwrap();
        assert!(
            doc.sfc_variables(0)
                .unwrap()
                .iter()
                .any(|v| v.name == name && v.data_type == ty)
        );
    }
    let expected = doc.sfc_variables(0).unwrap();
    let bytes = doc.to_verified_bytes().unwrap();
    for name in ["Count", "Delay", "TRANS", "GOTO_INIT"] {
        assert!(
            doc.edit_sfc_variable(&SfcVariablePatch {
                program_index: 0,
                expected_variables: expected.clone(),
                name: name.into(),
                data_type: "DINT".into(),
                description: "".into(),
                remove: true
            })
            .is_err()
        );
        assert_eq!(doc.to_verified_bytes().unwrap(), bytes);
    }
    doc.edit_sfc_variable(&SfcVariablePatch {
        program_index: 0,
        expected_variables: expected,
        name: "Var_WORD".into(),
        data_type: "WORD".into(),
        description: "".into(),
        remove: true,
    })
    .unwrap();
    assert!(
        !doc.sfc_variables(0)
            .unwrap()
            .iter()
            .any(|v| v.name == "Var_WORD")
    );
}

#[cfg(feature = "write")]
#[test]
fn native_st_save_as_retains_typed_variables_blocks_and_bool_references() {
    let source = XgwxDocument::from_path("fixtures/sfc/st-programs-generated.xgwx").unwrap();
    let native = XgwxDocument::from_path("fixtures/sfc/st-programs-native-roundtrip.xgwx").unwrap();
    let a = source.sfc_programs().remove(0);
    let b = native.sfc_programs().remove(0);
    assert_eq!(a.variables, b.variables);
    assert_eq!(a.variables_error, None);
    assert_eq!(b.variables_error, None);
    assert_eq!(a.blocks[0].editable_rows, b.blocks[0].editable_rows);
    let rows = b.blocks[0].editable_rows.as_ref().unwrap();
    assert_eq!(rows[3].action.as_deref(), Some("Value_BOOL"));
    assert_eq!(rows[4].title, "Value_BOOL");
    let mut doc = native.clone();
    let mut edited = rows.clone();
    edited[1].action_code = Some("Count := ADD(Count, 2);".into());
    let patch = xgwx::SfcSequencePatch {
        program_index: 0,
        block_index: 0,
        expected_entities: b.blocks[0].entities.clone(),
        expected_rows: Some(rows.clone()),
        rows: edited.clone(),
    };
    doc.replace_sfc_sequence(&patch).unwrap();
    assert_eq!(doc.sfc_programs()[0].blocks[0].editable_rows, Some(edited));
    let bytes = doc.to_verified_bytes().unwrap();
    assert!(doc.replace_sfc_sequence(&patch).is_err());
    assert_eq!(doc.to_verified_bytes().unwrap(), bytes);
}

#[cfg(feature = "write")]
#[test]
fn sfc_st_unknown_metadata_and_post_scan_actions_are_guarded() {
    for (before, after) in [
        ("ActionPostScan=\"0\"", "ActionPostScan=\"1\""),
        ("CodeCount=\"50\"", "CodeCount=\"50\" Unknown=\"keep\""),
        (
            "LanguageType=\"1\" Language=\"4\"",
            "LanguageType=\"1\" Language=\"1\"",
        ),
        (
            "<Breakpoints></Breakpoints>",
            "<Breakpoints><Breakpoint Line=\"0\"/></Breakpoints>",
        ),
    ] {
        let mut source = XgwxDocument::from_path("fixtures/sfc/native-st-programs.xgwx").unwrap();
        assert!(
            source.xml.contains(before),
            "missing capture pattern {before}"
        );
        source.xml = source.xml.replacen(before, after, 1);
        let mut doc = XgwxDocument::parse(&source.to_bytes().unwrap()).unwrap();
        let program = doc.sfc_programs().remove(0);
        assert!(program.blocks[0].editable_rows.is_none());
        let bytes = doc.to_verified_bytes().unwrap();
        assert!(
            doc.replace_sfc_sequence(&xgwx::SfcSequencePatch {
                program_index: 0,
                block_index: 0,
                expected_entities: program.blocks[0].entities.clone(),
                expected_rows: None,
                rows: vec![]
            })
            .is_err()
        );
        assert_eq!(doc.to_verified_bytes().unwrap(), bytes);
    }
}
