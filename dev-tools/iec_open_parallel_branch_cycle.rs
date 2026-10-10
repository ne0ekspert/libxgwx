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
    if args.len() != 4 && args.len() != 7 {
        return Err("usage: iec_open_parallel_branch_cycle SOURCE FIRST SECOND RESTORED [NATIVE_FIRST NATIVE_SECOND NATIVE_RESTORED]".into());
    }
    let source = XgwxDocument::from_path(&args[0])?;
    let before = source.ladder_programs().remove(0)?;
    let endpoints = before
        .iec_circuit_layout()
        .ok_or("layout")?
        .open_branch_endpoints;
    assert_eq!(endpoints.len(), 2);
    let geometry = before.iec_geometry().ok_or("geometry")?;
    let mut checkpoints = Vec::new();
    for reverse in [false, true] {
        let mut edited = source.clone();
        let mut selected = Vec::new();
        for group in [8, 26, 34] {
            let candidates = geometry
                .vertical
                .iter()
                .filter(|wire| {
                    wire.group_index == group
                        && source
                            .clone()
                            .edit_iec_ld_branch_segment(
                                0,
                                group,
                                wire.start_row_index,
                                wire.end_row_index,
                                wire.x,
                                true,
                                false,
                            )
                            .is_ok()
                })
                .collect::<Vec<_>>();
            assert_eq!(candidates.len(), 2, "parallel pair in group {group}");
            let wire = candidates[usize::from(reverse)];
            println!(
                "remove group {group} L{}-L{} x{}",
                wire.start_row_index, wire.end_row_index, wire.x
            );
            edited.edit_iec_ld_branch_segment(
                0,
                group,
                wire.start_row_index,
                wire.end_row_index,
                wire.x,
                true,
                false,
            )?;
            selected.push(wire);
        }
        let after = edited.ladder_programs().remove(0)?;
        assert_eq!(after.data.len() + 3 * 36, before.data.len());
        assert_eq!(
            after.iec_row_frames().ok_or("rows")?.len(),
            before.iec_row_frames().ok_or("rows")?.len()
        );
        assert_eq!(
            after
                .iec_circuit_layout()
                .ok_or("layout")?
                .open_branch_endpoints,
            endpoints
        );
        edited.write_to(&args[1 + usize::from(reverse)])?;
        checkpoints.push(edited.clone());
        for wire in selected.into_iter().rev() {
            edited.edit_iec_ld_vertical_wire(
                0,
                wire.group_index,
                wire.start_row_index,
                wire.end_row_index,
                wire.x,
                false,
                true,
            )?;
        }
        exact(&source, &edited)?;
        if reverse {
            edited.write_to(&args[3])?;
            checkpoints.push(edited);
        }
    }
    println!(
        "PASS six parallel removals beside gap and exact refill: every payload and local field"
    );
    if args.len() == 7 {
        for (checkpoint, path) in checkpoints.iter().zip(&args[4..]) {
            exact(checkpoint, &XgwxDocument::from_path(path)?)?;
        }
        println!(
            "PASS three native checkpoints: all seven payloads and every local field including offsets exact"
        );
    }
    Ok(())
}
