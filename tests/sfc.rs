use xgwx::XgwxDocument;
const NATIVE: &[u8] = include_bytes!("../fixtures/sfc/native-loop.xgwx");

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
    assert_eq!(roundtrip.sfc_programs(), expected);
    assert_eq!(
        roundtrip
            .root
            .descendants_named("LocalVar")
            .collect::<Vec<_>>(),
        doc.root.descendants_named("LocalVar").collect::<Vec<_>>()
    );
}
