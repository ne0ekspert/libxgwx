//! Diagnose comparison refill after the native chain deletion closes one row.
//! Generated candidates require native acceptance before enabling a writer.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 8 {
        return Err(
            "usage: iec_chain_refill_probe DELETED OUTPUT PROGRAM ROW X IN1 IN2 DESTINATION".into(),
        );
    }
    let mut doc = XgwxDocument::from_path(&args[0])?;
    let p = args[2].parse()?;
    let row: u16 = args[3].parse()?;
    let x = args[4].parse()?;
    doc.insert_iec_ld_blank_row(p, row + 2)?;
    std::fs::write(&args[1], doc.to_bytes()?)?;
    println!("blank row restored; insertion candidate saved");
    let result = doc.insert_iec_ld_function(p, row, x, "EQ", &args[5..8]);
    println!("refill: {result:?}");
    if result.is_ok() {
        std::fs::write(format!("{}.refill.xgwx", args[1]), doc.to_bytes()?)?;
    }
    Ok(())
}
