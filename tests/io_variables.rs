#![cfg(feature = "write")]
use xgwx::{VariablePatch, XgwxDocument};
fn empty() -> XgwxDocument {
    XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgk.xgwx")).unwrap()
}
#[test]
fn generation_matches_native_save_as_symbols_and_preserves_programs() {
    use base64::Engine;
    use std::io::Read;
    fn symbols(doc: &XgwxDocument) -> Vec<u8> {
        let xml = roxmltree::Document::parse(&doc.xml).unwrap();
        let node = xml
            .descendants()
            .find(|n| n.has_tag_name("Symbols"))
            .unwrap();
        let encoded: String = node
            .text()
            .unwrap()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        let mut decoded = Vec::new();
        bzip2::read::BzDecoder::new(bytes.as_slice())
            .read_to_end(&mut decoded)
            .unwrap();
        decoded
    }
    let mut doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/io-variable-generation/base.xgwx"
    ))
    .unwrap();
    let original_program = doc.ladder_program(0).unwrap().unwrap().data;
    doc.generate_io_variables().unwrap();
    let native = XgwxDocument::parse(include_bytes!(
        "../fixtures/io-variable-generation/resaved-native.xgwx"
    ))
    .unwrap();
    assert_eq!(doc.variables().unwrap().len(), 120);
    assert_eq!(symbols(&doc), symbols(&native));
    assert_eq!(
        doc.ladder_program(0).unwrap().unwrap().data,
        original_program
    );
    assert_eq!(
        native.ladder_program(0).unwrap().unwrap().data,
        original_program
    );
}
#[test]
fn generate_preview_duplicate_overwrite_and_undo_bytes() {
    let mut doc = empty();
    doc.insert_module(0, 0, "XGI-D21A").unwrap();
    doc.insert_module(0, 2, "XGQ-TR4A/B").unwrap();
    let before = doc.to_bytes().unwrap();
    let plan = doc.preview_io_variables().unwrap();
    assert_eq!(plan.len(), 40);
    assert_eq!(plan[0].address, "P00000");
    assert_eq!(plan[8].address, "P00020");
    assert!(plan.iter().all(|r| r.action == "create"));
    doc.generate_io_variables().unwrap();
    assert_eq!(doc.variables().unwrap().len(), 40);
    let generated = doc.to_bytes().unwrap();
    assert!(doc
        .preview_io_variables()
        .unwrap()
        .iter()
        .all(|r| r.action == "unchanged"));
    doc.generate_io_variables().unwrap();
    assert_eq!(doc.to_bytes().unwrap(), generated);
    doc.update_variable(
        0,
        &VariablePatch {
            name: Some("CustomInput".into()),
            description: Some("Changed".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let rows = doc.preview_io_variables().unwrap();
    assert_eq!(rows[0].action, "overwrite");
    assert_eq!(rows[0].existing_names, ["CustomInput"]);
    doc.generate_io_variables().unwrap();
    assert_eq!(
        doc.variables().unwrap()[0].name.as_deref(),
        Some("_0000_IN00")
    );
    assert_eq!(
        doc.ladder_program(0).unwrap().unwrap().data,
        XgwxDocument::parse(&before)
            .unwrap()
            .ladder_program(0)
            .unwrap()
            .unwrap()
            .data
    );
    assert_eq!(doc.to_bytes().unwrap(), generated);
}
#[test]
fn native_generated_digital_symbols_are_already_exact() {
    let mut doc = XgwxDocument::parse(include_bytes!("../fixtures/elements-io.xgwx")).unwrap();
    let before = doc.to_bytes().unwrap();
    let rows = doc.preview_io_variables().unwrap();
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|r| r.action == "unchanged"));
    let native = doc.variables().unwrap();
    for row in &rows {
        let variable = native
            .iter()
            .find(|v| v.name.as_deref() == Some(&row.name))
            .unwrap();
        assert_eq!(row.address, variable.address.as_deref().unwrap());
    }
    doc.generate_io_variables().unwrap();
    assert_eq!(doc.to_bytes().unwrap(), before);
}
#[test]
fn unsupported_cpu_and_allocation_fail_without_mutation() {
    let mut doc =
        XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgi.xgwx")).unwrap();
    let before = doc.to_bytes().unwrap();
    assert!(doc.generate_io_variables().is_err());
    assert_eq!(doc.to_bytes().unwrap(), before);
}

#[test]
fn matching_source_overwrites_previous_mapping_and_preserves_other_symbols() {
    let mut doc = XgwxDocument::parse(include_bytes!("../fixtures/elements-io.xgwx")).unwrap();
    let before = doc.variables().unwrap();
    // A matching I/O source is regenerated even if its name/address changed.
    doc.update_variable(
        0,
        &VariablePatch {
            name: Some("UserMemory".into()),
            address_area: Some("M".into()),
            address_number: Some(100),
            ..Default::default()
        },
    )
    .unwrap();
    // Its source still matches the I/O point, so generation overwrites it.
    assert_eq!(doc.preview_io_variables().unwrap()[0].action, "overwrite");
    doc.generate_io_variables().unwrap();
    let after = doc.variables().unwrap();
    assert_eq!(after.len(), before.len());
    let mismatches = before
        .iter()
        .filter(|v| after.iter().find(|a| a.name == v.name) != Some(*v))
        .map(|v| v.name.as_deref().unwrap_or("?"))
        .collect::<Vec<_>>();
    assert!(mismatches.is_empty(), "changed variables: {mismatches:?}");
}

#[test]
fn preview_uses_decimal_word_and_hexadecimal_bit_addresses_with_native_binary_metadata() {
    let mut doc = empty();
    doc.insert_module(1, 0, "XGI-D22A/B").unwrap();
    let rows = doc.preview_io_variables().unwrap();
    assert_eq!(rows[0].address, "P00120");
    assert_eq!(rows[15].address, "P0012F");
    doc.generate_io_variables().unwrap();
    let xml = roxmltree::Document::parse(&doc.xml).unwrap();
    let symbols = xml
        .descendants()
        .find(|n| n.has_tag_name("Symbols"))
        .unwrap();
    assert_eq!(symbols.attribute("Compressed"), Some("1"));
    assert_eq!(
        symbols.attribute(("urn:schemas-microsoft-com:datatypes", "dt")),
        Some("bin.base64")
    );
}
