//! Reproduce XG5000's mapped BOOL Delete edit on the smart-home fixture.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, native, output, rest @ ..] = args.as_slice() else {
        return Err(
            "usage: iec_address_unmap_acceptance <source.xgwx> <native.xgwx> <output.xgwx> [resaved-generated.xgwx]".into(),
        );
    };
    if rest.len() > 1 {
        return Err("expected at most one resaved-generated file".into());
    }
    let source_bytes = fs::read(source)?;
    let mut generated = XgwxDocument::parse(&source_bytes)?;
    generated.update_iec_local_symbol_address(0, 5, "ON", "%MX8", "")?;
    let generated_bytes = generated.to_bytes()?;
    let native = XgwxDocument::from_path(native)?;
    assert_equivalent(&generated, &native, "native unmap")?;
    if let Some(resaved) = rest.first() {
        assert_equivalent(
            &generated,
            &XgwxDocument::from_path(resaved)?,
            "resaved generated unmap",
        )?;
    }
    let mut restored = XgwxDocument::parse(&generated_bytes)?;
    restored.update_iec_local_symbol_address(0, 5, "ON", "", "%MX8")?;
    let restored = XgwxDocument::parse(&restored.to_bytes()?)?;
    let original = XgwxDocument::parse(&source_bytes)?;
    if restored
        .iec_local_symbols()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        != original
            .iec_local_symbols()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
    {
        return Err("inverse did not restore original local symbols".into());
    }
    fs::write(output, generated_bytes)?;
    println!(
        "PASS program 0 ON unmap matches native structured symbols; inverse restores original"
    );
    Ok(())
}

fn assert_equivalent(
    generated: &XgwxDocument,
    native: &XgwxDocument,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    for (index, (left, right)) in generated
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .enumerate()
    {
        if left?.data != right?.data {
            return Err(format!("program {index} differs from {label}").into());
        }
    }
    let mut generated_symbols = generated
        .iec_local_symbols()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let mut native_symbols = native
        .iec_local_symbols()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    for (left, right) in generated_symbols.iter_mut().zip(&mut native_symbols) {
        for symbol in left.iter_mut().chain(right.iter_mut()) {
            symbol.record_offset = 0;
        }
    }
    if generated_symbols != native_symbols {
        return Err(format!("{label} structured local symbols differ").into());
    }
    Ok(())
}
