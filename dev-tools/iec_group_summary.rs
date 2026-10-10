//! Summarize stored rows and record kinds in one IEC group.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, program_index, group_index] = args.as_slice() else {
        return Err("usage: iec_group_summary SOURCE PROGRAM GROUP".into());
    };
    let document = XgwxDocument::from_path(source)?;
    let program = document.ladder_programs().remove(program_index.parse()?)?;
    let group_index = group_index.parse::<usize>()?;
    let rows = program.iec_row_frames().ok_or("invalid IEC rows")?;
    let records = program.iec_record_frames().ok_or("invalid IEC records")?;
    let geometry = program.iec_geometry().ok_or("invalid IEC geometry")?;
    for row in rows
        .into_iter()
        .filter(|row| row.group_index == group_index)
    {
        println!("  header {:?}", &program.data[row.start..row.records_start]);
        let kinds = records
            .iter()
            .filter(|record| record.group_index == group_index && record.row_index == row.row_index)
            .map(|record| {
                format!(
                    "{:?}@{}[{}..{}]",
                    record.kind,
                    program.data[record.offset + 5],
                    record.offset,
                    record.end
                )
            })
            .collect::<Vec<_>>();
        println!("L{}: {}", row.row_index, kinds.join(" "));
    }
    for branch in geometry
        .vertical
        .into_iter()
        .filter(|branch| branch.group_index == group_index)
    {
        println!(
            "branch L{}-L{} x{}",
            branch.start_row_index, branch.end_row_index, branch.x
        );
    }
    Ok(())
}
