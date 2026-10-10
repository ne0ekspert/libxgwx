use xgwx::{LadderCellEdit, LadderEditElement, LadderEditKind, XgwxDocument};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/text-cpu/generated".into());
    std::fs::create_dir_all(&directory)?;
    let source = XgwxDocument::from_path("fixtures/text-programs/generated.xgwx")?;
    for (model, stem) in [
        ("XGI-CPUE", "TCPUE"),
        ("XGI-CPUS", "TCPUS"),
        ("XGI-CPUH", "TCPUH"),
        ("XGI-CPUU", "TCPUU"),
        ("XGI-CPUU/D", "TCPUD"),
        ("XGI-CPUUN", "TCPUN"),
    ] {
        let mut doc = source.clone();
        doc.select_cpu(model)?;
        assert_eq!(doc.text_programs(), source.text_programs());
        std::fs::write(format!("{directory}/{stem}.xgwx"), doc.to_verified_bytes()?)?;
    }
    let mut xgk = XgwxDocument::from_path("fixtures/empty-projects/new-xgk.xgwx")?;
    for (row, contact, output, base) in [
        (0, LadderEditKind::NormallyOpen, LadderEditKind::Output, 0),
        (
            1,
            LadderEditKind::AddressedRisingPulse,
            LadderEditKind::RisingPulseOutput,
            3,
        ),
        (
            2,
            LadderEditKind::AddressedFallingPulse,
            LadderEditKind::Reset,
            5,
        ),
        (3, LadderEditKind::NormallyOpen, LadderEditKind::Set, 7),
    ] {
        let row = row * 4;
        if row > 0 {
            xgk.insert_ladder_row(0, row)?;
        }
        xgk.edit_ladder_cell(
            0,
            &LadderCellEdit {
                raw_y: row,
                column: 0,
                expected: None,
                replacement: Some(LadderEditElement {
                    kind: contact,
                    operand: format!("M{base:05}"),
                }),
            },
        )?;
        if row == 0 {
            xgk.edit_ladder_cell(
                0,
                &LadderCellEdit {
                    raw_y: row,
                    column: 1,
                    expected: None,
                    replacement: Some(LadderEditElement {
                        kind: LadderEditKind::NormallyClosed,
                        operand: "M00001".into(),
                    }),
                },
            )?;
        }
        xgk.edit_ladder_cell(
            0,
            &LadderCellEdit {
                raw_y: row,
                column: 9,
                expected: None,
                replacement: Some(LadderEditElement {
                    kind: output,
                    operand: format!("M{:05}", if row == 0 { 2 } else { base + 1 }),
                }),
            },
        )?;
    }
    std::fs::write(format!("{directory}/GKIL.xgwx"), xgk.to_verified_bytes()?)?;
    let il = xgk.ladder_programs().remove(0)?.to_il()?.to_string();
    std::fs::write(format!("{directory}/GKIL.txt"), &il)?;
    println!("XGK-CPUSN IL:\n{il}");
    Ok(())
}
