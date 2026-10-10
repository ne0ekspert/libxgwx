#![cfg(all(feature = "write", feature = "il"))]
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
        assert_eq!(original.configurations()[0].type_code, Some(code));
        for language in ["ST", "IL"] {
            let mut doc = original.clone();
            let index = doc.programs().len();
            doc.create_program(&NewProgram {
                name: "Added".into(),
                language: language.into(),
                object_id: "abcd0001-1234-4567-89ab-0123456789ab".into(),
                symbol_id: "abcd0002-1234-4567-89ab-0123456789ab".into(),
            })
            .unwrap();
            let text = doc
                .text_programs()
                .into_iter()
                .find(|p| p.program_index == index)
                .unwrap();
            assert!(text.editable);
            doc.edit_text_program(&TextProgramPatch {
                program_index: index,
                expected_object_id: text.object_id,
                expected_language: language.into(),
                expected_source: "".into(),
                source: if language == "ST" {
                    "%MX0 := TRUE;".into()
                } else {
                    "LD 1\nST %MW0".into()
                },
            })
            .unwrap();
            assert_eq!(doc.modules(), original.modules());
            assert_eq!(doc.network_modules(), original.network_modules());
            assert_eq!(doc.configurations(), original.configurations());
            let saved = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
            assert_eq!(saved.text_programs(), doc.text_programs());
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
    assert!(doc.create_program(&patch).is_err());
    assert_eq!(doc, source);
    let doc = XgwxDocument::from_path("fixtures/text-cpus/xgk-auto-native-blank.xgwx").unwrap();
    assert!(doc.program_languages().contains(&"ST"));
    assert!(!doc.program_languages().contains(&"IL"));
    for code in [110, 2, 18, 999] {
        let mut doc = source.clone();
        doc.xml = doc.xml.replace("Type=\"17\"", &format!("Type=\"{code}\""));
        let mut doc = XgwxDocument::parse(&doc.to_bytes().unwrap()).unwrap();
        let before = doc.clone();
        assert!(doc.create_program(&patch).is_err());
        assert_eq!(doc, before);
    }
}
#[test]
fn vendor_il_is_native_ladder_and_rejects_stale_or_unsupported_source() {
    let mut doc = XgwxDocument::from_path("fixtures/empty-projects/new-xgk.xgwx").unwrap();
    let p = doc.vendor_il_programs().remove(0);
    let patch = VendorIlPatch {
        program_index: 0,
        expected_object_id: p.object_id,
        expected_source: p.source.unwrap(),
        source: "LOAD M00000\nAND NOT M00001\nOUT M00002\nLOADP M00003\nMOV 1 D100".into(),
    };
    doc.edit_vendor_il(&patch).unwrap();
    assert_eq!(doc.programs()[0].kind, Some(0));
    assert_eq!(
        doc.ladder_program(0).unwrap().unwrap().project_type,
        Some(1)
    );
    assert_eq!(
        doc.vendor_il_programs()[0].source.as_deref(),
        Some(patch.source.as_str())
    );
    let before = doc.clone();
    assert!(doc.edit_vendor_il(&patch).is_err());
    assert_eq!(doc, before);
    for source in [
        "LD TRUE\nST Flag",
        "LOAD M00000\nOR M00001\nOUT M00002",
        "LOAD M00000",
        "LOAD M00000\nNO_SUCH_OPCODE D100",
        "LOAD M00000\nOUT M00001\nOUT M00002",
    ] {
        let p = doc.vendor_il_programs().remove(0);
        let patch = VendorIlPatch {
            expected_source: p.source.unwrap(),
            source: source.into(),
            ..patch.clone()
        };
        assert!(doc.edit_vendor_il(&patch).is_err(), "{source}");
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
        let p = doc.text_programs().remove(0);
        doc.edit_text_variable(
            &p.object_id,
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
        .unwrap();
        assert_eq!(doc.text_programs()[0].variables[0].data_type, name);
        let table = doc
            .root
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
            generated.configurations()[0].type_code,
            saved.configurations()[0].type_code,
            "{stem}"
        );
        for p in generated.text_programs() {
            let actual = saved
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
        generated.vendor_il_programs()[0].source,
        saved.vendor_il_programs()[0].source
    );
    assert!(saved.vendor_il_programs()[0].editable);
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
        fn replace(node: &mut xgwx::XmlElement, raw: &[u8], local: bool) -> bool {
            let local = local || node.name == "LocalVar";
            if local && node.name == "Symbols" {
                node.text = base64::engine::general_purpose::STANDARD.encode(raw);
                for a in &mut node.attributes {
                    if a.name == "Count" {
                        a.value = "1".into();
                    }
                    if a.name == "Compressed" {
                        a.value = "0".into();
                    }
                }
                return true;
            }
            node.children.iter_mut().any(|n| replace(n, raw, local))
        }
        // The native blank has only its local Symbols table.
        assert!(replace(&mut doc.root, &raw, false));
        let p = doc.text_programs().remove(0);
        assert_eq!(p.variables_error, None, "{ty}");
        assert_eq!(
            p.variables[0].data_type,
            if ty == "BIT" { "BOOL" } else { ty }
        );
    }
}
