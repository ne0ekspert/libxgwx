#![cfg(feature = "write")]
use xgwx::{BrowserNetworkPatch, FenetFieldPatch, XgwxDocument};
fn project() -> XgwxDocument {
    let mut doc =
        XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgk.xgwx")).unwrap();
    doc.insert_module(0, 0, "XGL-EFMT(B)").unwrap();
    doc.insert_module(0, 1, "XGL-EFMT(B)").unwrap();
    doc
}
fn patch(field: &str, expected: &str, replacement: &str) -> FenetFieldPatch {
    FenetFieldPatch {
        base: 0,
        slot: 0,
        field: field.into(),
        expected_value: expected.into(),
        replacement: replacement.into(),
    }
}
#[test]
fn fenet_address_and_dhcp_edits_preserve_other_modules_and_programs() {
    let mut doc = project();
    let original = doc.xml.clone();
    let before = doc.ladder_programs().remove(0).unwrap().data;
    doc.edit_fenet_field(&patch("ipAddress", "192.168.0.100", "10.20.30.40"))
        .unwrap();
    let ips = doc.fenet_config_infos();
    assert_eq!(ips[0].ip_address.as_ref().unwrap().address, "10.20.30.40");
    assert_eq!(ips[1].ip_address.as_ref().unwrap().address, "192.168.0.100");
    doc.edit_fenet_field(&patch("ipAddress", "10.20.30.40", "192.168.0.100"))
        .unwrap();
    assert_eq!(doc.xml, original);
    for (field, old, new) in [
        ("subnet", "255.255.255.0", "255.255.0.0"),
        ("gateway", "192.168.0.1", "10.20.0.1"),
        ("dns", "0.0.0.0", "1.1.1.1"),
        ("ipAddress2", "0.0.0.0", "172.16.0.2"),
        ("subnet2", "0.0.0.0", "255.255.255.0"),
        ("gateway2", "0.0.0.0", "172.16.0.1"),
        ("dns2", "0.0.0.0", "8.8.8.8"),
        ("dhcp", "0", "1"),
        ("dhcp2", "0", "1"),
    ] {
        doc.edit_fenet_field(&patch(field, old, new)).unwrap();
        XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
        doc.edit_fenet_field(&patch(field, new, old)).unwrap();
        assert_eq!(doc.xml, original);
    }
    assert_eq!(doc.ladder_programs().remove(0).unwrap().data, before);
}
#[test]
fn invalid_and_stale_fenet_edits_are_atomic() {
    let mut doc = project();
    let original = doc.xml.clone();
    for p in [
        patch("ipAddress", "wrong", "1.2.3.4"),
        patch("ipAddress", "192.168.0.100", "1.2.3.256"),
        patch("ipAddress", "192.168.0.100", "1.2.3"),
        patch("ipAddress", "192.168.0.100", "+1.2.3.4"),
        patch("subnet", "255.255.255.0", "255.0.255.0"),
        patch("dhcp", "0", "2"),
        patch("driverType", "2", "3"),
        FenetFieldPatch {
            slot: 5,
            ..patch("dhcp", "0", "1")
        },
    ] {
        assert!(doc.edit_fenet_field(&p).is_err());
        assert_eq!(doc.xml, original);
    }
    let mut wrong = doc.clone();
    wrong.xml = wrong.xml.replacen("Type=\"23041\"", "Type=\"23042\"", 1);
    assert!(wrong.edit_fenet_field(&patch("dhcp", "0", "1")).is_err());
}
#[test]
fn network_metadata_edits_preserve_linked_configuration() {
    let mut doc = project();
    let fenet = doc.fenet_config_infos();
    for (module, field, old, new) in [
        (false, "name", "기본 네트워크", "Home & Office"),
        (true, "alias", "", "Main PLC"),
        (true, "description", "", "Ethernet <uplink>"),
    ] {
        doc.edit_browser_network(&BrowserNetworkPatch {
            network_index: 0,
            module,
            base: Some(0),
            slot: Some(0),
            field: field.into(),
            expected_value: old.into(),
            replacement: new.into(),
        })
        .unwrap();
        assert_eq!(doc.fenet_config_infos(), fenet);
        XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
    }
}

#[test]
#[ignore = "set LIBXGWX_NETWORK_ACCEPTANCE_DIR for native XG5000 validation"]
fn generate_native_network_acceptance_files() {
    let directory =
        std::path::PathBuf::from(std::env::var("LIBXGWX_NETWORK_ACCEPTANCE_DIR").unwrap());
    std::fs::create_dir_all(&directory).unwrap();
    let mut doc = project();
    std::fs::write(
        directory.join("NETBASE.xgwx"),
        doc.to_verified_bytes().unwrap(),
    )
    .unwrap();
    for (field, old, new) in [
        ("ipAddress", "192.168.0.100", "10.20.30.40"),
        ("subnet", "255.255.255.0", "255.255.0.0"),
        ("gateway", "192.168.0.1", "10.20.0.1"),
        ("dns", "0.0.0.0", "1.1.1.1"),
        ("dhcp", "0", "1"),
    ] {
        doc.edit_fenet_field(&patch(field, old, new)).unwrap();
    }
    doc.edit_browser_network(&BrowserNetworkPatch {
        network_index: 0,
        module: true,
        base: Some(0),
        slot: Some(0),
        field: "alias".into(),
        expected_value: "".into(),
        replacement: "Main Ethernet".into(),
    })
    .unwrap();
    std::fs::write(
        directory.join("NETEDIT.xgwx"),
        doc.to_verified_bytes().unwrap(),
    )
    .unwrap();
}

#[test]
fn native_saved_fenet_project_remains_editable() {
    let mut doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/networks/fenet-edited-native.xgwx"
    ))
    .unwrap();
    let native = doc.fenet_config_infos();
    assert_eq!(
        native[0].ip_address.as_ref().unwrap().address,
        "10.20.30.40"
    );
    assert_eq!(native[0].subnet.as_ref().unwrap().address, "255.255.0.0");
    assert_eq!(native[0].dhcp, Some(1));
    assert_eq!(
        doc.networks()[0].modules[0].alias.as_deref(),
        Some("Main Ethernet")
    );
    let original = doc.xml.clone();
    doc.edit_fenet_field(&patch("ipAddress", "10.20.30.40", "10.20.30.41"))
        .unwrap();
    assert_eq!(doc.fenet_config_infos()[1], native[1]);
    doc.edit_fenet_field(&patch("ipAddress", "10.20.30.41", "10.20.30.40"))
        .unwrap();
    assert_eq!(doc.xml, original);
}

#[test]
fn fenet_numeric_fields_edit_atomically_and_preserve_other_settings() {
    let mut doc = project();
    let original = doc.xml.clone();
    let other = doc.fenet_config_infos()[1].clone();
    for (field, old, new) in [
        ("stationNo", "0", "63"),
        ("driverType", "2", "5"),
        ("driverType", "2", "7"),
        ("rcvWaitTime", "100", "255"),
        ("clientWaitTime", "60", "2"),
        ("glofaSocketCount", "3", "16"),
    ] {
        doc.edit_fenet_field(&patch(field, old, new)).unwrap();
        assert_eq!(doc.fenet_config_infos()[1], other);
        XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
        doc.edit_fenet_field(&patch(field, new, old)).unwrap();
        assert_eq!(doc.xml, original);
    }
    for (field, old, value) in [
        ("stationNo", "0", "64"),
        ("glofaSocketCount", "3", "0"),
        ("glofaSocketCount", "3", "17"),
        ("rcvWaitTime", "100", "1"),
        ("rcvWaitTime", "100", "256"),
        ("clientWaitTime", "60", "-2"),
        ("clientWaitTime", "60", "2.5"),
    ] {
        assert!(doc.edit_fenet_field(&patch(field, old, value)).is_err());
        assert_eq!(doc.xml, original);
    }
}

#[test]
fn native_driver_and_timeout_encodings_remain_editable() {
    for (bytes, driver, receive, client, unit) in [
        (
            include_bytes!("../fixtures/networks/fenet-modbus-native.xgwx").as_slice(),
            "5",
            "255",
            "2",
            "0",
        ),
        (
            include_bytes!("../fixtures/networks/fenet-smart-native.xgwx").as_slice(),
            "7",
            "30",
            "20",
            "1",
        ),
    ] {
        let mut doc = XgwxDocument::parse(bytes).unwrap();
        let original = doc.xml.clone();
        let other = doc.fenet_config_infos()[1].clone();
        let program = doc.ladder_programs().remove(0).unwrap().data;
        for (field, old, new) in [
            ("driverType", driver, "2"),
            ("rcvWaitTime", receive, "100"),
            ("clientWaitTime", client, "60"),
        ] {
            doc.edit_fenet_field(&patch(field, old, new)).unwrap();
            let xml = roxmltree::Document::parse(&doc.xml).unwrap();
            let record = xml
                .descendants()
                .find(|n| n.has_tag_name("XGPD_CONFIG_INFO_FENET"))
                .unwrap();
            assert_eq!(record.attribute("RcvWaitTimeUnit"), Some(unit));
            assert_eq!(record.attribute("ClientWaitTimeUnit"), Some(unit));
            assert_eq!(doc.fenet_config_infos()[1], other);
            assert_eq!(doc.ladder_programs().remove(0).unwrap().data, program);
            doc.edit_fenet_field(&patch(field, new, old)).unwrap();
            assert_eq!(doc.xml, original);
        }
    }
}

#[test]
fn rapienet_station_range_uses_the_existing_protocol() {
    let mut doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/networks/fenet-smart-native.xgwx"
    ))
    .unwrap();
    doc.xml = doc
        .xml
        .replacen("RapienetProtocol=\"0\"", "RapienetProtocol=\"1\"", 1);
    doc.edit_fenet_field(&patch("stationNo", "63", "220"))
        .unwrap();
    let original = doc.xml.clone();
    assert!(
        doc.edit_fenet_field(&patch("stationNo", "220", "221"))
            .is_err()
    );
    assert_eq!(doc.xml, original);
}
