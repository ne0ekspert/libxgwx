//! Generate and compare native basic-operation acceptance projects.
use std::{error::Error, path::Path};
use xgwx::{BasicParameterControl, XgwxDocument};

fn generate(model: &str, mode: &str, output: &Path) -> Result<(), Box<dyn Error>> {
    let iec = model.starts_with("XGI");
    let mut doc = xgwx::create_project(model, if iec { "ST" } else { "LD" })?;
    if iec {
        let old = doc
            .program()
            .get(0)?
            .text()?
            .source
            .ok_or("missing source")?;
        doc.program_mut()
            .get(0)?
            .text()?
            .update(&xgwx::TextSourcePatch {
                expected_source: old,
                source: "%MX0 := TRUE;\r\n".into(),
            })?;
    } else {
        for (column, kind, operand) in [
            (0, xgwx::LadderEditKind::NormallyOpen, "M00000"),
            (9, xgwx::LadderEditKind::Output, "M00001"),
        ] {
            doc.program_mut()
                .get(0)?
                .ladder()?
                .edit_cell(&xgwx::LadderCellEdit {
                    raw_y: 0,
                    column,
                    expected: None,
                    replacement: Some(xgwx::LadderEditElement {
                        kind,
                        operand: operand.into(),
                    }),
                })?;
        }
    }
    let record = doc
        .parameter()
        .list()
        .into_iter()
        .find(|p| !p.basic_operation_fields().is_empty())
        .ok_or("no basic operation fields")?;
    let index = record.index();
    let fields = record.basic_operation_fields();
    let values = fields
        .iter()
        .map(|field| {
            let value = match &field.control {
                BasicParameterControl::Checkbox { checked, unchecked } => {
                    if mode == "on" || (mode == "overall" && field.name.starts_with("O_")) {
                        *checked
                    } else {
                        *unchecked
                    }
                }
                BasicParameterControl::Number { minimum, maximum } => {
                    if mode == "on" {
                        *maximum
                    } else if mode == "off" {
                        *minimum
                    } else if field.name == "STATIC_PERIOD_TIME" {
                        321
                    } else {
                        777
                    }
                }
                BasicParameterControl::Select { choices } => {
                    if field.name == "STANDARD_INPUT_FILTER_TIME" {
                        if mode == "on" {
                            100
                        } else if mode == "off" {
                            1
                        } else {
                            5
                        }
                    } else if mode == "on" {
                        choices.last().unwrap().value
                    } else {
                        choices[0].value
                    }
                }
            };
            (field.name.as_str(), field.value.as_str(), value)
        })
        .collect::<Vec<_>>();
    doc.parameter_mut()
        .get(index)?
        .edit_basic_operation(&values)?;
    std::fs::write(output, doc.to_verified_bytes()?)?;
    for field in doc.parameter().get(index)?.basic_operation_fields() {
        println!("{}={}", field.name, field.value);
    }
    Ok(())
}
fn check(original: &Path, resaved: &Path) -> Result<(), Box<dyn Error>> {
    let a = XgwxDocument::parse(&std::fs::read(original)?)?;
    let b = XgwxDocument::parse(&std::fs::read(resaved)?)?;
    let before = a
        .parameter()
        .list()
        .into_iter()
        .find(|p| !p.basic_operation_fields().is_empty())
        .ok_or("no original basic fields")?;
    let after = b
        .parameter()
        .list()
        .into_iter()
        .find(|p| !p.basic_operation_fields().is_empty())
        .ok_or("no resaved basic fields")?;
    assert_eq!(
        before.basic_operation_fields(),
        after.basic_operation_fields()
    );
    // Compare every basic attribute, including hidden watchdog slots, other tabs,
    // reserved fields, native checksum markers and undocumented attributes.
    assert_eq!(before.summary(), after.summary());
    fn payloads(doc: &XgwxDocument) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        use base64::Engine;
        use std::io::Read;
        doc.container()
            .root()
            .descendants_named("ProgramData")
            .map(|n| {
                let text = n.text.split_whitespace().collect::<String>();
                let raw = base64::engine::general_purpose::STANDARD.decode(text)?;
                if n.attribute("Compressed") == Some("1") {
                    let mut output = Vec::new();
                    bzip2::read::BzDecoder::new(raw.as_slice()).read_to_end(&mut output)?;
                    Ok(output)
                } else {
                    Ok(raw)
                }
            })
            .collect()
    }
    assert_eq!(payloads(&a)?, payloads(&b)?);
    println!(
        "PASS: every basic attribute and program payload preserved: {}",
        resaved.display()
    );
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["generate", model, mode @ ("on" | "off" | "overall"), output] => {
            generate(model, mode, Path::new(output))
        }
        ["check", original, resaved] => check(Path::new(original), Path::new(resaved)),
        _ => Err("use generate MODEL on|off|overall OUTPUT or check GENERATED RESAVED".into()),
    }
}
