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
    if args.len() != 2 && args.len() != 6 {
        return Err("usage: iec_open_feed_acceptance SOURCE OUTPUT_DIR [NATIVE_AF NATIVE_AT NATIVE_BF NATIVE_BT]".into());
    }
    let source = XgwxDocument::from_path(&args[0])?;
    let directory = std::path::Path::new(&args[1]);
    for (index, (name, p, gap_group, gap_start, gap_end, gap_x, group, start, end, x)) in [
        ("OFA", 2, 7, 19, 20, 21, 8, 28, 29, 18),
        ("OFB", 3, 24, 71, 72, 18, 23, 66, 67, 21),
    ]
    .into_iter()
    .enumerate()
    {
        let mut edited = source.clone();
        edited.edit_iec_ld_vertical_wire(p, gap_group, gap_start, gap_end, gap_x, true, false)?;
        edited.write_to(directory.join(format!("{name}SRC.xgwx")))?;
        let before = edited.ladder_programs().remove(p)?;
        let original = before.iec_circuit_layout().ok_or("invalid source layout")?;
        assert_eq!(original.open_branch_endpoints.len(), 2);
        assert!(
            original
                .open_branch_endpoints
                .iter()
                .all(|point| point.group_index == gap_group)
        );
        edited.edit_iec_ld_branch_segment(p, group, start, end, x, true, false)?;
        let intermediate = edited.ladder_programs().remove(p)?;
        let layout = intermediate
            .iec_circuit_layout()
            .ok_or("invalid feed layout")?;
        let mut endpoints = original.open_branch_endpoints.clone();
        for point in &mut endpoints {
            if point.row_index > end {
                point.row_index -= 1;
            }
        }
        let retained_endpoints = endpoints.clone();
        endpoints.push(xgwx::IecCircuitPoint {
            group_index: group,
            row_index: start,
            x,
        });
        endpoints.sort_by_key(|p| (p.group_index, p.row_index, p.x));
        assert_eq!(layout.open_branch_endpoints, endpoints);
        assert_eq!(
            intermediate.iec_row_frames().unwrap().len() + 1,
            before.iec_row_frames().unwrap().len()
        );
        edited.write_to(directory.join(format!("{name}FGEN.xgwx")))?;
        if let Some(native) = args.get(2 + index * 2) {
            exact(&edited, &XgwxDocument::from_path(native)?)?;
            println!(
                "PASS {name} feed native Save As: all seven payloads and every local field including offsets exact"
            );
        }
        let tail = intermediate
            .iec_geometry()
            .unwrap()
            .vertical
            .into_iter()
            .find(|b| b.group_index == group && b.end_row_index == start && b.x == x)
            .ok_or("missing tail")?;
        edited.edit_iec_ld_branch_segment(
            p,
            group,
            tail.start_row_index,
            tail.end_row_index,
            x,
            true,
            false,
        )?;
        let after = edited.ladder_programs().remove(p)?;
        let layout = after.iec_circuit_layout().ok_or("invalid tail layout")?;
        assert_eq!(layout.open_branch_endpoints, retained_endpoints);
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
            translate(original.function_bindings, true),
            translate(layout.function_bindings, false)
        );
        for (i, (a, b)) in source
            .ladder_programs()
            .into_iter()
            .zip(edited.ladder_programs())
            .enumerate()
        {
            if i != p {
                assert_eq!(a?.data, b?.data, "unmodified program {i}");
            }
        }
        edited.write_to(directory.join(format!("{name}TGEN.xgwx")))?;
        if let Some(native) = args.get(3 + index * 2) {
            exact(&edited, &XgwxDocument::from_path(native)?)?;
            println!(
                "PASS {name} tail native Save As: all seven payloads and every local field including offsets exact"
            );
        }
        // Closing the unrelated gap must produce exactly the cleanup of the
        // original closed network: no masked native error may hide a different
        // edited circuit. Compare every payload and every local field.
        let mut closed = edited.clone();
        closed.edit_iec_ld_vertical_wire(
            p,
            gap_group,
            gap_start - u16::from(gap_start > end),
            gap_end - u16::from(gap_end > end),
            gap_x,
            false,
            true,
        )?;
        assert!(
            closed
                .ladder_programs()
                .remove(p)?
                .iec_circuit_graph()
                .is_some()
        );
        let mut expected = source.clone();
        expected.edit_iec_ld_branch_segment(p, group, start, end, x, true, false)?;
        expected.edit_iec_ld_branch_segment(
            p,
            group,
            tail.start_row_index,
            tail.end_row_index,
            x,
            true,
            false,
        )?;
        exact(&closed, &expected)?;
        println!(
            "PASS {name}: feed and tail cleanup, every translated binding, and exact commutativity with unrelated gap refill"
        );
    }
    Ok(())
}
