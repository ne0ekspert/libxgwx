//! Compare branch-row MOVE placement and row shifts with a native XG5000 save.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if !(3..=4).contains(&args.len()) {
        return Err(
            "usage: iec_function_room_acceptance BRANCH_SOURCE OUTPUT NATIVE_OR_DASH [MOVE|ADD|EQ]"
                .into(),
        );
    }
    let (source, output, native) = (&args[0], &args[1], &args[2]);
    let name = args.get(3).map_or("MOVE", String::as_str);
    let operands = match name {
        "MOVE" => vec!["1".into(), "%MW100".into()],
        "ADD" => vec!["1".into(), "2".into(), "%MW100".into()],
        "EQ" => vec!["1".into(), "2".into(), "%MX100".into()],
        _ => return Err("unsupported acceptance instruction".into()),
    };
    let mut generated = XgwxDocument::from_path(source)?;
    generated.insert_iec_ld_function(0, 5, 7, name, &operands)?;
    std::fs::write(output, generated.to_bytes()?)?;
    let mut deleted = generated.clone();
    let block = deleted
        .ladder_programs()
        .remove(0)?
        .iec_function_blocks()
        .ok_or("invalid blocks")?
        .into_iter()
        .find(|block| block.row_index == 5 && block.raw_x == 7)
        .ok_or("missing block")?;
    deleted.delete_iec_ld_branch_function(0, block.record_offset, name)?;
    std::fs::write(format!("{output}.deleted.xgwx"), deleted.to_bytes()?)?;
    deleted.insert_iec_ld_function(0, 5, 7, name, &operands)?;
    assert_eq!(
        generated.to_bytes()?,
        deleted.to_bytes()?,
        "delete/reinsert must restore the complete document"
    );
    if native == "-" {
        return Ok(());
    }
    let native = XgwxDocument::from_path(native)?;
    for (index, (generated, native)) in generated
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .enumerate()
    {
        let (mut generated, mut native) = (generated?, native?);
        let layout = native
            .iec_circuit_layout()
            .ok_or("native layout did not decode")?;
        println!(
            "program {index}: native bindings {}, open branch endpoints {}",
            layout.function_bindings.len(),
            layout.open_branch_endpoints.len()
        );
        for program in [&mut generated, &mut native] {
            for row in program.iec_row_frames().ok_or("invalid row framing")? {
                program.data[row.start + 17..row.start + 21].fill(0);
            }
        }
        let differences = (0..generated.data.len().max(native.data.len()))
            .filter(|&offset| generated.data.get(offset) != native.data.get(offset))
            .collect::<Vec<_>>();
        for offset in differences.iter().take(20) {
            println!(
                "  offset {offset}: {:?} -> {:?}",
                generated.data.get(*offset),
                native.data.get(*offset)
            );
        }
        if !differences.is_empty() {
            return Err(format!(
                "program {index}: {} native differences (lengths {} / {})",
                differences.len(),
                generated.data.len(),
                native.data.len()
            )
            .into());
        }
    }
    println!("all seven program payloads match after masking row display caches");
    Ok(())
}
