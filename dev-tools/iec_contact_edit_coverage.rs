//! Preflight every original addressed contact independently; emit representative candidates.
use std::{env, error::Error, fs, path::PathBuf};
use xgwx::{IecRecordKind as K, XgwxDocument};
fn main() -> Result<(), Box<dyn Error>> {
    let source = XgwxDocument::from_path(env::args().nth(1).ok_or("source")?)?;
    let output = env::args().nth(2).map(PathBuf::from);
    if let Some(p) = &output {
        fs::create_dir_all(p)?;
    }
    let filter = env::args().nth(3);
    let selected = filter.as_ref().map(|text| {
        text.split(';')
            .map(|entry| {
                let numbers = entry
                    .split(',')
                    .map(|n| n.parse::<usize>().unwrap())
                    .collect::<Vec<_>>();
                (numbers[0], numbers[1] as u16, numbers[2] as u8)
            })
            .collect::<Vec<_>>()
    });
    let mut total = 0;
    let mut deleted = 0;
    let mut refilled = 0;
    for (p, program) in source.ladder_programs().into_iter().enumerate() {
        let program = program?;
        for r in program
            .iec_record_frames()
            .unwrap()
            .into_iter()
            .filter(|r| matches!(r.kind, K::Contact(6..=11)))
        {
            if selected
                .as_ref()
                .is_some_and(|sites| !sites.contains(&(p, r.row_index, program.data[r.offset + 5])))
            {
                continue;
            }
            total += 1;
            let K::Contact(code) = r.kind else {
                unreachable!()
            };
            let kind = [
                "NO",
                "NC",
                "RISING",
                "FALLING",
                "NEGATED_RISING",
                "NEGATED_FALLING",
            ][usize::from(code - 6)];
            let x = program.data[r.offset + 5];
            let variable = String::from_utf16(
                &program.data[r.offset + 19..r.end]
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    .collect::<Vec<_>>(),
            )?;
            let mut edit = source.clone();
            if let Err(e) = edit.delete_iec_ld_contact(p, r.offset, x, kind, &variable) {
                println!("DELETE GUARDED p{p} L{} x{x} {e}", r.row_index);
                continue;
            }
            deleted += 1;
            let candidate = edit.to_bytes()?;
            if let Some(out) = &output {
                if selected.is_some()
                    || [(6, 80, 7), (6, 81, 7), (6, 85, 1), (0, 3, 1), (2, 40, 13)].contains(&(
                        p,
                        r.row_index,
                        x,
                    ))
                {
                    fs::write(
                        out.join(format!("CD{p}R{}X{x}.xgwx", r.row_index)),
                        &candidate,
                    )?;
                }
            }
            if let Err(e) =
                edit.insert_iec_ld_single_element(p, r.row_index, x, "contact", kind, &variable)
            {
                println!(
                    "REFILL GUARDED p{p} L{} x{x} operand {variable:?} {e}",
                    r.row_index
                );
                continue;
            }
            let restored = edit.ladder_programs().remove(p)?;
            // Contact record order and all native fields must recover exactly;
            // the previously validated leading mesh edit refreshes six heights.
            let mut expected = program.data.clone();
            if p == 6 && r.row_index == 80 && x == 1 {
                for row in program
                    .iec_row_frames()
                    .unwrap()
                    .iter()
                    .filter(|row| row.group_index == r.group_index)
                {
                    expected[row.start + 17..row.start + 21].copy_from_slice(&39u32.to_le_bytes());
                }
            }
            if restored.data != expected {
                println!(
                    "REFILL DIFFERENT p{p} L{} x{x}, first {:?}",
                    r.row_index,
                    restored
                        .data
                        .iter()
                        .zip(&expected)
                        .position(|(a, b)| a != b)
                );
                continue;
            }
            refilled += 1;
            if let Some(out) = &output {
                if selected.is_some()
                    || [(6, 80, 7), (6, 81, 7), (6, 85, 1), (0, 3, 1), (2, 40, 13)].contains(&(
                        p,
                        r.row_index,
                        x,
                    ))
                {
                    fs::write(
                        out.join(format!("CR{p}R{}X{x}.xgwx", r.row_index)),
                        edit.to_bytes()?,
                    )?;
                }
            }
        }
    }
    println!("contacts {total}; Delete preflight {deleted}; exact refill {refilled}");
    Ok(())
}
