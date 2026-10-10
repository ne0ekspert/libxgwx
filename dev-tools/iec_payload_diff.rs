//! Inspect bounded native IEC payload differences without dumping whole projects.
use std::{env, error::Error};
use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn Error>> {
    let a = XgwxDocument::from_path(env::args().nth(1).ok_or("source")?)?;
    let b = XgwxDocument::from_path(env::args().nth(2).ok_or("native")?)?;
    for (p, (a, b)) in a
        .ladder_programs()
        .into_iter()
        .zip(b.ladder_programs())
        .enumerate()
    {
        let (a, b) = (a?, b?);
        println!("program {p} {} -> {} bytes", a.data.len(), b.data.len());
        if a.data.len() != b.data.len() {
            continue;
        }
        let rows = b.iec_row_frames().unwrap();
        let records = b.iec_record_frames().unwrap();
        for (i, (x, y)) in a
            .data
            .iter()
            .zip(&b.data)
            .enumerate()
            .filter(|(_, (x, y))| x != y)
            .take(30)
        {
            println!(
                "{i} {x:02x}->{y:02x} row {:?} record {:?}",
                rows.iter()
                    .find(|r| i >= r.start && i < r.end)
                    .map(|r| (r.row_index, i - r.start)),
                records
                    .iter()
                    .find(|r| i >= r.offset && i < r.end)
                    .map(|r| (r.kind, r.row_index, i - r.offset))
            );
        }
    }
    Ok(())
}
