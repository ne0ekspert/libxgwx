//! Generate a typed terminal TON insertion into a retained contact rung.
use std::{env, error::Error};
use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn Error>> {
    let a = env::args().skip(1).collect::<Vec<_>>();
    if a.len() != 7 {
        return Err("usage: iec_terminal_timer_acceptance SOURCE OUTPUT PROGRAM ROW INSTANCE PRESET ELAPSED".into());
    }
    let mut doc = XgwxDocument::from_path(&a[0])?;
    let p = a[2].parse::<usize>()?;
    let row = a[3].parse::<u16>()?;
    let site = doc
        .program()
        .ladder_programs()
        .into_iter()
        .nth(p)
        .ok_or("missing program")??
        .iec_terminal_timer_insertion_sites()
        .ok_or("invalid layout")?
        .into_iter()
        .find(|s| s.row_index == row)
        .ok_or("missing insertion site")?;
    doc.program_mut().get(p).and_then(|mut selected_program| {
        selected_program
            .iec_ld()?
            .insert_terminal_timer(site.contact_offset, &a[4], &a[5], &a[6])
    })?;
    std::fs::write(&a[1], doc.to_bytes()?)?;
    Ok(())
}
