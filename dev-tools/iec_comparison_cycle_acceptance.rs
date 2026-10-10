//! Exercise every captured IEC comparison mnemonic through guarded edits.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn payloads(document: &XgwxDocument) -> Result<Vec<Vec<u8>>, xgwx::XgwxError> {
    document
        .ladder_programs()
        .into_iter()
        .map(|program| program.map(|program| program.data))
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_comparison_cycle_acceptance SOURCE OUTPUT".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    let original = payloads(&document)?;
    let original_programs = document.ladder_programs();
    let replacements = [
        ("EQ", "GT"),
        ("GT", "GE"),
        ("GE", "LT"),
        ("LT", "LE"),
        ("LE", "EQ"),
    ];
    let mut sites = Vec::new();
    for (expected, replacement) in replacements {
        let site = original_programs
            .iter()
            .enumerate()
            .find_map(|(index, program)| {
                program
                    .as_ref()
                    .ok()?
                    .iec_function_blocks()?
                    .into_iter()
                    .find_map(|block| {
                        (block.name.value == expected).then_some((
                            index,
                            block.name.offset,
                            block.row_index,
                        ))
                    })
            })
            .ok_or(format!("missing captured {expected} comparison"))?;
        document.update_iec_ld_comparison_function(site.0, site.1, expected, replacement)?;
        println!(
            "program {} L{} offset 0x{:X}: {expected} -> {replacement}",
            site.0, site.2, site.1
        );
        sites.push((site.0, site.1, expected, replacement));
    }
    for program in document.ladder_programs() {
        if program?.iec_circuit_graph().is_none() {
            return Err("edited IEC graph failed validation".into());
        }
    }
    document.write_to(output)?;
    for (index, offset, expected, replacement) in sites {
        document.update_iec_ld_comparison_function(index, offset, replacement, expected)?;
    }
    if payloads(&document)? != original {
        return Err("inverse comparison edits did not restore ProgramData".into());
    }
    println!("PASS five IEC comparison edits and inverses");
    Ok(())
}
