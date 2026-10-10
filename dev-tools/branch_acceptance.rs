//! Generate mixed-program and branch-construction native acceptance cases.
use std::{error::Error, path::Path};
use xgwx::{LadderBranchEdit, LadderCellEdit, LadderEditElement, LadderEditKind, XgwxDocument};
fn element(kind: LadderEditKind, operand: &str) -> Option<LadderEditElement> {
    Some(LadderEditElement {
        kind,
        operand: operand.into(),
    })
}
fn main() -> Result<(), Box<dyn Error>> {
    let destination = std::env::args()
        .nth(1)
        .ok_or("usage: branch-acceptance NEW_DIRECTORY")?;
    let path = Path::new(&destination);
    std::fs::create_dir(path)?;
    let source = XgwxDocument::from_path("fixtures/elements.xgwx")?;
    source.write_to(path.join("E00.XGWX"))?;
    for (name, raw_y, column, expected, replacement) in [
        (
            "E01",
            12,
            0,
            element(LadderEditKind::NormallyOpen, "P00001"),
            element(LadderEditKind::NormallyClosed, "M42"),
        ),
        (
            "E02",
            12,
            0,
            element(LadderEditKind::NormallyOpen, "P00001"),
            None,
        ),
        (
            "E03",
            20,
            1,
            None,
            element(LadderEditKind::NormallyOpen, "M00002"),
        ),
        (
            "E04",
            20,
            9,
            element(LadderEditKind::Set, "M00020"),
            element(LadderEditKind::Reset, "M00020"),
        ),
        (
            "E08",
            4,
            0,
            element(LadderEditKind::NormallyOpen, "M00000"),
            element(LadderEditKind::NormallyClosed, "M42"),
        ),
    ] {
        let mut document = source.clone();
        document.edit_ladder_cell(
            0,
            &LadderCellEdit {
                raw_y,
                column,
                expected,
                replacement,
            },
        )?;
        document.write_to(path.join(format!("{name}.XGWX")))?;
    }
    let mut branch = XgwxDocument::from_path("fixtures/ladder-edit/linear.xgwx")?;
    branch.insert_ladder_row(0, 4)?;
    branch.edit_ladder_branch(
        0,
        &LadderBranchEdit {
            raw_y: 0,
            boundary: 1,
            expected: false,
            present: true,
        },
    )?;
    branch.edit_ladder_cell(
        0,
        &LadderCellEdit {
            raw_y: 4,
            column: 0,
            expected: None,
            replacement: element(LadderEditKind::NormallyOpen, "M00002"),
        },
    )?;
    branch.write_to(path.join("E05.XGWX"))?;
    let mut removed = branch.clone();
    removed.edit_ladder_branch(
        0,
        &LadderBranchEdit {
            raw_y: 0,
            boundary: 1,
            expected: true,
            present: false,
        },
    )?;
    removed.write_to(path.join("E06.XGWX"))?;
    branch.insert_ladder_row(0, 4)?;
    branch.write_to(path.join("E07.XGWX"))?;
    println!("Generated E00 through E08 in {}", path.display());
    Ok(())
}
