//! Generate variable-length global names/comments, or compare native Save As.
use base64::Engine;
use std::{env, error::Error, io::Read};
use xgwx::{VariablePatch, XgwxDocument};

fn symbol_payloads(doc: &XgwxDocument) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let xml = roxmltree::Document::parse(&doc.xml)?;
    xml.descendants()
        .filter(|node| node.has_tag_name("Symbols"))
        .map(|node| {
            let text = node
                .text()
                .unwrap_or_default()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>();
            let bytes = base64::engine::general_purpose::STANDARD.decode(text)?;
            if node
                .attribute("Compressed")
                .is_some_and(|v| matches!(v, "1" | "true" | "TRUE" | "True"))
            {
                let mut decoded = Vec::new();
                bzip2::read::BzDecoder::new(bytes.as_slice()).read_to_end(&mut decoded)?;
                Ok(decoded)
            } else {
                Ok(bytes)
            }
        })
        .collect()
}
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [dump, source, output] if dump == "--symbols" => {
            let doc = XgwxDocument::from_path(source)?;
            let tables = symbol_payloads(&doc)?;
            if tables.len() != 1 { return Err("expected one global Symbols table".into()); }
            std::fs::write(output, &tables[0])?;
            println!("dumped {} symbol bytes", tables[0].len());
        }
        [source, output] => {
            let mut doc = XgwxDocument::from_path(source)?;
            let last = doc.variables()?.len().checked_sub(1).ok_or("no variables")?;
            for (index, name, description) in [
                (0, "VariableNameLongerThanBefore", "Longer comment for variable zero".to_owned()),
                (1, "V", "".to_owned()),
                (2, "가변_이름", "길어진 변수 설명 😀".to_owned()),
                (last, "LastVariableWithLongerName", "x".repeat(255)),
            ] {
                doc.update_variable(index, &VariablePatch { name: Some(name.to_owned()), description: Some(description), ..VariablePatch::default() })?;
            }
            std::fs::write(output, doc.to_bytes()?)?;
            println!("generated {} variables", doc.variables()?.len());
        }
        [compare, before, after] if compare == "--compare" => {
            let a = XgwxDocument::from_path(before)?;
            let b = XgwxDocument::from_path(after)?;
            let aa = symbol_payloads(&a)?;
            let bb = symbol_payloads(&b)?;
            if aa != bb { return Err("decoded Symbols payloads differ".into()); }
            let pa = a.ladder_programs().into_iter().collect::<Result<Vec<_>, _>>()?;
            let pb = b.ladder_programs().into_iter().collect::<Result<Vec<_>, _>>()?;
            if pa.len() != pb.len() || pa.iter().zip(&pb).any(|(a,b)| a.data != b.data) { return Err("ProgramData payloads differ".into()); }
            println!("{} symbol tables match exactly; {} ProgramData payloads match exactly; {} globals", aa.len(), pa.len(), a.variables()?.len());
        }
        _ => return Err("usage: variable_text_acceptance SOURCE OUTPUT | --compare GENERATED NATIVE_SAVED | --symbols SOURCE OUTPUT".into()),
    }
    Ok(())
}
