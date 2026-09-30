//! Copy a three-row contact and function network with its missing mapped BOOL locals.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_cross_program_local_copy_acceptance SOURCE OUTPUT".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    document.delete_iec_ld_group(6, 2, 2)?;
    document.delete_iec_ld_group(6, 1, 1)?;
    document.delete_iec_ld_group(6, 0, 0)?;
    let before = document.to_bytes()?;
    if document
        .copy_iec_ld_group_to_program(0, 26, 48, 6, 0)
        .is_ok()
    {
        return Err("copy without source locals unexpectedly succeeded".into());
    }
    if document.to_bytes()? != before {
        return Err("rejected copy changed the document".into());
    }
    let mut collision = document.clone();
    collision.insert_iec_local_symbol(6, "COLLIDE", "BOOL", "")?;
    let collision_index = collision
        .iec_local_symbols()
        .remove(6)?
        .iter()
        .position(|item| item.name == "COLLIDE")
        .ok_or("collision symbol missing")?;
    collision.update_iec_local_symbol_address(6, collision_index, "COLLIDE", "", "%MX8")?;
    let collision_before = collision.to_bytes()?;
    if collision
        .copy_iec_ld_group_to_program_with_locals(0, 26, 48, 6, 0)
        .is_ok()
        || collision.to_bytes()? != collision_before
    {
        return Err("mapped address collision did not reject atomically".into());
    }
    let mut incompatible = document.clone();
    incompatible.insert_iec_local_symbol(6, "ON", "INT", "")?;
    let incompatible_before = incompatible.to_bytes()?;
    if incompatible
        .copy_iec_ld_group_to_program_with_locals(0, 26, 48, 6, 0)
        .is_ok()
        || incompatible.to_bytes()? != incompatible_before
    {
        return Err("incompatible destination local did not reject atomically".into());
    }
    let mut automatic = document.clone();
    automatic.copy_iec_ld_group_to_program_with_locals(0, 26, 48, 6, 0)?;
    let source_symbols = document.iec_local_symbols().remove(0)?;
    for (name, address) in [("OFF", "%MX7"), ("ON", "%MX8")] {
        let source_symbol = source_symbols
            .iter()
            .find(|item| item.name == name)
            .ok_or("source local missing")?;
        document.insert_iec_local_symbol(
            6,
            name,
            "BOOL",
            source_symbol.description.as_deref().unwrap_or(""),
        )?;
        let index = document
            .iec_local_symbols()
            .remove(6)?
            .iter()
            .position(|item| item.name == name)
            .ok_or("new symbol missing")?;
        document.update_iec_local_symbol_address(6, index, name, "", address)?;
    }
    document.copy_iec_ld_group_to_program(0, 26, 48, 6, 0)?;
    if document.to_bytes()? != automatic.to_bytes()? {
        return Err("automatic local copy differs from explicit declaration edits".into());
    }
    let program = document.ladder_programs().remove(6)?;
    let symbols = document.iec_local_symbols().remove(6)?;
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
