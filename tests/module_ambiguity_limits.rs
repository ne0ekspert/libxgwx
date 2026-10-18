use flate2::{Compression, write::GzEncoder};
use std::io::Write;
use xgwx::{ConfigurationMatch, XgwxDocument, XgwxError};

fn bytes(xml: &str) -> Vec<u8> {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    bytes
}
fn ambiguous(count: usize) -> String {
    format!(
        "<Project><Configuration>PLC{}{}{}</Configuration></Project>",
        (0..count)
            .map(|i| format!("<Module Id='23041' Base='0' Slot='0' Name='H{i}'/>"))
            .collect::<String>(),
        (0..count)
            .map(|i| format!(
                "<XGPD_CONFIG_INFO_FENET Type='23041' Base='0' Slot='0' StationNo='{i}'/>"
            ))
            .collect::<String>(),
        "<NetworkModule Id='23041' Base='0' Slot='0'/>".repeat(count)
    )
}

#[test]
fn typed_module_cartesian_expansion_is_rejected_before_candidate_copies() {
    let input = bytes(&ambiguous(128));
    assert!(input.len() < 4096);
    match XgwxDocument::parse(&input) {
        Err(XgwxError::ResourceLimitExceeded { .. }) => {}
        Err(error) => panic!("unexpected rejection: {error}"),
        Ok(doc) => {
            let networks = doc.resolved_network_modules();
            let copies: usize = networks
                .iter()
                .map(|network| {
                    network.hardware_candidates.len()
                        + match &network.configuration {
                            ConfigurationMatch::Ambiguous(items) => items.len(),
                            _ => 0,
                        }
                })
                .sum();
            panic!("accepted compact input and materialized {copies} owned candidates");
        }
    }
}

#[test]
fn ordinary_ambiguity_keeps_all_candidates_and_original_order() {
    let doc = XgwxDocument::parse(&bytes(&ambiguous(8))).unwrap();
    let networks = doc.resolved_network_modules();
    assert_eq!(networks.len(), 8);
    for network in networks {
        assert!(network.base.is_none());
        assert_eq!(network.hardware_candidates.len(), 8);
        for (index, candidate) in network.hardware_candidates.iter().enumerate() {
            assert_eq!(
                candidate.hardware.name.as_deref(),
                Some(format!("H{index}").as_str())
            );
        }
        if let ConfigurationMatch::Ambiguous(items) = &network.configuration {
            for (index, candidate) in items.iter().enumerate() {
                let xgwx::NetworkConfiguration::Fenet(candidate) = candidate else {
                    panic!("wrong protocol")
                };
                assert_eq!(candidate.station_no, Some(index as u32));
            }
        }
        assert!(
            matches!(network.configuration, ConfigurationMatch::Ambiguous(ref items) if items.len() == 8)
        );
    }
    assert_eq!(doc.modules().len(), 16);
}

fn rejects(xml: &str, expected_resource: &str) {
    match XgwxDocument::parse(&bytes(xml)) {
        Err(XgwxError::ResourceLimitExceeded { resource, .. }) => {
            assert_eq!(resource, expected_resource)
        }
        Err(error) => panic!("expected {expected_resource}, got {error}"),
        Ok(_) => panic!("expected {expected_resource}, but parsing succeeded"),
    }
}

#[test]
fn cnet_port_attributes_are_charged_before_repeated_cloning() {
    let setting = format!(
        "<XGPD_CONFIG_INFO_CNET Type='1' Base='0' Slot='0'><XGPD_CONFIG_INFO_CNET_PORT Payload='{}'/></XGPD_CONFIG_INFO_CNET>",
        "x".repeat(128 * 1024)
    );
    let xml = format!(
        "<Project>{setting}{}</Project>",
        "<NetworkModule Id='1' Base='0' Slot='0'/>".repeat(80)
    );
    rejects(&xml, "typed module storage");
}

#[test]
fn unknown_cnet_children_do_not_count_as_copied_ports() {
    let setting = format!(
        "<XGPD_CONFIG_INFO_CNET Type='1' Base='0' Slot='0'><Unknown Payload='{}'/></XGPD_CONFIG_INFO_CNET>",
        "x".repeat(128 * 1024)
    );
    let xml = format!(
        "<Project>{setting}{}</Project>",
        "<NetworkModule Id='1' Base='0' Slot='0'/>".repeat(80)
    );
    let doc = XgwxDocument::parse(&bytes(&xml)).unwrap();
    assert_eq!(doc.resolved_network_modules().len(), 80);
    assert_eq!(
        doc.container().root().descendants_named("Unknown").count(),
        1
    );
}

#[test]
fn long_duplicate_configuration_names_have_a_finite_comparison_budget() {
    let name = "x".repeat(16 * 1024);
    let xml = format!(
        "<Project>{}{}</Project>",
        format!("<Configuration>{name}</Configuration>").repeat(64),
        format!("<NetworkModule ConfigName='{name}'/>").repeat(64)
    );
    rejects(&xml, "typed module work");
}

#[test]
fn nonmatching_candidates_still_have_a_finite_work_budget() {
    let xml = format!(
        "<Project>{}{}</Project>",
        "<Module Id='1' Base='0' Slot='0'/>".repeat(1500),
        "<NetworkModule Id='2' Base='0' Slot='0'/>".repeat(1500)
    );
    rejects(&xml, "typed module work");
}

#[test]
fn repeated_per_base_inventories_are_included_in_the_budget() {
    let xml = format!(
        "<Project>{}{}</Project>",
        "<Base/>".repeat(200),
        "<NetworkModule Id='1' Base='0' Slot='0'/>".repeat(100)
    );
    rejects(&xml, "typed module work");
}

#[test]
fn per_base_copies_of_owned_configuration_names_are_charged() {
    let xml = format!(
        "<Project><Configuration>{}{}<NetworkModule/></Configuration></Project>",
        "x".repeat(1024 * 1024),
        "<Base/>".repeat(20)
    );
    rejects(&xml, "typed module storage");
}

#[test]
fn numeric_aliases_and_optional_subtypes_cannot_bypass_the_limit() {
    let xml = ambiguous(128)
        .replace("Id='23041'", "Id=' 023041 '")
        .replace("Type='23041'", "Type=' 023041 ' SubType='7'")
        .replace("Slot='0'", "Slot=' 00 '");
    rejects(&xml, "typed module associations");
}

#[test]
fn exactly_the_association_limit_preserves_every_candidate() {
    let xml = format!(
        "<Project>{}{}</Project>",
        "<Module Id='1' Base='0' Slot='0'/>".repeat(64),
        "<NetworkModule Id='1' Base='0' Slot='0'/>".repeat(64)
    );
    let doc = XgwxDocument::parse(&bytes(&xml)).unwrap();
    let networks = doc.resolved_network_modules();
    assert_eq!(
        networks
            .iter()
            .map(|n| n.hardware_candidates.len())
            .sum::<usize>(),
        4096
    );
    rejects(
        &xml.replace(
            "</Project>",
            "<NetworkModule Id='1' Base='0' Slot='0'/></Project>",
        ),
        "typed module associations",
    );
}

#[cfg(feature = "write")]
#[test]
fn excessive_replacement_is_atomic() {
    let mut doc = XgwxDocument::parse(&bytes(&ambiguous(8))).unwrap();
    let original = doc.clone();
    assert!(matches!(
        doc.container_mut().replace_xml(ambiguous(128)),
        Err(XgwxError::ResourceLimitExceeded { .. })
    ));
    assert_eq!(doc, original);
}

#[cfg(all(feature = "ffi", not(target_arch = "wasm32")))]
#[test]
fn ffi_parse_rejects_expansion_without_publishing_a_handle() {
    use xgwx::ffi::*;
    let input = bytes(&ambiguous(128));
    let mut handle = std::ptr::null_mut();
    assert_eq!(
        unsafe { xgwx_document_parse(input.as_ptr(), input.len(), &mut handle) },
        XGWX_PARSE_ERROR
    );
    assert!(handle.is_null());
}
