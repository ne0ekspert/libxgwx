#![cfg(feature = "write")]
#[cfg(feature = "write")]
#[path = "support/hardware_support.rs"]
mod hardware_support;
use xgwx::{BrowserNetworkPatch, FenetFieldPatch, XgwxDocument};
fn project() -> XgwxDocument {
    let mut doc =
        XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgk.xgwx")).unwrap();
    doc.base_mut()
        .by_number(0)
        .and_then(|mut selected_record| selected_record.insert_module(0, "XGL-EFMT(B)"))
        .unwrap();
    doc.base_mut()
        .by_number(0)
        .and_then(|mut selected_record| selected_record.insert_module(1, "XGL-EFMT(B)"))
        .unwrap();
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
    let original = doc.container().xml().to_owned();
    let before = doc.program().ladder_programs().remove(0).unwrap().data;
    {
        let field_patch = &patch("ipAddress", "192.168.0.100", "10.20.30.40");
        hardware_support::edit_fenet_field(&mut doc, field_patch)
    }
    .unwrap();
    let ips = doc.network().fenet_config_infos();
    assert_eq!(ips[0].ip_address.as_ref().unwrap().address, "10.20.30.40");
    assert_eq!(ips[1].ip_address.as_ref().unwrap().address, "192.168.0.100");
    {
        let field_patch = &patch("ipAddress", "10.20.30.40", "192.168.0.100");
        hardware_support::edit_fenet_field(&mut doc, field_patch)
    }
    .unwrap();
    assert_eq!(doc.container().xml(), original);
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
        {
            let field_patch = &patch(field, old, new);
            hardware_support::edit_fenet_field(&mut doc, field_patch)
        }
        .unwrap();
        XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
        {
            let field_patch = &patch(field, new, old);
            hardware_support::edit_fenet_field(&mut doc, field_patch)
        }
        .unwrap();
        assert_eq!(doc.container().xml(), original);
    }
    assert_eq!(
        doc.program().ladder_programs().remove(0).unwrap().data,
        before
    );
}
#[test]
fn invalid_and_stale_fenet_edits_are_atomic() {
    let mut doc = project();
    let original = doc.container().xml().to_owned();
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
        assert!(
            {
                let field_patch = &p;
                hardware_support::edit_fenet_field(&mut doc, field_patch)
            }
            .is_err()
        );
        assert_eq!(doc.container().xml(), original);
    }
    let mut wrong = doc.clone();
    {
        let replacement_xml =
            wrong
                .container()
                .xml()
                .replacen("Type=\"23041\"", "Type=\"23042\"", 1);
        wrong.container_mut().replace_xml(replacement_xml).unwrap();
    };
    assert!(
        {
            let field_patch = &patch("dhcp", "0", "1");
            hardware_support::edit_fenet_field(&mut wrong, field_patch)
        }
        .is_err()
    );
}
#[test]
fn network_metadata_edits_preserve_linked_configuration() {
    let mut doc = project();
    let fenet = doc.network().fenet_config_infos();
    for (module, field, old, new) in [
        (false, "name", "기본 네트워크", "Home & Office"),
        (true, "alias", "", "Main PLC"),
        (true, "description", "", "Ethernet <uplink>"),
    ] {
        {
            let field_patch = &BrowserNetworkPatch {
                network_index: 0,
                module,
                base: Some(0),
                slot: Some(0),
                field: field.into(),
                expected_value: old.into(),
                replacement: new.into(),
            };
            hardware_support::edit_browser_network(&mut doc, field_patch)
        }
        .unwrap();
        assert_eq!(doc.network().fenet_config_infos(), fenet);
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
        {
            let field_patch = &patch(field, old, new);
            hardware_support::edit_fenet_field(&mut doc, field_patch)
        }
        .unwrap();
    }
    {
        let field_patch = &BrowserNetworkPatch {
            network_index: 0,
            module: true,
            base: Some(0),
            slot: Some(0),
            field: "alias".into(),
            expected_value: "".into(),
            replacement: "Main Ethernet".into(),
        };
        hardware_support::edit_browser_network(&mut doc, field_patch)
    }
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
    let native = doc.network().fenet_config_infos();
    assert_eq!(
        native[0].ip_address.as_ref().unwrap().address,
        "10.20.30.40"
    );
    assert_eq!(native[0].subnet.as_ref().unwrap().address, "255.255.0.0");
    assert_eq!(native[0].dhcp, Some(1));
    assert_eq!(
        doc.network().list()[0].modules[0].alias.as_deref(),
        Some("Main Ethernet")
    );
    let original = doc.container().xml().to_owned();
    {
        let field_patch = &patch("ipAddress", "10.20.30.40", "10.20.30.41");
        hardware_support::edit_fenet_field(&mut doc, field_patch)
    }
    .unwrap();
    assert_eq!(doc.network().fenet_config_infos()[1], native[1]);
    {
        let field_patch = &patch("ipAddress", "10.20.30.41", "10.20.30.40");
        hardware_support::edit_fenet_field(&mut doc, field_patch)
    }
    .unwrap();
    assert_eq!(doc.container().xml(), original);
}

#[test]
fn fenet_numeric_fields_edit_atomically_and_preserve_other_settings() {
    let mut doc = project();
    let original = doc.container().xml().to_owned();
    let other = doc.network().fenet_config_infos()[1].clone();
    for (field, old, new) in [
        ("stationNo", "0", "63"),
        ("driverType", "2", "5"),
        ("driverType", "2", "7"),
        ("rcvWaitTime", "100", "255"),
        ("clientWaitTime", "60", "2"),
        ("glofaSocketCount", "3", "16"),
    ] {
        {
            let field_patch = &patch(field, old, new);
            hardware_support::edit_fenet_field(&mut doc, field_patch)
        }
        .unwrap();
        assert_eq!(doc.network().fenet_config_infos()[1], other);
        XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
        {
            let field_patch = &patch(field, new, old);
            hardware_support::edit_fenet_field(&mut doc, field_patch)
        }
        .unwrap();
        assert_eq!(doc.container().xml(), original);
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
        assert!(
            {
                let field_patch = &patch(field, old, value);
                hardware_support::edit_fenet_field(&mut doc, field_patch)
            }
            .is_err()
        );
        assert_eq!(doc.container().xml(), original);
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
        let original = doc.container().xml().to_owned();
        let other = doc.network().fenet_config_infos()[1].clone();
        let program = doc.program().ladder_programs().remove(0).unwrap().data;
        for (field, old, new) in [
            ("driverType", driver, "2"),
            ("rcvWaitTime", receive, "100"),
            ("clientWaitTime", client, "60"),
        ] {
            {
                let field_patch = &patch(field, old, new);
                hardware_support::edit_fenet_field(&mut doc, field_patch)
            }
            .unwrap();
            let xml = roxmltree::Document::parse(&doc.container().xml()).unwrap();
            let record = xml
                .descendants()
                .find(|n| n.has_tag_name("XGPD_CONFIG_INFO_FENET"))
                .unwrap();
            assert_eq!(record.attribute("RcvWaitTimeUnit"), Some(unit));
            assert_eq!(record.attribute("ClientWaitTimeUnit"), Some(unit));
            assert_eq!(doc.network().fenet_config_infos()[1], other);
            assert_eq!(
                doc.program().ladder_programs().remove(0).unwrap().data,
                program
            );
            {
                let field_patch = &patch(field, new, old);
                hardware_support::edit_fenet_field(&mut doc, field_patch)
            }
            .unwrap();
            assert_eq!(doc.container().xml(), original);
        }
    }
}

#[test]
fn rapienet_station_range_uses_the_existing_protocol() {
    let mut doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/networks/fenet-smart-native.xgwx"
    ))
    .unwrap();
    {
        let replacement_xml =
            doc.container()
                .xml()
                .replacen("RapienetProtocol=\"0\"", "RapienetProtocol=\"1\"", 1);
        doc.container_mut().replace_xml(replacement_xml).unwrap();
    };
    {
        let field_patch = &patch("stationNo", "63", "220");
        hardware_support::edit_fenet_field(&mut doc, field_patch)
    }
    .unwrap();
    let original = doc.container().xml().to_owned();
    assert!(
        {
            let field_patch = &patch("stationNo", "220", "221");
            hardware_support::edit_fenet_field(&mut doc, field_patch)
        }
        .is_err()
    );
    assert_eq!(doc.container().xml(), original);
}
