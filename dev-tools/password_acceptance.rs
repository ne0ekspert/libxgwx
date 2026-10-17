//! Generate password fixtures for native XG5000 acceptance.
use xgwx::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dest =
        std::path::PathBuf::from(std::env::args().nth(1).ok_or("output directory required")?);
    std::fs::create_dir_all(&dest)?;
    if std::env::args().nth(2).as_deref() == Some("iec") {
        let mut doc = create_project("XGI-CPUE", "LD")?;
        doc.project_mut().rename("IECPasswords")?;
        doc.program_mut().get(0)?.rename("MainLD")?;
        {
            let mut program = doc.program_mut().get(0)?;
            let mut ladder = program.iec_ld()?;
            ladder.insert_single_element(0, 1, "contact", "NO", "%MX0")?;
            ladder.insert_single_element(0, 94, "coil", "OUTPUT", "%MX1")?;
        }
        for (i, language) in ["ST", "IL", "SFC"].into_iter().enumerate() {
            doc.program_mut().create(&NewProgram {
                name: format!("Main{language}"),
                language: language.into(),
                object_id: format!("ea00000{i}-1111-4222-8333-444444444444"),
                symbol_id: format!("eb00000{i}-1111-4222-8333-444444444444"),
            })?;
            if language != "SFC" {
                let mut program = doc.program_mut().get(i + 1)?;
                let mut text = program.text()?;
                let expected_source = text.source()?.ok_or("missing source")?;
                text.update(&TextSourcePatch {
                    expected_source,
                    source: if language == "ST" {
                        "%MX1 := %MX0;\r\n".into()
                    } else {
                        "LD %MX0\r\nST %MX1\r\n".into()
                    },
                })?;
            }
        }
        for i in 0..4 {
            doc.program_mut()
                .get(i)?
                .set_password(None, Some("Ab1! x"))?;
        }
        let bytes = doc.to_verified_bytes()?;
        std::fs::write(dest.join("SETIEC.xgwx"), &bytes)?;
        let mut doc = XgwxDocument::parse(&bytes)?;
        for i in 0..4 {
            doc.program_mut()
                .get(i)?
                .set_password(Some("Ab1! x"), Some("12345678"))?;
        }
        let bytes = doc.to_verified_bytes()?;
        std::fs::write(dest.join("CHGIEC.xgwx"), &bytes)?;
        let mut doc = XgwxDocument::parse(&bytes)?;
        for i in 0..4 {
            doc.program_mut()
                .get(i)?
                .set_password(Some("12345678"), None)?;
        }
        std::fs::write(dest.join("REMIEC.xgwx"), doc.to_verified_bytes()?)?;
        return Ok(());
    }
    if std::env::args().nth(2).as_deref() == Some("iec-plain") {
        for language in ["LD", "ST", "IL", "SFC"] {
            let mut doc = create_project("XGI-CPUE", language)?;
            doc.project_mut().rename(&format!("Password{language}"))?;
            if language == "LD" {
                let mut program = doc.program_mut().get(0)?;
                let mut ladder = program.iec_ld()?;
                ladder.insert_single_element(0, 1, "contact", "NO", "%MX0")?;
                ladder.insert_single_element(0, 94, "coil", "OUTPUT", "%MX1")?;
            } else if language == "ST" || language == "IL" {
                let mut program = doc.program_mut().get(0)?;
                let mut text = program.text()?;
                let expected_source = text.source()?.ok_or("missing source")?;
                text.update(&TextSourcePatch {
                    expected_source,
                    source: if language == "ST" {
                        "%MX1 := %MX0;\r\n".into()
                    } else {
                        "LD %MX0\r\nST %MX1\r\n".into()
                    },
                })?;
            }
            std::fs::write(
                dest.join(format!("{language}PLAIN.xgwx")),
                doc.to_verified_bytes()?,
            )?;
        }
        return Ok(());
    }
    let mut doc = XgwxDocument::parse(include_bytes!("../fixtures/passwords/xgk-plain.xgwx"))?;
    for (column, kind, operand) in [
        (0, LadderEditKind::NormallyOpen, "M00000"),
        (9, LadderEditKind::Output, "M00001"),
    ] {
        doc.program_mut()
            .get(0)?
            .ladder()?
            .edit_cell(&LadderCellEdit {
                raw_y: 0,
                column,
                expected: None,
                replacement: Some(LadderEditElement {
                    kind,
                    operand: operand.into(),
                }),
            })?;
    }
    doc.project_mut().rename("GeneratedPassword")?;
    doc.project_mut().set_password(None, Some("Ab1! x"))?;
    doc.program_mut()
        .get(0)?
        .set_password(None, Some("12345678"))?;
    std::fs::write(dest.join("BOTH.xgwx"), doc.to_verified_bytes()?)?;
    let mut reopened = XgwxDocument::parse_with_password(&doc.to_verified_bytes()?, "Ab1! x")?;
    reopened.unlock_program(0, "12345678")?;
    reopened
        .program_mut()
        .get(0)?
        .ladder()?
        .edit_cell(&LadderCellEdit {
            raw_y: 0,
            column: 0,
            expected: Some(LadderEditElement {
                kind: LadderEditKind::NormallyOpen,
                operand: "M00000".into(),
            }),
            replacement: Some(LadderEditElement {
                kind: LadderEditKind::NormallyOpen,
                operand: "M00002".into(),
            }),
        })?;
    std::fs::write(dest.join("EDITED.xgwx"), reopened.to_verified_bytes()?)?;
    reopened.project_mut().set_password(Some("Ab1! x"), None)?;
    reopened
        .program_mut()
        .get(0)?
        .set_password(Some("12345678"), None)?;
    std::fs::write(dest.join("REMOVED.xgwx"), reopened.to_verified_bytes()?)?;
    Ok(())
}
