//! Generate variable-length instruction edits for native XG5000 acceptance.
use std::{error::Error, path::Path};
use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn Error>> {
    let destination = std::env::args()
        .nth(1)
        .ok_or("usage: instruction-acceptance NEW_DIRECTORY")?;
    let path = Path::new(&destination);
    std::fs::create_dir(path)?;
    let source = XgwxDocument::from_path("fixtures/elements.xgwx")?;
    for (name, original, replacement) in [
        ("F05", "MOV,0,D000000", "ADD,1,2,D000000"),
        ("F06", "MOV,0,D000000", "TON,T0000,100"),
        ("F01", "MOV,0,D000000", "MOV,1,D1"),
        ("F02", "MOV,0,D000000", "MOV,12345,D000042"),
        ("F03", "MOV,0,D000000", "MOV,12345,D1"),
        (
            "F04",
            "XDST,1,1,7000,1000,100,0,0",
            "XDST,1,1,700,100,10,0,0",
        ),
    ] {
        let mut doc = source.clone();
        let data = doc.ladder_programs().remove(0)?;
        let offset = data
            .strings
            .iter()
            .find(|s| s.value == original)
            .ok_or("missing instruction")?
            .offset;
        doc.update_ladder_cell_text(0, offset, original, replacement)?;
        doc.write_to(path.join(format!("{name}.XGWX")))?;
    }
    println!("Generated F01 through F06 in {}", path.display());
    Ok(())
}
