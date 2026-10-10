//! Acceptance probe for connected arithmetic edits beside an unrelated open network.
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
    if args.len() != 3 && args.len() != 5 {
        return Err(
            "usage: iec_open_arithmetic_cycle SOURCE DELETED RESTORED [NATIVE_DELETED NATIVE_RESTORED]"
                .into(),
        );
    }
    let source = XgwxDocument::from_path(&args[0])?;
    let original = source.ladder_programs().remove(0)?;
    let open = original
        .iec_circuit_layout()
        .ok_or("invalid source")?
        .open_branch_endpoints;
    assert!(!open.is_empty());
    let mut deleted = source.clone();
    let mut instances = Vec::new();
    for (row, x, name) in [(20, 16, "ADD"), (26, 16, "SUB")] {
        let program = deleted.ladder_programs().remove(0)?;
        let block = program
            .iec_function_blocks()
            .ok_or("invalid blocks")?
            .into_iter()
            .find(|b| b.row_index == row && b.raw_x == x && b.name.value == name)
            .ok_or("missing target")?;
        instances.push((row, x, name));
        deleted.delete_iec_ld_connected_arithmetic(0, block.record_offset, name)?;
    }
    let mut moved_open = open.clone();
    for point in &mut moved_open {
        point.group_index += 2;
    }
    assert_eq!(
        moved_open,
        deleted
            .ladder_programs()
            .remove(0)?
            .iec_circuit_layout()
            .unwrap()
            .open_branch_endpoints
    );
    deleted.write_to(&args[1])?;
    let mut restored = deleted.clone();
    for (row, x, name) in instances {
        restored.insert_iec_ld_function(
            0,
            row,
            x,
            name,
            &["%MW700".into(), "1".into(), "%MW700".into()],
        )?;
    }
    assert_eq!(
        open,
        restored
            .ladder_programs()
            .remove(0)?
            .iec_circuit_layout()
            .unwrap()
            .open_branch_endpoints
    );
    restored.write_to(&args[2])?;
    println!(
        "PASS ADD L20 and SUB L26 split/delete and merge/refill beside retained exposed endpoints"
    );
    if args.len() == 5 {
        exact(&deleted, &XgwxDocument::from_path(&args[3])?)?;
        exact(&restored, &XgwxDocument::from_path(&args[4])?)?;
        println!(
            "PASS deleted and restored native Save As: all seven payloads and every local field exact"
        );
    }
    Ok(())
}
