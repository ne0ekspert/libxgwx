#![cfg(feature = "write")]
use xgwx::{ModuleInputFilter, ModulePatch, XgwxDocument, create_project};
fn ladders(doc: &XgwxDocument) -> Vec<xgwx::LadderProgramData> {
    doc.program()
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect()
}
fn modules(doc: &XgwxDocument) -> Vec<xgwx::ModuleSummary> {
    doc.module()
        .list()
        .into_iter()
        .map(|m| m.summary())
        .collect()
}

fn select(doc: &mut XgwxDocument, model: &str) {
    doc.configuration_mut()
        .get(0)
        .unwrap()
        .select_cpu(model)
        .unwrap();
}

#[test]
fn iec_ladder_and_installed_io_survive_all_standard_xgi_cpu_conversions() {
    let mut doc = create_project("XGI-CPUE", "LD").unwrap();
    doc.base_mut()
        .get(0)
        .unwrap()
        .insert_module(0, "XGI-D24A/B")
        .unwrap();
    doc.module_mut()
        .at(0, 0)
        .unwrap()
        .update(&ModulePatch {
            comment: Some("Retain input configuration".into()),
            ..Default::default()
        })
        .unwrap();
    doc.module_mut()
        .at(0, 0)
        .unwrap()
        .set_input_filter(ModuleInputFilter::Ms5)
        .unwrap();
    doc.base_mut().get(0).unwrap().set_slot_count(8).unwrap();
    let before = doc.clone();
    for model in [
        "XGI-CPUS",
        "XGI-CPUH",
        "XGI-CPUU",
        "XGI-CPUU/D",
        "XGI-CPUUN",
        "XGI-CPUE",
    ] {
        select(&mut doc, model);
        assert_eq!(ladders(&doc), ladders(&before));
        assert_eq!(modules(&doc), modules(&before));
        let saved = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
        assert_eq!(ladders(&saved), ladders(&before));
        assert_eq!(modules(&saved), modules(&before));
    }
    doc.module_mut()
        .at(0, 0)
        .unwrap()
        .select_model("XGQ-RY2A/B")
        .unwrap();
    assert_eq!(
        doc.module().list()[0].summary().comment.as_deref(),
        Some("Retain input configuration")
    );
    doc.module_mut().at(0, 0).unwrap().delete().unwrap();
    assert!(doc.module().list().is_empty());
}

#[test]
fn xgi_module_and_cpu_limit_failures_are_atomic() {
    let mut doc = create_project("XGI-CPUE", "LD").unwrap();
    let before = doc.clone();
    assert!(
        doc.base_mut()
            .get(2)
            .unwrap()
            .insert_module(0, "XGI-D24A/B")
            .is_err()
    );
    assert_eq!(doc, before);
    select(&mut doc, "XGI-CPUU");
    doc.base_mut()
        .get(2)
        .unwrap()
        .insert_module(0, "XGI-D24A/B")
        .unwrap();
    let before = doc.clone();
    assert!(
        doc.configuration_mut()
            .get(0)
            .unwrap()
            .select_cpu("XGI-CPUE")
            .is_err()
    );
    assert_eq!(doc, before);
    assert!(
        doc.base_mut()
            .get(2)
            .unwrap()
            .insert_module(0, "XGI-D24A/B")
            .is_err()
    );
    assert_eq!(doc, before);
    assert!(
        doc.module_mut()
            .at(2, 0)
            .unwrap()
            .select_model("unknown")
            .is_err()
    );
    assert_eq!(doc, before);
}

#[test]
fn configured_communication_modules_are_preserved_during_cpu_changes() {
    let mut doc = create_project("XGI-CPUE", "LD").unwrap();
    doc.base_mut()
        .get(0)
        .unwrap()
        .insert_module(1, "XGL-C22A/B")
        .unwrap();
    let before = doc.clone();
    for model in ["XGI-CPUS", "XGI-CPUUN", "XGI-CPUE"] {
        select(&mut doc, model);
        assert_eq!(modules(&doc), modules(&before));
        assert_eq!(doc.network().modules(), before.network().modules());
        doc.to_verified_bytes().unwrap();
    }
}

#[test]
fn malformed_xgi_base_information_rejects_conversion_without_mutation() {
    let source = create_project("XGI-CPUE", "LD").unwrap();
    for xml in [
        source
            .container()
            .xml()
            .replace("BaseInfo", "UnknownBaseInfo"),
        source
            .container()
            .xml()
            .replace("SlotCount=\"12\"", "SlotCount=\"2\""),
        source.container().xml().replace("Base=\"1\"", "Base=\"0\""),
    ] {
        let mut doc = source.clone();
        doc.container_mut().replace_xml(xml).unwrap();
        let before = doc.clone();
        assert!(
            doc.configuration_mut()
                .get(0)
                .unwrap()
                .select_cpu("XGI-CPUUN")
                .is_err()
        );
        assert_eq!(doc, before);
    }
}

#[test]
fn opaque_native_basic_metadata_is_preserved_across_cpu_conversions() {
    let seed = create_project("XGI-CPUE", "LD").unwrap();
    let mut root = seed.container().root().clone();
    fn replace_metadata(node: &mut xgwx::XmlElement) {
        if node.name == "XGIBasicParam" {
            for attribute in &mut node.attributes {
                if attribute.name.starts_with("OUTPUT_PARAMETER_RESERVED_") {
                    let index: u32 = attribute.name.rsplit('_').next().unwrap().parse().unwrap();
                    attribute.value = (12345 + index * 111).to_string();
                } else if attribute.name == "check_sum" {
                    attribute.value = "2147483648".into();
                }
            }
        }
        for child in &mut node.children {
            replace_metadata(child);
        }
    }
    replace_metadata(&mut root);
    let mut source = seed.clone();
    // Preserve XML spelling; editing the container does not modify any program.
    let mut xml = seed.container().xml().to_owned();
    let old = seed
        .container()
        .root()
        .descendants_named("XGIBasicParam")
        .next()
        .unwrap();
    let new = root.descendants_named("XGIBasicParam").next().unwrap();
    for attribute in &new.attributes {
        if old.attribute(&attribute.name) != Some(attribute.value.as_str()) {
            xml = xml.replace(
                &format!(
                    "{}=\"{}\"",
                    attribute.name,
                    old.attribute(&attribute.name).unwrap()
                ),
                &format!("{}=\"{}\"", attribute.name, attribute.value),
            );
        }
    }
    source.container_mut().replace_xml(xml).unwrap();
    for model in [
        "XGI-CPUS",
        "XGI-CPUH",
        "XGI-CPUU",
        "XGI-CPUU/D",
        "XGI-CPUUN",
    ] {
        let mut doc = source.clone();
        select(&mut doc, model);
        let basic = doc
            .container()
            .root()
            .descendants_named("XGIBasicParam")
            .next()
            .unwrap();
        for name in (0..5)
            .map(|i| format!("OUTPUT_PARAMETER_RESERVED_{i}"))
            .chain(std::iter::once("check_sum".into()))
        {
            assert_eq!(basic.attribute(&name), new.attribute(&name));
        }
        assert_eq!(ladders(&doc), ladders(&source));
        doc.to_verified_bytes().unwrap();
        select(&mut doc, "XGI-CPUE");
        let flags = source.configuration().list()[0].attribute.unwrap();
        assert_eq!(
            doc.container().xml(),
            source.container().xml().replacen(
                &format!("Attribute=\"{flags}\""),
                &format!("Attribute=\"{}\"", flags & !0x80000),
                1
            )
        );
    }
}

#[test]
fn malformed_basic_metadata_and_functional_custom_settings_remain_atomic() {
    let source = create_project("XGI-CPUE", "LD").unwrap();
    for (old, new) in [
        ("check_sum=\"0\"", "check_sum=\"4294967296\""),
        ("check_sum=\"0\"", "check_sum=\"invalid\""),
        ("SCAN_WD_TIME_0=\"500\"", "SCAN_WD_TIME_0=\"600\""),
    ] {
        let mut doc = source.clone();
        let xml = source.container().xml().replace(old, new);
        assert_ne!(xml, source.container().xml());
        doc.container_mut().replace_xml(xml).unwrap();
        let before = doc.clone();
        assert!(
            doc.configuration_mut()
                .get(0)
                .unwrap()
                .select_cpu("XGI-CPUS")
                .is_err()
        );
        assert_eq!(doc, before);
    }
}
