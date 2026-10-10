#![cfg(feature = "write")]
use xgwx::{XgwxDocument, create_project};

#[test]
fn generated_native_defaults_preserve_xml_and_security() {
    for (cpu, language, native) in [
        (
            "XGK-CPUSN",
            "LD",
            include_bytes!("../fixtures/empty-projects/new-xgk.xgwx").as_slice(),
        ),
        (
            "XGI-CPUE",
            "LD",
            include_bytes!("../fixtures/empty-projects/new-xgi.xgwx").as_slice(),
        ),
        (
            "XGI-CPUE",
            "SFC",
            include_bytes!("../fixtures/sfc/new-xgi-sfc.xgwx").as_slice(),
        ),
        (
            "XGK-CPUA",
            "ST",
            include_bytes!("../fixtures/text-cpus/xgk-auto-native-blank.xgwx").as_slice(),
        ),
    ] {
        let expected = XgwxDocument::parse(native).unwrap();
        let generated = create_project(cpu, language).unwrap();
        assert_eq!(generated.root, expected.root, "{cpu} {language}");
        let security = |doc: &XgwxDocument| {
            let padding = doc.header.compressed_size_hint.unwrap() as usize - doc.main_gzip.len();
            doc.trailer[padding..].to_vec()
        };
        assert_eq!(security(&generated), security(&expected));
        let bytes = generated.to_verified_bytes().unwrap();
        let aligned = generated.header.compressed_size_hint.unwrap() as usize;
        assert_eq!(aligned % 4, 0);
        let checksum: u32 = bytes[68..134]
            .iter()
            .chain(generated.main_gzip.iter())
            .map(|v| u32::from(*v))
            .sum::<u32>()
            + aligned as u32;
        assert_eq!(
            u32::from_le_bytes(bytes[64..68].try_into().unwrap()),
            checksum
        );
    }
}

#[test]
fn all_editor_cpu_language_choices_generate_editable_blank_programs() {
    for cpu in [
        "XGI-CPUE",
        "XGK-CPUA",
        "XGB-XECE",
        "XGB-XECH",
        "XGB-XECS",
        "XGB-XECU",
        "XGB-XEMH2",
        "XGB-XEMHP",
        "XGB-GIPAM",
        "XGB-KL",
        "XGR-CPUH",
    ] {
        for language in ["ST", "IL"] {
            if cpu == "XGK-CPUA" && language == "IL" {
                continue;
            }
            let doc = create_project(cpu, language).unwrap();
            assert_eq!(doc.programs().len(), 1, "{cpu} {language}");
            assert_eq!(doc.programs()[0].name.as_deref(), Some("NewProgram"));
            let texts = doc.text_programs();
            assert_eq!(texts.len(), 1, "{cpu} {language}");
            let program = &texts[0];
            assert_eq!(program.language, language);
            assert_eq!(program.source.as_deref(), Some(""));
            assert!(program.editable);
            doc.to_verified_bytes().unwrap();
        }
    }
    create_project("XGK-CPUSN", "IL").unwrap();
    assert!(create_project("missing", "LD").is_err());
    assert!(create_project("XGK-CPUSN", "ST").is_err());
    assert!(create_project("XGB-XECE", "SFC").is_err());
    assert!(create_project("XGI-CPUE", "unknown").is_err());
}
