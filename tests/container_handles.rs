use flate2::{Compression, write::GzEncoder};
use std::io::Write;
use xgwx::XgwxDocument;

fn document() -> (XgwxDocument, Vec<u8>) {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(b"<Project>Factory<Unknown Flag=\"yes\"/></Project>")
        .unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    bytes.extend(b"opaque trailer");
    (XgwxDocument::parse(&bytes).unwrap(), bytes)
}

#[test]
fn borrowed_container_retains_unknown_data() {
    let (doc, bytes) = document();
    let view = doc.container();
    assert_eq!(view.header().raw, b"XG");
    assert_eq!(view.trailer(), b"opaque trailer");
    assert_eq!(
        view.main_gzip(),
        &bytes[2..bytes.len() - view.trailer().len()]
    );
    assert!(view.trailer_gzip_members().is_empty());
    assert_eq!(view.root().children[0].attribute("Flag"), Some("yes"));
    assert!(view.properties().is_empty());
    assert!(view.decoded_payloads().is_empty());
}

#[cfg(feature = "write")]
#[test]
fn unchanged_container_emits_original_bytes() {
    let (doc, bytes) = document();
    assert_eq!(doc.to_bytes().unwrap(), bytes);
}

#[cfg(feature = "write")]
#[test]
fn replacement_is_atomic_and_updates_domain_views() {
    let mut doc = xgwx::create_project("XGK-CPUSN", "LD").unwrap();
    let original = doc.clone();
    assert!(doc.container_mut().replace_xml("<Project>").is_err());
    assert_eq!(doc, original);

    let xml = doc
        .container()
        .xml()
        .replacen("NewProject", "Factory", 1)
        .replace("</Project>", "<Unknown Flag=\"yes\"/></Project>");
    doc.container_mut().replace_xml(xml.clone()).unwrap();
    assert_eq!(doc.container().xml(), xml);
    assert_eq!(doc.project().name(), Some("Factory"));
    assert!(
        doc.container()
            .root()
            .descendants_named("Unknown")
            .next()
            .is_some()
    );
    assert_eq!(doc.container().header(), original.container().header());
    assert_eq!(
        doc.container().main_gzip(),
        original.container().main_gzip()
    );
    assert_eq!(doc.container().trailer(), original.container().trailer());
    doc.project_mut().rename("Renamed").unwrap();
    let saved = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
    assert_eq!(saved.project().name(), Some("Renamed"));
    assert!(saved.container().xml().contains("<Unknown Flag=\"yes\"/>"));
    assert_eq!(saved.container().root(), doc.container().root());
}
