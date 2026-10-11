#![cfg(feature = "write")]
#[cfg(feature = "write")]
#[path = "support/hardware_support.rs"]
mod hardware_support;
use xgwx::{CnetFieldEdit, CnetSettingsPatch, XgwxDocument};
fn base() -> XgwxDocument {
    XgwxDocument::parse(include_bytes!("../fixtures/networks/cnetbase-native.xgwx")).unwrap()
}
fn change(port_index: usize, field: &str, old: &str, new: &str) -> CnetFieldEdit {
    CnetFieldEdit {
        port_index,
        field: field.into(),
        expected_value: old.into(),
        replacement: new.into(),
    }
}
fn patch(changes: Vec<CnetFieldEdit>) -> CnetSettingsPatch {
    CnetSettingsPatch {
        base: 0,
        slot: 2,
        changes,
    }
}
fn native_changes(doc: &XgwxDocument, target: &XgwxDocument) -> Vec<CnetFieldEdit> {
    let fields = [
        ("StationNo", "stationNo"),
        ("Mode", "modeRaw"),
        ("Bps", "bps"),
        ("DataBit", "dataBitRaw"),
        ("StopBit", "stopBitRaw"),
        ("Parity", "parityRaw"),
        ("RxTimeOut", "rxTimeout"),
        ("CharTimeOut", "charTimeout"),
        ("DriverType", "driverType"),
        ("RequestDelayTime", "requestDelayTime"),
        ("ParityErrorIgnore", "parityErrorIgnore"),
        ("TerminatingResister", "terminatingResister"),
        ("Repeater", "repeater"),
    ];
    let a = doc.network().cnet_config_infos();
    let b = target.network().cnet_config_infos();
    let mut result = Vec::new();
    for (index, (before, after)) in a[0].ports.iter().zip(&b[0].ports).enumerate() {
        for (attribute, field) in fields {
            let old = &before
                .attributes
                .iter()
                .find(|a| a.name == attribute)
                .unwrap()
                .value;
            let new = &after
                .attributes
                .iter()
                .find(|a| a.name == attribute)
                .unwrap()
                .value;
            if old != new {
                result.push(change(index, field, old, new));
            }
        }
    }
    result
}
#[test]
fn edits_match_native_ascii_rtu_smart_and_repeater_records_exactly() {
    for bytes in [
        include_bytes!("../fixtures/networks/cnetascii-native.xgwx").as_slice(),
        include_bytes!("../fixtures/networks/cnetrtu-native.xgwx").as_slice(),
        include_bytes!("../fixtures/networks/cnetrep-native.xgwx").as_slice(),
    ] {
        let mut doc = base();
        let target = XgwxDocument::parse(bytes).unwrap();
        let original = doc.clone();
        {
            let field_patch = &patch(native_changes(&doc, &target));
            hardware_support::edit_cnet_settings(&mut doc, field_patch)
        }
        .unwrap();
        assert_eq!(
            doc.network().cnet_config_infos(),
            target.network().cnet_config_infos()
        );
        assert_eq!(
            doc.network().fenet_config_infos(),
            original.network().fenet_config_infos()
        );
        assert_eq!(
            doc.variable().variables().unwrap(),
            original.variable().variables().unwrap()
        );
        assert_eq!(
            doc.program().ladder_programs().remove(0).unwrap().data,
            original.program().ladder_programs().remove(0).unwrap().data
        );
        XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
        {
            let field_patch = &patch(native_changes(&doc, &original));
            hardware_support::edit_cnet_settings(&mut doc, field_patch)
        }
        .unwrap();
        assert_eq!(doc.container().xml(), original.container().xml());
    }
}
#[test]
fn invalid_stale_and_ambiguous_cnet_edits_are_atomic() {
    let mut doc = base();
    let original = doc.container().xml().to_owned();
    for changes in [
        vec![change(0, "stationNo", "0", "32")],
        vec![change(0, "driverType", "2", "5")],
        vec![change(0, "stationNo", "wrong", "1")],
        vec![change(0, "stationNo", "0", "-1")],
        vec![change(0, "modeRaw", "0", "1")],
        vec![change(1, "modeRaw", "1", "0")],
        vec![change(0, "bps", "8", "15")],
        vec![change(0, "rxTimeout", "1", "51")],
        vec![change(0, "charTimeout", "1", "256")],
        vec![change(1, "requestDelayTime", "0", "256")],
        vec![change(0, "dataBitRaw", "1", "2")],
        vec![change(0, "stopBitRaw", "0", "2")],
        vec![change(0, "parityRaw", "0", "3")],
        vec![change(0, "terminatingResister", "0", "1")],
        vec![change(1, "terminatingResister", "0", "2")],
        vec![change(0, "repeater", "0", "1")],
        vec![
            change(0, "driverType", "2", "3"),
            change(0, "stationNo", "0", "255"),
        ],
        vec![
            change(0, "stationNo", "0", "1"),
            change(0, "stationNo", "0", "2"),
        ],
        vec![change(2, "stationNo", "0", "1")],
        vec![change(0, "opaque", "0", "1")],
        vec![
            change(0, "stationNo", "0", "1"),
            change(1, "charTimeout", "1", "256"),
        ],
    ] {
        assert!(
            {
                let field_patch = &patch(changes);
                hardware_support::edit_cnet_settings(&mut doc, field_patch)
            }
            .is_err()
        );
        assert_eq!(doc.container().xml(), original);
    }
    let xml = roxmltree::Document::parse(&doc.container().xml()).unwrap();
    let record = xml
        .descendants()
        .find(|n| n.has_tag_name("XGPD_CONFIG_INFO_CNET"))
        .unwrap()
        .range();
    let extra = doc.container().xml()[record].to_owned();
    {
        let replacement_xml = doc.container().xml().replace(
            "</XGPD_CONFIG_INFO_GROUP>",
            &format!("{extra}</XGPD_CONFIG_INFO_GROUP>"),
        );
        doc.container_mut().replace_xml(replacement_xml).unwrap();
    };
    let original = doc.container().xml().to_owned();
    assert!(
        {
            let field_patch = &patch(vec![change(0, "stationNo", "0", "1")]);
            hardware_support::edit_cnet_settings(&mut doc, field_patch)
        }
        .is_err()
    );
    assert_eq!(doc.container().xml(), original);
}
#[test]
fn repeater_requires_equal_baud_rates_and_driver_changes_validate_final_station() {
    let mut doc = base();
    {
        let field_patch = &patch(vec![
            change(0, "driverType", "2", "3"),
            change(0, "stationNo", "0", "255"),
            change(0, "dataBitRaw", "1", "0"),
        ]);
        hardware_support::edit_cnet_settings(&mut doc, field_patch)
    }
    .unwrap();
    {
        let field_patch = &patch(vec![
            change(0, "driverType", "3", "2"),
            change(0, "stationNo", "255", "31"),
        ]);
        hardware_support::edit_cnet_settings(&mut doc, field_patch)
    }
    .unwrap();
    {
        let field_patch = &patch(vec![
            change(0, "repeater", "0", "1"),
            change(1, "repeater", "0", "1"),
        ]);
        hardware_support::edit_cnet_settings(&mut doc, field_patch)
    }
    .unwrap();
    let original = doc.container().xml().to_owned();
    assert!(
        {
            let field_patch = &patch(vec![change(1, "bps", "8", "14")]);
            hardware_support::edit_cnet_settings(&mut doc, field_patch)
        }
        .is_err()
    );
    assert_eq!(doc.container().xml(), original);
    {
        let field_patch = &patch(vec![
            change(0, "bps", "8", "14"),
            change(1, "bps", "8", "14"),
        ]);
        hardware_support::edit_cnet_settings(&mut doc, field_patch)
    }
    .unwrap();
}
#[test]
fn new_cnet_modules_have_native_defaults_and_other_modules_are_preserved() {
    for (model, modes) in [
        ("XGL-C22A/B", [0, 0]),
        ("XGL-CH2A/B", [0, 1]),
        ("XGL-C42A/B", [1, 1]),
    ] {
        let mut doc = base();
        let existing = doc.network().cnet_config_infos();
        let fenet = doc.network().fenet_config_infos();
        doc.base_mut()
            .by_number(0)
            .and_then(|mut selected_record| selected_record.insert_module(3, model))
            .unwrap();
        let records = doc.network().cnet_config_infos();
        assert_eq!(records[0], existing[0]);
        assert_eq!(records[1].ports.len(), 2);
        for (port, mode) in records[1].ports.iter().zip(modes) {
            assert_eq!(port.mode, Some(mode));
            assert_eq!(port.baud_rate, Some(9600));
            assert_eq!(port.driver_type, Some(2));
        }
        {
            let field_patch = &CnetSettingsPatch {
                base: 0,
                slot: 3,
                changes: vec![change(1, "stationNo", "0", "9")],
            };
            hardware_support::edit_cnet_settings(&mut doc, field_patch)
        }
        .unwrap();
        assert_eq!(doc.network().cnet_config_infos()[0], existing[0]);
        assert_eq!(doc.network().fenet_config_infos(), fenet);
        if model == "XGL-C22A/B" {
            let original = doc.container().xml().to_owned();
            assert!(
                {
                    let field_patch = &CnetSettingsPatch {
                        base: 0,
                        slot: 3,
                        changes: vec![
                            change(0, "repeater", "0", "1"),
                            change(1, "repeater", "0", "1"),
                        ],
                    };
                    hardware_support::edit_cnet_settings(&mut doc, field_patch)
                }
                .is_err()
            );
            assert_eq!(doc.container().xml(), original);
        }
        doc.module_mut()
            .at(0, 3)
            .and_then(|selected_record| selected_record.delete())
            .unwrap();
        assert_eq!(doc.network().cnet_config_infos(), existing);
    }
}
#[test]
fn native_baud_selectors_decode_without_guessing_or_overflow() {
    for (bytes, rates) in [
        (
            include_bytes!("../fixtures/networks/cnetascii-native.xgwx").as_slice(),
            [300, 64000],
        ),
        (
            include_bytes!("../fixtures/networks/cnetrtu-native.xgwx").as_slice(),
            [600, 115200],
        ),
    ] {
        let doc = XgwxDocument::parse(bytes).unwrap();
        for (port, rate) in doc.network().cnet_config_infos()[0].ports.iter().zip(rates) {
            assert_eq!(port.baud_rate, Some(rate));
        }
    }
    let mut doc = base();
    {
        let replacement_xml = doc
            .container()
            .xml()
            .replacen("Bps=\"8\"", "Bps=\"4294967295\"", 1);
        doc.container_mut().replace_xml(replacement_xml).unwrap();
    };
    let doc = XgwxDocument::parse(&doc.to_bytes().unwrap()).unwrap();
    assert_eq!(
        doc.network().cnet_config_infos()[0].ports[0].baud_rate,
        None
    );
}
#[test]
#[ignore = "set LIBXGWX_CNET_ACCEPTANCE for native XG5000 validation"]
fn generate_cnet_native_acceptance() {
    let mut doc = base();
    let target =
        XgwxDocument::parse(include_bytes!("../fixtures/networks/cnetascii-native.xgwx")).unwrap();
    {
        let field_patch = &patch(native_changes(&doc, &target));
        hardware_support::edit_cnet_settings(&mut doc, field_patch)
    }
    .unwrap();
    doc.base_mut()
        .by_number(0)
        .and_then(|mut selected_record| selected_record.insert_module(3, "XGL-C22A/B"))
        .unwrap();
    doc.base_mut()
        .by_number(0)
        .and_then(|mut selected_record| selected_record.insert_module(4, "XGL-C42A/B"))
        .unwrap();
    std::fs::write(
        std::env::var("LIBXGWX_CNET_ACCEPTANCE").unwrap(),
        doc.to_verified_bytes().unwrap(),
    )
    .unwrap();
}

#[test]
fn generated_edits_and_new_models_survive_native_save_as() {
    let mut generated = base();
    let ascii =
        XgwxDocument::parse(include_bytes!("../fixtures/networks/cnetascii-native.xgwx")).unwrap();
    {
        let field_patch = &patch(native_changes(&generated, &ascii));
        hardware_support::edit_cnet_settings(&mut generated, field_patch)
    }
    .unwrap();
    generated
        .base_mut()
        .by_number(0)
        .and_then(|mut selected_record| selected_record.insert_module(3, "XGL-C22A/B"))
        .unwrap();
    generated
        .base_mut()
        .by_number(0)
        .and_then(|mut selected_record| selected_record.insert_module(4, "XGL-C42A/B"))
        .unwrap();
    let saved = XgwxDocument::parse(include_bytes!(
        "../fixtures/networks/cnet-round-native.xgwx"
    ))
    .unwrap();
    assert_eq!(
        saved.network().cnet_config_infos(),
        generated.network().cnet_config_infos()
    );
    assert_eq!(
        saved.program().ladder_programs().remove(0).unwrap().data,
        generated
            .program()
            .ladder_programs()
            .remove(0)
            .unwrap()
            .data
    );
    assert_eq!(
        saved.network().fenet_config_infos(),
        generated.network().fenet_config_infos()
    );
}
