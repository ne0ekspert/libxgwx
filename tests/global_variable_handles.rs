use flate2::{Compression, write::GzEncoder};
use std::io::Write;
use xgwx::{XgwxDocument, XgwxError};
fn document(xml: &str) -> XgwxDocument {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    XgwxDocument::parse(&bytes).unwrap()
}
#[test]
fn missing_table_and_out_of_range_selection_are_explicit() {
    let doc = document("<Project/>");
    assert!(doc.variable().list().unwrap().is_empty());
    assert!(matches!(
        doc.variable().get(0),
        Err(XgwxError::VariableNotFound { index: 0 })
    ));
}
#[test]
fn native_read_handles_retain_indices_and_case_insensitive_selection() {
    let doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/io-variable-generation/resaved-native.xgwx"
    ))
    .unwrap();
    let snapshots = doc.variable().globals().unwrap();
    let handles = doc.variable().list().unwrap();
    assert_eq!(handles.len(), 120);
    for (index, handle) in handles.iter().enumerate() {
        assert_eq!(handle.index(), index);
        assert_eq!(handle.summary(), &snapshots[index]);
    }
    let name = handles[0].name().unwrap();
    assert_eq!(
        doc.variable().named(&name.to_lowercase()).unwrap().index(),
        0
    );
    assert_eq!(
        handles[0].variables().get(1).unwrap().summary(),
        &snapshots[1]
    );
    assert!(doc.variable().named("not a global").is_err());
}
#[test]
fn duplicate_names_and_multiple_tables_are_not_silently_selected() {
    use base64::Engine;
    let mut payload = Vec::new();
    for name in ["Count", "COUNT"] {
        for field in ["SV5.0", name, "M", "BOOL", "", "", ""] {
            payload.extend_from_slice(b"\xff\xfe\xff");
            payload.push(field.encode_utf16().count() as u8);
            for unit in field.encode_utf16() {
                payload.extend(unit.to_le_bytes());
            }
            if field == "M" {
                payload.extend(0u32.to_le_bytes());
            }
        }
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(payload);
    let doc = document(&format!("<Project><Symbols>{encoded}</Symbols></Project>"));
    assert_eq!(doc.variable().list().unwrap().len(), 2);
    assert!(doc.variable().named("count").is_err());
    let ambiguous = document(
        "<Project><Configuration><Symbols/></Configuration><Configuration><Symbols/></Configuration></Project>",
    );
    assert!(ambiguous.variable().list().is_err());
    assert!(ambiguous.variable().get(0).is_err());
}
#[cfg(feature = "write")]
mod writes {
    use super::*;
    use xgwx::VariablePatch;
    fn project() -> XgwxDocument {
        let mut doc = xgwx::create_project("XGK-CPUSN", "LD").unwrap();
        doc.base_mut()
            .by_number(0)
            .unwrap()
            .insert_module(0, "XGI-D21A")
            .unwrap();
        doc.variable_mut().generate_io_variables().unwrap();
        doc
    }
    #[test]
    fn selected_updates_rebind_and_preserve_other_symbols_and_xml() {
        let mut doc = project();
        let original = doc.clone();
        let symbols = doc.variable().globals().unwrap();
        let mut variable = doc.variable_mut().get(0).unwrap();
        variable.rename("Long_name_변수").unwrap();
        variable
            .update(&VariablePatch {
                description: Some("Unicode 😀".into()),
                address_number: Some(42),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(variable.index(), 0);
        assert_eq!(variable.summary().name.as_deref(), Some("Long_name_변수"));
        assert_eq!(
            variable.view().summary().description.as_deref(),
            Some("Unicode 😀")
        );
        variable.rename("V").unwrap();
        let after = doc.variable().globals().unwrap();
        assert_eq!(after[0].address_number, Some(42));
        assert_eq!(after[1..], symbols[1..]);
        fn outside(doc: &XgwxDocument) -> (String, String) {
            let xml = roxmltree::Document::parse(&doc.container().xml()).unwrap();
            let node = xml
                .descendants()
                .find(|n| n.has_tag_name("Symbols"))
                .unwrap();
            (
                doc.container().xml()[..node.range().start].into(),
                doc.container().xml()[node.range().end..].into(),
            )
        }
        assert_eq!(outside(&doc), outside(&original));
        assert_eq!(
            XgwxDocument::parse(&doc.to_verified_bytes().unwrap())
                .unwrap()
                .variable()
                .globals()
                .unwrap(),
            after
        );
    }
    #[test]
    fn duplicate_and_invalid_strings_leave_document_and_handle_unchanged() {
        let mut doc = project();
        let duplicate = doc
            .variable()
            .get(1)
            .unwrap()
            .name()
            .unwrap()
            .to_lowercase();
        for patch in [
            VariablePatch {
                name: Some(duplicate),
                ..Default::default()
            },
            VariablePatch {
                name: Some("".into()),
                ..Default::default()
            },
            VariablePatch {
                description: Some("x".repeat(256)),
                ..Default::default()
            },
            VariablePatch {
                name: Some("bad\nname".into()),
                ..Default::default()
            },
            VariablePatch {
                address_area: Some("LONG".into()),
                ..Default::default()
            },
            VariablePatch {
                data_type: Some("OTHER_TYPE".into()),
                ..Default::default()
            },
            VariablePatch {
                name: Some("SV5.0".into()),
                ..Default::default()
            },
        ] {
            let before = doc.clone();
            let mut variable = doc.variable_mut().get(0).unwrap();
            let expected = variable.summary().clone();
            assert!(variable.update(&patch).is_err());
            assert_eq!(variable.summary(), &expected);
            assert_eq!(doc, before);
        }
    }
    #[test]
    fn captured_metadata_detects_changes_and_generation_overwrites() {
        let mut doc = project();
        let expected = doc.variable().get(0).unwrap().summary().clone();
        doc.variable_mut()
            .get_if(0, &expected)
            .unwrap()
            .rename("CustomInput")
            .unwrap();
        let current = doc.variable().get(0).unwrap().summary().clone();
        let before = doc.clone();
        assert!(doc.variable_mut().get_if(0, &expected).is_err());
        assert_eq!(doc, before);
        doc.variable_mut().generate_io_variables().unwrap();
        let regenerated = doc.clone();
        assert!(doc.variable_mut().get_if(0, &current).is_err());
        assert_eq!(doc, regenerated);
        assert_eq!(doc.variable().get(0).unwrap().summary(), &expected);
    }
    #[test]
    fn named_editor_checks_uniqueness_then_keeps_record_index_after_rename() {
        let mut doc = project();
        let name = doc
            .variable()
            .get(1)
            .unwrap()
            .name()
            .unwrap()
            .to_lowercase();
        let mut selected = doc.variable_mut().named(&name).unwrap();
        assert_eq!(selected.index(), 1);
        selected.rename("SelectedInput").unwrap();
        selected
            .update(&VariablePatch {
                description: Some("Updated".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(doc.variable().named("selectedinput").unwrap().index(), 1);
    }
    #[test]
    fn ambiguous_tables_refuse_editor_selection() {
        let mut doc = document("<Project><Symbols/><Symbols/></Project>");
        let before = doc.clone();
        assert!(doc.variable_mut().get(0).is_err());
        assert_eq!(doc, before);
    }
}
