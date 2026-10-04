//! Native acceptance for WORD, DINT and REAL local memory mappings.
use std::{error::Error, path::Path};
use xgwx::XgwxDocument;
fn exact(a: &XgwxDocument, b: &XgwxDocument) -> Result<(), Box<dyn Error>> {
    assert_eq!(a.ladder_programs().len(), 7);
    assert_eq!(b.ladder_programs().len(), 7);
    for (i, (a, b)) in a
        .ladder_programs()
        .into_iter()
        .zip(b.ladder_programs())
        .enumerate()
    {
        assert_eq!(a?.data, b?.data, "program payload {i}");
    }
    assert_eq!(a.iec_local_symbols().len(), b.iec_local_symbols().len());
    for (i, (a, b)) in a
        .iec_local_symbols()
        .into_iter()
        .zip(b.iec_local_symbols())
        .enumerate()
    {
        assert_eq!(a?, b?, "all local fields including offsets {i}");
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let a = std::env::args().skip(1).collect::<Vec<_>>();
    let source = XgwxDocument::from_path(&a[0])?;
    let output = Path::new(&a[1]);
    if a.get(2).is_some_and(|mode| mode == "prepare") {
        let mut d = source;
        for (name, ty) in [
            ("ADDR_DINT", "DINT"),
            ("ADDR_REAL", "REAL"),
            ("ADDR_WORD", "WORD"),
        ] {
            d.insert_iec_local_symbol(0, name, ty, "")?;
        }
        d.write_to(output.join("ADPRE.xgwx"))?;
        return Ok(());
    }
    let cases = [
        ("ADDR_DINT", "%MD1000", "%MD1001"),
        ("ADDR_REAL", "%MD1010", "%MD1011"),
        ("ADDR_WORD", "%MW2200", "%MW2202"),
    ];
    let mut base = source.clone();
    for (name, address, _) in cases {
        let index = base
            .iec_local_symbols()
            .remove(0)?
            .iter()
            .position(|s| s.name == name)
            .ok_or("missing capture symbol")?;
        base.update_iec_local_symbol_address(0, index, name, address, "")?;
    }
    base.write_to(output.join("ADBASE.xgwx"))?;
    let mut recreated = base.clone();
    for (name, address, _) in cases {
        let index = recreated
            .iec_local_symbols()
            .remove(0)?
            .iter()
            .position(|s| s.name == name)
            .unwrap();
        recreated.update_iec_local_symbol_address(0, index, name, "", address)?;
    }
    exact(&recreated, &source)?;
    recreated.write_to(output.join("ADREGEN.xgwx"))?;
    println!("PASS assign and clear recreate all captured fields exactly");
    let mut edited = source.clone();
    for (name, address, replacement) in cases {
        let index = edited
            .iec_local_symbols()
            .remove(0)?
            .iter()
            .position(|s| s.name == name)
            .unwrap();
        edited.update_iec_local_symbol_address(0, index, name, address, replacement)?;
    }
    let bytes = edited.to_bytes()?;
    let word_index = edited
        .iec_local_symbols()
        .remove(0)?
        .iter()
        .position(|s| s.name == "ADDR_WORD")
        .unwrap();
    for rejected in ["%MW2002", "%MD2202", "%MW+2203", "%MW4294967295"] {
        assert!(
            edited
                .update_iec_local_symbol_address(0, word_index, "ADDR_WORD", "%MW2202", rejected)
                .is_err(),
            "must reject {rejected}"
        );
        assert_eq!(bytes, edited.to_bytes()?, "rejection must be atomic");
    }
    edited.write_to(output.join("ADGEN.xgwx"))?;
    exact(
        &edited,
        &XgwxDocument::from_path(output.join("ADGEN.xgwx"))?,
    )?;
    if let Some(native) = a.get(2) {
        exact(&edited, &XgwxDocument::from_path(native)?)?;
        println!("PASS native all seven program payloads and every local field exact");
    }
    println!("PASS remapping and overlap/type/overflow guards");
    Ok(())
}
