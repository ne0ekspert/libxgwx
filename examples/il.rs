use std::error::Error;
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1).expect("usage: il <file.xgwx>");
    let doc = XgwxDocument::from_path(path)?;

    for program in doc.ladder_programs() {
        println!("{}", program?.to_il()?);
    }

    Ok(())
}
