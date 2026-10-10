//! Copy a captured R_TRIG network after cloning its instance declaration.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_cross_program_instance_copy_acceptance SOURCE OUTPUT".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    document.delete_iec_ld_group(4, 14, 22)?;
    let cleared = document.to_bytes()?;
    if document
        .copy_iec_ld_group_to_program(0, 18, 32, 4, 22)
        .is_ok()
        || document.to_bytes()? != cleared
    {
        return Err("missing destination R_TRIG instance was not rejected atomically".into());
    }
    if document
        .copy_iec_instance_declaration_to_program(0, "INST_사본2", 4, "Timer")
        .is_ok()
        || document.to_bytes()? != cleared
    {
        return Err("existing destination instance name was not rejected atomically".into());
    }
    document.copy_iec_instance_declaration_to_program(0, "INST_사본2", 4, "INST_사본2")?;
    let instance = document
        .iec_local_symbols()
        .remove(4)?
        .into_iter()
        .find(|symbol| symbol.name == "INST_사본2")
        .ok_or("copied instance declaration missing")?;
    if !instance.is_instance
        || instance.type_reference.as_deref() != Some("R_TRIG")
        || instance.allocation_number != Some(2688)
        || instance.allocation_width != Some(8)
    {
        return Err(format!("copied R_TRIG declaration has wrong allocation: {instance:?}").into());
    }
    document.copy_iec_ld_group_to_program(0, 18, 32, 4, 22)?;
    let program = document.ladder_programs().remove(4)?;
    if program.iec_circuit_graph().is_none()
        || !program
            .iec_function_blocks()
            .ok_or("function decode failed")?
            .iter()
            .any(|block| {
                block.row_index == 22
                    && block.name.value == "R_TRIG"
                    && block
                        .instance
                        .as_ref()
                        .is_some_and(|item| item.value == "INST_사본2")
            })
    {
        return Err("copied R_TRIG network failed graph or instance verification".into());
    }
    let mut automatic = XgwxDocument::from_path(source)?;
    automatic.delete_iec_ld_group(4, 14, 22)?;
    automatic.copy_iec_ld_group_to_program_with_locals(0, 18, 32, 4, 22)?;
    if automatic.to_bytes()? != document.to_bytes()? {
        return Err(
            "automatic instance copy differs from the validated declaration and network copy"
                .into(),
        );
    }
    let mut replacement = XgwxDocument::from_path(source)?;
    let original = replacement.to_bytes()?;
    if replacement
        .replace_iec_ld_group_from_program(0, 18, 32, 4, 14, 22, false)
        .is_ok()
        || replacement.to_bytes()? != original
    {
        return Err(
            "replacement without the missing function instance was not rejected atomically".into(),
        );
    }
    replacement.replace_iec_ld_group_from_program(0, 18, 32, 4, 14, 22, true)?;
    if replacement.to_bytes()? != document.to_bytes()? {
        return Err("cross-program replacement differs from the native-validated copy".into());
    }
    document.write_to(output)?;
    println!("PASS program 0 R_TRIG L32-L35 copied to program 4 L22-L25 with INST_사본2");
    Ok(())
}
