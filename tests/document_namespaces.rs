#[cfg(feature = "write")]
#[path = "support/mutation_support.rs"]
mod mutation_support;
use xgwx::XgwxDocument;

#[test]
fn independent_read_views_expose_native_programs_and_symbols_without_mutation() {
    let doc = XgwxDocument::from_path("fixtures/text-programs/native-blank.xgwx").unwrap();
    let original = doc.clone();
    let programs = doc.program();
    let variables = doc.variable();
    assert_eq!(programs.list().len(), 4);
    assert_eq!(
        programs
            .text_programs()
            .iter()
            .map(|p| (p.program_index, p.language.as_str()))
            .collect::<Vec<_>>(),
        [(2, "ST"), (3, "IL")]
    );
    assert!(programs.languages().contains(&"ST"));
    assert_eq!(variables.locals().len(), 4);
    assert!(variables.globals().is_ok());
    assert!(variables.sfc_variables(2).is_ok());
    assert_eq!(doc, original);
}

#[cfg(feature = "write")]
#[test]
fn program_and_variable_editors_preserve_identity_and_reject_stale_snapshots() {
    use xgwx::{SfcVariablePatch, TextProgramPatch, create_project};
    let mut doc = create_project("XGI-CPUE", "ST").unwrap();
    let program = doc.program().text_programs().remove(0);
    let declarations = doc.variable().sfc_variables(program.program_index).unwrap();
    let variable_patch = SfcVariablePatch {
        program_index: program.program_index,
        expected_variables: declarations,
        name: "Enabled".into(),
        data_type: "BOOL".into(),
        description: "Example flag".into(),
        remove: false,
        update: false,
        declaration: None,
    };
    doc.program_mut()
        .get_if((&variable_patch).program_index, &program.object_id)
        .and_then(|selected_program| {
            mutation_support::edit_text_variable(selected_program, &variable_patch)
        })
        .unwrap();
    let source_patch = TextProgramPatch {
        program_index: program.program_index,
        expected_object_id: program.object_id.clone(),
        expected_language: program.language,
        expected_source: program.source.unwrap(),
        source: "Enabled := TRUE;\r\n".into(),
    };
    {
        let mut selected = doc.program_mut().get(source_patch.program_index).unwrap();
        let mut editor = selected.text().unwrap();
        let patch = xgwx::TextSourcePatch {
            expected_source: source_patch.expected_source.clone(),
            source: source_patch.source.clone(),
        };
        editor.update(&patch).unwrap();
        assert_eq!(
            editor.source().unwrap().as_deref(),
            Some("Enabled := TRUE;\r\n")
        );
        assert!(editor.update(&patch).is_err());
    }
    let valid = doc.to_verified_bytes().unwrap();
    assert!(
        doc.program_mut()
            .get_if((&variable_patch).program_index, &program.object_id)
            .and_then(|selected_program| mutation_support::edit_text_variable(
                selected_program,
                &variable_patch
            ))
            .is_err()
    );
    assert_eq!(doc.to_verified_bytes().unwrap(), valid);
    let reparsed = XgwxDocument::parse(&valid).unwrap();
    assert!(
        reparsed
            .variable()
            .sfc_variables(program.program_index)
            .unwrap()
            .iter()
            .any(|v| v.name == "Enabled" && v.data_type == "BOOL")
    );
    assert_eq!(
        reparsed.program().text_programs()[0].source,
        Some(source_patch.source)
    );
}
