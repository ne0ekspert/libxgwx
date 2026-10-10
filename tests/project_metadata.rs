#![cfg(feature = "write")]
use xgwx::XgwxDocument;

#[test]
fn rename_project_checks_expected_name_and_preserves_every_other_byte() {
    let bytes = include_bytes!("../fixtures/empty-projects/new-xgk.xgwx");
    let mut doc = XgwxDocument::parse(bytes).unwrap();
    let before = doc.xml.clone();
    for (expected, name) in [("stale", "Machine"), ("NewProject", "  "), ("NewProject", "bad\nname")] {
        assert!(doc.rename_project(expected, name).is_err());
        assert_eq!(doc.xml, before);
    }
    doc.rename_project("NewProject", "NewProject").unwrap();
    assert_eq!(doc.to_verified_bytes().unwrap(), bytes);
    doc.rename_project("NewProject", "공장 & <Line 1>").unwrap();
    assert_eq!(doc.project_name(), Some("공장 & <Line 1>"));
    assert_eq!(doc.xml, before.replacen("NewProject", "공장 &amp; &lt;Line 1&gt;", 1));
    let serialized = doc.to_verified_bytes().unwrap();
    assert_eq!(XgwxDocument::parse(&serialized).unwrap().project_name(), Some("공장 & <Line 1>"));
}
