//! Generate the FF and indexed-bit native acceptance case from XGKZEROS.
use std::{env, error::Error, fs::OpenOptions, io::Write};
use xgwx::{LadderCellEdit, LadderEditElement, LadderEditKind, XgwxDocument};

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: xgk_indexed_bit_acceptance XGKZEROS.xgwx NEW_OUTPUT.xgwx".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    let program = document.ladder_programs().remove(0)?;
    let stop = program
        .strings
        .iter()
        .find(|s| s.value == "STOP")
        .ok_or("STOP missing")?;
    document.update_ladder_cell_text(0, stop.offset, "STOP", "FF,M00030")?;
    for (y, operand, mnemonic, values) in [
        (0, "M00000", "B", ["D000100", "4"]),
        (4, "M00001", "BN", ["D000104", "D000108"]),
    ] {
        document.edit_ladder_cell(
            0,
            &LadderCellEdit {
                raw_y: y,
                column: 0,
                expected: Some(LadderEditElement {
                    kind: LadderEditKind::NormallyOpen,
                    operand: operand.into(),
                }),
                replacement: None,
            },
        )?;
        document.insert_ladder_comparison(0, y, 0, mnemonic, &values.map(String::from))?;
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?
        .write_all(&document.to_bytes()?)?;
    println!("wrote FF and B/BN native acceptance project");
    Ok(())
}
