#![cfg(feature = "write")]
use xgwx::XgwxDocument;

#[test]
fn rename_project_checks_expected_name_and_preserves_every_other_byte() {
    let bytes = include_bytes!("../fixtures/empty-projects/new-xgk.xgwx");
    let mut doc = XgwxDocument::parse(bytes).unwrap();
    let before = doc.container().xml().to_owned();
    for (expected, name) in [
        ("stale", "Machine"),
        ("NewProject", "  "),
        ("NewProject", "bad\nname"),
    ] {
        assert!(doc.project_mut().rename_if(expected, name).is_err());
        assert_eq!(doc.container().xml(), before);
    }
    doc.project_mut()
        .rename_if("NewProject", "NewProject")
        .unwrap();
    assert_eq!(doc.to_verified_bytes().unwrap(), bytes);
    doc.project_mut()
        .rename_if("NewProject", "공장 & <Line 1>")
        .unwrap();
    assert_eq!(doc.project().name(), Some("공장 & <Line 1>"));
    assert_eq!(
        doc.container().xml(),
        before.replacen("NewProject", "공장 &amp; &lt;Line 1&gt;", 1)
    );
    let serialized = doc.to_verified_bytes().unwrap();
    assert_eq!(
        XgwxDocument::parse(&serialized).unwrap().project().name(),
        Some("공장 & <Line 1>")
    );
}
