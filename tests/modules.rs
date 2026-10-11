use flate2::{Compression, write::GzEncoder};
use std::io::Write;
use xgwx::{ConfigurationMatch, IOKind, Module, NetworkConfiguration, XgwxDocument};

fn document(xml: &str) -> XgwxDocument {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    XgwxDocument::parse(&bytes).unwrap()
}

fn configuration(name: &str, slot: u32, ip: &str) -> String {
    let octets: Vec<_> = ip.split('.').collect();
    let [a, b, c, d] = octets.as_slice() else {
        panic!("invalid test IP")
    };
    format!(
        r#"<Configuration>{name}
      <Base Base="0"><Module Id="23041" SubType="32769" Base="0" Slot="{slot}" /></Base>
      <XGPD_CONFIG_INFO_FENET Type="23041" SubType="32769" Base="0" Slot="{slot}"
        IpAddr_0="{a}" IpAddr_1="{b}" IpAddr_2="{c}" IpAddr_3="{d}">
      </XGPD_CONFIG_INFO_FENET>
    </Configuration>"#
    )
}

fn network(name: &str, slot: u32) -> String {
    format!(
        r#"<Network><NetworkModule ConfigName="{name}" Id="23041"
      OptionType="32769" Base="0" Slot="{slot}" /></Network>"#
    )
}

#[test]
fn default_feature_views_classify_by_numeric_identity_and_preserve_unknown_hardware() {
    let doc = document(
        r#"<Project><Configuration>PLC<Base Base="0">
      <Module Id="42242" SubType="0" Base="0" Slot="0" Name="renamed" />
      <Module Id="23169" SubType="0" Base="0" Slot="1" />
      <Module Id="999999" SubType="0" Base="0" Slot="2" Name="입력 모듈:XGI-D24A/B" />
      <Module Id="42242" SubType="99" Base="0" Slot="3" />
      <Module Id="42242" Base="0" Slot="4" />
    </Base></Configuration></Project>"#,
    );
    let before = doc.clone();
    assert_eq!(doc.hardware_modules().len(), 5);
    assert_eq!(
        doc.io_modules()
            .iter()
            .map(|io| io.kind)
            .collect::<Vec<_>>(),
        [IOKind::DigitalInput, IOKind::AnalogInput]
    );
    let modules = doc.modules();
    assert_eq!(modules.len(), 5);
    assert!(matches!(modules[0], Module::Io(_)));
    assert!(matches!(modules[2], Module::Other(_)));
    assert!(matches!(modules[3], Module::Other(_)));
    assert!(matches!(modules[4], Module::Other(_)));
    assert_eq!(doc, before);
}

#[test]
fn network_configuration_and_hardware_are_scoped_to_the_named_plc() {
    let doc = document(&format!(
        "<Project>{}{}{}{}</Project>",
        configuration("PLC-A", 0, "10.0.0.1"),
        configuration("PLC-B", 0, "10.0.0.2"),
        network("PLC-B", 0),
        network("PLC-A", 0)
    ));
    let networks = doc.resolved_network_modules();
    assert_eq!(networks.len(), 2);
    for (network, scope, ip) in [(&networks[0], 1, "10.0.0.2"), (&networks[1], 0, "10.0.0.1")] {
        assert_eq!(network.configuration_index, Some(scope));
        assert_eq!(
            network.base.as_ref().unwrap().configuration_index,
            Some(scope)
        );
        assert!(network.hardware_candidates.is_empty());
        let ConfigurationMatch::Matched(NetworkConfiguration::Fenet(config)) =
            &network.configuration
        else {
            panic!("expected unique FEnet configuration")
        };
        assert_eq!(config.ip_address.as_ref().unwrap().address, ip);
        assert_eq!(network.fenet_config_info(), Some(config.as_ref()));
        assert!(network.cnet_config_info().is_none());
    }
    assert!(
        doc.modules()
            .iter()
            .all(|m| matches!(m, Module::Network(_)))
    );
}

#[test]
fn network_namespace_retains_unassociated_settings_in_document_order() {
    let doc = document(&format!(
        "<Project>{}{}</Project>",
        configuration("PLC-A", 0, "10.0.0.1"),
        configuration("PLC-B", 1, "10.0.0.2")
    ));
    let before = doc.clone();
    let network = doc.network();
    assert!(network.list().is_empty());
    assert!(network.modules().is_empty());
    assert!(network.cnet_config_infos().is_empty());
    let configs = network.fenet_config_infos();
    assert_eq!(configs.len(), 2);
    assert_eq!(configs[0].ip_address.as_ref().unwrap().address, "10.0.0.1");
    assert_eq!(configs[1].ip_address.as_ref().unwrap().address, "10.0.0.2");
    assert_eq!(doc, before);
}

#[test]
fn repeated_module_types_are_disambiguated_by_current_location() {
    let doc = XgwxDocument::from_path("fixtures/networks/cnetbase-native.xgwx").unwrap();
    let networks = doc.resolved_network_modules();
    assert_eq!(networks.len(), 3);
    assert_eq!(doc.network().cnet_config_infos().len(), 1);
    assert_eq!(doc.network().fenet_config_infos().len(), 2);
    for network in networks {
        assert_eq!(
            network.base.as_ref().unwrap().hardware.slot,
            network.metadata.slot
        );
        match &network.configuration {
            ConfigurationMatch::Matched(NetworkConfiguration::Fenet(config)) => {
                assert_eq!(config.slot, network.metadata.slot);
                assert_eq!(network.fenet_config_info(), Some(config.as_ref()));
                assert!(network.cnet_config_info().is_none());
            }
            ConfigurationMatch::Matched(NetworkConfiguration::Cnet(config)) => {
                assert_eq!(config.slot, network.metadata.slot);
                assert_eq!(config.ports.len(), 2);
                assert_eq!(network.cnet_config_info(), Some(config));
                assert!(network.fenet_config_info().is_none());
            }
            other => panic!("unexpected configuration {other:?}"),
        }
    }
    assert_eq!(doc.modules().len(), 3);
}

#[test]
fn built_in_network_interface_without_hardware_is_retained() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").unwrap();
    let networks = doc.resolved_network_modules();
    let raw: Vec<xgwx::ModuleSummary> = doc.module().summaries();
    let all: Vec<Module> = doc.modules();
    assert_eq!(raw.len(), 2);
    assert_eq!(all.len(), 3);
    let cnet = networks
        .iter()
        .find(|n| n.metadata.id == Some(23104))
        .unwrap();
    assert!(cnet.base.is_none());
    assert!(cnet.hardware_candidates.is_empty());
    assert!(matches!(
        cnet.configuration,
        ConfigurationMatch::Matched(NetworkConfiguration::Cnet(_))
    ));
    assert_eq!(doc.modules().len(), 3);
}

#[test]
fn duplicate_configuration_names_remain_ambiguous() {
    let doc = document(&format!(
        "<Project>{}{}{}</Project>",
        configuration("PLC", 0, "10.0.0.1"),
        configuration("PLC", 0, "10.0.0.2"),
        network("PLC", 0)
    ));
    let network = doc.resolved_network_modules().remove(0);
    assert_eq!(network.configuration_index, None);
    assert!(network.fenet_config_info().is_none());
    assert!(network.cnet_config_info().is_none());
    assert!(network.base.is_none());
    assert_eq!(network.hardware_candidates.len(), 2);
    assert!(matches!(network.configuration, ConfigurationMatch::Ambiguous(ref c) if c.len() == 2));
    assert_eq!(doc.modules().len(), 3);
}

#[test]
fn ambiguous_scope_is_not_resolved_by_a_single_matching_setting() {
    let doc = document(&format!(
        "<Project>{}<Configuration>PLC</Configuration>{}</Project>",
        configuration("PLC", 0, "10.0.0.1"),
        network("PLC", 0)
    ));
    let network = doc.resolved_network_modules().remove(0);
    assert!(network.base.is_none());
    assert!(matches!(network.configuration, ConfigurationMatch::Ambiguous(ref c) if c.len() == 1));
}

#[test]
fn unknown_config_name_does_not_fall_back_to_another_plc() {
    let doc = document(&format!(
        "<Project>{}{}</Project>",
        configuration("PLC", 0, "10.0.0.1"),
        network("wrong", 0)
    ));
    let network = doc.resolved_network_modules().remove(0);
    assert!(network.base.is_none());
    assert!(matches!(network.configuration, ConfigurationMatch::Missing));
}

#[test]
fn missing_config_name_only_resolves_when_scope_is_unique() {
    let network = r#"<NetworkModule Id="23041" Base="0" Slot="0" />"#;
    let one = document(&format!(
        "<Project>{}{network}</Project>",
        configuration("PLC-A", 0, "10.0.0.1")
    ));
    assert!(matches!(
        one.resolved_network_modules()[0].configuration,
        ConfigurationMatch::Matched(_)
    ));
    let two = document(&format!(
        "<Project>{}{}{network}</Project>",
        configuration("PLC-A", 0, "10.0.0.1"),
        configuration("PLC-B", 0, "10.0.0.2")
    ));
    assert!(matches!(
        two.resolved_network_modules()[0].configuration,
        ConfigurationMatch::Ambiguous(_)
    ));
}

#[test]
fn explicit_xml_owner_takes_precedence_over_global_name_lookup() {
    let doc = document(
        r#"<Project>
      <Configuration>PLC<NetworkModule Id="23041" Base="0" Slot="0" />
        <XGPD_CONFIG_INFO_FENET Type="23041" Base="0" Slot="0" />
      </Configuration>
      <Configuration>PLC<XGPD_CONFIG_INFO_FENET Type="23041" Base="0" Slot="0" /></Configuration>
    </Project>"#,
    );
    let network = doc.resolved_network_modules().remove(0);
    assert_eq!(network.configuration_index, Some(0));
    assert!(matches!(
        network.configuration,
        ConfigurationMatch::Matched(_)
    ));
}

#[test]
fn duplicate_records_and_hardware_candidates_are_preserved() {
    let doc = document(
        r#"<Project><Configuration>PLC
      <Module Id="23041" Base="0" Slot="0" />
      <Module Id="23041" Base="0" Slot="0" />
      <XGPD_CONFIG_INFO_FENET Type="23041" Base="0" Slot="0" />
      <XGPD_CONFIG_INFO_FENET Type="23041" Base="0" Slot="0" />
    </Configuration><NetworkModule ConfigName="PLC" Id="23041" Base="0" Slot="0" /></Project>"#,
    );
    let network = doc.resolved_network_modules().remove(0);
    assert!(network.base.is_none());
    assert_eq!(network.hardware_candidates.len(), 2);
    assert!(matches!(network.configuration, ConfigurationMatch::Ambiguous(ref c) if c.len() == 2));
    assert_eq!(doc.modules().len(), 3);
}

#[test]
fn missing_identity_and_wrong_subtype_or_location_do_not_match() {
    for attrs in [
        "",
        r#"Id="23041" Base="0" Slot="0" OptionType="99""#,
        r#"Id="23041" Base="0" Slot="1""#,
    ] {
        let doc = document(&format!(
            "<Project>{}<NetworkModule ConfigName=\"PLC\" {attrs} /></Project>",
            configuration("PLC", 0, "10.0.0.1")
        ));
        let network = doc.resolved_network_modules().remove(0);
        assert!(network.base.is_none());
        assert!(matches!(network.configuration, ConfigurationMatch::Missing));
    }
}

#[test]
fn other_protocol_attributes_are_preserved_and_unsupported_hardware_stays_other() {
    let doc = XgwxDocument::from_path("fixtures/elements-io.xgwx").unwrap();
    assert_eq!(doc.hardware_modules().len(), doc.module().summaries().len());
    let networks = doc.resolved_network_modules();
    assert!(!networks.is_empty());
    for network in networks {
        let ConfigurationMatch::Matched(NetworkConfiguration::Other(config)) =
            network.configuration
        else {
            panic!("expected other protocol settings")
        };
        assert!(!config.attributes.is_empty());
        assert_eq!(config.type_code, network.metadata.id);
    }
    assert!(doc.modules().iter().any(|m| matches!(m, Module::Other(_))));
}

#[test]
fn unscoped_records_and_duplicate_network_memberships_are_not_lost() {
    let doc = document(
        r#"<Project><Module Id="23041" Base="0" Slot="0" />
      <XGPD_CONFIG_INFO_FENET Type="23041" Base="0" Slot="0" />
      <NetworkModule Id="23041" Base="0" Slot="0" />
      <NetworkModule Id="23041" Base="0" Slot="0" />
    </Project>"#,
    );
    assert_eq!(doc.hardware_modules()[0].configuration_index, None);
    assert_eq!(doc.modules().len(), 2);
    assert!(
        doc.modules()
            .iter()
            .all(|m| matches!(m, Module::Network(_)))
    );
}

#[test]
fn base_handles_scope_typed_modules_to_their_xml_owner_and_plc() {
    let a = configuration("PLC-A", 0, "10.0.0.1").replace("</Base>",
        r#"<Module Id="42242" SubType="0" Base="0" Slot="1"/><Module Id="999999" Base="0" Slot="2"/></Base>"#);
    let doc = document(&format!(
        "<Project>{a}{}{}{}</Project>",
        configuration("PLC-B", 0, "10.0.0.2"),
        network("PLC-B", 0),
        network("PLC-A", 0)
    ));
    let before = doc.clone();
    let bases = doc.bases();
    assert_eq!(bases.len(), 2);
    assert_eq!(
        bases.iter().map(|b| b.summary()).collect::<Vec<_>>(),
        doc.base().summaries()
    );
    for (index, ip, count) in [(0, "10.0.0.1", 3), (1, "10.0.0.2", 1)] {
        let base = doc.base().get(index).unwrap();
        assert_eq!(base.index(), index);
        assert_eq!(base.number(), Some(0));
        assert_eq!(base.configuration_index(), Some(index));
        let modules = base.modules();
        assert_eq!(modules.len(), count);
        let Module::Network(network) = &modules[0] else {
            panic!("network module")
        };
        assert_eq!(
            network
                .fenet_config_info()
                .unwrap()
                .ip_address
                .as_ref()
                .unwrap()
                .address,
            ip
        );
    }
    let modules = bases[0].modules();
    assert!(matches!(modules[1], Module::Io(_)));
    assert!(matches!(modules[2], Module::Other(_)));
    assert_eq!(doc, before);
    assert!(matches!(
        doc.base().get(2),
        Err(xgwx::XgwxError::BaseIndexNotFound { index: 2 })
    ));
}

#[test]
fn physical_base_ownership_wins_over_missing_or_conflicting_module_numbers() {
    let doc = document(
        r#"<Project><Configuration>PLC
      <Base Base="7" SlotCount="4"><Module Id="42242" SubType="0" Slot="1"/></Base>
      <Base Base="7" SlotCount="6"><Module Id="999999" Base="99" Slot="0"/></Base>
      <Network><NetworkModule Id="23041" Base="7" Slot="2"/></Network>
    </Configuration></Project>"#,
    );
    let bases = doc.bases();
    assert_eq!(bases[0].slot_count(), Some(4));
    assert_eq!(bases[1].slot_count(), Some(6));
    assert_eq!(bases[0].modules().len(), 1);
    assert_eq!(bases[1].modules().len(), 1);
    assert!(matches!(bases[0].modules()[0], Module::Io(_)));
    assert!(matches!(bases[1].modules()[0], Module::Other(_)));
    // The network membership's location is ambiguous between two base records.
    assert_eq!(doc.modules().len(), 3);
}

#[test]
fn unplaced_network_records_require_a_unique_base_and_configuration() {
    let doc = document(
        r#"<Project>
      <Configuration>PLC-A<Base Base="0"/></Configuration>
      <Configuration>PLC-B<Base Base="0"/></Configuration>
      <Network>
        <NetworkModule ConfigName="PLC-B" Id="23104" Base="0" Slot="0"/>
        <NetworkModule Id="23041" Base="0" Slot="1"/>
      </Network>
    </Project>"#,
    );
    assert!(doc.bases()[0].modules().is_empty());
    let modules = doc.bases()[1].modules();
    assert_eq!(modules.len(), 1);
    let Module::Network(network) = &modules[0] else {
        panic!("network module")
    };
    assert_eq!(network.metadata.id, Some(23104));
    assert!(network.base.is_none());
    assert_eq!(doc.modules().len(), 2);
}

#[test]
fn nested_bases_and_unscoped_hardware_remain_distinct_records() {
    let doc = document(
        r#"<Project>
      <Base Base="1"><Module Id="999901" Slot="0"/>
        <Base Base="2"><Module Id="999902" Slot="0"/></Base>
      </Base>
      <Module Id="999903" Base="1" Slot="1"/>
      <Module Id="999904" Slot="2"/>
    </Project>"#,
    );
    let bases = doc.bases();
    assert_eq!(bases.len(), 2);
    assert_eq!(bases[0].configuration_index(), None);
    assert_eq!(bases[0].modules().len(), 2);
    assert_eq!(bases[1].modules().len(), 1);
    assert_eq!(doc.modules().len(), 4);
    let Module::Other(hardware) = &bases[1].modules()[0] else {
        panic!("hardware")
    };
    assert_eq!(hardware.hardware.id, Some(999902));
}
