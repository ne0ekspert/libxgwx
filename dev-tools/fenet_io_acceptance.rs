//! Generate and compare legacy FEnet and I/O parameter acceptance projects.
use std::{error::Error, path::Path};
use xgwx::{NativeParameterChange, NativeParameterTarget, XgwxDocument};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("generate") => {
            let out = Path::new(args.get(2).ok_or("output directory")?);
            std::fs::create_dir_all(out)?;
            if out.join("edited.xgwx").exists() {
                return Err("destination exists".into());
            }
            let mut doc = xgwx::create_project("XGK-CPUSN", "LD")?;
            doc.base_mut()
                .by_number(0)?
                .insert_module(0, "XGI-D24A/B")?;
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
            std::fs::write(out.join("baseline.xgwx"), doc.to_verified_bytes()?)?;
            let parameter = doc
                .parameter()
                .list()
                .into_iter()
                .find(|p| p.type_name() == Some("FENET PARAMETER"))
                .ok_or("missing FEnet")?;
            let index = parameter.index();
            let changes = parameter
                .native_fields()
                .into_iter()
                .filter_map(|f| {
                    let replacement = match f.name.as_str() {
                        "IPAddress" => u32::from_le_bytes([10, 20, 30, 40]),
                        "Gateway" => u32::from_le_bytes([10, 20, 0, 1]),
                        "Subnet" => u32::from_le_bytes([255, 255, 0, 0]),
                        "RcvWaitTime" => 20,
                        "RetransTime" => 200,
                        "GlofaSocketCnt" => 4,
                        _ => return None,
                    };
                    Some(NativeParameterChange {
                        target: NativeParameterTarget::SafetyCommunication {},
                        name: f.name,
                        expected: f.value,
                        replacement: replacement.into(),
                    })
                })
                .collect::<Vec<_>>();
            doc.parameter_mut().get(index)?.edit_native(&changes)?;
            doc.module_mut()
                .at(0, 0)?
                .set_input_filter(xgwx::ModuleInputFilter::from_raw(5))?;
            std::fs::write(out.join("edited.xgwx"), doc.to_verified_bytes()?)?;
            println!("generated FEnet and I/O candidate");
        }
        Some("check") => {
            let a = XgwxDocument::from_path(args.get(2).ok_or("candidate")?)?;
            let b = XgwxDocument::from_path(args.get(3).ok_or("resave")?)?;
            let fields = |d: &XgwxDocument| {
                d.parameter()
                    .list()
                    .into_iter()
                    .find(|p| p.type_name() == Some("FENET PARAMETER"))
                    .unwrap()
                    .native_fields()
            };
            assert_eq!(fields(&a), fields(&b), "FEnet values changed");
            assert_eq!(
                a.module().summaries(),
                b.module().summaries(),
                "I/O module values changed"
            );
            println!("FEnet fields and I/O modules retained");
        }
        _ => return Err("generate OUTDIR | check CANDIDATE RESAVE".into()),
    }
    Ok(())
}
