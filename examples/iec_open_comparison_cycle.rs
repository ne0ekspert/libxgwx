//! Native checkpoints for comparison edits within a network retaining a wire gap.
use std::{env, error::Error};
use xgwx::{IecFunctionBlock, IecFunctionPinDirection, LadderProgramData, XgwxDocument};
fn operands(program: &LadderProgramData, block: &IecFunctionBlock) -> Option<Vec<String>> {
    let records = program.iec_record_frames()?;
    let links = program.iec_function_operand_links()?;
    let mut result = block
        .instance
        .iter()
        .map(|s| s.value.clone())
        .collect::<Vec<_>>();
    let mut pins = block.pins.iter().collect::<Vec<_>>();
    pins.sort_by_key(|pin| {
        (
            pin.direction == IecFunctionPinDirection::Output,
            pin.row_index,
            pin.raw_x,
        )
    });
    for pin in pins {
        let Some(link) = links.iter().find(|l| {
            l.target_record_offset == block.record_offset
                && Some(l.ordinal) == pin.reference_ordinal
        }) else {
            continue;
        };
        let record = records.iter().find(|r| r.offset == link.record_offset)?;
        // Use the framed expression record directly: the heuristic source
        // string inventory can omit short Unicode operands.
        let text = program.data.get(record.offset + 15..record.end)?;
        if text.get(..3)? != [0xff, 0xfe, 0xff] || text.len() != 4 + usize::from(*text.get(3)?) * 2
        {
            return None;
        }
        result.push(
            String::from_utf16(
                &text[4..]
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    .collect::<Vec<_>>(),
            )
            .ok()?,
        );
    }
    Some(result)
}

fn exact(a: &XgwxDocument, b: &XgwxDocument) -> Result<(), Box<dyn Error>> {
    let aa = a.ladder_programs();
    let bb = b.ladder_programs();
    assert_eq!(aa.len(), bb.len());
    for (i, (a, b)) in aa.into_iter().zip(bb).enumerate() {
        assert_eq!(a?.data, b?.data, "native payload {i}");
    }
    let aa = a.iec_local_symbols();
    let bb = b.iec_local_symbols();
    assert_eq!(aa.len(), bb.len());
    for (i, (a, b)) in aa.into_iter().zip(bb).enumerate() {
        assert_eq!(a?, b?, "all local fields including offsets {i}");
    }
    Ok(())
}

fn block(doc: &XgwxDocument, row: u16, x: u8) -> Result<IecFunctionBlock, Box<dyn Error>> {
    doc.ladder_programs()
        .remove(0)?
        .iec_function_blocks()
        .ok_or("invalid blocks")?
        .into_iter()
        .find(|b| b.row_index == row && b.raw_x == x)
        .ok_or("missing block".into())
}
fn main() -> Result<(), Box<dyn Error>> {
    let a = env::args().skip(1).collect::<Vec<_>>();
    if a.len() != 3 && a.len() != 5 {
        return Err("usage: iec_open_comparison_cycle SOURCE DELETED RESTORED [NATIVE_DELETED NATIVE_RESTORED]".into());
    }
    let source = XgwxDocument::from_path(&a[0])?;
    let program = source.ladder_programs().remove(0)?;
    let open = program
        .iec_circuit_layout()
        .ok_or("invalid source")?
        .open_branch_endpoints;
    assert!(!open.is_empty());
    let mut values = Vec::new();
    for (row, x) in [(67, 16), (71, 16), (75, 16)] {
        let b = block(&source, row, x)?;
        values.push((
            row,
            x,
            b.name.value.clone(),
            operands(&program, &b).ok_or("missing operands")?,
        ));
    }
    println!("Instruction operands: {values:?}");
    let mut deleted = source.clone();
    for (row, x, name, _) in &values {
        let b = block(&deleted, *row, *x)?;
        deleted.delete_iec_ld_branch_function(0, b.record_offset, name)?;
    }
    let moved = open.clone();
    assert_eq!(
        moved,
        deleted
            .ladder_programs()
            .remove(0)?
            .iec_circuit_layout()
            .unwrap()
            .open_branch_endpoints
    );
    deleted.write_to(&a[1])?;
    let mut restored = deleted.clone();
    for (row, x, name, args) in &values {
        restored.insert_iec_ld_function(0, *row, *x, name, args)?;
    }
    assert_eq!(
        open,
        restored
            .ladder_programs()
            .remove(0)?
            .iec_circuit_layout()
            .unwrap()
            .open_branch_endpoints
    );
    restored.write_to(&a[2])?;
    println!(
        "PASS three comparisons in an open network: deletions and refills preserve unrelated exposed endpoints"
    );
    if a.len() == 5 {
        exact(&deleted, &XgwxDocument::from_path(&a[3])?)?;
        exact(&restored, &XgwxDocument::from_path(&a[4])?)?;
        println!(
            "PASS both native Save As checkpoints: all seven payloads and every local field including offsets exact"
        );
    }
    Ok(())
}
