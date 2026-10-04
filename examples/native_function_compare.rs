//! Compare decoded program payloads after native Save As.
use std::{env, error::Error};
use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [before, after] = args.as_slice() else {
        return Err("usage: native_function_compare BEFORE AFTER".into());
    };
    let old_document = XgwxDocument::from_path(before)?;
    let new_document = XgwxDocument::from_path(after)?;
    let old_locals = old_document
        .iec_local_symbols()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let new_locals = new_document
        .iec_local_symbols()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    if old_locals != new_locals {
        return Err("parsed local symbols or record offsets changed".into());
    }
    println!(
        "local symbols: {} program tables match exactly",
        old_locals.len()
    );
    let old = old_document.ladder_programs();
    let new = new_document.ladder_programs();
    if old.len() != new.len() {
        return Err("program count changed".into());
    }
    let mut changed = Vec::new();
    for (index, (old, new)) in old.into_iter().zip(new).enumerate() {
        let old = old?;
        let new = new?;
        let differences = (0..old.data.len().max(new.data.len()))
            .filter(|&i| old.data.get(i) != new.data.get(i))
            .collect::<Vec<_>>();
        println!(
            "program {index}: {} -> {} bytes; exact {}; differences {} {:?}",
            old.data.len(),
            new.data.len(),
            old.data == new.data,
            differences.len(),
            &differences[..differences.len().min(25)]
        );
        if let (Some(before), Some(after)) = (old.iec_function_blocks(), new.iec_function_blocks())
        {
            let shape = |blocks: Vec<xgwx::IecFunctionBlock>| {
                blocks
                    .into_iter()
                    .map(|b| {
                        (
                            b.row_index,
                            b.raw_x,
                            b.name.value,
                            b.opcode_family,
                            b.opcode,
                            b.pin_count,
                        )
                    })
                    .collect::<Vec<_>>()
            };
            let before = shape(before);
            let after = shape(after);
            if before != after {
                for block in before.iter().filter(|block| !after.contains(block)) {
                    println!("program {index}: removed function {block:?}");
                }
                for block in after.iter().filter(|block| !before.contains(block)) {
                    println!("program {index}: added function {block:?}");
                }
            }
            if before != after
                || new.iec_circuit_layout().is_none()
                || (old.iec_circuit_graph().is_some() && new.iec_circuit_graph().is_none())
            {
                return Err(
                    format!("program {index}: function shape or circuit graph changed").into(),
                );
            }
        }
        if old.data != new.data {
            changed.push(index);
        }
    }
    if !changed.is_empty() {
        return Err(format!("decoded program payloads differ: {changed:?}").into());
    }
    Ok(())
}
