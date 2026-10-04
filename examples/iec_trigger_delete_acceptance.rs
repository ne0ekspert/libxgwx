//! Generate a connected trigger deletion and its native F5 gap repair.
use std::{collections::BTreeSet, env, error::Error};
use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, deleted, repaired, program, row, native @ ..] = args.as_slice() else {
        return Err(
            "usage: iec_trigger_delete_acceptance SOURCE DELETED REPAIRED PROGRAM ROW [NATIVE_DELETED NATIVE_REPAIRED]".into(),
        );
    };
    if !native.is_empty() && native.len() != 2 {
        return Err("provide both native files".into());
    }
    let p: usize = program.parse()?;
    let row: u16 = row.parse()?;
    let mut doc = XgwxDocument::from_path(source)?;
    let block = doc
        .ladder_programs()
        .remove(p)?
        .iec_function_blocks()
        .ok_or("invalid functions")?
        .into_iter()
        .find(|b| b.name.value == "R_TRIG" && b.row_index == row)
        .ok_or("missing trigger")?;
    doc.delete_iec_ld_function_cell(p, block.record_offset, "R_TRIG")?;
    doc.write_to(deleted)?;
    if let Ok(restored_output) = env::var("LIBXGWX_TRIGGER_RESTORED_OUTPUT") {
        let mut restored = doc.clone();
        let site = restored
            .ladder_programs()
            .remove(p)?
            .iec_function_cell_insertion_sites()
            .ok_or("invalid insertion sites")?
            .into_iter()
            .find(|s| s.function_name == "R_TRIG" && s.row_index == row && s.raw_x == block.raw_x)
            .ok_or("missing restoration site")?;
        restored.insert_iec_ld_function_cell(
            p,
            site.insertion_offset,
            "R_TRIG",
            &block.instance.as_ref().ok_or("missing instance")?.value,
        )?;
        restored.write_to(restored_output)?;
    }

    let gap = doc
        .ladder_programs()
        .remove(p)?
        .iec_horizontal_wire_repair_sites()
        .ok_or("invalid gaps")?
        .into_iter()
        .find(|g| g.row_index == row && g.raw_x == block.raw_x)
        .ok_or("missing trigger gap")?;
    doc.repair_iec_ld_horizontal_wire(p, gap.insertion_offset, gap.raw_x)?;
    doc.write_to(repaired)?;
    for (generated, captured) in [deleted, repaired].iter().zip(native) {
        let generated = XgwxDocument::from_path(generated)?;
        let captured = XgwxDocument::from_path(captured)?;
        let left = generated.ladder_programs();
        let right = captured.ladder_programs();
        if left.len() != right.len() {
            return Err("program count mismatch".into());
        }
        for (index, (left, right)) in left.into_iter().zip(right).enumerate() {
            let left = left?;
            let right = right?;
            let caches = if index == p {
                left.iec_row_frames()
                    .ok_or("invalid rows")?
                    .iter()
                    .map(|r| r.start + 17)
                    .collect::<BTreeSet<_>>()
            } else {
                BTreeSet::new()
            };
            let diffs = (0..left.data.len().max(right.data.len()))
                .filter(|i| left.data.get(*i) != right.data.get(*i))
                .collect::<Vec<_>>();
            if left.data.len() != right.data.len() || diffs.iter().any(|i| !caches.contains(i)) {
                return Err(format!("program {index}: structural differences {diffs:?}").into());
            }
            println!("program {index}: only row-height cache differences {diffs:?}");
        }
    }
    Ok(())
}
