use xgwx::{NewProgram, XgwxDocument};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or("target/program-create".into());
    std::fs::create_dir_all(&out)?;
    for (file, language, stem) in [
        ("fixtures/empty-projects/new-xgk.xgwx", "LD", "NEWGK"),
        ("fixtures/sfc/native-loop.xgwx", "LD", "NEWGI"),
        ("fixtures/sfc/native-loop.xgwx", "SFC", "NEWSFC"),
    ] {
        let mut doc = XgwxDocument::from_path(file)?;
        doc.create_program(&NewProgram {
            name: "AddedProgram".into(),
            language: language.into(),
            object_id: "abcdef01-1234-4567-89ab-0123456789ab".into(),
            symbol_id: "abcdef02-1234-4567-89ab-0123456789ab".into(),
        })?;
        std::fs::write(format!("{out}/{stem}.xgwx"), doc.to_verified_bytes()?)?;
    }
    Ok(())
}
