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
