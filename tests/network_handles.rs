use flate2::{Compression, write::GzEncoder};
use std::io::Write;
use xgwx::XgwxDocument;
fn document(xml: &str) -> XgwxDocument {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    XgwxDocument::parse(&bytes).unwrap()
}
#[test]
fn memberships_are_scoped_by_parent_and_keep_document_indices() {
    let doc = document(
        r#"<Project><Network Name="A"><NetworkModule Base="0" Slot="1"/></Network><Network Name="B"><NetworkModule Base="0" Slot="1"/></Network><NetworkModule Base="3" Slot="4"/></Project>"#,
    );
    assert_eq!(doc.network().get(0).unwrap().module(0).unwrap().index(), 0);
    assert_eq!(doc.network().get(1).unwrap().module(0).unwrap().index(), 1);
    assert_eq!(
        doc.network()
            .module(1)
            .unwrap()
            .network()
            .unwrap()
            .summary()
            .name
            .as_deref(),
        Some("B")
    );
    assert!(doc.network().module(2).unwrap().network().is_none());
    assert!(doc.network().module_at(0, 1).is_err());
    assert!(doc.network().get(2).is_err());
    assert!(doc.network().get(0).unwrap().module(1).is_err());
}
#[cfg(feature = "write")]
mod writes {
    use super::*;
    use xgwx::{CnetFieldEdit, NetworkFieldEdit, NetworkModulePatch, NetworkPatch};
    fn edit(field: &str, old: &str, new: &str) -> NetworkFieldEdit {
        NetworkFieldEdit {
            field: field.into(),
            expected_value: old.into(),
            replacement: new.into(),
        }
    }
    fn project() -> XgwxDocument {
        let mut doc = xgwx::create_project("XGK-CPUSN", "LD").unwrap();
        doc.base_mut()
            .by_number(0)
            .unwrap()
            .insert_module(0, "XGL-EFMT(B)")
            .unwrap();
        doc.base_mut()
            .by_number(0)
            .unwrap()
            .insert_module(1, "XGL-EFMT(B)")
            .unwrap();
        doc
    }
    #[test]
    fn selected_metadata_and_fenet_edits_rebind_and_preserve_other_records() {
        let mut doc = project();
        let programs: Vec<_> = doc
            .program()
            .ladder_programs()
            .into_iter()
            .map(Result::unwrap)
            .collect();
        let other = doc.network().fenet_config_infos()[1].clone();
        let mut network = doc.network_mut().get(0).unwrap();
        network
            .update(&NetworkPatch {
                name: Some("Office & factory".into()),
                ..Default::default()
            })
            .unwrap();
        network
            .edit_field(&edit("name", "Office & factory", "Factory"))
            .unwrap();
        let mut module = network.module(0).unwrap();
        module
            .update(&NetworkModulePatch {
                alias: Some("Uplink".into()),
                ..Default::default()
            })
            .unwrap();
        module
            .edit_field(&edit("alias", "Uplink", "Main uplink"))
            .unwrap();
        let mut fenet = module.fenet().unwrap();
        fenet
            .edit_field(&edit("ipAddress", "192.168.0.100", "10.20.30.40"))
            .unwrap();
        assert_eq!(fenet.summary().ip_address.unwrap().address, "10.20.30.40");
        assert_eq!(doc.network().fenet_config_infos()[1], other);
        assert_eq!(
            doc.program()
                .ladder_programs()
                .into_iter()
                .map(Result::unwrap)
                .collect::<Vec<_>>(),
            programs
        );
        assert_eq!(
            XgwxDocument::parse(&doc.to_verified_bytes().unwrap())
                .unwrap()
                .container()
                .root(),
            doc.container().root()
        );
    }
    #[test]
    fn stale_and_invalid_protocol_edits_are_atomic() {
        let mut doc = project();
        let original = doc.container().xml().to_owned();
        for change in [
            edit("ipAddress", "stale", "10.0.0.1"),
            edit("ipAddress", "192.168.0.100", "999.0.0.1"),
        ] {
            assert!(
                doc.network_mut()
                    .module_at(0, 0)
                    .unwrap()
                    .fenet()
                    .unwrap()
                    .edit_field(&change)
                    .is_err()
            );
            assert_eq!(doc.container().xml(), original);
        }
        assert!(doc.network_mut().module_at(0, 0).unwrap().cnet().is_err());
        assert!(
            doc.network_mut()
                .get(0)
                .unwrap()
                .edit_field(&edit("name", "stale", "X"))
                .is_err()
        );
        assert_eq!(doc.container().xml(), original);
    }
    #[test]
    fn ambiguous_memberships_and_inconsistent_trees_refuse_writes() {
        let mut doc = document(
            r#"<Project><Network Name="A"><NetworkModule Base="0" Slot="1" Alias=""/></Network><Network Name="B"><NetworkModule Base="0" Slot="1" Alias=""/></Network></Project>"#,
        );
        let before = doc.container().xml().to_owned();
        assert!(
            doc.network_mut()
                .get(0)
                .unwrap()
                .module(0)
                .unwrap()
                .update(&NetworkModulePatch {
                    alias: Some("X".into()),
                    ..Default::default()
                })
                .is_err()
        );
        assert_eq!(doc.container().xml(), before);
    }
    #[test]
    fn cnet_batch_is_bound_and_atomic_on_native_fixture() {
        let mut doc =
            XgwxDocument::parse(include_bytes!("../fixtures/networks/cnetbase-native.xgwx"))
                .unwrap();
        let config = doc
            .network()
            .module_at(0, 2)
            .unwrap()
            .typed()
            .cnet_config_info()
            .unwrap()
            .clone();
        let old = config.ports[0]
            .attributes
            .iter()
            .find(|a| a.name == "StationNo")
            .unwrap()
            .value
            .clone();
        let before = doc.container().xml().to_owned();
        let good = CnetFieldEdit {
            port_index: 0,
            field: "stationNo".into(),
            expected_value: old,
            replacement: "5".into(),
        };
        let bad = CnetFieldEdit {
            port_index: 1,
            field: "stationNo".into(),
            expected_value: "stale".into(),
            replacement: "6".into(),
        };
        assert!(
            doc.network_mut()
                .module_at(0, 2)
                .unwrap()
                .cnet()
                .unwrap()
                .edit_settings(&[good.clone(), bad])
                .is_err()
        );
        assert_eq!(doc.container().xml(), before);
        doc.network_mut()
            .module_at(0, 2)
            .unwrap()
            .cnet()
            .unwrap()
            .edit_settings(&[good])
            .unwrap();
        doc.to_verified_bytes().unwrap();
    }
    #[test]
    fn duplicated_configuration_keeps_protocol_selection_ambiguous() {
        let doc = project();
        let xml = roxmltree::Document::parse(&doc.container().xml()).unwrap();
        let record = xml
            .descendants()
            .find(|n| n.has_tag_name("XGPD_CONFIG_INFO_FENET"))
            .unwrap();
        let mut expanded = doc.container().xml().to_owned();
        expanded.insert_str(record.range().end, &doc.container().xml()[record.range()]);
        let mut doc = document(&expanded);
        let before = doc.container().xml().to_owned();
        assert!(doc.network_mut().module_at(0, 0).unwrap().fenet().is_err());
        assert_eq!(doc.container().xml(), before);
    }
}
