#![cfg(feature = "write")]
use xgwx::{NewGlobalVariable, XgwxDocument};
fn variable(name: &str) -> NewGlobalVariable {
    NewGlobalVariable {
        name: name.into(),
        data_type: "BIT".into(),
        address_area: "M".into(),
        address_number: 42,
        description: "설명 😀".into(),
    }
}
#[test]
fn insert_delete_empty_and_populated_tables_roundtrip() {
    for seed in [
        include_bytes!("../fixtures/io-variable-generation/resaved-native.xgwx").as_slice(),
        include_bytes!("../fixtures/empty-projects/new-xgk.xgwx").as_slice(),
    ] {
        let mut doc = XgwxDocument::parse(seed).unwrap();
        let original = doc.clone();
        let before = doc.variable().globals().unwrap();
        let payload = doc.program().ladder_program(0).unwrap().unwrap().data;
        let index = doc.variable_mut().insert(&variable("UserGlobal")).unwrap();
        assert_eq!(
            doc.variable().get(index).unwrap().name(),
            Some("UserGlobal")
        );
        assert_eq!(
            doc.program().ladder_program(0).unwrap().unwrap().data,
            payload
        );
        doc = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
        let snapshot = doc.variable().get(index).unwrap().summary().clone();
        doc.variable_mut()
            .get_if(index, &snapshot)
            .unwrap()
            .delete()
            .unwrap();
        assert_eq!(doc.variable().globals().unwrap(), before);
        assert_eq!(
            doc.program().ladder_program(0).unwrap().unwrap().data,
            payload
        );
        fn outside(doc: &XgwxDocument) -> (String, String) {
            let xml = roxmltree::Document::parse(doc.container().xml()).unwrap();
            let table = xml
                .descendants()
                .find(|n| n.has_tag_name("Symbols"))
                .unwrap();
            (
                doc.container().xml()[..table.range().start].into(),
                doc.container().xml()[table.range().end..].into(),
            )
        }
        assert_eq!(outside(&doc), outside(&original));
    }
}
#[test]
fn insertion_rejects_invalid_inputs_atomically() {
    let mut doc = xgwx::create_project("XGK-CPUSN", "LD").unwrap();
    doc.variable_mut().insert(&variable("Existing")).unwrap();
    for candidate in [
        variable("existing"),
        variable(""),
        variable("bad name"),
        NewGlobalVariable {
            description: "x".repeat(256),
            ..variable("Good")
        },
        NewGlobalVariable {
            data_type: "UNKNOWN".into(),
            ..variable("Good")
        },
        NewGlobalVariable {
            address_number: u32::MAX,
            ..variable("Good")
        },
        NewGlobalVariable {
            description: "SV5.0".into(),
            ..variable("Good")
        },
    ] {
        let before = doc.clone();
        assert!(doc.variable_mut().insert(&candidate).is_err());
        assert_eq!(doc, before);
    }
}
#[test]
fn unicode_limits_and_stale_indices() {
    let mut doc = xgwx::create_project("XGK-CPUSN", "LD").unwrap();
    let mut v = variable("Zulu");
    v.description = "x".repeat(255);
    doc.variable_mut().insert(&v).unwrap();
    let old = doc.variable().get(0).unwrap().summary().clone();
    doc.variable_mut().insert(&variable("Alpha")).unwrap();
    assert!(doc.variable_mut().get_if(0, &old).is_err());
    assert_eq!(doc.variable().get(1).unwrap().summary(), &old);
    assert!(doc.to_verified_bytes().is_ok());
}
#[test]
fn malformed_counts_and_unsupported_layouts_are_atomic() {
    use flate2::{Compression, write::GzEncoder};
    use std::io::Write;
    fn parse(xml: &str) -> XgwxDocument {
        let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
        gzip.write_all(xml.as_bytes()).unwrap();
        let mut bytes = b"XG".to_vec();
        bytes.extend(gzip.finish().unwrap());
        XgwxDocument::parse(&bytes).unwrap()
    }
    for xml in [
        "<Project><Configuration Type='17'><Symbols Count='1'/></Configuration></Project>",
        "<Project><Configuration Type='17'><Symbols Count='0'>AAAA</Symbols></Configuration></Project>",
        "<Project><Configuration Type='17'><Symbols Count='0'/><Symbols Count='0'/></Configuration></Project>",
    ] {
        let mut doc = parse(xml);
        let original = doc.clone();
        assert!(doc.variable_mut().insert(&variable("Test")).is_err());
        assert_eq!(doc, original);
    }
    let mut doc =
        parse("<Project><Configuration Type='17'><Symbols Count='0'/></Configuration></Project>");
    doc.variable_mut().insert(&variable("SelfClosing")).unwrap();
    doc.variable_mut().get(0).unwrap().delete().unwrap();
    assert!(doc.variable().globals().unwrap().is_empty());
    let mut iec = xgwx::create_project("XGI-CPUU", "LD").unwrap();
    let before = iec.clone();
    assert!(iec.variable_mut().insert(&variable("Unsupported")).is_err());
    assert_eq!(iec, before);
}
#[test]
fn deletion_rejects_program_name_references_without_mutation() {
    let mut doc = XgwxDocument::parse(include_bytes!("../fixtures/elements.xgwx")).unwrap();
    let index = doc.variable_mut().insert(&variable("M00000")).unwrap();
    let before = doc.clone();
    let error = doc.variable_mut().get(index).unwrap().delete().unwrap_err();
    assert!(error.to_string().contains("referenced"));
    assert_eq!(doc, before);
}
