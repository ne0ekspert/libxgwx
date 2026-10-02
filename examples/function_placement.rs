//! Generate function placements for native XG5000 acceptance.
use std::{env, error::Error};
use xgwx::{LadderCellEdit, LadderEditElement, LadderEditKind, XgwxDocument};
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() == 3 && args[2] == "operand-suite" {
        let mut doc = XgwxDocument::from_path(&args[0])?;
        let program = doc.ladder_programs().remove(0)?;
        let row = u16::from_le_bytes(program.data[4..6].try_into()?);
        for (i, (name, values)) in [
            ("MOV", vec!["65535", "D100"]),
            ("MOV", vec!["-32768", "D101"]),
            ("ADD", vec!["32767", "-32768", "D102"]),
            ("ADD", vec!["HFFFF", "1", "D103"]),
            ("RADD", vec!["3.4E38", "D104", "D106"]),
        ]
        .into_iter()
        .enumerate()
        {
            let y = u8::try_from((row + u16::try_from(i)?) * 4)?;
            doc.insert_ladder_instruction(
                0,
                y,
                name,
                &values.into_iter().map(String::from).collect::<Vec<_>>(),
            )?;
            doc.edit_ladder_cell(
                0,
                &LadderCellEdit {
                    raw_y: y,
                    column: 0,
                    expected: None,
                    replacement: Some(LadderEditElement {
                        kind: LadderEditKind::NormallyOpen,
                        operand: "M0000".into(),
                    }),
                },
            )?;
        }
        doc.write_to(&args[1])?;
        return Ok(());
    }
    if args.len() == 3 && args[2] == "element-suite" {
        let mut doc = XgwxDocument::from_path(&args[0])?;
        let program = doc.ladder_programs().remove(0)?;
        let row = u16::from_le_bytes(program.data[4..6].try_into()?);
        // Enter from right to left, then fill the gaps, exercising sorted insertion.
        doc.insert_iec_ld_single_element(0, row, 94, "coil", "OUTPUT", "%MX1300")?;
        for x in (1..94).step_by(3).rev() {
            doc.insert_iec_ld_single_element(
                0,
                row,
                x,
                "contact",
                "NO",
                &format!("%MX{}", 1200 + u16::from(x)),
            )?;
        }
        doc.write_to(&args[1])?;
        return Ok(());
    }
    if args.len() == 3 && args[2] == "comparison-suite" {
        let mut doc = XgwxDocument::from_path(&args[0])?;
        let program = doc
            .ladder_programs()
            .into_iter()
            .next()
            .ok_or("source contains no programs")??;
        let row_count = u16::from_le_bytes(
            program
                .data
                .get(4..6)
                .ok_or("missing row count")?
                .try_into()?,
        );
        for (index, spec) in xgwx::ladder_comparison_catalog().iter().enumerate() {
            let row = row_count
                .checked_add(u16::try_from(index)?)
                .ok_or("row coordinate overflow")?;
            let y = u8::try_from(row.checked_mul(4).ok_or("row coordinate overflow")?)?;
            doc.insert_ladder_comparison(0, y, 0, spec.mnemonic, &["D100".into(), "D102".into()])?;
            doc.insert_ladder_instruction(0, y, "MOV", &["0".into(), format!("D{}", 200 + index)])?;
        }
        doc.write_to(&args[1])?;
        return Ok(());
    }
    if args.len() == 3 && args[2] == "xgk-suite" {
        let mut doc = XgwxDocument::from_path(&args[0])?;
        let program = doc
            .ladder_programs()
            .into_iter()
            .next()
            .ok_or("source contains no programs")??;
        let row_count = u16::from_le_bytes(
            program
                .data
                .get(4..6)
                .ok_or("missing row count")?
                .try_into()?,
        );
        let y = u8::try_from(row_count.checked_mul(4).ok_or("row coordinate overflow")?)?;
        doc.insert_ladder_comparison(0, y, 0, "=", &["D100".into(), "D102".into()])?;
        doc.insert_ladder_instruction(0, y, "MOV", &["0".into(), "D104".into()])?;
        doc.insert_ladder_instruction(0, y + 4, "I2R", &["D100".into(), "D102".into()])?;
        doc.edit_ladder_cell(
            0,
            &LadderCellEdit {
                raw_y: y + 4,
                column: 0,
                expected: None,
                replacement: Some(LadderEditElement {
                    kind: LadderEditKind::NormallyOpen,
                    operand: "M11".into(),
                }),
            },
        )?;
        doc.insert_ladder_comparison(0, y + 8, 0, ">=", &["D100".into(), "D102".into()])?;
        doc.insert_ladder_instruction(0, y + 8, "MOV", &["1".into(), "D106".into()])?;
        doc.write_to(&args[1])?;
        return Ok(());
    }
    if args.len() == 3 && args[2] == "suite" {
        let mut doc = XgwxDocument::from_path(&args[0])?;
        let program = doc
            .ladder_programs()
            .into_iter()
            .next()
            .ok_or("source contains no programs")??;
        let mut row = u16::from_le_bytes(program.data[4..6].try_into()?);
        for (i, name) in [
            "MOVE", "ADD", "SUB", "MUL", "DIV", "EQ", "GT", "GE", "LT", "LE",
        ]
        .iter()
        .enumerate()
        {
            let operands = if *name == "MOVE" {
                vec!["1", "%MW100"]
            } else if i < 5 {
                vec!["%MW100", "1", "%MW102"]
            } else {
                vec!["%MW100", "1", "%MX1000"]
            };
            doc.insert_iec_ld_function(
                0,
                row,
                4 + (i as u8 % 6) * 3,
                name,
                &operands.into_iter().map(String::from).collect::<Vec<_>>(),
            )?;
            row += if *name == "MOVE" { 3 } else { 4 };
        }
        doc.write_to(&args[1])?;
        return Ok(());
    }
    let [source, output, kind, program, row, x, name, operands @ ..] = args.as_slice() else {
        return Err("usage: function_placement SOURCE OUTPUT iec|xgk|comparison PROGRAM ROW X NAME OPERANDS...".into());
    };
    let mut doc = XgwxDocument::from_path(source)?;
    match kind.as_str() {
        "iec" => {
            doc.insert_iec_ld_function(program.parse()?, row.parse()?, x.parse()?, name, operands)?
        }
        "xgk" => doc.insert_ladder_instruction(program.parse()?, row.parse()?, name, operands)?,
        "comparison" => doc.insert_ladder_comparison(
            program.parse()?,
            row.parse()?,
            x.parse()?,
            name,
            operands,
        )?,
        _ => return Err("unknown placement kind".into()),
    }
    doc.write_to(output)?;
    Ok(())
}
