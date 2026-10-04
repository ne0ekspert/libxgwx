//! Acceptance probe for function-cell edits beside an unrelated open network.
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
            "usage: iec_open_cell_cycle SOURCE DELETED RESTORED [NATIVE_DELETED NATIVE_RESTORED]"
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
    for (row, x, name) in [(14, 4, "FF"), (20, 7, "R_TRIG"), (26, 7, "R_TRIG")] {
        let program = deleted.ladder_programs().remove(0)?;
        let block = program
            .iec_function_blocks()
            .ok_or("invalid blocks")?
            .into_iter()
            .find(|b| b.row_index == row && b.raw_x == x && b.name.value == name)
            .ok_or("missing target")?;
        instances.push((
            row,
            x,
            name,
            block.instance.ok_or("missing instance")?.value,
        ));
        deleted.delete_iec_ld_function_cell(0, block.record_offset, name)?;
    }
    assert_eq!(
        open,
        deleted
            .ladder_programs()
            .remove(0)?
            .iec_circuit_layout()
            .unwrap()
            .open_branch_endpoints
    );
    deleted.write_to(&args[1])?;
    let mut restored = deleted.clone();
    for (row, x, name, instance) in instances {
        let site = restored
            .ladder_programs()
            .remove(0)?
            .iec_function_cell_insertion_sites()
            .ok_or("invalid sites")?
            .into_iter()
            .find(|s| s.row_index == row && s.raw_x == x && s.function_name == name)
            .ok_or("missing refill site")?;
        restored.insert_iec_ld_function_cell(0, site.insertion_offset, name, &instance)?;
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
    println!("PASS FF L14 and R_TRIG L20/L26 delete and refill beside retained exposed endpoints");
    if args.len() == 5 {
        exact(&deleted, &XgwxDocument::from_path(&args[3])?)?;
        exact(&restored, &XgwxDocument::from_path(&args[4])?)?;
        println!(
            "PASS deleted and restored native Save As: all seven payloads and every local field exact"
        );
    }
    Ok(())
}
