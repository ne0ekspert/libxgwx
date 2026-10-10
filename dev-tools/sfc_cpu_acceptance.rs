use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or("target/sfc-cpu-models".into());
    std::fs::create_dir_all(&directory)?;
    let input = std::env::args()
        .nth(2)
        .unwrap_or("fixtures/sfc/st-programs-generated.xgwx".into());
    let source = XgwxDocument::from_path(input)?;
    let source_model = xgwx::cpu_catalog()
        .iter()
        .find(|cpu| Some(cpu.type_code) == source.configurations()[0].type_code)
        .unwrap()
        .model;
    for (model, stem) in [
        ("XGI-CPUE", "CPUEGEN"),
        ("XGI-CPUS", "CPUSGEN"),
        ("XGI-CPUH", "CPUHGEN"),
        ("XGI-CPUU", "CPUUGEN"),
        ("XGI-CPUU/D", "CPUDGEN"),
        ("XGI-CPUUN", "CPUUNGEN"),
    ] {
        let mut doc = source.clone();
        doc.select_cpu(model)?;
        std::fs::write(format!("{directory}/{stem}.xgwx"), doc.to_verified_bytes()?)?;
        doc.select_cpu(source_model)?;
        assert_eq!(doc.sfc_programs(), source.sfc_programs());
        let mut expected_configurations = source.configurations();
        if model != source_model {
            let flags = expected_configurations[0].attribute.unwrap();
            expected_configurations[0].attribute = Some(flags & !0x80000);
            expected_configurations[0]
                .attributes
                .iter_mut()
                .find(|a| a.name == "Attribute")
                .unwrap()
                .value = (flags & !0x80000).to_string();
        }
        assert_eq!(doc.configurations(), expected_configurations);
        assert_eq!(
            doc.root.attribute("WksNodeCount"),
            source.root.attribute("WksNodeCount")
        );
        // Native Save As reindents CPUUN-only default sections. Recreating
        // them can normalize that whitespace while preserving their settings.
        if source_model != "XGI-CPUUN" {
            assert!(
                doc.xml
                    == if model == source_model {
                        source.xml.clone()
                    } else {
                        let flags = source.configurations()[0].attribute.unwrap();
                        source.xml.replacen(
                            &format!("Attribute=\"{flags}\""),
                            &format!("Attribute=\"{}\"", flags & !0x80000),
                            1,
                        )
                    },
                "{model} conversion back to {source_model}"
            );
        }
    }
    Ok(())
}
