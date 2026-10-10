//! Delete a standalone function, including the last block left in a chain.
use std::{env, error::Error};
use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn Error>> {
    let a = env::args().skip(1).collect::<Vec<_>>();
    if !(5..=6).contains(&a.len()) {
        return Err(
            "usage: iec_standalone_function_acceptance SOURCE OUTPUT PROGRAM ROW EXPECTED [NATIVE]"
                .into(),
        );
    }
    let mut doc = XgwxDocument::from_path(&a[0])?;
    let p = a[2].parse::<usize>()?;
    let row = a[3].parse::<u16>()?;
    let b = doc
        .ladder_programs()
        .into_iter()
        .nth(p)
        .ok_or("missing program")??
        .iec_function_blocks()
        .ok_or("invalid functions")?
        .into_iter()
        .find(|b| b.row_index == row && b.name.value == a[4])
        .ok_or("missing target")?;
    doc.delete_iec_ld_standalone_function(p, b.record_offset, &a[4])?;
    std::fs::write(&a[1], doc.to_bytes()?)?;
    if let Some(native_path) = a.get(5) {
        let native = XgwxDocument::from_path(native_path)?;
        let generated = doc.ladder_programs();
        let captured = native.ladder_programs();
        assert_eq!(generated.len(), captured.len());
        for (i, (generated, captured)) in generated.into_iter().zip(captured).enumerate() {
            let (mut generated, mut captured) = (generated?, captured?);
            let differences = (0..generated.data.len().max(captured.data.len()))
                .filter(|&offset| generated.data.get(offset) != captured.data.get(offset))
                .collect::<Vec<_>>();
            let values = differences
                .iter()
                .map(|&offset| {
                    (
                        offset,
                        generated.data.get(offset).copied(),
                        captured.data.get(offset).copied(),
                    )
                })
                .collect::<Vec<_>>();
            println!("program {i}: raw differences {values:?}");
            for program in [&mut generated, &mut captured] {
                for row in program.iec_row_frames().ok_or("invalid row framing")? {
                    program.data[row.start + 17..row.start + 21].fill(0);
                }
            }
            if generated.data != captured.data {
                return Err(
                    format!("program {i}: payload differs beyond row display caches").into(),
                );
            }
        }
        println!("All program payloads match native Delete after masking only row display caches");
    }
    Ok(())
}
