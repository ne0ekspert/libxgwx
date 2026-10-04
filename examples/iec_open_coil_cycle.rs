//! Native checkpoints for terminal coil editing beside an unrelated wire gap.
use std::{env, error::Error};
use xgwx::{IecRecordKind, XgwxDocument};

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
        return Err(
            "usage: iec_open_coil_cycle SOURCE DELETED RESTORED [NATIVE_DELETED NATIVE_RESTORED]"
                .into(),
        );
    }
    let source = XgwxDocument::from_path(&a[0])?;
    let program = source.ladder_programs().remove(0)?;
    let endpoints = program
        .iec_circuit_layout()
        .ok_or("missing layout")?
        .open_branch_endpoints;
    assert!(!endpoints.is_empty());
    let records = program.iec_record_frames().ok_or("missing records")?;
    let mut coils = Vec::new();
    for row in [2, 6, 10] {
        let record = records
            .iter()
            .find(|r| r.row_index == row && matches!(r.kind, IecRecordKind::Coil(14..=19)))
            .ok_or("missing coil")?;
        assert_eq!(program.data[record.offset + 5], 94);
        let IecRecordKind::Coil(code) = record.kind else {
            unreachable!()
        };
        let kind =
            ["OUTPUT", "INVERSE", "SET", "RESET", "RISING", "FALLING"][usize::from(code - 14)];
        let variable = String::from_utf16(
            &program.data[record.offset + 19..record.end]
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect::<Vec<_>>(),
        )?;
        coils.push((row, kind, variable));
    }
    println!("Coils: {coils:?}");
    let mut edited = source.clone();
    for (row, _, variable) in &coils {
        let p = edited.ladder_programs().remove(0)?;
        let record = p
            .iec_record_frames()
            .ok_or("missing records")?
            .into_iter()
            .find(|r| r.row_index == *row && matches!(r.kind, IecRecordKind::Coil(14..=19)))
            .ok_or("missing coil")?;
        edited.delete_iec_ld_terminal_coil(0, record.offset, variable)?;
        assert_eq!(
            edited
                .ladder_programs()
                .remove(0)?
                .iec_circuit_layout()
                .ok_or("missing layout")?
                .open_branch_endpoints,
            endpoints
        );
    }
    edited.write_to(&a[1])?;
    let deleted = edited.clone();
    for (row, kind, variable) in &coils {
        edited.insert_iec_ld_single_element(0, *row, 94, "coil", kind, variable)?;
        assert_eq!(
            edited
                .ladder_programs()
                .remove(0)?
                .iec_circuit_layout()
                .ok_or("missing layout")?
                .open_branch_endpoints,
            endpoints
        );
    }
    edited.write_to(&a[2])?;
    exact(&source, &edited)?;
    println!(
        "PASS three terminal coils: deletion/refill preserves exposed endpoints; restoration exact"
    );
    if a.len() == 5 {
        exact(&deleted, &XgwxDocument::from_path(&a[3])?)?;
        exact(&edited, &XgwxDocument::from_path(&a[4])?)?;
        println!(
            "PASS both native Save As checkpoints: all seven payloads and every local field including offsets exact"
        );
    }
    Ok(())
}
