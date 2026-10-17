//! Extract isolated native IEC prototypes and their factual pin metadata.
use std::{env, error::Error, fs, path::Path};
use xgwx::*;
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let d = XgwxDocument::from_path(&args[0])?;
    let p = d.program().ladder_programs().remove(0)?;
    let rows = p.iec_row_frames().ok_or("rows")?;
    let records = p.iec_record_frames().ok_or("records")?;
    let blocks = match p.iec_function_blocks() {
        Some(b) => b,
        None => {
            for r in records
                .iter()
                .filter(|r| r.kind == IecRecordKind::FunctionBlock)
            {
                let rs = rows
                    .iter()
                    .filter(|row| row.group_index == r.group_index)
                    .collect::<Vec<_>>();
                let mut one = p.clone();
                one.data = p.data[..8].to_vec();
                one.data[6..8].copy_from_slice(&1u16.to_le_bytes());
                one.data
                    .extend_from_slice(&p.data[rs[0].start - 10..rs.last().unwrap().end]);
                one.data[8..12].fill(0);
                if one.iec_function_blocks().is_none() {
                    println!(
                        "bad block row={} bytes={:02x?}",
                        r.row_index,
                        &p.data[r.offset..r.end]
                    );
                }
            }
            return Err("blocks".into());
        }
    };
    fs::create_dir_all(&args[1])?;
    for b in blocks.iter().filter(|b| b.row_index >= 30) {
        let group_rows = rows
            .iter()
            .filter(|r| r.group_index == b.group_index)
            .collect::<Vec<_>>();
        if group_rows.len() != usize::from(b.pin_count) + 1 {
            return Err(format!(
                "{} rows {} count {}",
                b.name.value,
                group_rows.len(),
                b.pin_count
            )
            .into());
        }
        let start = group_rows[0].start - 10;
        let end = group_rows.last().unwrap().end;
        let mut data = p.data[..8].to_vec();
        data[4..6].copy_from_slice(&(u16::from(b.pin_count) + 1).to_le_bytes());
        data[6..8].copy_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&p.data[start..end]);
        data[8..12].fill(0);
        let base = 8usize.wrapping_sub(start);
        let y = b.row_index * 4;
        for row in &group_rows {
            let at = row.start.wrapping_add(base);
            data[at..at + 4].copy_from_slice(&u32::from(row.row_index - b.row_index).to_le_bytes());
            for field in [22, 26, 30] {
                let value = u16::from_le_bytes(data[at + field..at + field + 2].try_into()?);
                data[at + field..at + field + 2]
                    .copy_from_slice(&value.checked_sub(y).ok_or("header y")?.to_le_bytes());
            }
            for r in records
                .iter()
                .filter(|r| r.group_index == b.group_index && r.row_index == row.row_index)
            {
                let at = r.offset.wrapping_add(base);
                let old = u16::from_le_bytes(data[at + 6..at + 8].try_into()?);
                data[at + 6..at + 8].copy_from_slice(&(old - y).to_le_bytes());
                if r.kind == IecRecordKind::LongWire {
                    let old = u16::from_le_bytes(data[at + 16..at + 18].try_into()?);
                    data[at + 16..at + 18].copy_from_slice(&(old - y).to_le_bytes());
                }
                if r.kind == IecRecordKind::FunctionBlock {
                    let old = u16::from_le_bytes(data[at + 27..at + 29].try_into()?);
                    data[at + 27..at + 29].copy_from_slice(&(old - y).to_le_bytes());
                    let mut cursor = at + 38;
                    for descriptor in 0..6 {
                        cursor += 4 + usize::from(data[cursor + 3]) * 2;
                        if descriptor == 0 || descriptor == 2 {
                            cursor += 4;
                        }
                        if descriptor == 1 || descriptor == 3 {
                            cursor += 5;
                        }
                    }
                    for ordinal in 1..=b.pin_count {
                        let old = u16::from_le_bytes(data[cursor + 1..cursor + 3].try_into()?);
                        data[cursor + 1..cursor + 3].copy_from_slice(&(old - y).to_le_bytes());
                        cursor += 12;
                        if ordinal == b.pin_count {
                            break;
                        }
                        for _ in 0..2 {
                            cursor += 4 + usize::from(data[cursor + 3]) * 2 + 4;
                            cursor += 4 + usize::from(data[cursor + 3]) * 2 + 5;
                        }
                    }
                }
            }
        }
        let mut proto = p.clone();
        proto.data = data.clone();
        proto.decoded_len = data.len();
        let bp = proto
            .iec_function_blocks()
            .ok_or("normalized blocks")?
            .remove(0);
        if proto.iec_circuit_layout().is_none() {
            return Err(format!("normalized graph {}", b.name.value).into());
        }
        fs::write(
            Path::new(&args[1]).join(format!("{}.bin", b.name.value.to_lowercase())),
            &data,
        )?;
        println!(
            "{} row={} opcode={:x}/{:x} count={} ctrl={}/{} instance={:?} pins={:?}",
            b.name.value,
            b.row_index,
            b.opcode_family,
            b.opcode,
            b.pin_count,
            b.control_input.name.value,
            b.control_output.name.value,
            b.instance.as_ref().map(|i| &i.value),
            bp.pins
                .iter()
                .map(|p| (
                    &p.name.value,
                    p.direction,
                    p.data_type_mask,
                    p.reference_ordinal,
                    p.row_index
                ))
                .collect::<Vec<_>>()
        );
    }
    Ok(())
}
