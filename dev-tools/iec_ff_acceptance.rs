//! Compare fresh FF insertion with native capture and generate a clean acceptance file.
use xgwx::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|s| s == "--wire") {
        let mut doc = XgwxDocument::from_path(&args[1])?;
        let block = doc
            .program()
            .get(0)?
            .ladder()?
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| {
                b.name.value == "FF"
                    && b.row_index == args.get(3).map(|s| s.parse::<u16>().unwrap()).unwrap_or(0)
            })
            .unwrap();
        for x in [block.control_output.raw_x, block.control_output.raw_x + 3] {
            doc.program_mut()
                .get(0)?
                .iec_ld()?
                .insert_function_output_wire(block.record_offset, "FF", "Q", x)?;
        }
        doc.write_to(&args[2])?;
        return Ok(());
    }
    let native = XgwxDocument::from_path(&args[0])?;
    let p = native.program().get(0)?.ladder()?;
    for block in p
        .iec_function_blocks()
        .unwrap()
        .iter()
        .filter(|b| b.name.value == "FF" && b.row_index >= 94)
    {
        let mut restored = native.clone();
        restored
            .program_mut()
            .get(0)?
            .iec_ld()?
            .delete_group(block.group_index, block.row_index)?;
        restored.program_mut().get(0)?.iec_ld()?.insert_function(
            block.row_index,
            block.raw_x,
            "FF",
            &[block.instance.as_ref().unwrap().value.clone()],
        )?;
        let after = restored.program().ladder_programs().remove(0)?;
        assert_eq!(
            p.data, after.data,
            "exact native fresh FF payload at {}/{}",
            block.row_index, block.raw_x
        );
        println!(
            "native fresh FF reconstructed exactly at {}/{}",
            block.row_index, block.raw_x
        );
    }
    let mut doc = create_project("XGI-CPUE", "LD")?;
    let xml = roxmltree::Document::parse(native.container().xml())?;
    let table = xml
        .descendants()
        .find(|n| n.has_tag_name("Program"))
        .unwrap()
        .descendants()
        .find(|n| n.has_tag_name("LocalVar"))
        .unwrap()
        .descendants()
        .find(|n| n.has_tag_name("Symbols"))
        .unwrap();
    let compressed = table.attribute("Compressed") == Some("1");
    let data = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        table
            .text()
            .unwrap()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>(),
    )?;
    let data = if compressed {
        use std::io::Read;
        let mut out = vec![];
        bzip2::read::BzDecoder::new(data.as_slice()).read_to_end(&mut out)?;
        out
    } else {
        data
    };
    let symbols = native.variable().iec_local_symbols().remove(0)?;
    let i = symbols.iter().position(|s| s.name == "FF").unwrap();
    let record =
        &data[symbols[i].record_offset..symbols.get(i + 1).map_or(data.len(), |s| s.record_offset)];
    let mut out = data[..symbols[0].record_offset].to_vec();
    out.extend(record);
    let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &out);
    // Retain native scalar instance record; remove all project-specific declarations.
    let text = doc.container().xml().to_string();
    let parsed = roxmltree::Document::parse(&text)?;
    let target = parsed
        .descendants()
        .find(|n| n.has_tag_name("LocalVar"))
        .unwrap()
        .descendants()
        .find(|n| n.has_tag_name("Symbols"))
        .unwrap();
    let replacement = format!(
        "<Symbols Count=\"1\" Compressed=\"0\" dt:dt=\"bin.base64\" xmlns:dt=\"urn:schemas-microsoft-com:datatypes\">{encoded}</Symbols>"
    );
    let mut text = text.clone();
    text.replace_range(target.range(), &replacement);
    doc.container_mut().replace_xml(text)?;
    assert_eq!(doc.variable().iec_local_symbols().remove(0)?.len(), 1);
    doc.write_to(&args[1])?;
    for (row, x) in [(0, 4), (6, 16), (12, 91)] {
        doc.program_mut()
            .get(0)?
            .iec_ld()?
            .insert_function(row, x, "FF", &["FF".into()])?;
    }
    doc.write_to(&args[2])?;
    println!("generated three clean FF placements");
    Ok(())
}
