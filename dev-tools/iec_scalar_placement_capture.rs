//! Recreate an isolated native scalar group and compare the whole project.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 8 {
        return Err(
            "usage: iec_scalar_placement_capture NATIVE OUTPUT PROGRAM ROW X NAME OPERANDS..."
                .into(),
        );
    }
    let native = XgwxDocument::from_path(&args[0])?;
    let program_index = args[2].parse::<usize>()?;
    let row = args[3].parse::<u16>()?;
    let x = args[4].parse::<u8>()?;
    let program = native.ladder_programs().remove(program_index)?;
    let blocks = program
        .iec_function_blocks()
        .ok_or("invalid native blocks")?;
    let target = blocks
        .iter()
        .find(|b| b.row_index == row && b.raw_x == x)
        .ok_or("missing native scalar")?;
    println!(
        "native name {} family {:x} opcode {:x}",
        target.name.value, target.opcode_family, target.opcode
    );
    if blocks
        .iter()
        .filter(|b| b.group_index == target.group_index)
        .count()
        != 1
    {
        return Err("capture group contains another function".into());
    }
    let mut generated = native.clone();
    generated.delete_iec_ld_group(program_index, target.group_index, row)?;
    generated.insert_iec_ld_function(program_index, row, x, &args[5], &args[6..])?;
    generated.write_to(&args[1])?;
    let a = generated.ladder_programs();
    let b = native.ladder_programs();
    assert_eq!(a.len(), b.len());
    let mut exact = true;
    for (index, (a, b)) in a.into_iter().zip(b).enumerate() {
        let (a, b) = (a?, b?);
        let diffs = (0..a.data.len().max(b.data.len()))
            .filter(|&at| a.data.get(at) != b.data.get(at))
            .map(|at| (at, a.data.get(at).copied(), b.data.get(at).copied()))
            .collect::<Vec<_>>();
        println!(
            "program {index}: {} differences {:?}",
            diffs.len(),
            &diffs[..diffs.len().min(30)]
        );
        exact &= diffs.is_empty();
    }
    let a = generated.iec_local_symbols();
    let b = native.iec_local_symbols();
    assert_eq!(a.len(), b.len());
    for (index, (a, b)) in a.into_iter().zip(b).enumerate() {
        assert_eq!(
            a?, b?,
            "every local field including offsets, program {index}"
        );
    }
    assert!(
        exact,
        "native scalar group must be recreated without payload exceptions"
    );
    println!("PASS every program payload and local field matches the native capture exactly");
    Ok(())
}
