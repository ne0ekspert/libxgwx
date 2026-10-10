//! Compare decoded IEC PB50 local-symbol payloads across an XG5000 Save As.
use base64::Engine;
use bzip2::read::BzDecoder;
use std::{env, error::Error, io::Read};
use xgwx::XgwxDocument;

fn symbols(document: &XgwxDocument) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let xml = roxmltree::Document::parse(&document.xml)?;
    xml.descendants()
        .filter(|node| node.has_tag_name("Program"))
        .enumerate()
        .map(|(index, program)| {
            let node = program
                .descendants()
                .find(|node| node.has_tag_name("LocalVar"))
                .and_then(|local| {
                    local
                        .descendants()
                        .find(|node| node.has_tag_name("Symbols"))
                })
                .ok_or(format!("program {index}: local symbols missing"))?;
            let encoded: String = node
                .text()
                .unwrap_or_default()
                .chars()
                .filter(|ch| !ch.is_whitespace())
                .collect();
            let bytes = base64::engine::general_purpose::STANDARD.decode(encoded)?;
            if node
                .attribute("Compressed")
                .is_some_and(|value| value == "1")
            {
                let mut decoded = Vec::new();
                BzDecoder::new(bytes.as_slice()).read_to_end(&mut decoded)?;
                Ok(decoded)
            } else {
                Ok(bytes)
            }
        })
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [left, right] = args.as_slice() else {
        return Err("usage: iec_symbol_compare <generated.xgwx> <resaved.xgwx>".into());
    };
    let left_document = XgwxDocument::from_path(left)?;
    let right_document = XgwxDocument::from_path(right)?;
    let left = symbols(&left_document)?;
    let right = symbols(&right_document)?;
    if left.len() != right.len() {
        return Err("program count differs".into());
    }
    let mut all_equal = true;
    for (index, (left, right)) in left.iter().zip(&right).enumerate() {
        let same = left == right;
        all_equal &= same;
        println!(
            "program {index}: {} -> {} PB50 bytes; equal={same}",
            left.len(),
            right.len()
        );
        if !same {
            let prefix = left.iter().zip(right).take_while(|(a, b)| a == b).count();
            println!("  first difference at 0x{prefix:X}");
            println!(
                "  generated: {:02X?}",
                &left[prefix..left.len().min(prefix + 32)]
            );
            println!(
                "  resaved:   {:02X?}",
                &right[prefix..right.len().min(prefix + 32)]
            );
        }
    }
    for (index, (left, right)) in left_document
        .iec_local_symbols()
        .into_iter()
        .zip(right_document.iec_local_symbols())
        .enumerate()
    {
        let mut left = left?;
        let mut right = right?;
        for symbol in &mut left {
            symbol.record_offset = 0;
        }
        for symbol in &mut right {
            symbol.record_offset = 0;
        }
        println!("program {index}: decoded symbols equal={}", left == right);
        if left != right {
            return Err("one or more decoded IEC symbols changed".into());
        }
    }
    if !all_equal {
        return Err("one or more PB50 payloads changed".into());
    }
    Ok(())
}
