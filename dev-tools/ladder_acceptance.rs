//! Generate native LD acceptance cases from the captured linear program.
use std::{error::Error, path::Path};
use xgwx::{LadderCellEdit, LadderEditElement, LadderEditKind, XgwxDocument};
fn element(kind: LadderEditKind, operand: &str) -> Option<LadderEditElement> {
    Some(LadderEditElement {
        kind,
        operand: operand.into(),
    })
}
fn main() -> Result<(), Box<dyn Error>> {
    let directory = std::env::args()
        .nth(1)
        .ok_or("usage: ladder-acceptance NEW_DIRECTORY")?;
    let path = Path::new(&directory);
    std::fs::create_dir(path)?;
    let source = XgwxDocument::from_path("fixtures/ladder-edit/linear.xgwx")?;
    source.write_to(path.join("E00.XGWX"))?;
    for (name, column, expected, replacement) in [
        (
            "E01",
            0,
            element(LadderEditKind::NormallyOpen, "M00000"),
            element(LadderEditKind::NormallyClosed, "M42"),
        ),
        (
            "E02",
            2,
            None,
            element(LadderEditKind::NormallyOpen, "M00002"),
        ),
        (
            "E03",
            1,
            element(LadderEditKind::NormallyClosed, "M00001"),
            None,
        ),
        (
            "E04",
            9,
            element(LadderEditKind::Output, "M00010"),
            element(LadderEditKind::Set, "M00010"),
        ),
    ] {
        let mut doc = source.clone();
        doc.edit_ladder_cell(
            0,
            &LadderCellEdit {
                raw_y: 0,
                column,
                expected,
                replacement,
            },
        )?;
        doc.write_to(path.join(format!("{name}.XGWX")))?;
        println!("{name}: {}", doc.ladder_programs().remove(0)?.decoded_len);
    }
    Ok(())
}
