//! Add a standalone IEC comment to an empty row of the smart-home project.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let (source, output, program_one) = match args.as_slice() {
        [source, output] => (source, output, false),
        [source, output, mode] if mode == "program1" => (source, output, true),
        _ => {
            return Err(
                "usage: iec_comment_acceptance <source.xgwx> <output.xgwx> [program1]".into(),
            );
        }
    };
    let mut document = XgwxDocument::from_path(source)?;
    if program_one {
        document
            .program_mut()
            .get(1)
            .and_then(|mut selected_program| selected_program.iec_ld()?.insert_blank_row(5))?;
        document
            .program_mut()
            .get(1)
            .and_then(|mut selected_program| {
                selected_program.iec_ld()?.insert_comment(6, "중간 설명")
            })?;
    } else {
        document
            .program_mut()
            .get(0)
            .and_then(|mut selected_program| {
                selected_program.iec_ld()?.insert_comment(5, "새 설명")
            })?;
    }
    fs::write(output, document.to_bytes()?)?;
    Ok(())
}
