//! Acceptance probe for fresh scalar blocks appended to an existing project.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if !(2..=3).contains(&args.len()) {
        return Err("usage: iec_fresh_scalar_sequence SOURCE OUTPUT [NATIVE_SAVE_AS]".into());
    }
    let source = XgwxDocument::from_path(&args[0])?;
    let mut generated = source.clone();
    for (row, x, name, operands) in [
        (87, 4, "MOVE", vec!["%MW129", "%MW130"]),
        (91, 16, "ADD", vec!["%MW129", "1", "%MW130"]),
        (96, 10, "EQ", vec!["%MW129", "1", "%MX735"]),
    ] {
        generated.insert_iec_ld_function(
            6,
            row,
            x,
            name,
            &operands.into_iter().map(String::from).collect::<Vec<_>>(),
        )?;
    }
    let before = source.ladder_programs();
    let after = generated.ladder_programs();
    assert_eq!(before.len(), after.len());
    for (i, (before, after)) in before.into_iter().zip(after).enumerate() {
        let (before, after) = (before?, after?);
        if i == 6 {
            assert_eq!(&before.data[8..], &after.data[8..before.data.len()]);
            assert_eq!(
                after
                    .iec_function_blocks()
                    .ok_or("invalid functions")?
                    .len(),
                before
                    .iec_function_blocks()
                    .ok_or("invalid source functions")?
                    .len()
                    + 3
            );
        } else {
            assert_eq!(before.data, after.data, "preserved payload {i}");
        }
    }
    let before = source.iec_local_symbols();
    let after = generated.iec_local_symbols();
    assert_eq!(before.len(), after.len());
    for (i, (before, after)) in before.into_iter().zip(after).enumerate() {
        assert_eq!(
            before?, after?,
            "preserved local fields including offsets {i}"
        );
    }
    generated.write_to(&args[1])?;
    println!("PASS original prefix, six other payloads and every local field preserved");
    if let Some(path) = args.get(2) {
        let native = XgwxDocument::from_path(path)?;
        let a = generated.ladder_programs();
        let b = native.ladder_programs();
        assert_eq!(a.len(), b.len());
        for (i, (a, b)) in a.into_iter().zip(b).enumerate() {
            assert_eq!(a?.data, b?.data, "native Save As payload {i}");
            println!("program {i}: exact");
        }
        let a = generated.iec_local_symbols();
        let b = native.iec_local_symbols();
        assert_eq!(a.len(), b.len());
        for (i, (a, b)) in a.into_iter().zip(b).enumerate() {
            assert_eq!(
                a?, b?,
                "native Save As all local fields including offsets {i}"
            );
        }
        println!("PASS native Save As preserves all payloads and all local fields exactly");
    }
    Ok(())
}
