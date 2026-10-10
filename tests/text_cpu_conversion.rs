#![cfg(feature = "write")]
use xgwx::XgwxDocument;
const MODELS: &[(&str, u32)] = &[
    ("XGI-CPUU", 100),
    ("XGI-CPUH", 102),
    ("XGI-CPUS", 104),
    ("XGI-CPUE", 106),
    ("XGI-CPUU/D", 107),
    ("XGI-CPUUN", 111),
];
#[test]
fn text_programs_change_between_all_captured_xgi_models_without_source_or_declaration_changes() {
    for bytes in [
        include_bytes!("../fixtures/text-programs/generated.xgwx").as_slice(),
        include_bytes!("../fixtures/text-programs/native-roundtrip.xgwx").as_slice(),
    ] {
        let original = XgwxDocument::parse(bytes).unwrap();
        let expected = original.text_programs();
        for (from, _) in MODELS {
            let mut start = original.clone();
            start.select_cpu(from).unwrap();
            for (to, code) in MODELS {
                let mut doc = start.clone();
                doc.select_cpu(to).unwrap();
                assert_eq!(doc.configurations()[0].type_code, Some(*code));
                assert_eq!(doc.text_programs(), expected, "{from} -> {to}");
                assert_eq!(doc.programs(), original.programs());
                assert_eq!(
                    doc.root.descendants_named("Safety_Comm").count(),
                    usize::from(*code == 111)
                );
                let saved = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
                assert_eq!(saved.text_programs(), expected);
                doc.select_cpu(from).unwrap();
                assert_eq!(doc.text_programs(), expected);
                assert_eq!(
                    doc.root.attribute("WksNodeCount"),
                    start.root.attribute("WksNodeCount")
                );
            }
        }
    }
}
#[test]
fn unsafe_text_cpu_changes_fail_without_modifying_the_document() {
    let original =
        XgwxDocument::parse(include_bytes!("../fixtures/text-programs/generated.xgwx")).unwrap();
    for target in ["XGK-CPUSN", "XGB-XBMS", "XGI-CPUS/P"] {
        let mut doc = original.clone();
        assert!(doc.select_cpu(target).is_err());
        assert_eq!(doc, original);
    }
    for xml in [
        original
            .xml
            .replace("SCAN_WD_TIME_0=\"500\"", "SCAN_WD_TIME_0=\"600\""),
        original
            .xml
            .replace("<Bookmarks>", "<Bookmarks><Bookmark Line=\"1\"/>"),
    ] {
        assert_ne!(xml, original.xml);
        let mut doc = original.clone();
        doc.xml = xml;
        doc = XgwxDocument::parse(&doc.to_bytes().unwrap()).unwrap();
        let unchanged = doc.clone();
        assert!(doc.select_cpu("XGI-CPUUN").is_err());
        assert_eq!(doc, unchanged);
    }
}
#[test]
fn mixed_sfc_st_il_projects_preserve_every_program_through_cpu_conversion() {
    let mut doc =
        XgwxDocument::parse(include_bytes!("../fixtures/sfc/st-programs-generated.xgwx")).unwrap();
    for (n, language) in ["ST", "IL"].iter().enumerate() {
        doc.create_program(&xgwx::NewProgram {
            name: format!("Text{language}"),
            language: (*language).into(),
            object_id: format!("abcd000{}-1234-4567-89ab-0123456789ab", n * 2 + 1),
            symbol_id: format!("abcd000{}-1234-4567-89ab-0123456789ab", n * 2 + 2),
        })
        .unwrap();
    }
    let before = doc.clone();
    doc.select_cpu("XGI-CPUUN").unwrap();
    assert_eq!(doc.sfc_programs(), before.sfc_programs());
    assert_eq!(doc.text_programs(), before.text_programs());
    doc.select_cpu("XGI-CPUE").unwrap();
    assert_eq!(doc.sfc_programs(), before.sfc_programs());
    assert_eq!(doc.text_programs(), before.text_programs());
}

#[test]
fn freshly_created_text_projects_accept_captured_blank_parameter_defaults() {
    for language in ["ST", "IL"] {
        let mut doc =
            XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgi.xgwx")).unwrap();
        let identity = doc.programs()[0].object_id.clone().unwrap();
        doc.delete_program(0, &identity).unwrap();
        doc.create_program(&xgwx::NewProgram {
            name: "NewProgram".into(),
            language: language.into(),
            object_id: "aaaa0001-1234-4567-89ab-0123456789ab".into(),
            symbol_id: "aaaa0002-1234-4567-89ab-0123456789ab".into(),
        })
        .unwrap();
        let before = doc.text_programs();
        for (model, _) in MODELS {
            doc.select_cpu(model).unwrap();
            assert_eq!(doc.text_programs(), before);
        }
    }
}

#[test]
fn text_cpu_conversion_round_trips_through_native_xg5000_save_as() {
    for (generated, saved, code) in [
        (
            include_bytes!("../fixtures/text-programs/cpu/cpuun-generated.xgwx").as_slice(),
            include_bytes!("../fixtures/text-programs/cpu/cpuun-native.xgwx").as_slice(),
            111,
        ),
        (
            include_bytes!("../fixtures/text-programs/cpu/cpuh-generated.xgwx").as_slice(),
            include_bytes!("../fixtures/text-programs/cpu/cpuh-native.xgwx").as_slice(),
            102,
        ),
        (
            include_bytes!("../fixtures/text-programs/cpu/cpue-generated.xgwx").as_slice(),
            include_bytes!("../fixtures/text-programs/cpu/cpue-native.xgwx").as_slice(),
            106,
        ),
        (
            include_bytes!("../fixtures/text-programs/cpu/cpus-generated.xgwx").as_slice(),
            include_bytes!("../fixtures/text-programs/cpu/cpus-native.xgwx").as_slice(),
            104,
        ),
        (
            include_bytes!("../fixtures/text-programs/cpu/cpuu-generated.xgwx").as_slice(),
            include_bytes!("../fixtures/text-programs/cpu/cpuu-native.xgwx").as_slice(),
            100,
        ),
        (
            include_bytes!("../fixtures/text-programs/cpu/cpuud-generated.xgwx").as_slice(),
            include_bytes!("../fixtures/text-programs/cpu/cpuud-native.xgwx").as_slice(),
            107,
        ),
    ] {
        let generated = XgwxDocument::parse(generated).unwrap();
        let saved = XgwxDocument::parse(saved).unwrap();
        assert_eq!(saved.configurations()[0].type_code, Some(code));
        assert_eq!(saved.text_programs(), generated.text_programs());
        let mut reverse = saved.clone();
        reverse.select_cpu("XGI-CPUE").unwrap();
        assert_eq!(reverse.text_programs(), generated.text_programs());
    }
}

#[cfg(feature = "il")]
#[test]
fn xgk_cpusn_basic_mnemonics_match_the_vendor_instruction_manual() {
    let doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/text-programs/cpu/xgk-il-generated.xgwx"
    ))
    .unwrap();
    assert_eq!(doc.configurations()[0].type_code, Some(17));
    assert_eq!(
        doc.ladder_programs()
            .remove(0)
            .unwrap()
            .to_il()
            .unwrap()
            .to_string(),
        include_str!("../fixtures/text-programs/cpu/xgk-il.txt")
    );
}
