//! Inspect the raw fields of native SFC declaration records.
use base64::Engine;
use std::io::Read;
use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = XgwxDocument::from_path(
        std::env::args()
            .nth(1)
            .ok_or("usage: sfc_declaration_inventory file.xgwx")?,
    )?;
    if std::env::args().nth(2).as_deref() == Some("--xml") {
        println!("{}", doc.xml);
        return Ok(());
    }
    let table = doc
        .root
        .descendants_named("Program")
        .next()
        .ok_or("program")?
        .descendants_named("LocalVar")
        .next()
        .ok_or("locals")?
        .descendants_named("Symbols")
        .next()
        .ok_or("symbols")?;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(table.text.split_whitespace().collect::<String>())?;
    let mut bytes = Vec::new();
    if table.attribute("Compressed") == Some("1") {
        bzip2::read::BzDecoder::new(raw.as_slice()).read_to_end(&mut bytes)?;
    } else {
        bytes = raw;
    }
    let mut last = 0;
    let mut i = 0;
    while i + 4 <= bytes.len() {
        if bytes[i..].starts_with(&[0xff, 0xfe, 0xff]) {
            let end = i + 4 + usize::from(bytes[i + 3]) * 2;
            if end > bytes.len() {
                return Err("truncated string".into());
            }
            let units = bytes[i + 4..end]
                .chunks_exact(2)
                .map(|v| u16::from_le_bytes([v[0], v[1]]))
                .collect::<Vec<_>>();
            println!(
                "{last:04x} {:02x?} {:?}",
                &bytes[last..i],
                String::from_utf16(&units)?
            );
            last = end;
            i = end;
        } else {
            i += 1;
        }
    }
    println!("tail {:02x?}", &bytes[last..]);
    Ok(())
}
