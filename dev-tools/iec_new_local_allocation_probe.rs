//! Capture native allocation when newly declared locals become LD operands.
use std::{error::Error, path::Path};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !(2..=3).contains(&args.len()) {
        return Err("usage: iec_new_local_allocation_probe SOURCE OUTPUT [NATIVE]".into());
    }
    let source = XgwxDocument::from_path(&args[0])?;
    let mut edited = source.clone();
    let program = source.ladder_programs().remove(6)?;
    let row = u16::from_le_bytes(program.data[4..6].try_into()?);
    println!(
        "new BOOL rung L{row}; INT MOVE L{}",
        row.checked_add(4).ok_or("row overflow")?
    );
    edited.insert_iec_local_symbol(6, "ZZZ_NEW_BOOL", "BOOL", "")?;
    edited.insert_iec_local_symbol(6, "ZZZ_NEW_INT", "INT", "")?;
    edited.insert_iec_ld_linear_rung(6, row, "%MX900", "ZZZ_NEW_BOOL")?;
    edited.insert_iec_ld_function(
        6,
        row.checked_add(4).ok_or("row overflow")?,
        4,
        "MOVE",
        &["1".into(), "ZZZ_NEW_INT".into()],
    )?;
    edited.write_to(Path::new(&args[1]))?;
    for (index, (before, after)) in source
        .ladder_programs()
        .into_iter()
        .zip(edited.ladder_programs())
        .enumerate()
    {
        let (before, after) = (before?, after?);
        if index != 6 {
            assert_eq!(before.data, after.data, "untouched program {index}");
        }
    }
    for (index, (before, after)) in source
        .iec_local_symbols()
        .into_iter()
        .zip(edited.iec_local_symbols())
        .enumerate()
    {
        let (before, after) = (before?, after?);
        if index != 6 {
            assert_eq!(before, after, "untouched local table {index}");
        } else {
            assert_eq!(before.len() + 2, after.len());
            let shift = after
                .iter()
                .find(|item| item.name == before[0].name)
                .unwrap()
                .record_offset;
            for old in &before {
                let mut expected = old.clone();
                expected.record_offset += shift;
                assert_eq!(
                    &expected,
                    after.iter().find(|item| item.name == old.name).unwrap(),
                    "existing declaration and insertion offset shift"
                );
            }
            for symbol in after
                .iter()
                .filter(|item| item.name.starts_with("ZZZ_NEW_"))
            {
                assert_eq!(symbol.storage_class, "");
                assert_eq!(symbol.allocation_number, None);
                assert_eq!(symbol.allocation_width, None);
            }
        }
    }
    if let Some(native) = args.get(2) {
        let native = XgwxDocument::from_path(native)?;
        assert_eq!(edited.ladder_programs().len(), 7);
        assert_eq!(native.ladder_programs().len(), 7);
        assert_eq!(edited.iec_local_symbols().len(), 7);
        assert_eq!(native.iec_local_symbols().len(), 7);
        for (index, (a, b)) in edited
            .ladder_programs()
            .into_iter()
            .zip(native.ladder_programs())
            .enumerate()
        {
            assert_eq!(a?.data, b?.data, "native program payload {index}");
        }
        for (index, (a, b)) in edited
            .iec_local_symbols()
            .into_iter()
            .zip(native.iec_local_symbols())
            .enumerate()
        {
            let (a, b) = (a?, b?);
            assert_eq!(a.len(), b.len(), "native local count {index}");
            assert_eq!(a, b, "native every local field including offsets {index}");
        }
        println!(
            "PASS native all seven payloads and every local field including offsets; new BOOL/INT locals remain unallocated"
        );
    }
    println!("PASS generated new-local probe; native allocation requires separate capture");
    Ok(())
}
