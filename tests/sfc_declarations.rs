use xgwx::XgwxDocument;
#[test]
fn native_array_initial_value_retention_and_string_are_decoded() {
    let doc = XgwxDocument::from_path("fixtures/sfc/declarations-array-native.xgwx").unwrap();
    let vars = doc.sfc_variables(0).unwrap();
    let v = vars.iter().find(|v| v.name == "Numbers").unwrap();
    assert_eq!(v.data_type, "BOOL");
    let d = v.declaration.as_ref().unwrap();
    assert_eq!((d.dimensions[0].lower, d.dimensions[0].upper), (0, 1));
    let doc = XgwxDocument::from_path("fixtures/sfc/declarations-string-native.xgwx").unwrap();
    let vars = doc.sfc_variables(0).unwrap();
    let c = vars
        .iter()
        .find(|v| v.name == "Count")
        .unwrap()
        .declaration
        .as_ref()
        .unwrap();
    assert!(c.retain);
    assert_eq!(c.initial_value, "7");
    let v = vars.iter().find(|v| v.name == "StatusText").unwrap();
    assert_eq!(v.data_type, "STRING");
    assert_eq!(v.declaration.as_ref().unwrap().initial_value, "'Hello'");
    assert!(doc.sfc_programs()[0].blocks[0].editable_rows.is_some());
}
#[cfg(feature = "write")]
#[test]
fn advanced_declarations_update_add_delete_and_reject_invalid_or_stale_writes_atomically() {
    use xgwx::{SfcArrayBound, SfcDeclaration, SfcVariablePatch};
    let source = XgwxDocument::from_path("fixtures/sfc/declarations-string-native.xgwx").unwrap();
    let mut patch = SfcVariablePatch {
        program_index: 0,
        expected_variables: source.sfc_variables(0).unwrap(),
        name: "Samples".into(),
        data_type: "DINT".into(),
        description: "array".into(),
        remove: false,
        update: false,
        declaration: Some(SfcDeclaration {
            dimensions: vec![SfcArrayBound { lower: 0, upper: 3 }],
            initial_value: "4(2)".into(),
            retain: true,
        }),
    };
    let mut doc = source.clone();
    doc.edit_sfc_variable(&patch).unwrap();
    assert_eq!(
        doc.sfc_variables(0)
            .unwrap()
            .iter()
            .find(|v| v.name == "Samples")
            .unwrap()
            .declaration,
        patch.declaration
    );
    let saved = doc.to_verified_bytes().unwrap();
    assert!(doc.edit_sfc_variable(&patch).is_err());
    assert_eq!(doc.to_verified_bytes().unwrap(), saved);
    patch.expected_variables = doc.sfc_variables(0).unwrap();
    patch.update = true;
    patch.description = "updated".into();
    doc.edit_sfc_variable(&patch).unwrap();
    assert_eq!(
        doc.sfc_variables(0)
            .unwrap()
            .iter()
            .find(|v| v.name == "Samples")
            .unwrap()
            .description,
        "updated"
    );
    patch.expected_variables = doc.sfc_variables(0).unwrap();
    patch.update = false;
    patch.remove = true;
    doc.edit_sfc_variable(&patch).unwrap();
    assert_eq!(
        doc.sfc_variables(0).unwrap(),
        source.sfc_variables(0).unwrap()
    );
    for invalid in 0..7 {
        let mut doc = source.clone();
        let mut p = patch.clone();
        p.remove = false;
        p.expected_variables = source.sfc_variables(0).unwrap();
        match invalid {
            0 => p.declaration.as_mut().unwrap().dimensions[0].upper = -1,
            1 => p.declaration.as_mut().unwrap().initial_value = "5(2)".into(),
            2 => p.declaration.as_mut().unwrap().initial_value = "2147483648".into(),
            3 => p.data_type = "TON".into(),
            4 => {
                p.name = "Count".into();
                p.update = true;
                p.data_type = "BOOL".into();
                p.declaration = Some(SfcDeclaration::default());
            }
            5 => {
                p.data_type = "STRING".into();
                p.declaration = Some(SfcDeclaration {
                    initial_value: format!("'{}'", "x".repeat(33)),
                    ..Default::default()
                });
            }
            _ => p.name = "TRANS".into(),
        }
        assert!(doc.edit_sfc_variable(&p).is_err(), "case {invalid}");
        assert_eq!(doc.xml, source.xml);
    }
    let mut doc = source.clone();
    patch.name = "Count".into();
    patch.remove = false;
    patch.update = true;
    patch.expected_variables = doc.sfc_variables(0).unwrap();
    patch.description = "retained counter".into();
    patch.declaration = Some(SfcDeclaration {
        initial_value: "12".into(),
        retain: false,
        ..Default::default()
    });
    doc.edit_sfc_variable(&patch).unwrap();
    assert!(doc.sfc_programs()[0].blocks[0].editable_rows.is_some());
}

#[test]
fn uncaptured_member_override_maps_are_guarded() {
    let doc =
        XgwxDocument::from_path("fixtures/sfc/declarations-array-initial-native.xgwx").unwrap();
    assert!(doc.sfc_variables(0).is_err());
    assert!(doc.sfc_programs()[0].variables_error.is_some());
}
#[cfg(feature = "write")]
#[test]
fn original_symbol_offsets_survive_normalizing_array_initializers() {
    use xgwx::{SfcArrayBound, SfcDeclaration, SfcVariablePatch};
    let mut doc = XgwxDocument::from_path("fixtures/sfc/declarations-array-native.xgwx").unwrap();
    doc.edit_sfc_variable(&SfcVariablePatch {
        program_index: 0,
        expected_variables: doc.sfc_variables(0).unwrap(),
        name: "Numbers".into(),
        data_type: "BOOL".into(),
        description: "".into(),
        remove: false,
        update: true,
        declaration: Some(SfcDeclaration {
            dimensions: vec![SfcArrayBound { lower: 0, upper: 1 }],
            initial_value: "TRUE,FALSE".into(),
            retain: false,
        }),
    })
    .unwrap();
    let before = doc.sfc_variables(0).unwrap();
    assert_eq!(
        before
            .iter()
            .find(|v| v.name == "Numbers")
            .unwrap()
            .declaration
            .as_ref()
            .unwrap()
            .initial_value,
        "TRUE,FALSE"
    );
    doc.edit_sfc_variable(&SfcVariablePatch {
        program_index: 0,
        expected_variables: before.clone(),
        name: "Value_USINT".into(),
        data_type: "USINT".into(),
        description: "".into(),
        remove: true,
        update: false,
        declaration: None,
    })
    .unwrap();
    assert_eq!(
        doc.sfc_variables(0).unwrap(),
        before
            .into_iter()
            .filter(|v| v.name != "Value_USINT")
            .collect::<Vec<_>>()
    );
    assert!(doc.sfc_programs()[0].blocks[0].editable_rows.is_some());
}

#[test]
fn native_strict_check_and_save_as_retain_advanced_declarations_and_st_sources() {
    let before = XgwxDocument::from_path("fixtures/sfc/declarations-generated.xgwx")
        .unwrap()
        .sfc_programs()
        .remove(0);
    let after = XgwxDocument::from_path("fixtures/sfc/declarations-roundtrip.xgwx")
        .unwrap()
        .sfc_programs()
        .remove(0);
    assert_eq!(after.variables_error, None);
    assert_eq!(after.variables, before.variables);
    assert!(after.blocks[0].editable_rows.is_some());
    assert_eq!(
        after.blocks[0].editable_rows,
        before.blocks[0].editable_rows
    );
    assert!(after
        .variables
        .iter()
        .any(|v| v.name == "Matrix" && v.declaration.as_ref().unwrap().dimensions.len() == 2));
    assert!(after
        .variables
        .iter()
        .any(|v| v.name == "Cube" && v.declaration.as_ref().unwrap().initial_value == "8(FALSE)"));
}
