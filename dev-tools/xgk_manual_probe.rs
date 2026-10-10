//! Extract a private native XGK instruction capture for record-level comparison.
use std::{env, error::Error, fs::OpenOptions, io::Write};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: xgk_manual_probe SOURCE.xgwx NEW_PAYLOAD.bin".into());
    };
    let document = XgwxDocument::from_path(source)?;
    let program = document
        .ladder_programs()
        .into_iter()
        .next()
        .ok_or("no program")??;
    println!(
        "{} bytes; {} rungs; {} unknown records",
        program.data.len(),
        program.structure.rungs.len(),
        program.structure.unknown_records.len()
    );
    for element in &program.elements {
        println!("{element:?}");
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?
        .write_all(&program.data)?;
    Ok(())
}
