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
    for (type_code, model, max_base) in [
        (100, "XGI-CPUU", 8),
        (102, "XGI-CPUH", 8),
        (104, "XGI-CPUS", 4),
        (106, "XGI-CPUE", 2),
        (107, "XGI-CPUU/D", 8),
        (110, "XGI-CPUS/P", 1),
        (111, "XGI-CPUUN", 8),
    ] {
        let entry = cpu_for_type(type_code).expect("XGI type is cataloged");
        assert_eq!(entry.model, model);
        assert_eq!(entry.family, "XGI");
        assert_eq!(entry.max_base, max_base);
    }
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
fn xgi_cpu_selection_recognizes_current_model_but_rejects_migration() {
    let mut doc = XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
    let xml = doc.xml.replace("Type=\"17\"", "Type=\"106\"");
    doc.root = parse_xml(&xml).unwrap();
    doc.xml = xml.clone();

    doc.select_cpu("xgi-cpue").expect("same XGI CPU is a no-op");
    assert_eq!(doc.xml, xml);
    assert!(matches!(
        doc.select_cpu("XGI-CPUS"),
        Err(XgwxError::UnsupportedCpuChange { .. })
    ));
    assert_eq!(doc.xml, xml);
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
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_function_bodies_decode_typed_pin_geometry() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let doc = XgwxDocument::from_path(path).expect("fixture parses");
    let programs = doc
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("all programs parse");
    let blocks = programs
        .iter()
        .flat_map(|program| program.iec_function_blocks().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(blocks.len(), 81);
    assert_eq!(
        blocks.iter().map(|block| block.pins.len()).sum::<usize>(),
        192
    );
    assert_eq!(
        blocks
            .iter()
            .flat_map(|block| &block.pins)
            .filter(|pin| pin.is_array)
            .count(),
        46
    );
    assert!(blocks.iter().all(|block| {
        block.control_input.direction == IecFunctionPinDirection::Input
            && block.control_output.direction == IecFunctionPinDirection::Output
            && block.control_input.data_type == Some("BOOL")
            && block.control_output.data_type == Some("BOOL")
            && block.pins.iter().all(|pin| pin.data_type.is_some())
    }));

    let add = blocks
        .iter()
        .find(|block| block.name.value == "ADD")
        .unwrap();
    assert_eq!(
        add.pins
            .iter()
            .map(|pin| (
                pin.name.value.as_str(),
                pin.direction,
                pin.reference_ordinal,
                pin.row_index,
                pin.raw_x,
                pin.data_type,
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "IN1",
                IecFunctionPinDirection::Input,
                Some(1),
                add.row_index + 1,
                add.raw_x,
                Some("ANY_NUM"),
            ),
            (
                "OUT",
                IecFunctionPinDirection::Output,
                Some(3),
                add.row_index + 1,
                add.raw_x + 3,
                Some("ANY_NUM"),
            ),
            (
                "IN2",
                IecFunctionPinDirection::Input,
                Some(2),
                add.row_index + 2,
                add.raw_x,
                Some("ANY_NUM"),
            ),
        ]
    );
    let trigger = blocks
        .iter()
        .find(|block| block.name.value == "R_TRIG")
        .unwrap();
    assert!(trigger.pins.is_empty());
    assert_eq!(trigger.control_input.name.value, "CLK");
    assert_eq!(trigger.control_output.name.value, "Q");
    assert_eq!(trigger.control_output.reference_ordinal, Some(1));
    let timer = blocks
        .iter()
        .find(|block| block.name.value == "TON")
        .unwrap();
    assert_eq!(
        timer
            .pins
            .iter()
            .map(|pin| (
                pin.name.value.as_str(),
                pin.reference_ordinal,
                pin.data_type,
            ))
            .collect::<Vec<_>>(),
        vec![("PT", Some(1), Some("TIME")), ("ET", Some(2), Some("TIME"))]
    );
    let mover = blocks
        .iter()
        .find(|block| block.name.value == "MOVE")
        .unwrap();
    assert!(mover.pins.iter().all(|pin| {
        pin.is_array
            && pin.data_type == Some("ANY")
            && pin
                .type_expression
                .as_ref()
                .map(|field| field.value.as_str())
                == Some("ARRAY[0..-1] OF ANY")
    }));

    let references = programs
        .iter()
        .flat_map(|program| program.iec_function_references().unwrap())
        .collect::<Vec<_>>();
    let operands = programs
        .iter()
        .flat_map(|program| program.iec_function_operand_links().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(references.len(), 196);
    assert_eq!(operands.len(), 176);
    for program in &programs {
        let program_blocks = program.iec_function_blocks().unwrap();
        for reference in program.iec_function_references().unwrap() {
            let block = program_blocks
                .iter()
                .find(|block| block.record_offset == reference.target_record_offset)
                .unwrap();
            let pin = block
                .pins
                .iter()
                .chain([&block.control_input, &block.control_output])
                .find(|pin| pin.reference_ordinal == Some(reference.ordinal))
                .unwrap();
            assert_eq!(reference.pin_row_index, pin.row_index);
            assert_eq!(reference.pin_raw_x, pin.raw_x);
            assert_eq!(reference.data_type_mask, pin.data_type_mask);
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_TERMINAL_MOVE_DELETED_FIXTURE and LIBXGWX_NATIVE_TERMINAL_MOVE_INSERT_FIXTURE"]
fn xgi_terminal_move_insertion_matches_native() {
    let deleted = env::var("LIBXGWX_TERMINAL_MOVE_DELETED_FIXTURE")
        .expect("deleted terminal MOVE fixture path");
    let native = env::var("LIBXGWX_NATIVE_TERMINAL_MOVE_INSERT_FIXTURE")
        .expect("native terminal MOVE insertion fixture path");
    let mut document = XgwxDocument::from_path(deleted).expect("deleted fixture parses");
    let before = document.to_bytes().unwrap();
    assert!(
        document
            .insert_iec_ld_terminal_move(3, 137, "1", "%MX300")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), before);
    let mut alternate = document.clone();
    alternate
        .insert_iec_ld_terminal_move(3, 137, "2", "%MW301")
        .expect("alternate MOVE operands are valid");
    assert!(
        alternate.ladder_programs()[3]
            .as_ref()
            .unwrap()
            .iec_circuit_graph()
            .is_some()
    );
    document
        .insert_iec_ld_terminal_move(3, 137, "1", "%MW300")
        .expect("restore terminal MOVE");
    let native = XgwxDocument::from_path(native).expect("native insertion parses");
    for (index, (changed, native)) in document
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .enumerate()
    {
        let changed = changed.unwrap();
        let native = native.unwrap();
        let mut changed_data = changed.data.clone();
        let mut native_data = native.data.clone();
        if index == 3 {
            for row in changed.iec_row_frames().unwrap() {
                changed_data[row.start + 17] = 0;
            }
            for row in native.iec_row_frames().unwrap() {
                native_data[row.start + 17] = 0;
            }
        }
        assert_eq!(changed_data, native_data, "program {index}");
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_NATIVE_TERMINAL_MOVE_L4_FIXTURE"]
fn xgi_elevator_move_insertion_covers_all_five_groups() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let native_l4 = env::var("LIBXGWX_NATIVE_TERMINAL_MOVE_L4_FIXTURE")
        .expect("native L4 MOVE insertion fixture path");
    let source = XgwxDocument::from_path(source).expect("source fixture parses");
    let native_l4 = XgwxDocument::from_path(native_l4).expect("native L4 fixture parses");
    let source_programs = source.ladder_programs();
    let source_program = source_programs[3].as_ref().unwrap();
    for group_index in 1..=5 {
        let site = source_program
            .iec_terminal_function_deletion_sites()
            .unwrap()
            .into_iter()
            .find(|site| site.group_index == group_index)
            .expect("original elevator MOVE deletion site");
        let mut generated = source.clone();
        generated
            .delete_iec_ld_terminal_function(3, site.block_offset, "MOVE")
            .expect("delete elevator MOVE");
        let retained = generated.ladder_programs()[3]
            .as_ref()
            .unwrap()
            .iec_terminal_function_insertion_sites()
            .unwrap();
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].group_index, group_index);
        generated
            .insert_iec_ld_terminal_move(
                3,
                retained[0].contact_offset,
                &group_index.to_string(),
                "%MW300",
            )
            .expect("restore elevator MOVE");
        let changed_programs = generated.ladder_programs();
        let changed = changed_programs[3].as_ref().unwrap();
        assert!(changed.iec_circuit_graph().is_some());
        let mut changed_data = changed.data.clone();
        let mut source_data = source_program.data.clone();
        for row in changed.iec_row_frames().unwrap() {
            changed_data[row.start + 17] = 0;
        }
        for row in source_program.iec_row_frames().unwrap() {
            source_data[row.start + 17] = 0;
        }
        assert_eq!(changed_data, source_data, "group {group_index}");
        if group_index == 2 {
            let native_programs = native_l4.ladder_programs();
            let native_program = native_programs[3].as_ref().unwrap();
            let mut native_data = native_program.data.clone();
            for row in native_program.iec_row_frames().unwrap() {
                native_data[row.start + 17] = 0;
            }
            assert_eq!(changed_data, native_data, "native L4 insertion");
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_NATIVE_TERMINAL_MOVE_P0_ALT_FIXTURE"]
fn xgi_lighting_terminal_move_insertion_restores_native_group() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let native_alt = env::var("LIBXGWX_NATIVE_TERMINAL_MOVE_P0_ALT_FIXTURE")
        .expect("native resaved program-0 MOVE alternate fixture path");
    let source = XgwxDocument::from_path(source).expect("source fixture parses");
    let native_alt = XgwxDocument::from_path(native_alt).expect("native alternate fixture parses");
    let mut restored = source.clone();
    restored
        .delete_iec_ld_terminal_function(0, 10289, "MOVE")
        .expect("delete lighting MOVE");
    let deleted_bytes = restored.to_bytes().unwrap();
    let sites = restored.ladder_programs()[0]
        .as_ref()
        .unwrap()
        .iec_terminal_function_insertion_sites()
        .unwrap();
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].group_index, 32);
    assert_eq!(sites[0].row_index, 63);
    assert_eq!(sites[0].contact_offset, 10247);
    assert_eq!(sites[0].raw_x, 16);
    let mut alternate = restored.clone();
    alternate
        .insert_iec_ld_terminal_move(0, 10247, "1", "자기유지1")
        .expect("compatible BOOL output is writable");
    assert!(
        alternate.ladder_programs()[0]
            .as_ref()
            .unwrap()
            .iec_circuit_graph()
            .is_some()
    );
    for (index, (actual, native)) in alternate
        .ladder_programs()
        .into_iter()
        .zip(native_alt.ladder_programs())
        .enumerate()
    {
        assert_eq!(
            actual.unwrap().data,
            native.unwrap().data,
            "native program {index}"
        );
    }
    assert!(
        restored
            .insert_iec_ld_terminal_move(0, 10247, "0", "UNKNOWN_OUTPUT")
            .is_err()
    );
    assert_eq!(restored.to_bytes().unwrap(), deleted_bytes);
    restored
        .insert_iec_ld_terminal_move(0, 10247, "0", "자기유지2")
        .expect("restore original lighting MOVE");
    for (index, (actual, expected)) in restored
        .ladder_programs()
        .into_iter()
        .zip(source.ladder_programs())
        .enumerate()
    {
        assert_eq!(
            actual.unwrap().data,
            expected.unwrap().data,
            "program {index}"
        );
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_NATIVE_FUNCTION_DELETE_FIXTURE"]
fn xgi_terminal_function_deletion_matches_native() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let native = env::var("LIBXGWX_NATIVE_FUNCTION_DELETE_FIXTURE")
        .expect("native XG5000 function deletion capture");
    let mut document = XgwxDocument::from_path(&source).expect("fixture parses");
    assert_eq!(
        document
            .ladder_programs()
            .iter()
            .map(|program| {
                program
                    .as_ref()
                    .unwrap()
                    .iec_terminal_function_deletion_sites()
                    .unwrap()
                    .len()
            })
            .collect::<Vec<_>>(),
        vec![1, 0, 0, 5, 0, 0, 0]
    );
    let before = document.to_bytes().unwrap();
    assert!(
        document
            .delete_iec_ld_terminal_function(3, 185, "ADD")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), before);
    document
        .delete_iec_ld_terminal_function(3, 185, "MOVE")
        .expect("delete first elevator MOVE block");

    let changed = document
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(changed[3].iec_row_frames().unwrap().len(), 75);
    assert_eq!(changed[3].iec_record_frames().unwrap().len(), 314);
    assert_eq!(changed[3].iec_function_blocks().unwrap().len(), 9);
    assert!(changed[3].iec_circuit_graph().is_some());

    let native = XgwxDocument::from_path(native).expect("native fixture parses");
    let native_programs = native
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for (index, (changed, native)) in changed.iter().zip(&native_programs).enumerate() {
        let mut changed_data = changed.data.clone();
        let mut native_data = native.data.clone();
        if index == 3 {
            let changed_rows = changed.iec_row_frames().unwrap();
            let native_rows = native.iec_row_frames().unwrap();
            let deleted_group_row = changed_rows
                .iter()
                .find(|row| row.group_index == 1 && row.row_index == 1)
                .unwrap();
            let native_deleted_group_row = native_rows
                .iter()
                .find(|row| row.group_index == 1 && row.row_index == 1)
                .unwrap();
            assert_eq!(changed_data[deleted_group_row.start + 17], 0x27);
            assert_eq!(native_data[native_deleted_group_row.start + 17], 0x27);

            // XG5000 refreshes this undocumented row cache only for function
            // rows visible in the editor viewport. Ignore it for all retained
            // rows while comparing the structural deletion byte-for-byte.
            for row in changed_rows {
                changed_data[row.start + 17] = 0;
            }
            for row in native_rows {
                native_data[row.start + 17] = 0;
            }
        }
        assert_eq!(changed_data, native_data, "program {index}");
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_NATIVE_STANDALONE_FUNCTION_DELETE_FIXTURE"]
fn xgi_standalone_function_deletion_matches_native() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let native = env::var("LIBXGWX_NATIVE_STANDALONE_FUNCTION_DELETE_FIXTURE")
        .expect("native XG5000 standalone function deletion capture");
    let mut document = XgwxDocument::from_path(&source).expect("fixture parses");
    assert_eq!(
        document
            .ladder_programs()
            .iter()
            .map(|program| {
                program
                    .as_ref()
                    .unwrap()
                    .iec_standalone_function_deletion_sites()
                    .unwrap()
                    .len()
            })
            .collect::<Vec<_>>(),
        vec![0, 0, 1, 0, 0, 0, 0]
    );
    let before = document.to_bytes().unwrap();
    assert!(
        document
            .delete_iec_ld_standalone_function(2, 206, "MOVE")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), before);
    document
        .delete_iec_ld_standalone_function(2, 206, "WORD_TO_UDINT")
        .expect("delete standalone WORD_TO_UDINT group");

    let changed = document
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(changed[2].iec_row_frames().unwrap().len(), 37);
    assert_eq!(changed[2].iec_record_frames().unwrap().len(), 151);
    assert_eq!(changed[2].iec_function_blocks().unwrap().len(), 8);
    assert!(changed[2].iec_circuit_graph().is_some());

    let native = XgwxDocument::from_path(native).expect("native fixture parses");
    let native_programs = native
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for (index, (changed, native)) in changed.iter().zip(&native_programs).enumerate() {
        let mut changed_data = changed.data.clone();
        let mut native_data = native.data.clone();
        if index == 2 {
            // XG5000 refreshed two visible retained-row caches while deleting
            // the group. The structural payload otherwise matches exactly.
            for row in changed.iec_row_frames().unwrap() {
                changed_data[row.start + 17] = 0;
            }
            for row in native.iec_row_frames().unwrap() {
                native_data[row.start + 17] = 0;
            }
        }
        assert_eq!(changed_data, native_data, "program {index}");
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_NATIVE_L26_STANDALONE_INSERT_FIXTURE"]
fn xgi_l26_standalone_function_insertion_matches_native() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let native = env::var("LIBXGWX_NATIVE_L26_STANDALONE_INSERT_FIXTURE")
        .expect("native L26 insertion fixture path");
    let mut document = XgwxDocument::from_path(source).expect("source parses");
    let source_programs = document.ladder_programs();
    assert_eq!(
        source_programs[4]
            .as_ref()
            .unwrap()
            .iec_standalone_function_insertion_sites()
            .unwrap(),
        vec![crate::IecStandaloneFunctionInsertionSite {
            group_index: 15,
            row_index: 26,
            insertion_offset: 4054,
            raw_x: 4,
        }]
    );
    document
        .insert_iec_ld_standalone_function(4, 4054, "WORD_TO_UDINT", "%MW301", "div_값")
        .expect("insert native L26 block");
    let native = XgwxDocument::from_path(native).expect("native insertion parses");
    for (index, (changed, native)) in document
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .enumerate()
    {
        let changed = changed.unwrap();
        let native = native.unwrap();
        let mut changed_data = changed.data.clone();
        let mut native_data = native.data.clone();
        if index == 4 {
            // XG5000 refreshes preexisting row display caches on Save As.
            for row in changed.iec_row_frames().unwrap() {
                changed_data[row.start + 17] = 0;
            }
            for row in native.iec_row_frames().unwrap() {
                native_data[row.start + 17] = 0;
            }
        }
        assert_eq!(changed_data, native_data, "program {index}");
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_NATIVE_STANDALONE_FUNCTION_DELETE_FIXTURE and LIBXGWX_NATIVE_STANDALONE_FUNCTION_INSERT_FIXTURE"]
fn xgi_standalone_function_insertion_matches_native() {
    let deleted = env::var("LIBXGWX_NATIVE_STANDALONE_FUNCTION_DELETE_FIXTURE")
        .expect("native deleted fixture path");
    let native = env::var("LIBXGWX_NATIVE_STANDALONE_FUNCTION_INSERT_FIXTURE")
        .expect("native inserted fixture path");
    let mut document = XgwxDocument::from_path(&deleted).expect("deleted fixture parses");
    let programs = document.ladder_programs();
    let program = programs[2].as_ref().unwrap();
    let sites = program.iec_standalone_function_insertion_sites().unwrap();
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].group_index, 1);
    assert_eq!(sites[0].row_index, 1);
    assert_eq!(sites[0].insertion_offset, 142);
    let before = document.to_bytes().unwrap();
    assert!(
        document
            .insert_iec_ld_standalone_function(2, 142, "MOVE", "%MW301", "변환")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), before);
    assert!(
        document
            .insert_iec_ld_standalone_function(2, 142, "WORD_TO_UDINT", "%MX302", "변환")
            .is_err()
    );
    assert!(
        document
            .insert_iec_ld_standalone_function(2, 142, "WORD_TO_UDINT", "%MW302", "missing")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), before);
    let mut alternate = XgwxDocument::from_path(&deleted).unwrap();
    alternate
        .insert_iec_ld_standalone_function(2, 142, "WORD_TO_UDINT", "%MW302", "변환")
        .expect("alternate WORD input is accepted");
    let alternate_program = alternate.ladder_programs()[2].as_ref().unwrap().clone();
    assert!(
        crate::iec_ld::function_operands(&alternate_program)
            .iter()
            .any(|item| item.value == "%MW302")
    );
    document
        .insert_iec_ld_standalone_function(2, 142, "WORD_TO_UDINT", "%MW301", "변환")
        .expect("insert native standalone WORD_TO_UDINT group");
    let native = XgwxDocument::from_path(native).expect("native inserted fixture parses");
    let changed = document.ladder_programs();
    let native_programs = native.ladder_programs();
    for (index, (changed, native)) in changed.into_iter().zip(native_programs).enumerate() {
        let changed = changed.unwrap();
        let native = native.unwrap();
        let mut changed_data = changed.data.clone();
        let mut native_data = native.data.clone();
        if index == 2 {
            let changed_rows = changed.iec_row_frames().unwrap();
            let native_rows = native.iec_row_frames().unwrap();
            for row in changed_rows {
                changed_data[row.start + 17] = 0;
            }
            for row in native_rows {
                native_data[row.start + 17] = 0;
            }
        }
        assert_eq!(changed_data, native_data, "program {index}");
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_NATIVE_STANDALONE_FUNCTION_DELETE_FIXTURE and LIBXGWX_NATIVE_STANDALONE_FUNCTION_ALT_OUTPUT_FIXTURE"]
fn xgi_standalone_function_insertion_with_new_output_survives_native_save() {
    let deleted = env::var("LIBXGWX_NATIVE_STANDALONE_FUNCTION_DELETE_FIXTURE")
        .expect("native deleted fixture path");
    let native = env::var("LIBXGWX_NATIVE_STANDALONE_FUNCTION_ALT_OUTPUT_FIXTURE")
        .expect("native resave with new output symbol");
    let mut generated = XgwxDocument::from_path(deleted).expect("deleted fixture parses");
    generated
        .insert_iec_local_symbol(2, "변환_2", "UDINT", "")
        .expect("insert output symbol");
    generated
        .insert_iec_ld_standalone_function(2, 142, "WORD_TO_UDINT", "%MW302", "변환_2")
        .expect("insert function with alternate bindings");
    let native = XgwxDocument::from_path(native).expect("native resave parses");
    let generated_programs = generated
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let native_programs = native
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for (index, (generated, native)) in generated_programs.iter().zip(&native_programs).enumerate()
    {
        assert_eq!(generated.data, native.data, "program {index}");
    }
    let generated_symbols = generated
        .iec_local_symbols()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let native_symbols = native
        .iec_local_symbols()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(generated_symbols, native_symbols);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_NATIVE_COIL_DELETE_FIXTURE"]
fn xgi_terminal_coil_deletion_matches_native_and_restores() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let native = env::var("LIBXGWX_NATIVE_COIL_DELETE_FIXTURE").expect("native coil deletion path");
    let mut document = XgwxDocument::from_path(source).expect("source parses");
    let original = document.to_bytes().unwrap();
    assert!(
        document
            .delete_iec_ld_terminal_coil(0, 271, "wrong")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), original);
    document
        .delete_iec_ld_terminal_coil(0, 271, "시작")
        .unwrap();
    let saved = XgwxDocument::from_path(native).expect("native deletion parses");
    for (generated, native) in document
        .ladder_programs()
        .into_iter()
        .zip(saved.ladder_programs())
    {
        assert_eq!(generated.unwrap().data, native.unwrap().data);
    }
    if let Ok(path) = env::var("LIBXGWX_NATIVE_COIL_DELETE_RESAVED_FIXTURE") {
        let resaved = XgwxDocument::from_path(path).expect("XG5000 resave parses");
        for (generated, native) in document
            .ladder_programs()
            .into_iter()
            .zip(resaved.ladder_programs())
        {
            assert_eq!(generated.unwrap().data, native.unwrap().data);
        }
    }
    let deleted = document.to_bytes().unwrap();
    assert!(
        document
            .insert_iec_ld_terminal_coil(0, 223, "스위치_1", "OUTPUT", "UNKNOWN")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), deleted);
    for (kind, code) in [
        ("OUTPUT", 0x0e),
        ("INVERSE", 0x0f),
        ("SET", 0x10),
        ("RESET", 0x11),
        ("RISING", 0x12),
        ("FALLING", 0x13),
    ] {
        let mut variant = XgwxDocument::parse(&deleted).unwrap();
        variant
            .insert_iec_ld_terminal_coil(0, 223, "스위치_1", kind, "시작")
            .unwrap();
        let program = variant.ladder_programs().remove(0).unwrap();
        assert!(
            program
                .iec_record_frames()
                .unwrap()
                .iter()
                .any(|record| { record.offset == 271 && record.kind == IecRecordKind::Coil(code) })
        );
        assert!(program.iec_circuit_graph().is_some());
        if kind == "SET" {
            if let Ok(path) = env::var("LIBXGWX_NATIVE_SET_COIL_RESAVED_FIXTURE") {
                let resaved = XgwxDocument::from_path(path).expect("XG5000 SET resave parses");
                for (generated, native) in variant
                    .ladder_programs()
                    .into_iter()
                    .zip(resaved.ladder_programs())
                {
                    assert_eq!(generated.unwrap().data, native.unwrap().data);
                }
            }
        }
    }
    document
        .insert_iec_ld_terminal_coil(0, 223, "스위치_1", "OUTPUT", "시작")
        .unwrap();
    let restored = XgwxDocument::parse(&original).unwrap();
    for (generated, source) in document
        .ladder_programs()
        .into_iter()
        .zip(restored.ladder_programs())
    {
        let generated = generated.unwrap().data;
        let source = source.unwrap().data;
        assert_eq!(generated.len(), source.len());
        assert_eq!(
            generated.iter().zip(&source).position(|(a, b)| a != b),
            None
        );
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_complete_iec_group_deletion_keeps_other_networks() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    for (group_index, first_row) in [(3, 3), (14, 20)] {
        let mut document = XgwxDocument::from_path(&source).unwrap();
        let original = document.to_bytes().unwrap();
        assert!(
            document
                .delete_iec_ld_group(0, group_index, first_row + 1)
                .is_err()
        );
        assert_eq!(document.to_bytes().unwrap(), original);
        let before = document.ladder_programs().remove(0).unwrap();
        let removed_rows = before
            .iec_row_frames()
            .unwrap()
            .iter()
            .filter(|row| row.group_index == group_index)
            .count();
        document
            .delete_iec_ld_group(0, group_index, first_row)
            .unwrap();
        let after = document.ladder_programs().remove(0).unwrap();
        assert_eq!(
            after.iec_row_frames().unwrap().len(),
            before.iec_row_frames().unwrap().len() - removed_rows
        );
        assert!(after.iec_circuit_graph().is_some());
        for (before, after) in XgwxDocument::parse(&original)
            .unwrap()
            .ladder_programs()
            .into_iter()
            .skip(1)
            .zip(document.ladder_programs().into_iter().skip(1))
        {
            assert_eq!(before.unwrap().data, after.unwrap().data);
        }
        let native_key = if group_index == 3 {
            "LIBXGWX_NATIVE_GROUP3_RESAVED_FIXTURE"
        } else {
            "LIBXGWX_NATIVE_GROUP14_RESAVED_FIXTURE"
        };
        if let Ok(path) = env::var(native_key) {
            let native = XgwxDocument::from_path(path).unwrap();
            for (generated, resaved) in document
                .ladder_programs()
                .into_iter()
                .zip(native.ladder_programs())
            {
                assert_eq!(generated.unwrap().data, resaved.unwrap().data);
            }
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_every_captured_iec_group_can_be_cleared() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let baseline = XgwxDocument::from_path(&source).unwrap();
    for (program_index, program) in baseline.ladder_programs().into_iter().enumerate() {
        let program = program.unwrap();
        let rows = program.iec_row_frames().unwrap();
        let groups = rows
            .iter()
            .map(|row| row.group_index)
            .collect::<std::collections::BTreeSet<_>>();
        if groups.len() <= 1 {
            continue;
        }
        for group_index in groups {
            let first_row = rows
                .iter()
                .find(|row| row.group_index == group_index)
                .unwrap()
                .row_index;
            let mut document = XgwxDocument::from_path(&source).unwrap();
            document
                .delete_iec_ld_group(program_index, group_index, first_row)
                .unwrap_or_else(|error| {
                    panic!("program {program_index} group {group_index} L{first_row}: {error}")
                });
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_complete_iec_groups_move_into_empty_ranges() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    for (delete, group_index, first_row, destination, native_key) in [
        (
            None,
            4,
            6,
            5,
            "LIBXGWX_NATIVE_GROUP_MOVE_SIMPLE_RESAVED_FIXTURE",
        ),
        (
            Some((33, 67)),
            14,
            20,
            67,
            "LIBXGWX_NATIVE_GROUP_MOVE_FUNCTION_RESAVED_FIXTURE",
        ),
    ] {
        let mut document = XgwxDocument::from_path(&source).unwrap();
        if let Some((removed_group, removed_row)) = delete {
            document
                .delete_iec_ld_group(0, removed_group, removed_row)
                .unwrap();
        }
        let before = document.to_bytes().unwrap();
        assert!(
            document
                .move_iec_ld_group(0, group_index, first_row, first_row)
                .is_err()
        );
        assert!(
            document
                .move_iec_ld_group(0, group_index, first_row + 1, destination)
                .is_err()
        );
        assert_eq!(document.to_bytes().unwrap(), before);
        let program = document.ladder_programs().remove(0).unwrap();
        let before_records = program.iec_record_frames().unwrap().len();
        let before_functions = program.iec_function_blocks().unwrap().len();
        document
            .move_iec_ld_group(0, group_index, first_row, destination)
            .unwrap();
        let moved = document.ladder_programs().remove(0).unwrap();
        assert_eq!(moved.iec_record_frames().unwrap().len(), before_records);
        assert_eq!(moved.iec_function_blocks().unwrap().len(), before_functions);
        assert!(moved.iec_circuit_graph().is_some());
        assert!(
            moved
                .iec_row_frames()
                .unwrap()
                .iter()
                .any(|row| row.row_index == destination)
        );
        assert!(
            !moved
                .iec_row_frames()
                .unwrap()
                .iter()
                .any(|row| row.row_index == first_row)
        );
        if let Ok(path) = env::var(native_key) {
            let native = XgwxDocument::from_path(path).unwrap();
            for (generated, resaved) in document
                .ladder_programs()
                .into_iter()
                .zip(native.ladder_programs())
            {
                assert_eq!(generated.unwrap().data, resaved.unwrap().data);
            }
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_complete_iec_groups_copy_into_empty_ranges() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    for (delete, group_index, first_row, destination, native_key) in [
        (
            None,
            4,
            6,
            5,
            "LIBXGWX_NATIVE_GROUP_COPY_SIMPLE_RESAVED_FIXTURE",
        ),
        (
            Some((33, 67)),
            14,
            20,
            67,
            "LIBXGWX_NATIVE_GROUP_COPY_FUNCTION_RESAVED_FIXTURE",
        ),
    ] {
        let mut document = XgwxDocument::from_path(&source).unwrap();
        if let Some((removed_group, removed_row)) = delete {
            document
                .delete_iec_ld_group(0, removed_group, removed_row)
                .unwrap();
        }
        let before = document.to_bytes().unwrap();
        assert!(
            document
                .copy_iec_ld_group(0, group_index, first_row, first_row)
                .is_err()
        );
        assert!(
            document
                .copy_iec_ld_group(0, group_index, first_row + 1, destination)
                .is_err()
        );
        assert_eq!(document.to_bytes().unwrap(), before);
        let prior = document.ladder_programs().remove(0).unwrap();
        let source_rows = prior
            .iec_row_frames()
            .unwrap()
            .iter()
            .filter(|row| row.group_index == group_index)
            .count();
        let source_records = prior
            .iec_record_frames()
            .unwrap()
            .iter()
            .filter(|record| record.group_index == group_index)
            .count();
        document
            .copy_iec_ld_group(0, group_index, first_row, destination)
            .unwrap();
        let copied = document.ladder_programs().remove(0).unwrap();
        assert_eq!(
            copied.iec_row_frames().unwrap().len(),
            prior.iec_row_frames().unwrap().len() + source_rows
        );
        assert_eq!(
            copied.iec_record_frames().unwrap().len(),
            prior.iec_record_frames().unwrap().len() + source_records
        );
        assert!(
            copied
                .iec_row_frames()
                .unwrap()
                .iter()
                .any(|row| row.row_index == first_row)
        );
        assert!(
            copied
                .iec_row_frames()
                .unwrap()
                .iter()
                .any(|row| row.row_index == destination)
        );
        assert!(copied.iec_circuit_graph().is_some());
        if let Ok(path) = env::var(native_key) {
            let native = XgwxDocument::from_path(path).unwrap();
            for (generated, resaved) in document
                .ladder_programs()
                .into_iter()
                .zip(native.ladder_programs())
            {
                assert_eq!(generated.unwrap().data, resaved.unwrap().data);
            }
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_iec_network_replacement_is_atomic_and_preserves_other_programs() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let original = XgwxDocument::from_path(path).unwrap();
    let before = original.to_bytes().unwrap();
    for (source_group, source_row, destination_group, destination_row) in [
        (2, 2, 4, 6),
        (4, 6, 2, 2),
        (14, 20, 16, 26),
        (16, 26, 14, 20),
    ] {
        let mut document = original.clone();
        assert!(
            document
                .replace_iec_ld_group(
                    0,
                    source_group,
                    source_row + 1,
                    destination_group,
                    destination_row
                )
                .is_err()
        );
        assert!(
            document
                .replace_iec_ld_group(0, source_group, source_row, source_group, source_row)
                .is_err()
        );
        assert_eq!(document.to_bytes().unwrap(), before);
        document
            .replace_iec_ld_group(
                0,
                source_group,
                source_row,
                destination_group,
                destination_row,
            )
            .unwrap();
        let reparsed = XgwxDocument::parse(&document.to_bytes().unwrap()).unwrap();
        let original_programs = original.ladder_programs();
        let edited_programs = reparsed.ladder_programs();
        for index in 1..7 {
            assert_eq!(
                original_programs[index].as_ref().unwrap().data,
                edited_programs[index].as_ref().unwrap().data
            );
        }
        let old = original_programs[0].as_ref().unwrap();
        let new = edited_programs[0].as_ref().unwrap();
        assert_eq!(
            old.iec_row_frames().unwrap().len(),
            new.iec_row_frames().unwrap().len()
        );
        assert_eq!(
            old.iec_record_frames().unwrap().len(),
            new.iec_record_frames().unwrap().len()
        );
        assert!(new.iec_circuit_graph().is_some());
        assert_ne!(old.data, new.data);
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_cross_program_function_copy_checks_instances_and_restores() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let original = XgwxDocument::from_path(path).unwrap();
    let mut missing_instance = original.clone();
    missing_instance.delete_iec_ld_group(3, 1, 1).unwrap();
    let before = missing_instance.to_bytes().unwrap();
    assert!(
        missing_instance
            .copy_iec_ld_group_to_program(0, 14, 20, 3, 1)
            .is_err()
    );
    assert_eq!(missing_instance.to_bytes().unwrap(), before);

    let mut document = original.clone();
    document.delete_iec_ld_group(3, 1, 1).unwrap();
    let cleared = document.ladder_programs();
    let before = document.to_bytes().unwrap();
    assert!(
        document
            .copy_iec_ld_group_to_program(2, 1, 2, 3, 1)
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), before);
    document
        .copy_iec_ld_group_to_program(2, 1, 1, 3, 1)
        .unwrap();
    let copied = XgwxDocument::parse(&document.to_bytes().unwrap()).unwrap();
    let programs = copied.ladder_programs();
    let destination = programs[3].as_ref().unwrap();
    assert!(destination.iec_circuit_graph().is_some());
    let group = destination
        .iec_row_frames()
        .unwrap()
        .into_iter()
        .find(|row| row.row_index == 1)
        .unwrap()
        .group_index;
    assert!(
        destination
            .iec_function_blocks()
            .unwrap()
            .iter()
            .any(|block| block.group_index == group && block.name.value == "WORD_TO_UDINT")
    );
    for index in [0, 1, 2, 4, 5, 6] {
        assert_eq!(
            programs[index].as_ref().unwrap().data,
            original.ladder_programs()[index].as_ref().unwrap().data
        );
    }
    document.delete_iec_ld_group(3, group, 1).unwrap();
    for (restored, expected) in document.ladder_programs().into_iter().zip(cleared) {
        assert_eq!(restored.unwrap().data, expected.unwrap().data);
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_NATIVE_GROUP_COPY_FUNCTION_FIXTURE to the generated function-copy project"]
fn xgi_copied_function_can_receive_independent_instance() {
    let path = env::var("LIBXGWX_NATIVE_GROUP_COPY_FUNCTION_FIXTURE")
        .expect("generated function-copy fixture path");
    let mut doc = XgwxDocument::from_path(path).expect("fixture parses");
    let original = doc.to_bytes().expect("original bytes");
    let blocks = doc.ladder_programs()[0]
        .as_ref()
        .unwrap()
        .iec_function_blocks()
        .unwrap();
    let copied = blocks
        .iter()
        .find(|block| block.group_index == 33 && block.name.value == "R_TRIG")
        .expect("copied R_TRIG");
    let original_instance = copied.instance.as_ref().unwrap().value.clone();
    assert!(
        doc.duplicate_iec_ld_function_instance(0, copied.record_offset, "WRONG", "INST4")
            .is_err()
    );
    assert!(
        doc.duplicate_iec_ld_function_instance(
            0,
            copied.record_offset,
            &original_instance,
            "INST3"
        )
        .is_err()
    );
    assert_eq!(doc.to_bytes().unwrap(), original);
    doc.duplicate_iec_ld_function_instance(0, copied.record_offset, &original_instance, "INST4")
        .expect("duplicate instance and rebind copied block");
    let saved = XgwxDocument::parse(&doc.to_bytes().unwrap()).expect("round trip");
    let symbols = saved.iec_local_symbols();
    let local = symbols[0].as_ref().unwrap();
    assert_eq!(local.len(), 16);
    assert!(local.iter().any(|symbol| {
        symbol.name == "INST4"
            && symbol.is_instance
            && symbol.type_reference.as_deref() == Some("R_TRIG")
    }));
    let blocks = saved.ladder_programs()[0]
        .as_ref()
        .unwrap()
        .iec_function_blocks()
        .unwrap();
    assert_eq!(
        blocks
            .iter()
            .find(|block| block.group_index == 14 && block.name.value == "R_TRIG")
            .unwrap()
            .instance
            .as_ref()
            .unwrap()
            .value,
        original_instance
    );
    assert_eq!(
        blocks
            .iter()
            .find(|block| block.group_index == 33 && block.name.value == "R_TRIG")
            .unwrap()
            .instance
            .as_ref()
            .unwrap()
            .value,
        "INST4"
    );
    if let Ok(path) = env::var("LIBXGWX_NATIVE_GROUP_COPY_INSTANCE_RESAVED_FIXTURE") {
        let native = XgwxDocument::from_path(path).expect("native Save As parses");
        assert_eq!(
            saved
                .iec_local_symbols()
                .into_iter()
                .map(Result::unwrap)
                .collect::<Vec<_>>(),
            native
                .iec_local_symbols()
                .into_iter()
                .map(Result::unwrap)
                .collect::<Vec<_>>()
        );
        for (generated, resaved) in saved
            .ladder_programs()
            .into_iter()
            .zip(native.ladder_programs())
        {
            assert_eq!(generated.unwrap().data, resaved.unwrap().data);
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_NATIVE_CONNECTED_ADD_RESAVE_FIXTURE"]
fn xgi_connected_add_deletion_survives_native_save() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let resaved = env::var("LIBXGWX_NATIVE_CONNECTED_ADD_RESAVE_FIXTURE")
        .expect("native resave of generated ADD deletion");
    let mut document = XgwxDocument::from_path(source).expect("source parses");
    let before = document.to_bytes().unwrap();
    assert_eq!(
        document.ladder_programs()[0]
            .as_ref()
            .unwrap()
            .iec_connected_arithmetic_deletion_sites()
            .unwrap()
            .len(),
        2,
    );
    assert!(
        document
            .delete_iec_ld_connected_arithmetic(0, 2858, "SUB")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), before);
    document
        .delete_iec_ld_connected_arithmetic(0, 2858, "ADD")
        .unwrap();
    let resaved = XgwxDocument::from_path(resaved).expect("native resave parses");
    let generated = document
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let native = resaved
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(generated.len(), native.len());
    for (generated, native) in generated.iter().zip(&native) {
        assert_eq!(generated.data, native.data);
        assert!(generated.iec_circuit_graph().is_some());
    }
    let rows = generated[0].iec_row_frames().unwrap();
    assert_eq!(
        rows.iter()
            .find(|row| row.row_index == 20)
            .unwrap()
            .group_index,
        14
    );
    assert_eq!(
        rows.iter()
            .find(|row| row.row_index == 22)
            .unwrap()
            .group_index,
        15
    );
    assert_eq!(generated[0].iec_record_frames().unwrap().len(), 281);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_NATIVE_CONNECTED_SUB_RESAVE_FIXTURE"]
fn xgi_connected_sub_deletion_survives_native_save() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let resaved = env::var("LIBXGWX_NATIVE_CONNECTED_SUB_RESAVE_FIXTURE")
        .expect("native resave of generated SUB deletion");
    let mut document = XgwxDocument::from_path(source).expect("source parses");
    let before = document.to_bytes().unwrap();
    assert!(
        document
            .delete_iec_ld_connected_arithmetic(0, 4076, "ADD")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), before);
    document
        .delete_iec_ld_connected_arithmetic(0, 4076, "SUB")
        .unwrap();
    let resaved = XgwxDocument::from_path(resaved).expect("native resave parses");
    let generated = document
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let native = resaved
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(generated.len(), native.len());
    for (generated, native) in generated.iter().zip(&native) {
        assert_eq!(generated.data, native.data);
        assert!(generated.iec_circuit_graph().is_some());
    }
    let rows = generated[0].iec_row_frames().unwrap();
    assert_eq!(
        rows.iter()
            .find(|row| row.row_index == 26)
            .unwrap()
            .group_index,
        16
    );
    assert_eq!(
        rows.iter()
            .find(|row| row.row_index == 28)
            .unwrap()
            .group_index,
        17
    );
    assert_eq!(generated[0].iec_record_frames().unwrap().len(), 281);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_NATIVE_FUNCTION_CELL_DELETE_FIXTURE"]
fn xgi_function_cell_deletion_matches_native() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let native = env::var("LIBXGWX_NATIVE_FUNCTION_CELL_DELETE_FIXTURE")
        .expect("native XG5000 function cell deletion capture");
    let mut document = XgwxDocument::from_path(&source).expect("fixture parses");
    assert_eq!(
        document
            .ladder_programs()
            .iter()
            .map(|program| {
                program
                    .as_ref()
                    .unwrap()
                    .iec_function_cell_deletion_sites()
                    .unwrap()
                    .len()
            })
            .collect::<Vec<_>>(),
        vec![1, 0, 0, 0, 0, 0, 0]
    );
    let before = document.to_bytes().unwrap();
    assert!(
        document
            .delete_iec_ld_function_cell(0, 1639, "R_TRIG")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), before);
    document
        .delete_iec_ld_function_cell(0, 1639, "FF")
        .expect("delete connected FF function cell");

    let changed = document
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(changed[0].iec_row_frames().unwrap().len(), 86);
    assert_eq!(changed[0].iec_record_frames().unwrap().len(), 287);
    assert_eq!(changed[0].iec_function_blocks().unwrap().len(), 22);
    assert!(changed[0].iec_circuit_graph().is_some());

    let native = XgwxDocument::from_path(native).expect("native fixture parses");
    let native_programs = native
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for (index, (changed, native)) in changed.iter().zip(&native_programs).enumerate() {
        let mut changed_data = changed.data.clone();
        let mut native_data = native.data.clone();
        if index == 0 {
            // XG5000 refreshed seven visible retained-row caches. The function
            // and link deletion otherwise matches the native payload exactly.
            for row in changed.iec_row_frames().unwrap() {
                changed_data[row.start + 17] = 0;
            }
            for row in native.iec_row_frames().unwrap() {
                native_data[row.start + 17] = 0;
            }
        }
        assert_eq!(changed_data, native_data, "program {index}");
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_NATIVE_FUNCTION_CELL_DELETE_FIXTURE and LIBXGWX_NATIVE_FUNCTION_CELL_INSERT_FIXTURE"]
fn xgi_function_cell_insertion_matches_native() {
    let deleted = env::var("LIBXGWX_NATIVE_FUNCTION_CELL_DELETE_FIXTURE")
        .expect("native XG5000 function cell deletion capture");
    let native = env::var("LIBXGWX_NATIVE_FUNCTION_CELL_INSERT_FIXTURE")
        .expect("native XG5000 function cell insertion capture");
    let mut document = XgwxDocument::from_path(&deleted).expect("deleted fixture parses");
    assert_eq!(
        document
            .ladder_programs()
            .iter()
            .map(|program| {
                program
                    .as_ref()
                    .unwrap()
                    .iec_function_cell_insertion_sites()
                    .unwrap()
                    .len()
            })
            .collect::<Vec<_>>(),
        vec![1, 0, 0, 0, 0, 0, 0]
    );
    let site = document.ladder_programs()[0]
        .as_ref()
        .unwrap()
        .iec_function_cell_insertion_sites()
        .unwrap()[0];
    assert_eq!(site.group_index, 9);
    assert_eq!(site.row_index, 14);
    assert_eq!(site.insertion_offset, 1639);
    assert_eq!(site.reference_offset, 1766);
    assert_eq!(site.raw_x, 4);

    let before = document.to_bytes().unwrap();
    assert!(
        document
            .insert_iec_ld_function_cell(0, 1639, "R_TRIG", "FF")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), before);
    assert!(
        document
            .insert_iec_ld_function_cell(0, 1639, "FF", "INST3")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), before);
    document
        .insert_iec_ld_function_cell(0, 1639, "FF", "FF")
        .expect("insert connected FF function cell");

    let changed = document
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(changed[0].iec_row_frames().unwrap().len(), 86);
    assert_eq!(changed[0].iec_record_frames().unwrap().len(), 289);
    assert_eq!(changed[0].iec_function_blocks().unwrap().len(), 23);
    assert_eq!(changed[0].iec_function_references().unwrap().len(), 50);
    assert_eq!(changed[0].iec_function_operand_links().unwrap().len(), 45);
    assert!(
        changed[0]
            .iec_function_cell_insertion_sites()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        changed[0].iec_function_cell_deletion_sites().unwrap().len(),
        1
    );
    assert!(changed[0].iec_circuit_graph().is_some());

    let native = XgwxDocument::from_path(native).expect("native inserted fixture parses");
    let native_programs = native
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for (index, (changed, native)) in changed.iter().zip(&native_programs).enumerate() {
        let mut changed_data = changed.data.clone();
        let mut native_data = native.data.clone();
        if index == 0 {
            // Native insertion refreshes the selected row's visible cache.
            // The generated function, link, counts, and graph match exactly.
            for row in changed.iec_row_frames().unwrap() {
                changed_data[row.start + 17] = 0;
            }
            for row in native.iec_row_frames().unwrap() {
                native_data[row.start + 17] = 0;
            }
        }
        assert_eq!(changed_data, native_data, "program {index}");
    }
}

#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_circuit_graph_covers_every_decoded_element_and_function_binding() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let doc = XgwxDocument::from_path(path).expect("fixture parses");
    let programs = doc
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("all programs parse");
    let graphs = programs
        .iter()
        .map(|program| {
            program
                .iec_circuit_graph()
                .expect("validated circuit graph")
        })
        .collect::<Vec<_>>();

    assert_eq!(
        graphs
            .iter()
            .map(|graph| graph.edges.len())
            .collect::<Vec<_>>(),
        [153, 47, 101, 239, 69, 76, 212]
    );
    assert_eq!(
        graphs
            .iter()
            .map(|graph| graph.occupied_areas.len())
            .collect::<Vec<_>>(),
        [171, 44, 105, 224, 88, 80, 169]
    );
    assert_eq!(
        graphs
            .iter()
            .map(|graph| graph.function_bindings.len())
            .collect::<Vec<_>>(),
        [50, 4, 23, 24, 23, 28, 44]
    );
    assert_eq!(
        graphs
            .iter()
            .map(|graph| graph.power_components.len())
            .collect::<Vec<_>>(),
        [24, 5, 15, 29, 13, 7, 9]
    );
    assert_eq!(
        graphs
            .iter()
            .flat_map(|graph| &graph.function_bindings)
            .filter(|binding| binding.expression_record_offset.is_some())
            .count(),
        176
    );

    for graph in &graphs {
        let mut component_edges = graph
            .power_components
            .iter()
            .flat_map(|component| component.edge_indices.iter().copied())
            .collect::<Vec<_>>();
        component_edges.sort_unstable();
        assert_eq!(component_edges, (0..graph.edges.len()).collect::<Vec<_>>());
        assert!(graph.edges.iter().all(|edge| {
            edge.start.group_index == edge.end.group_index
                && edge.start.x <= 96
                && edge.end.x <= 96
                && edge.start.x % 3 == 0
                && edge.end.x % 3 == 0
        }));
        assert!(graph.function_bindings.iter().all(|binding| {
            binding.data_type_mask != 0
                && binding
                    .expression_cell_x
                    .is_none_or(|raw_x| match binding.direction {
                        IecFunctionPinDirection::Input => {
                            raw_x.checked_add(2) == Some(binding.pin_point.x)
                        }
                        IecFunctionPinDirection::Output => {
                            raw_x.checked_sub(1) == Some(binding.pin_point.x)
                        }
                    })
        }));
    }

    let mut overlap = programs[0].clone();
    let short_wire = overlap
        .iec_record_frames()
        .unwrap()
        .into_iter()
        .find(|record| {
            record.group_index == 16
                && record.row_index == 26
                && record.kind == IecRecordKind::ShortWire
        })
        .expect("captured short wire");
    overlap.data[short_wire.offset + 5] = 1;
    assert!(overlap.iec_record_frames().is_some());
    assert!(overlap.iec_circuit_graph().is_none());

    let mut isolated_branch = programs[0].clone();
    let branch = isolated_branch
        .iec_geometry()
        .unwrap()
        .vertical
        .into_iter()
        .find(|connection| {
            connection.group_index == 3
                && connection.start_row_index == 3
                && connection.end_row_index == 4
                && connection.x == 6
        })
        .expect("captured branch");
    isolated_branch.data[branch.start_offset + 7] = 9;
    isolated_branch.data[branch.start_offset + 17] = 8;
    isolated_branch.data[branch.end_offset + 5] = 9;
    assert!(isolated_branch.iec_record_frames().is_some());
    assert!(isolated_branch.iec_geometry().is_some());
    assert!(isolated_branch.iec_circuit_graph().is_none());
}

#[test]
#[ignore = "set LIBXGWX_NATIVE_GROUP33_DELETE_FIXTURE to the native XG5000 row deletion capture"]
fn xgi_native_eq_row_deletion_retains_flagged_vertical_branch() {
    let path = env::var("LIBXGWX_NATIVE_GROUP33_DELETE_FIXTURE")
        .expect("native XG5000 EQ row deletion capture");
    let document = XgwxDocument::from_path(path).expect("native file parses");
    let program = document
        .ladder_programs()
        .remove(0)
        .expect("program decodes");
    let rows = program.iec_row_frames().expect("rows frame");
    let records = program.iec_record_frames().expect("records frame");
    let geometry = program.iec_geometry().expect("geometry frames");
    assert_eq!(rows.iter().filter(|row| row.group_index == 33).count(), 15);
    assert_eq!(program.iec_function_blocks().unwrap().len(), 22);
    assert_eq!(program.iec_circuit_graph().unwrap().edges.len(), 150);
    let flagged = records
        .iter()
        .find(|record| {
            record.group_index == 33
                && record.row_index == 68
                && record.kind == IecRecordKind::BranchStart
                && program.data[record.offset + 7] == 12
        })
        .expect("flagged x12 branch start");
    assert_eq!(
        &program.data[flagged.offset + 10..flagged.offset + 17],
        &[0, 0, 0, 4, 0, 0, 0]
    );
    assert_eq!(
        &program.data[flagged.offset + 20..flagged.offset + 27],
        &[0, 0, 0, 4, 0, 0, 0]
    );
    assert!(geometry.vertical.iter().any(|branch| {
        branch.group_index == 33
            && branch.start_row_index == 68
            && branch.end_row_index == 69
            && branch.x == 12
    }));
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_NATIVE_GROUP33_DELETE_FIXTURE"]
fn xgi_eq_chain_head_delete_matches_native_group_and_preserves_other_programs() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home source");
    let native =
        env::var("LIBXGWX_NATIVE_GROUP33_DELETE_FIXTURE").expect("native XG5000 deletion capture");
    let mut generated = XgwxDocument::from_path(source).expect("source parses");
    let before = generated.to_bytes().unwrap();
    assert!(
        generated
            .delete_iec_ld_eq_chain_head(0, 0x2a5c, "GT")
            .is_err()
    );
    assert_eq!(generated.to_bytes().unwrap(), before);
    generated
        .delete_iec_ld_eq_chain_head(0, 0x2a5c, "EQ")
        .expect("delete captured EQ chain head");
    let native = XgwxDocument::from_path(native).expect("native capture parses");
    let generated_programs = generated.ladder_programs();
    let native_programs = native.ladder_programs();
    for (index, (generated, native)) in generated_programs
        .into_iter()
        .zip(native_programs)
        .enumerate()
    {
        let generated = generated.unwrap();
        let native = native.unwrap();
        if index == 0 {
            let mut generated_data = generated.data.clone();
            let mut native_data = native.data.clone();
            // XG5000 refreshes four pre-existing row display caches outside
            // the edited group when saving this project.
            for offset in [0xc88, 0xf1f, 0x114a, 0x27f5] {
                generated_data[offset] = 0;
                native_data[offset] = 0;
            }
            assert_eq!(generated_data, native_data);
        } else {
            assert_eq!(generated.data, native.data, "program {index}");
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_inserts_standalone_comment_into_empty_row() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let source = XgwxDocument::from_path(path).expect("fixture parses");
    let mut document = source.clone();
    let before = document.to_bytes().unwrap();
    assert!(document.insert_iec_ld_comment(0, 5, "").is_err());
    assert!(document.insert_iec_ld_comment(0, 6, "새 설명").is_err());
    assert_eq!(document.to_bytes().unwrap(), before);
    document.insert_iec_ld_comment(0, 5, "새 설명").unwrap();
    let updated = XgwxDocument::parse(&document.to_bytes().unwrap()).unwrap();
    let original = source.ladder_programs()[0].as_ref().unwrap().clone();
    let program = updated.ladder_programs()[0].as_ref().unwrap().clone();
    let row = program
        .iec_row_frames()
        .unwrap()
        .into_iter()
        .find(|row| row.row_index == 5)
        .unwrap();
    assert_eq!(row.record_count, 1);
    let comment = crate::iec_ld::comments(&program)
        .into_iter()
        .find(|item| item.value == "새 설명")
        .unwrap();
    assert!(comment.offset >= row.records_start && comment.offset < row.end);
    assert!(program.iec_circuit_graph().is_some());
    assert_eq!(
        program.iec_row_frames().unwrap().len(),
        original.iec_row_frames().unwrap().len() + 1
    );
    for index in 1..7 {
        assert_eq!(
            updated.ladder_programs()[index].as_ref().unwrap().data,
            source.ladder_programs()[index].as_ref().unwrap().data
        );
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_shifts_function_pin_rows_for_program_one_comment() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let source = XgwxDocument::from_path(path).expect("fixture parses");
    let mut document = source.clone();
    document
        .insert_iec_ld_blank_row(1, 12)
        .expect("append blank row after TON and MOVE");
    document
        .insert_iec_ld_comment(1, 13, "추가 설명")
        .expect("fill new row");
    let updated = XgwxDocument::parse(&document.to_bytes().unwrap()).unwrap();
    let program = updated.ladder_programs()[1].as_ref().unwrap().clone();
    assert!(program.iec_circuit_graph().is_some());
    assert!(
        crate::iec_ld::comments(&program)
            .iter()
            .any(|item| item.value == "추가 설명")
    );
    let mut shifted = source;
    shifted
        .insert_iec_ld_blank_row(1, 5)
        .expect("shift MOVE function and its pins");
    shifted
        .insert_iec_ld_comment(1, 6, "중간 설명")
        .expect("fill middle row");
    let program = shifted.ladder_programs()[1].as_ref().unwrap().clone();
    assert!(program.iec_circuit_graph().is_some());
    assert!(
        program
            .iec_function_blocks()
            .unwrap()
            .iter()
            .any(|block| block.name.value == "MOVE" && block.row_index == 7)
    );
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_reuses_existing_contact_operand_for_serial_insertion() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let mut document = XgwxDocument::from_path(path).expect("fixture parses");
    let before = document.to_bytes().unwrap();
    assert!(
        document
            .insert_iec_ld_contact(0, 252, 25, 4, 91, "NO", "UNKNOWN_SWITCH")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), before);
    document
        .insert_iec_ld_contact(0, 252, 25, 4, 91, "NO", "ON")
        .unwrap();
    let program = document.ladder_programs()[0].as_ref().unwrap().clone();
    let right = program
        .iec_no_contact_insertion_sites()
        .unwrap()
        .into_iter()
        .find(|site| site.row_index == 2 && site.start_x == 28 && site.end_x == 91)
        .expect("right wire remains available");
    document
        .insert_iec_ld_contact(0, right.wire_offset, 49, 28, 91, "NC", "스위치_1")
        .expect("captured contact operand can be reused");
    let changed = XgwxDocument::parse(&document.to_bytes().unwrap()).unwrap();
    let program = changed.ladder_programs()[0].as_ref().unwrap().clone();
    assert!(program.iec_circuit_graph().is_some());
    let contacts = crate::iec_ld::element_operands(&program);
    assert!(
        contacts.iter().any(|item| {
            item.record_code == 0x06 && item.raw_x == 25 && item.string.value == "ON"
        })
    );
    assert!(contacts.iter().any(|item| {
        item.record_code == 0x07 && item.raw_x == 49 && item.string.value == "스위치_1"
    }));
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_element_operand_edits_check_bool_type_and_writable_output() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let source = XgwxDocument::from_path(path).expect("fixture parses");
    let programs = source.ladder_programs();
    let program = programs[0].as_ref().unwrap();
    let operands = crate::iec_ld::element_operands(program);
    let contact = operands
        .iter()
        .find(|item| item.record_code == 0x08 && item.string.value == "스위치_1")
        .expect("captured rising contact");
    let coil = operands
        .iter()
        .find(|item| item.record_code == 0x0e && item.string.value != "ON")
        .expect("captured output coil");

    let mut valid_contact = source.clone();
    valid_contact
        .update_iec_ld_element_operand(0, contact.string.offset, "스위치_1", "ON")
        .expect("existing BOOL contact operand");
    let changed = XgwxDocument::parse(&valid_contact.to_bytes().unwrap()).unwrap();
    assert!(
        changed.ladder_programs()[0]
            .as_ref()
            .unwrap()
            .strings
            .iter()
            .any(|item| item.offset == contact.string.offset && item.value == "ON")
    );

    let mut valid_coil = source.clone();
    valid_coil
        .update_iec_ld_element_operand(0, coil.string.offset, &coil.string.value, "%MX77")
        .expect("writable BOOL coil address");

    for replacement in ["1", "%MW700", "%MX", "%MX1.", "%NONSENSE"] {
        let mut rejected = source.clone();
        let before = rejected.to_bytes().unwrap();
        assert!(
            rejected
                .update_iec_ld_element_operand(0, contact.string.offset, "스위치_1", replacement)
                .is_err(),
            "{replacement}"
        );
        assert_eq!(rejected.to_bytes().unwrap(), before, "{replacement}");
    }
    let mut input_coil = source.clone();
    let before = input_coil.to_bytes().unwrap();
    assert!(
        input_coil
            .update_iec_ld_element_operand(0, coil.string.offset, &coil.string.value, "%IX0")
            .is_err()
    );
    assert_eq!(input_coil.to_bytes().unwrap(), before);

    let mut inserted = source;
    inserted.insert_iec_ld_blank_row(0, 30).unwrap();
    let blank = inserted.to_bytes().unwrap();
    assert!(
        inserted
            .insert_iec_ld_rung(0, 31, "NO", "%MW700", "OUTPUT", "ON")
            .is_err()
    );
    assert!(
        inserted
            .insert_iec_ld_rung(0, 31, "NO", "ON", "OUTPUT", "%IX0")
            .is_err()
    );
    assert_eq!(inserted.to_bytes().unwrap(), blank);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_function_operand_edits_validate_decoded_pin_types() {
    fn site(
        document: &XgwxDocument,
        program_index: usize,
        block_name: &str,
        pin_name: &str,
        value: &str,
    ) -> usize {
        let program = document.ladder_programs()[program_index]
            .as_ref()
            .unwrap()
            .clone();
        let blocks = program.iec_function_blocks().unwrap();
        program
            .iec_function_operand_links()
            .unwrap()
            .into_iter()
            .find_map(|link| {
                let block = blocks
                    .iter()
                    .find(|block| block.record_offset == link.target_record_offset)?;
                let pin = block
                    .pins
                    .iter()
                    .chain([&block.control_input, &block.control_output])
                    .find(|pin| pin.reference_ordinal == Some(link.ordinal))?;
                let strings = extract_utf16_marker_strings(&program.data, false, false);
                let expression = strings
                    .iter()
                    .find(|string| string.offset == link.record_offset + 15)?;
                (block.name.value == block_name
                    && pin.name.value == pin_name
                    && expression.value == value)
                    .then_some(expression.offset)
            })
            .unwrap_or_else(|| panic!("captured operand site {block_name}.{pin_name}={value}"))
    }

    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let source = XgwxDocument::from_path(&path).expect("fixture parses");

    let mut numeric = source.clone();
    let offset = site(&numeric, 0, "ADD", "IN2", "1");
    numeric
        .update_iec_ld_function_operand(0, offset, "1", "123")
        .expect("numeric literal remains compatible with ANY_NUM");
    assert_eq!(
        XgwxDocument::parse(&numeric.to_bytes().unwrap())
            .unwrap()
            .ladder_programs()[0]
            .as_ref()
            .unwrap()
            .strings
            .iter()
            .find(|string| string.offset == offset)
            .unwrap()
            .value,
        "123"
    );

    let mut output_literal = source.clone();
    let offset = site(&output_literal, 0, "ADD", "OUT", "%MW700");
    let before = output_literal.to_bytes().unwrap();
    assert!(
        output_literal
            .update_iec_ld_function_operand(0, offset, "%MW700", "123")
            .is_err()
    );
    assert_eq!(output_literal.to_bytes().unwrap(), before);
    for replacement in ["%NONSENSE", "%MW", "%MW700.", "%MX700", "%IW0.0.0"] {
        assert!(
            output_literal
                .update_iec_ld_function_operand(0, offset, "%MW700", replacement)
                .is_err(),
            "{replacement}"
        );
        assert_eq!(output_literal.to_bytes().unwrap(), before, "{replacement}");
    }
    output_literal
        .update_iec_ld_function_operand(0, offset, "%MW700", "%MW701")
        .expect("writable WORD address remains valid on ADD output");

    let mut input_word = source.clone();
    let offset = site(&input_word, 0, "ADD", "IN1", "%MW700");
    input_word
        .update_iec_ld_function_operand(0, offset, "%MW700", "%IW0.0.0")
        .expect("three-part input WORD address is readable on ADD input");

    let mut timer = source.clone();
    let offset = site(&timer, 1, "TON", "PT", "T#5s");
    let before = timer.to_bytes().unwrap();
    assert!(
        timer
            .update_iec_ld_function_operand(1, offset, "T#5s", "123")
            .is_err()
    );
    assert_eq!(timer.to_bytes().unwrap(), before);
    timer
        .update_iec_ld_function_operand(1, offset, "T#5s", "T#6s")
        .expect("TIME literal remains compatible with PT");

    let mut conversion = source;
    let offset = site(&conversion, 4, "INT_TO_UDINT", "IN", "메모리값");
    let before = conversion.to_bytes().unwrap();
    assert!(
        conversion
            .update_iec_ld_function_operand(4, offset, "메모리값", "LED상태")
            .is_err()
    );
    assert_eq!(conversion.to_bytes().unwrap(), before);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_GENERATED_TYPED_OPERAND_FIXTURE and LIBXGWX_NATIVE_TYPED_OPERAND_FIXTURE"]
fn xgi_native_save_preserves_typed_timer_operand() {
    fn timer_value(document: &XgwxDocument) -> String {
        let programs = document.ladder_programs();
        let program = programs[1].as_ref().expect("curtain program parses");
        let graph = program
            .iec_circuit_graph()
            .expect("curtain circuit graph validates");
        let binding = graph
            .function_bindings
            .iter()
            .find(|binding| {
                binding.function_name == "TON"
                    && binding.pin_name == "PT"
                    && binding.expression_record_offset.is_some()
            })
            .expect("TON.PT binding");
        let record_offset = binding.expression_record_offset.unwrap();
        let record = program
            .iec_record_frames()
            .unwrap()
            .into_iter()
            .find(|record| record.offset == record_offset)
            .expect("TON.PT expression record");
        program
            .strings
            .iter()
            .find(|string| string.offset >= record.offset && string.end_offset <= record.end)
            .expect("TON.PT expression")
            .value
            .clone()
    }

    let generated = XgwxDocument::from_path(
        env::var("LIBXGWX_GENERATED_TYPED_OPERAND_FIXTURE")
            .expect("generated typed-operand fixture path"),
    )
    .expect("generated typed-operand fixture parses");
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_TYPED_OPERAND_FIXTURE")
            .expect("native typed-operand Save As fixture path"),
    )
    .expect("native typed-operand fixture parses");

    assert_eq!(timer_value(&generated), "T#6s");
    assert_eq!(timer_value(&native), "T#6s");
    let generated_programs = generated
        .ladder_programs()
        .into_iter()
        .map(|program| program.expect("generated program parses").data)
        .collect::<Vec<_>>();
    let native_programs = native
        .ladder_programs()
        .into_iter()
        .map(|program| program.expect("native program parses").data)
        .collect::<Vec<_>>();
    assert_eq!(native_programs, generated_programs);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_GENERATED_NC_INSERT_FIXTURE and LIBXGWX_NATIVE_NC_INSERT_FIXTURE"]
fn xgi_native_save_preserves_inserted_nc_contact() {
    fn contact_value(document: &XgwxDocument) -> String {
        let programs = document.ladder_programs();
        let program = programs[0].as_ref().expect("lighting program parses");
        program
            .iec_circuit_graph()
            .expect("lighting circuit graph validates");
        let record = program
            .iec_record_frames()
            .expect("IEC records frame")
            .into_iter()
            .find(|record| {
                record.offset == 271
                    && record.kind == IecRecordKind::Contact(0x07)
                    && program.data.get(record.offset + 5) == Some(&7)
            })
            .expect("inserted NC contact at record 271 and x=7");
        let strings = program
            .strings
            .iter()
            .filter(|item| item.offset >= record.offset && item.end_offset <= record.end)
            .collect::<Vec<_>>();
        assert_eq!(strings.len(), 1, "inserted contact has one operand");
        strings[0].value.clone()
    }

    let generated = XgwxDocument::from_path(
        env::var("LIBXGWX_GENERATED_NC_INSERT_FIXTURE").expect("generated NC-insert fixture path"),
    )
    .expect("generated NC-insert fixture parses");
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_NC_INSERT_FIXTURE")
            .expect("native NC-insert Save As fixture path"),
    )
    .expect("native NC-insert fixture parses");

    assert_eq!(contact_value(&generated), "ON");
    assert_eq!(contact_value(&native), "ON");
    let generated_programs = generated
        .ladder_programs()
        .into_iter()
        .map(|program| program.expect("generated program parses").data)
        .collect::<Vec<_>>();
    let native_programs = native
        .ladder_programs()
        .into_iter()
        .map(|program| program.expect("native program parses").data)
        .collect::<Vec<_>>();
    assert_eq!(native_programs, generated_programs);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_GENERATED_RISING_DELETE_FIXTURE and LIBXGWX_NATIVE_RISING_DELETE_FIXTURE"]
fn xgi_native_save_preserves_rising_contact_cell_delete() {
    let generated = XgwxDocument::from_path(
        env::var("LIBXGWX_GENERATED_RISING_DELETE_FIXTURE")
            .expect("generated rising-delete fixture path"),
    )
    .expect("generated rising-delete fixture parses");
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_RISING_DELETE_FIXTURE")
            .expect("native rising-delete Save As fixture path"),
    )
    .expect("native rising-delete fixture parses");

    for document in [&generated, &native] {
        let programs = document.ladder_programs();
        let program = programs[0].as_ref().expect("lighting program parses");
        program
            .iec_circuit_graph()
            .expect("lighting circuit graph validates");
        let rows = program.iec_row_frames().expect("IEC rows frame");
        let row = rows
            .iter()
            .find(|row| row.group_index == 2 && row.row_index == 2)
            .expect("edited lighting row");
        assert_eq!(row.record_count, 4);
        let records = program.iec_record_frames().expect("IEC records frame");
        assert!(!records.iter().any(|record| {
            record.group_index == 2
                && record.row_index == 2
                && record.kind == IecRecordKind::Contact(0x08)
                && program.data.get(record.offset + 5) == Some(&7)
        }));
        assert!(records.iter().any(|record| {
            record.group_index == 2
                && record.row_index == 2
                && record.kind == IecRecordKind::LongWire
                && program.data.get(record.offset + 5) == Some(&7)
        }));
    }

    let generated_programs = generated
        .ladder_programs()
        .into_iter()
        .map(|program| program.expect("generated program parses").data)
        .collect::<Vec<_>>();
    let native_programs = native
        .ladder_programs()
        .into_iter()
        .map(|program| program.expect("native program parses").data)
        .collect::<Vec<_>>();
    assert_eq!(native_programs, generated_programs);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE, LIBXGWX_NATIVE_BLANK_ROW_FIXTURE, and LIBXGWX_NATIVE_BLANK_ROW_DELETE_FIXTURE"]
fn xgi_blank_row_insertion_matches_native_coordinates() {
    let source_path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let mut generated = XgwxDocument::from_path(source_path).expect("source fixture parses");
    let before = generated.ladder_programs();
    let before_program = before[0].as_ref().unwrap();
    let before_rows = before_program.iec_row_frames().unwrap();
    let before_graph = before_program.iec_circuit_graph().unwrap();

    generated
        .insert_iec_ld_blank_row(0, 30)
        .expect("captured blank row inserts");
    let encoded = generated.to_bytes().unwrap();
    let generated = XgwxDocument::parse(&encoded).unwrap();
    let generated_programs = generated.ladder_programs();
    let generated_program = generated_programs[0].as_ref().unwrap();
    let generated_rows = generated_program.iec_row_frames().unwrap();
    let generated_graph = generated_program.iec_circuit_graph().unwrap();
    assert_eq!(
        u16::from_le_bytes(generated_program.data[4..6].try_into().unwrap()),
        91
    );
    assert_eq!(generated_rows.len(), before_rows.len());
    for (before, after) in before_rows.iter().zip(&generated_rows) {
        assert_eq!(
            after.row_index,
            before.row_index + u16::from(before.row_index > 30)
        );
        assert_eq!(after.group_index, before.group_index);
        assert_eq!(after.record_count, before.record_count);
    }
    assert_eq!(generated_graph.edges.len(), before_graph.edges.len());
    assert_eq!(
        generated_graph.function_bindings.len(),
        before_graph.function_bindings.len()
    );

    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_BLANK_ROW_FIXTURE").expect("native blank-row fixture path"),
    )
    .expect("native blank-row fixture parses");
    let native_programs = native.ladder_programs();
    for index in 0..generated_programs.len() {
        let generated = generated_programs[index].as_ref().unwrap();
        let native = native_programs[index].as_ref().unwrap();
        let differences = generated
            .data
            .iter()
            .zip(&native.data)
            .enumerate()
            .filter_map(|(offset, (left, right))| (left != right).then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(
            differences,
            if index == 0 {
                vec![0x0c88, 0x0f1f, 0x114a]
            } else {
                vec![]
            },
            "program {index} differences"
        );
    }

    let mut stale = XgwxDocument::parse(&encoded).unwrap();
    let unchanged = stale.to_bytes().unwrap();
    assert!(stale.insert_iec_ld_blank_row(0, 91).is_err());
    assert_eq!(stale.to_bytes().unwrap(), unchanged);

    let mut deleted = XgwxDocument::parse(&encoded).unwrap();
    deleted
        .delete_iec_ld_blank_row(0, 31)
        .expect("captured blank row deletes");
    let deleted = XgwxDocument::parse(&deleted.to_bytes().unwrap()).unwrap();
    let deleted_programs = deleted.ladder_programs();
    for index in 0..before.len() {
        let original = before[index].as_ref().unwrap();
        let deleted = deleted_programs[index].as_ref().unwrap();
        let differences = original
            .data
            .iter()
            .zip(&deleted.data)
            .enumerate()
            .filter_map(|(offset, (left, right))| (left != right).then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(
            differences,
            if index == 0 { vec![0x130c] } else { vec![] },
            "program {index} generated delete differences"
        );
    }
    let native_deleted = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_BLANK_ROW_DELETE_FIXTURE")
            .expect("native blank-row delete fixture path"),
    )
    .expect("native blank-row delete fixture parses");
    let native_deleted_programs = native_deleted.ladder_programs();
    for index in 0..deleted_programs.len() {
        let generated = deleted_programs[index].as_ref().unwrap();
        let native = native_deleted_programs[index].as_ref().unwrap();
        let differences = generated
            .data
            .iter()
            .zip(&native.data)
            .enumerate()
            .filter_map(|(offset, (left, right))| (left != right).then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(
            differences,
            if index == 0 {
                vec![0x0c88, 0x0f1f, 0x114a, 0x1389]
            } else {
                vec![]
            },
            "program {index} native delete differences"
        );
    }
    let mut occupied = XgwxDocument::parse(&encoded).unwrap();
    let unchanged = occupied.to_bytes().unwrap();
    assert!(occupied.delete_iec_ld_blank_row(0, 30).is_err());
    assert_eq!(occupied.to_bytes().unwrap(), unchanged);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_GENERATED_BLANK_ROW_FIXTURE and LIBXGWX_NATIVE_BLANK_ROW_RESAVE_FIXTURE"]
fn xgi_native_save_preserves_generated_blank_row() {
    let generated = XgwxDocument::from_path(
        env::var("LIBXGWX_GENERATED_BLANK_ROW_FIXTURE").expect("generated blank-row fixture path"),
    )
    .expect("generated blank-row fixture parses");
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_BLANK_ROW_RESAVE_FIXTURE")
            .expect("native blank-row Save As fixture path"),
    )
    .expect("native blank-row fixture parses");
    let generated_programs = generated.ladder_programs();
    let native_programs = native.ladder_programs();
    assert_eq!(generated_programs.len(), native_programs.len());
    for (generated, native) in generated_programs.into_iter().zip(native_programs) {
        assert_eq!(generated.unwrap().data, native.unwrap().data);
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_GENERATED_BLANK_ROW_DELETE_FIXTURE and LIBXGWX_NATIVE_BLANK_ROW_DELETE_RESAVE_FIXTURE"]
fn xgi_native_save_preserves_generated_blank_row_delete() {
    let generated = XgwxDocument::from_path(
        env::var("LIBXGWX_GENERATED_BLANK_ROW_DELETE_FIXTURE")
            .expect("generated blank-row delete fixture path"),
    )
    .expect("generated blank-row delete fixture parses");
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_BLANK_ROW_DELETE_RESAVE_FIXTURE")
            .expect("native blank-row delete Save As fixture path"),
    )
    .expect("native blank-row delete fixture parses");
    let generated_programs = generated.ladder_programs();
    let native_programs = native.ladder_programs();
    assert_eq!(generated_programs.len(), native_programs.len());
    for (generated, native) in generated_programs.into_iter().zip(native_programs) {
        assert_eq!(generated.unwrap().data, native.unwrap().data);
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_append_rung_preserves_existing_programs_and_supports_repeated_edits() {
    let source = XgwxDocument::from_path(
        env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path"),
    )
    .unwrap();
    let original_bytes = source.to_bytes().unwrap();
    let original_programs = source.ladder_programs();
    for program_index in 0..original_programs.len() {
        let before = original_programs[program_index].as_ref().unwrap();
        let row_index = before.iec_row_frames().unwrap().last().unwrap().row_index + 1;
        let mut edited = source.clone();
        edited
            .insert_iec_ld_rung(program_index, row_index, "NC", "%MX1000", "SET", "%MX1001")
            .expect("new rung appends after final network");
        let reparsed = XgwxDocument::parse(&edited.to_bytes().unwrap()).unwrap();
        let programs = reparsed.ladder_programs();
        for (index, program) in programs.iter().enumerate() {
            let program = program.as_ref().unwrap();
            if index != program_index {
                assert_eq!(
                    program.data,
                    original_programs[index].as_ref().unwrap().data
                );
                continue;
            }
            assert_eq!(&program.data[..4], &before.data[..4]);
            assert_eq!(
                u16::from_le_bytes(program.data[4..6].try_into().unwrap()),
                row_index + 1
            );
            assert_eq!(&program.data[8..before.data.len()], &before.data[8..]);
            let appended = program.iec_row_frames().unwrap().pop().unwrap();
            assert_eq!(appended.row_index, row_index);
            assert_eq!(appended.record_count, 3);
            assert!(
                program
                    .iec_circuit_graph()
                    .unwrap()
                    .power_components
                    .iter()
                    .any(|component| {
                        component.group_index == appended.group_index
                            && component.touches_left_rail
                            && component.touches_right_rail
                    })
            );
        }
        edited
            .insert_iec_ld_rung(
                program_index,
                row_index + 1,
                "NO",
                "%MX1002",
                "OUTPUT",
                "%MX1003",
            )
            .expect("a second rung appends to the edited program");
        edited
            .delete_iec_ld_rung(
                program_index,
                row_index + 1,
                "NO",
                "%MX1002",
                "OUTPUT",
                "%MX1003",
            )
            .unwrap();
        edited
            .delete_iec_ld_rung(program_index, row_index, "NC", "%MX1000", "SET", "%MX1001")
            .unwrap();
        let restored = edited.ladder_programs().remove(program_index).unwrap();
        assert_eq!(&restored.data[..4], &before.data[..4]);
        assert_eq!(&restored.data[6..], &before.data[6..]);
        let unchanged = edited.to_bytes().unwrap();
        assert!(
            edited
                .insert_iec_ld_rung(
                    program_index,
                    u16::MAX / 4,
                    "NO",
                    "%MX1000",
                    "OUTPUT",
                    "%MX1001"
                )
                .is_err()
        );
        assert_eq!(edited.to_bytes().unwrap(), unchanged);
    }
    assert_eq!(source.to_bytes().unwrap(), original_bytes);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_GENERATED_BLANK_ROW_FIXTURE and LIBXGWX_NATIVE_LINEAR_RUNG_FIXTURE"]
fn xgi_linear_rung_creation_matches_native_group() {
    let source_path =
        env::var("LIBXGWX_GENERATED_BLANK_ROW_FIXTURE").expect("generated blank-row fixture path");
    let mut generated = XgwxDocument::from_path(source_path).expect("source fixture parses");
    let source_bytes = generated.to_bytes().unwrap();
    let before = generated.ladder_programs();
    let before_program = before[0].as_ref().unwrap();
    let before_rows = before_program.iec_row_frames().unwrap();
    let before_records = before_program.iec_record_frames().unwrap();
    let before_graph = before_program.iec_circuit_graph().unwrap();

    generated
        .insert_iec_ld_linear_rung(0, 31, "ON", "OFF")
        .expect("captured linear rung inserts");
    let encoded = generated.to_bytes().unwrap();
    let generated = XgwxDocument::parse(&encoded).unwrap();
    let generated_programs = generated.ladder_programs();
    let program = generated_programs[0].as_ref().unwrap();
    let rows = program.iec_row_frames().unwrap();
    let records = program.iec_record_frames().unwrap();
    let graph = program.iec_circuit_graph().unwrap();
    let row = rows
        .iter()
        .find(|row| row.group_index == 17 && row.row_index == 31)
        .expect("created row");
    assert_eq!(rows.len(), before_rows.len() + 1);
    assert_eq!(records.len(), before_records.len() + 3);
    assert_eq!(row.record_count, 3);
    assert_eq!(
        records
            .iter()
            .filter(|record| record.group_index == 17 && record.row_index == 31)
            .map(|record| record.kind)
            .collect::<Vec<_>>(),
        [
            IecRecordKind::Contact(0x06),
            IecRecordKind::LongWire,
            IecRecordKind::Coil(0x0e),
        ]
    );
    assert_eq!(graph.edges.len(), before_graph.edges.len() + 3);
    assert!(graph.power_components.iter().any(|component| {
        component.group_index == 17 && component.touches_left_rail && component.touches_right_rail
    }));
    let created_operands = crate::iec_ld::element_operands(program)
        .into_iter()
        .filter(|operand| operand.raw_y == 31 * 4)
        .collect::<Vec<_>>();
    let contact_offset = created_operands
        .iter()
        .find(|operand| operand.record_code == 0x06)
        .unwrap()
        .string
        .offset;
    let coil_offset = created_operands
        .iter()
        .find(|operand| operand.record_code == 0x0e)
        .unwrap()
        .string
        .offset;
    for (contact_kind, coil_kind) in [
        ("NC", "INVERSE"),
        ("RISING", "RISING"),
        ("FALLING", "SET"),
        ("NEGATED_RISING", "FALLING"),
        ("NEGATED_FALLING", "RESET"),
    ] {
        let mut changed = XgwxDocument::parse(&encoded).unwrap();
        changed
            .update_iec_ld_contact_kind(0, contact_offset, "NO", contact_kind)
            .unwrap();
        changed
            .update_iec_ld_coil_kind(0, coil_offset, "OUTPUT", coil_kind)
            .unwrap();

        let mut directly_created = XgwxDocument::parse(&source_bytes).unwrap();
        directly_created
            .insert_iec_ld_rung(0, 31, contact_kind, "ON", coil_kind, "OFF")
            .unwrap();
        assert_eq!(
            changed
                .ladder_programs()
                .into_iter()
                .map(|program| program.unwrap().data)
                .collect::<Vec<_>>(),
            directly_created
                .ladder_programs()
                .into_iter()
                .map(|program| program.unwrap().data)
                .collect::<Vec<_>>(),
            "kind update matches direct {contact_kind} to {coil_kind} creation",
        );
        changed
            .update_iec_ld_contact_kind(0, contact_offset, contact_kind, "NO")
            .unwrap();
        changed
            .update_iec_ld_coil_kind(0, coil_offset, coil_kind, "OUTPUT")
            .unwrap();
        assert_eq!(changed.to_bytes().unwrap(), encoded);
    }
    for index in 1..generated_programs.len() {
        assert_eq!(
            generated_programs[index].as_ref().unwrap().data,
            before[index].as_ref().unwrap().data
        );
    }

    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_LINEAR_RUNG_FIXTURE").expect("native linear-rung fixture path"),
    )
    .expect("native linear-rung fixture parses");
    for (index, (generated, native)) in generated_programs
        .iter()
        .zip(native.ladder_programs())
        .enumerate()
    {
        let generated = generated.as_ref().unwrap();
        let native = native.unwrap();
        let differences = generated
            .data
            .iter()
            .zip(&native.data)
            .enumerate()
            .filter_map(|(offset, (left, right))| (left != right).then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(
            differences,
            if index == 0 {
                vec![0x0c88, 0x0f1f, 0x114a, 0x13f9]
            } else {
                vec![]
            },
            "program {index} differences"
        );
    }

    let mut stale = XgwxDocument::parse(&encoded).unwrap();
    let unchanged = stale.to_bytes().unwrap();
    assert!(stale.insert_iec_ld_linear_rung(0, 31, "ON", "OFF").is_err());
    assert!(stale.insert_iec_ld_linear_rung(0, 30, "ON", "OFF").is_err());
    assert!(
        stale
            .insert_iec_ld_linear_rung(0, 66, "WW700", "OFF")
            .is_err()
    );
    assert_eq!(stale.to_bytes().unwrap(), unchanged);

    let mut restored = XgwxDocument::parse(&encoded).unwrap();
    restored
        .delete_iec_ld_linear_rung(0, 31, "ON", "OFF")
        .expect("captured linear rung deletes");
    assert_eq!(restored.to_bytes().unwrap(), source_bytes);
    let mut stale_delete = XgwxDocument::parse(&encoded).unwrap();
    let unchanged = stale_delete.to_bytes().unwrap();
    assert!(
        stale_delete
            .delete_iec_ld_linear_rung(0, 31, "STALE", "OFF")
            .is_err()
    );
    assert_eq!(stale_delete.to_bytes().unwrap(), unchanged);

    for (contact_kind, contact_code) in [
        ("NO", 0x06),
        ("NC", 0x07),
        ("RISING", 0x08),
        ("FALLING", 0x09),
        ("NEGATED_RISING", 0x0a),
        ("NEGATED_FALLING", 0x0b),
    ] {
        for (coil_kind, coil_code) in [
            ("OUTPUT", 0x0e),
            ("INVERSE", 0x0f),
            ("SET", 0x10),
            ("RESET", 0x11),
            ("RISING", 0x12),
            ("FALLING", 0x13),
        ] {
            let mut variant = XgwxDocument::parse(&source_bytes).unwrap();
            variant
                .insert_iec_ld_rung(0, 31, contact_kind, "ON", coil_kind, "OFF")
                .unwrap();
            let variant_bytes = variant.to_bytes().unwrap();
            let variant_program = XgwxDocument::parse(&variant_bytes)
                .unwrap()
                .ladder_programs()
                .remove(0)
                .unwrap();
            let kinds = variant_program
                .iec_record_frames()
                .unwrap()
                .into_iter()
                .filter(|record| record.group_index == 17 && record.row_index == 31)
                .map(|record| record.kind)
                .collect::<Vec<_>>();
            assert_eq!(
                kinds,
                [
                    IecRecordKind::Contact(contact_code),
                    IecRecordKind::LongWire,
                    IecRecordKind::Coil(coil_code),
                ],
                "{contact_kind} to {coil_kind}",
            );
            let mut restored = XgwxDocument::parse(&variant_bytes).unwrap();
            restored
                .delete_iec_ld_rung(0, 31, contact_kind, "ON", coil_kind, "OFF")
                .unwrap();
            assert_eq!(
                restored.to_bytes().unwrap(),
                source_bytes,
                "{contact_kind} to {coil_kind} inverse",
            );
        }
    }
    let mut invalid_kind = XgwxDocument::parse(&source_bytes).unwrap();
    let unchanged = invalid_kind.to_bytes().unwrap();
    assert!(
        invalid_kind
            .insert_iec_ld_rung(0, 31, "INVALID", "ON", "OUTPUT", "OFF")
            .is_err()
    );
    assert_eq!(invalid_kind.to_bytes().unwrap(), unchanged);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_GENERATED_LINEAR_RUNG_FIXTURE and LIBXGWX_NATIVE_LINEAR_RUNG_RESAVE_FIXTURE"]
fn xgi_native_save_preserves_generated_linear_rung() {
    let generated = XgwxDocument::from_path(
        env::var("LIBXGWX_GENERATED_LINEAR_RUNG_FIXTURE")
            .expect("generated linear-rung fixture path"),
    )
    .expect("generated linear-rung fixture parses");
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_LINEAR_RUNG_RESAVE_FIXTURE")
            .expect("native linear-rung Save As fixture path"),
    )
    .expect("native linear-rung fixture parses");
    let generated_programs = generated.ladder_programs();
    let native_programs = native.ladder_programs();
    assert_eq!(generated_programs.len(), native_programs.len());
    for (generated, native) in generated_programs.into_iter().zip(native_programs) {
        assert_eq!(generated.unwrap().data, native.unwrap().data);
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_NATIVE_RUNG_KIND_CAPTURE_DIR to the captured generated/native-resaved pair directory"]
fn xgi_native_save_preserves_generated_rung_kinds() {
    let directory = std::path::PathBuf::from(
        env::var("LIBXGWX_NATIVE_RUNG_KIND_CAPTURE_DIR")
            .expect("native rung-kind capture directory"),
    );
    for stem in [
        "falling_set",
        "nc_inverse",
        "rising_rising",
        "negated_rising_falling",
        "negated_falling_reset",
    ] {
        let generated =
            XgwxDocument::from_path(directory.join(format!("generated_{stem}_l31.xgwx")))
                .expect("generated rung-kind fixture parses");
        let native =
            XgwxDocument::from_path(directory.join(format!("native_resaved_{stem}_l31.xgwx")))
                .expect("native-resaved rung-kind fixture parses");
        let generated_programs = generated.ladder_programs();
        let native_programs = native.ladder_programs();
        assert_eq!(generated_programs.len(), native_programs.len(), "{stem}");
        for (index, (generated, native)) in generated_programs
            .into_iter()
            .zip(native_programs)
            .enumerate()
        {
            assert_eq!(
                generated.unwrap().data,
                native.unwrap().data,
                "{stem} program {index}",
            );
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_local_address_updates_text_and_binary_number() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let mut doc = XgwxDocument::from_path(path).expect("fixture parses");
    let before_programs = doc
        .ladder_programs()
        .into_iter()
        .map(|program| program.expect("program parses").data)
        .collect::<Vec<_>>();
    doc.update_iec_local_symbol_address(0, 5, "ON", "%MX8", "%MX9")
        .expect("mapped address edit");
    let encoded = doc.to_bytes().expect("serialize");
    let edited = XgwxDocument::parse(&encoded).expect("round trip");
    assert_eq!(
        edited.iec_local_symbols()[0].as_ref().unwrap()[5]
            .address
            .as_deref(),
        Some("%MX9")
    );
    let after_programs = edited
        .ladder_programs()
        .into_iter()
        .map(|program| program.expect("program parses").data)
        .collect::<Vec<_>>();
    assert_eq!(before_programs, after_programs);

    let xml = roxmltree::Document::parse(&edited.xml).expect("XML parses");
    let table = xml
        .descendants()
        .find(|node| node.has_tag_name("Program"))
        .and_then(|program| {
            program
                .descendants()
                .find(|node| node.has_tag_name("Symbols"))
        })
        .expect("local symbols");
    let data = decode_base64_payload(table.text().unwrap_or_default(), true)
        .expect("symbols decode")
        .data;
    let strings = extract_utf16_marker_strings(&data, false, true);
    let starts = strings
        .iter()
        .enumerate()
        .filter_map(|(index, field)| (field.value == "PB50").then_some(index))
        .collect::<Vec<_>>();
    let record = &strings[starts[5]..starts[6]];
    assert_eq!(record[2].value, "%MX9");
    let numeric_offset = record[record.len() - 3].end_offset;
    assert_eq!(
        &data[numeric_offset..numeric_offset + 4],
        &9u32.to_le_bytes()
    );

    let mut longer = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap())
        .expect("fixture parses");
    longer
        .update_iec_local_symbol_address(0, 5, "ON", "%MX8", "%MX100")
        .expect("variable-length address edit");
    let encoded = longer.to_bytes().expect("serialize longer address");
    let mut restored = XgwxDocument::parse(&encoded).expect("parse longer address");
    assert_eq!(
        restored.iec_local_symbols()[0].as_ref().unwrap()[5]
            .address
            .as_deref(),
        Some("%MX100")
    );
    restored
        .update_iec_local_symbol_address(0, 5, "ON", "%MX100", "%MX8")
        .expect("restore shorter address");
    assert_eq!(
        restored.iec_local_symbols()[0].as_ref().unwrap()[5]
            .address
            .as_deref(),
        Some("%MX8")
    );

    let mut unmapped = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap())
        .expect("fixture parses");
    unmapped
        .update_iec_local_symbol_address(0, 5, "ON", "%MX8", "")
        .expect("clear mapped BOOL address");
    let generated_symbols = unmapped.iec_local_symbols();
    let generated_on = &generated_symbols[0].as_ref().unwrap()[5];
    assert_eq!(generated_on.address, None);
    assert_eq!(generated_on.storage_class, "");
    assert_eq!(generated_on.allocation_number, None);
    assert_eq!(generated_on.allocation_width, None);
    if let Ok(native_path) = env::var("LIBXGWX_NATIVE_UNMAP_FIXTURE") {
        let native = XgwxDocument::from_path(native_path).expect("native unmap capture parses");
        let native_symbols = native.iec_local_symbols();
        let native_on = &native_symbols[0].as_ref().unwrap()[5];
        assert_eq!(generated_on.address, native_on.address);
        assert_eq!(generated_on.storage_class, native_on.storage_class);
        assert_eq!(generated_on.allocation_number, native_on.allocation_number);
        assert_eq!(generated_on.allocation_width, native_on.allocation_width);
        assert_eq!(generated_on.data_type, native_on.data_type);
    }
    unmapped
        .update_iec_local_symbol_address(0, 5, "ON", "", "%MX8")
        .expect("restore mapped BOOL address");
    assert_eq!(
        unmapped.iec_local_symbols()[0].as_ref().unwrap()[5],
        XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap())
            .unwrap()
            .iec_local_symbols()[0]
            .as_ref()
            .unwrap()[5]
    );
    let mut retyped =
        XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    retyped
        .update_iec_local_symbol_address(0, 5, "ON", "%MX8", "")
        .unwrap();
    retyped
        .update_iec_local_symbol_type(0, 5, "ON", "BOOL", "WORD")
        .expect("cleared mapping permits primitive type change");

    let mut dotted = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap())
        .expect("fixture parses");
    dotted
        .update_iec_local_symbol_address(4, 3, "LED상태", "%QX0.1.1", "%QX0.1.5")
        .expect("captured dotted address edit");
    let xml = roxmltree::Document::parse(&dotted.xml).expect("XML parses");
    let table = xml
        .descendants()
        .filter(|node| node.has_tag_name("Program"))
        .nth(4)
        .unwrap()
        .descendants()
        .find(|node| node.has_tag_name("Symbols"))
        .unwrap();
    let data = decode_base64_payload(table.text().unwrap_or_default(), true)
        .unwrap()
        .data;
    let strings = extract_utf16_marker_strings(&data, false, true);
    let starts = strings
        .iter()
        .enumerate()
        .filter_map(|(index, field)| (field.value == "PB50").then_some(index))
        .collect::<Vec<_>>();
    let record = &strings[starts[3]..starts[4]];
    assert_eq!(record[2].value, "%QX0.1.5");
    let numeric_offset = record[record.len() - 3].end_offset;
    assert_eq!(
        &data[numeric_offset..numeric_offset + 4],
        &69u32.to_le_bytes()
    );
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_all_mapped_bool_addresses_clear_and_restore() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let original = XgwxDocument::from_path(&path).expect("fixture parses");
    let original_symbols = original
        .iec_local_symbols()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("symbol tables parse");
    let original_programs = original
        .ladder_programs()
        .into_iter()
        .map(|program| program.expect("program parses").data)
        .collect::<Vec<_>>();
    let mut checked = 0;
    for (program_index, symbols) in original_symbols.iter().enumerate() {
        for (symbol_index, symbol) in symbols.iter().enumerate() {
            let Some(address) = symbol.address.as_deref() else {
                continue;
            };
            if symbol.data_type.as_deref() != Some("BOOL")
                || !["%MX", "%IX", "%QX"]
                    .iter()
                    .any(|area| address.starts_with(area))
            {
                continue;
            }
            let mut edited = XgwxDocument::from_path(&path).expect("fixture parses");
            edited
                .update_iec_local_symbol_address(
                    program_index,
                    symbol_index,
                    &symbol.name,
                    address,
                    "",
                )
                .unwrap_or_else(|error| panic!("cannot clear {} {address}: {error}", symbol.name));
            let cleared = XgwxDocument::parse(&edited.to_bytes().expect("serialize clear"))
                .expect("clear reparses");
            let cleared_symbols = cleared.iec_local_symbols();
            let cleared_symbol = &cleared_symbols[program_index].as_ref().unwrap()[symbol_index];
            assert_eq!(cleared_symbol.address, None, "{}", symbol.name);
            assert_eq!(cleared_symbol.storage_class, "", "{}", symbol.name);
            assert_eq!(cleared_symbol.allocation_number, None, "{}", symbol.name);
            assert_eq!(cleared_symbol.allocation_width, None, "{}", symbol.name);
            let mut restored = cleared;
            restored
                .update_iec_local_symbol_address(
                    program_index,
                    symbol_index,
                    &symbol.name,
                    "",
                    address,
                )
                .unwrap_or_else(|error| {
                    panic!("cannot restore {} {address}: {error}", symbol.name)
                });
            let restored = XgwxDocument::parse(&restored.to_bytes().expect("serialize restore"))
                .expect("restore reparses");
            assert_eq!(
                restored
                    .iec_local_symbols()
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()
                    .expect("restored tables parse"),
                original_symbols,
                "{} {address}",
                symbol.name
            );
            assert_eq!(
                restored
                    .ladder_programs()
                    .into_iter()
                    .map(|program| program.expect("restored program parses").data)
                    .collect::<Vec<_>>(),
                original_programs,
                "{} {address}",
                symbol.name
            );
            checked += 1;
        }
    }
    assert!(checked > 1, "expected multiple mapped BOOL symbols");
    println!("checked {checked} mapped BOOL symbols across seven IEC programs");
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_short_wire_delete_and_repair_restore_program_data() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let original = XgwxDocument::from_path(&path).expect("fixture parses");
    let original_programs = original
        .ladder_programs()
        .into_iter()
        .map(|program| program.expect("program parses").data)
        .collect::<Vec<_>>();
    let expected = [
        (0, 0x1fd1, 7),
        (0, 0x2340, 7),
        (1, 0x448, 10),
        (2, 0x180e, 10),
    ];
    let programs = original.ladder_programs();
    let sites = programs
        .iter()
        .enumerate()
        .flat_map(|(program_index, program)| {
            program
                .as_ref()
                .unwrap()
                .iec_horizontal_wire_deletion_sites()
                .expect("wire sites parse")
                .into_iter()
                .map(move |site| (program_index, site.wire_offset, site.raw_x))
        })
        .collect::<Vec<_>>();
    assert_eq!(sites, expected);
    for (program_index, wire_offset, raw_x) in expected {
        let mut edited = XgwxDocument::from_path(&path).expect("fixture parses");
        assert!(
            edited
                .delete_iec_ld_horizontal_wire(program_index, wire_offset, raw_x + 1)
                .is_err()
        );
        edited
            .delete_iec_ld_horizontal_wire(program_index, wire_offset, raw_x)
            .expect("captured short wire deletes");
        let deleted = XgwxDocument::parse(&edited.to_bytes().unwrap()).expect("deletion reparses");
        assert!(
            deleted.ladder_programs()[program_index]
                .as_ref()
                .unwrap()
                .iec_horizontal_wire_repair_sites()
                .unwrap()
                .iter()
                .any(|site| site.insertion_offset == wire_offset && site.raw_x == raw_x)
        );
        let mut repaired = deleted;
        repaired
            .repair_iec_ld_horizontal_wire(program_index, wire_offset, raw_x)
            .expect("short wire repairs");
        assert_eq!(
            repaired
                .ladder_programs()
                .into_iter()
                .map(|program| program.expect("program parses").data)
                .collect::<Vec<_>>(),
            original_programs
        );
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_local_rename_updates_all_captured_references() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let mut doc = XgwxDocument::from_path(path).expect("fixture parses");
    let before = doc.to_bytes().expect("original bytes");
    assert!(doc.rename_iec_local_symbol(0, 5, "ON", "OFF").is_err());
    assert_eq!(doc.to_bytes().unwrap(), before);
    doc.rename_iec_local_symbol(0, 5, "ON", "ON2")
        .expect("rename known references");
    let saved = XgwxDocument::parse(&doc.to_bytes().unwrap()).expect("round trip");
    assert_eq!(
        saved.iec_local_symbols()[0].as_ref().unwrap()[5].name,
        "ON2"
    );
    let programs = saved.ladder_programs();
    let strings = &programs[0].as_ref().unwrap().strings;
    assert_eq!(strings.iter().filter(|item| item.value == "ON2").count(), 6);
    assert_eq!(strings.iter().filter(|item| item.value == "ON").count(), 0);

    let mut unicode = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap())
        .expect("fixture parses");
    unicode
        .rename_iec_local_symbol(0, 12, "조명_1", "조명_1A")
        .expect("rename Unicode references");
    let programs = unicode.ladder_programs();
    let strings = extract_utf16_marker_strings(&programs[0].as_ref().unwrap().data, false, false);
    assert_eq!(
        strings
            .iter()
            .filter(|item| item.value == "조명_1A")
            .count(),
        8
    );
    assert_eq!(
        strings.iter().filter(|item| item.value == "조명_1").count(),
        0
    );
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_contact_kind_changes_cover_all_captured_contact_codes() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let original = XgwxDocument::from_path(&path).expect("fixture parses");
    let original_programs = original
        .ladder_programs()
        .into_iter()
        .map(|program| program.unwrap().data)
        .collect::<Vec<_>>();

    let mut changed = XgwxDocument::from_path(&path).unwrap();
    changed
        .update_iec_ld_contact_kind(0, 238, "RISING", "NEGATED_RISING")
        .unwrap();
    let changed_programs = changed.ladder_programs();
    assert_eq!(changed_programs[0].as_ref().unwrap().data[224], 0x0a);
    for index in 1..changed_programs.len() {
        assert_eq!(
            changed_programs[index].as_ref().unwrap().data,
            original_programs[index]
        );
    }
    assert!(
        changed
            .update_iec_ld_contact_kind(0, 238, "RISING", "NO")
            .is_err()
    );
    changed
        .update_iec_ld_contact_kind(0, 238, "NEGATED_RISING", "RISING")
        .unwrap();
    assert_eq!(
        changed.ladder_programs()[0].as_ref().unwrap().data,
        original_programs[0]
    );

    let mut negated = XgwxDocument::from_path(path).unwrap();
    negated
        .update_iec_ld_contact_kind(6, 555, "NEGATED_RISING", "NO")
        .unwrap();
    assert_eq!(
        negated.ladder_programs()[6].as_ref().unwrap().data[541],
        0x06
    );
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_branch_segment_remove_and_restore_is_byte_exact() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let mut document = XgwxDocument::from_path(path).expect("fixture parses");
    let original = document
        .ladder_programs()
        .into_iter()
        .map(|program| program.unwrap().data)
        .collect::<Vec<_>>();

    document
        .edit_iec_ld_branch_segment(6, 3, 3, 4, 6, true, false)
        .expect("remove middle parallel branch segment");
    let removed = document.ladder_programs();
    assert_eq!(
        removed[6]
            .as_ref()
            .unwrap()
            .iec_geometry()
            .unwrap()
            .vertical
            .len(),
        85
    );
    assert!(
        removed[6]
            .as_ref()
            .unwrap()
            .iec_geometry()
            .unwrap()
            .vertical
            .iter()
            .all(|connection| !(connection.group_index == 3
                && connection.start_row_index == 3
                && connection.end_row_index == 4
                && connection.x == 6))
    );
    for index in 0..6 {
        assert_eq!(removed[index].as_ref().unwrap().data, original[index]);
    }

    let removed_bytes = document.to_bytes().unwrap();
    let mut restored = XgwxDocument::parse(&removed_bytes).expect("removed file reparses");
    assert!(
        restored
            .edit_iec_ld_branch_segment(6, 3, 3, 4, 6, true, true)
            .is_err()
    );
    restored
        .edit_iec_ld_branch_segment(6, 3, 3, 4, 6, false, true)
        .expect("restore middle parallel branch segment");
    let restored_programs = restored.ladder_programs();
    for index in 0..restored_programs.len() {
        assert_eq!(
            restored_programs[index].as_ref().unwrap().data,
            original[index]
        );
    }

    restored
        .edit_iec_ld_branch_segment(0, 3, 3, 4, 6, true, false)
        .expect("remove captured final branch row");
    let final_removed = restored.ladder_programs();
    let changed = final_removed[0].as_ref().unwrap();
    assert_eq!(changed.iec_row_frames().unwrap().len(), 85);
    assert_eq!(changed.iec_record_frames().unwrap().len(), 286);
    assert_eq!(changed.iec_geometry().unwrap().vertical.len(), 26);
    for index in 1..7 {
        assert_eq!(final_removed[index].as_ref().unwrap().data, original[index]);
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_GENERATED_BRANCH_SEGMENT_FIXTURE and LIBXGWX_NATIVE_BRANCH_SEGMENT_FIXTURE"]
fn xgi_native_save_preserves_generated_branch_segment_removal() {
    let generated = XgwxDocument::from_path(
        env::var("LIBXGWX_GENERATED_BRANCH_SEGMENT_FIXTURE")
            .expect("generated branch-segment fixture path"),
    )
    .expect("generated fixture parses");
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_BRANCH_SEGMENT_FIXTURE")
            .expect("native branch-segment fixture path"),
    )
    .expect("native Save As fixture parses");
    let generated_programs = generated.ladder_programs();
    let native_programs = native.ladder_programs();
    assert_eq!(generated_programs.len(), 7);
    assert_eq!(native_programs.len(), generated_programs.len());
    for (index, (generated, native)) in generated_programs
        .iter()
        .zip(native_programs.iter())
        .enumerate()
    {
        assert_eq!(
            generated.as_ref().unwrap().data,
            native.as_ref().unwrap().data,
            "program {index}"
        );
    }
    let geometry = native_programs[6].as_ref().unwrap().iec_geometry().unwrap();
    assert_eq!(geometry.vertical.len(), 85);
    assert!(geometry.vertical.iter().all(|connection| {
        !(connection.group_index == 3
            && connection.start_row_index == 3
            && connection.end_row_index == 4
            && connection.x == 6)
    }));
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_NATIVE_FINAL_BRANCH_FIXTURE"]
fn xgi_final_branch_row_removal_matches_native_structure() {
    let source =
        std::fs::read(env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path"))
            .expect("read smart home fixture");
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_FINAL_BRANCH_FIXTURE").expect("native final-branch fixture path"),
    )
    .expect("native final-branch fixture parses");
    let original = XgwxDocument::parse(&source).expect("fixture parses");
    let original_programs = original.ladder_programs();
    let mut generated = XgwxDocument::parse(&source).expect("fixture reparses");
    generated
        .edit_iec_ld_branch_segment(0, 3, 3, 4, 6, true, false)
        .expect("remove captured final branch row");
    let generated_programs = generated.ladder_programs();
    let native_programs = native.ladder_programs();
    assert_eq!(generated_programs.len(), 7);
    assert_eq!(native_programs.len(), 7);
    let generated_program = generated_programs[0].as_ref().unwrap();
    let native_program = native_programs[0].as_ref().unwrap();
    let generated_rows = generated_program.iec_row_frames().unwrap();
    let opaque_row_cache_offsets = generated_rows
        .iter()
        .filter(|row| matches!(row.row_index, 21 | 25 | 27))
        .map(|row| row.start + 17)
        .collect::<Vec<_>>();
    let differences = generated_program
        .data
        .iter()
        .zip(&native_program.data)
        .enumerate()
        .filter_map(|(offset, (generated, native))| (generated != native).then_some(offset))
        .collect::<Vec<_>>();
    assert_eq!(differences, opaque_row_cache_offsets);
    for index in 1..7 {
        assert_eq!(
            generated_programs[index].as_ref().unwrap().data,
            original_programs[index].as_ref().unwrap().data,
        );
        assert_eq!(
            native_programs[index].as_ref().unwrap().data,
            original_programs[index].as_ref().unwrap().data,
        );
    }
    let changed = generated_program;
    assert_eq!(changed.iec_row_frames().unwrap().len(), 85);
    assert_eq!(changed.iec_record_frames().unwrap().len(), 286);
    assert_eq!(changed.iec_geometry().unwrap().vertical.len(), 26);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_GENERATED_CONTACT_KIND_FIXTURE and LIBXGWX_NATIVE_CONTACT_KIND_FIXTURE"]
fn xgi_native_save_preserves_generated_contact_kind() {
    let generated = XgwxDocument::from_path(
        env::var("LIBXGWX_GENERATED_CONTACT_KIND_FIXTURE")
            .expect("generated contact-kind fixture path"),
    )
    .expect("generated fixture parses");
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_CONTACT_KIND_FIXTURE").expect("native contact-kind fixture path"),
    )
    .expect("native Save As fixture parses");

    let generated_programs = generated.ladder_programs();
    let native_programs = native.ladder_programs();
    assert_eq!(generated_programs.len(), native_programs.len());
    for (index, (generated, native)) in generated_programs
        .into_iter()
        .zip(native_programs.iter())
        .enumerate()
    {
        assert_eq!(
            generated.unwrap().data,
            native.as_ref().unwrap().data,
            "program {index}"
        );
    }
    assert_eq!(native_programs[0].as_ref().unwrap().data[224], 0x0a);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_local_instance_rename_updates_function_header() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let mut doc = XgwxDocument::from_path(path).expect("fixture parses");
    let original = doc.to_bytes().expect("original bytes");
    assert!(
        doc.rename_iec_local_symbol(0, 1, "INST3", "INST_사본1")
            .is_err()
    );
    assert_eq!(doc.to_bytes().unwrap(), original);
    doc.rename_iec_local_symbol(0, 1, "INST3", "INST4")
        .expect("rename classified function instance");
    let saved = XgwxDocument::parse(&doc.to_bytes().unwrap()).expect("round trip");
    let symbols = saved.iec_local_symbols();
    assert_eq!(symbols[0].as_ref().unwrap()[1].name, "INST4");
    assert_eq!(
        symbols[0].as_ref().unwrap()[1].type_reference.as_deref(),
        Some("R_TRIG")
    );
    let programs = saved.ladder_programs();
    let renamed = programs[0]
        .as_ref()
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .filter(|block| {
            block
                .instance
                .as_ref()
                .is_some_and(|item| item.value == "INST4")
        })
        .collect::<Vec<_>>();
    assert_eq!(renamed.len(), 1);
    assert_eq!(renamed[0].name.value, "R_TRIG");
    assert!(
        programs[0]
            .as_ref()
            .unwrap()
            .iec_function_blocks()
            .unwrap()
            .iter()
            .all(|block| block
                .instance
                .as_ref()
                .is_none_or(|item| item.value != "INST3"))
    );

    let mut same_name = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap())
        .expect("fixture parses");
    same_name
        .rename_iec_local_symbol(0, 0, "FF", "FF_INST")
        .expect("rename instance without changing function type");
    let saved = XgwxDocument::parse(&same_name.to_bytes().unwrap()).expect("round trip");
    assert_eq!(
        saved.iec_local_symbols()[0].as_ref().unwrap()[0].name,
        "FF_INST"
    );
    let block = saved.ladder_programs()[0]
        .as_ref()
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|block| {
            block
                .instance
                .as_ref()
                .is_some_and(|item| item.value == "FF_INST")
        })
        .expect("renamed FF instance");
    assert_eq!(block.name.value, "FF");
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_NATIVE_INSTANCE_FIXTURE to an XG5000 Save As capture"]
fn xgi_native_instance_save_preserves_rename() {
    let path = env::var("LIBXGWX_NATIVE_INSTANCE_FIXTURE").expect("native Save As fixture path");
    let doc = XgwxDocument::from_path(path).expect("native project parses");
    let generated = XgwxDocument::from_path(
        env::var("LIBXGWX_GENERATED_INSTANCE_FIXTURE").expect("generated fixture path"),
    )
    .expect("generated project parses");
    assert_eq!(doc.programs().len(), 7);
    for (native, generated) in doc
        .ladder_programs()
        .into_iter()
        .zip(generated.ladder_programs())
    {
        assert_eq!(native.unwrap().data, generated.unwrap().data);
    }
    let symbols = doc.iec_local_symbols();
    let instance = &symbols[0].as_ref().unwrap()[1];
    assert_eq!(instance.name, "INST4");
    assert_eq!(instance.type_reference.as_deref(), Some("R_TRIG"));
    let blocks = doc.ladder_programs()[0]
        .as_ref()
        .unwrap()
        .iec_function_blocks()
        .unwrap();
    assert_eq!(
        blocks
            .iter()
            .filter(|block| block
                .instance
                .as_ref()
                .is_some_and(|item| item.value == "INST4"))
            .count(),
        1
    );
    assert!(blocks.iter().all(|block| {
        block
            .instance
            .as_ref()
            .is_none_or(|item| item.value != "INST3")
    }));
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_local_symbol_description_round_trips() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let mut doc = XgwxDocument::from_path(path).expect("fixture parses");
    let before = doc
        .ladder_programs()
        .into_iter()
        .map(|program| program.unwrap().data)
        .collect::<Vec<_>>();
    let original = doc.to_bytes().unwrap();
    assert!(
        doc.update_iec_local_symbol_description(0, 5, "ON", "stale", "Living room switch")
            .is_err()
    );
    assert_eq!(doc.to_bytes().unwrap(), original);
    doc.update_iec_local_symbol_description(0, 5, "ON", "", "Living room switch")
        .expect("edit empty local description");
    let saved = XgwxDocument::parse(&doc.to_bytes().unwrap()).expect("round trip");
    assert_eq!(
        saved.iec_local_symbols()[0].as_ref().unwrap()[5]
            .description
            .as_deref(),
        Some("Living room switch")
    );
    assert_eq!(
        saved
            .ladder_programs()
            .into_iter()
            .map(|program| program.unwrap().data)
            .collect::<Vec<_>>(),
        before
    );
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE, LIBXGWX_NATIVE_DESCRIPTION_FIXTURE and LIBXGWX_GENERATED_DESCRIPTION_FIXTURE"]
fn xgi_native_description_save_preserves_edit() {
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_DESCRIPTION_FIXTURE").expect("native Save As fixture path"),
    )
    .expect("native project parses");
    let generated = XgwxDocument::from_path(
        env::var("LIBXGWX_GENERATED_DESCRIPTION_FIXTURE").expect("generated fixture path"),
    )
    .expect("generated project parses");
    let original = XgwxDocument::from_path(
        env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("source fixture path"),
    )
    .expect("source project parses");
    assert_eq!(native.programs().len(), 7);
    assert_eq!(
        native.iec_local_symbols()[0].as_ref().unwrap()[5]
            .description
            .as_deref(),
        Some("Living room switch")
    );
    for (index, ((native, generated), original)) in native
        .ladder_programs()
        .into_iter()
        .zip(generated.ladder_programs())
        .zip(original.ladder_programs())
        .enumerate()
    {
        let native = native.unwrap();
        let generated = generated.unwrap();
        let original = original.unwrap();
        assert_eq!(original.data, generated.data, "generated program {index}");
        assert_eq!(
            native.data.len(),
            generated.data.len(),
            "program {index} length"
        );
        let differences = native
            .data
            .iter()
            .zip(&generated.data)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(offset, _)| offset)
            .collect::<Vec<_>>();
        assert_eq!(
            differences,
            if index == 0 {
                vec![3208, 3871, 4426]
            } else {
                vec![]
            },
            "native program {index} changes"
        );
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE, LIBXGWX_NATIVE_LOCAL_ADD_FIXTURE and LIBXGWX_NATIVE_GENERATED_LOCAL_ADD_FIXTURE"]
fn xgi_local_symbol_insert_matches_native_record() {
    fn local_payload(doc: &XgwxDocument) -> Vec<u8> {
        let xml = roxmltree::Document::parse(&doc.xml).unwrap();
        let table = xml
            .descendants()
            .find(|node| node.has_tag_name("Program"))
            .unwrap()
            .descendants()
            .find(|node| node.has_tag_name("LocalVar"))
            .unwrap()
            .descendants()
            .find(|node| node.has_tag_name("Symbols"))
            .unwrap();
        decode_base64_payload(
            table.text().unwrap(),
            table.attribute("Compressed") == Some("1"),
        )
        .unwrap()
        .data
    }
    let source =
        XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture"))
            .unwrap();
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_LOCAL_ADD_FIXTURE").expect("native local add fixture"),
    )
    .unwrap();
    let native_generated = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_GENERATED_LOCAL_ADD_FIXTURE")
            .expect("XG5000 Save As of generated local add"),
    )
    .unwrap();
    let mut generated = source.clone();
    let original_bytes = generated.to_bytes().unwrap();
    assert!(
        generated
            .insert_iec_local_symbol(0, "ON", "BOOL", "")
            .is_err()
    );
    assert!(
        generated
            .insert_iec_local_symbol(0, "TEST_LOCAL", "INVALID", "")
            .is_err()
    );
    assert_eq!(generated.to_bytes().unwrap(), original_bytes);
    generated
        .insert_iec_local_symbol(0, "TEST_LOCAL", "BOOL", "")
        .unwrap();
    let generated = XgwxDocument::parse(&generated.to_bytes().unwrap()).unwrap();
    let generated_tables = generated.iec_local_symbols();
    let native_tables = native.iec_local_symbols();
    let symbols = generated_tables[0].as_ref().unwrap();
    let native_symbols = native_tables[0].as_ref().unwrap();
    assert_eq!(symbols.len(), 16);
    assert_eq!(symbols[6].name, "TEST_LOCAL");
    assert_eq!(symbols[6].data_type.as_deref(), Some("BOOL"));
    assert_eq!(symbols[6].storage_class, "");
    assert_eq!(symbols[6].allocation_number, None);
    assert_eq!(symbols[6].allocation_width, None);
    assert_eq!(symbols[6].name, native_symbols[6].name);
    let generated_payload = local_payload(&generated);
    let native_payload = local_payload(&native);
    let generated_record = &generated_payload[symbols[6].record_offset..symbols[7].record_offset];
    let native_record =
        &native_payload[native_symbols[6].record_offset..native_symbols[7].record_offset];
    assert_eq!(generated_record, native_record);
    assert_eq!(local_payload(&native_generated), native_payload);
    for (before, after) in source
        .ladder_programs()
        .into_iter()
        .zip(generated.ladder_programs())
    {
        assert_eq!(before.unwrap().data, after.unwrap().data);
    }
    for (before, after) in generated
        .ladder_programs()
        .into_iter()
        .zip(native_generated.ladder_programs())
    {
        assert_eq!(before.unwrap().data, after.unwrap().data);
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE, LIBXGWX_GENERATED_LOCAL_ADD_FIXTURE, LIBXGWX_NATIVE_LOCAL_ADD_FIXTURE, LIBXGWX_NATIVE_LOCAL_DELETE_FIXTURE and LIBXGWX_NATIVE_GENERATED_LOCAL_DELETE_FIXTURE"]
fn xgi_local_symbol_delete_matches_native_record_removal() {
    fn local_payload(doc: &XgwxDocument) -> Vec<u8> {
        let xml = roxmltree::Document::parse(&doc.xml).unwrap();
        let table = xml
            .descendants()
            .find(|node| node.has_tag_name("Program"))
            .unwrap()
            .descendants()
            .find(|node| node.has_tag_name("LocalVar"))
            .unwrap()
            .descendants()
            .find(|node| node.has_tag_name("Symbols"))
            .unwrap();
        decode_base64_payload(
            table.text().unwrap(),
            table.attribute("Compressed") == Some("1"),
        )
        .unwrap()
        .data
    }
    let source =
        XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture"))
            .unwrap();
    let generated_add = XgwxDocument::from_path(
        env::var("LIBXGWX_GENERATED_LOCAL_ADD_FIXTURE").expect("generated add fixture"),
    )
    .unwrap();
    let native_add = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_LOCAL_ADD_FIXTURE").expect("native add fixture"),
    )
    .unwrap();
    let native_delete = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_LOCAL_DELETE_FIXTURE").expect("native delete fixture"),
    )
    .unwrap();
    let native_generated_delete = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_GENERATED_LOCAL_DELETE_FIXTURE")
            .expect("XG5000 Save As of generated delete"),
    )
    .unwrap();
    let native_add_symbols = native_add.iec_local_symbols()[0].as_ref().unwrap().clone();
    let native_add_payload = local_payload(&native_add);
    let mut native_expected = native_add_payload.clone();
    native_expected.drain(native_add_symbols[6].record_offset..native_add_symbols[7].record_offset);
    assert_eq!(local_payload(&native_delete), native_expected);
    assert_eq!(local_payload(&native_generated_delete), native_expected);
    let mut edited = generated_add.clone();
    let before_bytes = edited.to_bytes().unwrap();
    assert!(edited.delete_iec_local_symbol(0, 6, "STALE").is_err());
    assert!(edited.delete_iec_local_symbol(0, 5, "ON").is_err());
    assert_eq!(edited.to_bytes().unwrap(), before_bytes);
    edited.delete_iec_local_symbol(0, 6, "TEST_LOCAL").unwrap();
    let edited = XgwxDocument::parse(&edited.to_bytes().unwrap()).unwrap();
    assert_eq!(
        edited
            .iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>(),
        source
            .iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    );
    assert_eq!(local_payload(&edited), local_payload(&source));
    for (before, after) in generated_add
        .ladder_programs()
        .into_iter()
        .zip(edited.ladder_programs())
    {
        assert_eq!(before.unwrap().data, after.unwrap().data);
    }
    for (index, (generated, native)) in edited
        .ladder_programs()
        .into_iter()
        .zip(native_generated_delete.ladder_programs())
        .enumerate()
    {
        let generated = generated.unwrap();
        let native = native.unwrap();
        assert_eq!(
            generated.data.len(),
            native.data.len(),
            "program {index} length"
        );
        let differences = generated
            .data
            .iter()
            .zip(&native.data)
            .enumerate()
            .filter_map(|(offset, (left, right))| (left != right).then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(
            differences,
            if index == 0 {
                vec![3208, 3871, 4426]
            } else {
                vec![]
            },
            "program {index} differences"
        );
    }
    assert_eq!(
        native_delete.iec_local_symbols()[0].as_ref().unwrap().len(),
        15
    );
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_NATIVE_LINEAR_CONTACT_FIXTURE"]
fn xgi_linear_row_allows_second_wire_contact_insertion() {
    let fixture = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture");
    let mut document = XgwxDocument::from_path(&fixture).unwrap();
    let original = document
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let sites = original[2].iec_no_contact_insertion_sites().unwrap();
    assert!(
        sites
            .iter()
            .any(|site| site.row_index == 40 && site.wire_offset == 6856),
        "sites {sites:?}, wire {:?}",
        &original[2].data[6856..6875]
    );
    document
        .insert_iec_ld_no_contact(2, 6856, 19, 16, 91, "도어열림")
        .unwrap();
    let changed = document
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_LINEAR_CONTACT_FIXTURE").expect("XG5000 Save As capture"),
    )
    .unwrap();
    let native_programs = native
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let row = changed[2]
        .iec_row_frames()
        .unwrap()
        .into_iter()
        .find(|row| row.group_index == 14 && row.row_index == 40)
        .unwrap();
    assert_eq!(row.record_count, 7);
    assert_eq!(changed[2].data[row.start + 29], 19);
    assert_eq!(
        changed[2]
            .iec_record_frames()
            .unwrap()
            .into_iter()
            .filter(|record| record.group_index == 14 && record.row_index == 40)
            .count(),
        7
    );
    for index in [0, 1, 3, 4, 5, 6] {
        assert_eq!(changed[index].data, original[index].data, "program {index}");
    }
    for index in 0..7 {
        assert_eq!(changed[index].data.len(), native_programs[index].data.len());
        let differences = changed[index]
            .data
            .iter()
            .zip(&native_programs[index].data)
            .enumerate()
            .filter_map(|(offset, (left, right))| (left != right).then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(
            differences,
            Vec::<usize>::new(),
            "program {index} differences"
        );
    }
    for (kind, code) in [
        ("NO", 0x06),
        ("NC", 0x07),
        ("RISING", 0x08),
        ("FALLING", 0x09),
        ("NEGATED_RISING", 0x0a),
        ("NEGATED_FALLING", 0x0b),
    ] {
        let mut variant = XgwxDocument::from_path(&fixture).unwrap();
        variant
            .insert_iec_ld_contact(2, 6856, 19, 16, 91, kind, "도어열림")
            .unwrap();
        let program = variant.ladder_programs().remove(2).unwrap();
        assert!(
            program.iec_circuit_graph().is_some(),
            "{kind} circuit graph"
        );
        assert!(program.iec_record_frames().unwrap().iter().any(|record| {
            record.kind == IecRecordKind::Contact(code)
                && program.data.get(record.offset + 5) == Some(&19)
        }));
        let site = program
            .iec_no_contact_cell_deletion_sites()
            .unwrap()
            .into_iter()
            .find(|site| site.raw_x == 19 && site.contact_code == code)
            .expect("inserted contact deletion site");
        let before = variant.to_bytes().unwrap();
        assert!(
            variant
                .delete_iec_ld_contact_cell(
                    2,
                    site.contact_offset,
                    site.raw_x,
                    if kind == "NO" { "NC" } else { "NO" },
                    "도어열림",
                )
                .is_err(),
            "{kind} stale kind guard"
        );
        assert_eq!(
            variant.to_bytes().unwrap(),
            before,
            "{kind} rejected atomically"
        );
        variant
            .delete_iec_ld_contact_cell(2, site.contact_offset, site.raw_x, kind, "도어열림")
            .unwrap();
        let deleted = variant.ladder_programs().remove(2).unwrap();
        assert!(
            deleted.iec_circuit_graph().is_some(),
            "{kind} deletion graph"
        );
        assert!(!deleted.iec_record_frames().unwrap().iter().any(|record| {
            record.group_index == 14
                && record.row_index == 40
                && record.kind == IecRecordKind::Contact(code)
                && deleted.data.get(record.offset + 5) == Some(&19)
        }));
    }
    let mut invalid = XgwxDocument::from_path(&fixture).unwrap();
    assert!(
        invalid
            .insert_iec_ld_contact(2, 6856, 19, 16, 91, "INVALID", "도어열림")
            .is_err()
    );
    assert!(
        invalid
            .insert_iec_ld_contact(2, 6856, 19, 16, 91, "NO", "MISSING_BOOL")
            .is_err()
    );
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE, LIBXGWX_NATIVE_EXISTING_DELETE_FIXTURE, and LIBXGWX_NATIVE_GENERATED_EXISTING_DELETE_FIXTURE"]
fn xgi_original_nc_contact_deletion_matches_native() {
    let mut generated =
        XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture"))
            .unwrap();
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_EXISTING_DELETE_FIXTURE").expect("native XG5000 deletion capture"),
    )
    .unwrap();
    let native_generated = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_GENERATED_EXISTING_DELETE_FIXTURE")
            .expect("native Save As of generated deletion"),
    )
    .unwrap();
    assert!(
        generated.ladder_programs()[2]
            .as_ref()
            .unwrap()
            .iec_no_contact_deletion_sites()
            .unwrap()
            .iter()
            .any(|site| site.contact_offset == 6825 && site.raw_x == 13)
    );
    generated
        .delete_iec_ld_no_contact(2, 6825, 13, "현관도어닫힘")
        .unwrap();
    let generated_programs = generated
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let native_programs = native
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let native_generated_programs = native_generated
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for index in 0..7 {
        assert_eq!(
            generated_programs[index].data, native_generated_programs[index].data,
            "program {index} after native Save As"
        );
        assert_eq!(
            generated_programs[index].data.len(),
            native_programs[index].data.len(),
            "program {index} length"
        );
        let differences = generated_programs[index]
            .data
            .iter()
            .zip(&native_programs[index].data)
            .enumerate()
            .filter_map(|(offset, (left, right))| (left != right).then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(
            differences,
            if index == 2 {
                vec![169, 407, 687, 972]
            } else {
                vec![]
            },
            "program {index} differences"
        );
        if index == 2 {
            assert_eq!(
                [169, 407, 687, 972].map(|offset| native_programs[index].data[offset]),
                [50, 39, 39, 50]
            );
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_NATIVE_DELETED_CONTACT_FIXTURE and LIBXGWX_NATIVE_WIRE_REPAIR_FIXTURE"]
fn xgi_native_horizontal_wire_repair_shape() {
    let mut generated = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_DELETED_CONTACT_FIXTURE").expect("native deleted-contact fixture"),
    )
    .unwrap();
    let repaired = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_WIRE_REPAIR_FIXTURE").expect("native wire repair fixture"),
    )
    .unwrap();
    let site = generated.ladder_programs()[2]
        .as_ref()
        .unwrap()
        .iec_horizontal_wire_repair_sites()
        .unwrap()
        .into_iter()
        .find(|site| site.row_index == 40)
        .unwrap();
    assert_eq!(site.insertion_offset, 6825);
    assert_eq!(site.raw_x, 13);
    generated
        .repair_iec_ld_horizontal_wire(2, site.insertion_offset, site.raw_x)
        .unwrap();
    let generated_programs = generated
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let repaired_programs = repaired
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for index in 0..7 {
        assert_eq!(
            generated_programs[index].data, repaired_programs[index].data,
            "program {index} after native F5 wire repair"
        );
    }
    let program = &generated_programs[2];
    let rows = program.iec_row_frames().unwrap();
    let records = program.iec_record_frames().unwrap();
    let row = rows.iter().find(|row| row.row_index == 40).unwrap();
    assert_eq!(row.record_count, 5);
    let row_records = records
        .iter()
        .filter(|record| record.group_index == row.group_index && record.row_index == 40)
        .collect::<Vec<_>>();
    assert_eq!(row_records[2].kind, IecRecordKind::ShortWire);
    assert_eq!(
        &program.data[row_records[2].offset..row_records[2].end],
        &[0xff, 0x01, 0, 0, 0, 13, 160, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert!(
        program
            .iec_horizontal_wire_repair_sites()
            .unwrap()
            .is_empty()
    );
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_NATIVE_LINEAR_CONTACT_FIXTURE, LIBXGWX_NATIVE_LINEAR_CELL_DELETE_FIXTURE, and LIBXGWX_NATIVE_LINEAR_MIDDLE_CELL_DELETE_FIXTURE"]
fn xgi_native_cell_delete_matches_writer() {
    let source_path = env::var("LIBXGWX_NATIVE_LINEAR_CONTACT_FIXTURE")
        .expect("native linear-contact source fixture");
    let source = XgwxDocument::from_path(&source_path).unwrap();
    let source_programs = source.ladder_programs();
    let program = source_programs[2].as_ref().unwrap();
    let sites = program.iec_no_contact_cell_deletion_sites().unwrap();
    assert!(
        sites
            .iter()
            .any(|site| site.contact_offset == 6825 && site.raw_x == 13)
    );
    assert!(
        sites
            .iter()
            .any(|site| site.contact_offset == 6875 && site.raw_x == 19)
    );

    for (native_env, offset, raw_x, variable) in [
        (
            "LIBXGWX_NATIVE_LINEAR_CELL_DELETE_FIXTURE",
            6875,
            19,
            "도어열림",
        ),
        (
            "LIBXGWX_NATIVE_LINEAR_MIDDLE_CELL_DELETE_FIXTURE",
            6825,
            13,
            "현관도어닫힘",
        ),
    ] {
        let mut generated = XgwxDocument::from_path(&source_path).unwrap();
        generated
            .delete_iec_ld_no_contact_cell(2, offset, raw_x, variable)
            .unwrap();
        let native = XgwxDocument::from_path(env::var(native_env).expect(native_env)).unwrap();
        for (index, (generated, native)) in generated
            .ladder_programs()
            .into_iter()
            .zip(native.ladder_programs())
            .enumerate()
        {
            let generated = generated.unwrap();
            let native = native.unwrap();
            let differences = generated
                .data
                .iter()
                .zip(&native.data)
                .enumerate()
                .filter_map(|(offset, (left, right))| (left != right).then_some(offset))
                .collect::<Vec<_>>();
            assert_eq!(
                (generated.data.len(), differences),
                (
                    native.data.len(),
                    if index == 2 {
                        vec![169, 407, 687, 972]
                    } else {
                        vec![]
                    }
                ),
                "program {index} for {native_env}",
            );
        }
        let generated_programs = generated.ladder_programs();
        let program = generated_programs[2].as_ref().unwrap();
        let row = program
            .iec_row_frames()
            .unwrap()
            .into_iter()
            .find(|row| row.group_index == 14 && row.row_index == 40)
            .unwrap();
        assert_eq!(row.record_count, 6);
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE, LIBXGWX_NATIVE_WIRE_REPAIR_FIXTURE, and LIBXGWX_NATIVE_GENERATED_WIRE_REPAIR_FIXTURE"]
fn xgi_generated_delete_and_wire_repair_matches_native() {
    let mut generated =
        XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture"))
            .unwrap();
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_WIRE_REPAIR_FIXTURE").expect("native wire repair fixture"),
    )
    .unwrap();
    let native_generated = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_GENERATED_WIRE_REPAIR_FIXTURE")
            .expect("native Save As of generated wire repair"),
    )
    .unwrap();
    generated
        .delete_iec_ld_no_contact(2, 6825, 13, "현관도어닫힘")
        .unwrap();
    generated
        .repair_iec_ld_horizontal_wire(2, 6825, 13)
        .unwrap();
    let generated_programs = generated
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let native_programs = native
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let native_generated_programs = native_generated
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for index in 0..7 {
        assert_eq!(
            generated_programs[index].data, native_generated_programs[index].data,
            "program {index} after native Save As of generated repair"
        );
        let differences = generated_programs[index]
            .data
            .iter()
            .zip(&native_programs[index].data)
            .enumerate()
            .filter_map(|(offset, (left, right))| (left != right).then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(
            differences,
            if index == 2 {
                vec![169, 407, 687, 972]
            } else {
                vec![]
            },
            "program {index} differences"
        );
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to the captured XGI smart home file"]
fn xgi_local_type_change_matches_native_allocation_reset() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture path");
    let source = XgwxDocument::from_path(path).expect("fixture parses");
    assert_eq!(
        source.iec_local_symbols()[4].as_ref().unwrap()[18]
            .data_type
            .as_deref(),
        Some("INT")
    );
    for (replacement, native_env) in [
        ("WORD", "LIBXGWX_NATIVE_TYPE_WORD"),
        ("DINT", "LIBXGWX_NATIVE_TYPE_DINT"),
    ] {
        let mut edited = source.clone();
        let before_programs = edited
            .ladder_programs()
            .into_iter()
            .map(|program| program.unwrap().data)
            .collect::<Vec<_>>();
        edited
            .update_iec_local_symbol_type(4, 18, "메모리값", "INT", replacement)
            .expect("captured primitive type edit");
        let saved = XgwxDocument::parse(&edited.to_bytes().unwrap()).unwrap();
        let saved_symbols = saved.iec_local_symbols();
        let symbol = &saved_symbols[4].as_ref().unwrap()[18];
        assert_eq!(symbol.data_type.as_deref(), Some(replacement));
        assert_eq!(symbol.storage_class, "");
        assert_eq!(symbol.allocation_number, None);
        assert_eq!(symbol.allocation_width, None);
        assert_eq!(
            saved
                .ladder_programs()
                .into_iter()
                .map(|program| program.unwrap().data)
                .collect::<Vec<_>>(),
            before_programs
        );
        if let Ok(native_path) = env::var(native_env) {
            let native = XgwxDocument::from_path(native_path).unwrap();
            let native_symbols = native.iec_local_symbols();
            let native_symbol = &native_symbols[4].as_ref().unwrap()[18];
            assert_eq!(symbol.data_type_code, native_symbol.data_type_code);
            assert_eq!(symbol.storage_class, native_symbol.storage_class);
            assert_eq!(symbol.allocation_number, native_symbol.allocation_number);
            assert_eq!(symbol.allocation_width, native_symbol.allocation_width);
            assert_eq!(
                saved
                    .ladder_programs()
                    .into_iter()
                    .map(|program| program.unwrap().data)
                    .collect::<Vec<_>>(),
                native
                    .ladder_programs()
                    .into_iter()
                    .map(|program| program.unwrap().data)
                    .collect::<Vec<_>>()
            );
        }
    }
    let mut mapped = source.clone();
    let before = mapped.to_bytes().unwrap();
    assert!(
        mapped
            .update_iec_local_symbol_type(0, 5, "ON", "BOOL", "WORD")
            .is_err()
    );
    assert_eq!(mapped.to_bytes().unwrap(), before);
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
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to an IEC project"]
fn xgi_scalar_function_placement_validates_types_and_preserves_other_programs() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap();
    let mut doc = XgwxDocument::from_path(source).unwrap();
    let before = doc
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let row = u16::from_le_bytes(before[0].data[4..6].try_into().unwrap());
    for (name, operands) in [
        ("MOVE", vec!["1", "%IW100"]),
        ("MOVE", vec!["TRUE", "%MW100"]),
        ("ADD", vec!["%MW100", "1", "%MX100"]),
        ("EQ", vec!["%MW100", "1", "%MW100"]),
    ] {
        let snapshot = doc.to_bytes().unwrap();
        assert!(
            doc.insert_iec_ld_function(
                0,
                row,
                10,
                name,
                &operands.into_iter().map(String::from).collect::<Vec<_>>()
            )
            .is_err()
        );
        assert_eq!(doc.to_bytes().unwrap(), snapshot);
    }
    doc.insert_iec_ld_function(0, row, 10, "MOVE", &["1".into(), "%MW100".into()])
        .unwrap();
    doc.insert_iec_ld_function(
        0,
        row,
        22,
        "ADD",
        &["%MW100".into(), "1".into(), "%MW102".into()],
    )
    .unwrap();
    let after = doc
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    for (old, new) in before.iter().zip(after.iter()).skip(1) {
        assert_eq!(old.data, new.data);
    }
    let blocks = after[0].iec_function_blocks().unwrap();
    assert!(
        blocks
            .iter()
            .any(|b| b.row_index == row && b.raw_x == 10 && b.name.value == "MOVE")
    );
    assert!(
        blocks
            .iter()
            .any(|b| b.row_index == row && b.raw_x == 22 && b.name.value == "ADD")
    );
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

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_NATIVE_BRANCHED_CONTACT_FIXTURE"]
fn xgi_branched_row_contact_insertion_survives_native_save_as() {
    let source =
        XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("smart home fixture"))
            .unwrap();
    let original = source.ladder_programs().remove(0).unwrap();
    assert!(
        original
            .iec_no_contact_insertion_sites()
            .unwrap()
            .iter()
            .any(|site| {
                site.group_index == 3
                    && site.row_index == 3
                    && site.wire_offset == 416
                    && site.start_x == 7
                    && site.end_x == 91
            })
    );

    let mut edited = source.clone();
    edited
        .insert_iec_ld_contact(0, 416, 10, 7, 91, "NO", "ON")
        .unwrap();
    let program = edited.ladder_programs().remove(0).unwrap();
    assert!(program.iec_circuit_graph().is_some());
    assert!(program.iec_record_frames().unwrap().iter().any(|record| {
        record.offset == 435
            && record.kind == IecRecordKind::Contact(6)
            && record.group_index == 3
            && record.row_index == 3
    }));

    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_BRANCHED_CONTACT_FIXTURE")
            .expect("XG5000 branched contact Save As capture"),
    )
    .unwrap();
    for (index, (edited, native)) in edited
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .enumerate()
    {
        assert_eq!(
            edited.unwrap().data,
            native.unwrap().data,
            "program {index}"
        );
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_BRANCHED_CONTACT_FIXTURE and LIBXGWX_NATIVE_BRANCHED_DELETE_FIXTURE and LIBXGWX_BRANCHED_DELETE_RESAVE_FIXTURE"]
fn xgi_branched_row_contact_delete_matches_native_and_survives_save_as() {
    let fixture = env::var("LIBXGWX_BRANCHED_CONTACT_FIXTURE").expect("inserted contact fixture");
    let mut edited = XgwxDocument::from_path(&fixture).unwrap();
    let original = edited.ladder_programs().remove(0).unwrap();
    assert!(
        original
            .iec_no_contact_deletion_sites()
            .unwrap()
            .iter()
            .any(|site| {
                site.group_index == 3
                    && site.row_index == 3
                    && site.contact_offset == 435
                    && site.raw_x == 10
                    && site.contact_code == 6
            })
    );
    edited
        .delete_iec_ld_contact(0, 435, 10, "NO", "ON")
        .unwrap();
    let changed = edited.ladder_programs().remove(0).unwrap();
    assert!(changed.iec_circuit_graph().is_some());
    assert_eq!(
        changed
            .iec_row_frames()
            .unwrap()
            .iter()
            .find(|row| { row.group_index == 3 && row.row_index == 3 })
            .unwrap()
            .record_count,
        6
    );

    let native_delete = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_BRANCHED_DELETE_FIXTURE").expect("native Delete capture"),
    )
    .unwrap();
    let native_resave = XgwxDocument::from_path(
        env::var("LIBXGWX_BRANCHED_DELETE_RESAVE_FIXTURE").expect("generated Save As capture"),
    )
    .unwrap();
    for (index, ((edited, native), resaved)) in edited
        .ladder_programs()
        .into_iter()
        .zip(native_delete.ladder_programs())
        .zip(native_resave.ladder_programs())
        .enumerate()
    {
        let edited = edited.unwrap();
        let native = native.unwrap();
        let resaved = resaved.unwrap();
        let differences = edited
            .data
            .iter()
            .zip(&native.data)
            .enumerate()
            .filter_map(|(offset, (left, right))| (left != right).then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(
            differences,
            if index == 0 {
                vec![0x4a4, 0x58d, 0xc9b, 0xf32, 0x115d]
            } else {
                vec![]
            },
            "native Delete differences in program {index}"
        );
        assert_eq!(
            edited.data, resaved.data,
            "generated Save As program {index}"
        );
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_NATIVE_BRANCHED_DELETE_FIXTURE and LIBXGWX_NATIVE_BRANCHED_REPAIR_FIXTURE"]
fn xgi_branched_gap_f5_repair_matches_native() {
    let mut document = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_BRANCHED_DELETE_FIXTURE").expect("native branched Delete capture"),
    )
    .unwrap();
    let program = document.ladder_programs().remove(0).unwrap();
    assert!(
        program
            .iec_horizontal_wire_repair_sites()
            .unwrap()
            .iter()
            .any(|site| {
                site.group_index == 3
                    && site.row_index == 3
                    && site.insertion_offset == 435
                    && site.raw_x == 10
            })
    );
    document.repair_iec_ld_horizontal_wire(0, 435, 10).unwrap();
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_BRANCHED_REPAIR_FIXTURE").expect("native F5 repair capture"),
    )
    .unwrap();
    for (index, (generated, native)) in document
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .enumerate()
    {
        assert_eq!(
            generated.unwrap().data,
            native.unwrap().data,
            "program {index}"
        );
    }
    if let Ok(resaved_path) = env::var("LIBXGWX_LEADING_CONTACT_INSERT_RESAVE_FIXTURE") {
        let resaved = XgwxDocument::from_path(resaved_path).unwrap();
        for (index, (generated, resaved)) in document
            .ladder_programs()
            .into_iter()
            .zip(resaved.ladder_programs())
            .enumerate()
        {
            assert_eq!(
                generated.unwrap().data,
                resaved.unwrap().data,
                "Save As program {index}"
            );
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_BRANCHED_CONTACT_FIXTURE and LIBXGWX_NATIVE_BRANCHED_CELL_DELETE_FIXTURE and LIBXGWX_BRANCHED_CELL_DELETE_RESAVE_FIXTURE"]
fn xgi_branched_contact_cell_delete_matches_native_and_survives_save_as() {
    let mut document = XgwxDocument::from_path(
        env::var("LIBXGWX_BRANCHED_CONTACT_FIXTURE").expect("branched contact fixture"),
    )
    .unwrap();
    let program = document.ladder_programs().remove(0).unwrap();
    assert!(
        program
            .iec_no_contact_cell_deletion_sites()
            .unwrap()
            .iter()
            .any(|site| {
                site.group_index == 3
                    && site.row_index == 3
                    && site.contact_offset == 435
                    && site.raw_x == 10
                    && site.contact_code == 6
            })
    );
    document
        .delete_iec_ld_contact_cell(0, 435, 10, "NO", "ON")
        .unwrap();
    let changed = document.ladder_programs().remove(0).unwrap();
    assert!(changed.iec_circuit_graph().is_some());
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_BRANCHED_CELL_DELETE_FIXTURE")
            .expect("native branched Cell Delete capture"),
    )
    .unwrap();
    let resaved = XgwxDocument::from_path(
        env::var("LIBXGWX_BRANCHED_CELL_DELETE_RESAVE_FIXTURE")
            .expect("library-generated Cell Delete Save As capture"),
    )
    .unwrap();
    for (index, ((generated, native), resaved)) in document
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .zip(resaved.ladder_programs())
        .enumerate()
    {
        let generated = generated.unwrap();
        let native = native.unwrap();
        let resaved = resaved.unwrap();
        let differences = generated
            .data
            .iter()
            .zip(&native.data)
            .enumerate()
            .filter_map(|(offset, (left, right))| (left != right).then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(
            differences,
            if index == 0 {
                vec![0x4a4, 0x58d, 0x649, 0x753, 0xc9b, 0xf32, 0x115d]
            } else {
                vec![]
            },
            "native Cell Delete differences in program {index}"
        );
        assert_eq!(generated.data, resaved.data, "Save As program {index}");
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_BRANCHED_CONTACT_FIXTURE, LIBXGWX_NATIVE_SIMPLE_ROW_DELETE_FIXTURE, and LIBXGWX_SIMPLE_ROW_DELETE_RESAVE_FIXTURE"]
fn xgi_simple_occupied_row_delete_matches_native() {
    let mut document = XgwxDocument::from_path(
        env::var("LIBXGWX_BRANCHED_CONTACT_FIXTURE").expect("branched contact fixture"),
    )
    .unwrap();
    let unchanged = document.to_bytes().unwrap();
    assert!(
        document
            .delete_iec_ld_simple_row(0, 2, "RISING", "STALE", "OUTPUT", "시작")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), unchanged);
    document
        .delete_iec_ld_simple_row(0, 2, "RISING", "스위치_1", "OUTPUT", "시작")
        .unwrap();
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_SIMPLE_ROW_DELETE_FIXTURE")
            .expect("native occupied-row deletion capture"),
    )
    .unwrap();
    let resaved = XgwxDocument::from_path(
        env::var("LIBXGWX_SIMPLE_ROW_DELETE_RESAVE_FIXTURE")
            .expect("library occupied-row Save As capture"),
    )
    .unwrap();
    for (index, ((generated, native), resaved)) in document
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .zip(resaved.ladder_programs())
        .enumerate()
    {
        let generated = generated.unwrap();
        let native = native.unwrap();
        let resaved = resaved.unwrap();
        let differences = generated
            .data
            .iter()
            .zip(&native.data)
            .enumerate()
            .filter_map(|(offset, (left, right))| (left != right).then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(generated.data.len(), native.data.len());
        assert_eq!(
            differences,
            if index == 0 {
                vec![0x447, 0x530, 0x5ec, 0x6f6, 0x9ff, 0xc3e, 0xed5, 0x1100]
            } else {
                vec![]
            },
            "native occupied-row differences in program {index}"
        );
        assert!(generated.iec_circuit_graph().is_some());
        assert_eq!(generated.data, resaved.data, "Save As program {index}");
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_BRANCHED_CONTACT_FIXTURE, LIBXGWX_NATIVE_BRANCH_TOP_ROW_DELETE_FIXTURE, and LIBXGWX_BRANCH_TOP_ROW_DELETE_RESAVE_FIXTURE"]
fn xgi_branched_upper_row_delete_matches_native() {
    let mut document = XgwxDocument::from_path(
        env::var("LIBXGWX_BRANCHED_CONTACT_FIXTURE").expect("branched contact fixture"),
    )
    .unwrap();
    let unchanged = document.to_bytes().unwrap();
    assert!(document.delete_iec_ld_branch_top_row(0, 3, 4).is_err());
    assert_eq!(document.to_bytes().unwrap(), unchanged);
    document.delete_iec_ld_branch_top_row(0, 3, 3).unwrap();
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_BRANCH_TOP_ROW_DELETE_FIXTURE")
            .expect("native branch upper-row deletion capture"),
    )
    .unwrap();
    let resaved = XgwxDocument::from_path(
        env::var("LIBXGWX_BRANCH_TOP_ROW_DELETE_RESAVE_FIXTURE")
            .expect("library branch upper-row Save As capture"),
    )
    .unwrap();
    for (index, ((generated, native), resaved)) in document
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .zip(resaved.ladder_programs())
        .enumerate()
    {
        let generated = generated.unwrap();
        let native = native.unwrap();
        let resaved = resaved.unwrap();
        let differences = generated
            .data
            .iter()
            .zip(&native.data)
            .enumerate()
            .filter_map(|(offset, (left, right))| (left != right).then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(generated.data.len(), native.data.len());
        assert_eq!(
            differences,
            if index == 0 {
                vec![
                    0x14d, 0x3ea, 0x4d3, 0x58f, 0x699, 0x9a2, 0xbe1, 0xe78, 0x10a3,
                ]
            } else {
                vec![]
            },
            "native branch upper-row differences in program {index}"
        );
        assert!(generated.iec_circuit_graph().is_some());
        assert_eq!(generated.data, resaved.data, "Save As program {index}");
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_BRANCHED_CONTACT_FIXTURE, LIBXGWX_NATIVE_LEADING_CONTACT_DELETE_FIXTURE, and LIBXGWX_LEADING_CONTACT_DELETE_RESAVE_FIXTURE"]
fn xgi_branched_leading_contact_delete_matches_native() {
    let mut document = XgwxDocument::from_path(
        env::var("LIBXGWX_BRANCHED_CONTACT_FIXTURE").expect("branched contact fixture"),
    )
    .unwrap();
    let program = document.ladder_programs().remove(0).unwrap();
    assert!(
        program
            .iec_no_contact_deletion_sites()
            .unwrap()
            .iter()
            .any(|site| {
                site.group_index == 3
                    && site.row_index == 3
                    && site.contact_offset == 339
                    && site.raw_x == 1
                    && site.contact_code == 6
            })
    );
    let unchanged = document.to_bytes().unwrap();
    assert!(
        document
            .delete_iec_ld_contact(0, 339, 1, "NO", "STALE")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), unchanged);
    document
        .delete_iec_ld_contact(0, 339, 1, "NO", "시작")
        .unwrap();
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_LEADING_CONTACT_DELETE_FIXTURE")
            .expect("native leading-contact Delete capture"),
    )
    .unwrap();
    let resaved = XgwxDocument::from_path(
        env::var("LIBXGWX_LEADING_CONTACT_DELETE_RESAVE_FIXTURE")
            .expect("library leading-contact Save As capture"),
    )
    .unwrap();
    for (index, ((generated, native), resaved)) in document
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .zip(resaved.ladder_programs())
        .enumerate()
    {
        let generated = generated.unwrap();
        let native = native.unwrap();
        let resaved = resaved.unwrap();
        let differences = generated
            .data
            .iter()
            .zip(&native.data)
            .enumerate()
            .filter_map(|(offset, (left, right))| (left != right).then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(generated.data.len(), native.data.len());
        assert_eq!(
            differences,
            if index == 0 {
                vec![0x4a4, 0x58d, 0x649, 0x753, 0xa5c, 0xc9b, 0xf32, 0x115d]
            } else {
                vec![]
            },
            "native leading-contact differences in program {index}"
        );
        assert_eq!(generated.data, resaved.data, "Save As program {index}");
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_NATIVE_LEADING_CONTACT_DELETE_FIXTURE and LIBXGWX_NATIVE_LEADING_CONTACT_INSERT_FIXTURE"]
fn xgi_branched_leading_contact_insert_matches_native() {
    let mut document = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_LEADING_CONTACT_DELETE_FIXTURE")
            .expect("native leading-contact Delete capture"),
    )
    .unwrap();
    let program = document.ladder_programs().remove(0).unwrap();
    assert!(
        program
            .iec_leading_contact_insertion_sites()
            .unwrap()
            .iter()
            .any(|site| site.group_index == 3
                && site.row_index == 3
                && site.insertion_offset == 339)
    );
    let unchanged = document.to_bytes().unwrap();
    assert!(
        document
            .insert_iec_ld_leading_contact(0, 339, "NO", "MISSING_BOOL")
            .is_err()
    );
    assert!(
        document
            .insert_iec_ld_leading_contact(0, 340, "NO", "시작")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), unchanged);
    document
        .insert_iec_ld_leading_contact(0, 339, "NO", "시작")
        .unwrap();
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_LEADING_CONTACT_INSERT_FIXTURE")
            .expect("native leading-contact insert capture"),
    )
    .unwrap();
    for (index, (generated, native)) in document
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .enumerate()
    {
        assert_eq!(
            generated.unwrap().data,
            native.unwrap().data,
            "program {index}"
        );
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_NATIVE_LEADING_CONTACT_INSERT_FIXTURE and LIBXGWX_NATIVE_LEADING_CELL_DELETE_FIXTURE"]
fn xgi_branched_leading_cell_delete_matches_native() {
    let mut document = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_LEADING_CONTACT_INSERT_FIXTURE")
            .expect("native leading-contact insertion capture"),
    )
    .unwrap();
    let program = document.ladder_programs().remove(0).unwrap();
    assert!(
        program
            .iec_no_contact_cell_deletion_sites()
            .unwrap()
            .iter()
            .any(|site| site.group_index == 3
                && site.row_index == 3
                && site.contact_offset == 339
                && site.raw_x == 1)
    );
    let unchanged = document.to_bytes().unwrap();
    assert!(
        document
            .delete_iec_ld_contact_cell(0, 339, 1, "NC", "시작")
            .is_err()
    );
    assert!(
        document
            .delete_iec_ld_contact_cell(0, 339, 1, "NO", "STALE")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), unchanged);
    document
        .delete_iec_ld_contact_cell(0, 339, 1, "NO", "시작")
        .unwrap();
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_LEADING_CELL_DELETE_FIXTURE")
            .expect("native leading Cell Delete capture"),
    )
    .unwrap();
    for (index, (generated, native)) in document
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .enumerate()
    {
        assert_eq!(
            generated.unwrap().data,
            native.unwrap().data,
            "program {index}"
        );
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_NATIVE_LEADING_CELL_DELETE_FIXTURE and LIBXGWX_NATIVE_SHORT_WIRE_INSERT_FIXTURE"]
fn xgi_short_wire_contact_insert_matches_native() {
    let mut document = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_LEADING_CELL_DELETE_FIXTURE")
            .expect("native leading Cell Delete capture"),
    )
    .unwrap();
    let program = document.ladder_programs().remove(0).unwrap();
    assert!(
        program
            .iec_short_wire_contact_insertion_sites()
            .unwrap()
            .iter()
            .any(|site| site.group_index == 3
                && site.row_index == 3
                && site.wire_offset == 366
                && site.raw_x == 4)
    );
    let unchanged = document.to_bytes().unwrap();
    assert!(
        document
            .insert_iec_ld_short_wire_contact(0, 366, 7, "NO", "시작")
            .is_err()
    );
    assert!(
        document
            .insert_iec_ld_short_wire_contact(0, 366, 4, "NO", "MISSING_BOOL")
            .is_err()
    );
    assert_eq!(document.to_bytes().unwrap(), unchanged);
    document
        .insert_iec_ld_short_wire_contact(0, 366, 4, "NO", "시작")
        .unwrap();
    let native = XgwxDocument::from_path(
        env::var("LIBXGWX_NATIVE_SHORT_WIRE_INSERT_FIXTURE")
            .expect("native short-wire insertion capture"),
    )
    .unwrap();
    for (index, (generated, native)) in document
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .enumerate()
    {
        assert_eq!(
            generated.unwrap().data,
            native.unwrap().data,
            "program {index}"
        );
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to a native IEC project"]
fn xgi_empty_row_inserts_only_the_requested_element() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    for (category, kinds, raw_x) in [
        (
            "contact",
            &[
                "NO",
                "NC",
                "RISING",
                "FALLING",
                "NEGATED_RISING",
                "NEGATED_FALLING",
            ][..],
            1,
        ),
        (
            "coil",
            &["OUTPUT", "INVERSE", "SET", "RESET", "RISING", "FALLING"][..],
            94,
        ),
    ] {
        for kind in kinds {
            let mut edited = source.clone();
            let before = edited.to_bytes().unwrap();
            assert!(
                edited
                    .insert_iec_ld_single_element(
                        1,
                        13,
                        raw_x,
                        category,
                        kind,
                        if category == "coil" { "%IX0" } else { "%MW0" }
                    )
                    .is_err()
            );
            assert_eq!(edited.to_bytes().unwrap(), before);
            edited
                .insert_iec_ld_single_element(1, 13, raw_x, category, kind, "%MX1000")
                .unwrap();
            let programs = edited.ladder_programs();
            let program = programs[1].as_ref().unwrap();
            let records = program
                .iec_record_frames()
                .unwrap()
                .into_iter()
                .filter(|record| record.row_index == 13)
                .collect::<Vec<_>>();
            assert_eq!(records.len(), 1);
            assert_eq!(program.data[records[0].offset + 5], raw_x);
            assert!(program.iec_circuit_graph().is_some());
            assert!(
                matches!(records[0].kind, IecRecordKind::Contact(_)) == (category == "contact")
            );
            for index in [0, 2, 3, 4, 5, 6] {
                assert_eq!(
                    programs[index].as_ref().unwrap().data,
                    source.ladder_programs()[index].as_ref().unwrap().data
                );
            }
            let before = edited.to_bytes().unwrap();
            assert!(
                edited
                    .insert_iec_ld_single_element(1, 13, raw_x, category, kind, "%MX1001")
                    .is_err()
            );
            assert_eq!(edited.to_bytes().unwrap(), before);
        }
    }
    let mut generated = source.clone();
    for program_index in 0..7 {
        let original = generated.ladder_programs()[program_index]
            .as_ref()
            .unwrap()
            .clone();
        let row_index = original
            .iec_row_frames()
            .unwrap()
            .iter()
            .map(|row| row.row_index)
            .max()
            .unwrap()
            + 1;
        let (category, kind, raw_x) = if program_index % 2 == 0 {
            ("contact", "NO", 1)
        } else {
            ("coil", "OUTPUT", 94)
        };
        generated
            .insert_iec_ld_single_element(
                program_index,
                row_index,
                raw_x,
                category,
                kind,
                &format!("%MX{}", 1000 + program_index),
            )
            .unwrap();
    }
    if let Ok(path) = env::var("LIBXGWX_SINGLE_OUTPUT") {
        generated.write_to(path).unwrap();
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to a native IEC project"]
fn xgi_existing_empty_cells_allow_consecutive_entry_and_guard_conversion() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let mut edited = source.clone();
    let program = source.ladder_programs().remove(0).unwrap();
    let row = u16::from_le_bytes(program.data[4..6].try_into().unwrap());
    for (x, category, kind, operand) in [
        (1, "contact", "NO", "%MX1000"),
        (4, "contact", "NC", "%MX1001"),
        (94, "coil", "OUTPUT", "%MX1002"),
    ] {
        edited
            .insert_iec_ld_single_element(0, row, x, category, kind, operand)
            .unwrap();
    }
    let records = edited.ladder_programs()[0]
        .as_ref()
        .unwrap()
        .iec_record_frames()
        .unwrap();
    assert_eq!(
        records
            .iter()
            .filter(|record| record.row_index == row)
            .count(),
        3
    );
    let before = edited.to_bytes().unwrap();
    assert!(
        edited
            .insert_iec_ld_single_element(0, row, 7, "coil", "OUTPUT", "%IX0")
            .is_err()
    );
    assert_eq!(edited.to_bytes().unwrap(), before);
    for index in 1..7 {
        assert_eq!(
            edited.ladder_programs()[index].as_ref().unwrap().data,
            source.ladder_programs()[index].as_ref().unwrap().data
        );
    }
    let program = source.ladder_programs().remove(2).unwrap();
    let row = u16::from_le_bytes(program.data[4..6].try_into().unwrap());
    let mut converted = source.clone();
    for operands in [["%MX100", "변환"], ["%MW100", "%MX101"], ["%MW100", "1"]] {
        assert!(
            converted
                .insert_iec_ld_function(2, row, 10, "WORD_TO_UDINT", &operands.map(String::from))
                .is_err()
        );
        assert_eq!(converted.to_bytes().unwrap(), source.to_bytes().unwrap());
    }
    assert!(
        converted
            .insert_iec_ld_function(
                2,
                row,
                10,
                "WORD_TO_UDINT",
                &["%MW100".into(), "변환".into()],
            )
            .is_err()
    );
    assert_eq!(converted.to_bytes().unwrap(), source.to_bytes().unwrap());
}

#[cfg(feature = "write")]
#[test]
fn unverified_general_conversion_placement_is_atomic() {
    let mut doc = XgwxDocument::parse(include_bytes!("../fixtures/elements.xgwx")).unwrap();
    let original = doc.to_bytes().unwrap();
    let error = doc
        .insert_iec_ld_function(
            0,
            0,
            4,
            "WORD_TO_UDINT",
            &["%MW100".into(), "Result".into()],
        )
        .unwrap_err();
    assert!(error.to_string().contains("not native-validated"));
    assert_eq!(doc.to_bytes().unwrap(), original);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to a native IEC project"]
fn xgi_terminal_feed_and_tail_cleanup_preserve_functions_and_other_programs() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let mut edited = source.clone();
    for (p, g, start, end, x) in [(2, 8, 28, 29, 18), (2, 7, 21, 22, 21)] {
        let original_count = edited.ladder_programs()[p]
            .as_ref()
            .unwrap()
            .iec_function_blocks()
            .unwrap()
            .len();
        edited
            .edit_iec_ld_branch_segment(p, g, start, end, x, true, false)
            .unwrap();
        let program = edited.ladder_programs().remove(p).unwrap();
        assert!(program.iec_circuit_graph().is_none());
        let layout = program.iec_circuit_layout().unwrap();
        assert_eq!(
            layout.open_branch_endpoints,
            [IecCircuitPoint {
                group_index: g,
                row_index: start,
                x
            }]
        );
        let segment = program
            .iec_geometry()
            .unwrap()
            .vertical
            .into_iter()
            .find(|b| b.group_index == g && b.end_row_index == start && b.x == x)
            .unwrap();
        let before = edited.to_bytes().unwrap();
        assert!(
            edited
                .edit_iec_ld_branch_segment(
                    p,
                    g,
                    segment.start_row_index,
                    segment.end_row_index,
                    segment.x,
                    false,
                    false
                )
                .is_err()
        );
        assert_eq!(edited.to_bytes().unwrap(), before);
        edited
            .edit_iec_ld_branch_segment(
                p,
                g,
                segment.start_row_index,
                segment.end_row_index,
                segment.x,
                true,
                false,
            )
            .unwrap();
        let program = edited.ladder_programs().remove(p).unwrap();
        assert!(program.iec_circuit_graph().is_some());
        assert_eq!(program.iec_function_blocks().unwrap().len(), original_count);
        assert!(
            program
                .iec_geometry()
                .unwrap()
                .vertical
                .iter()
                .all(|b| b.group_index != g)
        );
    }
    let mut unsupported = source.clone();
    assert!(
        unsupported
            .edit_iec_ld_branch_segment(3, 24, 72, 73, 18, true, false)
            .is_err()
    );
    assert_eq!(unsupported.to_bytes().unwrap(), source.to_bytes().unwrap());
    for p in [0, 1, 3, 4, 5, 6] {
        assert_eq!(
            edited.ladder_programs()[p].as_ref().unwrap().data,
            source.ladder_programs()[p].as_ref().unwrap().data
        );
    }
    if let Ok(path) = env::var("LIBXGWX_TERMINAL_SUITE_OUTPUT") {
        edited.write_to(path).unwrap();
    }
    let mut recovery = source.clone();
    recovery
        .edit_iec_ld_branch_segment(2, 8, 28, 29, 18, true, false)
        .unwrap();
    recovery.delete_iec_ld_group(2, 8, 23).unwrap();
    assert!(
        recovery.ladder_programs()[2]
            .as_ref()
            .unwrap()
            .iec_circuit_graph()
            .is_some()
    );
}
