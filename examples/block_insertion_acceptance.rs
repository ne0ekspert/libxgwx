//! Inspect and compare native function-block insertion captures.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let source = args
        .first()
        .ok_or("usage: block_insertion_acceptance SOURCE [PROGRAM [DUMP [FIXTURE_DIRECTORY]]]")?;
    if args.len() > 4 {
        return Err("too many arguments".into());
    }
    let document = XgwxDocument::from_path(source)?;
    let index = args
        .get(1)
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(0);
    let program = document
        .ladder_programs()
        .into_iter()
        .nth(index)
        .ok_or("program index out of range")??;
    if let Some(path) = args.get(2) {
        fs::write(path, &program.data)?;
    }
    println!(
        "rows {} groups {}",
        u16::from_le_bytes(
            program
                .data
                .get(4..6)
                .ok_or("missing row count")?
                .try_into()?
        ),
        u16::from_le_bytes(
            program
                .data
                .get(6..8)
                .ok_or("missing group count")?
                .try_into()?
        )
    );
    for instruction in &program.instructions {
        let start = instruction.offset.saturating_sub(19);
        println!(
            "instruction 0x{start:x}: {} header {:02x?}",
            instruction.raw,
            &program.data[start..instruction.offset]
        );
    }
    if let Some(blocks) = program.iec_function_blocks() {
        for block in blocks {
            if let Some(directory) = args.get(3) {
                if ["MOVE", "ADD", "EQ"].contains(&block.name.value.as_str()) {
                    fs::write(
                        format!(
                            "{directory}/iec_{}.bin",
                            block.name.value.to_ascii_lowercase()
                        ),
                        &program.data[block.record_offset..block.record_end],
                    )?;
                }
            }
            println!(
                "IEC {} L{} x{}: {:02x?}",
                block.name.value,
                block.row_index,
                block.raw_x,
                &program.data[block.record_offset..block.record_end]
            );
        }
    }
    Ok(())
}
