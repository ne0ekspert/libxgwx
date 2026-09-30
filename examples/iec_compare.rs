use std::env;
use std::error::Error;
use std::io;

use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let left_path = args.next().ok_or_else(usage)?;
    let right_path = args.next().ok_or_else(usage)?;
    let details = match args.next().as_deref() {
        None => false,
        Some("--details") => true,
        Some(_) => return Err(usage().into()),
    };
    if args.next().is_some() {
        return Err(usage().into());
    }

    let left = XgwxDocument::from_path(&left_path)?;
    let right = XgwxDocument::from_path(&right_path)?;
    let left_programs = left.ladder_programs();
    let right_programs = right.ladder_programs();
    if left_programs.len() != right_programs.len() {
        return Err(format!(
            "program count differs: {} versus {}",
            left_programs.len(),
            right_programs.len()
        )
        .into());
    }

    for (index, (left, right)) in left_programs.into_iter().zip(right_programs).enumerate() {
        let left = left?;
        let right = right?;
        let common_prefix = left
            .data
            .iter()
            .zip(&right.data)
            .take_while(|(left, right)| left == right)
            .count();
        let common_suffix = left.data[common_prefix..]
            .iter()
            .rev()
            .zip(right.data[common_prefix..].iter().rev())
            .take_while(|(left, right)| left == right)
            .count();
        let changed = left
            .data
            .iter()
            .zip(&right.data)
            .filter(|(left, right)| left != right)
            .count();
        println!(
            "program {index}: {} -> {} bytes; changed={changed}; common-prefix=0x{common_prefix:X}; common-suffix=0x{common_suffix:X}",
            left.data.len(),
            right.data.len(),
        );
        let left_rows_for_diff = details
            .then(|| left.iec_row_frames())
            .flatten()
            .unwrap_or_default();
        let left_records = details
            .then(|| left.iec_record_frames())
            .flatten()
            .unwrap_or_default();
        if left.data.len() == right.data.len() && (details || changed <= 64) {
            for (offset, (left, right)) in left.data.iter().zip(&right.data).enumerate() {
                if left != right {
                    let owner = left_records
                        .iter()
                        .find(|record| record.offset <= offset && offset < record.end)
                        .map(|record| {
                            format!(
                                " {:?} group {} L{} +0x{:X}",
                                record.kind,
                                record.group_index,
                                record.row_index,
                                offset - record.offset
                            )
                        })
                        .or_else(|| {
                            left_rows_for_diff
                                .iter()
                                .find(|row| row.start <= offset && offset < row.end)
                                .map(|row| {
                                    format!(
                                        " row-header group {} L{} +0x{:X}",
                                        row.group_index,
                                        row.row_index,
                                        offset - row.start
                                    )
                                })
                        })
                        .unwrap_or_default();
                    println!("  0x{offset:X}: {left:02X} -> {right:02X}{owner}");
                }
            }
        }
        if details {
            let left_rows = left.iec_row_frames().ok_or("left IEC row framing failed")?;
            let right_rows = right
                .iec_row_frames()
                .ok_or("right IEC row framing failed")?;
            println!("  rows: {} -> {}", left_rows.len(), right_rows.len());
            for (row_index, (left, right)) in left_rows.iter().zip(&right_rows).enumerate() {
                if left != right {
                    println!("  row[{row_index}]: {left:?} -> {right:?}");
                }
            }
            for (left_row, right_row) in left_rows.iter().zip(&right_rows) {
                let left_header = left.data.get(left_row.start..left_row.records_start);
                let right_header = right.data.get(right_row.start..right_row.records_start);
                if left_header != right_header {
                    println!(
                        "  row-header group {} L{}: {:02X?} -> {:02X?}",
                        left_row.group_index,
                        left_row.row_index,
                        left_header.unwrap_or_default(),
                        right_header.unwrap_or_default(),
                    );
                }
            }
        }
    }
    Ok(())
}

fn usage() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "usage: cargo run --example iec_compare -- <left.xgwx> <right.xgwx> [--details]",
    )
}
