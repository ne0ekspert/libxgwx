//! Convert supported ladder programs into the library's typed IL representation.
//!
//! Run: `cargo run --features il --example il -- path/to/project.xgwx`.
//! Reads the workspace and prints IL; it does not modify the input. Unsupported
//! ladder topology returns an error instead of guessing a conversion.
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
