//! Strict native acceptance for branch-row cleanup beside an unrelated wire gap.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn exact(a: &XgwxDocument, b: &XgwxDocument) -> Result<(), Box<dyn Error>> {
    let aa = a.ladder_programs();
    let bb = b.ladder_programs();
    assert_eq!(aa.len(), bb.len());
    for (i, (a, b)) in aa.into_iter().zip(bb).enumerate() {
        assert_eq!(a?.data, b?.data, "native program payload {i}");
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
    if args.len() != 3 && args.len() != 5 {
        return Err("usage: iec_open_branch_row_acceptance SOURCE FIRST SECOND [NATIVE_FIRST NATIVE_SECOND]".into());
    }
    let source = XgwxDocument::from_path(&args[0])?;
    let before = source.ladder_programs().remove(0)?;
    let rows = before.iec_row_frames().ok_or("invalid source rows")?;
    let original = before.iec_circuit_layout().ok_or("invalid source layout")?;
    assert_eq!(original.open_branch_endpoints.len(), 2);
    for (index, (group, start, end, x)) in [(3, 3, 4, 6), (22, 42, 43, 3)].into_iter().enumerate() {
        let mut edited = source.clone();
        edited.edit_iec_ld_branch_segment(0, group, start, end, x, true, false)?;
        let after = edited.ladder_programs().remove(0)?;
        assert_eq!(after.iec_row_frames().unwrap().len() + 1, rows.len());
        let mut endpoints = original.open_branch_endpoints.clone();
        for point in &mut endpoints {
            if point.row_index > end {
                point.row_index -= 1;
            }
        }
        let layout = after.iec_circuit_layout().ok_or("invalid result layout")?;
        assert_eq!(layout.open_branch_endpoints, endpoints);
        assert_eq!(
            layout.function_bindings.len(),
            original.function_bindings.len()
        );
        // Record offsets change when a row is removed; compare each binding's
        // semantic fields after the expected row translation. Native checks
        // below compare entire payloads and every local field with no allowance.
        let translate = |bindings: Vec<xgwx::IecFunctionBinding>, shift: bool| {
            bindings
                .into_iter()
                .map(|mut b| {
                    if shift && b.pin_point.row_index > end {
                        b.pin_point.row_index -= 1;
                    }
                    b.block_record_offset = 0;
                    b.reference_record_offset = 0;
                    b.expression_record_offset = None;
                    b
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            translate(original.function_bindings.clone(), true),
            translate(layout.function_bindings, false)
        );
        for (p, (a, b)) in source
            .ladder_programs()
            .into_iter()
            .zip(edited.ladder_programs())
            .enumerate()
            .skip(1)
        {
            assert_eq!(a?.data, b?.data, "unmodified program {p}");
        }
        edited.write_to(&args[index + 1])?;
        if let Some(native) = args.get(index + 3) {
            exact(&edited, &XgwxDocument::from_path(native)?)?;
            println!(
                "PASS checkpoint {index}: all program payloads and every local field exact against native Save As"
            );
        }
        println!(
            "PASS group {group}: removed row L{end}, preserved unrelated gap and translated all function pin bindings"
        );
    }
    Ok(())
}
