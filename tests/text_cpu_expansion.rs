#![cfg(all(feature = "write", feature = "il"))]
#[cfg(feature = "write")]
#[path = "support/mutation_support.rs"]
mod mutation_support;
use xgwx::{NewProgram, TextProgramPatch, VendorIlPatch, XgwxDocument};
#[test]
fn compact_and_redundant_projects_create_edit_and_preserve_hardware() {
    for (stem, code) in [
        ("xece", 109),
        ("xech", 103),
        ("xecs", 108),
        ("xecu", 112),
        ("xemh2", 116),
        ("xemhp", 115),
        ("gipam", 114),
        ("kl", 113),
        ("xgr", 101),
    ] {
        let original =
            XgwxDocument::from_path(format!("fixtures/text-cpus/{stem}-native-blank.xgwx"))
                .unwrap();
        assert_eq!(original.configuration().list()[0].type_code, Some(code));
        for language in ["ST", "IL"] {
            let mut doc = original.clone();
            let index = doc.program().programs().len();
            doc.program_mut()
                .create(&NewProgram {
                    name: "Added".into(),
                    language: language.into(),
                    object_id: "abcd0001-1234-4567-89ab-0123456789ab".into(),
                    symbol_id: "abcd0002-1234-4567-89ab-0123456789ab".into(),
                })
                .unwrap();
            let text = doc
                .program()
                .text_programs()
                .into_iter()
                .find(|p| p.program_index == index)
                .unwrap();
            assert!(text.editable);
            doc.program_mut()
                .get(index)
                .and_then(|selected_program| {
                    mutation_support::edit_text_program(
                        selected_program,
                        &TextProgramPatch {
                            program_index: index,
                            expected_object_id: text.object_id,
                            expected_language: language.into(),
                            expected_source: "".into(),
                            source: if language == "ST" {
                                "%MX0 := TRUE;".into()
                            } else {
                                "LD 1\nST %MW0".into()
                            },
                        },
                    )
                })
                .unwrap();
            assert_eq!(doc.module().summaries(), original.module().summaries());
            assert_eq!(
                doc.network().module_summaries(),
                original.network().module_summaries()
            );
            assert_eq!(doc.configuration().list(), original.configuration().list());
            let saved = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
            assert_eq!(
                saved.program().text_programs(),
                doc.program().text_programs()
            );
        }
    }
}
#[test]
fn cpu_and_mode_guards_fail_atomically() {
    let source = XgwxDocument::from_path("fixtures/empty-projects/new-xgk.xgwx").unwrap();
    let mut doc = source.clone();
    let patch = NewProgram {
        name: "NoST".into(),
        language: "ST".into(),
        object_id: "abcd0001-1234-4567-89ab-0123456789ab".into(),
        symbol_id: "abcd0002-1234-4567-89ab-0123456789ab".into(),
    };
    assert!(doc.program_mut().create(&patch).is_err());
    assert_eq!(doc, source);
    let doc = XgwxDocument::from_path("fixtures/text-cpus/xgk-auto-native-blank.xgwx").unwrap();
    assert!(doc.program().program_languages().contains(&"ST"));
    assert!(!doc.program().program_languages().contains(&"IL"));
    for code in [110, 2, 18, 999] {
        let mut doc = source.clone();
        {
            let replacement_xml = doc
                .container()
                .xml()
                .replace("Type=\"17\"", &format!("Type=\"{code}\""));
            doc.container_mut().replace_xml(replacement_xml).unwrap();
        };
        let mut doc = XgwxDocument::parse(&doc.to_bytes().unwrap()).unwrap();
        let before = doc.clone();
        assert!(doc.program_mut().create(&patch).is_err());
        assert_eq!(doc, before);
    }
}
#[test]
fn vendor_il_is_native_ladder_and_rejects_stale_or_unsupported_source() {
    let mut doc = XgwxDocument::from_path("fixtures/empty-projects/new-xgk.xgwx").unwrap();
    let p = doc.program().vendor_il_programs().remove(0);
    let patch = VendorIlPatch {
        program_index: 0,
        expected_object_id: p.object_id,
        expected_source: p.source.unwrap(),
        source: "LOAD M00000\nAND NOT M00001\nOUT M00002\nLOADP M00003\nMOV 1 D100".into(),
    };
    doc.program_mut()
        .get((&patch).program_index)
        .and_then(|selected_program| mutation_support::edit_vendor_il(selected_program, &patch))
        .unwrap();
    assert_eq!(doc.program().programs()[0].kind, Some(0));
    assert_eq!(
        doc.program()
            .ladder_program(0)
            .unwrap()
            .unwrap()
            .project_type,
        Some(1)
    );
    assert_eq!(
        doc.program().vendor_il_programs()[0].source.as_deref(),
        Some(patch.source.as_str())
    );
    let before = doc.clone();
    assert!(
        doc.program_mut()
            .get((&patch).program_index)
            .and_then(|selected_program| mutation_support::edit_vendor_il(selected_program, &patch))
            .is_err()
    );
    assert_eq!(doc, before);
    for source in [
        "LD TRUE\nST Flag",
        "LOAD M00000\nOR M00001\nOUT M00002",
        "LOAD M00000",
        "LOAD M00000\nNO_SUCH_OPCODE D100",
        "LOAD M00000\nOUT M00001\nOUT M00002",
    ] {
        let p = doc.program().vendor_il_programs().remove(0);
        let patch = VendorIlPatch {
            expected_source: p.source.unwrap(),
            source: source.into(),
            ..patch.clone()
        };
        assert!(
            doc.program_mut()
                .get((&patch).program_index)
                .and_then(|selected_program| mutation_support::edit_vendor_il(
                    selected_program,
                    &patch
                ))
                .is_err(),
            "{source}"
        );
        assert_eq!(doc, before);
    }
}

#[test]
fn native_xgk_scalar_codes_decode_and_write_without_iec_type_confusion() {
    for (name, code) in [
        ("BOOL", 1),
        ("BYTE", 3),
        ("WORD", 4),
        ("DWORD", 5),
        ("LWORD", 6),
        ("SINT", 16),
        ("INT", 9),
        ("DINT", 10),
        ("LINT", 11),
        ("USINT", 17),
        ("UINT", 18),
        ("UDINT", 19),
        ("ULINT", 20),
        ("REAL", 7),
        ("LREAL", 8),
    ] {
        let mut doc =
            XgwxDocument::from_path("fixtures/text-cpus/xgk-auto-native-blank.xgwx").unwrap();
        let p = doc.program().text_programs().remove(0);
        doc.program_mut()
            .get_if(
                (&xgwx::SfcVariablePatch {
                    program_index: 0,
                    expected_variables: vec![],
                    name: "Count".into(),
                    data_type: name.into(),
                    description: "".into(),
                    remove: false,
                    update: false,
                    declaration: None,
                })
                    .program_index,
                &p.object_id,
            )
            .and_then(|selected_program| {
                mutation_support::edit_text_variable(
                    selected_program,
                    &xgwx::SfcVariablePatch {
                        program_index: 0,
                        expected_variables: vec![],
                        name: "Count".into(),
                        data_type: name.into(),
                        description: "".into(),
                        remove: false,
                        update: false,
                        declaration: None,
                    },
                )
            })
            .unwrap();
        assert_eq!(
            doc.program().text_programs()[0].variables[0].data_type,
            name
        );
        let table = doc
            .container()
            .root()
            .descendants_named("Symbols")
            .find(|s| s.attribute("Count") == Some("1"))
            .unwrap();
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&table.text)
            .unwrap();
        let marker = "Count"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        let at = bytes
            .windows(marker.len())
            .position(|b| b == marker)
            .unwrap()
            + marker.len()
            + 4;
        assert_eq!(
            u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()),
            code
        );
    }
}
#[test]
fn native_roundtrip_preserves_sources_and_declarations_on_captured_cpus() {
    for stem in [
        "xgk-auto", "xece", "xech", "xecs", "xecu", "xemh2", "xemhp", "gipam", "kl", "xgr",
    ] {
        let generated =
            XgwxDocument::from_path(format!("fixtures/text-cpus/{stem}-generated.xgwx")).unwrap();
        let saved =
            XgwxDocument::from_path(format!("fixtures/text-cpus/{stem}-native-roundtrip.xgwx"))
                .unwrap();
        assert_eq!(
            generated.configuration().list()[0].type_code,
            saved.configuration().list()[0].type_code,
            "{stem}"
        );
        for p in generated.program().text_programs() {
            let actual = saved
                .program()
                .text_programs()
                .into_iter()
                .find(|v| v.object_id == p.object_id)
                .unwrap();
            assert_eq!(actual.source, p.source, "{stem}");
            assert_eq!(actual.variables, p.variables, "{stem}");
            assert!(actual.editable);
        }
    }
}

#[test]
fn vendor_il_native_save_as_preserves_instructions() {
    let generated = XgwxDocument::from_path("fixtures/text-cpus/xgk-il-generated.xgwx").unwrap();
    let saved = XgwxDocument::from_path("fixtures/text-cpus/xgk-il-native-roundtrip.xgwx").unwrap();
    assert_eq!(
        generated.program().vendor_il_programs()[0].source,
        saved.program().vendor_il_programs()[0].source
    );
    assert!(saved.program().vendor_il_programs()[0].editable);
}
#[test]
fn captured_native_xgk_scalar_tables_keep_their_types() {
    use base64::Engine;
    for ty in [
        "BIT", "BYTE", "WORD", "DWORD", "LWORD", "SINT", "INT", "DINT", "LINT", "USINT", "UINT",
        "UDINT", "ULINT", "REAL", "LREAL",
    ] {
        let mut doc =
            XgwxDocument::from_path("fixtures/text-cpus/xgk-auto-native-blank.xgwx").unwrap();
        let raw = std::fs::read(format!(
            "fixtures/text-cpus/scalars/{}.bin",
            ty.to_lowercase()
        ))
        .unwrap();
        let parsed = roxmltree::Document::parse(doc.container().xml()).unwrap();
        let symbols = parsed
            .descendants()
            .find(|node| {
                node.has_tag_name("Symbols")
                    && node
                        .ancestors()
                        .any(|parent| parent.has_tag_name("LocalVar"))
            })
            .unwrap();
        let range = symbols.range();
        let replacement = format!(
            r#"<Symbols Count="1" Compressed="0" Format="Base64">{}</Symbols>"#,
            base64::engine::general_purpose::STANDARD.encode(&raw)
        );
        let mut xml = doc.container().xml().to_owned();
        xml.replace_range(range, &replacement);
        {
            let replacement_xml = xml;
            doc.container_mut().replace_xml(replacement_xml).unwrap();
        };
        let p = doc.program().text_programs().remove(0);
        assert_eq!(p.variables_error, None, "{ty}");
        assert_eq!(
            p.variables[0].data_type,
            if ty == "BIT" { "BOOL" } else { ty }
        );
    }
}
