//! Copy and move a selected incomplete four-comparison network.
use std::{env, error::Error, path::Path};
use xgwx::XgwxDocument;
fn exact(a: &XgwxDocument, b: &XgwxDocument) -> Result<(), Box<dyn Error>> {
    let aa = a.ladder_programs();
    let bb = b.ladder_programs();
    assert_eq!(aa.len(), bb.len());
    for (i, (a, b)) in aa.into_iter().zip(bb).enumerate() {
        assert_eq!(a?.data, b?.data, "all native program bytes {i}");
    }
    let aa = a.iec_local_symbols();
    let bb = b.iec_local_symbols();
    assert_eq!(aa.len(), bb.len());
    for (i, (a, b)) in aa.into_iter().zip(bb).enumerate() {
        assert_eq!(a?, b?, "every native local field including offsets {i}");
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 && args.len() != 3 {
        return Err("SOURCE OUTPUT_DIR [NATIVE_DIR]".into());
    }
    let original = XgwxDocument::from_path(&args[0])?;
    let mut source = original.clone();
    for (group, row) in [
        (21, 41),
        (20, 37),
        (19, 36),
        (18, 32),
        (17, 31),
        (16, 26),
        (15, 25),
        (14, 20),
    ] {
        source.delete_iec_ld_group(0, group, row)?;
    }
    std::fs::create_dir_all(&args[1])?;
    source.write_to(Path::new(&args[1]).join("INPRE.xgwx"))?;
    let before = source.ladder_programs().remove(0)?;
    let endpoints = before
        .iec_circuit_layout()
        .ok_or("source layout")?
        .open_branch_endpoints;
    assert_eq!(endpoints.len(), 2);
    assert!(endpoints.iter().all(|p| p.group_index == 25));
    for (stem, copy) in [("INC", true), ("INM", false)] {
        let mut edited = source.clone();
        if copy {
            edited.copy_iec_ld_group(0, 25, 67, 20)?;
        } else {
            edited.move_iec_ld_group(0, 25, 67, 20)?;
        }
        let after = edited.ladder_programs().remove(0)?;
        let mut expected = endpoints.clone();
        for p in &mut expected {
            p.group_index = 14;
            p.row_index -= 47;
        }
        if copy {
            expected.extend(endpoints.iter().map(|p| {
                let mut p = *p;
                p.group_index += 1;
                p
            }));
        }
        expected.sort_unstable();
        let mut actual = after
            .iec_circuit_layout()
            .ok_or("result layout")?
            .open_branch_endpoints;
        actual.sort_unstable();
        assert_eq!(actual, expected, "{stem}: exact endpoint translation");
        assert_eq!(
            after.iec_function_blocks().ok_or("blocks")?.len(),
            before.iec_function_blocks().unwrap().len() + if copy { 4 } else { 0 }
        );
        let mut restored = edited.clone();
        if copy {
            restored.delete_iec_ld_group(0, 14, 20)?;
        } else {
            restored.move_iec_ld_group(0, 14, 20, 67)?;
        }
        exact(&restored, &source)?;
        for (i, (a, b)) in original
            .ladder_programs()
            .into_iter()
            .zip(edited.ladder_programs())
            .enumerate()
            .skip(1)
        {
            assert_eq!(a?.data, b?.data, "untouched program {i}");
        }
        let path = Path::new(&args[1]).join(format!("{stem}GEN.xgwx"));
        edited.write_to(&path)?;
        exact(&edited, &XgwxDocument::from_path(path)?)?;
        if let Some(native) = args.get(2) {
            exact(
                &edited,
                &XgwxDocument::from_path(Path::new(native).join(format!("{stem}GS.xgwx")))?,
            )?;
            println!("PASS {stem}: native all seven payloads and every local field exact");
        }
        println!(
            "PASS {stem}: selected gap translated exactly; inverse restores all payloads and declarations"
        );
    }
    let mut cross_source = source.clone();
    let target = cross_source.ladder_programs().remove(6)?;
    let mut starts = std::collections::BTreeMap::new();
    for row in target.iec_row_frames().ok_or("target rows")? {
        starts.entry(row.group_index).or_insert(row.row_index);
    }
    for (group, row) in starts.into_iter().rev().filter(|(_, row)| *row < 16) {
        cross_source.delete_iec_ld_group(6, group, row)?;
    }
    cross_source.write_to(Path::new(&args[1]).join("INXPRE.xgwx"))?;
    let mut cross = cross_source.clone();
    cross.copy_iec_ld_group_to_program_with_locals(0, 25, 67, 6, 0)?;
    let after = cross.ladder_programs().remove(6)?;
    let mut expected = endpoints.clone();
    for point in &mut expected {
        point.group_index = 0;
        point.row_index -= 67;
    }
    expected.sort_unstable();
    let mut actual = after
        .iec_circuit_layout()
        .ok_or("cross result")?
        .open_branch_endpoints;
    actual.sort_unstable();
    assert_eq!(expected, actual, "cross-program gap endpoints");
    for (i, (a, b)) in cross_source
        .ladder_programs()
        .into_iter()
        .zip(cross.ladder_programs())
        .enumerate()
        .filter(|(i, _)| *i != 6)
    {
        assert_eq!(a?.data, b?.data, "cross untouched program {i}");
    }
    cross.write_to(Path::new(&args[1]).join("INXGEN.xgwx"))?;
    if let Some(native) = args.get(2) {
        exact(
            &cross,
            &XgwxDocument::from_path(Path::new(native).join("INXGS.xgwx"))?,
        )?;
        println!("PASS INX: native all seven payloads and every local field exact");
    }
    println!(
        "PASS INX: selected incomplete network copied across programs with local declarations"
    );
    Ok(())
}
