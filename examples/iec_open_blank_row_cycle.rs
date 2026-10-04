//! Native checkpoints for blank-row editing beside an unrelated wire gap.
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
    let a = env::args().skip(1).collect::<Vec<_>>();
    if a.len() != 3 && a.len() != 5 {
        return Err("usage: iec_open_blank_row_cycle SOURCE INSERTED RESTORED [NATIVE_INSERTED NATIVE_RESTORED]".into());
    }
    let source = XgwxDocument::from_path(&a[0])?;
    let before = source.ladder_programs().remove(0)?;
    assert_eq!(
        before
            .iec_circuit_layout()
            .ok_or("layout")?
            .open_branch_endpoints
            .len(),
        2
    );
    let mut edited = source.clone();
    edited.insert_iec_ld_blank_row(0, 10)?;
    edited.write_to(&a[1])?;
    let inserted = edited.clone();
    let p = edited.ladder_programs().remove(0)?;
    let expected = before
        .iec_circuit_layout()
        .ok_or("layout")?
        .open_branch_endpoints
        .into_iter()
        .map(|mut p| {
            p.row_index += 1;
            p
        })
        .collect::<Vec<_>>();
    assert_eq!(
        p.iec_circuit_layout()
            .ok_or("layout")?
            .open_branch_endpoints,
        expected
    );
    assert!(
        p.iec_row_frames()
            .ok_or("rows")?
            .iter()
            .all(|r| r.row_index != 11)
    );
    edited.delete_iec_ld_blank_row(0, 11)?;
    edited.write_to(&a[2])?;
    exact(&source, &edited)?;
    println!(
        "PASS blank row insertion/deletion preserves shifted gap and restores all payloads/locals exactly"
    );
    if a.len() == 5 {
        exact(&inserted, &XgwxDocument::from_path(&a[3])?)?;
        exact(&edited, &XgwxDocument::from_path(&a[4])?)?;
        println!(
            "PASS both native Save As checkpoints: all seven payloads and every local field including offsets exact"
        );
    }
    Ok(())
}
