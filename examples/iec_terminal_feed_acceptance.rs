//! Generate and compare a native Delete Line on a terminal IEC contact feed.
use std::{collections::BTreeSet, env, error::Error};
use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output, program, group, start, end, x, native @ ..] = args.as_slice() else {
        return Err(
            "usage: iec_terminal_feed_acceptance SOURCE OUTPUT PROGRAM GROUP START END X [NATIVE]"
                .into(),
        );
    };
    if native.len() > 1 {
        return Err("at most one native comparison file is supported".into());
    }
    let p: usize = program.parse()?;
    let mut doc = XgwxDocument::from_path(source)?;
    doc.edit_iec_ld_branch_segment(
        p,
        group.parse()?,
        start.parse()?,
        end.parse()?,
        x.parse()?,
        true,
        false,
    )?;
    doc.write_to(output)?;
    if let Some(native) = native.first() {
        let native = XgwxDocument::from_path(native)?;
        let left = doc.ladder_programs();
        let right = native.ladder_programs();
        if left.len() != right.len() {
            return Err("program count mismatch".into());
        }
        for (index, (generated, native)) in left.into_iter().zip(right).enumerate() {
            let generated = generated?;
            let native = native?;
            let caches = if index == p {
                generated
                    .iec_row_frames()
                    .ok_or("invalid rows")?
                    .iter()
                    .map(|r| r.start + 17)
                    .collect::<BTreeSet<_>>()
            } else {
                BTreeSet::new()
            };
            let diffs = (0..generated.data.len().max(native.data.len()))
                .filter(|i| generated.data.get(*i) != native.data.get(*i))
                .collect::<Vec<_>>();
            if generated.data.len() != native.data.len()
                || diffs.iter().any(|i| !caches.contains(i))
            {
                return Err(format!("program {index}: structural differences {diffs:?}").into());
            }
            println!("program {index}: only native row-height cache changes {diffs:?}");
        }
    }
    println!("PASS terminal IEC feed deletion");
    Ok(())
}
