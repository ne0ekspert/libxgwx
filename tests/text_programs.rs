use xgwx::XgwxDocument;
const BLANK: &[u8] = include_bytes!("../fixtures/text-programs/native-blank.xgwx");
#[test]
fn native_st_and_il_share_source_container_but_have_distinct_languages() {
    let doc = XgwxDocument::parse(BLANK).unwrap();
    let text = doc.text_programs();
    assert_eq!(
        text.iter()
            .map(|p| (
                p.program_index,
                p.language.as_str(),
                p.source.as_deref(),
                p.editable
            ))
            .collect::<Vec<_>>(),
        vec![(2, "ST", Some(""), true), (3, "IL", Some(""), true)]
    );
    assert_eq!(doc.to_bytes().unwrap(), BLANK);
}
#[cfg(feature = "write")]
#[test]
fn edits_preserve_other_programs_and_metadata_and_reject_stale_or_unknown_layouts() {
    use xgwx::TextProgramPatch;
    let original = XgwxDocument::parse(BLANK).unwrap();
    for text in original.text_programs() {
        let mut doc = original.clone();
        let source = if text.language == "ST" {
            "(* 한글 😀 *)\r\n%MX0 := TRUE;\r\n"
        } else {
            "start: LD TRUE\r\nST %MX0\r\nRET\r\n"
        };
        let patch = TextProgramPatch {
            program_index: text.program_index,
            expected_object_id: text.object_id.clone(),
            expected_language: text.language.clone(),
            expected_source: String::new(),
            source: source.into(),
        };
        doc.edit_text_program(&patch).unwrap();
        let saved = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
        assert_eq!(
            saved
                .text_programs()
                .into_iter()
                .find(|p| p.program_index == text.program_index)
                .unwrap()
                .source
                .as_deref(),
            Some(source)
        );
        assert_eq!(saved.programs(), original.programs());
        assert_eq!(
            saved
                .iec_local_symbols()
                .remove(text.program_index)
                .unwrap(),
            original
                .iec_local_symbols()
                .remove(text.program_index)
                .unwrap()
        );
        assert_eq!(doc.trailer, original.trailer);
        for (i, (a, b)) in saved
            .root
            .descendants_named("Program")
            .zip(original.root.descendants_named("Program"))
            .enumerate()
        {
            if i != text.program_index {
                assert_eq!(a, b);
            }
        }
        let unchanged = doc.clone();
        assert!(doc.edit_text_program(&patch).is_err());
        assert_eq!(doc, unchanged);
        for bad in [
            TextProgramPatch {
                expected_object_id: "stale".into(),
                ..patch.clone()
            },
            TextProgramPatch {
                expected_language: "LD".into(),
                ..patch.clone()
            },
            TextProgramPatch {
                source: "\0".into(),
                ..patch.clone()
            },
            TextProgramPatch {
                source: "😀".repeat(32769),
                ..patch.clone()
            },
        ] {
            let mut rejected = original.clone();
            assert!(rejected.edit_text_program(&bad).is_err());
            assert_eq!(rejected, original);
        }
        let mut reverse = patch.clone();
        reverse.expected_source = source.into();
        reverse.source.clear();
        doc.edit_text_program(&reverse).unwrap();
        assert_eq!(doc.text_programs(), original.text_programs());
    }
    let mut unknown = original.clone();
    unknown.xml = unknown
        .xml
        .replace("<Bookmarks>", "<Bookmarks><Bookmark Line=\"1\"/>");
    // Reparse a modified container so the XML tree and source agree.
    unknown = xgwx::XgwxDocument::parse(&unknown.to_bytes().unwrap()).unwrap();
    assert!(unknown.text_programs().iter().all(|p| !p.editable));
}
#[cfg(feature = "write")]
#[test]
fn native_templates_create_st_and_il_with_fresh_identities() {
    use xgwx::NewProgram;
    for language in ["ST", "IL"] {
        let mut doc =
            XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgi.xgwx")).unwrap();
        let before = doc.clone();
        doc.create_program(&NewProgram {
            name: "AddedText".into(),
            language: language.into(),
            object_id: "abcd0001-1234-4567-89ab-0123456789ab".into(),
            symbol_id: "abcd0002-1234-4567-89ab-0123456789ab".into(),
        })
        .unwrap();
        let text = doc.text_programs();
        assert_eq!(text.len(), 1);
        assert_eq!(text[0].language, language);
        assert!(text[0].editable);
        assert_eq!(doc.programs()[0], before.programs()[0]);
        assert_eq!(
            doc.iec_local_symbols().remove(0).unwrap(),
            before.iec_local_symbols().remove(0).unwrap()
        );
    }
}
#[cfg(feature = "write")]
#[test]
fn source_declarations_use_iec_records_and_protect_references() {
    use xgwx::{SfcVariablePatch, TextProgramPatch};
    let mut doc = XgwxDocument::parse(BLANK).unwrap();
    for index in [2, 3] {
        let text = doc
            .text_programs()
            .into_iter()
            .find(|p| p.program_index == index)
            .unwrap();
        let patch = SfcVariablePatch {
            program_index: index,
            expected_variables: text.variables.clone(),
            name: "Count".into(),
            data_type: "INT".into(),
            description: "Counter".into(),
            remove: false,
            declaration: None,
            update: false,
        };
        let unchanged = doc.clone();
        assert!(doc.edit_text_variable("stale", &patch).is_err());
        assert_eq!(doc, unchanged);
        doc.edit_text_variable(&text.object_id, &patch).unwrap();
        let current = doc
            .text_programs()
            .into_iter()
            .find(|p| p.program_index == index)
            .unwrap();
        assert_eq!(current.variables.len(), 1);
        assert_eq!(current.variables[0].name, "Count");
        assert_eq!(current.variables[0].data_type, "INT");
        let source = if text.language == "ST" {
            "Count := 1;"
        } else {
            "LD 1\nST Count"
        };
        doc.edit_text_program(&TextProgramPatch {
            program_index: index,
            expected_object_id: text.object_id.clone(),
            expected_language: text.language,
            expected_source: String::new(),
            source: source.into(),
        })
        .unwrap();
        let mut remove = patch;
        remove.expected_variables = current.variables;
        remove.remove = true;
        let unchanged = doc.clone();
        assert!(doc.edit_text_variable(&text.object_id, &remove).is_err());
        assert_eq!(doc, unchanged);
    }
}
#[test]
fn native_strict_check_save_as_retains_sources_and_advanced_declarations() {
    let generated =
        XgwxDocument::parse(include_bytes!("../fixtures/text-programs/generated.xgwx")).unwrap();
    let saved = XgwxDocument::parse(include_bytes!(
        "../fixtures/text-programs/native-roundtrip.xgwx"
    ))
    .unwrap();
    assert_eq!(saved.text_programs().len(), 4);
    for p in generated.text_programs() {
        let actual = saved
            .text_programs()
            .into_iter()
            .find(|v| v.object_id == p.object_id)
            .unwrap();
        assert_eq!(actual.language, p.language);
        assert_eq!(actual.source, p.source);
        assert_eq!(actual.variables, p.variables);
        assert!(actual.editable);
    }
}
