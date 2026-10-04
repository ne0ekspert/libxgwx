//! Whole-network edits preserve other networks, including existing wiring gaps.
use std::{env, error::Error, path::Path};
use xgwx::XgwxDocument;

fn exact(a: &XgwxDocument, b: &XgwxDocument) -> Result<(), Box<dyn Error>> {
    let aa = a.ladder_programs();
    let bb = b.ladder_programs();
    assert_eq!(aa.len(), bb.len());
    for (i, (a, b)) in aa.into_iter().zip(bb).enumerate() {
        assert_eq!(a?.data, b?.data, "program payload {i}");
    }
    let aa = a.iec_local_symbols();
    let bb = b.iec_local_symbols();
    assert_eq!(aa.len(), bb.len());
    for (i, (a, b)) in aa.into_iter().zip(bb).enumerate() {
        assert_eq!(a?, b?, "every local field including offsets {i}");
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 && args.len() != 3 {
        return Err("usage: iec_open_network_acceptance SOURCE OUTPUT_DIR [NATIVE_DIR]".into());
    }
    let source = XgwxDocument::from_path(&args[0])?;
    let before = source.ladder_programs().remove(0)?;
    let original = before.iec_circuit_layout().ok_or("source layout")?;
    assert_eq!(original.open_branch_endpoints.len(), 2);
    let mut rejected = source.clone();
    let unchanged = rejected.to_bytes()?;
    assert!(rejected.copy_iec_ld_group(0, 33, 67, 3).is_err());
    assert_eq!(
        rejected.to_bytes()?,
        unchanged,
        "rejected copy into occupied rows is atomic"
    );
    assert!(rejected.move_iec_ld_group(0, 33, 67, 3).is_err());
    assert_eq!(
        rejected.to_bytes()?,
        unchanged,
        "rejected move into occupied rows is atomic"
    );
    println!(
        "stored extent {}",
        u16::from_le_bytes(before.data[4..6].try_into()?)
    );
    std::fs::create_dir_all(&args[1])?;
    for (name, operation) in ["OND", "ONC", "ONM", "ONR", "ONF", "ONX"]
        .into_iter()
        .enumerate()
    {
        let stem = operation;
        let mut edited = source.clone();
        match name {
            0 => edited.delete_iec_ld_group(0, 3, 3)?,
            1 => {
                edited.delete_iec_ld_group(0, 3, 3)?;
                edited.copy_iec_ld_group(0, 4, 7, 3)?;
            }
            2 => {
                edited.delete_iec_ld_group(0, 3, 3)?;
                edited.move_iec_ld_group(0, 4, 7, 3)?;
            }
            3 => edited.replace_iec_ld_group(0, 22, 42, 3, 3)?,
            4 => {
                edited.delete_iec_ld_group(0, 3, 3)?;
                edited.copy_iec_ld_group(0, 25, 48, 3)?;
            }
            5 => {
                edited.delete_iec_ld_group(6, 2, 2)?;
                edited.delete_iec_ld_group(6, 1, 1)?;
                edited.delete_iec_ld_group(6, 0, 0)?;
                edited.copy_iec_ld_group_to_program_with_locals(0, 26, 48, 6, 0)?;
            }
            _ => unreachable!(),
        }
        if name == 1 || name == 2 || name == 4 {
            let mut restored = edited.clone();
            if name == 2 {
                restored.move_iec_ld_group(0, 3, 3, 7)?;
            } else {
                restored.delete_iec_ld_group(0, 3, 3)?;
            }
            let mut expected = source.clone();
            expected.delete_iec_ld_group(0, 3, 3)?;
            exact(&restored, &expected)?;
        }
        let after = edited.ladder_programs().remove(0)?;
        let layout = after.iec_circuit_layout().ok_or("result layout")?;
        let mut endpoints = original.open_branch_endpoints.clone();
        let shift = if name == 0 || name == 2 { 1 } else { 0 };
        for point in &mut endpoints {
            point.group_index -= shift;
        }
        assert_eq!(
            layout.open_branch_endpoints, endpoints,
            "{stem}: untouched gap"
        );
        for (p, (a, b)) in source
            .ladder_programs()
            .into_iter()
            .zip(edited.ladder_programs())
            .enumerate()
            .filter(|(p, _)| if name == 5 { *p != 6 } else { *p != 0 })
        {
            assert_eq!(a?.data, b?.data, "{stem}: unmodified program {p}");
        }
        let output = Path::new(&args[1]).join(format!("{stem}GEN.xgwx"));
        edited.write_to(&output)?;
        exact(&edited, &XgwxDocument::from_path(&output)?)?;
        if let Some(native) = args.get(2) {
            exact(
                &edited,
                &XgwxDocument::from_path(Path::new(native).join(format!("{stem}GS.xgwx")))?,
            )?;
            println!("PASS {stem}: native all seven payloads and every local field exact");
        }
        println!("PASS {stem}: generated, reparsed, preserved unrelated gap and six programs");
    }
    Ok(())
}
