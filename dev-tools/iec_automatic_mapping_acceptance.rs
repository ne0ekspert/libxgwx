//! Native acceptance for mapping the smart-home project's automatic primitives.
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

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let source = XgwxDocument::from_path(&args[0])?;
    let mut edited = source.clone();
    let symbols = source.iec_local_symbols().remove(4)?;
    for (name, allocation, width, address) in [
        ("div_값", 160, 32, "%MD700"),
        ("LNG가스누출", 2, 1, "%MX21000"),
        ("메모리값", 16, 16, "%MW1500"),
        ("시간", 1088, 32, "%MD800"),
    ] {
        let index = symbols
            .iter()
            .position(|symbol| symbol.name == name)
            .unwrap();
        let symbol = &symbols[index];
        assert_eq!(symbol.storage_class, "A");
        assert_eq!(symbol.address, None);
        assert_eq!(symbol.allocation_number, Some(allocation));
        assert_eq!(symbol.allocation_width, Some(width));
        let before = edited.to_bytes()?;
        for rejected in ["%MX0", "%MD+700", "%MW4294967296"] {
            assert!(
                edited
                    .update_iec_local_symbol_address(4, index, name, "", rejected)
                    .is_err()
            );
            assert_eq!(before, edited.to_bytes()?, "rejection is atomic");
        }
        if symbol.data_type_code == 1 {
            for rejected in ["%IX0.5.1", "%QX400"] {
                assert!(
                    edited
                        .update_iec_local_symbol_address(4, index, name, "", rejected)
                        .is_err()
                );
                assert_eq!(
                    before,
                    edited.to_bytes()?,
                    "automatic I/O mapping is atomic"
                );
            }
        }
        edited.update_iec_local_symbol_address(4, index, name, "", address)?;
    }
    let output = Path::new(&args[1]);
    edited.write_to(output.join("ATGEN.xgwx"))?;
    exact(
        &edited,
        &XgwxDocument::from_path(output.join("ATGEN.xgwx"))?,
    )?;
    for (index, (a, b)) in source
        .ladder_programs()
        .into_iter()
        .zip(edited.ladder_programs())
        .enumerate()
    {
        assert_eq!(
            a?.data, b?.data,
            "allocation edit preserves program {index}"
        );
    }
    if let Some(native) = args.get(2) {
        exact(&edited, &XgwxDocument::from_path(native)?)?;
        println!("PASS native all seven payloads and every local field including offsets");
    }
    println!("PASS automatic BOOL/INT/UDINT/TIME mappings and atomic guards");
    Ok(())
}
