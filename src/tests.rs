use super::*;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::env;
use std::io::Write;

#[test]
fn parses_synthetic_workspace() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Project FileVer="1.2.3.4" GUID="synthetic-guid" Version="513" Attribute="7" WksNodeCount="1">Synthetic
  <Configuration GUID="config-guid">PLC</Configuration>
  <Network Name="Network" Type="Ethernet" NetworkType="FEnet">
    <NetworkModule ConfigName="PLC" Base="0" Slot="1" ID="42" />
  </Network>
  <Program Task="scan" ObjectID="1" Version="2" Kind="0">Main</Program>
</Project>"#;
    let doc = XgwxDocument::parse(&synthetic_xgwx_bytes(xml)).expect("synthetic parses");

    assert!(doc.header.gzip_offset > 0);
    assert_eq!(doc.header.label.as_deref(), Some("XG5000 WORKSPACE FILE"));
    assert_eq!(doc.header.label_following_u32, Some(1));
    assert_eq!(doc.root.name, "Project");
    assert_eq!(doc.project_name(), Some("Synthetic"));
    assert_eq!(doc.project_info().file_version.as_deref(), Some("1.2.3.4"));
    assert_eq!(doc.configurations().len(), 1);
    assert_eq!(doc.networks()[0].modules.len(), 1);
    assert_eq!(doc.programs()[0].name.as_deref(), Some("Main"));
}

#[test]
fn reports_base64_payload_lengths() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Project>
  <TableData dt="bin.base64">
    YW
    Jj
  </TableData>
</Project>"#;
    let doc = XgwxDocument::parse(&synthetic_xgwx_bytes(xml)).expect("synthetic parses");
    let payload = doc
        .decoded_payloads()
        .into_iter()
        .next()
        .expect("payload exists")
        .expect("payload decodes");

    assert_eq!(payload.encoded_len, 4);
    assert_eq!(payload.raw_len, 3);
    assert_eq!(payload.decoded_len, 3);
    assert_eq!(payload.data, b"abc");
}

#[test]
fn decodes_xgi_d24_input_filter() {
    let doc = XgwxDocument::from_path("fixtures/elements.xgwx").expect("fixture parses");
    let module = doc
        .modules()
        .into_iter()
        .find(|module| {
            module
                .name
                .as_deref()
                .is_some_and(|name| name.contains("XGI-D24A/B"))
        })
        .expect("fixture has an XGI-D24A/B module");

    assert_eq!(module.input_filter_raw, Some(0));
    assert_eq!(module.input_filter, Some(ModuleInputFilter::Default));
    assert_eq!(
        module.input_filter.and_then(|value| value.milliseconds()),
        None
    );

    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Project>
  <Parameter Type="IO PARAMETER">
    <Module Name="XGI-D24A/B" Details="0500000000000000" />
  </Parameter>
</Project>"#;
    let doc = XgwxDocument::parse(&synthetic_xgwx_bytes(xml)).expect("synthetic parses");
    let module = doc.modules().into_iter().next().expect("module exists");

    assert_eq!(module.input_filter_raw, Some(5));
    assert_eq!(module.input_filter, Some(ModuleInputFilter::Ms5));
    assert_eq!(
        module.input_filter.and_then(|value| value.milliseconds()),
        Some(5)
    );
}

#[test]
fn maps_xgi_d24_input_filter_steps() {
    let expected = [
        (0, ModuleInputFilter::Default, None),
        (1, ModuleInputFilter::Ms1, Some(1)),
        (3, ModuleInputFilter::Ms3, Some(3)),
        (5, ModuleInputFilter::Ms5, Some(5)),
        (10, ModuleInputFilter::Ms10, Some(10)),
        (20, ModuleInputFilter::Ms20, Some(20)),
        (70, ModuleInputFilter::Ms70, Some(70)),
        (100, ModuleInputFilter::Ms100, Some(100)),
    ];

    for (raw, filter, milliseconds) in expected {
        assert_eq!(ModuleInputFilter::from_raw(raw), filter);
        assert_eq!(filter.milliseconds(), milliseconds);
    }
    assert_eq!(
        ModuleInputFilter::from_raw(4),
        ModuleInputFilter::Unknown(4)
    );
}

#[cfg(feature = "write")]
#[test]
fn preserves_unchanged_bytes_and_rewrites_supported_workspace() {
    let source = std::fs::read("fixtures/elements.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    let original_aligned_len = usize::try_from(
        doc.header
            .compressed_size_hint
            .expect("fixture has a size hint"),
    )
    .expect("size hint fits usize");
    let original_padding_len = original_aligned_len - doc.main_gzip.len();
    assert_eq!(original_padding_len, 3, "fixture exercises alignment");
    assert!(
        doc.trailer[..original_padding_len]
            .iter()
            .all(|byte| *byte == 0)
    );
    let original_payloads = doc
        .decoded_payloads()
        .into_iter()
        .map(|payload| payload.expect("fixture payload decodes"))
        .collect::<Vec<_>>();

    assert_eq!(doc.to_bytes().expect("unchanged document writes"), source);
    assert_eq!(
        doc.header.compressed_size_hint,
        u32::try_from((doc.main_gzip.len() + 3) & !3).ok()
    );

    doc.update_module(
        0,
        0,
        &ModulePatch {
            comment: Some("VS Code & XG5000 <module> \"edit\"".to_owned()),
            ..ModulePatch::default()
        },
    )
    .expect("module comment updates");
    doc.set_module_input_filter(0, 0, ModuleInputFilter::Ms5)
        .expect("input filter updates");

    let module = doc
        .modules()
        .into_iter()
        .find(|module| module.base == Some(0) && module.slot == Some(0))
        .expect("edited module exists");

    assert_eq!(
        module.comment.as_deref(),
        Some("VS Code & XG5000 <module> \"edit\"")
    );
    assert_eq!(module.input_filter_raw, Some(5));
    assert_eq!(module.input_filter, Some(ModuleInputFilter::Ms5));
    let rewritten = doc.to_bytes().expect("edited document writes");
    let rewritten_doc = XgwxDocument::parse(&rewritten).expect("rewritten document parses");
    let rewritten_module = rewritten_doc
        .modules()
        .into_iter()
        .find(|module| module.base == Some(0) && module.slot == Some(0))
        .expect("rewritten module exists");
    assert_eq!(
        rewritten_module.comment.as_deref(),
        Some("VS Code & XG5000 <module> \"edit\"")
    );
    assert_eq!(rewritten_module.input_filter_raw, Some(5));

    let rewritten_aligned_len = usize::try_from(
        rewritten_doc
            .header
            .compressed_size_hint
            .expect("rewritten workspace has size hint"),
    )
    .expect("size hint fits usize");
    let expected_checksum = rewritten[68..134]
        .iter()
        .chain(rewritten[138..138 + rewritten_aligned_len].iter())
        .fold(
            u32::try_from(rewritten_aligned_len).expect("aligned length fits u32"),
            |sum, byte| sum.wrapping_add(u32::from(*byte)),
        );
    assert_eq!(
        u32::from_le_bytes(rewritten[64..68].try_into().expect("checksum field")),
        expected_checksum
    );

    let original_metadata = &source[138 + original_aligned_len..];
    let rewritten_metadata = &rewritten[138 + rewritten_aligned_len..];
    assert_eq!(rewritten_metadata, original_metadata);
    assert_eq!(
        rewritten_doc
            .decoded_payloads()
            .into_iter()
            .map(|payload| payload.expect("rewritten payload decodes"))
            .collect::<Vec<_>>(),
        original_payloads
    );
}

#[cfg(feature = "write")]
#[test]
fn validates_recovered_xg_security_crc64() {
    let doc = XgwxDocument::from_path("fixtures/elements-io-resaved-filter5.xgwx")
        .expect("XG5000-resaved fixture parses");
    let security = doc
        .trailer_gzip_members
        .iter()
        .find(|member| member.data.starts_with(b"HEAD"))
        .expect("security member exists");

    assert!(crate::writer::validate_xg_frame(&security.data));
    assert_eq!(
        crate::writer::xg_crc64(&security.data[0x10..0x34]).to_le_bytes(),
        security.data[0x34..0x3c]
    );
    assert_eq!(
        crate::writer::xg_crc64(&security.data[8..0xc4]).to_le_bytes(),
        security.data[0xc4..0xcc]
    );
}

#[cfg(feature = "write")]
#[test]
fn module_writer_rejects_invalid_or_unsafe_edits_without_mutating_xml() {
    let source = std::fs::read("fixtures/elements-io.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    let original_xml = doc.xml.clone();

    let invalid_details = doc
        .update_module(
            0,
            2,
            &ModulePatch {
                details: Some("not-hex".to_owned()),
                ..ModulePatch::default()
            },
        )
        .expect_err("invalid Details must fail");
    assert!(matches!(
        invalid_details,
        XgwxError::InvalidHexPayload { .. }
    ));
    assert_eq!(doc.xml, original_xml);

    let missing = doc
        .update_module(99, 99, &ModulePatch::default())
        .expect_err("missing module must fail");
    assert!(matches!(
        missing,
        XgwxError::ModuleNotFound { base: 99, slot: 99 }
    ));
    assert_eq!(doc.xml, original_xml);

    let wrong_filter_target = doc
        .set_module_input_filter(0, 0, ModuleInputFilter::Ms5)
        .expect_err("wrong module type must fail");
    assert!(matches!(
        wrong_filter_target,
        XgwxError::InvalidModuleInputFilterTarget { base: 0, slot: 0 }
    ));
    assert_eq!(doc.xml, original_xml);
}

#[test]
fn cpu_catalog_maps_fixture_configuration_types() {
    assert_eq!(cpu_for_type(17).map(|entry| entry.model), Some("XGK-CPUSN"));
    assert_eq!(cpu_for_type(2).map(|entry| entry.model), Some("XGB-XBMS"));
    assert!(cpu_catalog().iter().any(|entry| entry.model == "XGB-XBMH2"));
}

#[cfg(feature = "write")]
#[test]
fn selects_cpu_type_and_preserves_other_configuration_attributes() {
    let source = std::fs::read("fixtures/elements.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    let before = doc.configurations().remove(0);

    doc.select_cpu("xgk-cpuhn")
        .expect("CPU matching is ASCII-case-insensitive");
    let after = doc.configurations().remove(0);
    assert_eq!(after.type_code, Some(16));
    assert_eq!(after.name, before.name);
    assert_eq!(after.attribute, before.attribute);
    assert_eq!(after.guid, before.guid);
    assert_eq!(after.write_signature, before.write_signature);

    let rewritten = doc.to_bytes().expect("selected workspace writes");
    let reparsed = XgwxDocument::parse(&rewritten).expect("selected workspace reparses");
    assert_eq!(reparsed.configurations()[0].type_code, Some(16));
}

#[cfg(feature = "write")]
#[test]
fn cpu_writer_rejects_unknown_models_without_mutating_xml() {
    let source = std::fs::read("fixtures/elements.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    let original_xml = doc.xml.clone();

    let error = doc
        .select_cpu("XGK-NOT-A-CPU")
        .expect_err("unknown CPU must fail");
    assert!(matches!(error, XgwxError::UnknownCpuModel { .. }));
    assert_eq!(doc.xml, original_xml);
}

#[cfg(feature = "write")]
#[test]
fn selects_module_from_embedded_xgk_catalog() {
    let catalog = xgk_module_catalog();
    assert_eq!(catalog.len(), 90);
    assert_eq!(
        catalog
            .iter()
            .map(|entry| entry.visible_options.len())
            .sum::<usize>(),
        391
    );
    let selected = catalog
        .iter()
        .find(|entry| entry.model == "XGF-RD8A")
        .expect("RD8A is selectable");

    let source = std::fs::read("fixtures/elements-io.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    let original = doc
        .modules()
        .into_iter()
        .find(|module| module.base == Some(0) && module.slot == Some(2))
        .expect("target module exists");

    doc.select_module(0, 2, "xgf-rd8a")
        .expect("model matching is ASCII-case-insensitive");
    let module = doc
        .modules()
        .into_iter()
        .find(|module| module.base == Some(0) && module.slot == Some(2))
        .expect("selected module exists");

    assert_eq!(module.base, original.base);
    assert_eq!(module.slot, original.slot);
    assert_eq!(module.comment, original.comment);
    assert_eq!(module.id, Some(selected.id));
    assert_eq!(module.sub_type, Some(selected.sub_type));
    assert_eq!(module.name.as_deref(), Some(selected.name));
    assert_eq!(module.details.as_deref(), Some(selected.details));

    let rewritten = doc.to_bytes().expect("selected workspace writes");
    let reparsed = XgwxDocument::parse(&rewritten).expect("selected workspace reparses");
    let module = reparsed
        .modules()
        .into_iter()
        .find(|module| module.base == Some(0) && module.slot == Some(2))
        .expect("selected module survives serialization");
    assert_eq!(module.id, Some(selected.id));
    assert_eq!(module.details.as_deref(), Some(selected.details));
}

#[cfg(feature = "write")]
#[test]
fn deletes_one_module_and_round_trips() {
    let source = std::fs::read("fixtures/elements-io.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    let original_count = doc.modules().len();

    doc.delete_module(0, 2).expect("target module deletes");
    assert_eq!(doc.modules().len(), original_count - 1);
    assert!(
        !doc.modules()
            .iter()
            .any(|module| module.base == Some(0) && module.slot == Some(2))
    );

    let xml_after_delete = doc.xml.clone();
    let missing = doc
        .delete_module(0, 2)
        .expect_err("deleting an absent module must fail");
    assert!(matches!(
        missing,
        XgwxError::ModuleNotFound { base: 0, slot: 2 }
    ));
    assert_eq!(doc.xml, xml_after_delete);

    let bytes = doc.to_bytes().expect("deleted workspace writes");
    let reparsed = XgwxDocument::parse(&bytes).expect("deleted workspace reparses");
    assert_eq!(reparsed.modules().len(), original_count - 1);
    assert!(
        !reparsed
            .modules()
            .iter()
            .any(|module| module.base == Some(0) && module.slot == Some(2))
    );
}

#[cfg(feature = "write")]
#[test]
fn inserts_a_catalog_module_into_an_empty_slot_and_round_trips() {
    let source = std::fs::read("fixtures/elements-io.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    let original_count = doc.modules().len();

    doc.delete_module(0, 2).expect("slot becomes empty");
    doc.insert_module(0, 2, "XGF-RD8A")
        .expect("catalog module inserts into empty slot");
    let inserted = doc
        .modules()
        .into_iter()
        .find(|module| module.base == Some(0) && module.slot == Some(2))
        .expect("inserted module exists");
    let entry = xgk_module_catalog()
        .iter()
        .find(|entry| entry.model == "XGF-RD8A")
        .expect("RD8A is in the catalog");
    assert_eq!(inserted.id, Some(entry.id));
    assert_eq!(inserted.sub_type, Some(entry.sub_type));
    assert_eq!(inserted.name.as_deref(), Some(entry.name));
    assert_eq!(inserted.comment.as_deref(), Some(""));
    assert_eq!(inserted.details.as_deref(), Some(entry.details));
    assert_eq!(doc.modules().len(), original_count);

    let bytes = doc.to_bytes().expect("workspace with insertion writes");
    let reparsed = XgwxDocument::parse(&bytes).expect("workspace with insertion reparses");
    assert!(reparsed.modules().iter().any(|module| {
        module.base == Some(0) && module.slot == Some(2) && module.id == Some(entry.id)
    }));
}

#[cfg(feature = "write")]
#[test]
fn rejects_module_insertion_conflicts_without_mutating_xml() {
    let source = std::fs::read("fixtures/elements-io.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    let original_xml = doc.xml.clone();

    let occupied = doc
        .insert_module(0, 2, "XGF-RD8A")
        .expect_err("occupied slot rejects insertion");
    assert!(matches!(
        occupied,
        XgwxError::ModulePlacementConflict {
            conflicting_slot: 2,
            ..
        }
    ));
    assert_eq!(doc.xml, original_xml);

    doc.delete_module(0, 7).expect("slot 7 becomes empty");
    let xml_after_delete = doc.xml.clone();
    let overlap = doc
        .insert_module(0, 7, "XGF-TC4UD")
        .expect_err("two-slot module cannot overlap occupied slot 8");
    assert!(matches!(
        overlap,
        XgwxError::ModulePlacementConflict {
            conflicting_slot: 8,
            ..
        }
    ));
    assert_eq!(doc.xml, xml_after_delete);
}

#[cfg(feature = "write")]
#[test]
fn synchronizes_captured_network_module_configurations() {
    let source = std::fs::read("fixtures/elements-io.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");

    assert!(
        doc.networks()
            .iter()
            .flat_map(|network| &network.modules)
            .any(|module| {
                module.base == Some(1) && module.slot == Some(11) && module.id == Some(23056)
            })
    );
    assert!(doc.xml.contains("XGPD_CONFIG_INFO_DNET"));

    doc.delete_module(1, 11)
        .expect("network module and configuration delete");
    assert!(
        !doc.networks()
            .iter()
            .flat_map(|network| &network.modules)
            .any(|module| { module.base == Some(1) && module.slot == Some(11) })
    );
    assert!(!doc.xml.contains("XGPD_CONFIG_INFO_DNET"));

    doc.insert_module(1, 11, "XGL-DMEA/B")
        .expect("network module inserts with configuration");
    assert!(
        doc.networks()
            .iter()
            .flat_map(|network| &network.modules)
            .any(|module| {
                module.base == Some(1)
                    && module.slot == Some(11)
                    && module.id == Some(23056)
                    && module.option_type == Some(32771)
            })
    );
    assert!(doc.xml.contains(
        "<XGPD_CONFIG_INFO_DNET StationNo=\"0\" Type=\"23056\" Base=\"1\" Slot=\"11\" SubType=\"32771\"></XGPD_CONFIG_INFO_DNET>"
    ));

    doc.select_module(1, 11, "XGQ-RY1A")
        .expect("ordinary module replaces network module");
    assert!(
        !doc.networks()
            .iter()
            .flat_map(|network| &network.modules)
            .any(|module| { module.base == Some(1) && module.slot == Some(11) })
    );
    assert!(!doc.xml.contains("XGPD_CONFIG_INFO_DNET"));

    doc.select_module(0, 2, "XGL-EDMF")
        .expect("FDEnet module creates captured configuration");
    assert!(doc.xml.contains(
        "<XGPD_CONFIG_INFO_FDENET StationNo=\"0\" Type=\"23072\" Base=\"0\" Slot=\"2\" SubType=\"32770\" Media=\"6\" Master=\"0\"></XGPD_CONFIG_INFO_FDENET>"
    ));
    assert!(doc.xgpd_config_infos().iter().any(|config| {
        config.kind == "XGPD_CONFIG_INFO_FDENET"
            && config.type_code == Some(23072)
            && config
                .attributes
                .iter()
                .any(|attribute| attribute.name == "Media" && attribute.value == "6")
    }));
    let bytes = doc.to_bytes().expect("network configuration writes");
    let reparsed = XgwxDocument::parse(&bytes).expect("network configuration reparses");
    assert!(
        reparsed
            .networks()
            .iter()
            .flat_map(|network| &network.modules)
            .any(|module| {
                module.base == Some(0) && module.slot == Some(2) && module.id == Some(23072)
            })
    );

    for (model, id) in [("XGL-EIPT", 23064), ("XGL-BIPT", 23152)] {
        doc.select_module(0, 2, model)
            .expect("network-capable module selects");
        assert!(
            doc.networks()
                .iter()
                .flat_map(|network| &network.modules)
                .any(|module| {
                    module.base == Some(0) && module.slot == Some(2) && module.id == Some(id)
                })
        );
    }

    doc.select_module(0, 2, "XGL-EFMT(B)")
        .expect("FEnet module selects");
    let fenet = doc
        .fenet_config_infos()
        .into_iter()
        .find(|config| config.type_code == Some(23041))
        .expect("FEnet configuration exists");
    assert_eq!(
        fenet
            .ip_address
            .as_ref()
            .map(|value| value.address.as_str()),
        Some("192.168.0.100")
    );
    assert_eq!(
        fenet.gateway.as_ref().map(|value| value.address.as_str()),
        Some("192.168.0.1")
    );
}

#[cfg(feature = "write")]
#[test]
fn writes_network_and_network_module_metadata() {
    let source = std::fs::read("fixtures/elements-io.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    doc.select_module(0, 2, "XGL-EDMF")
        .expect("FDEnet module creates a network module");

    doc.update_network(
        0,
        &NetworkPatch {
            name: Some("Field network".to_owned()),
            type_name: Some("Ethernet".to_owned()),
            network_type: Some("FEnet".to_owned()),
        },
    )
    .expect("network metadata updates");
    doc.update_network_module(
        0,
        2,
        &NetworkModulePatch {
            config_name: Some("PLC-1".to_owned()),
            alias: Some("Uplink".to_owned()),
            description: Some("Plant Ethernet".to_owned()),
        },
    )
    .expect("network-module metadata updates");

    let rewritten = doc.to_bytes().expect("network metadata writes");
    let reparsed = XgwxDocument::parse(&rewritten).expect("rewritten document parses");
    assert_eq!(
        reparsed.networks()[0].name.as_deref(),
        Some("Field network")
    );
    let module = reparsed
        .networks()
        .into_iter()
        .flat_map(|network| network.modules)
        .find(|module| module.base == Some(0) && module.slot == Some(2))
        .expect("edited network module exists");
    assert_eq!(module.config_name.as_deref(), Some("PLC-1"));
    assert_eq!(module.alias.as_deref(), Some("Uplink"));
    assert_eq!(module.description.as_deref(), Some("Plant Ethernet"));
}

#[cfg(feature = "write")]
#[test]
fn exposes_dl16a_nested_file_and_data_options() {
    let entry = xgk_module_catalog()
        .iter()
        .find(|entry| entry.model == "XGF-DL16A")
        .expect("DL16A is in the catalog");

    for key in [
        "fileConfiguration.fileEnabled",
        "fileConfiguration.fileName",
        "fileConfiguration.timeName",
        "fileConfiguration.indexName",
    ] {
        let option = entry
            .visible_options
            .iter()
            .find(|option| option.key == key)
            .expect("per-file option is visible");
        assert_eq!(option.scope, "file");
        assert_eq!(option.count, 8);
        assert_eq!(option.items_per_parent, 0);
    }

    for key in ["dataDefinitions.type", "dataDefinitions.name"] {
        let option = entry
            .visible_options
            .iter()
            .find(|option| option.key == key)
            .expect("per-file data definition is visible");
        assert_eq!(option.scope, "fileData");
        assert_eq!(option.count, 8 * 32);
        assert_eq!(option.items_per_parent, 32);
    }
}

#[cfg(feature = "write")]
#[test]
fn exposes_all_catalog_channel_instances() {
    let catalog = xgk_module_catalog();
    for entry in catalog {
        for writable in entry.options {
            let visible = entry
                .visible_options
                .iter()
                .find(|option| option.key == writable.key)
                .expect("every writable option must also be visible");
            assert!(
                visible.count >= writable.count,
                "{}:{} exposes {} rows but writes {}",
                entry.model,
                writable.key,
                visible.count,
                writable.count
            );
        }
    }

    for (model, key, channel_count) in [
        ("XGF-AC4H", "processAlarmEnabled", 4),
        ("XGF-AD4S", "inputRange", 4),
        ("XGF-AH6A", "input.averageProcessing", 4),
        ("XGF-AW4S", "outputDataType", 4),
        ("XGF-DA4S", "abnormalStateOutput", 4),
        ("XGF-DV4A", "powerLossOutput", 4),
        ("XGF-DV4S", "powerLossOutput", 4),
        ("XGF-HD2A", "counterMode", 2),
        ("XGF-HO2A", "counterMode", 2),
        ("XGF-RD4A", "sensorType", 4),
        ("XGF-RD4S", "sensorType", 4),
        ("XGF-RD8A", "sensorType", 8),
        ("XGF-TC4S", "sensorType", 4),
        ("XGF-TC4SB", "inputRange", 4),
    ] {
        let option = catalog
            .iter()
            .find(|entry| entry.model == model)
            .and_then(|entry| {
                entry
                    .visible_options
                    .iter()
                    .find(|option| option.key == key)
            })
            .expect("audited channel option is visible");
        assert_eq!(option.scope, "channel", "{model}:{key}");
        assert_eq!(option.count, channel_count, "{model}:{key}");
    }

    for (model, key) in [
        ("XGF-HD2A", "outputStateSetting"),
        ("XGF-HO2A", "outputStateSetting"),
        ("XGF-DA4S", "dssOutput"),
        ("XGF-DA4S", "analogOutputRetention"),
        ("XGF-TC4SB", "conversionSpeed"),
    ] {
        let option = catalog
            .iter()
            .find(|entry| entry.model == model)
            .and_then(|entry| {
                entry
                    .visible_options
                    .iter()
                    .find(|option| option.key == key)
            })
            .expect("audited module-wide option is visible");
        assert_eq!(option.scope, "module", "{model}:{key}");
        assert_eq!(option.count, 1, "{model}:{key}");
    }
}

#[cfg(feature = "write")]
#[test]
fn exposes_and_writes_dt4a_emergency_output_groups() {
    let entry = xgk_module_catalog()
        .iter()
        .find(|entry| entry.model == "XGH-DT4A")
        .expect("DT4A is in the catalog");
    assert!(
        entry
            .visible_options
            .iter()
            .any(|option| option.key == "emergencyOutput")
    );
    assert!(
        entry
            .options
            .iter()
            .any(|option| option.key == "emergencyOutput")
    );

    let source = std::fs::read("fixtures/elements-io.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    doc.select_module(0, 2, "XGH-DT4A").expect("DT4A selects");
    doc.set_module_option(0, 2, "emergencyOutput", 0, 1)
        .expect("first output group writes");
    doc.set_module_option(0, 2, "emergencyOutput", 1, 1)
        .expect("second output group writes");

    let module = doc
        .modules()
        .into_iter()
        .find(|module| module.base == Some(0) && module.slot == Some(2))
        .expect("DT4A remains present");
    assert_eq!(module.details.as_deref(), Some("00000C0000000000"));
    let values = doc.module_option_values(0, 2).expect("DT4A options decode");
    assert_eq!(
        values
            .iter()
            .filter(|item| item.key == "emergencyOutput")
            .map(|item| item.value)
            .collect::<Vec<_>>(),
        vec![1, 1]
    );
}

#[cfg(feature = "write")]
#[test]
fn exposes_and_writes_relay_emergency_output_groups() {
    let catalog = xgk_module_catalog();
    for (model, group_count) in [("XGQ-RY1A", 1), ("XGQ-RY2A/B", 2)] {
        let entry = catalog
            .iter()
            .find(|entry| entry.model == model)
            .expect("relay module is in the catalog");
        let option = entry
            .options
            .iter()
            .find(|option| option.key == "emergencyOutput")
            .expect("relay emergency output is writable");
        assert_eq!(option.count, group_count);
        assert!(
            entry
                .visible_options
                .iter()
                .any(|option| option.key == "emergencyOutput")
        );
    }

    let source = std::fs::read("fixtures/elements-io.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    doc.set_module_option(1, 0, "emergencyOutput", 0, 1)
        .expect("RY1A output group writes");
    doc.set_module_option(1, 1, "emergencyOutput", 0, 1)
        .expect("RY2A/B first output group writes");
    doc.set_module_option(1, 1, "emergencyOutput", 1, 1)
        .expect("RY2A/B second output group writes");

    let modules = doc.modules();
    assert_eq!(
        modules
            .iter()
            .find(|module| module.base == Some(1) && module.slot == Some(0))
            .and_then(|module| module.details.as_deref()),
        Some("0000010000000000")
    );
    assert_eq!(
        modules
            .iter()
            .find(|module| module.base == Some(1) && module.slot == Some(1))
            .and_then(|module| module.details.as_deref()),
        Some("0000030000000000")
    );
}

#[cfg(feature = "write")]
#[test]
fn treats_tc4ud_as_a_two_slot_module() {
    let entry = xgk_module_catalog()
        .iter()
        .find(|entry| entry.model == "XGF-TC4UD")
        .expect("TC4UD is in the catalog");
    assert_eq!(entry.slot_span, 2);
    assert!(
        xgk_module_catalog()
            .iter()
            .filter(|entry| entry.model != "XGF-TC4UD")
            .all(|entry| entry.slot_span == 1)
    );

    let source = std::fs::read("fixtures/elements.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    doc.select_module(0, 2, "XGF-TC4UD")
        .expect("TC4UD fits in empty slots 2 and 3");
    let selected = doc
        .modules()
        .into_iter()
        .find(|module| module.base == Some(0) && module.slot == Some(2))
        .expect("TC4UD remains anchored at its starting slot");
    assert_eq!(selected.id, Some(entry.id));

    let dense_source = std::fs::read("fixtures/elements-io.xgwx").expect("fixture reads");
    let mut dense = XgwxDocument::parse(&dense_source).expect("fixture parses");
    let original_xml = dense.xml.clone();
    let error = dense
        .select_module(0, 7, "XGF-TC4UD")
        .expect_err("TC4UD must not overlap slot 8");
    assert!(matches!(
        error,
        XgwxError::ModulePlacementConflict {
            conflicting_slot: 8,
            ..
        }
    ));
    assert_eq!(dense.xml, original_xml);

    let error = dense
        .select_module(1, 11, "XGF-TC4UD")
        .expect_err("TC4UD must not extend past a 12-slot base");
    assert!(matches!(
        error,
        XgwxError::ModulePlacementExceedsBase { slot_count: 12, .. }
    ));
    assert_eq!(dense.xml, original_xml);
}

#[cfg(feature = "write")]
#[test]
fn rejects_unknown_catalog_module_without_mutating_xml() {
    let source = std::fs::read("fixtures/elements-io.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    let original_xml = doc.xml.clone();

    let error = doc
        .select_module(0, 2, "XGF-NOT-A-MODULE")
        .expect_err("unknown model must fail");
    assert!(matches!(error, XgwxError::UnknownModuleCatalogModel { .. }));
    assert_eq!(doc.xml, original_xml);
}

#[cfg(feature = "write")]
#[test]
fn writes_verified_module_dropdown_options_and_round_trips() {
    let source = std::fs::read("fixtures/elements-io.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    doc.select_module(0, 2, "XGF-AD8A")
        .expect("catalog module selects");

    doc.set_module_option(0, 2, "channelOperation", 3, 1)
        .expect("channel operation writes");
    doc.set_module_option(0, 2, "inputRange", 5, 6)
        .expect("split packed range writes");
    doc.set_module_option(0, 2, "samplingProcessing", 7, 2)
        .expect("packed selection writes");

    let values = doc
        .module_option_values(0, 2)
        .expect("module options decode");
    let value = |key, index| {
        values
            .iter()
            .find(|item| item.key == key && item.index == index)
            .map(|item| item.value)
    };
    assert_eq!(value("channelOperation", 3), Some(1));
    assert_eq!(value("inputRange", 5), Some(6));
    assert_eq!(value("samplingProcessing", 7), Some(2));
    assert_eq!(value("inputRange", 4), Some(0));

    let rewritten = doc.to_bytes().expect("workspace writes");
    let reparsed = XgwxDocument::parse(&rewritten).expect("workspace reparses");
    assert_eq!(
        reparsed
            .module_option_values(0, 2)
            .expect("options survive serialization"),
        values
    );
}

#[cfg(feature = "write")]
#[test]
fn exposes_verified_high_speed_counter_options() {
    for (model, expected_keys) in [
        (
            "XGF-HD2A",
            &[
                "counterMode",
                "pulseInputMode",
                "compareOutput0Mode",
                "compareOutput1Mode",
                "outputStateSetting",
                "auxiliaryFunctionMode",
            ][..],
        ),
        (
            "XGF-HO2A",
            &[
                "counterMode",
                "pulseInputMode",
                "compareOutput0Mode",
                "compareOutput1Mode",
                "outputStateSetting",
                "auxiliaryFunctionMode",
            ][..],
        ),
        (
            "XGF-HO8A",
            &[
                "counterMode",
                "pulseInputMode",
                "compareOutputMode",
                "outputStateSetting",
                "inputFilter",
                "auxiliaryFunctionMode",
                "pulseInputLevel",
            ][..],
        ),
    ] {
        let entry = xgk_module_catalog()
            .iter()
            .find(|entry| entry.model == model)
            .expect("high-speed-counter module is in the catalog");
        assert_eq!(
            entry
                .options
                .iter()
                .map(|option| option.key)
                .collect::<Vec<_>>(),
            expected_keys
        );
    }
}

#[cfg(feature = "write")]
#[test]
fn writes_high_speed_counter_options_at_xg5000_verified_offsets() {
    fn details_bytes(doc: &XgwxDocument, slot: u32) -> Vec<u8> {
        let details = doc
            .modules()
            .into_iter()
            .find(|module| module.base == Some(1) && module.slot == Some(slot))
            .and_then(|module| module.details)
            .expect("fixture module has Details");
        details
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                u8::from_str_radix(std::str::from_utf8(pair).expect("hex is UTF-8"), 16)
                    .expect("Details contains hex")
            })
            .collect()
    }

    let source = std::fs::read("fixtures/elements-io.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");

    // XGF-HO2A: two little-endian u32 channel records with a 100-byte stride.
    doc.set_module_option(1, 8, "counterMode", 1, 1)
        .expect("HO2A counter mode writes");
    doc.set_module_option(1, 8, "pulseInputMode", 0, 7)
        .expect("HO2A pulse mode writes");
    doc.set_module_option(1, 8, "compareOutput0Mode", 1, 6)
        .expect("HO2A compare 0 mode writes");
    doc.set_module_option(1, 8, "compareOutput1Mode", 0, 5)
        .expect("HO2A compare 1 mode writes");
    doc.set_module_option(1, 8, "outputStateSetting", 0, 1)
        .expect("HO2A output state writes");
    doc.set_module_option(1, 8, "auxiliaryFunctionMode", 1, 6)
        .expect("HO2A auxiliary mode writes");
    let ho2a = details_bytes(&doc, 8);
    assert_eq!(&ho2a[4..8], &7u32.to_le_bytes());
    assert_eq!(&ho2a[36..40], &5u32.to_le_bytes());
    assert_eq!(&ho2a[100..104], &1u32.to_le_bytes());
    assert_eq!(&ho2a[132..136], &6u32.to_le_bytes());
    assert_eq!(&ho2a[172..176], &6u32.to_le_bytes());
    assert_eq!(&ho2a[200..204], &1u32.to_le_bytes());

    // XGF-HD2A uses the same record layout, independently verified in XG5000.
    doc.set_module_option(1, 9, "pulseInputMode", 1, 5)
        .expect("HD2A pulse mode writes");
    doc.set_module_option(1, 9, "auxiliaryFunctionMode", 0, 5)
        .expect("HD2A auxiliary mode writes");
    doc.set_module_option(1, 9, "outputStateSetting", 0, 1)
        .expect("HD2A output state writes");
    let hd2a = details_bytes(&doc, 9);
    assert_eq!(&hd2a[104..108], &5u32.to_le_bytes());
    assert_eq!(&hd2a[72..76], &5u32.to_le_bytes());
    assert_eq!(&hd2a[200..204], &1u32.to_le_bytes());

    // XGF-HO8A packs two four-bit channels per byte and eight flags per byte.
    doc.set_module_option(1, 10, "counterMode", 7, 1)
        .expect("HO8A counter mode writes");
    doc.set_module_option(1, 10, "pulseInputMode", 0, 4)
        .expect("HO8A pulse mode 0 writes");
    doc.set_module_option(1, 10, "pulseInputMode", 7, 7)
        .expect("HO8A pulse mode 7 writes");
    doc.set_module_option(1, 10, "compareOutputMode", 1, 6)
        .expect("HO8A compare mode writes");
    doc.set_module_option(1, 10, "outputStateSetting", 4, 1)
        .expect("HO8A output state writes");
    doc.set_module_option(1, 10, "inputFilter", 2, 3)
        .expect("HO8A input filter writes");
    doc.set_module_option(1, 10, "auxiliaryFunctionMode", 5, 6)
        .expect("HO8A auxiliary mode writes");
    doc.set_module_option(1, 10, "pulseInputLevel", 7, 1)
        .expect("HO8A pulse input level writes");
    let ho8a = details_bytes(&doc, 10);
    assert_eq!(ho8a[0], 0x80);
    assert_eq!(ho8a[4], 0x04);
    assert_eq!(ho8a[7], 0x70);
    assert_eq!(ho8a[8], 0x60);
    assert_eq!(ho8a[13], 0x03);
    assert_eq!(ho8a[16], 0x10);
    assert_eq!(ho8a[26], 0x60);
    assert_eq!(ho8a[28], 0x80);

    let selections = doc
        .module_option_values(1, 10)
        .expect("HO8A options decode");
    assert!(selections.iter().any(|selection| {
        selection.key == "auxiliaryFunctionMode" && selection.index == 5 && selection.value == 6
    }));

    let rewritten = doc.to_bytes().expect("workspace writes");
    let reparsed = XgwxDocument::parse(&rewritten).expect("workspace reparses");
    assert_eq!(details_bytes(&reparsed, 8), ho2a);
    assert_eq!(details_bytes(&reparsed, 9), hd2a);
    assert_eq!(details_bytes(&reparsed, 10), ho8a);
}

#[cfg(feature = "write")]
#[test]
fn rejects_unverified_module_option_values_without_mutating_xml() {
    let source = std::fs::read("fixtures/elements-io.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    doc.select_module(0, 2, "XGF-AD8A")
        .expect("catalog module selects");
    let original_xml = doc.xml.clone();

    let error = doc
        .set_module_option(0, 2, "inputRange", 0, 99)
        .expect_err("unknown dropdown value must fail");
    assert!(matches!(error, XgwxError::InvalidModuleOptionValue { .. }));
    assert_eq!(doc.xml, original_xml);

    let error = doc
        .set_module_option(0, 2, "averageValue", 0, 100)
        .expect_err("numeric fields are not exposed as dropdown options");
    assert!(matches!(error, XgwxError::UnknownModuleOption { .. }));
    assert_eq!(doc.xml, original_xml);
}

#[cfg(feature = "write")]
#[test]
fn writes_program_metadata_and_same_length_ladder_cell() {
    let source = std::fs::read("fixtures/elements.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    let original_program = doc
        .ladder_programs()
        .into_iter()
        .next()
        .expect("fixture has a ladder program")
        .expect("ladder program decodes");
    let cell = original_program
        .strings
        .iter()
        .find(|string| string.value == "M00000")
        .expect("fixture has editable cell");

    doc.update_program(
        0,
        &ProgramPatch {
            name: Some("EditedProgram".to_owned()),
            task: Some("Edited task".to_owned()),
            comment: Some("VS Code & XG5000 <program>".to_owned()),
            ..ProgramPatch::default()
        },
    )
    .expect("program metadata updates");
    doc.update_ladder_cell_text(0, cell.offset, "M00000", "M00042")
        .expect("same-length ladder cell updates");

    let rewritten = doc.to_bytes().expect("edited document writes");
    let reparsed = XgwxDocument::parse(&rewritten).expect("rewritten document parses");
    let metadata = reparsed
        .programs()
        .into_iter()
        .next()
        .expect("program exists");
    assert_eq!(metadata.name.as_deref(), Some("EditedProgram"));
    assert_eq!(metadata.task.as_deref(), Some("Edited task"));
    assert_eq!(
        metadata.comment.as_deref(),
        Some("VS Code & XG5000 <program>")
    );
    let ladder = reparsed
        .ladder_programs()
        .into_iter()
        .next()
        .expect("program exists")
        .expect("program decodes");
    assert!(ladder.strings.iter().any(|string| string.value == "M00042"));
    assert!(!ladder.strings.iter().any(|string| string.value == "M00000"));
}

#[cfg(feature = "write")]
#[test]
fn rejects_unsafe_ladder_cell_edits_without_mutating_xml() {
    let source = std::fs::read("fixtures/elements.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    let original_xml = doc.xml.clone();
    let program = doc
        .ladder_programs()
        .into_iter()
        .next()
        .expect("fixture has a ladder program")
        .expect("ladder program decodes");
    let cell = program
        .strings
        .iter()
        .find(|string| string.value == "M00000")
        .expect("fixture has editable cell");

    let length_error = doc
        .update_ladder_cell_text(0, cell.offset, "M00000", "M000001")
        .expect_err("length changes must fail");
    assert!(matches!(
        length_error,
        XgwxError::LadderCellLengthChanged { .. }
    ));
    assert_eq!(doc.xml, original_xml);

    let stale_error = doc
        .update_ladder_cell_text(0, cell.offset, "M99999", "M00042")
        .expect_err("stale text must fail");
    assert!(matches!(stale_error, XgwxError::LadderCellChanged { .. }));
    assert_eq!(doc.xml, original_xml);
}

#[cfg(feature = "write")]
#[test]
fn writes_same_length_variable_fields_and_numeric_address() {
    let source = std::fs::read("fixtures/elements.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");

    doc.update_variable(
        0,
        &VariablePatch {
            name: Some("_0000_DI00".to_owned()),
            address_area: Some("M".to_owned()),
            address_number: Some(42),
            description: Some("수정 접점 00".to_owned()),
            ..VariablePatch::default()
        },
    )
    .expect("supported variable fields update");

    let rewritten = doc.to_bytes().expect("edited document writes");
    let reparsed = XgwxDocument::parse(&rewritten).expect("rewritten document parses");
    let variables = reparsed.variables().expect("variables decode");
    assert_eq!(variables.len(), 82);
    assert_eq!(variables[0].name.as_deref(), Some("_0000_DI00"));
    assert_eq!(variables[0].address_area.as_deref(), Some("M"));
    assert_eq!(variables[0].address_number, Some(42));
    assert_eq!(variables[0].address.as_deref(), Some("M0002A"));
    assert_eq!(variables[0].description.as_deref(), Some("수정 접점 00"));
    assert_eq!(variables[1].name.as_deref(), Some("_0000_IN01"));
}

#[cfg(feature = "write")]
#[test]
fn rejects_unsafe_variable_edits_without_mutating_xml() {
    let source = std::fs::read("fixtures/elements.xgwx").expect("fixture reads");
    let mut doc = XgwxDocument::parse(&source).expect("fixture parses");
    let original_xml = doc.xml.clone();

    let length_error = doc
        .update_variable(
            0,
            &VariablePatch {
                name: Some("length-changing-name".to_owned()),
                ..VariablePatch::default()
            },
        )
        .expect_err("length-changing strings must fail");
    assert!(matches!(
        length_error,
        XgwxError::VariableFieldLengthChanged { .. }
    ));
    assert_eq!(doc.xml, original_xml);

    let missing_error = doc
        .update_variable(999, &VariablePatch::default())
        .expect_err("missing variables must fail");
    assert!(matches!(
        missing_error,
        XgwxError::VariableNotFound { index: 999 }
    ));
    assert_eq!(doc.xml, original_xml);
}

#[test]
fn decodes_synthetic_ladder_records() {
    let data = synthetic_ladder_data();
    let strings = extract_ladder_strings(&data);
    let elements = extract_ladder_elements(&data, &strings);
    let structure = extract_ladder_structure(&data, &elements);

    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::DeviceRef
            && element.value == "M00001"
            && element.contact == Some(LadderContact::NormallyOpen)
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::DeviceRef
            && element.value == "M00002"
            && element.contact == Some(LadderContact::NormallyClosed)
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::DeviceRef
            && element.value == "M00005"
            && element.contact == Some(LadderContact::Inverse)
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::DeviceRef
            && element.value == "M00006"
            && element.contact == Some(LadderContact::RisingPulse)
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::DeviceRef
            && element.value == "M00007"
            && element.contact == Some(LadderContact::FallingPulse)
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::DeviceRef
            && element.value == "M00003"
            && element.coil == Some(LadderCoil::Output)
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::DeviceRef
            && element.value == "M00004"
            && element.coil == Some(LadderCoil::Reset)
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::InstructionCall
            && element.value == "MOV"
            && element.operands == ["D000001", "D000002"]
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::InternalRef
            && element.value == "F0092"
            && element.contact == Some(LadderContact::NormallyOpen)
    }));
    assert!(structure.rungs.iter().any(|rung| {
        rung.cells
            .iter()
            .any(|cell| cell.value == "F0092" && cell.contact == Some(LadderContact::NormallyOpen))
    }));
    assert!(structure.vertical_lines.iter().any(|line| (
        line.raw_x,
        line.raw_y_start,
        line.raw_y_end
    ) == (0x03, 0x00, 0x04)));
    assert_eq!(
        structure
            .vertical_lines
            .iter()
            .filter(|line| (line.raw_x, line.raw_y_start, line.raw_y_end) == (0x06, 0x04, 0x0c))
            .count(),
        1
    );
    assert!(structure.unknown_records.iter().any(|record| {
        record.marker == [0xff, 0x55]
            && (record.raw_x, record.raw_y) == (0x20, 0x04)
            && record.bytes.starts_with(&[0xff, 0x55])
    }));
    assert!(structure.horizontal_lines.iter().any(|line| (
        line.raw_y,
        line.raw_x_start,
        line.raw_x_end
    ) == (0x00, 0x01, 0x58)));
    assert!(structure.horizontal_lines.iter().any(|line| (
        line.raw_y,
        line.raw_x_start,
        line.raw_x_end
    ) == (0x04, 0x08, 0x5e)));
    assert!(
        !structure
            .unknown_records
            .iter()
            .any(|record| record.marker == [0xff, 0x02])
    );
}

#[test]
fn recognizes_additional_ladder_mnemonics() {
    let categorized_samples = [
        ("LOAD NOT", LadderMnemonicCategory::BasicInstructions),
        ("BRST", LadderMnemonicCategory::BasicInstructions),
        ("TON", LadderMnemonicCategory::TimerCounter),
        ("CTUD", LadderMnemonicCategory::TimerCounter),
        ("$MOVP", LadderMnemonicCategory::DataTransfer),
        ("GBMOVP", LadderMnemonicCategory::DataTransfer),
        ("WTODWP", LadderMnemonicCategory::BcdBinConversion),
        ("L2UDP", LadderMnemonicCategory::DataTypeConversion),
        ("CMP8P", LadderMnemonicCategory::Comparison),
        ("LOAD X", LadderMnemonicCategory::Comparison),
        ("OR4X", LadderMnemonicCategory::Comparison),
        ("DINCUP", LadderMnemonicCategory::IncrementDecrement),
        ("RCR8P", LadderMnemonicCategory::Rotation),
        ("DBSFRP", LadderMnemonicCategory::Shift),
        ("GSWAP2P", LadderMnemonicCategory::Exchange),
        ("$ADDP", LadderMnemonicCategory::BinaryArithmetic),
        ("GADDP", LadderMnemonicCategory::BinaryArithmetic),
        ("ADDCP", LadderMnemonicCategory::BcdArithmetic),
        ("ABXNRP", LadderMnemonicCategory::LogicalOperations),
        ("SEGP", LadderMnemonicCategory::Display),
        ("DETECTP", LadderMnemonicCategory::DataProcessing),
        ("FIINSP", LadderMnemonicCategory::DataTableProcessing),
        ("DDABCDP", LadderMnemonicCategory::StringProcessing),
        ("EXPTP", LadderMnemonicCategory::SpecialFunctions),
        ("PIDRUN", LadderMnemonicCategory::DataControl),
        ("ADDCAL", LadderMnemonicCategory::Time),
        ("JMP", LadderMnemonicCategory::Branching),
        ("BREAK", LadderMnemonicCategory::Loop),
        ("STC", LadderMnemonicCategory::Flag),
        ("TFLK", LadderMnemonicCategory::System),
        ("EIN", LadderMnemonicCategory::Interrupt),
        ("LNEGP", LadderMnemonicCategory::SignInversion),
        ("RSETP", LadderMnemonicCategory::File),
        ("FWRITE", LadderMnemonicCategory::FAreaControl),
        ("BRESET", LadderMnemonicCategory::WordBitControl),
        ("GETEP", LadderMnemonicCategory::SpecialCommunication),
        ("GETIP", LadderMnemonicCategory::Communication),
        ("PWM", LadderMnemonicCategory::Positioning),
        ("XCCCONEX", LadderMnemonicCategory::Positioning),
        ("XGETP", LadderMnemonicCategory::MotionControl),
    ];

    for (mnemonic, category) in categorized_samples {
        let info = ladder_mnemonic_info(mnemonic).expect("mnemonic metadata exists");
        assert_eq!(info.category, category, "{mnemonic} category");
        assert!(!info.description.is_empty());
        assert!(is_ladder_operation(mnemonic), "{mnemonic} is recognized");
    }

    let known = known_ladder_mnemonics();
    assert!(known.len() > 500, "manual mnemonic coverage is broad");
    for (index, info) in known.iter().enumerate() {
        assert!(!info.mnemonic.is_empty());
        assert!(!info.description.is_empty());
        assert_eq!(
            known
                .iter()
                .position(|other| other.mnemonic == info.mnemonic),
            Some(index),
            "{} appears only once",
            info.mnemonic
        );
    }

    for invalid_group_arithmetic in ["GADDU", "GADDUP", "GMUL", "GMULP", "GDIV", "GDIVP"] {
        assert_eq!(ladder_mnemonic_info(invalid_group_arithmetic), None);
    }
    for invalid_string_arithmetic in ["$SUB", "$SUBP", "$MUL", "$MULP", "$DIV", "$DIVP"] {
        assert_eq!(ladder_mnemonic_info(invalid_string_arithmetic), None);
    }

    assert_eq!(ladder_operation_kind("BRSTP"), LadderElementKind::Operation);
    assert_eq!(
        ladder_operation_kind("LOAD NOT"),
        LadderElementKind::Operation
    );
    assert_eq!(
        ladder_operation_kind("MCSCLR"),
        LadderElementKind::Operation
    );

    for comparison in [
        "=3", "<>3", ">3", "<3", ">=3", "<=3", "4=", "4<>", "4>", "4<", "4>=", "4<=", "8=", "8<>",
        "8>", "8<", "8>=", "8<=",
    ] {
        assert!(
            is_ladder_comparison_mnemonic(comparison),
            "{comparison} is recognized as comparison mnemonic"
        );
        assert_eq!(
            ladder_operation_kind(comparison),
            LadderElementKind::Comparison,
            "{comparison} is a comparison"
        );
    }

    for comparison in ["4=3", "4>=3", "8<>3", "8<=3"] {
        assert!(
            !is_ladder_comparison_mnemonic(comparison),
            "{comparison} is invalid because prefix and suffix are both present"
        );
    }

    for timer in ["TFLK", "TMON", "TRTG", "CTR", "CTUD"] {
        assert_eq!(
            ladder_operation_kind(timer),
            LadderElementKind::Timer,
            "{timer} is timer-like"
        );
    }

    for (source, expected_value, expected_kind, expected_operands) in [
        (
            "INC",
            "INC",
            LadderElementKind::InstructionCall,
            Vec::<&str>::new(),
        ),
        ("TFLK", "TFLK", LadderElementKind::Timer, Vec::new()),
        ("<=3", "<=3", LadderElementKind::Comparison, Vec::new()),
        ("CTUD", "CTUD", LadderElementKind::Timer, Vec::new()),
        (
            "INC D00001",
            "INC",
            LadderElementKind::InstructionCall,
            vec!["D00001"],
        ),
        (
            "TFLK T0 D0",
            "TFLK",
            LadderElementKind::Timer,
            vec!["T0", "D0"],
        ),
        (
            "<=3 D0 D1",
            "<=3",
            LadderElementKind::Comparison,
            vec!["D0", "D1"],
        ),
        (
            "CTUD C0 D0",
            "CTUD",
            LadderElementKind::Timer,
            vec!["C0", "D0"],
        ),
        (
            "BRST,M0001,3",
            "BRST",
            LadderElementKind::Operation,
            vec!["M0001", "3"],
        ),
        (
            "TFLK,P0040,1000,1000,D0000",
            "TFLK",
            LadderElementKind::Timer,
            vec!["P0040", "1000", "1000", "D0000"],
        ),
    ] {
        let element = parse_ladder_element(
            &[],
            &LadderString {
                offset: 0,
                end_offset: source.len(),
                value: source.to_owned(),
            },
        )
        .unwrap_or_else(|| panic!("{source} parses"));

        assert_eq!(element.value, expected_value);
        assert_eq!(element.kind, expected_kind, "{source} kind");
        assert_eq!(element.operands, expected_operands, "{source} operands");
    }

    assert_eq!(
        parse_ladder_element(
            &[],
            &LadderString {
                offset: 0,
                end_offset: 8,
                value: "Auto Run".to_owned(),
            },
        ),
        None
    );

    for instruction in ["INC,D00001", "INCP,D00001", "DEC,D00001", "DECP,D00001"] {
        let element = parse_ladder_element(
            &[],
            &LadderString {
                offset: 0,
                end_offset: instruction.len(),
                value: instruction.to_owned(),
            },
        )
        .expect("instruction parses");

        assert_eq!(element.kind, LadderElementKind::InstructionCall);
        assert_eq!(element.operands, ["D00001"]);
    }

    let comparison = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 12,
            value: ">=3,D0,D1".to_owned(),
        },
    )
    .expect("comparison parses");

    assert_eq!(comparison.kind, LadderElementKind::Comparison);
    assert_eq!(comparison.value, ">=3");
    assert_eq!(comparison.operands, ["D0", "D1"]);

    let prefixed_comparison = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 13,
            value: "4>=,D0,D1".to_owned(),
        },
    )
    .expect("prefixed comparison parses");

    assert_eq!(prefixed_comparison.kind, LadderElementKind::Comparison);
    assert_eq!(prefixed_comparison.value, "4>=");
    assert_eq!(prefixed_comparison.operands, ["D0", "D1"]);

    let bcd_instruction = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 19,
            value: "DADDBP,D0,D2,D4".to_owned(),
        },
    )
    .expect("BCD arithmetic instruction parses");

    assert_eq!(bcd_instruction.kind, LadderElementKind::InstructionCall);
    assert_eq!(bcd_instruction.value, "DADDBP");
    assert_eq!(bcd_instruction.operands, ["D0", "D2", "D4"]);

    let conversion_instruction = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 15,
            value: "GBCDP,D0,D2".to_owned(),
        },
    )
    .expect("BCD/BIN conversion instruction parses");

    assert_eq!(
        conversion_instruction.kind,
        LadderElementKind::InstructionCall
    );
    assert_eq!(conversion_instruction.value, "GBCDP");
    assert_eq!(conversion_instruction.operands, ["D0", "D2"]);

    let string_add_instruction = parse_ladder_instruction(&LadderString {
        offset: 0,
        end_offset: 17,
        value: "$ADDP,S0,S1,S2".to_owned(),
    })
    .expect("string add instruction parses");

    assert_eq!(string_add_instruction.mnemonic, "$ADDP");
    assert_eq!(string_add_instruction.operands, ["S0", "S1", "S2"]);

    let whitespace_instruction = parse_ladder_instruction(&LadderString {
        offset: 0,
        end_offset: 14,
        value: "CTUD C0 D0".to_owned(),
    })
    .expect("whitespace instruction parses");

    assert_eq!(whitespace_instruction.mnemonic, "CTUD");
    assert_eq!(whitespace_instruction.operands, ["C0", "D0"]);

    let binary_instruction = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 15,
            value: "DADDUP,D0,D2,D4".to_owned(),
        },
    )
    .expect("binary arithmetic instruction parses");

    assert_eq!(binary_instruction.kind, LadderElementKind::InstructionCall);
    assert_eq!(binary_instruction.value, "DADDUP");
    assert_eq!(binary_instruction.operands, ["D0", "D2", "D4"]);

    let exchange_instruction = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 17,
            value: "SWAP2P,D0,D2".to_owned(),
        },
    )
    .expect("exchange instruction parses");

    assert_eq!(
        exchange_instruction.kind,
        LadderElementKind::InstructionCall
    );
    assert_eq!(exchange_instruction.value, "SWAP2P");
    assert_eq!(exchange_instruction.operands, ["D0", "D2"]);

    let logical_instruction = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 17,
            value: "DWXORP,D0,D2,D4".to_owned(),
        },
    )
    .expect("logical instruction parses");

    assert_eq!(logical_instruction.kind, LadderElementKind::InstructionCall);
    assert_eq!(logical_instruction.value, "DWXORP");
    assert_eq!(logical_instruction.operands, ["D0", "D2", "D4"]);

    let string_move_instruction = parse_ladder_instruction(&LadderString {
        offset: 0,
        end_offset: 15,
        value: "$MOVP,S0,S1".to_owned(),
    })
    .expect("string move instruction parses");

    assert_eq!(string_move_instruction.mnemonic, "$MOVP");
    assert_eq!(string_move_instruction.operands, ["S0", "S1"]);

    let transfer_instruction = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 17,
            value: "GBMOVP,M0,M10".to_owned(),
        },
    )
    .expect("data transfer instruction parses");

    assert_eq!(
        transfer_instruction.kind,
        LadderElementKind::InstructionCall
    );
    assert_eq!(transfer_instruction.value, "GBMOVP");
    assert_eq!(transfer_instruction.operands, ["M0", "M10"]);

    let spaced_comparison_instruction = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 15,
            value: "LOAD X,D0,D1".to_owned(),
        },
    )
    .expect("spaced comparison instruction parses");

    assert_eq!(
        spaced_comparison_instruction.kind,
        LadderElementKind::Comparison
    );
    assert_eq!(spaced_comparison_instruction.value, "LOAD X");
    assert_eq!(spaced_comparison_instruction.operands, ["D0", "D1"]);
}

#[test]
fn decodes_elements_fixture_pulse_contacts_and_coils() {
    let doc = XgwxDocument::from_path("fixtures/elements.xgwx").expect("fixture parses");
    let program = doc
        .ladder_programs()
        .into_iter()
        .next()
        .expect("fixture has a ladder program")
        .expect("ladder program decodes");

    assert_ladder_contact(&program, "M00002", LadderContact::AddressedRisingPulse);
    assert_ladder_contact(&program, "M00003", LadderContact::AddressedRisingPulseNot);
    assert_ladder_contact(&program, "M00004", LadderContact::AddressedFallingPulse);
    assert_ladder_contact(&program, "M00005", LadderContact::AddressedFallingPulseNot);
    assert_ladder_coil(&program, "P00021", LadderCoil::Inverse);
    assert_ladder_coil(&program, "M00100", LadderCoil::RisingPulse);
    assert_ladder_coil(&program, "M00101", LadderCoil::FallingPulse);
    assert_ladder_instruction(
        &program,
        "XDST",
        &["1", "1", "7000", "1000", "100", "0", "0"],
    );
    assert_eq!(
        program.structure.rung_comments,
        vec![LadderRungComment {
            offset: 0x0035,
            raw_x: 0x01,
            raw_y: 0x00,
            text: "렁 설명문 1".to_owned(),
        }]
    );
    assert!(
        program
            .structure
            .branch_groups
            .contains(&LadderBranchGroup {
                raw_x: 0x03,
                raw_y_start: 0x08,
                raw_y_end: 0x0c,
            })
    );
    assert!(
        program
            .structure
            .branch_groups
            .contains(&LadderBranchGroup {
                raw_x: 0x06,
                raw_y_start: 0x08,
                raw_y_end: 0x10,
            })
    );
    assert!(
        program
            .structure
            .branch_groups
            .contains(&LadderBranchGroup {
                raw_x: 0x18,
                raw_y_start: 0x14,
                raw_y_end: 0x1c,
            })
    );
    assert!(
        program
            .structure
            .branch_groups
            .contains(&LadderBranchGroup {
                raw_x: 0x18,
                raw_y_start: 0x20,
                raw_y_end: 0x28,
            })
    );
    assert_marker_only_ladder_contact(&program, 0x04, 0x08, LadderContact::Inverse);
    assert_marker_only_ladder_contact(&program, 0x13, 0x04, LadderContact::RisingPulse);
    assert_marker_only_ladder_contact(&program, 0x16, 0x04, LadderContact::FallingPulse);
    // Only stored wires are unconditional connections; contact footprints are rendered separately.
    assert!(
        !program
            .structure
            .horizontal_lines
            .iter()
            .any(|line| line.raw_y == 0x0c)
    );
    assert!(program.structure.horizontal_lines.iter().any(|line| (
        line.raw_y,
        line.raw_x_start,
        line.raw_x_end
    ) == (0x08, 0x07, 0x5d)));
    assert!(program.structure.horizontal_lines.iter().any(|line| (
        line.raw_y,
        line.raw_x_start,
        line.raw_x_end
    ) == (0x10, 0x04, 0x06)));
    assert_eq!(
        program.structure.output_comments,
        vec![LadderOutputComment {
            offset: 0x0195,
            raw_x: 0x61,
            raw_y: 0x04,
            text: "출력 설명문 1".to_owned(),
        }]
    );
    assert!(!program.structure.unknown_records.iter().any(|record| {
        matches!(
            record.marker,
            [0xff, 0x01]
                | [0xff, 0x06]
                | [0xff, 0x3e]
                | [0xff, 0x3f]
                | [0xff, 0x40]
                | [0xff, 0x48]
                | [0xff, 0x49]
        )
    }));
}

#[cfg(feature = "il")]
#[test]
fn converts_elements_fixture_from_ld_to_il() {
    let doc = XgwxDocument::from_path("fixtures/elements.xgwx").expect("fixture parses");
    let program = doc
        .ladder_programs()
        .into_iter()
        .next()
        .expect("fixture has a ladder program")
        .expect("ladder program decodes");
    let il = program.to_il().expect("ladder converts to IL");

    let expected = "Comment: 렁 설명문 1\
\nLOAD M00000\
\nAND NOT M00001\
\nANDP M00002\
\nANDP NOT M00003\
\nANDN M00004\
\nANDN NOT M00005\
\nR_EDGE\
\nF_EDGE\
\nOUT P00020\
\nLOAD P00000\
\nOR P00001\
\nNOT\
\nOR P00002\
\nOUT NOT P00021\
\nLOAD M00010\
\nSET M00020\
\nRST M00021\
\nOUTP M00100\
\nLOAD M00011\
\nRST M00020\
\nSET M00021\
\nOUTN M00101\
\nLOAD F00091\
\nMOV 0 D000000\
\nLOAD P00005\
\nXDST 1 1 7000 1000 100 0 0";

    assert_eq!(il.program_name.as_deref(), Some("NewProgram"));
    assert_eq!(il.steps.len(), 27);
    assert_eq!(il.to_string(), expected);
    assert_eq!(il.steps[0], IlStep::Comment("렁 설명문 1".to_owned()));
    assert_eq!(
        il.steps[7],
        IlStep::Instruction {
            mnemonic: "R_EDGE".to_owned(),
            operands: Vec::new(),
        }
    );
    assert_eq!(
        il.steps[8],
        IlStep::Instruction {
            mnemonic: "F_EDGE".to_owned(),
            operands: Vec::new(),
        }
    );
}

#[cfg(feature = "il")]
#[test]
fn formats_ld_to_il_with_unique_variable_names() {
    let doc = XgwxDocument::from_path("fixtures/elements.xgwx").expect("fixture parses");
    let mut variables = doc.variables().expect("variables decode");
    let program = doc
        .ladder_programs()
        .into_iter()
        .next()
        .expect("fixture has a ladder program")
        .expect("ladder program decodes");
    let il = program
        .to_il_with_variable_names(&variables)
        .expect("ladder converts to named IL")
        .to_string();

    assert!(il.contains("LOAD _0000_IN00"));
    assert!(il.contains("OUT _0001_OUT00"));
    assert!(il.contains("OUT NOT _0001_OUT01"));
    assert!(il.contains("LOAD M00010"));

    let mut duplicate = variables
        .iter()
        .find(|variable| variable.address.as_deref() == Some("P00020"))
        .expect("output variable exists")
        .clone();
    duplicate.name = Some("duplicate_output".to_owned());
    variables.push(duplicate);
    let duplicate_il = program
        .to_il_with_variable_names(&variables)
        .expect("duplicate symbols fall back to addresses")
        .to_string();
    assert!(duplicate_il.contains("OUT P00020"));
    assert!(!duplicate_il.contains("OUT _0001_OUT00"));

    let mut name_collision_variables = doc.variables().expect("variables decode again");
    let mut name_collision = name_collision_variables
        .iter()
        .find(|variable| variable.address.as_deref() == Some("P00020"))
        .expect("output variable exists")
        .clone();
    name_collision.address = Some("P99999".to_owned());
    name_collision_variables.push(name_collision);
    let name_collision_il = program
        .to_il_with_variable_names(&name_collision_variables)
        .expect("duplicate names fall back to addresses")
        .to_string();
    assert!(name_collision_il.contains("OUT P00020"));
    assert!(!name_collision_il.contains("OUT _0001_OUT00"));
}

#[cfg(feature = "il")]
#[test]
fn converts_tracked_xgb_ladders_with_unconditional_end() {
    let expected = "LOAD M0010\
\nAND F0092\
\nOUT M0110\
\nLOAD M0022\
\nAND F0092\
\nOUT M0122\
\nEND";

    for path in ["fixtures/XGB_Enet01.xgwx", "fixtures/XGB_Enet02.xgwx"] {
        let doc = XgwxDocument::from_path(path).expect("fixture parses");
        let programs = doc.ladder_programs();
        assert_eq!(programs.len(), 1, "{path}");
        let il = programs
            .into_iter()
            .next()
            .expect("program exists")
            .expect("program decodes")
            .to_il()
            .expect("ladder converts to IL");

        assert_eq!(il.steps.len(), 7, "{path}");
        assert_eq!(il.to_string(), expected, "{path}");
    }
}

#[cfg(feature = "il")]
#[test]
fn ld_to_il_rejects_unknown_positioned_records() {
    let doc = XgwxDocument::from_path("fixtures/elements.xgwx").expect("fixture parses");
    let mut program = doc
        .ladder_programs()
        .into_iter()
        .next()
        .expect("fixture has a ladder program")
        .expect("ladder program decodes");
    program.structure.unknown_records.push(LadderUnknownRecord {
        offset: 42,
        marker: [0xff, 0xaa],
        raw_x: 7,
        raw_y: 8,
        bytes: vec![0xff, 0xaa],
    });

    assert_eq!(
        program.to_il(),
        Err(LdToIlError::UnknownRecord {
            offset: 42,
            marker: [0xff, 0xaa],
            raw_x: 7,
            raw_y: 8,
        })
    );
}

#[test]
fn decodes_synthetic_hsc_parameter_counter_modes() {
    let mut payload = vec!['0'; 448];
    payload[1] = '1';
    payload[6] = '1';
    payload[7] = '2';
    payload[8] = '3';
    payload[9] = '4';
    payload[16] = '1';
    payload[17] = '8';
    payload[18] = '2';
    payload[19] = 'A';
    payload[40] = 'C';
    payload[41] = '8';
    payload[48] = '1';
    payload[49] = '4';
    payload[56] = '6';
    payload[57] = '4';
    payload[88] = '8';
    payload[89] = '8';
    payload[90] = '1';
    payload[91] = '3';
    payload[92] = '0';
    payload[93] = '4';
    let payload = payload.into_iter().collect::<String>();
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Project>
  <Parameters>
    <Parameter Type="HSC PARAMETER" PAYLOAD_ASC_LENGTH="448" PAYLOAD="{payload}" />
  </Parameters>
</Project>"#
    );
    let doc = XgwxDocument::parse(&synthetic_xgwx_bytes(&xml)).expect("synthetic parses");
    let hsc = doc
        .hsc_parameters()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("hsc payload decodes");

    assert_eq!(hsc.len(), 1);
    assert_eq!(hsc[0].payload_asc_length, Some(448));
    assert_eq!(hsc[0].payload_bytes.len(), 224);
    assert_eq!(hsc[0].initial_unknown_nibble, Some(0));
    assert_eq!(
        hsc[0]
            .channels
            .iter()
            .map(|channel| channel.counter_mode_raw)
            .collect::<Vec<_>>(),
        [Some(1), Some(0), Some(0), Some(0)]
    );
    assert_eq!(
        hsc[0]
            .channels
            .iter()
            .map(|channel| channel.pulse_input_mode_raw)
            .collect::<Vec<_>>(),
        [Some(0), Some(1), Some(2), Some(3)]
    );
    assert_eq!(
        hsc[0].channels[0].counter_mode,
        Some(HscCounterMode::RingCounter)
    );
    assert_eq!(
        hsc[0].channels[1].counter_mode,
        Some(HscCounterMode::LinearCounter)
    );
    assert_eq!(
        hsc[0].channels[0].pulse_input_mode,
        Some(HscPulseInputMode::OnePhaseOneInputOneX)
    );
    assert_eq!(
        hsc[0].channels[1].pulse_input_mode,
        Some(HscPulseInputMode::OnePhaseTwoInputOneX)
    );
    assert_eq!(
        hsc[0].channels[2].pulse_input_mode,
        Some(HscPulseInputMode::CwCcw)
    );
    assert_eq!(
        hsc[0].channels[3].pulse_input_mode,
        Some(HscPulseInputMode::TwoPhaseFourX)
    );
    assert_eq!(hsc[0].channels[0].compare_output_mode_raw, Some(4));
    assert_eq!(
        hsc[0].channels[0].compare_output_mode,
        Some(HscCompareOutputMode::GreaterThan)
    );
    assert_eq!(hsc[0].channels[0].internal_preset, Some(0x18));
    assert_eq!(hsc[0].channels[0].external_preset, Some(0x2a));
    assert_eq!(hsc[0].channels[0].ring_counter_max, Some(200));
    assert_eq!(hsc[0].channels[0].compare_output_min, Some(20));
    assert_eq!(hsc[0].channels[0].compare_output_max, Some(100));
    assert_eq!(hsc[0].channels[0].unit_time_ms, Some(5000));
    assert_eq!(hsc[0].channels[0].pulses_per_revolution, Some(4));
    assert!(
        hsc[0]
            .channels
            .iter()
            .all(|channel| channel.raw.len() == 56)
    );
}

#[test]
fn decodes_xgb_enet01_hsc_parameter_counter_modes() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").expect("fixture parses");
    let hsc = doc
        .hsc_parameters()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("hsc payload decodes");

    assert_eq!(hsc.len(), 1);
    assert_eq!(hsc[0].payload_asc_length, Some(448));
    assert_eq!(hsc[0].payload_bytes.len(), 224);
    assert_eq!(hsc[0].initial_unknown_nibble, Some(0));
    assert_eq!(
        hsc[0]
            .channels
            .iter()
            .map(|channel| channel.counter_mode_raw)
            .collect::<Vec<_>>(),
        [Some(1), Some(0), Some(0), Some(0)]
    );
    assert_eq!(
        hsc[0]
            .channels
            .iter()
            .map(|channel| channel.pulse_input_mode_raw)
            .collect::<Vec<_>>(),
        [Some(2), Some(0), Some(0), Some(0)]
    );
    assert_eq!(
        hsc[0].channels[0].counter_mode,
        Some(HscCounterMode::RingCounter)
    );
    assert_eq!(
        hsc[0].channels[0].pulse_input_mode,
        Some(HscPulseInputMode::CwCcw)
    );
    assert_eq!(hsc[0].channels[0].compare_output_mode_raw, Some(4));
    assert_eq!(
        hsc[0].channels[0].compare_output_mode,
        Some(HscCompareOutputMode::GreaterThan)
    );
    assert_eq!(hsc[0].channels[0].internal_preset, Some(0x18));
    assert_eq!(hsc[0].channels[0].external_preset, Some(0));
    assert_eq!(hsc[0].channels[0].ring_counter_max, Some(200));
    assert_eq!(hsc[0].channels[0].compare_output_min, Some(20));
    assert_eq!(hsc[0].channels[0].compare_output_max, Some(100));
    assert_eq!(hsc[0].channels[0].unit_time_ms, Some(5000));
    assert_eq!(hsc[0].channels[0].pulses_per_revolution, Some(4));
    assert!(
        hsc[0].channels[1..]
            .iter()
            .all(|channel| channel.counter_mode == Some(HscCounterMode::LinearCounter))
    );
    assert!(hsc[0].channels[1..].iter().all(|channel| {
        channel.pulse_input_mode == Some(HscPulseInputMode::OnePhaseOneInputOneX)
    }));
}

#[test]
fn decodes_xgb_enet01_position_parameter_axes() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").expect("fixture parses");
    let position = doc.position_parameters();

    assert_eq!(position.len(), 1);
    assert_eq!(position[0].axis_count, Some(2));
    assert_eq!(position[0].axes.len(), 2);
    assert_eq!(position[0].axes[0].axis_name, "X");
    assert_eq!(position[0].axes[1].axis_name, "Y");

    for axis in &position[0].axes {
        assert_eq!(axis.step_count, Some(30));
        assert_eq!(axis.steps.len(), 30);
        assert_eq!(axis.steps[0].target_position, Some(0));
        assert_eq!(axis.steps[0].operation_velocity, Some(0));

        let parameter = axis.parameter.as_ref().expect("axis parameter");
        assert_eq!(parameter.bias_velocity, Some(1));
        assert_eq!(parameter.velocity_limit, Some(100000));
        assert_eq!(
            parameter.accel_times,
            [Some(500), Some(1000), Some(1500), Some(2000)]
        );
        assert_eq!(
            parameter.decel_times,
            [Some(500), Some(1000), Some(1500), Some(2000)]
        );
        assert_eq!(parameter.soft_upper_limit, Some(i32::MAX));
        assert_eq!(parameter.soft_lower_limit, Some(i32::MIN));
        assert_eq!(parameter.s_curve_ratio, Some(50));
        assert_eq!(parameter.use_limit, Some(1));
        assert_eq!(parameter.return_velocity_high, Some(5000));
        assert_eq!(parameter.return_velocity_low, Some(500));
        assert_eq!(parameter.jog_velocity_high, Some(5000));
        assert_eq!(parameter.jog_velocity_low, Some(1000));
    }
}

#[test]
fn decodes_xgb_enet01_pid_parameters() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").expect("fixture parses");
    let cal = doc.pid_cal_parameters();
    let tune = doc.pid_tune_parameters();

    assert_eq!(cal.len(), 1);
    assert_eq!(cal[0].header, [Some(17736), Some(17473)]);
    assert_eq!(cal[0].set_pid_out, Some(0));
    assert_eq!(cal[0].prevent_anti_windup, Some(0));
    assert_eq!(cal[0].differential_control_method, Some(65535));
    assert_eq!(cal[0].loops.len(), 16);
    assert_eq!(cal[0].loops[0].target_value, Some(0));
    assert_eq!(cal[0].loops[0].scan_time, Some(100));
    assert_eq!(cal[0].loops[0].proportional_gain_left, Some(1));
    assert_eq!(cal[0].loops[0].mv_max, Some(4000));
    assert_eq!(cal[0].loops[0].mv_min, Some(0));
    assert_eq!(cal[0].loops[0].forward_pwm, Some(32));
    assert_eq!(cal[0].loops[0].pwm_out_period, Some(100));
    assert_eq!(cal[0].loops[0].pv_max, Some(4000));

    assert_eq!(tune.len(), 1);
    assert_eq!(tune[0].set_direction, Some(0));
    assert_eq!(tune[0].permit_pwm, Some(0));
    assert_eq!(tune[0].checksum, Some(0));
    assert_eq!(tune[0].footer, [Some(12358), Some(21552)]);
    assert_eq!(tune[0].loops.len(), 16);
    assert_eq!(tune[0].loops[0].target_value, Some(0));
    assert_eq!(tune[0].loops[0].scan_time, Some(100));
    assert_eq!(tune[0].loops[0].mv_max, Some(4000));
    assert_eq!(tune[0].loops[0].mv_min, Some(0));
    assert_eq!(tune[0].loops[0].set_pwm_at_point, Some(32));
    assert_eq!(tune[0].loops[0].out_period, Some(100));
    assert_eq!(tune[0].loops[0].hysteresis, Some(10));
}

#[test]
fn decodes_xgb_enet01_fenet_ipv4_parameters() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").expect("fixture parses");
    let fenet = doc.fenet_config_infos();

    assert_eq!(fenet.len(), 1);
    assert_eq!(fenet[0].type_code, Some(23041));
    assert_eq!(fenet[0].base, Some(0));
    assert_eq!(fenet[0].slot, Some(1));
    assert_eq!(fenet[0].sub_type, Some(32771));
    assert_eq!(
        fenet[0].ip_address.as_ref().map(|ip| ip.address.as_str()),
        Some("192.168.0.100")
    );
    assert_eq!(
        fenet[0].subnet.as_ref().map(|ip| ip.address.as_str()),
        Some("255.255.255.0")
    );
    assert_eq!(
        fenet[0].gateway.as_ref().map(|ip| ip.address.as_str()),
        Some("192.168.0.1")
    );
    assert_eq!(
        fenet[0].dns.as_ref().map(|ip| ip.address.as_str()),
        Some("0.0.0.0")
    );
    assert_eq!(
        fenet[0].ip_address2.as_ref().map(|ip| ip.address.as_str()),
        Some("0.0.0.0")
    );
}

#[test]
fn decodes_xgb_enet01_cnet_port_parameters() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").expect("fixture parses");
    let cnet = doc.cnet_config_infos();

    assert_eq!(cnet.len(), 1);
    assert_eq!(cnet[0].type_code, Some(23104));
    assert_eq!(cnet[0].base, Some(0));
    assert_eq!(cnet[0].slot, Some(0));
    assert_eq!(cnet[0].sub_type, Some(32773));
    assert_eq!(cnet[0].ports.len(), 2);

    assert_eq!(cnet[0].ports[0].station_no, Some(5));
    assert_eq!(cnet[0].ports[0].mode, Some(0));
    assert_eq!(cnet[0].ports[0].mode_kind, Some(CnetMode::Rs232C));
    assert_eq!(cnet[0].ports[0].bps, Some(8));
    assert_eq!(cnet[0].ports[0].baud_rate, Some(9600));
    assert_eq!(cnet[0].ports[0].data_bit, Some(1));
    assert_eq!(cnet[0].ports[0].data_bits, Some(CnetDataBits::Eight));
    assert_eq!(cnet[0].ports[0].stop_bit, Some(0));
    assert_eq!(cnet[0].ports[0].stop_bits, Some(CnetStopBits::One));
    assert_eq!(cnet[0].ports[0].parity, Some(0));
    assert_eq!(cnet[0].ports[0].parity_mode, Some(CnetParity::None));
    assert_eq!(cnet[0].ports[0].driver_type, Some(2));
    assert_eq!(cnet[0].ports[0].do_addr, Some(20));
    assert_eq!(cnet[0].ports[0].do_device, Some('P'));
    assert_eq!(cnet[0].ports[0].do_address.as_deref(), Some("P00014"));
    assert_eq!(cnet[0].ports[0].ai_addr, Some(20));
    assert_eq!(cnet[0].ports[0].ai_device, Some('P'));
    assert_eq!(cnet[0].ports[0].ai_address.as_deref(), Some("P00020"));
    assert_eq!(cnet[0].ports[0].ao_addr, Some(30));
    assert_eq!(cnet[0].ports[0].ao_address.as_deref(), Some("P00030"));

    assert_eq!(cnet[0].ports[1].station_no, Some(15));
    assert_eq!(cnet[0].ports[1].mode, Some(2));
    assert_eq!(cnet[0].ports[1].mode_kind, Some(CnetMode::Rs485));
    assert_eq!(cnet[0].ports[1].do_addr, Some(400));
    assert_eq!(cnet[0].ports[1].do_device, Some('M'));
    assert_eq!(cnet[0].ports[1].do_address.as_deref(), Some("M00250"));
}

#[test]
fn decodes_xgb_enet02_hsc_position_and_pid_parameters() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet02.xgwx").expect("fixture parses");

    let hsc = doc
        .hsc_parameters()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("hsc payload decodes");
    assert_eq!(hsc.len(), 1);
    assert_eq!(hsc[0].payload_asc_length, Some(448));
    assert_eq!(
        hsc[0].channels[0].counter_mode,
        Some(HscCounterMode::RingCounter)
    );
    assert_eq!(
        hsc[0].channels[0].pulse_input_mode,
        Some(HscPulseInputMode::CwCcw)
    );
    assert_eq!(
        hsc[0].channels[0].compare_output_mode,
        Some(HscCompareOutputMode::GreaterThan)
    );
    assert_eq!(hsc[0].channels[0].ring_counter_max, Some(200));
    assert_eq!(hsc[0].channels[0].unit_time_ms, Some(5000));
    assert_eq!(hsc[0].channels[0].pulses_per_revolution, Some(4));

    let position = doc.position_parameters();
    assert_eq!(position.len(), 1);
    assert_eq!(position[0].axis_count, Some(2));
    assert_eq!(position[0].axes.len(), 2);
    assert_eq!(position[0].axes[0].axis_name, "X");
    assert_eq!(position[0].axes[1].axis_name, "Y");

    let cal = doc.pid_cal_parameters();
    let tune = doc.pid_tune_parameters();
    assert_eq!(cal.len(), 1);
    assert_eq!(tune.len(), 1);
    assert_eq!(cal[0].loops.len(), 16);
    assert_eq!(tune[0].loops.len(), 16);
    assert_eq!(cal[0].loops[0].forward_pwm, Some(32));
    assert_eq!(tune[0].loops[0].set_pwm_at_point, Some(32));
}

#[test]
fn decodes_xgb_enet02_fenet_ipv4_parameters() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet02.xgwx").expect("fixture parses");
    let fenet = doc.fenet_config_infos();

    assert_eq!(fenet.len(), 1);
    assert_eq!(fenet[0].station_no, Some(1));
    assert_eq!(fenet[0].type_code, Some(23041));
    assert_eq!(fenet[0].base, Some(0));
    assert_eq!(fenet[0].slot, Some(1));
    assert_eq!(fenet[0].sub_type, Some(32771));
    assert_eq!(fenet[0].driver_type, Some(5));
    assert_eq!(fenet[0].rcv_wait_time, Some(20));
    assert_eq!(
        fenet[0].ip_address.as_ref().map(|ip| ip.address.as_str()),
        Some("192.168.0.100")
    );
    assert_eq!(
        fenet[0].subnet.as_ref().map(|ip| ip.address.as_str()),
        Some("255.255.255.0")
    );
    assert_eq!(
        fenet[0].gateway.as_ref().map(|ip| ip.address.as_str()),
        Some("192.168.0.1")
    );
}

#[test]
fn decodes_xgb_enet02_cnet_port_parameters() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet02.xgwx").expect("fixture parses");
    let cnet = doc.cnet_config_infos();

    assert_eq!(cnet.len(), 1);
    assert_eq!(cnet[0].type_code, Some(23104));
    assert_eq!(cnet[0].sub_type, Some(32773));
    assert_eq!(cnet[0].ports.len(), 2);
    assert_eq!(cnet[0].ports[0].station_no, Some(5));
    assert_eq!(cnet[0].ports[0].mode_kind, Some(CnetMode::Rs232C));
    assert_eq!(cnet[0].ports[0].baud_rate, Some(9600));
    assert_eq!(cnet[0].ports[0].data_bits, Some(CnetDataBits::Eight));
    assert_eq!(cnet[0].ports[0].stop_bits, Some(CnetStopBits::One));
    assert_eq!(cnet[0].ports[0].parity_mode, Some(CnetParity::None));
    assert_eq!(cnet[0].ports[0].do_address.as_deref(), Some("P00014"));
    assert_eq!(cnet[0].ports[0].ai_address.as_deref(), Some("P00020"));
    assert_eq!(cnet[0].ports[0].ao_address.as_deref(), Some("P00030"));
    assert_eq!(cnet[0].ports[1].station_no, Some(15));
    assert_eq!(cnet[0].ports[1].mode_kind, Some(CnetMode::Rs485));
    assert_eq!(cnet[0].ports[1].do_address.as_deref(), Some("M00250"));
}

#[test]
#[ignore = "set LIBXGWX_FIXTURE=/path/to/file.xgwx to run against a real workspace"]
fn parses_real_fixture_from_env() {
    let Ok(path) = env::var("LIBXGWX_FIXTURE") else {
        return;
    };
    let doc = XgwxDocument::from_path(path).expect("fixture parses");

    assert_eq!(doc.root.name, "Project");
    assert!(!doc.xml.is_empty());
    let _ = doc.project_info();
    let _ = doc.configurations();
    let _ = doc.networks();
    let _ = doc.network_modules();
    let _ = doc.bases();
    let _ = doc.modules();
    let _ = doc.tasks();
    let _ = doc.programs();
    let _ = doc.variables();
    let _ = doc.ladder_programs();
    let _ = doc.hsc_parameters();
    let _ = doc.position_parameters();
    let _ = doc.pid_cal_parameters();
    let _ = doc.pid_tune_parameters();
    let _ = doc.cnet_config_infos();
    let _ = doc.fenet_config_infos();
    let _ = doc.decoded_payloads();
}

#[test]
fn formats_xgwx_variable_addresses() {
    assert_eq!(format_ipv4_le(352364736), "192.168.0.21");
    assert_eq!(format_ipv4_le(16820416), "192.168.0.1");
    assert_eq!(format_ipv4_le(16777215), "255.255.255.0");
    assert_eq!(
        format_variable_address(Some("M"), Some(14), Some("BIT"), None).as_deref(),
        Some("M0000E")
    );
    assert_eq!(
        format_variable_address(Some("P"), Some(14), Some("BIT"), None).as_deref(),
        Some("P0000E")
    );
    assert_eq!(
        format_variable_address(Some("N"), Some(14), Some("WORD"), None).as_deref(),
        Some("N00014")
    );
    assert_eq!(
        format_variable_address(Some("X"), Some(14), Some("BIT"), None).as_deref(),
        Some("X00014")
    );
    assert_eq!(
        format_variable_address(Some("Y"), Some(14), Some("BIT"), None).as_deref(),
        Some("Y00014")
    );
}

#[test]
fn rejects_non_xgwx_data() {
    let error = XgwxDocument::parse(b"not an xgwx").expect_err("invalid magic");
    assert!(matches!(error, XgwxError::InvalidMagic));
}

fn synthetic_xgwx_bytes(xml: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"XG");
    bytes.extend_from_slice(UTF16_MARKER);
    let label = "XG5000 WORKSPACE FILE".encode_utf16().collect::<Vec<_>>();
    bytes.push(u8::try_from(label.len()).expect("label fits in u8"));
    for unit in label {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes.extend_from_slice(&1u32.to_le_bytes());

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(xml.as_bytes()).expect("gzip write");
    bytes.extend_from_slice(&encoder.finish().expect("gzip finish"));
    bytes
}

fn synthetic_ladder_data() -> Vec<u8> {
    let mut data = Vec::new();
    append_ff43_record(&mut data, 0x32, (0x58, 0x00), (0x01, 0x00));
    append_ff43_record(&mut data, 0x32, (0x58, 0x04), (0x03, 0x00));
    append_ff43_record(&mut data, 0x99, (0x44, 0x08), (0x06, 0x04));
    append_ff43_record(&mut data, 0x99, (0x44, 0x0c), (0x06, 0x08));
    append_unknown_positioned_record(&mut data, 0xff55, (0x20, 0x04));
    append_device_cell(&mut data, 0xff06, (0x01, 0x00), "M00001");
    append_device_cell(&mut data, 0xff07, (0x04, 0x00), "M00002");
    append_device_cell(&mut data, 0xff3e, (0x08, 0x00), "M00005");
    append_device_cell(&mut data, 0xff48, (0x0c, 0x00), "M00006");
    append_device_cell(&mut data, 0xff49, (0x10, 0x00), "M00007");
    append_wide_marker_device_cell(&mut data, 0xff06, (0x18, 0x00), "F0092");
    append_device_cell(&mut data, 0xff0e, (0x5e, 0x00), "M00003");
    append_ff02_horizontal_marker(&mut data, (0x08, 0x04));
    append_device_cell(&mut data, 0xff11, (0x5e, 0x04), "M00004");
    append_instruction_cell(&mut data, (0x58, 0x08), "MOV,D000001,D000002");
    data
}

fn assert_ladder_contact(program: &LadderProgramData, value: &str, expected: LadderContact) {
    let element = program
        .elements
        .iter()
        .find(|element| element.value == value)
        .unwrap_or_else(|| panic!("{value} element exists"));
    assert_eq!(element.contact, Some(expected), "{value} contact");
    assert_eq!(element.coil, None, "{value} is not a coil");
}

fn assert_ladder_coil(program: &LadderProgramData, value: &str, expected: LadderCoil) {
    let element = program
        .elements
        .iter()
        .find(|element| element.value == value)
        .unwrap_or_else(|| panic!("{value} element exists"));
    assert_eq!(element.coil, Some(expected), "{value} coil");
    assert_eq!(element.contact, None, "{value} is not a contact");
}

fn assert_ladder_instruction(program: &LadderProgramData, mnemonic: &str, operands: &[&str]) {
    let element = program
        .elements
        .iter()
        .find(|element| {
            element.kind == LadderElementKind::InstructionCall && element.value == mnemonic
        })
        .unwrap_or_else(|| panic!("{mnemonic} instruction exists"));
    assert_eq!(element.operands, operands);
    assert!(!program.elements.iter().any(|element| {
        element.kind == LadderElementKind::Comment
            && element.value.starts_with(&format!("{mnemonic},"))
    }));
}

fn assert_marker_only_ladder_contact(
    program: &LadderProgramData,
    raw_x: u8,
    raw_y: u8,
    expected: LadderContact,
) {
    let cell = program
        .structure
        .rungs
        .iter()
        .flat_map(|rung| &rung.cells)
        .find(|cell| cell.raw_x == raw_x && cell.raw_y == raw_y)
        .unwrap_or_else(|| panic!("marker-only cell exists at ({raw_x}, {raw_y})"));
    assert_eq!(cell.value, "");
    assert_eq!(cell.contact, Some(expected));
    assert_eq!(cell.coil, None);
}

fn append_ff43_record(data: &mut Vec<u8>, code: u8, target: (u8, u8), branch: (u8, u8)) {
    let start = data.len();
    data.resize(start + 48, 0);
    data[start] = 0xff;
    data[start + 1] = 0x43;
    data[start + 13] = code;
    data[start + 17] = target.0;
    data[start + 18] = target.1;
    data[start + 36] = branch.0;
    data[start + 37] = branch.1;
}

fn append_device_cell(data: &mut Vec<u8>, marker: u16, coord: (u8, u8), value: &str) {
    let [marker_hi, marker_lo] = marker.to_be_bytes();
    data.extend_from_slice(&[
        marker_hi, marker_lo, 0, 0, 0, coord.0, coord.1, 0, 0, 1, 0, 0, 0, 0, 0,
    ]);
    append_ladder_string(data, value);
}

fn append_wide_marker_device_cell(data: &mut Vec<u8>, marker: u16, coord: (u8, u8), value: &str) {
    let [marker_hi, marker_lo] = marker.to_be_bytes();
    data.extend_from_slice(&[
        marker_hi, marker_lo, 0, 0, 0, 0, 0, 0, 0, 0, coord.0, coord.1, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);
    append_ladder_string(data, value);
}

fn append_unknown_positioned_record(data: &mut Vec<u8>, marker: u16, coord: (u8, u8)) {
    let [marker_hi, marker_lo] = marker.to_be_bytes();
    data.extend_from_slice(&[
        marker_hi, marker_lo, 0xaa, 0xbb, 0xcc, coord.0, coord.1, 0xdd, 0xee, 0xff, 0, 1, 2, 3,
    ]);
}

fn append_ff02_horizontal_marker(data: &mut Vec<u8>, coord: (u8, u8)) {
    data.extend_from_slice(&[
        0xff, 0x02, 0xaa, 0xbb, 0xcc, coord.0, coord.1, 0xdd, 0xee, 0xff, 0, 1, 2, 3,
    ]);
}

fn append_instruction_cell(data: &mut Vec<u8>, coord: (u8, u8), value: &str) {
    data.extend_from_slice(&[coord.0, coord.1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    append_ladder_string(data, value);
}

fn append_ladder_string(data: &mut Vec<u8>, value: &str) {
    let units = value.encode_utf16().collect::<Vec<_>>();
    data.extend_from_slice(UTF16_MARKER);
    data.push(u8::try_from(units.len()).expect("string fits in u8"));
    for unit in units {
        data.extend_from_slice(&unit.to_le_bytes());
    }
}

#[cfg(feature = "write")]
#[test]
fn compact_cpu_guards_preserve_document_and_allow_comments() {
    let source = std::fs::read("fixtures/XGB_Enet01.xgwx").unwrap();
    let mut doc = XgwxDocument::parse(&source).unwrap();
    assert_eq!(doc.cpu_hardware_profile().unwrap().variant, "XBM-DR16S");
    let before = doc.to_bytes().unwrap();
    assert!(matches!(
        doc.delete_module(0, 0),
        Err(XgwxError::FixedCpuModule { .. })
    ));
    assert!(doc.select_module(0, 0, "XGI-D24A/B").is_err());
    assert!(doc.insert_module(0, 2, "XGI-D24A/B").is_err());
    assert!(doc.delete_module(0, 1).is_err());
    assert!(doc.module_option_values(0, 0).is_err());
    assert!(
        doc.set_module_option(0, 0, "emergencyOutput", 0, 1)
            .is_err()
    );
    assert!(
        doc.set_module_input_filter(0, 0, ModuleInputFilter::Ms5)
            .is_err()
    );
    for patch in [
        ModulePatch {
            id: Some(42242),
            ..Default::default()
        },
        ModulePatch {
            sub_type: Some(3),
            ..Default::default()
        },
        ModulePatch {
            name: Some("XGI-D24A/B".into()),
            ..Default::default()
        },
        ModulePatch {
            details: Some("00".into()),
            ..Default::default()
        },
    ] {
        assert!(doc.update_module(0, 0, &patch).is_err());
    }
    assert!(doc.select_cpu("XGK-CPUSN").is_err());
    assert!(doc.select_cpu("XGB-XBCH").is_err());
    doc.select_cpu("XGB-XBMS").unwrap();
    assert_eq!(doc.to_bytes().unwrap(), before);
    let original_networks = doc.network_modules();
    doc.update_module(
        0,
        0,
        &ModulePatch {
            comment: Some("CompactAcceptance".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let reparsed = XgwxDocument::parse(&doc.to_bytes().unwrap()).unwrap();
    assert_eq!(
        reparsed.modules()[0].comment.as_deref(),
        Some("CompactAcceptance")
    );
    assert_eq!(reparsed.network_modules(), original_networks);
    assert_eq!(reparsed.cpu_hardware_profile(), doc.cpu_hardware_profile());
}

#[cfg(feature = "write")]
#[test]
fn cpu_changes_reject_migration_and_hardware_limit_violations_atomically() {
    let mut doc = XgwxDocument::from_path("fixtures/elements-io.xgwx").unwrap();
    let before = doc.to_bytes().unwrap();
    assert!(matches!(
        doc.select_cpu("XGB-XBMS"),
        Err(XgwxError::UnsupportedCpuChange { .. })
    ));
    assert!(matches!(
        doc.select_cpu("XGK-CPUE"),
        Err(XgwxError::CpuHardwareLimit { .. })
    ));
    assert_eq!(doc.to_bytes().unwrap(), before);
}

#[cfg(feature = "write")]
#[test]
fn unverified_cpu_cannot_borrow_xgk_module_identity_or_options() {
    let source = XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
    // Retain an exact XGK module ID/name/options under another CPU, reproducing
    // the ID collision failure without using the now-guarded CPU writer.
    for type_code in [2, 7, 999] {
        let xml = source
            .xml
            .replace("Type=\"17\"", &format!("Type=\"{type_code}\""));
        let mut doc = source.clone();
        doc.root = parse_xml(&xml).unwrap();
        doc.xml = xml;
        let before = doc.xml.clone();
        assert!(doc.cpu_hardware_profile().is_none());
        assert!(doc.select_module(0, 0, "XGI-D24A/B").is_err());
        assert!(doc.module_option_values(0, 0).is_err());
        assert!(
            doc.set_module_input_filter(0, 0, ModuleInputFilter::Ms5)
                .is_err()
        );
        assert!(doc.delete_module(0, 0).is_err());
        assert_eq!(doc.xml, before);
    }
}

#[cfg(feature = "write")]
#[test]
fn raw_module_identity_patch_obeys_slot_span() {
    let mut doc = XgwxDocument::from_path("fixtures/elements-io.xgwx").unwrap();
    let before = doc.to_bytes().unwrap();
    let entry = xgk_module_catalog()
        .iter()
        .find(|m| m.model == "XGF-TC4UD")
        .unwrap();
    assert!(matches!(
        doc.update_module(
            0,
            7,
            &ModulePatch {
                id: Some(entry.id),
                sub_type: Some(entry.sub_type),
                ..Default::default()
            }
        ),
        Err(XgwxError::ModulePlacementConflict { .. })
    ));
    assert_eq!(doc.to_bytes().unwrap(), before);
}

#[cfg(feature = "write")]
#[test]
fn native_compact_subtype_remains_protected_after_save_as() {
    let mut doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").unwrap();
    let xml = doc
        .xml
        .replace("Id=\"42249\" SubType=\"1\"", "Id=\"42249\" SubType=\"0\"");
    assert_ne!(xml, doc.xml);
    doc.root = parse_xml(&xml).unwrap();
    doc.xml = xml;
    assert_eq!(doc.cpu_hardware_profile().unwrap().variant, "XBM-DR16S");
    assert!(matches!(
        doc.delete_module(0, 0),
        Err(XgwxError::FixedCpuModule { .. })
    ));
}

#[cfg(feature = "write")]
#[test]
fn structural_ladder_edits_round_trip_and_reject_protected_cells() {
    let mut doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").unwrap();
    let original = doc.to_bytes().unwrap();
    let edit = LadderCellEdit {
        raw_y: 0,
        column: 0,
        expected: Some(LadderEditElement {
            kind: LadderEditKind::NormallyOpen,
            operand: "M0010".into(),
        }),
        replacement: Some(LadderEditElement {
            kind: LadderEditKind::NormallyClosed,
            operand: "M00042".into(),
        }),
    };
    doc.edit_ladder_cell(0, &edit).unwrap();
    let reparsed = XgwxDocument::parse(&doc.to_bytes().unwrap()).unwrap();
    let ladder = reparsed.ladder_programs().remove(0).unwrap();
    assert_eq!(ladder.structure.rungs[0].cells[0].value, "M00042");
    assert_eq!(
        ladder.structure.rungs[0].cells[0].contact,
        Some(LadderContact::NormallyClosed)
    );
    let before = doc.to_bytes().unwrap();
    assert!(doc.edit_ladder_cell(0, &edit).is_err());
    assert_eq!(doc.to_bytes().unwrap(), before);
    assert_ne!(original, before);
    let mut complex = XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
    let before = complex.to_bytes().unwrap();
    assert!(matches!(
        complex.edit_ladder_cell(0, &edit),
        Err(XgwxError::InvalidLadderEdit { .. })
    ));
    assert_eq!(complex.to_bytes().unwrap(), before);
}

#[cfg(feature = "write")]
#[test]
fn base_slot_counts_round_trip_and_preserve_other_configuration() {
    let source = XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
    for base in 0..4 {
        for count in [4, 6, 8, 10, 12] {
            let mut doc = source.clone();
            let old = doc
                .bases()
                .into_iter()
                .find(|b| b.base == Some(base))
                .unwrap()
                .slot_count
                .unwrap();
            doc.set_base_slot_count(base, count).unwrap();
            assert_eq!(
                doc.xml,
                source.xml.replace(
                    &format!("Base=\"{base}\" SlotCount=\"{old}\""),
                    &format!("Base=\"{base}\" SlotCount=\"{count}\"")
                )
            );
            let mut saved = XgwxDocument::parse(&doc.to_bytes().unwrap()).unwrap();
            assert_eq!(
                saved
                    .bases()
                    .into_iter()
                    .find(|b| b.base == Some(base))
                    .unwrap()
                    .slot_count,
                Some(count)
            );
            saved.set_base_slot_count(base, old).unwrap();
            assert_eq!(saved.xml, source.xml);
        }
    }
}

#[cfg(feature = "write")]
#[test]
fn base_slot_count_rejections_are_atomic_and_respect_module_width() {
    let mut doc = XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
    doc.set_base_slot_count(1, 6).unwrap();
    doc.insert_module(1, 3, "XGF-TC4UD").unwrap();
    let before = doc.to_bytes().unwrap();
    assert!(matches!(
        doc.set_base_slot_count(1, 4),
        Err(XgwxError::ModulePlacementExceedsBase {
            slot: 3,
            slot_span: 2,
            ..
        })
    ));
    for (base, count) in [(0, 0), (0, 3), (0, 5), (0, 13), (0, u32::MAX), (4, 12)] {
        assert!(doc.set_base_slot_count(base, count).is_err());
    }
    assert_eq!(doc.to_bytes().unwrap(), before);
    doc.delete_module(1, 3).unwrap();
    doc.set_base_slot_count(1, 4).unwrap();
    assert!(doc.insert_module(1, 4, "XGI-D24A").is_err());
}

#[cfg(feature = "write")]
#[test]
fn base_slot_counts_reject_ambiguous_missing_and_compact_hardware() {
    for (bases, type_code) in [
        (
            "<Base Base=\"0\" SlotCount=\"4\"/><Base Base=\"0\" SlotCount=\"4\"/>",
            17,
        ),
        ("<Base Base=\"0\"/>", 17),
        ("<Base Base=\"1\" SlotCount=\"4\"/>", 17),
        ("<Base Base=\"0\" SlotCount=\"4\"/>", 2),
    ] {
        let xml = format!(
            "<Project><Configuration Type=\"{type_code}\"><Parameter Type=\"IO PARAMETER\"><BaseInfo>{bases}</BaseInfo></Parameter></Configuration></Project>"
        );
        let mut doc = XgwxDocument::parse(&synthetic_xgwx_bytes(&xml)).unwrap();
        assert!(doc.set_base_slot_count(0, 6).is_err());
        assert_eq!(doc.xml, xml);
    }
}
