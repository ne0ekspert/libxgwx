//! Native checkpoints for parallel-wire editing beside an unrelated wire gap.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn exact(a: &XgwxDocument, b: &XgwxDocument) -> Result<(), Box<dyn Error>> {
    let aa = a.ladder_programs();
    let bb = b.ladder_programs();
    assert_eq!(aa.len(), bb.len());
    for (i, (a, b)) in aa.into_iter().zip(bb).enumerate() {
        assert_eq!(a?.data, b?.data, "native payload {i}");
    }
    let aa = a.iec_local_symbols();
    let bb = b.iec_local_symbols();
    assert_eq!(aa.len(), bb.len());
    for (i, (a, b)) in aa.into_iter().zip(bb).enumerate() {
        assert_eq!(a?, b?, "all local fields including offsets {i}");
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 6 && args.len() != 7 {
        return Err(
            "usage: iec_parallel_wire_probe SOURCE OUTPUT GROUP START END X [NATIVE]".into(),
        );
    }
    let source = XgwxDocument::from_path(&args[0])?;
    let group = args[2].parse()?;
    let start = args[3].parse()?;
    let end = args[4].parse()?;
    let x = args[5].parse()?;
    let mut edited = source.clone();
    edited.edit_iec_ld_branch_segment(0, group, start, end, x, true, false)?;
    edited.write_to(&args[1])?;
    if let Some(native) = args.get(6) {
        exact(&edited, &XgwxDocument::from_path(native)?)?;
        println!("PASS native payloads and every local field exact");
    }
    edited.edit_iec_ld_vertical_wire(0, group, start, end, x, false, true)?;
    exact(&source, &edited)?;
    println!("PASS exact restoration after isolated branch deletion");
    Ok(())
}
