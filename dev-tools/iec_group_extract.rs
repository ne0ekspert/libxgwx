//! Extract one native IEC group as a standalone record template.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, program_index, group_index, output] = args.as_slice() else {
        return Err("usage: iec_group_extract <source> <program> <group> <output>".into());
    };
    let document = XgwxDocument::from_path(source)?;
    let program = document
        .ladder_programs()
        .into_iter()
        .nth(program_index.parse()?)
        .ok_or("program index out of range")??;
    let group_index = group_index.parse::<usize>()?;
    let rows = program.iec_row_frames().ok_or("invalid IEC rows")?;
    let group = rows
        .iter()
        .filter(|row| row.group_index == group_index)
        .collect::<Vec<_>>();
    let first = group.first().ok_or("group index out of range")?;
    let last = group.last().unwrap();
    let bytes = &program.data[first.start - 10..last.end];
    fs::write(output, bytes)?;
    println!("wrote {} bytes", bytes.len());
    Ok(())
}
