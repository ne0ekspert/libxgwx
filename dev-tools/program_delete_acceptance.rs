use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or("target/program-delete".into());
    std::fs::create_dir_all(&out)?;
    for (stem, file) in [
        ("DELGK", "xgk-ld"),
        ("DELGI", "xgi-ld"),
        ("DELSFC", "xgi-sfc"),
    ] {
        let mut doc =
            XgwxDocument::from_path(format!("fixtures/program-create/{file}-roundtrip.xgwx"))?;
        let parsed = roxmltree::Document::parse(&doc.xml)?;
        let identity = parsed
            .descendants()
            .filter(|n| n.has_tag_name("Program"))
            .nth(1)
            .unwrap()
            .attribute("ObjectId")
            .unwrap()
            .to_owned();
        doc.delete_program(1, &identity)?;
        std::fs::write(format!("{out}/{stem}.xgwx"), doc.to_verified_bytes()?)?;
    }
    Ok(())
}
