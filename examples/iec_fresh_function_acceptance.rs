//! Generate a fresh scalar placement and compare every program and local field.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if !(3..=4).contains(&args.len()) {
        return Err(
            "usage: iec_fresh_function_acceptance SOURCE OUTPUT NATIVE [GENERATED_SAVE_AS]".into(),
        );
    }
    let source = XgwxDocument::from_path(&args[0])?;
    let mut doc = source.clone();
    let p = doc.ladder_programs().remove(6)?;
    println!(
        "source rows {}",
        u16::from_le_bytes(p.data[4..6].try_into()?)
    );
    doc.insert_iec_ld_function(6, 87, 4, "MOVE", &["%MW129".into(), "%MW130".into()])?;
    doc.write_to(&args[1])?;
    let generated_original = doc.clone();
    let native = XgwxDocument::from_path(&args[2])?;
    for (index, (before, after)) in source
        .ladder_programs()
        .into_iter()
        .zip(doc.ladder_programs())
        .enumerate()
    {
        let (before, after) = (before?, after?);
        if index == 6 {
            assert_eq!(&before.data[8..], &after.data[8..before.data.len()]);
            let captured = native.ladder_programs().remove(6)?;
            let top = captured
                .iec_row_frames()
                .ok_or("invalid native rows")?
                .into_iter()
                .find(|row| row.row_index == 87)
                .ok_or("missing new group")?;
            assert_eq!(
                &after.data[before.data.len()..],
                &captured.data[top.start - 10..]
            );
        } else {
            assert_eq!(before.data, after.data);
        }
    }
    for (before, after) in source
        .iec_local_symbols()
        .into_iter()
        .zip(doc.iec_local_symbols())
    {
        assert_eq!(
            before?, after?,
            "placement preserves original local records"
        );
    }
    // Use the native-saved document to prove placement independently of
    // XG5000's normalization of old row caches and local record framing.
    // Removing this final group retains every preceding payload byte and
    // every local record. Recreating it must restore the capture exactly.
    let mut doc = native.clone();
    doc.delete_iec_ld_group(6, 15, 87)?;
    doc.insert_iec_ld_function(6, 87, 4, "MOVE", &["%MW129".into(), "%MW130".into()])?;
    let generated = doc.ladder_programs();
    let captured = native.ladder_programs();
    assert_eq!(generated.len(), captured.len());
    let mut exact = true;
    for (i, (a, b)) in generated.into_iter().zip(captured).enumerate() {
        let (a, b) = (a?, b?);
        let differences = (0..a.data.len().max(b.data.len()))
            .filter(|&at| a.data.get(at) != b.data.get(at))
            .map(|at| (at, a.data.get(at).copied(), b.data.get(at).copied()))
            .collect::<Vec<_>>();
        println!(
            "program {i}: {} differences {:?}",
            differences.len(),
            &differences[..differences.len().min(30)]
        );
        exact &= differences.is_empty();
    }
    let a = doc.iec_local_symbols();
    let b = native.iec_local_symbols();
    assert_eq!(a.len(), b.len());
    for (i, (a, b)) in a.into_iter().zip(b).enumerate() {
        assert_eq!(a?, b?, "all local fields including offsets in program {i}");
    }
    assert!(
        exact,
        "fresh placement must match all seven native payloads exactly"
    );
    println!("PASS all program payloads and all local fields match native Save As");
    if let Some(path) = args.get(3) {
        let saved = XgwxDocument::from_path(path)?;
        let a = generated_original.ladder_programs();
        let b = saved.ladder_programs();
        assert_eq!(a.len(), b.len());
        for (index, (a, b)) in a.into_iter().zip(b).enumerate() {
            assert_eq!(a?.data, b?.data, "native generated Save As payload {index}");
        }
        let a = generated_original.iec_local_symbols();
        let b = saved.iec_local_symbols();
        assert_eq!(a.len(), b.len());
        for (index, (a, b)) in a.into_iter().zip(b).enumerate() {
            assert_eq!(
                a?, b?,
                "native generated Save As locals including offsets {index}"
            );
        }
        println!(
            "PASS generated-original native Save As preserves all seven payloads and every local field exactly"
        );
    }
    Ok(())
}
