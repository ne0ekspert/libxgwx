//! Clear representative input and output mapped BOOLs in the smart-home fixture.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output, rest @ ..] = args.as_slice() else {
        return Err(
            "usage: iec_address_io_unmap_acceptance <source.xgwx> <output.xgwx> [resaved.xgwx]"
                .into(),
        );
    };
    if rest.len() > 1 {
        return Err("expected at most one native resave".into());
    }
    let original = XgwxDocument::from_path(source)?;
    let original_programs = original
        .ladder_programs()
        .into_iter()
        .map(|program| program.map(|value| value.data))
        .collect::<Result<Vec<_>, _>>()?;
    let mut generated = XgwxDocument::from_path(source)?;
    generated.update_iec_local_symbol_address(0, 12, "조명_1", "%QX10", "")?;
    generated.update_iec_local_symbol_address(1, 3, "커튼_제어", "%IX0.0.7", "")?;
    let encoded = generated.to_bytes()?;
    let reparsed = XgwxDocument::parse(&encoded)?;
    let symbols = reparsed
        .iec_local_symbols()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    for (program, symbol, name) in [(0, 12, "조명_1"), (1, 3, "커튼_제어")] {
        let entry = &symbols[program][symbol];
        if entry.name != name
            || entry.address.is_some()
            || !entry.storage_class.is_empty()
            || entry.allocation_number.is_some()
            || entry.allocation_width.is_some()
        {
            return Err(format!("unexpected unmapped symbol fields for {name}").into());
        }
    }
    let edited_programs = reparsed
        .ladder_programs()
        .into_iter()
        .map(|program| program.map(|value| value.data))
        .collect::<Result<Vec<_>, _>>()?;
    if edited_programs != original_programs {
        return Err("address edit changed IEC ProgramData".into());
    }
    if let Some(resaved_path) = rest.first() {
        let resaved = XgwxDocument::from_path(resaved_path)?;
        let mut native_programs = resaved
            .ladder_programs()
            .into_iter()
            .map(|program| program.map(|value| value.data))
            .collect::<Result<Vec<_>, _>>()?;
        for (offset, generated_byte, native_byte) in [
            (0x0c88, 0x34, 0x27),
            (0x0f1f, 0x34, 0x32),
            (0x114a, 0x34, 0x27),
            (0x1389, 0x4c, 0x32),
        ] {
            if edited_programs[0][offset] != generated_byte
                || native_programs[0][offset] != native_byte
            {
                return Err(format!("unexpected native program 0 byte at {offset:#x}").into());
            }
            native_programs[0][offset] = generated_byte;
        }
        if native_programs != edited_programs {
            return Err("XG5000 Save As changed IEC ProgramData beyond known display bytes".into());
        }
        let mut native_symbols = resaved
            .iec_local_symbols()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let mut generated_symbols = symbols;
        for (left, right) in generated_symbols.iter_mut().zip(&mut native_symbols) {
            for entry in left.iter_mut().chain(right.iter_mut()) {
                entry.record_offset = 0;
            }
        }
        if generated_symbols != native_symbols {
            return Err("XG5000 Save As changed parsed IEC symbols".into());
        }
    }
    fs::write(output, encoded)?;
    println!("PASS cleared %QX10 and %IX0.0.7; generated ProgramData unchanged");
    if !rest.is_empty() {
        println!("PASS native Save As preserved symbols and normalized only known display bytes");
    }
    Ok(())
}
