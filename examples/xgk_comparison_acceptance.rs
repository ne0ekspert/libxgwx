//! Generate the complete XGK input-comparison catalog for native validation.
use std::{env, error::Error, fs, path::Path};
use xgwx::XgwxDocument;

fn operands(spec: &xgwx::LadderInstructionSpec) -> Vec<String> {
    let bit = spec.mnemonic.starts_with('4') || spec.mnemonic.starts_with('8');
    let mut result = if bit {
        vec!["D100.0".into(), "D102.0".into()]
    } else {
        vec!["D100".into(), "D104".into()]
    };
    if spec.operand_count == 3 {
        result.push(
            if spec.mnemonic.starts_with('G') || spec.mnemonic.starts_with("DG") {
                "1"
            } else {
                "D108"
            }
            .into(),
        );
    }
    result
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: xgk_comparison_acceptance SOURCE.xgwx NEW_DIRECTORY".into());
    };
    let output = Path::new(output);
    fs::create_dir(output)?;
    let source = XgwxDocument::from_path(source)?;
    let catalog = xgwx::ladder_comparison_catalog()
        .iter()
        .filter(|spec| !matches!(spec.mnemonic, "B" | "BN"))
        .collect::<Vec<_>>();
    assert_eq!(catalog.len(), 78);
    for (batch, specs) in catalog.chunks(39).enumerate() {
        let mut document = source.clone();
        for (index, spec) in specs.iter().enumerate() {
            // Insert ahead of the captured END; never emit unreachable code.
            let y = u8::try_from(8 + index * 4)?;
            document.insert_ladder_row(0, y)?;
            document.insert_ladder_comparison(0, y, 0, spec.mnemonic, &operands(spec))?;
            document.insert_ladder_instruction(
                0,
                y,
                "MOV",
                &["1".into(), format!("D{}", 200 + index)],
            )?;
        }
        let programs = document
            .ladder_programs()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let cells = &programs[0].elements;
        for spec in specs {
            assert!(
                cells
                    .iter()
                    .any(|cell| cell.value == spec.mnemonic && cell.operands == operands(spec))
            );
        }
        document.write_to(output.join(format!("XGC{}GEN.xgwx", batch + 1)))?;
        println!(
            "batch {}: {} comparisons, {} payload bytes",
            batch + 1,
            specs.len(),
            programs[0].data.len()
        );
    }
    Ok(())
}
