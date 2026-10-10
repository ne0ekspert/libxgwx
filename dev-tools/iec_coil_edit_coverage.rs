//! Preflight every original addressed coil independently; emit representative candidates.
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
    let baseline = source
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let mut total = 0;
    let mut deleted = 0;
    let mut refilled = 0;
    for (p, program) in source.ladder_programs().into_iter().enumerate() {
        let program = program?;
        for r in program
            .iec_record_frames()
            .unwrap()
            .into_iter()
            .filter(|r| matches!(r.kind, K::Coil(14..=19)))
        {
            if selected
                .as_ref()
                .is_some_and(|sites| !sites.contains(&(p, r.row_index, program.data[r.offset + 5])))
            {
                continue;
            }
            total += 1;
            let K::Coil(code) = r.kind else {
                unreachable!()
            };
            let kind =
                ["OUTPUT", "INVERSE", "SET", "RESET", "RISING", "FALLING"][usize::from(code - 14)];
            let x = program.data[r.offset + 5];
            let variable = String::from_utf16(
                &program.data[r.offset + 19..r.end]
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    .collect::<Vec<_>>(),
            )?;
            let mut edit = source.clone();
            if let Err(e) = edit.delete_iec_ld_terminal_coil(p, r.offset, &variable) {
                println!("DELETE GUARDED p{p} L{} x{x} {e}", r.row_index);
                continue;
            }
            deleted += 1;
            let candidate = edit.to_bytes()?;
            if let Some(out) = &output {
                if selected.is_some()
                    || [
                        (6, 24, 94),
                        (3, 64, 94),
                        (0, 2, 94),
                        (0, 12, 94),
                        (1, 1, 94),
                        (2, 6, 94),
                    ]
                    .contains(&(p, r.row_index, x))
                {
                    fs::write(
                        out.join(format!("OD{p}R{}X{x}.xgwx", r.row_index)),
                        &candidate,
                    )?;
                }
            }
            if let Err(e) =
                edit.insert_iec_ld_single_element(p, r.row_index, x, "coil", kind, &variable)
            {
                println!(
                    "REFILL GUARDED p{p} L{} x{x} operand {variable:?} {e}",
                    r.row_index
                );
                continue;
            }
            let restored_programs = edit
                .ladder_programs()
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;
            for (index, (actual, original)) in restored_programs.iter().zip(&baseline).enumerate() {
                assert_eq!(
                    actual.data, original.data,
                    "refill changes program {index} after p{p} L{}",
                    r.row_index
                );
            }
            let restored = &restored_programs[p];
            let expected = program.data.clone();
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
                    || [
                        (6, 24, 94),
                        (3, 64, 94),
                        (0, 2, 94),
                        (0, 12, 94),
                        (1, 1, 94),
                        (2, 6, 94),
                    ]
                    .contains(&(p, r.row_index, x))
                {
                    fs::write(
                        out.join(format!("OR{p}R{}X{x}.xgwx", r.row_index)),
                        edit.to_bytes()?,
                    )?;
                }
            }
        }
    }
    println!("coils {total}; Delete preflight {deleted}; exact refill {refilled}");
    Ok(())
}
