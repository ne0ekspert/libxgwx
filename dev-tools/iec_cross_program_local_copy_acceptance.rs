//! Copy a three-row contact and function network with its missing mapped BOOL locals.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_cross_program_local_copy_acceptance SOURCE OUTPUT".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    document
        .program_mut()
        .get(6)
        .and_then(|mut selected_program| selected_program.iec_ld()?.delete_group(2, 2))?;
    document
        .program_mut()
        .get(6)
        .and_then(|mut selected_program| selected_program.iec_ld()?.delete_group(1, 1))?;
    document
        .program_mut()
        .get(6)
        .and_then(|mut selected_program| selected_program.iec_ld()?.delete_group(0, 0))?;
    let before = document.to_bytes()?;
    if document
        .program_mut()
        .get(0)
        .and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .copy_group_to_program(26, 48, 6, 0)
        })
        .is_ok()
    {
        return Err("copy without source locals unexpectedly succeeded".into());
    }
    if document.to_bytes()? != before {
        return Err("rejected copy changed the document".into());
    }
    let mut collision = document.clone();
    collision
        .variable_mut()
        .insert_iec_local_symbol(6, "COLLIDE", "BOOL", "")?;
    let collision_index = collision
        .variable()
        .iec_local_symbols()
        .remove(6)?
        .iter()
        .position(|item| item.name == "COLLIDE")
        .ok_or("collision symbol missing")?;
    collision.variable_mut().update_iec_local_symbol_address(
        6,
        collision_index,
        "COLLIDE",
        "",
        "%MX8",
    )?;
    let collision_before = collision.to_bytes()?;
    if collision
        .program_mut()
        .get(0)
        .and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .copy_group_to_program_with_locals(26, 48, 6, 0)
        })
        .is_ok()
        || collision.to_bytes()? != collision_before
    {
        return Err("mapped address collision did not reject atomically".into());
    }
    let mut incompatible = document.clone();
    incompatible
        .variable_mut()
        .insert_iec_local_symbol(6, "ON", "INT", "")?;
    let incompatible_before = incompatible.to_bytes()?;
    if incompatible
        .program_mut()
        .get(0)
        .and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .copy_group_to_program_with_locals(26, 48, 6, 0)
        })
        .is_ok()
        || incompatible.to_bytes()? != incompatible_before
    {
        return Err("incompatible destination local did not reject atomically".into());
    }
    let mut automatic = document.clone();
    automatic
        .program_mut()
        .get(0)
        .and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .copy_group_to_program_with_locals(26, 48, 6, 0)
        })?;
    let source_symbols = document.variable().iec_local_symbols().remove(0)?;
    for (name, address) in [("OFF", "%MX7"), ("ON", "%MX8")] {
        let source_symbol = source_symbols
            .iter()
            .find(|item| item.name == name)
            .ok_or("source local missing")?;
        document.variable_mut().insert_iec_local_symbol(
            6,
            name,
            "BOOL",
            source_symbol.description.as_deref().unwrap_or(""),
        )?;
        let index = document
            .variable()
            .iec_local_symbols()
            .remove(6)?
            .iter()
            .position(|item| item.name == name)
            .ok_or("new symbol missing")?;
        document
            .variable_mut()
            .update_iec_local_symbol_address(6, index, name, "", address)?;
    }
    document
        .program_mut()
        .get(0)
        .and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .copy_group_to_program(26, 48, 6, 0)
        })?;
    if document.to_bytes()? != automatic.to_bytes()? {
        return Err("automatic local copy differs from explicit declaration edits".into());
    }
    let program = document.program().ladder_programs().remove(6)?;
    let symbols = document.variable().iec_local_symbols().remove(6)?;
    if program.iec_circuit_graph().is_none()
        || !symbols
            .iter()
            .any(|item| item.name == "ON" && item.address.as_deref() == Some("%MX8"))
        || !symbols
            .iter()
            .any(|item| item.name == "OFF" && item.address.as_deref() == Some("%MX7"))
    {
        return Err("copied network or mapped locals did not decode".into());
    }
    document.write_to(output)?;
    println!("PASS copied program 0 L48-L50 into program 6 L0-L2 with ON/OFF locals");
    Ok(())
}
