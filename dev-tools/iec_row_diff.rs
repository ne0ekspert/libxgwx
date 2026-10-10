//! Compare framed IEC rows across edits without offset drift hiding differences.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [before, after, index] = args.as_slice() else {
        return Err("usage: iec_row_diff BEFORE AFTER PROGRAM".into());
    };
    let index: usize = index.parse()?;
    let a = XgwxDocument::from_path(before)?
        .ladder_programs()
        .remove(index)?;
    let b = XgwxDocument::from_path(after)?
        .ladder_programs()
        .remove(index)?;
    let ar = a.iec_row_frames().ok_or("invalid before rows")?;
    let br = b.iec_row_frames().ok_or("invalid after rows")?;
    println!(
        "program {index}: {} -> {} bytes",
        a.data.len(),
        b.data.len()
    );
    for row in &ar {
        let Some(other) = br.iter().find(|r| r.row_index == row.row_index) else {
            println!("L{} removed", row.row_index);
            continue;
        };
        let x = &a.data[row.records_start..row.end];
        let y = &b.data[other.records_start..other.end];
        if x != y {
            println!(
                "L{} records: {} -> {} bytes, {} -> {} records",
                row.row_index,
                x.len(),
                y.len(),
                row.record_count,
                other.record_count
            );
            if x.len() == y.len() {
                let diffs = x
                    .iter()
                    .zip(y)
                    .enumerate()
                    .filter(|(_, (x, y))| x != y)
                    .take(20)
                    .collect::<Vec<_>>();
                println!("  {diffs:?}");
            }
        }
        let x = &a.data[row.start..row.records_start];
        let y = &b.data[other.start..other.records_start];
        if x != y {
            let diffs = x
                .iter()
                .zip(y)
                .enumerate()
                .filter(|(_, (x, y))| x != y)
                .collect::<Vec<_>>();
            println!("L{} header: {diffs:?}", row.row_index);
        }
    }
    for row in &br {
        if !ar.iter().any(|r| r.row_index == row.row_index) {
            println!("L{} added ({} records)", row.row_index, row.record_count);
        }
    }
    Ok(())
}
