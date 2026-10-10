//! Remove one captured IEC short wire and verify that it can be reinserted.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, program, offset, raw_x, output, rest @ ..] = args.as_slice() else {
        return Err(
            "usage: iec_wire_delete_acceptance <source.xgwx> <program-index> <wire-offset> <raw-x> <output.xgwx> [native-resaved.xgwx]"
                .into(),
        );
    };
    if rest.len() > 1 {
        return Err("expected at most one native resave".into());
    }
    let program_index = program.parse::<usize>()?;
    let wire_offset = if let Some(hex) = offset.strip_prefix("0x") {
        usize::from_str_radix(hex, 16)?
    } else {
        offset.parse::<usize>()?
    };
    let raw_x = raw_x.parse::<u8>()?;
    let original = XgwxDocument::from_path(source)?;
    let mut edited = XgwxDocument::from_path(source)?;
    edited
        .delete_iec_ld_horizontal_wire(program_index, wire_offset, raw_x)
        .map_err(|error| format!("delete failed: {error}"))?;
    let deleted = XgwxDocument::parse(&edited.to_bytes()?)?;
    if let Some(native_path) = rest.first() {
        let native = XgwxDocument::from_path(native_path)?;
        let generated_programs = deleted
            .ladder_programs()
            .into_iter()
            .map(|program| program.map(|value| value.data))
            .collect::<Result<Vec<_>, _>>()?;
        let native_programs = native
            .ladder_programs()
            .into_iter()
            .map(|program| program.map(|value| value.data))
            .collect::<Result<Vec<_>, _>>()?;
        if generated_programs != native_programs {
            return Err("native Save As changed ProgramData".into());
        }
        let generated_symbols = deleted
            .iec_local_symbols()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let native_symbols = native
            .iec_local_symbols()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        if generated_symbols != native_symbols {
            return Err("native Save As changed local symbols".into());
        }
    }
    let mut restored = deleted.clone();
    restored
        .repair_iec_ld_horizontal_wire(program_index, wire_offset, raw_x)
        .map_err(|error| format!("repair failed: {error}"))?;
    let restored = XgwxDocument::parse(&restored.to_bytes()?)?;
    let original_programs = original
        .ladder_programs()
        .into_iter()
        .map(|program| program.map(|value| value.data))
        .collect::<Result<Vec<_>, _>>()?;
    let restored_programs = restored
        .ladder_programs()
        .into_iter()
        .map(|program| program.map(|value| value.data))
        .collect::<Result<Vec<_>, _>>()?;
    if original_programs != restored_programs {
        return Err("repair did not restore all seven ProgramData payloads".into());
    }
    fs::write(output, edited.to_bytes()?)?;
    println!("PASS IEC wire deletion and repair restore all ProgramData payloads");
    if !rest.is_empty() {
        println!("PASS native Save As retained all seven ProgramData and PB50 tables");
    }
    Ok(())
}
