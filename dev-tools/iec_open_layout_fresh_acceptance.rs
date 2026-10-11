//! Verify independent scalar placement while an earlier network has exposed endpoints.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if !(2..=3).contains(&args.len()) {
        return Err(
            "usage: iec_open_layout_fresh_acceptance SOURCE OUTPUT [NATIVE_SAVE_AS]".into(),
        );
    }
    let source = XgwxDocument::from_path(&args[0])?;
    let before = source.program().ladder_programs().remove(0)?;
    let layout = before.iec_circuit_layout().ok_or("invalid source layout")?;
    assert!(!layout.open_branch_endpoints.is_empty());
    let row = u16::from_le_bytes(before.data[4..6].try_into()?) + 2;
    let mut generated = source.clone();
    generated
        .program_mut()
        .get(0)
        .and_then(|mut selected_program| {
            selected_program.iec_ld()?.insert_function(
                row,
                4,
                "MOVE",
                &["1".into(), "%MW100".into()],
            )
        })?;
    let after = generated.program().ladder_programs().remove(0)?;
    assert_eq!(&before.data[8..], &after.data[8..before.data.len()]);
    let after_layout = after.iec_circuit_layout().ok_or("invalid result layout")?;
    assert_eq!(
        layout.open_branch_endpoints,
        after_layout.open_branch_endpoints
    );
    assert_eq!(
        layout.function_bindings,
        after_layout.function_bindings[..layout.function_bindings.len()]
    );
    for (i, (a, b)) in source
        .program()
        .ladder_programs()
        .into_iter()
        .zip(generated.program().ladder_programs())
        .enumerate()
    {
        if i != 0 {
            assert_eq!(a?.data, b?.data, "preserved program {i}");
        }
    }
    let a = source.variable().iec_local_symbols();
    let b = generated.variable().iec_local_symbols();
    assert_eq!(a.len(), b.len());
    for (i, (a, b)) in a.into_iter().zip(b).enumerate() {
        assert_eq!(a?, b?, "preserved locals {i}");
    }
    generated.write_to(&args[1])?;
    println!(
        "PASS independent MOVE L{row}/x4; earlier bytes, exposed endpoints, old bindings, six programs and all local fields preserved"
    );
    if let Some(path) = args.get(2) {
        let native = XgwxDocument::from_path(path)?;
        let a = generated.program().ladder_programs();
        let b = native.program().ladder_programs();
        assert_eq!(a.len(), b.len());
        for (i, (a, b)) in a.into_iter().zip(b).enumerate() {
            assert_eq!(a?.data, b?.data, "native Save As payload {i}");
            println!("program {i}: exact");
        }
        let a = generated.variable().iec_local_symbols();
        let b = native.variable().iec_local_symbols();
        assert_eq!(a.len(), b.len());
        for (i, (a, b)) in a.into_iter().zip(b).enumerate() {
            assert_eq!(a?, b?, "native Save As local fields including offsets {i}");
        }
        println!("PASS native Save As all seven payloads and every local field exact");
    }
    Ok(())
}
