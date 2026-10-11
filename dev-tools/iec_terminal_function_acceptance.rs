//! Generate a terminal function deletion and optionally compare a native save.
use std::{env, error::Error};
use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn Error>> {
    let a = env::args().skip(1).collect::<Vec<_>>();
    if a.len() != 6 {
        return Err("usage: iec_terminal_function_acceptance SOURCE OUTPUT PROGRAM ROW EXPECTED NATIVE_OR_DASH".into());
    }
    let mut doc = XgwxDocument::from_path(&a[0])?;
    let p = a[2].parse::<usize>()?;
    let row = a[3].parse::<u16>()?;
    let block = doc
        .program()
        .ladder_programs()
        .into_iter()
        .nth(p)
        .ok_or("missing program")??
        .iec_function_blocks()
        .ok_or("invalid blocks")?
        .into_iter()
        .find(|b| b.row_index == row && b.name.value == a[4])
        .ok_or("missing block")?;
    doc.program_mut().get(p).and_then(|mut selected_program| {
        selected_program
            .iec_ld()?
            .delete_terminal_function(block.record_offset, &a[4])
    })?;
    std::fs::write(&a[1], doc.to_bytes()?)?;
    if a[5] != "-" {
        let native = XgwxDocument::from_path(&a[5])?;
        let generated = doc.program().ladder_programs();
        let native = native.program().ladder_programs();
        if generated.len() != native.len() {
            return Err("program count changed".into());
        }
        for (i, (generated, native)) in generated.into_iter().zip(native).enumerate() {
            let (mut generated, mut native) = (generated?, native?);
            for program in [&mut generated, &mut native] {
                for row in program.iec_row_frames().ok_or("invalid rows")? {
                    program.data[row.start + 17..row.start + 21].fill(0);
                }
            }
            assert_eq!(generated.data, native.data, "program {i}");
        }
        println!("All program payloads match native deletion after masking row display caches");
    }
    Ok(())
}
