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
        .find(|cpu| Some(cpu.type_code) == source.configuration().list()[0].type_code)
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
        doc.configuration_mut()
            .get(0)
            .and_then(|mut plc| plc.select_cpu(model))?;
        std::fs::write(format!("{directory}/{stem}.xgwx"), doc.to_verified_bytes()?)?;
        doc.configuration_mut()
            .get(0)
            .and_then(|mut plc| plc.select_cpu(source_model))?;
        assert_eq!(
            doc.program().sfc_programs(),
            source.program().sfc_programs()
        );
        let mut expected_configurations = source.configuration().list();
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
        assert_eq!(doc.configuration().list(), expected_configurations);
        assert_eq!(
            doc.container().root().attribute("WksNodeCount"),
            source.container().root().attribute("WksNodeCount")
        );
        // Native Save As reindents CPUUN-only default sections. Recreating
        // them can normalize that whitespace while preserving their settings.
        if source_model != "XGI-CPUUN" {
            assert!(
                doc.container().xml()
                    == if model == source_model {
                        source.container().xml().to_owned()
                    } else {
                        let flags = source.configuration().list()[0].attribute.unwrap();
                        source.container().xml().replacen(
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
