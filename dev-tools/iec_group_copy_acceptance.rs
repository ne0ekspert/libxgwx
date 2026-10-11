//! Generate complete-network copy candidates for native XG5000 validation.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output, kind] = args.as_slice() else {
        return Err("usage: iec_group_copy_acceptance <source> <output> simple|function".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    match kind.as_str() {
        "simple" => document
            .program_mut()
            .get(0)
            .and_then(|mut selected_program| selected_program.iec_ld()?.copy_group(4, 6, 5))?,
        "function" => {
            document
                .program_mut()
                .get(0)
                .and_then(|mut selected_program| selected_program.iec_ld()?.delete_group(33, 67))?;
            document
                .program_mut()
                .get(0)
                .and_then(|mut selected_program| {
                    selected_program.iec_ld()?.copy_group(14, 20, 67)
                })?;
        }
        _ => return Err("kind must be simple or function".into()),
    }
    fs::write(output, document.to_bytes()?)?;
    Ok(())
}
