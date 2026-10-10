//! Exercise contact deletion followed by comparison refill on an already edited chain.
use std::{env, error::Error, fs, path::PathBuf};
use xgwx::{IecRecordKind as K, XgwxDocument};
fn main() -> Result<(), Box<dyn Error>> {
    let source = env::args().nth(1).ok_or("source")?;
    let out = PathBuf::from(env::args().nth(2).ok_or("output directory")?);
    fs::create_dir_all(&out)?;
    let original = XgwxDocument::from_path(source)?;
    for x in [7, 10, 13] {
        let mut doc = original.clone();
        let program = doc.ladder_programs().remove(6)?;
        let r = program
            .iec_record_frames()
            .ok_or("records")?
            .into_iter()
            .find(|r| {
                r.row_index == 62
                    && matches!(r.kind, K::Contact(_))
                    && program.data[r.offset + 5] == x
            })
            .ok_or("contact")?;
        let K::Contact(code) = r.kind else {
            unreachable!()
        };
        let kind = [
            "NO",
            "NC",
            "RISING",
            "FALLING",
            "NEGATED_RISING",
            "NEGATED_FALLING",
        ][usize::from(code - 6)];
        let variable = String::from_utf16(
            &program.data[r.offset + 19..r.end]
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect::<Vec<_>>(),
        )?;
        doc.delete_iec_ld_contact(6, r.offset, x, kind, &variable)?;
        doc.write_to(out.join(format!("C62D{x}.xgwx")))?;
        match doc.insert_iec_ld_function(
            6,
            62,
            19,
            "EQ",
            &["%MW129".into(), "1".into(), "%MX732".into()],
        ) {
            Ok(()) => {
                doc.write_to(out.join(format!("C62R{x}.xgwx")))?;
                let block = doc
                    .ladder_programs()
                    .remove(6)?
                    .iec_function_blocks()
                    .ok_or("blocks")?
                    .into_iter()
                    .find(|b| b.row_index == 62 && b.raw_x == 19)
                    .ok_or("block")?;
                doc.delete_iec_ld_scalar_chain_function(6, block.record_offset, "EQ")?;
                doc.write_to(out.join(format!("C62G{x}.xgwx")))?;
                doc.insert_iec_ld_function(
                    6,
                    62,
                    19,
                    "EQ",
                    &["%MW129".into(), "1".into(), "%MX732".into()],
                )?;
                doc.insert_iec_ld_single_element(6, 62, x, "contact", kind, &variable)?;
                doc.write_to(out.join(format!("C62F{x}.xgwx")))?;
                println!("x{x}: contact Delete then EQ refill accepted");
            }
            Err(e) => println!("x{x}: contact Delete then EQ refill rejected: {e}"),
        }
    }
    Ok(())
}
