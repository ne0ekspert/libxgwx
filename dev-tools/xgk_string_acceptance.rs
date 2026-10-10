//! Generate a private quoted-string instruction acceptance case.
use std::{env, error::Error, fs::OpenOptions, io::Write};
use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: xgk_string_acceptance SOURCE.xgwx NEW_OUTPUT.xgwx".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    for (original, replacement) in [
        ("STOP", "$MOVP,'Room B, off',D000200"),
        ("WDT", "$MOV,'Room A, on',D000100"),
    ] {
        let program = document.ladder_programs().remove(0)?;
        let offset = program
            .strings
            .iter()
            .find(|s| s.value == original)
            .ok_or("original instruction missing")?
            .offset;
        document.update_ladder_cell_text(0, offset, original, replacement)?;
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?
        .write_all(&document.to_bytes()?)?;
    Ok(())
}
