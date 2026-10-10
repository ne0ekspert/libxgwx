//! Synthetic sparse XGK projects for native XG5000 canvas boundary validation.
use xgwx::{LadderCellEdit, LadderEditElement, LadderEditKind, XgwxDocument};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let destination = std::env::args()
        .nth(1)
        .ok_or("destination directory required")?;
    std::fs::create_dir_all(&destination)?;
    for (name, last) in [("XGK256", 256), ("XGKMAX", 65534)] {
        let mut doc =
            XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgk.xgwx"))?;
        for (row, input, output) in [(0, "M00000", "M00001"), (last, "M00002", "M00003")] {
            for (column, kind, operand) in [
                (0, LadderEditKind::NormallyOpen, input),
                (9, LadderEditKind::Output, output),
            ] {
                doc.edit_ladder_cell(
                    0,
                    &LadderCellEdit {
                        raw_y: row * 4,
                        column,
                        expected: None,
                        replacement: Some(LadderEditElement {
                            kind,
                            operand: operand.into(),
                        }),
                    },
                )?;
            }
        }
        std::fs::write(
            std::path::Path::new(&destination).join(format!("{name}.xgwx")),
            doc.to_verified_bytes()?,
        )?;
    }
    Ok(())
}
