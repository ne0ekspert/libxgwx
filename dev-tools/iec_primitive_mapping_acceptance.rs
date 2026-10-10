//! Prepare a native capture of all primitive local memory address shapes.
use std::{error::Error, path::Path};
use xgwx::XgwxDocument;

fn exact(a: &XgwxDocument, b: &XgwxDocument) -> Result<(), Box<dyn Error>> {
    assert_eq!(a.ladder_programs().len(), 7);
    assert_eq!(b.ladder_programs().len(), 7);
    for (index, (a, b)) in a
        .ladder_programs()
        .into_iter()
        .zip(b.ladder_programs())
        .enumerate()
    {
        assert_eq!(a?.data, b?.data, "program payload {index}");
    }
    assert_eq!(a.iec_local_symbols().len(), 7);
    assert_eq!(b.iec_local_symbols().len(), 7);
    for (index, (a, b)) in a
        .iec_local_symbols()
        .into_iter()
        .zip(b.iec_local_symbols())
        .enumerate()
    {
        assert_eq!(a?, b?, "every local field including offsets {index}");
    }
    Ok(())
}

fn generate(
    source: XgwxDocument,
    output: &Path,
    native: Option<&String>,
) -> Result<(), Box<dyn Error>> {
    // These prefixes and widths were read from the native PRNS capture, not inferred
    // from the writer. DATE is a word; DATE_AND_TIME is a long word.
    let cases = [
        ("BOOL", "%MX", 1),
        ("BYTE", "%MB", 8),
        ("DATE", "%MW", 16),
        ("DATE_AND_TIME", "%ML", 64),
        ("DINT", "%MD", 32),
        ("DWORD", "%MD", 32),
        ("INT", "%MW", 16),
        ("LINT", "%ML", 64),
        ("LREAL", "%ML", 64),
        ("LWORD", "%ML", 64),
        ("REAL", "%MD", 32),
        ("SINT", "%MB", 8),
        ("TIME", "%MD", 32),
        ("TIME_OF_DAY", "%MD", 32),
        ("UDINT", "%MD", 32),
        ("UINT", "%MW", 16),
        ("ULINT", "%ML", 64),
        ("USINT", "%MB", 8),
        ("WORD", "%MW", 16),
    ];
    let symbols = source.iec_local_symbols().remove(0)?;
    assert_eq!(
        symbols
            .iter()
            .filter(|symbol| symbol.name.starts_with("AAA_"))
            .count(),
        19
    );
    let mut base = source.clone();
    let mut edited = source.clone();
    for (index, (ty, prefix, width)) in cases.iter().enumerate() {
        let name = format!("AAA_{ty}");
        let symbol_index = symbols
            .iter()
            .position(|symbol| symbol.name == name)
            .unwrap();
        let symbol = &symbols[symbol_index];
        let bit = 16000 + index as u32 * 128;
        let address = format!("{prefix}{}", bit / width);
        assert_eq!(symbol.data_type.as_deref(), Some(*ty));
        assert_eq!(symbol.address.as_deref(), Some(address.as_str()));
        assert_eq!(symbol.storage_class, "M");
        assert_eq!(symbol.allocation_number, Some(bit));
        assert_eq!(symbol.allocation_width, Some(*width));
        base.update_iec_local_symbol_address(0, symbol_index, &name, &address, "")?;
        edited.update_iec_local_symbol_address(
            0,
            symbol_index,
            &name,
            &address,
            &format!("{prefix}{}", bit / width + 1),
        )?;
    }
    base.write_to(output.join("PRBASE.xgwx"))?;
    let mut recreated = base;
    for (index, (ty, prefix, width)) in cases.iter().enumerate() {
        let name = format!("AAA_{ty}");
        let symbol_index = symbols
            .iter()
            .position(|symbol| symbol.name == name)
            .unwrap();
        recreated.update_iec_local_symbol_address(
            0,
            symbol_index,
            &name,
            "",
            &format!("{prefix}{}", (16000 + index as u32 * 128) / width),
        )?;
    }
    exact(&source, &recreated)?;
    recreated.write_to(output.join("PRREGEN.xgwx"))?;
    let before = edited.to_bytes()?;
    for (index, (ty, prefix, width)) in cases.iter().enumerate() {
        let name = format!("AAA_{ty}");
        let symbol_index = symbols
            .iter()
            .position(|symbol| symbol.name == name)
            .unwrap();
        let number = (16000 + index as u32 * 128) / width + 1;
        let current = format!("{prefix}{number}");
        let wrong_prefix = if *prefix == "%MX" { "%MW" } else { "%MX" };
        for rejected in [
            format!("{prefix}+{number}"),
            format!("{wrong_prefix}{number}"),
            format!("{prefix}4294967296"),
            format!("{prefix}0{number}"),
        ] {
            assert!(
                edited
                    .update_iec_local_symbol_address(0, symbol_index, &name, &current, &rejected)
                    .is_err(),
                "{name} must reject {rejected}"
            );
            assert_eq!(before, edited.to_bytes()?, "rejection is atomic");
        }
    }
    for (name, current, overlap) in [
        ("AAA_BOOL", "%MX16001", "%MX16136"),
        ("AAA_WORD", "%MW1145", "%MW1008"),
        ("AAA_LINT", "%ML265", "%ML252"),
    ] {
        let index = symbols
            .iter()
            .position(|symbol| symbol.name == name)
            .unwrap();
        assert!(
            edited
                .update_iec_local_symbol_address(0, index, name, current, overlap)
                .is_err()
        );
        assert_eq!(before, edited.to_bytes()?);
    }
    edited.write_to(output.join("PRGEN.xgwx"))?;
    exact(
        &edited,
        &XgwxDocument::from_path(output.join("PRGEN.xgwx"))?,
    )?;
    if let Some(native) = native {
        exact(&edited, &XgwxDocument::from_path(native)?)?;
        println!("PASS native all seven payloads and every local field including offsets");
    }
    println!(
        "PASS all 19 native mappings, clear/assign recreation, remapping, atomic type/format/range/overlap guards"
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut document = XgwxDocument::from_path(&args[0])?;
    if args.get(2).is_some_and(|mode| mode == "generate") {
        return generate(document, Path::new(&args[1]), args.get(3));
    }
    let before = document
        .ladder_programs()
        .into_iter()
        .map(|program| program.map(|program| program.data))
        .collect::<Result<Vec<_>, _>>()?;
    let mut types = vec![
        "BOOL",
        "BYTE",
        "WORD",
        "DWORD",
        "LWORD",
        "SINT",
        "INT",
        "DINT",
        "LINT",
        "USINT",
        "UINT",
        "UDINT",
        "ULINT",
        "REAL",
        "LREAL",
        "TIME",
        "DATE",
        "TIME_OF_DAY",
        "DATE_AND_TIME",
    ];
    types.sort();
    for (index, name) in types.iter().enumerate() {
        document.insert_iec_local_symbol(0, &format!("AAA_{name}"), name, "")?;
        println!(
            "{} AAA_{name} proposed bit {}",
            index + 1,
            16000 + index * 128
        );
    }
    let after = document
        .ladder_programs()
        .into_iter()
        .map(|program| program.map(|program| program.data))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(before.len(), 7);
    assert_eq!(
        before, after,
        "preparation must preserve all seven program payloads"
    );
    document.write_to(Path::new(&args[1]))?;
    Ok(())
}
