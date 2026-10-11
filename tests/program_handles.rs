use xgwx::XgwxDocument;

#[test]
fn shared_selection_reads_the_chosen_program_and_rejects_missing_indices() {
    let doc = XgwxDocument::from_path("fixtures/text-programs/native-blank.xgwx").unwrap();
    let program = doc.program().get(2).unwrap();
    assert_eq!(program.index(), 2);
    assert_eq!(program.summary(), doc.program().list()[2]);
    assert_eq!(program.text().unwrap().language, "ST");
    assert!(program.sfc().is_err());
    assert!(program.variables().is_ok());
    assert!(program.declarations().is_ok());
    assert!(matches!(
        doc.program().get(usize::MAX),
        Err(xgwx::XgwxError::ProgramNotFound { .. })
    ));
}

#[cfg(feature = "write")]
#[test]
fn handle_follows_program_through_metadata_updates_move_and_consuming_delete() {
    use xgwx::{NewProgram, ProgramPatch, create_project};
    let mut doc = create_project("XGI-CPUE", "ST").unwrap();
    let first_id = doc.program().list()[0].object_id.clone();
    doc.program_mut()
        .create(&NewProgram {
            name: "Second".into(),
            language: "ST".into(),
            object_id: "aaaaaaaa-1111-2222-3333-444444444444".into(),
            symbol_id: "bbbbbbbb-1111-2222-3333-444444444444".into(),
        })
        .unwrap();
    {
        let mut program = doc.program_mut().get(0).unwrap();
        program.rename("Selected").unwrap();
        program
            .update(&ProgramPatch {
                comment: Some("Scoped metadata".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(program.summary().object_id, first_id);
        let snapshot = program.summary();
        assert!(program.move_to(100).is_err());
        assert_eq!(program.summary(), snapshot);
        program.move_to(1).unwrap();
        assert_eq!(program.index(), 1);
        assert_eq!(program.summary().name.as_deref(), Some("Selected"));
        program.delete().unwrap();
    }
    let reloaded = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
    assert_eq!(reloaded.program().list().len(), 1);
    assert_eq!(reloaded.program().list()[0].name.as_deref(), Some("Second"));
}

#[cfg(feature = "write")]
#[test]
fn source_and_local_declarations_are_bound_to_only_the_selected_program() {
    use xgwx::{ProgramVariablePatch, TextSourcePatch};
    let mut doc = XgwxDocument::from_path("fixtures/text-programs/native-blank.xgwx").unwrap();
    let other = doc.program().get(3).unwrap().text().unwrap();
    let source_patch;
    {
        let mut program = doc.program_mut().get(2).unwrap();
        let declarations = program.view().declarations().unwrap();
        let patch = ProgramVariablePatch {
            expected_variables: declarations,
            name: "ScopedFlag".into(),
            data_type: "BOOL".into(),
            description: "Local to selected ST program".into(),
            remove: false,
            declaration: None,
            update: false,
        };
        let mut variables = program.variable_mut();
        variables.edit_declaration(&patch).unwrap();
        assert!(variables.edit_declaration(&patch).is_err());
        source_patch = TextSourcePatch {
            expected_source: program.view().text().unwrap().source.unwrap(),
            source: "ScopedFlag := TRUE;\r\n".into(),
        };
        let mut text = program.text().unwrap();
        text.update(&source_patch).unwrap();
        assert!(text.update(&source_patch).is_err());
        assert_eq!(
            text.source().unwrap().as_deref(),
            Some(source_patch.source.as_str())
        );
    }
    assert_eq!(doc.program().get(3).unwrap().text().unwrap(), other);
    let reloaded = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
    assert_eq!(
        reloaded.program().get(2).unwrap().text().unwrap().source,
        Some(source_patch.source)
    );
    assert!(
        reloaded
            .program()
            .get(2)
            .unwrap()
            .declarations()
            .unwrap()
            .iter()
            .any(|v| v.name == "ScopedFlag")
    );
    assert!(
        !reloaded
            .program()
            .get(3)
            .unwrap()
            .declarations()
            .unwrap()
            .iter()
            .any(|v| v.name == "ScopedFlag")
    );
}

#[cfg(feature = "write")]
#[test]
fn wrong_language_editors_reject_without_modifying_the_document() {
    use xgwx::create_project;
    for (cpu, language) in [("XGK-CPUSN", "LD"), ("XGI-CPUE", "ST"), ("XGI-CPUE", "SFC")] {
        let mut doc = create_project(cpu, language).unwrap();
        let before = doc.to_verified_bytes().unwrap();
        {
            let mut program = doc.program_mut().get(0).unwrap();
            assert!(program.iec_ld().is_err());
            if language != "ST" {
                assert!(program.text().is_err());
            }
            if language != "SFC" {
                assert!(program.sfc().is_err());
            }
            if language != "LD" {
                assert!(program.ladder().is_err());
            }
        }
        assert_eq!(doc.to_verified_bytes().unwrap(), before);
    }
}

#[cfg(feature = "write")]
#[test]
fn selected_sfc_chart_retains_identity_and_stale_chart_guards() {
    use xgwx::SfcSequenceEdit;
    let mut doc = XgwxDocument::from_path("fixtures/sfc/native-loop.xgwx").unwrap();
    let id = doc.program().list()[0].object_id.clone();
    {
        let mut program = doc.program_mut().get(0).unwrap();
        let mut sfc = program.sfc().unwrap();
        let block = sfc.data().unwrap().blocks.remove(0);
        let mut rows = block.editable_rows.clone().unwrap();
        rows[1].comment = "Scoped step".into();
        let patch = SfcSequenceEdit {
            block_index: 0,
            expected_entities: block.entities,
            expected_rows: block.editable_rows,
            rows,
        };
        sfc.replace_sequence(&patch).unwrap();
        assert!(sfc.replace_sequence(&patch).is_err());
        assert_eq!(
            sfc.data().unwrap().blocks[0]
                .editable_rows
                .as_ref()
                .unwrap()[1]
                .comment,
            "Scoped step"
        );
    }
    let reloaded = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
    assert_eq!(reloaded.program().list()[0].object_id, id);
}

#[cfg(all(feature = "write", feature = "il"))]
#[test]
fn selected_vendor_il_is_distinct_from_standalone_iec_il() {
    use xgwx::{VendorIlSourcePatch, create_project};
    let mut doc = create_project("XGK-CPUSN", "IL").unwrap();
    {
        let mut program = doc.program_mut().get(0).unwrap();
        assert!(program.text().is_err());
        let mut il = program.vendor_il().unwrap();
        let patch = VendorIlSourcePatch {
            expected_source: il.data().unwrap().source.unwrap(),
            source: "LOAD M00000\r\nOUT M00001\r\n".into(),
        };
        il.update(&patch).unwrap();
        assert!(il.update(&patch).is_err());
    }
    assert!(
        XgwxDocument::parse(&doc.to_verified_bytes().unwrap())
            .unwrap()
            .program()
            .vendor_il_programs()[0]
            .source
            .as_ref()
            .unwrap()
            .contains("M00000")
    );
}

#[cfg(feature = "write")]
#[test]
fn cross_program_replacement_binds_the_destination_and_preserves_the_source() {
    use xgwx::{NewProgram, create_project};
    let mut doc = create_project("XGI-CPUE", "LD").unwrap();
    doc.program_mut()
        .create(&NewProgram {
            name: "Destination".into(),
            language: "LD".into(),
            object_id: "aaaaaaaa-1111-2222-3333-444444444444".into(),
            symbol_id: "bbbbbbbb-1111-2222-3333-444444444444".into(),
        })
        .unwrap();
    for (index, contact, coil) in [(0, "%MX0", "%MX1"), (1, "%MX2", "%MX3")] {
        let mut program = doc.program_mut().get(index).unwrap();
        let mut ladder = program.iec_ld().unwrap();
        ladder
            .insert_single_element(0, 1, "contact", "NO", contact)
            .unwrap();
        ladder
            .insert_single_element(0, 94, "coil", "OUTPUT", coil)
            .unwrap();
        if index == 1 {
            // The existing writer guards reject deleting the only network.
            // Keep a second network while replacing the selected destination.
            ladder.insert_blank_row(0).unwrap();
            ladder.insert_linear_rung(1, "%MX10", "%MX11").unwrap();
        }
    }
    let source = doc.program().get(0).unwrap().ladder().unwrap();
    let target = doc.program().get(1).unwrap().ladder().unwrap();
    let source_row = &source.iec_row_frames().unwrap()[0];
    let target_row = &target.iec_row_frames().unwrap()[0];
    doc.program_mut()
        .get(1)
        .unwrap()
        .iec_ld()
        .unwrap()
        .replace_group_from_program(
            0,
            source_row.group_index,
            source_row.row_index,
            target_row.group_index,
            target_row.row_index,
            false,
        )
        .unwrap();
    assert_eq!(
        doc.program().get(0).unwrap().ladder().unwrap().data,
        source.data
    );
    let destination = doc.program().get(1).unwrap().ladder().unwrap();
    assert!(destination.strings.iter().any(|s| s.value == "%MX0"));
    assert!(!destination.strings.iter().any(|s| s.value == "%MX2"));
    doc.to_verified_bytes().unwrap();
}
