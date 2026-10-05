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

    let rewritten = doc.to_verified_bytes().expect("edited document writes");
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

    let rewritten = doc.to_verified_bytes().expect("edited document writes");
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
        vec![0, 0, 1, 0, 1, 0, 0]
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
        vec![4, 0, 0, 0, 0, 0, 0]
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
        4
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
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to an IEC project"]
fn xgi_branch_function_placement_creates_room_and_preserves_later_pins() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap();
    let mut baseline = XgwxDocument::from_path(source).unwrap();
    baseline.extend_iec_ld_vertical_wire(0, 3, 4, 3).unwrap();
    let before = baseline
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let layout = before[0].iec_circuit_layout().unwrap();
    let local_symbols = baseline
        .iec_local_symbols()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    for name in [
        "MOVE", "ADD", "SUB", "MUL", "DIV", "EQ", "GT", "GE", "LT", "LE",
    ] {
        let count = if name == "MOVE" { 2 } else { 3 };
        let comparison = matches!(name, "EQ" | "GT" | "GE" | "LT" | "LE");
        let operands = if count == 2 {
            vec!["1".into(), "%MW100".into()]
        } else {
            vec![
                "1".into(),
                "2".into(),
                if comparison {
                    "%MX100".into()
                } else {
                    "%MW100".into()
                },
            ]
        };
        let mut doc = baseline.clone();
        doc.insert_iec_ld_function(0, 5, 7, name, &operands)
            .unwrap();
        let after = doc
            .ladder_programs()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        for (old, new) in before.iter().zip(&after).skip(1) {
            assert_eq!(old.data, new.data);
        }
        assert_eq!(
            doc.iec_local_symbols()
                .into_iter()
                .map(Result::unwrap)
                .collect::<Vec<_>>(),
            local_symbols
        );
        let graph = after[0].iec_circuit_layout().unwrap();
        assert_eq!(graph.open_branch_endpoints.len(), 1);
        assert_eq!(graph.open_branch_endpoints[0].row_index, 5 + count);
        assert_eq!(
            graph.function_bindings.len(),
            layout.function_bindings.len() + usize::from(count)
        );
        let normalize = |mut binding: crate::IecFunctionBinding| {
            binding.block_record_offset = 0;
            binding.reference_record_offset = 0;
            binding.expression_record_offset = None;
            binding
        };
        let expected = layout
            .function_bindings
            .iter()
            .cloned()
            .map(|mut binding| {
                if binding.pin_point.row_index > 5 {
                    binding.pin_point.row_index += count;
                }
                normalize(binding)
            })
            .collect::<Vec<_>>();
        let actual = graph
            .function_bindings
            .iter()
            .filter(|binding| {
                binding.pin_point.row_index < 5 || binding.pin_point.row_index > 5 + count
            })
            .cloned()
            .map(normalize)
            .collect::<Vec<_>>();
        assert_eq!(expected, actual, "{name}");
        let snapshot = doc.to_bytes().unwrap();
        assert!(
            doc.insert_iec_ld_function(0, 5, 7, name, &operands)
                .is_err()
        );
        assert_eq!(doc.to_bytes().unwrap(), snapshot);
        let block = after[0]
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|block| block.row_index == 5 && block.raw_x == 7)
            .unwrap();
        assert!(
            doc.delete_iec_ld_branch_function(0, block.record_offset, "UNKNOWN")
                .is_err()
        );
        assert_eq!(doc.to_bytes().unwrap(), snapshot);
        doc.delete_iec_ld_branch_function(0, block.record_offset, name)
            .unwrap();
        let deleted = doc.ladder_programs().remove(0).unwrap();
        let deletion_graph = deleted.iec_circuit_layout().unwrap();
        assert_eq!(
            deletion_graph.open_branch_endpoints,
            graph.open_branch_endpoints
        );
        assert_eq!(
            deletion_graph.function_bindings.len(),
            layout.function_bindings.len()
        );
        let mut replacement = doc.clone();
        let replacement_name = if count == 2 { "ADD" } else { "MOVE" };
        let replacement_operands = if count == 2 {
            vec!["1".into(), "2".into(), "%MW100".into()]
        } else {
            vec!["1".into(), "%MW100".into()]
        };
        replacement
            .insert_iec_ld_function(0, 5, 7, replacement_name, &replacement_operands)
            .unwrap();
        let mut atomic = XgwxDocument::parse(&snapshot).unwrap();
        for (expected, new_name, new_operands) in [
            ("UNKNOWN", replacement_name, replacement_operands.clone()),
            (name, "UNKNOWN", replacement_operands.clone()),
            (name, "ADD", vec!["1".into(), "2".into(), "%IW100".into()]),
            (
                name,
                "ADD",
                vec!["TRUE".into(), "2".into(), "%MW100".into()],
            ),
        ] {
            assert!(
                atomic
                    .replace_iec_ld_branch_function(
                        0,
                        block.record_offset,
                        expected,
                        new_name,
                        &new_operands
                    )
                    .is_err()
            );
            assert_eq!(atomic.to_bytes().unwrap(), snapshot);
        }
        atomic
            .replace_iec_ld_branch_function(
                0,
                block.record_offset,
                name,
                replacement_name,
                &replacement_operands,
            )
            .unwrap();
        assert_eq!(atomic.to_bytes().unwrap(), replacement.to_bytes().unwrap());
        let replacement_program = replacement.ladder_programs().remove(0).unwrap();
        let replacement_graph = replacement_program.iec_circuit_layout().unwrap();
        assert_eq!(replacement_graph.open_branch_endpoints[0].row_index, 8);
        for (old, new) in before
            .iter()
            .skip(1)
            .zip(replacement.ladder_programs().into_iter().skip(1))
        {
            assert_eq!(old.data, new.unwrap().data);
        }
        let block = replacement_program
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|block| block.row_index == 5 && block.raw_x == 7)
            .unwrap();
        let replacement_snapshot = replacement.to_bytes().unwrap();
        replacement
            .delete_iec_ld_branch_function(0, block.record_offset, replacement_name)
            .unwrap();
        replacement
            .insert_iec_ld_function(0, 5, 7, replacement_name, &replacement_operands)
            .unwrap();
        assert_eq!(replacement.to_bytes().unwrap(), replacement_snapshot);
        doc.insert_iec_ld_function(0, 5, 7, name, &operands)
            .unwrap();
        assert_eq!(
            doc.to_bytes().unwrap(),
            snapshot,
            "{name} deletion and replacement"
        );
    }
    for operands in [
        vec!["TRUE".into(), "%MW100".into()],
        vec!["1".into(), "%IW100".into()],
    ] {
        let mut doc = baseline.clone();
        let snapshot = doc.to_bytes().unwrap();
        assert!(
            doc.insert_iec_ld_function(0, 5, 7, "MOVE", &operands)
                .is_err()
        );
        assert_eq!(doc.to_bytes().unwrap(), snapshot);
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
            .insert_iec_ld_leading_contact(0, 339, "NO", "MISSING_BOOL + 1")
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
            .insert_iec_ld_short_wire_contact(0, 366, 4, "NO", "MISSING_BOOL + 1")
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
    for (g, start, end, x) in [(24, 72, 73, 18), (23, 66, 67, 21)] {
        let mut with_reference = source.clone();
        let original_count = source.ladder_programs()[3]
            .as_ref()
            .unwrap()
            .iec_function_blocks()
            .unwrap()
            .len();
        with_reference
            .edit_iec_ld_branch_segment(3, g, start, end, x, true, false)
            .unwrap();
        let intermediate = with_reference.ladder_programs().remove(3).unwrap();
        assert_eq!(
            intermediate
                .iec_circuit_layout()
                .unwrap()
                .open_branch_endpoints
                .len(),
            1
        );
        assert_eq!(
            intermediate.iec_function_blocks().unwrap().len(),
            original_count
        );
        with_reference
            .edit_iec_ld_branch_segment(3, g, start - 1, start, x, true, false)
            .unwrap();
        let cleaned = with_reference.ladder_programs().remove(3).unwrap();
        assert!(cleaned.iec_circuit_graph().is_some());
        assert_eq!(cleaned.iec_function_blocks().unwrap().len(), original_count);
        for p in [0, 1, 2, 4, 5, 6] {
            assert_eq!(
                with_reference.ladder_programs()[p].as_ref().unwrap().data,
                source.ladder_programs()[p].as_ref().unwrap().data
            );
        }
    }
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

#[cfg(feature = "write")]
#[test]
#[ignore = "requires LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_branched_arithmetic_deletion_preserves_spine_and_other_programs() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    for (p, row) in [(5, 15), (6, 30)] {
        let before = source.ladder_programs().remove(p).unwrap();
        let block = before
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row)
            .unwrap();
        let mut doc = source.clone();
        assert!(
            doc.delete_iec_ld_branched_arithmetic(p, block.record_offset, "SUB")
                .is_err()
        );
        assert_eq!(doc.to_bytes().unwrap(), source.to_bytes().unwrap());
        doc.delete_iec_ld_branched_arithmetic(p, block.record_offset, &block.name.value)
            .unwrap();
        let after = doc.ladder_programs().remove(p).unwrap();
        assert!(after.iec_circuit_graph().is_some());
        assert_eq!(
            after.iec_function_blocks().unwrap().len() + 1,
            before.iec_function_blocks().unwrap().len()
        );
        assert_eq!(
            after.iec_geometry().unwrap().vertical.len() + 1,
            before.iec_geometry().unwrap().vertical.len()
        );
        assert_eq!(
            u16::from_le_bytes(after.data[4..6].try_into().unwrap()) + 1,
            u16::from_le_bytes(before.data[4..6].try_into().unwrap())
        );
        for other in (0..7).filter(|&i| i != p) {
            assert_eq!(
                doc.ladder_programs()[other].as_ref().unwrap().data,
                source.ladder_programs()[other].as_ref().unwrap().data
            );
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "requires LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_branch_removal_reuses_native_chained_row_deletions() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    for (p, group, row, contact) in [
        (2, 7, 21, false),
        (2, 8, 28, false),
        (6, 14, 83, true),
        (6, 14, 84, true),
    ] {
        let before = source.ladder_programs().remove(p).unwrap();
        let segment = before
            .iec_geometry()
            .unwrap()
            .vertical
            .into_iter()
            .find(|s| s.group_index == group && s.end_row_index == row)
            .unwrap();
        let mut direct = source.clone();
        if contact {
            direct
                .delete_iec_ld_chained_contact_branch_row(p, group, row)
                .unwrap();
        } else {
            direct
                .delete_iec_ld_empty_branch_row(p, group, row)
                .unwrap();
        }
        let mut selected = source.clone();
        selected
            .edit_iec_ld_branch_segment(
                p,
                group,
                segment.start_row_index,
                row,
                segment.x,
                true,
                false,
            )
            .unwrap();
        assert_eq!(selected.to_bytes().unwrap(), direct.to_bytes().unwrap());
        let after = selected.ladder_programs().remove(p).unwrap();
        assert!(after.iec_circuit_graph().is_some());
        assert_eq!(
            after.iec_geometry().unwrap().vertical.len() + 1,
            before.iec_geometry().unwrap().vertical.len()
        );
        assert_eq!(
            after.iec_function_blocks().unwrap().len(),
            before.iec_function_blocks().unwrap().len()
        );
        // An incorrect branch column must reject atomically.
        let mut stale = source.clone();
        let snapshot = stale.to_bytes().unwrap();
        assert!(
            stale
                .edit_iec_ld_branch_segment(
                    p,
                    group,
                    segment.start_row_index,
                    row,
                    segment.x + 3,
                    true,
                    false
                )
                .is_err()
        );
        assert_eq!(stale.to_bytes().unwrap(), snapshot);
        for other in (0..7).filter(|&i| i != p) {
            assert_eq!(
                selected.ladder_programs()[other].as_ref().unwrap().data,
                source.ladder_programs()[other].as_ref().unwrap().data
            );
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "requires LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_connected_trigger_delete_and_repair_preserves_neighbors() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    for row in [20, 26, 32] {
        let before = source.ladder_programs().remove(0).unwrap();
        let block = before
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row && b.name.value == "R_TRIG")
            .unwrap();
        let mut doc = source.clone();
        assert!(
            doc.delete_iec_ld_function_cell(0, block.record_offset, "F_TRIG")
                .is_err()
        );
        assert_eq!(doc.to_bytes().unwrap(), source.to_bytes().unwrap());
        doc.delete_iec_ld_function_cell(0, block.record_offset, "R_TRIG")
            .unwrap();
        let deleted = doc.ladder_programs().remove(0).unwrap();
        assert!(deleted.iec_circuit_graph().is_some());
        assert_eq!(
            deleted.iec_function_blocks().unwrap().len() + 1,
            before.iec_function_blocks().unwrap().len()
        );
        assert_eq!(
            deleted.iec_row_frames().unwrap().len(),
            before.iec_row_frames().unwrap().len()
        );
        assert_eq!(
            deleted.iec_function_references().unwrap().len() + 1,
            before.iec_function_references().unwrap().len()
        );
        assert_eq!(
            deleted.iec_function_operand_links().unwrap().len(),
            before.iec_function_operand_links().unwrap().len()
        );
        let restore_site = deleted
            .iec_function_cell_insertion_sites()
            .unwrap()
            .into_iter()
            .find(|site| {
                site.function_name == "R_TRIG" && site.row_index == row && site.raw_x == block.raw_x
            })
            .unwrap();
        let instance = block.instance.as_ref().unwrap().value.as_str();
        let mut restored = doc.clone();
        assert!(
            restored
                .insert_iec_ld_function_cell(
                    0,
                    restore_site.insertion_offset,
                    "R_TRIG",
                    "Missing_Instance"
                )
                .is_err()
        );
        assert_eq!(restored.to_bytes().unwrap(), doc.to_bytes().unwrap());
        restored
            .insert_iec_ld_function_cell(0, restore_site.insertion_offset, "R_TRIG", instance)
            .unwrap();
        for p in 0..7 {
            assert_eq!(
                restored.ladder_programs()[p].as_ref().unwrap().data,
                source.ladder_programs()[p].as_ref().unwrap().data
            );
        }
        let gap = deleted
            .iec_horizontal_wire_repair_sites()
            .unwrap()
            .into_iter()
            .find(|g| g.row_index == row && g.raw_x == block.raw_x)
            .unwrap();
        let snapshot = doc.to_bytes().unwrap();
        assert!(
            doc.repair_iec_ld_horizontal_wire(0, gap.insertion_offset, gap.raw_x + 3)
                .is_err()
        );
        assert_eq!(doc.to_bytes().unwrap(), snapshot);
        doc.repair_iec_ld_horizontal_wire(0, gap.insertion_offset, gap.raw_x)
            .unwrap();
        let after = doc.ladder_programs().remove(0).unwrap();
        assert!(after.iec_circuit_graph().is_some());
        assert!(
            !after
                .iec_horizontal_wire_repair_sites()
                .unwrap()
                .iter()
                .any(|g| g.row_index == row && g.raw_x == block.raw_x)
        );
        assert_eq!(
            doc.iec_local_symbols()
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            source
                .iec_local_symbols()
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        );
        for other in 1..7 {
            assert_eq!(
                doc.ladder_programs()[other].as_ref().unwrap().data,
                source.ladder_programs()[other].as_ref().unwrap().data
            );
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to a native IEC project"]
fn xgi_vertical_wire_edits_preserve_shared_rows_and_restore_every_segment() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let originals = source
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let mut count = 0;
    for (p, program) in originals.iter().enumerate() {
        for wire in program.iec_geometry().unwrap().vertical {
            let mut edited = source.clone();
            edited
                .edit_iec_ld_vertical_wire(
                    p,
                    wire.group_index,
                    wire.start_row_index,
                    wire.end_row_index,
                    wire.x,
                    true,
                    false,
                )
                .unwrap();
            let changed = edited.ladder_programs().remove(p).unwrap();
            assert_eq!(changed.data.len() + 36, program.data.len());
            assert_eq!(
                changed.iec_row_frames().unwrap().len(),
                program.iec_row_frames().unwrap().len()
            );
            assert_eq!(
                changed
                    .iec_circuit_layout()
                    .unwrap()
                    .function_bindings
                    .len(),
                program.iec_circuit_graph().unwrap().function_bindings.len()
            );
            let snapshot = edited.to_bytes().unwrap();
            assert!(
                edited
                    .edit_iec_ld_vertical_wire(
                        p,
                        wire.group_index,
                        wire.start_row_index,
                        wire.end_row_index,
                        wire.x,
                        true,
                        false
                    )
                    .is_err()
            );
            assert_eq!(edited.to_bytes().unwrap(), snapshot);
            edited
                .edit_iec_ld_vertical_wire(
                    p,
                    wire.group_index,
                    wire.start_row_index,
                    wire.end_row_index,
                    wire.x,
                    false,
                    true,
                )
                .unwrap();
            for (before, restored) in originals.iter().zip(edited.ladder_programs()) {
                assert_eq!(
                    before.data,
                    restored.unwrap().data,
                    "program {p}, segment {wire:?}"
                );
            }
            count += 1;
        }
    }
    assert_eq!(count, 192);
    let mut edited = source.clone();
    let snapshot = edited.to_bytes().unwrap();
    assert!(
        edited
            .edit_iec_ld_vertical_wire(0, 33, 69, 70, 15, false, true)
            .is_err()
    );
    assert_eq!(edited.to_bytes().unwrap(), snapshot);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to a native IEC project"]
fn xgi_text_edits_preserve_open_layout_and_reject_invalid_operands() {
    let mut doc = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    doc.edit_iec_ld_vertical_wire(0, 33, 69, 70, 12, true, false)
        .unwrap();
    let program = doc.ladder_programs().remove(0).unwrap();
    let endpoints = program.iec_circuit_layout().unwrap().open_branch_endpoints;
    assert!(program.iec_circuit_graph().is_none());
    let operand = crate::iec_ld::function_operands(&program)
        .into_iter()
        .find(|s| s.value == "0")
        .unwrap();
    doc.update_iec_ld_function_operand(0, operand.offset, "0", "123")
        .unwrap();
    let snapshot = doc.to_bytes().unwrap();
    assert!(
        doc.update_iec_ld_function_operand(0, operand.offset, "0", "2")
            .is_err()
    );
    assert!(
        doc.update_iec_ld_function_operand(0, operand.offset, "123", "%MWbad")
            .is_err()
    );
    assert_eq!(doc.to_bytes().unwrap(), snapshot);
    let program = doc.ladder_programs().remove(0).unwrap();
    let contact = crate::iec_ld::element_operands(&program)
        .into_iter()
        .find(|s| s.record_code == 6)
        .unwrap();
    doc.update_iec_ld_contact_kind(0, contact.string.offset, "NO", "NC")
        .unwrap();
    let program = doc.ladder_programs().remove(0).unwrap();
    let comment = crate::iec_ld::comments(&program)
        .into_iter()
        .next()
        .unwrap();
    doc.update_iec_ld_comment(0, comment.offset, &comment.value, "Open wiring text edit")
        .unwrap();
    let program = doc.ladder_programs().remove(0).unwrap();
    let function = crate::iec_ld::comparison_function_names(&program)
        .into_iter()
        .next()
        .unwrap();
    doc.update_iec_ld_comparison_function(0, function.offset, &function.value, "GE")
        .unwrap();
    assert_eq!(
        doc.ladder_programs()
            .remove(0)
            .unwrap()
            .iec_circuit_layout()
            .unwrap()
            .open_branch_endpoints,
        endpoints
    );
    doc.edit_iec_ld_vertical_wire(0, 33, 69, 70, 12, false, true)
        .unwrap();
    assert!(
        doc.ladder_programs()
            .remove(0)
            .unwrap()
            .iec_circuit_graph()
            .is_some()
    );
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to a native IEC project"]
fn xgi_connect_groups_preserves_typed_functions_and_rejects_stale_boundaries() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    for (p, upper, start, lower, end, x) in [(0, 2, 2, 3, 3, 3), (3, 7, 18, 8, 19, 3)] {
        let mut doc = source.clone();
        let before = doc.ladder_programs().remove(p).unwrap();
        doc.connect_iec_ld_groups(p, upper, start, lower, end, x)
            .unwrap();
        let after = doc.ladder_programs().remove(p).unwrap();
        assert_eq!(after.data.len(), before.data.len() + 26);
        assert_eq!(
            after.iec_row_frames().unwrap().len(),
            before.iec_row_frames().unwrap().len()
        );
        assert_eq!(
            after.iec_record_frames().unwrap().len(),
            before.iec_record_frames().unwrap().len() + 2
        );
        assert_eq!(
            after.iec_circuit_graph().unwrap().function_bindings.len(),
            before.iec_circuit_graph().unwrap().function_bindings.len()
        );
        let snapshot = doc.to_bytes().unwrap();
        assert!(
            doc.connect_iec_ld_groups(p, upper, start, lower, end, x)
                .is_err()
        );
        assert_eq!(doc.to_bytes().unwrap(), snapshot);
        assert!(doc.split_iec_ld_group(p, upper, start, end).is_err());
        assert_eq!(doc.to_bytes().unwrap(), snapshot);
        doc.edit_iec_ld_vertical_wire(p, upper, start, end, x, true, false)
            .unwrap();
        doc.split_iec_ld_group(p, upper, start, end).unwrap();
        assert_eq!(doc.ladder_programs().remove(p).unwrap().data, before.data);
    }
    let mut doc = source.clone();
    let snapshot = doc.to_bytes().unwrap();
    assert!(doc.connect_iec_ld_groups(2, 7, 22, 8, 23, 3).is_err());
    assert_eq!(doc.to_bytes().unwrap(), snapshot);
    doc.insert_iec_ld_single_element(0, 66, 1, "contact", "NO", "ON")
        .unwrap();
    let before = doc.ladder_programs().remove(0).unwrap();
    doc.connect_iec_ld_groups(0, 33, 66, 34, 67, 3).unwrap();
    assert_eq!(
        doc.ladder_programs()
            .remove(0)
            .unwrap()
            .iec_circuit_graph()
            .unwrap()
            .function_bindings
            .len(),
        before.iec_circuit_graph().unwrap().function_bindings.len()
    );
    doc.edit_iec_ld_vertical_wire(0, 33, 66, 67, 3, true, false)
        .unwrap();
    doc.split_iec_ld_group(0, 33, 66, 67).unwrap();
    let rows = before.iec_row_frames().unwrap();
    let upper = rows.iter().find(|r| r.group_index == 33).unwrap();
    let lower = rows.iter().find(|r| r.group_index == 34).unwrap();
    let mut expected = before.data.clone();
    expected[upper.start - 6..upper.start - 2].copy_from_slice(&1_u32.to_le_bytes());
    expected[lower.start - 6..lower.start - 2].copy_from_slice(&1_u32.to_le_bytes());
    let restored = doc.ladder_programs().remove(0).unwrap();
    assert!(
        restored.data == expected,
        "split must inherit the merged network execution state"
    );
    for (upper, start, lower, end, x) in [
        (2, 1, 3, 3, 3),
        (2, 2, 4, 6, 3),
        (2, 2, 3, 3, 2),
        (2, 2, 3, 3, 15),
    ] {
        let mut doc = source.clone();
        let snapshot = doc.to_bytes().unwrap();
        assert!(
            doc.connect_iec_ld_groups(0, upper, start, lower, end, x)
                .is_err()
        );
        assert_eq!(doc.to_bytes().unwrap(), snapshot);
    }
    let mut doc = source.clone();
    doc.edit_iec_ld_vertical_wire(0, 33, 69, 70, 12, true, false)
        .unwrap();
    let snapshot = doc.to_bytes().unwrap();
    assert!(doc.split_iec_ld_group(0, 33, 69, 70).is_err());
    assert_eq!(doc.to_bytes().unwrap(), snapshot);
}

#[test]
#[cfg(feature = "write")]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to a native IEC project"]
fn xgi_vertical_extension_materializes_a_blank_row_and_preserves_bindings() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let original = source
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let mut doc = source.clone();
    doc.extend_iec_ld_vertical_wire(0, 3, 4, 3).unwrap();
    let changed = doc.ladder_programs().remove(0).unwrap();
    assert_eq!(changed.data.len(), original[0].data.len() + 71);
    let rows = changed.iec_row_frames().unwrap();
    let new_row = rows.iter().find(|r| r.row_index == 5).unwrap();
    assert_eq!((new_row.group_index, new_row.record_count), (3, 1));
    assert_eq!(changed.data[6..8], original[0].data[6..8]);
    assert_eq!(changed.data[4..6], original[0].data[4..6]);
    let layout = changed.iec_circuit_layout().unwrap();
    assert!(
        layout
            .open_branch_endpoints
            .iter()
            .any(|p| p.group_index == 3 && p.row_index == 5 && p.x == 3)
    );
    assert_eq!(
        layout.function_bindings.len(),
        original[0]
            .iec_circuit_layout()
            .unwrap()
            .function_bindings
            .len()
    );
    for (p, old) in original.iter().enumerate().skip(1) {
        assert_eq!(doc.ladder_programs().remove(p).unwrap().data, old.data);
    }
    let saved = doc.to_bytes().unwrap();
    assert!(doc.extend_iec_ld_vertical_wire(0, 3, 4, 3).is_err());
    assert_eq!(doc.to_bytes().unwrap(), saved);
    doc.edit_iec_ld_branch_segment(0, 3, 4, 5, 3, true, false)
        .unwrap();
    for (p, old) in original.iter().enumerate() {
        assert_eq!(doc.ladder_programs().remove(p).unwrap().data, old.data);
    }
    for (group, row, x) in [(3, 3, 3), (2, 2, 3), (3, 4, 2), (3, 4, 12), (99, 4, 3)] {
        let mut bad = source.clone();
        let before = bad.to_bytes().unwrap();
        assert!(bad.extend_iec_ld_vertical_wire(0, group, row, x).is_err());
        assert_eq!(bad.to_bytes().unwrap(), before);
    }
}

#[test]
#[cfg(feature = "write")]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to a native IEC project"]
fn xgi_contact_placement_closes_a_new_branch_and_deletion_reopens_it() {
    let mut wire = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    wire.extend_iec_ld_vertical_wire(0, 3, 4, 3).unwrap();
    let original = wire
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for kind in [
        "NO",
        "NC",
        "RISING",
        "FALLING",
        "NEGATED_RISING",
        "NEGATED_FALLING",
    ] {
        let mut doc = wire.clone();
        doc.insert_iec_ld_single_element(0, 5, 1, "contact", kind, "ON")
            .unwrap();
        let program = doc.ladder_programs().remove(0).unwrap();
        assert!(program.iec_circuit_graph().is_some());
        let records = program.iec_record_frames().unwrap();
        let row_records = records
            .iter()
            .filter(|r| r.row_index == 5)
            .collect::<Vec<_>>();
        assert!(matches!(
            row_records[0].kind,
            crate::IecRecordKind::Contact(_)
        ));
        assert_eq!(row_records[1].kind, crate::IecRecordKind::BranchEnd);
        assert_eq!(program.data[row_records[0].offset + 11], 4);
        let offset = row_records[0].offset;
        let saved = doc.to_bytes().unwrap();
        assert!(
            doc.delete_iec_ld_contact(0, offset, 1, kind, "OFF")
                .is_err()
        );
        assert_eq!(doc.to_bytes().unwrap(), saved);
        doc.delete_iec_ld_contact(0, offset, 1, kind, "ON").unwrap();
        for (p, old) in original.iter().enumerate() {
            assert_eq!(doc.ladder_programs().remove(p).unwrap().data, old.data);
        }
    }
    for (x, category, kind, operand) in [
        (1, "contact", "NO", "%MW10"),
        (4, "contact", "NO", "ON"),
        (1, "coil", "OUTPUT", "ON"),
    ] {
        let mut bad = wire.clone();
        let before = bad.to_bytes().unwrap();
        assert!(
            bad.insert_iec_ld_single_element(0, 5, x, category, kind, operand)
                .is_err()
        );
        assert_eq!(bad.to_bytes().unwrap(), before);
    }
}

#[test]
#[cfg(feature = "write")]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to a native IEC project"]
fn xgi_coil_placement_connects_a_new_branch_and_deletion_reopens_it() {
    let mut wire = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    wire.extend_iec_ld_vertical_wire(0, 3, 4, 3).unwrap();
    let original = wire
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for (code, kind) in [
        (14, "OUTPUT"),
        (15, "INVERSE"),
        (16, "SET"),
        (17, "RESET"),
        (18, "RISING"),
        (19, "FALLING"),
    ] {
        for x in [4, 7, 94] {
            let mut doc = wire.clone();
            doc.insert_iec_ld_single_element(0, 5, x, "coil", kind, "%MX0")
                .unwrap();
            let program = doc.ladder_programs().remove(0).unwrap();
            assert!(program.iec_circuit_graph().is_some());
            let records = program.iec_record_frames().unwrap();
            let row = records
                .iter()
                .filter(|r| r.row_index == 5)
                .collect::<Vec<_>>();
            assert_eq!(row.len(), 3);
            assert_eq!(row[0].kind, crate::IecRecordKind::BranchEnd);
            assert_eq!(row[1].kind, crate::IecRecordKind::LongWire);
            assert_eq!(row[2].kind, crate::IecRecordKind::Coil(code));
            assert_eq!(program.data[row[2].offset + 5], 94);
            assert_eq!(program.data[row[2].offset + 11], 0x24);
            let before = doc.to_bytes().unwrap();
            assert!(
                doc.delete_iec_ld_terminal_coil(0, row[2].offset, "%MX1")
                    .is_err()
            );
            assert_eq!(doc.to_bytes().unwrap(), before);
            doc.delete_iec_ld_terminal_coil(0, row[2].offset, "%MX0")
                .unwrap();
            for (p, old) in original.iter().enumerate() {
                assert_eq!(doc.ladder_programs().remove(p).unwrap().data, old.data);
            }
        }
    }
    for (x, operand) in [(1, "%MX0"), (7, "%MW10"), (7, "TRUE")] {
        let mut bad = wire.clone();
        let before = bad.to_bytes().unwrap();
        assert!(
            bad.insert_iec_ld_single_element(0, 5, x, "coil", "OUTPUT", operand)
                .is_err()
        );
        assert_eq!(bad.to_bytes().unwrap(), before);
    }
}

#[test]
#[cfg(feature = "write")]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to a native IEC project"]
fn xgi_wider_coils_preserve_other_open_branch_endpoints() {
    let mut wire = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    wire.extend_iec_ld_vertical_wire(0, 3, 4, 6).unwrap();
    wire.extend_iec_ld_vertical_wire(0, 8, 12, 24).unwrap();
    let original = wire
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for order in [
        [(5, 7, "%MX0"), (13, 25, "%MX1")],
        [(13, 25, "%MX1"), (5, 7, "%MX0")],
    ] {
        let mut doc = wire.clone();
        for (i, (row, x, operand)) in order.iter().enumerate() {
            doc.insert_iec_ld_single_element(0, *row, *x, "coil", "OUTPUT", operand)
                .unwrap();
            let program = doc.ladder_programs().remove(0).unwrap();
            let layout = program.iec_circuit_layout().unwrap();
            assert_eq!(layout.open_branch_endpoints.len(), 1 - i);
            assert_eq!(
                layout.function_bindings.len(),
                original[0]
                    .iec_circuit_layout()
                    .unwrap()
                    .function_bindings
                    .len()
            );
            let saved = doc.to_bytes().unwrap();
            assert!(
                doc.insert_iec_ld_single_element(0, *row, *x, "coil", "OUTPUT", operand)
                    .is_err()
            );
            assert_eq!(doc.to_bytes().unwrap(), saved);
        }
        for (i, (row, _, operand)) in order.iter().enumerate() {
            let program = doc.ladder_programs().remove(0).unwrap();
            let coil = program
                .iec_record_frames()
                .unwrap()
                .into_iter()
                .find(|r| r.row_index == *row && r.kind == crate::IecRecordKind::Coil(14))
                .unwrap();
            let saved = doc.to_bytes().unwrap();
            assert!(
                doc.delete_iec_ld_terminal_coil(0, coil.offset, "%MX9")
                    .is_err()
            );
            assert_eq!(doc.to_bytes().unwrap(), saved);
            doc.delete_iec_ld_terminal_coil(0, coil.offset, operand)
                .unwrap();
            assert_eq!(
                doc.ladder_programs()
                    .remove(0)
                    .unwrap()
                    .iec_circuit_layout()
                    .unwrap()
                    .open_branch_endpoints
                    .len(),
                i + 1
            );
        }
        for (p, old) in original.iter().enumerate() {
            assert_eq!(doc.ladder_programs().remove(p).unwrap().data, old.data);
        }
    }
    let mut contact_wire =
        XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    contact_wire
        .extend_iec_ld_vertical_wire(0, 3, 4, 3)
        .unwrap();
    contact_wire
        .extend_iec_ld_vertical_wire(0, 8, 12, 24)
        .unwrap();
    let contact_before = contact_wire.to_bytes().unwrap();
    assert!(
        contact_wire
            .insert_iec_ld_single_element(0, 5, 1, "contact", "NO", "ON")
            .is_err()
    );
    assert_eq!(contact_wire.to_bytes().unwrap(), contact_before);
    for (row, x, operand) in [
        (5, 4, "%MX0"),
        (13, 22, "%MX1"),
        (5, 7, "%MW0"),
        (13, 25, "TRUE"),
    ] {
        let mut bad = wire.clone();
        let before = bad.to_bytes().unwrap();
        assert!(
            bad.insert_iec_ld_single_element(0, row, x, "coil", "OUTPUT", operand)
                .is_err()
        );
        assert_eq!(bad.to_bytes().unwrap(), before);
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to an IEC project"]
fn xgi_terminal_timer_short_wire_delete_preserves_declarations() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap();
    let mut doc = XgwxDocument::from_path(source).unwrap();
    let programs = doc
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let symbols = doc
        .iec_local_symbols()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let block = programs[4]
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 15 && b.name.value == "TON")
        .unwrap();
    assert!(block.instance.is_some());
    let site = programs[4]
        .iec_terminal_function_deletion_sites()
        .unwrap()
        .into_iter()
        .find(|s| s.block_offset == block.record_offset)
        .unwrap();
    assert!(
        programs[4]
            .iec_record_frames()
            .unwrap()
            .into_iter()
            .any(|r| r.offset == site.wire_offset && r.kind == crate::IecRecordKind::ShortWire)
    );
    let snapshot = doc.to_bytes().unwrap();
    assert!(
        doc.delete_iec_ld_terminal_function(4, block.record_offset, "UNKNOWN")
            .is_err()
    );
    assert_eq!(doc.to_bytes().unwrap(), snapshot);
    doc.delete_iec_ld_terminal_function(4, block.record_offset, "TON")
        .unwrap();
    let after = doc
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    for (i, (old, new)) in programs.iter().zip(&after).enumerate() {
        if i != 4 {
            assert_eq!(old.data, new.data);
        }
    }
    assert_eq!(
        doc.iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>(),
        symbols
    );
    let rows = after[4].iec_row_frames().unwrap();
    assert_eq!(
        rows.iter()
            .find(|r| r.row_index == 15)
            .unwrap()
            .record_count,
        1
    );
    assert!(!rows.iter().any(|r| r.row_index == 16 || r.row_index == 17));
    assert_eq!(
        after[4].iec_function_blocks().unwrap().len() + 1,
        programs[4].iec_function_blocks().unwrap().len()
    );
    assert_eq!(
        after[4]
            .iec_circuit_graph()
            .unwrap()
            .function_bindings
            .len()
            + 2,
        programs[4]
            .iec_circuit_graph()
            .unwrap()
            .function_bindings
            .len()
    );
    for old in programs[4]
        .iec_row_frames()
        .unwrap()
        .into_iter()
        .filter(|r| r.row_index > 17)
    {
        let new = rows.iter().find(|r| r.row_index == old.row_index).unwrap();
        assert_eq!(
            &programs[4].data[old.start..old.end],
            &after[4].data[new.start..new.end]
        );
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to an IEC project"]
fn xgi_terminal_timer_restoration_is_exact_and_typed() {
    let path = env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap();
    let mut doc = XgwxDocument::from_path(path).unwrap();
    let before = doc
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let locals = doc
        .iec_local_symbols()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let block = before[4]
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 15 && b.name.value == "TON")
        .unwrap();
    doc.delete_iec_ld_terminal_function(4, block.record_offset, "TON")
        .unwrap();
    let site = doc
        .ladder_programs()
        .into_iter()
        .nth(4)
        .unwrap()
        .unwrap()
        .iec_terminal_timer_insertion_sites()
        .unwrap()
        .into_iter()
        .find(|s| s.row_index == 15)
        .unwrap();
    let deleted = doc.to_bytes().unwrap();
    for (instance, preset, elapsed) in [
        ("UNKNOWN", "시간", "카운터"),
        ("Timer", "가스", "카운터"),
        ("Timer", "시간", "TRUE"),
        ("Timer", "시간", "T#1s"),
        ("Timer", "시간", "E0"),
        ("Timer", "UNKNOWN", "카운터"),
    ] {
        assert!(
            doc.insert_iec_ld_terminal_timer(4, site.contact_offset, instance, preset, elapsed)
                .is_err()
        );
        assert_eq!(doc.to_bytes().unwrap(), deleted);
    }
    doc.insert_iec_ld_terminal_timer(4, site.contact_offset, "timer", "시간", "카운터")
        .unwrap();
    for (a, b) in before
        .iter()
        .zip(doc.ladder_programs().into_iter().map(Result::unwrap))
    {
        assert_eq!(a.data, b.data);
    }
    assert_eq!(
        locals,
        doc.iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    );
    let restored = doc.to_bytes().unwrap();
    assert!(
        doc.insert_iec_ld_terminal_timer(4, site.contact_offset, "Timer", "시간", "카운터")
            .is_err()
    );
    assert_eq!(doc.to_bytes().unwrap(), restored);
    let mut literal = XgwxDocument::parse(&deleted).unwrap();
    literal
        .insert_iec_ld_terminal_timer(4, site.contact_offset, "Timer", "T#2s", "카운터")
        .unwrap();
    let b = literal
        .ladder_programs()
        .into_iter()
        .nth(4)
        .unwrap()
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 15)
        .unwrap();
    literal
        .delete_iec_ld_terminal_function(4, b.record_offset, "TON")
        .unwrap();
    assert_eq!(literal.to_bytes().unwrap(), deleted);
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to an IEC project"]
fn xgi_shared_scalar_chain_delete_and_refill_preserve_neighbors() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let original = source
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let locals = source
        .iec_local_symbols()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    for (x, name, args) in [
        (4, "MOVE", vec!["%MW10".into(), "메모리값".into()]),
        (
            13,
            "INT_TO_UDINT",
            vec!["메모리값".into(), "int변경".into()],
        ),
    ] {
        let b = original[4]
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == 18 && b.raw_x == x)
            .unwrap();
        let mut doc = source.clone();
        let bytes = doc.to_bytes().unwrap();
        assert!(
            doc.delete_iec_ld_scalar_chain_function(4, b.record_offset, "UNKNOWN")
                .is_err()
        );
        assert_eq!(doc.to_bytes().unwrap(), bytes);
        doc.delete_iec_ld_scalar_chain_function(4, b.record_offset, name)
            .unwrap();
        let programs = doc
            .ladder_programs()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        for (p, (a, b)) in original.iter().zip(&programs).enumerate() {
            if p != 4 {
                assert_eq!(a.data, b.data);
            }
        }
        assert_eq!(
            locals,
            doc.iec_local_symbols()
                .into_iter()
                .map(Result::unwrap)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            programs[4].iec_function_blocks().unwrap().len() + 1,
            original[4].iec_function_blocks().unwrap().len()
        );
        assert_eq!(
            programs[4]
                .iec_row_frames()
                .unwrap()
                .iter()
                .map(|r| r.row_index)
                .collect::<Vec<_>>(),
            original[4]
                .iec_row_frames()
                .unwrap()
                .iter()
                .map(|r| r.row_index)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            programs[4]
                .iec_circuit_graph()
                .unwrap()
                .function_bindings
                .len()
                + 2,
            original[4]
                .iec_circuit_graph()
                .unwrap()
                .function_bindings
                .len()
        );
        let deleted = doc.to_bytes().unwrap();
        if x == 13 {
            assert!(
                doc.insert_iec_ld_function(4, 18, 13, name, &["가스".into(), "int변경".into()])
                    .is_err()
            );
            assert_eq!(doc.to_bytes().unwrap(), deleted);
        }
        doc.insert_iec_ld_function(4, 18, x, name, &args).unwrap();
        let restored = doc
            .ladder_programs()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        assert_eq!(
            restored[4]
                .iec_function_blocks()
                .unwrap()
                .iter()
                .map(|b| (b.name.value.as_str(), b.raw_x))
                .collect::<Vec<_>>(),
            original[4]
                .iec_function_blocks()
                .unwrap()
                .iter()
                .map(|b| (b.name.value.as_str(), b.raw_x))
                .collect::<Vec<_>>()
        );
        for (a, b) in original.iter().zip(&restored) {
            assert_eq!(a.data, b.data);
        }
        let new = restored[4]
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == 18 && b.raw_x == x)
            .unwrap();
        doc.delete_iec_ld_scalar_chain_function(4, new.record_offset, name)
            .unwrap();
        assert_eq!(doc.to_bytes().unwrap(), deleted);
        assert!(
            source
                .clone()
                .delete_iec_ld_scalar_chain_function(
                    4,
                    original[4]
                        .iec_function_blocks()
                        .unwrap()
                        .into_iter()
                        .find(|b| b.row_index == 15)
                        .unwrap()
                        .record_offset,
                    "TON"
                )
                .is_err()
        );
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE to an IEC project"]
fn xgi_mixed_height_scalar_chains_restore_exactly() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let original = source
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    for (row, x, name, operands, empty_row) in [
        (22, 4, "MUL", vec!["int변경", "1000", "udint변경"], Some(25)),
        (22, 13, "UDINT_TO_TIME", vec!["udint변경", "시간"], None),
        (30, 4, "TIME_TO_UDINT", vec!["카운터", "time변경"], None),
        (30, 13, "DIV", vec!["time변경", "1000", "div_값"], Some(33)),
        (34, 4, "UDINT_TO_INT", vec!["div_값", "밀리변경"], None),
    ] {
        let block = original[4]
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row && b.raw_x == x)
            .unwrap();
        let mut doc = source.clone();
        doc.delete_iec_ld_scalar_chain_function(4, block.record_offset, name)
            .unwrap();
        let deleted = doc
            .ladder_programs()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        for (p, (a, b)) in original.iter().zip(&deleted).enumerate() {
            if p != 4 {
                assert_eq!(a.data, b.data);
            }
        }
        let indices = deleted[4]
            .iec_row_frames()
            .unwrap()
            .iter()
            .map(|r| r.row_index)
            .collect::<Vec<_>>();
        assert_eq!(
            indices,
            original[4]
                .iec_row_frames()
                .unwrap()
                .iter()
                .map(|r| r.row_index)
                .filter(|r| Some(*r) != empty_row)
                .collect::<Vec<_>>()
        );
        if name == "UDINT_TO_TIME" {
            let bytes = doc.to_bytes().unwrap();
            assert!(
                doc.insert_iec_ld_function(4, row, x, name, &["메모리값".into(), "시간".into()])
                    .is_err()
            );
            assert!(
                doc.insert_iec_ld_function(4, row, x, name, &["udint변경".into(), "T#1s".into()])
                    .is_err()
            );
            assert_eq!(doc.to_bytes().unwrap(), bytes);
        }
        doc.insert_iec_ld_function(
            4,
            row,
            x,
            name,
            &operands.into_iter().map(String::from).collect::<Vec<_>>(),
        )
        .unwrap();
        for (a, b) in original
            .iter()
            .zip(doc.ladder_programs().into_iter().map(Result::unwrap))
        {
            assert_eq!(a.data, b.data, "{name} at L{row} x{x}");
        }
        assert_eq!(
            source
                .iec_local_symbols()
                .into_iter()
                .map(Result::unwrap)
                .collect::<Vec<_>>(),
            doc.iec_local_symbols()
                .into_iter()
                .map(Result::unwrap)
                .collect::<Vec<_>>()
        );
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_last_scalar_chain_block_deletion_preserves_remaining_programs() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("fixture path");
    for (row, tail_name, last_name, pin_count) in [
        (22, "UDINT_TO_TIME", "MUL", 3),
        (30, "DIV", "TIME_TO_UDINT", 2),
        (41, "", "GE", 3),
    ] {
        let mut document = XgwxDocument::from_path(&source).unwrap();
        let original = document
            .ladder_programs()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        if !tail_name.is_empty() {
            let tail = original[4]
                .iec_function_blocks()
                .unwrap()
                .into_iter()
                .find(|b| b.row_index == row && b.name.value == tail_name)
                .unwrap();
            document
                .delete_iec_ld_scalar_chain_function(4, tail.record_offset, tail_name)
                .unwrap();
        }
        let program = document
            .ladder_programs()
            .into_iter()
            .nth(4)
            .unwrap()
            .unwrap();
        let last = program
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row && b.name.value == last_name)
            .unwrap();
        assert_eq!(last.pin_count, pin_count);
        assert!(
            program
                .iec_standalone_function_deletion_sites()
                .unwrap()
                .iter()
                .any(|s| s.block_offset == last.record_offset)
        );
        let before = document.to_bytes().unwrap();
        assert!(
            document
                .delete_iec_ld_standalone_function(4, last.record_offset, "stale")
                .is_err()
        );
        assert_eq!(document.to_bytes().unwrap(), before);
        document
            .delete_iec_ld_standalone_function(4, last.record_offset, last_name)
            .unwrap();
        let after = document
            .ladder_programs()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(after[4].iec_circuit_graph().is_some());
        assert_eq!(
            u16::from_le_bytes(after[4].data[4..6].try_into().unwrap()),
            if tail_name.is_empty() { 41 } else { 45 }
        );
        assert_eq!(
            after[4].iec_function_blocks().unwrap().len(),
            original[4].iec_function_blocks().unwrap().len()
                - if tail_name.is_empty() { 1 } else { 2 }
        );
        assert_eq!(
            after[4]
                .iec_row_frames()
                .unwrap()
                .iter()
                .map(|r| r.row_index)
                .collect::<Vec<_>>(),
            original[4]
                .iec_row_frames()
                .unwrap()
                .iter()
                .map(|r| r.row_index)
                .filter(|r| !(*r >= row && *r <= row + 3))
                .collect::<Vec<_>>()
        );
        for i in [0, 1, 2, 3, 5, 6] {
            assert_eq!(after[i].data, original[i].data);
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_staggered_move_deletion_preserves_neighbor_records() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("fixture path");
    for row in [22, 28, 53] {
        let mut document = XgwxDocument::from_path(&source).unwrap();
        let original = document
            .ladder_programs()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let block = original[0]
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row && b.raw_x == 4 && b.name.value == "MOVE")
            .unwrap();
        let neighbor = original[0]
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.group_index == block.group_index && b.raw_x == 16)
            .unwrap();
        let unchanged = document.to_bytes().unwrap();
        assert!(
            document
                .delete_iec_ld_scalar_chain_function(
                    0,
                    neighbor.record_offset,
                    &neighbor.name.value
                )
                .is_err()
        );
        assert_eq!(document.to_bytes().unwrap(), unchanged);
        let mut removed = vec![block.record_offset];
        removed.extend(
            original[0]
                .iec_function_operand_links()
                .unwrap()
                .into_iter()
                .filter(|l| l.target_record_offset == block.record_offset)
                .map(|l| l.record_offset),
        );
        removed.extend(
            original[0]
                .iec_function_references()
                .unwrap()
                .into_iter()
                .filter(|r| r.target_record_offset == block.record_offset)
                .map(|r| r.record_offset),
        );
        let records = original[0].iec_record_frames().unwrap();
        document
            .delete_iec_ld_scalar_chain_function(0, block.record_offset, "MOVE")
            .unwrap();
        let after = document
            .ladder_programs()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(after[0].iec_circuit_graph().is_some());
        let retained = after[0].iec_record_frames().unwrap();
        assert_eq!(records.len(), retained.len() + removed.len());
        for (before, after_record) in records
            .iter()
            .filter(|r| !removed.contains(&r.offset))
            .zip(retained.iter())
        {
            assert_eq!(
                (before.row_index, before.group_index, before.kind),
                (
                    after_record.row_index,
                    after_record.group_index,
                    after_record.kind
                )
            );
            assert_eq!(
                &original[0].data[before.offset..before.end],
                &after[0].data[after_record.offset..after_record.end]
            );
        }
        for index in 1..original.len() {
            assert_eq!(original[index].data, after[index].data);
        }
        assert_eq!(original[0].data[4..8], after[0].data[4..8]);
        document
            .insert_iec_ld_function(0, row, 4, "MOVE", &["%MW700".into(), "%MW200".into()])
            .unwrap();
        let restored = document.ladder_programs().remove(0).unwrap();
        assert!(restored.iec_circuit_graph().is_some());
        assert_eq!(
            restored.iec_function_blocks().unwrap().len(),
            original[0].iec_function_blocks().unwrap().len()
        );
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_completed_branch_tail_delete_refill_and_replace() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("fixture path");
    for (index, row, expected) in [
        (5, 19, "SUB"),
        (6, 34, "SUB"),
        (6, 42, "GT"),
        (5, 36, "GT"),
        (0, 79, "EQ"),
        (0, 87, "MOVE"),
        (6, 74, "EQ"),
    ] {
        let document = XgwxDocument::from_path(&source).unwrap();
        let original = document
            .ladder_programs()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let program = &original[index];
        let block = program
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row && b.name.value == expected)
            .unwrap();
        let links = program.iec_function_operand_links().unwrap();
        let refs = program.iec_function_references().unwrap();
        let records = program.iec_record_frames().unwrap();
        let removed = records
            .iter()
            .filter(|r| {
                r.group_index == block.group_index
                    && ((r.row_index == row
                        && matches!(
                            r.kind,
                            crate::IecRecordKind::ShortWire
                                | crate::IecRecordKind::LongWire
                                | crate::IecRecordKind::FunctionBlock
                        ))
                        || r.row_index > row)
            })
            .map(|r| r.offset)
            .collect::<Vec<_>>();
        assert_eq!(
            removed.len(),
            1 + 2 * usize::from(block.pin_count)
                + records
                    .iter()
                    .filter(|r| r.group_index == block.group_index
                        && r.row_index == row
                        && matches!(
                            r.kind,
                            crate::IecRecordKind::ShortWire | crate::IecRecordKind::LongWire
                        ))
                    .count()
        );
        let mut deleted = document.clone();
        deleted
            .delete_iec_ld_branch_function(index, block.record_offset, expected)
            .unwrap();
        let after = deleted
            .ladder_programs()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let retained = after[index].iec_record_frames().unwrap();
        for (before, after_record) in records
            .iter()
            .filter(|r| !removed.contains(&r.offset))
            .zip(retained.iter())
        {
            assert_eq!(
                (before.group_index, before.row_index, before.kind),
                (
                    after_record.group_index,
                    after_record.row_index,
                    after_record.kind
                )
            );
            assert_eq!(
                &program.data[before.offset..before.end],
                &after[index].data[after_record.offset..after_record.end]
            );
        }
        assert_eq!(records.len(), retained.len() + removed.len());
        assert_eq!(
            after[index].iec_function_operand_links().unwrap().len() + usize::from(block.pin_count),
            links.len()
        );
        assert_eq!(
            after[index].iec_function_references().unwrap().len() + usize::from(block.pin_count),
            refs.len()
        );
        assert_eq!(
            after[index]
                .iec_circuit_layout()
                .unwrap()
                .open_branch_endpoints
                .len(),
            1
        );
        for p in 0..original.len() {
            if p != index {
                assert_eq!(original[p].data, after[p].data);
            }
        }
        for name in [
            "MOVE", "ADD", "SUB", "MUL", "DIV", "EQ", "GT", "GE", "LT", "LE",
        ] {
            let operands = if name == "MOVE" {
                vec!["1".into(), "%MW414".into()]
            } else {
                vec![
                    "1".into(),
                    "2".into(),
                    if ["EQ", "GT", "GE", "LT", "LE"].contains(&name) {
                        "%MX32".into()
                    } else {
                        "%MW414".into()
                    },
                ]
            };
            let mut candidate = document.clone();
            candidate
                .replace_iec_ld_branch_function(
                    index,
                    block.record_offset,
                    expected,
                    name,
                    &operands,
                )
                .unwrap();
            let result = candidate.ladder_programs().remove(index).unwrap();
            assert!(result.iec_circuit_graph().is_some());
            assert!(
                result
                    .iec_circuit_layout()
                    .unwrap()
                    .open_branch_endpoints
                    .is_empty()
            );
            let tail = result
                .iec_function_blocks()
                .unwrap()
                .into_iter()
                .find(|b| b.row_index == row && b.raw_x == block.raw_x)
                .unwrap();
            candidate
                .delete_iec_ld_branch_function(index, tail.record_offset, name)
                .unwrap();
            assert!(
                candidate.to_bytes().unwrap() == deleted.to_bytes().unwrap(),
                "program {index}, row {row}, {expected} -> {name} -> Delete differs"
            );
        }
        let mut rejected = document.clone();
        assert!(
            rejected
                .replace_iec_ld_branch_function(
                    index,
                    block.record_offset,
                    expected,
                    "EQ",
                    &["1".into(), "2".into(), "%MW414".into()]
                )
                .is_err()
        );
        assert_eq!(rejected.to_bytes().unwrap(), document.to_bytes().unwrap());
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_contact_scalar_tail_delete_refill_and_replace() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("fixture path");
    for (index, row, x, output) in [(5, 43, 13, "%MW416"), (1, 6, 19, "%MW301")] {
        let document = XgwxDocument::from_path(&source).unwrap();
        let original = document
            .ladder_programs()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let block = original[index]
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row && b.raw_x == x)
            .unwrap();
        let mut deleted = document.clone();
        deleted
            .delete_iec_ld_branch_function(index, block.record_offset, "MOVE")
            .unwrap();
        let program = deleted.ladder_programs().remove(index).unwrap();
        let before = original[index].iec_record_frames().unwrap();
        let after = program.iec_record_frames().unwrap();
        let survivors = before
            .iter()
            .filter(|r| {
                r.group_index != block.group_index
                    || r.row_index < row
                    || (r.row_index == row
                        && matches!(
                            r.kind,
                            crate::IecRecordKind::Contact(_) | crate::IecRecordKind::BranchEnd
                        ))
            })
            .collect::<Vec<_>>();
        assert_eq!(survivors.len(), after.len());
        for (a, b) in survivors.iter().zip(&after) {
            assert_eq!(
                (a.group_index, a.row_index, a.kind),
                (b.group_index, b.row_index, b.kind)
            );
            assert_eq!(
                &original[index].data[a.offset..a.end],
                &program.data[b.offset..b.end]
            );
        }
        for name in [
            "MOVE", "ADD", "SUB", "MUL", "DIV", "EQ", "GT", "GE", "LT", "LE",
        ] {
            let operands = if name == "MOVE" {
                vec!["0".into(), output.into()]
            } else {
                vec![
                    "1".into(),
                    "2".into(),
                    if ["EQ", "GT", "GE", "LT", "LE"].contains(&name) {
                        "%MX32".into()
                    } else {
                        output.into()
                    },
                ]
            };
            let mut candidate = document.clone();
            candidate
                .replace_iec_ld_branch_function(index, block.record_offset, "MOVE", name, &operands)
                .unwrap();
            let programs = candidate
                .ladder_programs()
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert!(programs[index].iec_circuit_graph().is_some());
            assert!(
                programs[index]
                    .iec_circuit_layout()
                    .unwrap()
                    .open_branch_endpoints
                    .is_empty()
            );
            for p in 0..original.len() {
                if p != index {
                    assert_eq!(original[p].data, programs[p].data);
                }
            }
            let new_block = programs[index]
                .iec_function_blocks()
                .unwrap()
                .into_iter()
                .find(|b| b.row_index == row && b.raw_x == x)
                .unwrap();
            candidate
                .delete_iec_ld_branch_function(index, new_block.record_offset, name)
                .unwrap();
            if name == "MOVE" || index == 5 {
                assert_eq!(candidate.to_bytes().unwrap(), deleted.to_bytes().unwrap());
            } else {
                // A three-pin replacement shifts the next curtain network by
                // one native blank row. Its records must otherwise survive.
                let shifted = candidate.ladder_programs().remove(index).unwrap();
                let shifted_records = shifted.iec_record_frames().unwrap();
                for (a, b) in after.iter().zip(&shifted_records) {
                    assert_eq!(a.kind, b.kind);
                    assert_eq!(b.row_index, a.row_index + u16::from(a.row_index > row));
                }
                assert_eq!(after.len(), shifted_records.len());
            }
        }
        let mut rejected = document.clone();
        assert!(
            rejected
                .replace_iec_ld_branch_function(
                    index,
                    block.record_offset,
                    "MOVE",
                    "EQ",
                    &["1".into(), "2".into(), output.into()]
                )
                .is_err()
        );
        assert_eq!(rejected.to_bytes().unwrap(), document.to_bytes().unwrap());
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_continuing_branch_scalar_delete_refill_and_replace() {
    let source = env::var("LIBXGWX_SMARTHOME_FIXTURE").expect("fixture path");
    for (index, row, x, expected) in [
        (5, 32, 13, "LT"),
        (0, 71, 16, "EQ"),
        (0, 75, 16, "EQ"),
        (6, 66, 19, "EQ"),
        (6, 70, 19, "EQ"),
        (5, 24, 13, "LE"),
        (6, 38, 13, "LT"),
        (5, 40, 13, "MOVE"),
        (5, 28, 13, "GE"),
    ] {
        let document = XgwxDocument::from_path(&source).unwrap();
        let original = document
            .ladder_programs()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let program = &original[index];
        let block = program
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row && b.raw_x == x && b.name.value == expected)
            .unwrap();
        let records = program.iec_record_frames().unwrap();
        let links = program.iec_function_operand_links().unwrap();
        let refs = program.iec_function_references().unwrap();
        let mut removed = vec![block.record_offset];
        removed.extend(
            records
                .iter()
                .filter(|r| {
                    r.group_index == block.group_index
                        && r.row_index == row
                        && matches!(
                            r.kind,
                            crate::IecRecordKind::LongWire | crate::IecRecordKind::ShortWire
                        )
                })
                .map(|r| r.offset),
        );
        removed.extend(
            links
                .iter()
                .filter(|l| l.target_record_offset == block.record_offset)
                .map(|l| l.record_offset),
        );
        removed.extend(
            refs.iter()
                .filter(|r| r.target_record_offset == block.record_offset)
                .map(|r| r.record_offset),
        );
        let mut deleted = document.clone();
        deleted
            .delete_iec_ld_branch_function(index, block.record_offset, expected)
            .unwrap();
        let after = deleted.ladder_programs().remove(index).unwrap();
        assert_eq!(
            program
                .iec_row_frames()
                .unwrap()
                .iter()
                .map(|r| r.row_index)
                .collect::<Vec<_>>(),
            after
                .iec_row_frames()
                .unwrap()
                .iter()
                .map(|r| r.row_index)
                .collect::<Vec<_>>()
        );
        let remaining = after.iec_record_frames().unwrap();
        assert_eq!(records.len(), remaining.len() + removed.len());
        for (a, b) in records
            .iter()
            .filter(|r| !removed.contains(&r.offset))
            .zip(&remaining)
        {
            assert_eq!(
                (a.group_index, a.row_index, a.kind),
                (b.group_index, b.row_index, b.kind)
            );
            assert_eq!(&program.data[a.offset..a.end], &after.data[b.offset..b.end]);
        }
        let mut expanded_recovery = None;
        for name in [
            "MOVE", "ADD", "SUB", "MUL", "DIV", "EQ", "GT", "GE", "LT", "LE",
        ] {
            let operands = if name == "MOVE" {
                vec!["0".into(), "%MW416".into()]
            } else {
                vec![
                    "1".into(),
                    "2".into(),
                    if ["EQ", "GT", "GE", "LT", "LE"].contains(&name) {
                        "%MX32".into()
                    } else {
                        "%MW416".into()
                    },
                ]
            };
            let mut candidate = document.clone();
            candidate
                .replace_iec_ld_branch_function(
                    index,
                    block.record_offset,
                    expected,
                    name,
                    &operands,
                )
                .unwrap();
            let programs = candidate
                .ladder_programs()
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            for p in 0..original.len() {
                if p != index {
                    assert_eq!(original[p].data, programs[p].data);
                }
            }
            assert!(programs[index].iec_circuit_graph().is_some());
            let replacement = programs[index]
                .iec_function_blocks()
                .unwrap()
                .into_iter()
                .find(|b| b.row_index == row && b.raw_x == x)
                .unwrap();
            assert_eq!(
                programs[index].data[replacement.record_offset + 11],
                if index == 0 { 4 } else { 0 }
            );
            candidate
                .delete_iec_ld_branch_function(index, replacement.record_offset, name)
                .unwrap();
            if index == 5 && row == 40 && name != "MOVE" {
                let recovered = candidate.ladder_programs().remove(index).unwrap();
                assert_eq!(
                    recovered.iec_row_frames().unwrap().len(),
                    after.iec_row_frames().unwrap().len() + 1
                );
                let lower = recovered
                    .iec_function_blocks()
                    .unwrap()
                    .into_iter()
                    .find(|b| b.row_index == 44)
                    .unwrap();
                assert_eq!((lower.name.value.as_str(), lower.raw_x), ("MOVE", 13));
                assert!(recovered.iec_circuit_graph().is_some());
                let bytes = candidate.to_bytes().unwrap();
                if let Some(expected) = &expanded_recovery {
                    assert_eq!(&bytes, expected, "{name} recovery");
                } else {
                    expanded_recovery = Some(bytes);
                }
            } else {
                assert_eq!(
                    candidate.to_bytes().unwrap(),
                    deleted.to_bytes().unwrap(),
                    "{index} L{row} {name}"
                );
            }
        }
        let mut rejected = document.clone();
        assert!(
            rejected
                .replace_iec_ld_branch_function(
                    index,
                    block.record_offset,
                    expected,
                    "EQ",
                    &["1".into(), "2".into(), "%MW416".into()]
                )
                .is_err()
        );
        assert_eq!(rejected.to_bytes().unwrap(), document.to_bytes().unwrap());
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_trigger_scalar_tail_delete_refill_and_replace() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let original = source
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let block = original[0]
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 32 && b.raw_x == 19)
        .unwrap();
    let trigger = original[0]
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 32 && b.raw_x == 10)
        .unwrap();
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_scalar_chain_function(0, block.record_offset, "ADD")
        .unwrap();
    let baseline = deleted.to_bytes().unwrap();
    let after = deleted.ladder_programs().remove(0).unwrap();
    assert_eq!(
        after
            .iec_row_frames()
            .unwrap()
            .iter()
            .map(|r| r.row_index)
            .collect::<Vec<_>>(),
        original[0]
            .iec_row_frames()
            .unwrap()
            .iter()
            .filter(|r| ![34, 35].contains(&r.row_index))
            .map(|r| r.row_index)
            .collect::<Vec<_>>()
    );
    let retained_trigger = after
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 32 && b.raw_x == 10)
        .unwrap();
    assert_eq!(retained_trigger.instance, trigger.instance);
    let trigger_record = original[0]
        .iec_record_frames()
        .unwrap()
        .into_iter()
        .find(|r| r.offset == trigger.record_offset)
        .unwrap();
    let mut expected_trigger = original[0].data[trigger_record.offset..trigger_record.end].to_vec();
    expected_trigger[12] = 2;
    let retained_record = after
        .iec_record_frames()
        .unwrap()
        .into_iter()
        .find(|r| r.offset == retained_trigger.record_offset)
        .unwrap();
    assert_eq!(
        expected_trigger,
        after.data[retained_record.offset..retained_record.end]
    );
    for name in [
        "MOVE", "ADD", "SUB", "MUL", "DIV", "EQ", "GT", "GE", "LT", "LE",
    ] {
        let operands = if name == "MOVE" {
            vec!["0".into(), "%MW600".into()]
        } else {
            vec![
                "1".into(),
                "2".into(),
                if ["EQ", "GT", "GE", "LT", "LE"].contains(&name) {
                    "%MX32".into()
                } else {
                    "%MW600".into()
                },
            ]
        };
        let mut candidate = source.clone();
        candidate
            .replace_iec_ld_scalar_chain_function(0, block.record_offset, "ADD", name, &operands)
            .unwrap();
        let programs = candidate
            .ladder_programs()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        assert!(programs[0].iec_circuit_graph().is_some());
        for p in 1..7 {
            assert_eq!(programs[p].data, original[p].data);
        }
        assert_eq!(
            candidate
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
        let target = programs[0]
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == 32 && b.raw_x == 19)
            .unwrap();
        assert_eq!(target.name.value, name);
        candidate
            .delete_iec_ld_scalar_chain_function(0, target.record_offset, name)
            .unwrap();
        assert_eq!(candidate.to_bytes().unwrap(), baseline, "{name} recovery");
    }
    for (name, operands) in [
        ("EQ", vec!["1".into(), "2".into(), "%MW600".into()]),
        ("ADD", vec!["1".into(), "2".into(), "0".into()]),
        ("INT_TO_UDINT", vec!["1".into(), "%MD600".into()]),
    ] {
        let mut rejected = source.clone();
        assert!(
            rejected
                .replace_iec_ld_scalar_chain_function(
                    0,
                    block.record_offset,
                    "ADD",
                    name,
                    &operands
                )
                .is_err()
        );
        assert_eq!(rejected.to_bytes().unwrap(), source.to_bytes().unwrap());
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_comparison_result_move_delete_and_refill() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let original = source
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let block = original[0]
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 38 && b.raw_x == 19 && b.name.value == "MOVE")
        .unwrap();
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_scalar_chain_function(0, block.record_offset, "MOVE")
        .unwrap();
    let baseline = deleted.to_bytes().unwrap();
    let scaffold = deleted.ladder_programs().remove(0).unwrap();
    assert!(scaffold.iec_circuit_graph().is_some());
    assert_eq!(
        scaffold
            .iec_row_frames()
            .unwrap()
            .iter()
            .map(|r| r.row_index)
            .collect::<Vec<_>>(),
        original[0]
            .iec_row_frames()
            .unwrap()
            .iter()
            .map(|r| r.row_index)
            .collect::<Vec<_>>()
    );
    let eq = scaffold
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 37 && b.raw_x == 10)
        .unwrap();
    assert_eq!(scaffold.data[eq.record_offset + 12], 2);
    let links = scaffold.iec_function_operand_links().unwrap();
    assert_eq!(
        links
            .iter()
            .filter(|l| l.target_record_offset == eq.record_offset)
            .count(),
        2
    );
    assert!(
        !links
            .iter()
            .any(|l| l.target_record_offset == eq.record_offset && l.is_output)
    );
    for operands in [
        vec!["0".into(), "%MW600".into()],
        vec!["12".into(), "%MW602".into()],
    ] {
        let mut candidate = source.clone();
        candidate
            .replace_iec_ld_scalar_chain_function(0, block.record_offset, "MOVE", "MOVE", &operands)
            .unwrap();
        let programs = candidate
            .ladder_programs()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        for p in 1..7 {
            assert_eq!(programs[p].data, original[p].data);
        }
        let restored = programs[0]
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == 38 && b.raw_x == 19)
            .unwrap();
        assert!(programs[0].iec_circuit_graph().is_some());
        let eq = programs[0]
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == 37 && b.raw_x == 10)
            .unwrap();
        assert_eq!(programs[0].data[eq.record_offset + 12], 0);
        candidate
            .delete_iec_ld_scalar_chain_function(0, restored.record_offset, "MOVE")
            .unwrap();
        assert_eq!(candidate.to_bytes().unwrap(), baseline);
    }
    for (name, operands) in [
        ("ADD", vec!["1".into(), "2".into(), "%MW600".into()]),
        ("EQ", vec!["1".into(), "2".into(), "%MX32".into()]),
        ("INT_TO_UDINT", vec!["1".into(), "%MD600".into()]),
        ("MOVE", vec!["1".into(), "0".into()]),
    ] {
        let mut rejected = source.clone();
        assert!(
            rejected
                .replace_iec_ld_scalar_chain_function(
                    0,
                    block.record_offset,
                    "MOVE",
                    name,
                    &operands
                )
                .is_err()
        );
        assert_eq!(rejected.to_bytes().unwrap(), source.to_bytes().unwrap());
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_wired_comparison_delete_refill_and_replace() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let original = source
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let block = original[0]
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 37 && b.raw_x == 10)
        .unwrap();
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_scalar_chain_function(0, block.record_offset, "EQ")
        .unwrap();
    let baseline = deleted.to_bytes().unwrap();
    let scaffold = deleted.ladder_programs().remove(0).unwrap();
    assert!(scaffold.iec_circuit_graph().is_some());
    assert!(
        !scaffold
            .iec_row_frames()
            .unwrap()
            .iter()
            .any(|r| r.row_index == 37)
    );
    assert!(
        scaffold
            .iec_wired_comparison_insertion_sites()
            .unwrap()
            .iter()
            .any(|s| s.row_index == 37 && s.raw_x == 10)
    );
    for name in ["EQ", "GT", "GE", "LT", "LE"] {
        let mut candidate = source.clone();
        candidate
            .replace_iec_ld_scalar_chain_function(
                0,
                block.record_offset,
                "EQ",
                name,
                &["%MW600".into(), "8".into()],
            )
            .unwrap();
        let programs = candidate
            .ladder_programs()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        for p in 1..7 {
            assert_eq!(programs[p].data, original[p].data);
        }
        assert!(programs[0].iec_circuit_graph().is_some());
        assert_eq!(
            programs[0]
                .iec_row_frames()
                .unwrap()
                .iter()
                .map(|r| r.row_index)
                .collect::<Vec<_>>(),
            original[0]
                .iec_row_frames()
                .unwrap()
                .iter()
                .map(|r| r.row_index)
                .collect::<Vec<_>>()
        );
        let functions = programs[0].iec_function_blocks().unwrap();
        let comparison = functions
            .iter()
            .find(|b| b.row_index == 37 && b.raw_x == 10)
            .unwrap();
        assert_eq!(comparison.name.value, name);
        let links = programs[0].iec_function_operand_links().unwrap();
        assert_eq!(
            links
                .iter()
                .filter(|l| l.target_record_offset == comparison.record_offset)
                .count(),
            2
        );
        assert!(
            !links
                .iter()
                .any(|l| l.target_record_offset == comparison.record_offset && l.is_output)
        );
        let refs = programs[0].iec_function_references().unwrap();
        assert_eq!(
            refs.iter()
                .filter(|r| r.target_record_offset == comparison.record_offset)
                .count(),
            3
        );
        let consumer = functions
            .iter()
            .find(|b| b.row_index == 38 && b.raw_x == 19)
            .unwrap();
        let restored_bytes = candidate.to_bytes().unwrap();
        let mut consumer_deleted = candidate.clone();
        consumer_deleted
            .delete_iec_ld_scalar_chain_function(0, consumer.record_offset, "MOVE")
            .unwrap();
        consumer_deleted
            .insert_iec_ld_function(0, 38, 19, "MOVE", &["0".into(), "%MW600".into()])
            .unwrap();
        assert_eq!(consumer_deleted.to_bytes().unwrap(), restored_bytes);
        candidate
            .delete_iec_ld_scalar_chain_function(0, comparison.record_offset, name)
            .unwrap();
        assert_eq!(candidate.to_bytes().unwrap(), baseline);
    }
    let mut bool_inputs = source.clone();
    bool_inputs
        .replace_iec_ld_scalar_chain_function(
            0,
            block.record_offset,
            "EQ",
            "EQ",
            &["TRUE".into(), "FALSE".into()],
        )
        .unwrap();
    let bool_program = bool_inputs.ladder_programs().remove(0).unwrap();
    assert!(bool_program.iec_circuit_graph().is_some());
    let comparison = bool_program
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 37 && b.raw_x == 10)
        .unwrap();
    bool_inputs
        .delete_iec_ld_scalar_chain_function(0, comparison.record_offset, "EQ")
        .unwrap();
    assert_eq!(bool_inputs.to_bytes().unwrap(), baseline);
    for (name, operands) in [
        ("EQ", vec!["%MW600".into(), "%MX32".into()]),
        ("EQ", vec!["unknown_symbol".into(), "8".into()]),
        ("EQ", vec!["%MW600".into(), "8".into(), "%MX32".into()]),
        ("ADD", vec!["1".into(), "2".into(), "%MW600".into()]),
    ] {
        let mut rejected = source.clone();
        assert!(
            rejected
                .replace_iec_ld_scalar_chain_function(0, block.record_offset, "EQ", name, &operands)
                .is_err()
        );
        assert_eq!(rejected.to_bytes().unwrap(), source.to_bytes().unwrap());
    }
    let mut ordinary = source.clone();
    assert!(
        ordinary
            .insert_iec_ld_function(0, 90, 10, "EQ", &["1".into(), "2".into()])
            .is_err()
    );
    assert_eq!(ordinary.to_bytes().unwrap(), source.to_bytes().unwrap());
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_CURTAIN_TIMER_CAPTURE"]
fn xgi_connected_curtain_timer_delete_and_refill() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_CURTAIN_TIMER_CAPTURE").unwrap());
    let original = source
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let block = original[1]
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 1 && b.raw_x == 19)
        .unwrap();
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_scalar_chain_function(1, block.record_offset, "TON")
        .unwrap();
    let p = deleted.ladder_programs().remove(1).unwrap();
    let site = p
        .iec_terminal_timer_insertion_sites()
        .unwrap()
        .into_iter()
        .find(|s| s.row_index == 1 && s.raw_x == 19)
        .unwrap();
    assert!(!p.iec_row_frames().unwrap().iter().any(|r| r.row_index == 3));
    let group_bytes = |p: &LadderProgramData| {
        let rows = p.iec_row_frames().unwrap();
        let group = rows
            .iter()
            .filter(|r| r.group_index == 1)
            .collect::<Vec<_>>();
        p.data[group[0].start - 10..group.last().unwrap().end].to_vec()
    };
    let native_delete = XgwxDocument::from_path(capture.join("CTD1003.xgwx"))
        .unwrap()
        .ladder_programs()
        .remove(1)
        .unwrap();
    assert_eq!(group_bytes(&p), group_bytes(&native_delete));
    let baseline = deleted.to_bytes().unwrap();
    for (instance, preset, elapsed) in [
        ("UNKNOWN", "T#5s", ""),
        ("INST13", "8", ""),
        ("INST13", "T#5s", "T#1s"),
    ] {
        let mut rejected = deleted.clone();
        assert!(
            rejected
                .insert_iec_ld_terminal_timer(1, site.contact_offset, instance, preset, elapsed)
                .is_err()
        );
        assert_eq!(rejected.to_bytes().unwrap(), baseline);
    }
    let mut refilled = deleted.clone();
    refilled
        .insert_iec_ld_terminal_timer(1, site.contact_offset, "INST13", "T#5s", "")
        .unwrap();
    let restored = refilled.ladder_programs().remove(1).unwrap();
    let native_refill = XgwxDocument::from_path(capture.join("CTR1003.xgwx"))
        .unwrap()
        .ladder_programs()
        .remove(1)
        .unwrap();
    assert_eq!(group_bytes(&restored), group_bytes(&native_refill));
    assert!(restored.iec_circuit_graph().is_some());
    assert_eq!(
        restored
            .iec_row_frames()
            .unwrap()
            .iter()
            .map(|r| r.row_index)
            .collect::<Vec<_>>(),
        original[1]
            .iec_row_frames()
            .unwrap()
            .iter()
            .map(|r| r.row_index)
            .collect::<Vec<_>>()
    );
    for (i, p) in refilled.ladder_programs().into_iter().enumerate() {
        if i != 1 {
            assert_eq!(p.unwrap().data, original[i].data);
        }
    }
    refilled
        .delete_iec_ld_scalar_chain_function(1, block.record_offset, "TON")
        .unwrap();
    assert_eq!(refilled.to_bytes().unwrap(), baseline);
    if let Ok(out) = env::var("LIBXGWX_CURTAIN_TIMER_OUTPUT") {
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(std::path::Path::new(&out).join("CTDELGEN.xgwx"), &baseline).unwrap();
        let mut refill = deleted;
        refill
            .insert_iec_ld_terminal_timer(1, site.contact_offset, "INST13", "T#5s", "")
            .unwrap();
        std::fs::write(
            std::path::Path::new(&out).join("CTRESTGEN.xgwx"),
            refill.to_bytes().unwrap(),
        )
        .unwrap();
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_LONG_FEED_TIMER_CAPTURE"]
fn xgi_long_feed_timer_delete_and_refill() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_LONG_FEED_TIMER_CAPTURE").unwrap());
    let original = source
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    for (index, group, row, instance, prefix) in
        [(2, 6, 13, "INST10", "LTC"), (3, 22, 59, "INST1", "LTE")]
    {
        let group_bytes = |p: &LadderProgramData| {
            let rows = p.iec_row_frames().unwrap();
            let owned = rows
                .iter()
                .filter(|r| r.group_index == group)
                .collect::<Vec<_>>();
            p.data[owned[0].start - 10..owned.last().unwrap().end].to_vec()
        };
        let block = original[index]
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row && b.raw_x == 22)
            .unwrap();
        let mut deleted = source.clone();
        deleted
            .delete_iec_ld_scalar_chain_function(index, block.record_offset, "TON")
            .unwrap();
        let p = deleted.ladder_programs().remove(index).unwrap();
        let native = XgwxDocument::from_path(capture.join(format!("{prefix}DEL3.xgwx")))
            .unwrap()
            .ladder_programs()
            .remove(index)
            .unwrap();
        assert_eq!(
            group_bytes(&p),
            group_bytes(&native),
            "{prefix} native deletion"
        );
        let baseline = deleted.to_bytes().unwrap();
        for args in [
            vec!["UNKNOWN".to_owned(), "T#15s".to_owned()],
            vec![instance.to_owned(), "8".to_owned()],
            vec![instance.to_owned()],
        ] {
            let mut bad = deleted.clone();
            assert!(
                bad.insert_iec_ld_function(index, row, 22, "TON", &args)
                    .is_err()
            );
            assert_eq!(bad.to_bytes().unwrap(), baseline);
        }
        let mut restored = deleted.clone();
        restored
            .insert_iec_ld_function(
                index,
                row,
                22,
                "TON",
                &[instance.to_owned(), "T#15s".to_owned()],
            )
            .unwrap();
        let r = restored.ladder_programs().remove(index).unwrap();
        let native = XgwxDocument::from_path(capture.join(format!("{prefix}REF3.xgwx")))
            .unwrap()
            .ladder_programs()
            .remove(index)
            .unwrap();
        assert_eq!(
            group_bytes(&r),
            group_bytes(&native),
            "{prefix} native refill"
        );
        assert!(r.iec_circuit_graph().is_some());
        for (i, p) in restored.ladder_programs().into_iter().enumerate() {
            if i != index {
                assert_eq!(p.unwrap().data, original[i].data);
            }
        }
        if let Ok(out) = env::var("LIBXGWX_LONG_FEED_TIMER_OUTPUT") {
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(
                std::path::Path::new(&out).join(format!("{prefix}DELGEN.xgwx")),
                &baseline,
            )
            .unwrap();
            std::fs::write(
                std::path::Path::new(&out).join(format!("{prefix}REFGEN.xgwx")),
                restored.to_bytes().unwrap(),
            )
            .unwrap();
        }
        let new_offset = r
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row && b.raw_x == 22)
            .unwrap()
            .record_offset;
        restored
            .delete_iec_ld_scalar_chain_function(index, new_offset, "TON")
            .unwrap();
        assert_eq!(restored.to_bytes().unwrap(), baseline);
    }
}

#[test]
#[cfg(feature = "write")]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_PAIRED_COMPARISON_CAPTURE"]
fn xgi_paired_comparison_delete_and_refill() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_PAIRED_COMPARISON_CAPTURE").unwrap());
    let original = source
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let area = |p: &LadderProgramData, base: u16| {
        let rows = p.iec_row_frames().unwrap();
        let first = rows.iter().find(|r| r.row_index == base).unwrap();
        let last = rows.iter().find(|r| r.row_index == base + 6).unwrap();
        p.data[first.start - 10..last.end].to_vec()
    };
    for (index, base, x, instance, pt1, pt2, prefix) in [
        (2, 16, 7, "INST10.ET", "T#2s", "T#3s", "C7"),
        (2, 23, 4, "INST10.ET", "T#7s", "T#9s", "C4"),
        (3, 62, 7, "INST1.ET", "T#2s", "T#3s", "E7"),
        (3, 68, 4, "INST1.ET", "T#7s", "T#9s", "E4"),
    ] {
        for (head, name, pt, letter) in [(true, "GT", pt1, "G"), (false, "LE", pt2, "L")] {
            let row = base + u16::from(!head);
            let at = x + if head { 0 } else { 9 };
            let block = original[index]
                .iec_function_blocks()
                .unwrap()
                .into_iter()
                .find(|b| b.row_index == row && b.raw_x == at)
                .unwrap();
            let mut deleted = source.clone();
            deleted
                .delete_iec_ld_scalar_chain_function(index, block.record_offset, name)
                .unwrap();
            let p = deleted.ladder_programs().remove(index).unwrap();
            assert!(
                p.iec_wired_comparison_insertion_sites()
                    .unwrap()
                    .iter()
                    .any(|s| s.row_index == row && s.raw_x == at)
            );
            if index == 2 {
                let native =
                    XgwxDocument::from_path(capture.join(format!("C{letter}{}D3.xgwx", x)))
                        .unwrap()
                        .ladder_programs()
                        .remove(index)
                        .unwrap();
                assert_eq!(
                    area(&p, base),
                    area(&native, base),
                    "{prefix} {name} native Delete"
                );
            }
            let baseline = deleted.to_bytes().unwrap();
            for args in [
                vec!["UNKNOWN".to_owned(), pt.to_owned()],
                vec![instance.to_owned(), "8".to_owned()],
                vec![instance.to_owned()],
            ] {
                let mut bad = deleted.clone();
                assert!(
                    bad.insert_iec_ld_function(index, row, at, name, &args)
                        .is_err()
                );
                assert_eq!(bad.to_bytes().unwrap(), baseline);
            }
            let mut restored = deleted.clone();
            restored
                .insert_iec_ld_function(index, row, at, name, &[instance.to_owned(), pt.to_owned()])
                .unwrap();
            let r = restored.ladder_programs().remove(index).unwrap();
            if index == 2 {
                let native =
                    XgwxDocument::from_path(capture.join(format!("C{letter}{}R3.xgwx", x)))
                        .unwrap()
                        .ladder_programs()
                        .remove(index)
                        .unwrap();
                assert_eq!(
                    area(&r, base),
                    area(&native, base),
                    "{prefix} {name} native refill"
                );
            }
            for (i, p) in restored.ladder_programs().into_iter().enumerate() {
                if i != index {
                    assert_eq!(p.unwrap().data, original[i].data);
                }
            }
            if let Ok(out) = env::var("LIBXGWX_PAIRED_COMPARISON_OUTPUT") {
                std::fs::create_dir_all(&out).unwrap();
                std::fs::write(
                    std::path::Path::new(&out).join(format!("{prefix}{letter}DELGEN.xgwx")),
                    &baseline,
                )
                .unwrap();
                std::fs::write(
                    std::path::Path::new(&out).join(format!("{prefix}{letter}REFGEN.xgwx")),
                    restored.to_bytes().unwrap(),
                )
                .unwrap();
            }
            let new_offset = r
                .iec_function_blocks()
                .unwrap()
                .into_iter()
                .find(|b| b.row_index == row && b.raw_x == at)
                .unwrap()
                .record_offset;
            restored
                .delete_iec_ld_scalar_chain_function(index, new_offset, name)
                .unwrap();
            assert_eq!(
                restored.to_bytes().unwrap(),
                baseline,
                "{prefix} {name} repeated Delete"
            );
        }
        for reverse in [false, true] {
            let mut both = source.clone();
            for head in if reverse {
                [false, true]
            } else {
                [true, false]
            } {
                let row = base + u16::from(!head);
                let at = x + if head { 0 } else { 9 };
                let p = both.ladder_programs().remove(index).unwrap();
                let b = p
                    .iec_function_blocks()
                    .unwrap()
                    .into_iter()
                    .find(|b| b.row_index == row && b.raw_x == at)
                    .unwrap();
                both.delete_iec_ld_scalar_chain_function(index, b.record_offset, &b.name.value)
                    .unwrap();
            }
            let p = both.ladder_programs().remove(index).unwrap();
            if index == 2 {
                let native = XgwxDocument::from_path(capture.join(format!("CB{x}D3.xgwx")))
                    .unwrap()
                    .ladder_programs()
                    .remove(index)
                    .unwrap();
                assert_eq!(
                    area(&p, base),
                    area(&native, base),
                    "{prefix} both Delete reverse={reverse}"
                );
            }
            for head in if reverse {
                [false, true]
            } else {
                [true, false]
            } {
                let row = base + u16::from(!head);
                let at = x + if head { 0 } else { 9 };
                both.insert_iec_ld_function(
                    index,
                    row,
                    at,
                    if head { "GT" } else { "LE" },
                    &[instance.to_owned(), if head { pt1 } else { pt2 }.to_owned()],
                )
                .unwrap();
            }
            assert!(
                both.ladder_programs()
                    .remove(index)
                    .unwrap()
                    .iec_circuit_graph()
                    .is_some()
            );
            if let Ok(out) = env::var("LIBXGWX_PAIRED_COMPARISON_OUTPUT") {
                std::fs::write(
                    std::path::Path::new(&out)
                        .join(format!("{prefix}B{}REFGEN.xgwx", u8::from(reverse))),
                    both.to_bytes().unwrap(),
                )
                .unwrap();
            }
        }
    }
}

#[test]
#[cfg(feature = "write")]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_CONVERSION_PAIR_CAPTURE"]
fn xgi_conversion_pair_delete_and_refill() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_CONVERSION_PAIR_CAPTURE").unwrap());
    let original = source
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let area = |p: &LadderProgramData, base| {
        let rows = p.iec_row_frames().unwrap();
        let first = rows.iter().find(|r| r.row_index == base).unwrap();
        let last = rows.iter().find(|r| r.row_index == base + 5).unwrap();
        p.data[first.start - 10..last.end].to_vec()
    };
    let timer = |p: &LadderProgramData, base| {
        let b = p
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == base + 3 && b.raw_x == 10)
            .unwrap();
        p.data[b.record_offset..b.record_end].to_vec()
    };
    for (index, base, device, shared, time, prefix) in [
        (5, 8, "%MW410", "출력_2", "시간입력변환2", "BC"),
        (6, 12, "%MW500", "출력_1", "시간입력변환", "HC"),
    ] {
        for (x, name, args, letter) in [
            (
                10,
                "INT_TO_UDINT",
                vec![device.to_owned(), shared.to_owned()],
                "1",
            ),
            (
                19,
                "UDINT_TO_TIME",
                vec![shared.to_owned(), time.to_owned()],
                "2",
            ),
        ] {
            let b = original[index]
                .iec_function_blocks()
                .unwrap()
                .into_iter()
                .find(|b| b.row_index == base && b.raw_x == x)
                .unwrap();
            // Unknown timer bindings and altered shared spines must stay guarded.
            let timer_block = original[index]
                .iec_function_blocks()
                .unwrap()
                .into_iter()
                .find(|b| b.row_index == base + 3 && b.raw_x == 10)
                .unwrap();
            let top = original[index]
                .iec_row_frames()
                .unwrap()
                .into_iter()
                .find(|r| r.row_index == base)
                .unwrap();
            for offset in [timer_block.record_offset + 32, top.start - 6] {
                let mut unknown = original[index].clone();
                unknown.data[offset] = 1;
                assert!(
                    crate::iec_conversion_pair_write::remove(&unknown, b.record_offset).is_err()
                );
            }
            let mut deleted = source.clone();
            deleted
                .delete_iec_ld_scalar_chain_function(index, b.record_offset, name)
                .unwrap();
            let p = deleted.ladder_programs().remove(index).unwrap();
            assert_eq!(timer(&p, base), timer(&original[index], base));
            if index == 5 {
                let native =
                    XgwxDocument::from_path(capture.join(format!("N{prefix}{letter}D3.xgwx")))
                        .unwrap()
                        .ladder_programs()
                        .remove(index)
                        .unwrap();
                assert_eq!(
                    area(&p, base),
                    area(&native, base),
                    "{prefix}{letter} native Delete"
                );
            }
            let baseline = deleted.to_bytes().unwrap();
            for bad_args in [
                vec!["UNKNOWN".into(), args[1].clone()],
                vec![args[0].clone(), "1".into()],
                vec![args[0].clone()],
                vec![time.into(), shared.into()],
            ] {
                let mut bad = deleted.clone();
                assert!(
                    bad.insert_iec_ld_function(index, base, x, name, &bad_args)
                        .is_err()
                );
                assert_eq!(bad.to_bytes().unwrap(), baseline);
            }
            let mut restored = deleted.clone();
            restored
                .insert_iec_ld_function(index, base, x, name, &args)
                .unwrap();
            let p = restored.ladder_programs().remove(index).unwrap();
            assert_eq!(timer(&p, base), timer(&original[index], base));
            if index == 5 {
                let native =
                    XgwxDocument::from_path(capture.join(format!("N{prefix}{letter}R3.xgwx")))
                        .unwrap()
                        .ladder_programs()
                        .remove(index)
                        .unwrap();
                assert_eq!(
                    area(&p, base),
                    area(&native, base),
                    "{prefix}{letter} native refill"
                );
            }
            for (i, p) in restored.ladder_programs().into_iter().enumerate() {
                if i != index {
                    assert_eq!(p.unwrap().data, original[i].data);
                }
            }
            if let Ok(out) = env::var("LIBXGWX_CONVERSION_PAIR_OUTPUT") {
                std::fs::create_dir_all(&out).unwrap();
                std::fs::write(
                    std::path::Path::new(&out).join(format!("{prefix}{letter}DELGEN.xgwx")),
                    baseline.clone(),
                )
                .unwrap();
                std::fs::write(
                    std::path::Path::new(&out).join(format!("{prefix}{letter}REFGEN.xgwx")),
                    restored.to_bytes().unwrap(),
                )
                .unwrap();
            }
            let b = p
                .iec_function_blocks()
                .unwrap()
                .into_iter()
                .find(|b| b.row_index == base && b.raw_x == x)
                .unwrap();
            restored
                .delete_iec_ld_scalar_chain_function(index, b.record_offset, name)
                .unwrap();
            assert_eq!(restored.to_bytes().unwrap(), baseline);
        }
        let mut results = Vec::new();
        for reverse in [false, true] {
            let mut both = source.clone();
            let order = if reverse { [19, 10] } else { [10, 19] };
            for x in order {
                let p = both.ladder_programs().remove(index).unwrap();
                let b = p
                    .iec_function_blocks()
                    .unwrap()
                    .into_iter()
                    .find(|b| b.row_index == base && b.raw_x == x)
                    .unwrap();
                both.delete_iec_ld_scalar_chain_function(index, b.record_offset, &b.name.value)
                    .unwrap();
            }
            let deleted = both.to_bytes().unwrap();
            if index == 5 {
                let native = XgwxDocument::from_path(capture.join("NBCBD3.xgwx"))
                    .unwrap()
                    .ladder_programs()
                    .remove(index)
                    .unwrap();
                assert_eq!(
                    area(&both.ladder_programs().remove(index).unwrap(), base),
                    area(&native, base)
                );
            }
            for x in order {
                both.insert_iec_ld_function(
                    index,
                    base,
                    x,
                    if x == 10 {
                        "INT_TO_UDINT"
                    } else {
                        "UDINT_TO_TIME"
                    },
                    &if x == 10 {
                        vec![device.into(), shared.into()]
                    } else {
                        vec![shared.into(), time.into()]
                    },
                )
                .unwrap();
            }
            assert_eq!(
                timer(&both.ladder_programs().remove(index).unwrap(), base),
                timer(&original[index], base)
            );
            if let Ok(out) = env::var("LIBXGWX_CONVERSION_PAIR_OUTPUT") {
                std::fs::write(
                    std::path::Path::new(&out)
                        .join(format!("{prefix}B{}DELGEN.xgwx", u8::from(reverse))),
                    deleted,
                )
                .unwrap();
                std::fs::write(
                    std::path::Path::new(&out)
                        .join(format!("{prefix}B{}REFGEN.xgwx", u8::from(reverse))),
                    both.to_bytes().unwrap(),
                )
                .unwrap();
            }
            results.push(both.to_bytes().unwrap());
        }
        assert_eq!(results[0], results[1], "both refill orders");
    }
}

#[test]
#[cfg(feature = "write")]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_SHARED_TIMER_CAPTURE"]
fn xgi_shared_branch_timer_delete_and_refill() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_SHARED_TIMER_CAPTURE").unwrap());
    let area = |p: &LadderProgramData, base| {
        let rows = p.iec_row_frames().unwrap();
        let top = rows.iter().find(|r| r.row_index == base).unwrap();
        let last = rows
            .iter()
            .filter(|r| r.group_index == top.group_index)
            .last()
            .unwrap();
        p.data[top.start - 10..last.end].to_vec()
    };
    let original = source
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let save = |doc: &XgwxDocument, stem: &str| {
        if let Ok(out) = env::var("LIBXGWX_SHARED_TIMER_OUTPUT") {
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(
                std::path::Path::new(&out).join(format!("{stem}.xgwx")),
                doc.to_bytes().unwrap(),
            )
            .unwrap();
        }
    };
    for (index, base, instance, preset, prefix, native_prefix) in [
        (5, 8, "타이머4", "시간입력변환2", "SBT", "NSTB"),
        (6, 12, "타이머3", "시간입력변환", "SHT", "NSTH"),
    ] {
        let block = original[index]
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == base + 3 && b.raw_x == 10)
            .unwrap();
        let mut deleted = source.clone();
        deleted
            .delete_iec_ld_scalar_chain_function(index, block.record_offset, "TON")
            .unwrap();
        let native_deleted =
            XgwxDocument::from_path(capture.join(format!("{native_prefix}D3.xgwx"))).unwrap();
        assert_eq!(
            area(&deleted.ladder_programs().remove(index).unwrap(), base),
            area(
                &native_deleted.ladder_programs().remove(index).unwrap(),
                base
            ),
            "{prefix} Delete"
        );
        save(&deleted, &format!("{prefix}DELGEN"));
        let bytes = deleted.to_bytes().unwrap();
        for args in [
            vec!["UNKNOWN".into(), preset.into()],
            vec![instance.into(), "1".into()],
            vec![instance.into(), "%MW0".into()],
            vec![instance.into()],
            vec![instance.into(), preset.into(), preset.into()],
        ] {
            let mut invalid = deleted.clone();
            assert!(
                invalid
                    .insert_iec_ld_function(index, base + 3, 10, "TON", &args)
                    .is_err()
            );
            assert_eq!(invalid.to_bytes().unwrap(), bytes);
        }
        let mut restored = deleted.clone();
        restored
            .insert_iec_ld_function(
                index,
                base + 3,
                10,
                "TON",
                &[instance.into(), preset.into()],
            )
            .unwrap();
        let native_restored =
            XgwxDocument::from_path(capture.join(format!("{native_prefix}R3.xgwx"))).unwrap();
        assert_eq!(
            area(&restored.ladder_programs().remove(index).unwrap(), base),
            area(
                &native_restored.ladder_programs().remove(index).unwrap(),
                base
            ),
            "{prefix} refill"
        );
        save(&restored, &format!("{prefix}REFGEN"));
        for (i, p) in restored.ladder_programs().into_iter().enumerate() {
            if i != index {
                assert_eq!(p.unwrap().data, original[i].data);
            }
        }
        let block = restored
            .ladder_programs()
            .remove(index)
            .unwrap()
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == base + 3 && b.raw_x == 10)
            .unwrap();
        restored
            .delete_iec_ld_scalar_chain_function(index, block.record_offset, "TON")
            .unwrap();
        assert_eq!(restored.to_bytes().unwrap(), bytes, "repeat Delete");
        let mut literal = deleted.clone();
        literal
            .insert_iec_ld_function(
                index,
                base + 3,
                10,
                "TON",
                &[instance.into(), "T#2s".into()],
            )
            .unwrap();
        if index == 5 {
            let native = XgwxDocument::from_path(capture.join("NSTBL3.xgwx")).unwrap();
            assert_eq!(
                area(&literal.ladder_programs().remove(index).unwrap(), base),
                area(&native.ladder_programs().remove(index).unwrap(), base),
                "TIME literal"
            );
        }
        save(&literal, &format!("{prefix}LITGEN"));
        // Every deletion/refill order must retain the shared branch and leave other programs untouched.
        let permutations = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        for order in permutations {
            let positions = [
                (base, 10, "INT_TO_UDINT"),
                (base, 19, "UDINT_TO_TIME"),
                (base + 3, 10, "TON"),
            ];
            let operands = if index == 5 {
                [
                    vec!["%MW410".into(), "출력_2".into()],
                    vec!["출력_2".into(), preset.into()],
                    vec![instance.into(), preset.into()],
                ]
            } else {
                [
                    vec!["%MW500".into(), "출력_1".into()],
                    vec!["출력_1".into(), preset.into()],
                    vec![instance.into(), preset.into()],
                ]
            };
            let mut all = source.clone();
            for which in order {
                let (row, x, name) = positions[which];
                let b = all
                    .ladder_programs()
                    .remove(index)
                    .unwrap()
                    .iec_function_blocks()
                    .unwrap()
                    .into_iter()
                    .find(|b| b.row_index == row && b.raw_x == x)
                    .unwrap();
                all.delete_iec_ld_scalar_chain_function(index, b.record_offset, name)
                    .unwrap();
            }
            if index == 5 {
                let native = XgwxDocument::from_path(capture.join("NSTAD3.xgwx")).unwrap();
                assert_eq!(
                    area(&all.ladder_programs().remove(index).unwrap(), base),
                    area(&native.ladder_programs().remove(index).unwrap(), base)
                );
            }
            save(&all, &format!("{prefix}ALLDELGEN"));
            // Refill timer first while both conversions are absent.
            let mut timer_first = all.clone();
            timer_first
                .insert_iec_ld_function(index, base + 3, 10, "TON", &operands[2])
                .unwrap();
            if index == 5 {
                let native = XgwxDocument::from_path(capture.join("NSTATR3.xgwx")).unwrap();
                assert_eq!(
                    area(&timer_first.ladder_programs().remove(index).unwrap(), base),
                    area(&native.ladder_programs().remove(index).unwrap(), base)
                );
            }
            save(&timer_first, &format!("{prefix}ONLYREFGEN"));
            for refill in permutations {
                let mut filled = all.clone();
                for which in refill {
                    let (row, x, name) = positions[which];
                    filled
                        .insert_iec_ld_function(index, row, x, name, &operands[which])
                        .unwrap();
                }
                assert_eq!(
                    filled
                        .ladder_programs()
                        .remove(index)
                        .unwrap()
                        .iec_function_blocks()
                        .unwrap()
                        .iter()
                        .filter(|b| b.row_index == base || b.row_index == base + 3)
                        .count(),
                    3
                );
                for (i, p) in filled.ladder_programs().into_iter().enumerate() {
                    if i != index {
                        assert_eq!(p.unwrap().data, original[i].data);
                    }
                }
                save(
                    &filled,
                    &format!("{prefix}ALL{}{}{}REFGEN", refill[0], refill[1], refill[2]),
                );
            }
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_ENABLED_TIMER_CAPTURE"]
fn xgi_enabled_connected_timer_delete_and_refill() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_ENABLED_TIMER_CAPTURE").unwrap());
    let original = source
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let block = original[2]
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 31 && b.raw_x == 19)
        .unwrap();
    let group_bytes = |p: &LadderProgramData| {
        let rows = p.iec_row_frames().unwrap();
        let owned = rows
            .iter()
            .filter(|r| r.group_index == 10)
            .collect::<Vec<_>>();
        p.data[owned[0].start - 10..owned.last().unwrap().end].to_vec()
    };
    // Unknown native binding encodings and mode values must not become writable.
    let top = original[2]
        .iec_row_frames()
        .unwrap()
        .into_iter()
        .find(|r| r.row_index == 31)
        .unwrap();
    for offset in [
        block.record_offset + 32,
        block.record_end - 60,
        block.record_end - 6,
        top.start - 6,
    ] {
        let mut unknown = original[2].clone();
        unknown.data[offset] = 3;
        assert!(crate::iec_connected_timer_write::remove(&unknown, block.record_offset).is_err());
    }
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_scalar_chain_function(2, block.record_offset, "TON")
        .unwrap();
    let p = deleted.ladder_programs().remove(2).unwrap();
    let native = XgwxDocument::from_path(capture.join("NED3.xgwx"))
        .unwrap()
        .ladder_programs()
        .remove(2)
        .unwrap();
    assert_eq!(
        group_bytes(&p),
        group_bytes(&native),
        "native enabled Delete"
    );
    let site = p
        .iec_terminal_timer_insertion_sites()
        .unwrap()
        .into_iter()
        .find(|s| s.row_index == 31 && s.raw_x == 19)
        .unwrap();
    let baseline = deleted.to_bytes().unwrap();
    for (instance, preset, elapsed) in [
        ("UNKNOWN", "T#5s", ""),
        ("INST10", "T#5s", ""),
        ("INST12", "8", ""),
        ("INST12", "%MW0", ""),
        ("INST12", "T#5s", "T#1s"),
    ] {
        let mut rejected = deleted.clone();
        assert!(
            rejected
                .insert_iec_ld_terminal_timer(2, site.contact_offset, instance, preset, elapsed)
                .is_err()
        );
        assert_eq!(rejected.to_bytes().unwrap(), baseline);
    }
    let save = |doc: &XgwxDocument, stem: &str| {
        if let Ok(out) = env::var("LIBXGWX_ENABLED_TIMER_OUTPUT") {
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(
                std::path::Path::new(&out).join(format!("{stem}.xgwx")),
                doc.to_bytes().unwrap(),
            )
            .unwrap();
        }
    };
    save(&deleted, "ETDELGEN");
    for (preset, native_stem, stem) in
        [("T#5s", "NER3", "ETREFGEN"), ("T#7s", "NE7R3", "ET7REFGEN")]
    {
        let mut restored = deleted.clone();
        restored
            .insert_iec_ld_function(2, 31, 19, "TON", &["INST12".into(), preset.into()])
            .unwrap();
        let p = restored.ladder_programs().remove(2).unwrap();
        let native = XgwxDocument::from_path(capture.join(format!("{native_stem}.xgwx")))
            .unwrap()
            .ladder_programs()
            .remove(2)
            .unwrap();
        assert_eq!(
            group_bytes(&p),
            group_bytes(&native),
            "native {preset} refill"
        );
        for (i, p) in restored.ladder_programs().into_iter().enumerate() {
            if i != 2 {
                assert_eq!(p.unwrap().data, original[i].data);
            }
        }
        save(&restored, stem);
        let b = p
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == 31 && b.raw_x == 19)
            .unwrap();
        restored
            .delete_iec_ld_scalar_chain_function(2, b.record_offset, "TON")
            .unwrap();
        assert_eq!(restored.to_bytes().unwrap(), baseline, "repeat Delete");
    }
    let mut replaced = source.clone();
    replaced
        .replace_iec_ld_scalar_chain_function(
            2,
            block.record_offset,
            "TON",
            "TON",
            &["INST12".into(), "T#7s".into()],
        )
        .unwrap();
    save(&replaced, "ETREPLACEGEN");
    let mut rebound = source.clone();
    rebound
        .replace_iec_ld_scalar_chain_function(
            2,
            block.record_offset,
            "TON",
            "TON",
            &["INST9".into(), "T#5s".into()],
        )
        .unwrap();
    assert_eq!(
        rebound
            .ladder_programs()
            .remove(2)
            .unwrap()
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == 31 && b.raw_x == 19)
            .unwrap()
            .instance
            .unwrap()
            .value,
        "INST9"
    );
    assert_eq!(
        rebound.iec_local_symbols().remove(2).unwrap(),
        source.iec_local_symbols().remove(2).unwrap()
    );
    save(&rebound, "ETINSTANCEGEN");
    let mut invalid = source.clone();
    assert!(
        invalid
            .replace_iec_ld_scalar_chain_function(
                2,
                block.record_offset,
                "TON",
                "TON",
                &["INST10".into(), "T#7s".into()]
            )
            .is_err()
    );
    assert_eq!(invalid.to_bytes().unwrap(), source.to_bytes().unwrap());
    // Existing timer layouts in the same program remain editable in sequence.
    let mut combined = source.clone();
    for row in [31, 13] {
        let b = combined
            .ladder_programs()
            .remove(2)
            .unwrap()
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row && b.name.value == "TON")
            .unwrap();
        combined
            .delete_iec_ld_scalar_chain_function(2, b.record_offset, "TON")
            .unwrap();
    }
    combined
        .insert_iec_ld_function(2, 31, 19, "TON", &["INST12".into(), "T#5s".into()])
        .unwrap();
    combined
        .insert_iec_ld_function(2, 13, 22, "TON", &["INST10".into(), "T#15s".into()])
        .unwrap();
    save(&combined, "ETCOMBINEDGEN");
    assert_eq!(
        combined
            .ladder_programs()
            .remove(2)
            .unwrap()
            .iec_function_blocks()
            .unwrap()
            .iter()
            .filter(|b| b.name.value == "TON")
            .count(),
        2
    );
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_COMMON_MOVE_CAPTURE"]
fn xgi_common_contact_move_delete_and_refill() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_COMMON_MOVE_CAPTURE").unwrap());
    let original = source
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let block = original[2]
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 36 && b.raw_x == 19)
        .unwrap();
    let group = |p: &LadderProgramData| {
        let rows = p.iec_row_frames().unwrap();
        let own = rows
            .iter()
            .filter(|r| r.group_index == 12)
            .collect::<Vec<_>>();
        p.data[own[0].start - 10..own.last().unwrap().end].to_vec()
    };
    let top = original[2]
        .iec_row_frames()
        .unwrap()
        .into_iter()
        .find(|r| r.row_index == 36)
        .unwrap();
    for at in [
        block.record_offset + 32,
        block.record_offset + 12,
        top.start - 6,
    ] {
        let mut altered = original[2].clone();
        altered.data[at] = 3;
        assert!(
            crate::iec_function_write::remove_chain(&altered, block.record_offset, "MOVE").is_err()
        );
    }
    let input = crate::iec_ld::function_operands(&original[2])
        .into_iter()
        .find(|operand| {
            operand.value == "0"
                && original[2]
                    .iec_function_operand_links()
                    .unwrap()
                    .iter()
                    .any(|link| {
                        link.target_record_offset == block.record_offset
                            && operand.offset == link.record_offset + 15
                    })
        })
        .unwrap();
    let mut invalid_type = source.clone();
    let unchanged = invalid_type.to_bytes().unwrap();
    assert!(
        invalid_type
            .update_iec_ld_function_operand(2, input.offset, "0", "TRUE")
            .is_err()
    );
    assert_eq!(invalid_type.to_bytes().unwrap(), unchanged);
    invalid_type
        .update_iec_ld_function_operand(2, input.offset, "0", "1")
        .unwrap();
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_scalar_chain_function(2, block.record_offset, "MOVE")
        .unwrap();
    let before_refill = deleted.to_bytes().unwrap();
    let p = deleted.ladder_programs().remove(2).unwrap();
    let native_deleted = XgwxDocument::from_path(capture.join("CMD3.xgwx"))
        .unwrap()
        .ladder_programs()
        .remove(2)
        .unwrap();
    assert_eq!(group(&p), group(&native_deleted), "native Delete");
    assert_eq!(
        crate::iec_function_write::contact_tail(&p, 36, 19).unwrap(),
        Some(1)
    );
    for operands in [["0", "0"], ["TRUE", "%MW301"], ["UNKNOWN", "%MW301"]] {
        let mut bad = deleted.clone();
        assert!(
            bad.insert_iec_ld_function(2, 36, 19, "MOVE", &operands.map(String::from))
                .is_err()
        );
        assert_eq!(bad.to_bytes().unwrap(), before_refill);
    }
    let mut bad = deleted.clone();
    assert!(
        bad.insert_iec_ld_function(2, 36, 19, "ADD", &["0".into(), "1".into(), "%MW301".into()])
            .is_err()
    );
    assert_eq!(bad.to_bytes().unwrap(), before_refill);
    let native_refill = XgwxDocument::from_path(capture.join("CMR3.xgwx"))
        .unwrap()
        .ladder_programs()
        .remove(2)
        .unwrap();
    let output = env::var("LIBXGWX_COMMON_MOVE_OUTPUT")
        .ok()
        .map(std::path::PathBuf::from);
    let save = |doc: &XgwxDocument, name: &str| {
        if let Some(out) = &output {
            std::fs::create_dir_all(out).unwrap();
            std::fs::write(out.join(format!("{name}.xgwx")), doc.to_bytes().unwrap()).unwrap();
        }
    };
    save(&deleted, "CMDELGEN");
    for (name, input, output_operand) in [("CMREFGEN", "0", "%MW301"), ("CMALTGEN", "1", "%MW302")]
    {
        let mut filled = deleted.clone();
        filled
            .insert_iec_ld_function(2, 36, 19, "MOVE", &[input.into(), output_operand.into()])
            .unwrap();
        let p = filled.ladder_programs().remove(2).unwrap();
        if name == "CMREFGEN" {
            assert_eq!(group(&p), group(&native_refill), "native refill");
        }
        for (i, p) in filled.ladder_programs().into_iter().enumerate() {
            if i != 2 {
                assert_eq!(p.unwrap().data, original[i].data);
            }
        }
        save(&filled, name);
        let b = p
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == 36)
            .unwrap();
        filled
            .delete_iec_ld_scalar_chain_function(2, b.record_offset, "MOVE")
            .unwrap();
        assert_eq!(filled.to_bytes().unwrap(), before_refill, "repeat Delete");
    }
    let mut replaced = source.clone();
    replaced
        .replace_iec_ld_scalar_chain_function(
            2,
            block.record_offset,
            "MOVE",
            "MOVE",
            &["1".into(), "%MW302".into()],
        )
        .unwrap();
    save(&replaced, "CMREPLACEGEN");
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_LIGHTING_MOVE_CAPTURE"]
fn xgi_lighting_contact_moves_delete_and_refill() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_LIGHTING_MOVE_CAPTURE").unwrap());
    let original = source
        .ladder_programs()
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let output = env::var("LIBXGWX_LIGHTING_MOVE_OUTPUT")
        .ok()
        .map(std::path::PathBuf::from);
    for (row, group_index, input, deleted_name, refill_name) in [
        (59, 30, "0", "LMD59", "LMR59F"),
        (56, 29, "3", "LMD56B", "LMR56F"),
    ] {
        let group = |p: &LadderProgramData| {
            let rows = p.iec_row_frames().unwrap();
            let own = rows
                .iter()
                .filter(|r| r.group_index == group_index)
                .collect::<Vec<_>>();
            p.data[own[0].start - 10..own.last().unwrap().end].to_vec()
        };
        let block = original[0]
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row && b.raw_x == 16)
            .unwrap();
        let mut deleted = source.clone();
        deleted
            .delete_iec_ld_scalar_chain_function(0, block.record_offset, "MOVE")
            .unwrap();
        let empty = deleted.to_bytes().unwrap();
        let native_deleted = XgwxDocument::from_path(capture.join(format!("{deleted_name}.xgwx")))
            .unwrap()
            .ladder_programs()
            .remove(0)
            .unwrap();
        assert_eq!(
            group(&deleted.ladder_programs().remove(0).unwrap()),
            group(&native_deleted),
            "L{row} native Delete"
        );
        assert_eq!(
            crate::iec_function_write::contact_tail(
                &deleted.ladder_programs().remove(0).unwrap(),
                row,
                16
            )
            .unwrap(),
            Some(1)
        );
        let mut invalid = deleted.clone();
        assert!(
            invalid
                .insert_iec_ld_function(
                    0,
                    row,
                    16,
                    "ADD",
                    &["0".into(), "1".into(), "%MW700".into()]
                )
                .is_err()
        );
        assert_eq!(invalid.to_bytes().unwrap(), empty);
        let mut filled = deleted.clone();
        filled
            .insert_iec_ld_function(0, row, 16, "MOVE", &[input.into(), "%MW700".into()])
            .unwrap();
        let p = filled.ladder_programs().remove(0).unwrap();
        if let Some(out) = &output {
            std::fs::create_dir_all(out).unwrap();
            std::fs::write(out.join(format!("LMDEL{row}GEN.xgwx")), &empty).unwrap();
            std::fs::write(
                out.join(format!("LMREF{row}GEN.xgwx")),
                filled.to_bytes().unwrap(),
            )
            .unwrap();
        }
        let native_refill = XgwxDocument::from_path(capture.join(format!("{refill_name}.xgwx")))
            .unwrap()
            .ladder_programs()
            .remove(0)
            .unwrap();
        assert_eq!(group(&p), group(&native_refill), "L{row} native refill");
        for doc in [&deleted, &filled] {
            for (i, p) in doc.ladder_programs().into_iter().enumerate().skip(1) {
                assert_eq!(p.unwrap().data, original[i].data);
            }
        }
        let b = p
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row)
            .unwrap();
        filled
            .delete_iec_ld_scalar_chain_function(0, b.record_offset, "MOVE")
            .unwrap();
        assert_eq!(filled.to_bytes().unwrap(), empty, "L{row} repeat Delete");
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE"]
fn xgi_move_bool_numeric_literals_preserve_destination_editability() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let p = source.ladder_programs().remove(0).unwrap();
    let block = p
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 63 && b.raw_x == 16)
        .unwrap();
    let links = p.iec_function_operand_links().unwrap();
    let operands = crate::iec_ld::function_operands(&p);
    let fields = operands
        .iter()
        .filter(|f| {
            links.iter().any(|l| {
                l.target_record_offset == block.record_offset && f.offset == l.record_offset + 15
            })
        })
        .collect::<Vec<_>>();
    let input = fields.iter().find(|f| f.value == "0").unwrap();
    let output = fields.iter().find(|f| f.value == "자기유지2").unwrap();
    let mut edited = source.clone();
    edited
        .update_iec_ld_function_operand(0, input.offset, "0", "0")
        .unwrap();
    assert_eq!(edited.ladder_programs().remove(0).unwrap().data, p.data);
    edited
        .update_iec_ld_function_operand(0, output.offset, "자기유지2", "자기유지1")
        .unwrap();
    let output_dir = env::var("LIBXGWX_BOOL_MOVE_OUTPUT")
        .ok()
        .map(std::path::PathBuf::from);
    if let Some(dir) = &output_dir {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("BM0EDITGEN.xgwx"), edited.to_bytes().unwrap()).unwrap();
    }
    let mut direct_one = source.clone();
    direct_one
        .update_iec_ld_function_operand(0, input.offset, "0", "1")
        .unwrap();
    if let Some(dir) = &output_dir {
        std::fs::write(dir.join("BM1EDITGEN.xgwx"), direct_one.to_bytes().unwrap()).unwrap();
    }
    let mut invalid = source.clone();
    let unchanged = invalid.to_bytes().unwrap();
    assert!(
        invalid
            .update_iec_ld_function_operand(0, input.offset, "0", "2")
            .is_err()
    );
    assert_eq!(invalid.to_bytes().unwrap(), unchanged);
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_scalar_chain_function(0, block.record_offset, "MOVE")
        .unwrap();
    let empty = deleted.to_bytes().unwrap();
    if let Some(dir) = &output_dir {
        std::fs::write(dir.join("BMDELGEN.xgwx"), &empty).unwrap();
    }
    for value in ["2", "1.0", "UNKNOWN"] {
        let mut invalid = deleted.clone();
        assert!(
            invalid
                .insert_iec_ld_function(0, 63, 16, "MOVE", &[value.into(), "자기유지2".into()])
                .is_err()
        );
        assert_eq!(invalid.to_bytes().unwrap(), empty);
    }
    let mut filled = deleted.clone();
    filled
        .insert_iec_ld_function(0, 63, 16, "MOVE", &["0".into(), "자기유지2".into()])
        .unwrap();
    let mut replaced = source.clone();
    replaced
        .replace_iec_ld_scalar_chain_function(
            0,
            block.record_offset,
            "MOVE",
            "MOVE",
            &["0".into(), "자기유지2".into()],
        )
        .unwrap();
    assert_eq!(filled.to_bytes().unwrap(), replaced.to_bytes().unwrap());
    for doc in [&edited, &filled] {
        for (i, other) in doc.ladder_programs().into_iter().enumerate().skip(1) {
            assert_eq!(
                other.unwrap().data,
                source.ladder_programs().remove(i).unwrap().data
            );
        }
    }
    let mut one = deleted.clone();
    one.insert_iec_ld_function(0, 63, 16, "MOVE", &["1".into(), "자기유지2".into()])
        .unwrap();
    if let Some(dir) = &output_dir {
        std::fs::write(dir.join("BM1REFGEN.xgwx"), one.to_bytes().unwrap()).unwrap();
    }
    if let Some(dir) = &output_dir {
        std::fs::write(dir.join("BM0REFGEN.xgwx"), filled.to_bytes().unwrap()).unwrap();
    }
    if let Ok(dir) = env::var("LIBXGWX_BOOL_MOVE_CAPTURE") {
        for (generated, stem) in [
            (&edited, "BM0ES"),
            (&direct_one, "BM1ES"),
            (&filled, "BM0RS"),
            (&one, "BM1RS"),
        ] {
            let native =
                XgwxDocument::from_path(std::path::Path::new(&dir).join(format!("{stem}.xgwx")))
                    .unwrap();
            let actual = native.ladder_programs();
            let expected = generated.ladder_programs();
            assert_eq!(actual.len(), expected.len());
            for (i, (actual, expected)) in actual.into_iter().zip(expected).enumerate() {
                assert_eq!(
                    actual.unwrap().data,
                    expected.unwrap().data,
                    "{stem} program {i}"
                );
            }
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_PARALLEL_MOVE_CAPTURE"]
fn xgi_parallel_contact_move_delete_and_refill() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_PARALLEL_MOVE_CAPTURE").unwrap());
    let group_bytes = |doc: &XgwxDocument| {
        let p = doc.ladder_programs().remove(0).unwrap();
        let rows = p.iec_row_frames().unwrap();
        let group = rows
            .iter()
            .filter(|r| r.group_index == 26)
            .collect::<Vec<_>>();
        p.data[group[0].start - 10..group.last().unwrap().end].to_vec()
    };
    let p = source.ladder_programs().remove(0).unwrap();
    let b = p
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 48 && b.raw_x == 19)
        .unwrap();
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_scalar_chain_function(0, b.record_offset, "MOVE")
        .unwrap();
    let native_deleted = XgwxDocument::from_path(capture.join("LP48D.xgwx")).unwrap();
    assert_eq!(
        group_bytes(&deleted),
        group_bytes(&native_deleted),
        "native Delete group"
    );
    let empty = deleted.to_bytes().unwrap();
    let mut invalid = deleted.clone();
    assert!(
        invalid
            .insert_iec_ld_function(0, 48, 19, "ADD", &["1".into(), "2".into(), "%MW600".into()])
            .is_err()
    );
    assert_eq!(invalid.to_bytes().unwrap(), empty);
    let mut filled = deleted.clone();
    filled
        .insert_iec_ld_function(0, 48, 19, "MOVE", &["0".into(), "%MW600".into()])
        .unwrap();
    let native_filled = XgwxDocument::from_path(capture.join("LP48RF.xgwx")).unwrap();
    assert_eq!(
        group_bytes(&filled),
        group_bytes(&native_filled),
        "fresh native refill group"
    );
    if let Ok(dir) = env::var("LIBXGWX_PARALLEL_MOVE_SAVE_CAPTURE") {
        for (generated, stem) in [(&deleted, "LP48DS"), (&filled, "LP48RS")] {
            let native =
                XgwxDocument::from_path(std::path::Path::new(&dir).join(format!("{stem}.xgwx")))
                    .unwrap();
            let expected = generated.ladder_programs();
            let actual = native.ladder_programs();
            assert_eq!(actual.len(), expected.len());
            for (i, (actual, expected)) in actual.into_iter().zip(expected).enumerate() {
                assert_eq!(
                    actual.unwrap().data,
                    expected.unwrap().data,
                    "{stem} program {i}"
                );
            }
        }
    }
    for doc in [&deleted, &filled] {
        for (i, program) in doc.ladder_programs().into_iter().enumerate().skip(1) {
            assert_eq!(
                program.unwrap().data,
                source.ladder_programs().remove(i).unwrap().data
            );
        }
    }
    if let Ok(dir) = env::var("LIBXGWX_PARALLEL_MOVE_OUTPUT") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("LP48DELGEN.xgwx"), &empty).unwrap();
        std::fs::write(dir.join("LP48REFGEN.xgwx"), filled.to_bytes().unwrap()).unwrap();
    }
    let b = filled
        .ladder_programs()
        .remove(0)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 48 && b.raw_x == 19)
        .unwrap();
    filled
        .delete_iec_ld_scalar_chain_function(0, b.record_offset, "MOVE")
        .unwrap();
    assert_eq!(filled.to_bytes().unwrap(), empty, "repeat Delete");
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_STAGGERED_MOVE_CAPTURE"]
fn xgi_upper_staggered_move_delete_and_refill() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_STAGGERED_MOVE_CAPTURE").unwrap());
    let affected_bytes = |doc: &XgwxDocument| {
        let p = doc.ladder_programs().remove(0).unwrap();
        let rows = p.iec_row_frames().unwrap();
        let first = rows.iter().find(|r| r.row_index == 52).unwrap();
        let last = rows.iter().find(|r| r.row_index == 55).unwrap();
        p.data[first.start - 10..last.end].to_vec()
    };
    let p = source.ladder_programs().remove(0).unwrap();
    let b = p
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 52 && b.raw_x == 16)
        .unwrap();
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_scalar_chain_function(0, b.record_offset, "MOVE")
        .unwrap();
    let native = XgwxDocument::from_path(capture.join("LP52D.xgwx")).unwrap();
    assert_eq!(
        affected_bytes(&deleted),
        affected_bytes(&native),
        "native split groups"
    );
    let empty = deleted.to_bytes().unwrap();
    let mut filled = deleted.clone();
    filled
        .insert_iec_ld_function(0, 52, 16, "MOVE", &["0".into(), "%MW700".into()])
        .unwrap();
    let native_filled = XgwxDocument::from_path(capture.join("LS52RF.xgwx")).unwrap();
    assert_eq!(
        affected_bytes(&filled),
        affected_bytes(&native_filled),
        "fresh native merged group"
    );
    let mut replaced = source.clone();
    replaced
        .replace_iec_ld_scalar_chain_function(
            0,
            b.record_offset,
            "MOVE",
            "MOVE",
            &["0".into(), "%MW700".into()],
        )
        .unwrap();
    assert_eq!(replaced.to_bytes().unwrap(), filled.to_bytes().unwrap());
    let mut invalid = deleted.clone();
    assert!(
        invalid
            .insert_iec_ld_function(0, 52, 16, "ADD", &["1".into(), "2".into(), "%MW700".into()])
            .is_err()
    );
    assert_eq!(invalid.to_bytes().unwrap(), empty);
    for doc in [&deleted, &filled] {
        for (i, program) in doc.ladder_programs().into_iter().enumerate().skip(1) {
            assert_eq!(
                program.unwrap().data,
                source.ladder_programs().remove(i).unwrap().data
            );
        }
    }
    if let Ok(dir) = env::var("LIBXGWX_STAGGERED_MOVE_SAVE_CAPTURE") {
        for (generated, stem) in [(&deleted, "LS52DS"), (&filled, "LS52RS")] {
            let native =
                XgwxDocument::from_path(std::path::Path::new(&dir).join(format!("{stem}.xgwx")))
                    .unwrap();
            let expected = generated.ladder_programs();
            let actual = native.ladder_programs();
            assert_eq!(actual.len(), expected.len());
            for (i, (actual, expected)) in actual.into_iter().zip(expected).enumerate() {
                assert_eq!(
                    actual.unwrap().data,
                    expected.unwrap().data,
                    "{stem} program {i}"
                );
            }
        }
    }
    if let Ok(dir) = env::var("LIBXGWX_STAGGERED_MOVE_OUTPUT") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            std::path::Path::new(&dir).join("LS52DELGEN.xgwx"),
            deleted.to_bytes().unwrap(),
        )
        .unwrap();
        std::fs::write(
            std::path::Path::new(&dir).join("LS52REFGEN.xgwx"),
            filled.to_bytes().unwrap(),
        )
        .unwrap();
    }
    let b = filled
        .ladder_programs()
        .remove(0)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 52 && b.raw_x == 16)
        .unwrap();
    filled
        .delete_iec_ld_scalar_chain_function(0, b.record_offset, "MOVE")
        .unwrap();
    assert_eq!(filled.to_bytes().unwrap(), empty, "repeat split");
}

#[test]
#[cfg(feature = "write")]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_UPPER_CONTACT_MOVE_CAPTURE"]
fn xgi_upper_contact_move_delete_and_refill() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_UPPER_CONTACT_MOVE_CAPTURE").unwrap());
    let affected = |doc: &XgwxDocument| {
        let p = doc.ladder_programs().remove(0).unwrap();
        let rows = p.iec_row_frames().unwrap();
        let first = rows.iter().find(|r| r.row_index == 84).unwrap();
        let last = rows.iter().find(|r| r.row_index == 89).unwrap();
        p.data[first.start - 10..last.end].to_vec()
    };
    let p = source.ladder_programs().remove(0).unwrap();
    let b = p
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 84 && b.raw_x == 16)
        .unwrap();
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_scalar_chain_function(0, b.record_offset, "MOVE")
        .unwrap();
    let native = XgwxDocument::from_path(capture.join("LU84D.xgwx")).unwrap();
    assert_eq!(
        affected(&deleted),
        affected(&native),
        "native deletion network"
    );
    let empty = deleted.to_bytes().unwrap();
    let mut filled = deleted.clone();
    filled
        .insert_iec_ld_function(0, 84, 16, "MOVE", &["0".into(), "%MW10".into()])
        .unwrap();
    let native = XgwxDocument::from_path(capture.join("LU84RF.xgwx")).unwrap();
    assert_eq!(
        affected(&filled),
        affected(&native),
        "fresh native refill network"
    );
    let mut replaced = source.clone();
    replaced
        .replace_iec_ld_scalar_chain_function(
            0,
            b.record_offset,
            "MOVE",
            "MOVE",
            &["0".into(), "%MW10".into()],
        )
        .unwrap();
    assert_eq!(
        replaced.to_bytes().unwrap(),
        filled.to_bytes().unwrap(),
        "atomic replacement"
    );
    let mut invalid = deleted.clone();
    assert!(
        invalid
            .insert_iec_ld_function(0, 84, 16, "ADD", &["1".into(), "2".into(), "%MW10".into()])
            .is_err()
    );
    assert_eq!(invalid.to_bytes().unwrap(), empty);
    for doc in [&deleted, &filled] {
        let actual = doc.ladder_programs().remove(0).unwrap();
        let start = p
            .iec_row_frames()
            .unwrap()
            .into_iter()
            .find(|r| r.row_index == 84)
            .unwrap()
            .start;
        assert_eq!(
            actual.data[..start],
            p.data[..start],
            "earlier lighting networks"
        );
        for (i, program) in doc.ladder_programs().into_iter().enumerate().skip(1) {
            assert_eq!(
                program.unwrap().data,
                source.ladder_programs().remove(i).unwrap().data
            );
        }
    }
    if let Ok(dir) = env::var("LIBXGWX_UPPER_CONTACT_MOVE_SAVE_CAPTURE") {
        for (generated, stem) in [(&deleted, "LU84DS"), (&filled, "LU84RS")] {
            let native =
                XgwxDocument::from_path(std::path::Path::new(&dir).join(format!("{stem}.xgwx")))
                    .unwrap();
            let expected = generated.ladder_programs();
            let actual = native.ladder_programs();
            assert_eq!(actual.len(), expected.len());
            for (i, (actual, expected)) in actual.into_iter().zip(expected).enumerate() {
                assert_eq!(
                    actual.unwrap().data,
                    expected.unwrap().data,
                    "{stem} program {i}"
                );
            }
        }
    }
    if let Ok(dir) = env::var("LIBXGWX_UPPER_CONTACT_MOVE_OUTPUT") {
        std::fs::create_dir_all(&dir).unwrap();
        for (doc, stem) in [(&deleted, "LU84DELGEN"), (&filled, "LU84REFGEN")] {
            std::fs::write(
                std::path::Path::new(&dir).join(format!("{stem}.xgwx")),
                doc.to_bytes().unwrap(),
            )
            .unwrap();
        }
    }
    let b = filled
        .ladder_programs()
        .remove(0)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 84 && b.raw_x == 16)
        .unwrap();
    filled
        .delete_iec_ld_scalar_chain_function(0, b.record_offset, "MOVE")
        .unwrap();
    assert_eq!(filled.to_bytes().unwrap(), empty, "repeat deletion");
}

#[test]
#[cfg(feature = "write")]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_COIL_COMPARISON_CAPTURE"]
fn xgi_coil_comparison_delete_and_refill() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_COIL_COMPARISON_CAPTURE").unwrap());
    let affected = |doc: &XgwxDocument| {
        let p = doc.ladder_programs().remove(2).unwrap();
        let rows = p.iec_row_frames().unwrap();
        let group = rows
            .iter()
            .filter(|r| r.group_index == 3)
            .collect::<Vec<_>>();
        p.data[group[0].start - 10..group.last().unwrap().end].to_vec()
    };
    let p = source.ladder_programs().remove(2).unwrap();
    let b = p
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 5 && b.raw_x == 7)
        .unwrap();
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_scalar_chain_function(2, b.record_offset, "EQ")
        .unwrap();
    let native = XgwxDocument::from_path(capture.join("CE5D.xgwx")).unwrap();
    assert_eq!(affected(&deleted), affected(&native), "native deletion");
    assert!(
        deleted
            .ladder_programs()
            .remove(2)
            .unwrap()
            .iec_wired_comparison_insertion_sites()
            .unwrap()
            .iter()
            .any(|s| s.row_index == 5 && s.raw_x == 7)
    );
    let empty = deleted.to_bytes().unwrap();
    let mut filled = deleted.clone();
    filled
        .insert_iec_ld_function(2, 5, 7, "EQ", &["변환".into(), "8718".into()])
        .unwrap();
    let native = XgwxDocument::from_path(capture.join("CE5RF.xgwx")).unwrap();
    assert_eq!(affected(&filled), affected(&native), "fresh native refill");
    let mut replaced = source.clone();
    replaced
        .replace_iec_ld_scalar_chain_function(
            2,
            b.record_offset,
            "EQ",
            "EQ",
            &["변환".into(), "8718".into()],
        )
        .unwrap();
    assert_eq!(replaced.to_bytes().unwrap(), filled.to_bytes().unwrap());
    let mut alternate = deleted.clone();
    alternate
        .insert_iec_ld_function(2, 5, 7, "GT", &["변환".into(), "8718".into()])
        .unwrap();
    for doc in [&deleted, &filled, &alternate] {
        for (i, program) in doc
            .ladder_programs()
            .into_iter()
            .enumerate()
            .filter(|(i, _)| *i != 2)
        {
            assert_eq!(
                program.unwrap().data,
                source.ladder_programs().remove(i).unwrap().data
            );
        }
        let actual = doc.ladder_programs().remove(2).unwrap();
        let original = p.iec_row_frames().unwrap();
        let first = original.iter().find(|r| r.group_index == 3).unwrap().start - 10;
        let end = original
            .iter()
            .filter(|r| r.group_index == 3)
            .last()
            .unwrap()
            .end;
        let changed = actual.iec_row_frames().unwrap();
        let new_end = changed
            .iter()
            .filter(|r| r.group_index == 3)
            .last()
            .unwrap()
            .end;
        assert_eq!(actual.data[..first], p.data[..first]);
        assert_eq!(actual.data[new_end..], p.data[end..]);
    }
    let mut invalid = deleted.clone();
    assert!(
        invalid
            .insert_iec_ld_function(2, 5, 7, "MOVE", &["0".into(), "%MW0".into()])
            .is_err()
    );
    assert_eq!(invalid.to_bytes().unwrap(), empty);
    if let Ok(dir) = env::var("LIBXGWX_COIL_COMPARISON_OUTPUT") {
        std::fs::create_dir_all(&dir).unwrap();
        for (doc, stem) in [
            (&deleted, "CE5DELGEN"),
            (&filled, "CE5REFGEN"),
            (&alternate, "CE5GTGEN"),
        ] {
            std::fs::write(
                std::path::Path::new(&dir).join(format!("{stem}.xgwx")),
                doc.to_bytes().unwrap(),
            )
            .unwrap();
        }
    }
    if let Ok(dir) = env::var("LIBXGWX_COIL_COMPARISON_SAVE_CAPTURE") {
        for (generated, stem) in [
            (&deleted, "CE5DS"),
            (&filled, "CE5RS"),
            (&alternate, "CE5GS"),
        ] {
            let native =
                XgwxDocument::from_path(std::path::Path::new(&dir).join(format!("{stem}.xgwx")))
                    .unwrap();
            for (i, (actual, expected)) in native
                .ladder_programs()
                .into_iter()
                .zip(generated.ladder_programs())
                .enumerate()
            {
                assert_eq!(
                    actual.unwrap().data,
                    expected.unwrap().data,
                    "{stem} program {i}"
                );
            }
        }
    }
    let b = filled
        .ladder_programs()
        .remove(2)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 5 && b.raw_x == 7)
        .unwrap();
    filled
        .delete_iec_ld_scalar_chain_function(2, b.record_offset, "EQ")
        .unwrap();
    assert_eq!(filled.to_bytes().unwrap(), empty, "repeat deletion");
}

#[test]
#[cfg(feature = "write")]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_CONTACT_MESH_MOVE_CAPTURE"]
fn xgi_contact_mesh_move_delete_and_refill() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_CONTACT_MESH_MOVE_CAPTURE").unwrap());
    let affected = |doc: &XgwxDocument| {
        let p = doc.ladder_programs().remove(6).unwrap();
        let rows = p.iec_row_frames().unwrap();
        let first = rows.iter().find(|r| r.row_index == 80).unwrap();
        let last = rows.iter().find(|r| r.row_index == 85).unwrap();
        p.data[first.start - 10..last.end].to_vec()
    };
    let p = source.ladder_programs().remove(6).unwrap();
    let b = p
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 80 && b.raw_x == 19)
        .unwrap();
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_scalar_chain_function(6, b.record_offset, "MOVE")
        .unwrap();
    let native = XgwxDocument::from_path(capture.join("HM80D.xgwx")).unwrap();
    assert_eq!(
        affected(&deleted),
        affected(&native),
        "native deletion network"
    );
    let empty = deleted.to_bytes().unwrap();
    let mut filled = deleted.clone();
    filled
        .insert_iec_ld_function(6, 80, 19, "MOVE", &["0".into(), "%MW129".into()])
        .unwrap();
    let native = XgwxDocument::from_path(capture.join("HM80RF.xgwx")).unwrap();
    assert_eq!(
        affected(&filled),
        affected(&native),
        "fresh native refill network"
    );
    let mut replaced = source.clone();
    replaced
        .replace_iec_ld_scalar_chain_function(
            6,
            b.record_offset,
            "MOVE",
            "MOVE",
            &["0".into(), "%MW129".into()],
        )
        .unwrap();
    assert_eq!(
        replaced.to_bytes().unwrap(),
        filled.to_bytes().unwrap(),
        "atomic replacement"
    );
    let mut invalid = deleted.clone();
    assert!(
        invalid
            .insert_iec_ld_function(6, 80, 19, "ADD", &["1".into(), "2".into(), "%MW129".into()])
            .is_err()
    );
    assert_eq!(invalid.to_bytes().unwrap(), empty);
    for doc in [&deleted, &filled] {
        let actual = doc.ladder_programs().remove(6).unwrap();
        let start = p
            .iec_row_frames()
            .unwrap()
            .into_iter()
            .find(|r| r.row_index == 80)
            .unwrap()
            .start;
        assert_eq!(
            actual.data[..start],
            p.data[..start],
            "earlier heating networks"
        );
        let end_of_group = |program: &LadderProgramData| {
            program
                .iec_row_frames()
                .unwrap()
                .into_iter()
                .find(|r| r.row_index == 85)
                .unwrap()
                .end
        };
        assert_eq!(
            actual.data[end_of_group(&actual)..],
            p.data[end_of_group(&p)..],
            "later heating networks"
        );
        for (i, program) in doc
            .ladder_programs()
            .into_iter()
            .enumerate()
            .filter(|(i, _)| *i != 6)
        {
            assert_eq!(
                program.unwrap().data,
                source.ladder_programs().remove(i).unwrap().data
            );
        }
    }
    if let Ok(dir) = env::var("LIBXGWX_CONTACT_MESH_MOVE_SAVE_CAPTURE") {
        for (generated, stem) in [(&deleted, "HM80DS"), (&filled, "HM80RS")] {
            let native =
                XgwxDocument::from_path(std::path::Path::new(&dir).join(format!("{stem}.xgwx")))
                    .unwrap();
            let expected = generated.ladder_programs();
            let actual = native.ladder_programs();
            assert_eq!(actual.len(), expected.len());
            for (i, (actual, expected)) in actual.into_iter().zip(expected).enumerate() {
                assert_eq!(
                    actual.unwrap().data,
                    expected.unwrap().data,
                    "{stem} program {i}"
                );
            }
        }
    }
    if let Ok(dir) = env::var("LIBXGWX_CONTACT_MESH_MOVE_OUTPUT") {
        std::fs::create_dir_all(&dir).unwrap();
        for (doc, stem) in [(&deleted, "HM80DELGEN"), (&filled, "HM80REFGEN")] {
            std::fs::write(
                std::path::Path::new(&dir).join(format!("{stem}.xgwx")),
                doc.to_bytes().unwrap(),
            )
            .unwrap();
        }
    }
    let b = filled
        .ladder_programs()
        .remove(6)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .into_iter()
        .find(|b| b.row_index == 80 && b.raw_x == 19)
        .unwrap();
    filled
        .delete_iec_ld_scalar_chain_function(6, b.record_offset, "MOVE")
        .unwrap();
    assert_eq!(filled.to_bytes().unwrap(), empty, "repeat deletion");
}

#[test]
#[cfg(feature = "write")]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_MESH_CONTACT_CAPTURE"]
fn xgi_mesh_leading_contact_delete_and_refill() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_MESH_CONTACT_CAPTURE").unwrap());
    let program = source.ladder_programs().remove(6).unwrap();
    let site = program
        .iec_no_contact_deletion_sites()
        .unwrap()
        .into_iter()
        .find(|s| s.row_index == 80 && s.raw_x == 1)
        .unwrap();
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_contact(6, site.contact_offset, 1, "NO", "%MX729")
        .unwrap();
    let insert_site = deleted
        .ladder_programs()
        .remove(6)
        .unwrap()
        .iec_leading_contact_insertion_sites()
        .unwrap()
        .into_iter()
        .find(|s| s.row_index == 80)
        .unwrap();
    let mut filled = deleted.clone();
    filled
        .insert_iec_ld_leading_contact(6, insert_site.insertion_offset, "NO", "%MX729")
        .unwrap();
    if let Ok(dir) = env::var("LIBXGWX_MESH_CONTACT_OUTPUT") {
        std::fs::create_dir_all(&dir).unwrap();
        let mut restored = deleted.clone();
        restored
            .insert_iec_ld_leading_contact(6, insert_site.insertion_offset, "NO", "%MX729")
            .unwrap();
        for (doc, stem) in [(&deleted, "H80CDELGEN"), (&restored, "H80CREFGEN")] {
            std::fs::write(
                std::path::Path::new(&dir).join(format!("{stem}.xgwx")),
                doc.to_bytes().unwrap(),
            )
            .unwrap();
        }
    }
    let group_range = |program: &crate::LadderProgramData| {
        let rows = program.iec_row_frames().unwrap();
        let top = rows.iter().find(|r| r.row_index == 80).unwrap();
        let end = rows.iter().find(|r| r.row_index == 85).unwrap();
        top.start..end.end
    };
    for edited in [&deleted, &filled] {
        let p = edited.ladder_programs().remove(6).unwrap();
        let before = group_range(&program);
        let after = group_range(&p);
        assert_eq!(
            &p.data[..after.start],
            &program.data[..before.start],
            "earlier heating networks"
        );
        assert_eq!(
            &p.data[after.end..],
            &program.data[before.end..],
            "later heating networks"
        );
    }
    for (doc, stem) in [(&deleted, "H80C1D"), (&filled, "H80C1R")] {
        let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
        let actual = doc.ladder_programs();
        let expected = native.ladder_programs();
        assert_eq!(actual.len(), expected.len());
        for (i, (actual, expected)) in actual.into_iter().zip(expected).enumerate() {
            let actual = actual.unwrap().data;
            let expected = expected.unwrap().data;
            assert_eq!(
                actual.len(),
                expected.len(),
                "native {stem} program {i} length"
            );
            let differences = actual
                .iter()
                .zip(&expected)
                .enumerate()
                .filter_map(|(offset, (&generated, &native))| {
                    (generated != native).then_some((offset, generated, native))
                })
                .collect::<Vec<_>>();
            // Native manual editing also refreshed three earlier heating row
            // height caches. These exact observed changes are outside L80-L85;
            // generated edits preserve them. The generated Save As gate below
            // requires byte-for-byte equality without cache exceptions.
            let expected_differences = if i == 6 {
                vec![(317, 62, 39), (638, 62, 39), (1939, 78, 50)]
            } else {
                Vec::new()
            };
            assert_eq!(
                differences, expected_differences,
                "native {stem} program {i}"
            );
        }
    }
    let site = filled
        .ladder_programs()
        .remove(6)
        .unwrap()
        .iec_no_contact_deletion_sites()
        .unwrap()
        .into_iter()
        .find(|s| s.row_index == 80 && s.raw_x == 1)
        .unwrap();
    filled
        .delete_iec_ld_contact(6, site.contact_offset, 1, "NO", "%MX729")
        .unwrap();
    assert_eq!(
        filled.to_bytes().unwrap(),
        deleted.to_bytes().unwrap(),
        "repeat Delete"
    );
    let mut stale = source.clone();
    assert!(
        stale
            .delete_iec_ld_contact(6, site.contact_offset, 1, "NO", "%MX730")
            .is_err()
    );
    assert_eq!(stale.to_bytes().unwrap(), source.to_bytes().unwrap());
    let mut invalid = deleted.clone();
    assert!(
        invalid
            .insert_iec_ld_leading_contact(6, insert_site.insertion_offset, "NO", "%MW129")
            .is_err()
    );
    assert_eq!(invalid.to_bytes().unwrap(), deleted.to_bytes().unwrap());
    if let Ok(dir) = env::var("LIBXGWX_MESH_CONTACT_SAVE_CAPTURE") {
        let mut restored = deleted.clone();
        restored
            .insert_iec_ld_leading_contact(6, insert_site.insertion_offset, "NO", "%MX729")
            .unwrap();
        for (doc, stem) in [(&deleted, "H80CDS"), (&restored, "H80CRS")] {
            let native =
                XgwxDocument::from_path(std::path::Path::new(&dir).join(format!("{stem}.xgwx")))
                    .unwrap();
            for (i, (actual, expected)) in doc
                .ladder_programs()
                .into_iter()
                .zip(native.ladder_programs())
                .enumerate()
            {
                assert_eq!(
                    actual.unwrap().data,
                    expected.unwrap().data,
                    "Save As {stem} program {i}"
                );
            }
        }
    }
}

#[test]
#[cfg(feature = "write")]
#[ignore = "set LIBXGWX_SMARTHOME_FIXTURE and LIBXGWX_INTERIOR_CONTACT_CAPTURE"]
fn xgi_interior_contact_native_capture() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_INTERIOR_CONTACT_CAPTURE").unwrap());
    let p = source.ladder_programs().remove(6).unwrap();
    let site = p
        .iec_no_contact_deletion_sites()
        .unwrap()
        .into_iter()
        .find(|s| s.row_index == 80 && s.raw_x == 7)
        .unwrap();
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_contact(6, site.contact_offset, 7, "NC", "%MX726")
        .unwrap();
    let mut restored = deleted.clone();
    restored
        .insert_iec_ld_single_element(6, 80, 7, "contact", "NC", "%MX726")
        .unwrap();
    for (index, (a, b)) in restored
        .ladder_programs()
        .into_iter()
        .zip(source.ladder_programs())
        .enumerate()
    {
        assert!(
            a.unwrap().data == b.unwrap().data,
            "exact contact refill program {index}"
        );
    }
    for (doc, stem) in [(&deleted, "MI80C7D"), (&restored, "MI80C7R")] {
        let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
        for (index, (a, b)) in doc
            .ladder_programs()
            .into_iter()
            .zip(native.ladder_programs())
            .enumerate()
        {
            let (a, b) = (a.unwrap(), b.unwrap());
            assert_eq!(a.data.len(), b.data.len());
            let differences = a
                .data
                .iter()
                .zip(&b.data)
                .enumerate()
                .filter_map(|(offset, (&a, &b))| (a != b).then_some((offset, a, b)))
                .collect::<Vec<_>>();
            let expected = if index == 6 {
                let rows = a.iec_row_frames().unwrap();
                [
                    (3, 62, 39),
                    (4, 62, 39),
                    (12, 78, 50),
                    (80, 73, 39),
                    (81, 74, 39),
                    (82, 62, 39),
                    (83, 62, 39),
                    (84, 62, 39),
                    (85, 62, 39),
                ]
                .into_iter()
                .map(|(row, old, new)| {
                    (
                        rows.iter().find(|r| r.row_index == row).unwrap().start + 17,
                        old,
                        new,
                    )
                })
                .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            assert_eq!(differences, expected, "manual {stem} program {index}");
        }
    }
    if env::var_os("LIBXGWX_INTERIOR_CONTACT_SAVE_CAPTURE").is_some() {
        let lower_site = p
            .iec_no_contact_deletion_sites()
            .unwrap()
            .into_iter()
            .find(|s| s.row_index == 81 && s.raw_x == 7)
            .unwrap();
        let mut lower_deleted = source.clone();
        lower_deleted
            .delete_iec_ld_contact(6, lower_site.contact_offset, 7, "NC", "%MX727")
            .unwrap();
        for (doc, stem) in [
            (&deleted, "MI80C7DS"),
            (&lower_deleted, "MI81C7DS"),
            (&source, "MI81C7RS"),
        ] {
            let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
            for (index, (a, b)) in doc
                .ladder_programs()
                .into_iter()
                .zip(native.ladder_programs())
                .enumerate()
            {
                assert_eq!(
                    a.unwrap().data,
                    b.unwrap().data,
                    "Save As {stem} program {index}"
                );
            }
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "requires the supplied IEC project and native system flag Save As captures"]
fn xgi_system_flag_native_save_capture() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_SYSTEM_FLAG_CAPTURE").unwrap());
    let program = source.ladder_programs().remove(6).unwrap();
    let site = program
        .iec_no_contact_deletion_sites()
        .unwrap()
        .into_iter()
        .find(|s| s.row_index == 30 && s.raw_x == 4)
        .unwrap();
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_contact(6, site.contact_offset, 4, "NO", "_ON")
        .unwrap();
    let mut restored = deleted.clone();
    restored
        .insert_iec_ld_single_element(6, 30, 4, "contact", "NO", "_ON")
        .unwrap();
    for (a, b) in restored
        .ladder_programs()
        .into_iter()
        .zip(source.ladder_programs())
    {
        assert_eq!(
            a.unwrap().data,
            b.unwrap().data,
            "refill recovers source payload"
        );
    }
    for (doc, stem) in [(&deleted, "SYONDELS"), (&restored, "SYONREFS")] {
        let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
        assert_eq!(doc.ladder_programs().len(), 7);
        assert_eq!(native.ladder_programs().len(), 7);
        for (p, (a, b)) in doc
            .ladder_programs()
            .into_iter()
            .zip(native.ladder_programs())
            .enumerate()
        {
            assert_eq!(
                a.unwrap().data,
                b.unwrap().data,
                "native Save As {stem} program {p}"
            );
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "requires the supplied IEC project and native coil captures"]
fn xgi_addressed_coil_native_capture() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_COIL_CAPTURE").unwrap());
    let program = source.ladder_programs().remove(6).unwrap();
    let coil = program
        .iec_record_frames()
        .unwrap()
        .into_iter()
        .find(|r| r.row_index == 24 && r.kind == IecRecordKind::Coil(14))
        .unwrap();
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_terminal_coil(6, coil.offset, "%MX727")
        .unwrap();
    let mut restored = deleted.clone();
    restored
        .insert_iec_ld_single_element(6, 24, 94, "coil", "OUTPUT", "%MX727")
        .unwrap();
    for (a, b) in restored
        .ladder_programs()
        .into_iter()
        .zip(source.ladder_programs())
    {
        assert_eq!(
            a.unwrap().data,
            b.unwrap().data,
            "refill recovers source payload"
        );
    }
    for (doc, stem) in [(&deleted, "CO24NDEL"), (&restored, "CO24NREF")] {
        let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
        assert_eq!(native.ladder_programs().len(), 7);
        for (p, (a, b)) in doc
            .ladder_programs()
            .into_iter()
            .zip(native.ladder_programs())
            .enumerate()
        {
            let (mut a, b) = (a.unwrap(), b.unwrap());
            if p == 6 {
                let rows = a.iec_row_frames().unwrap();
                for (row, old, new) in [(3, 62, 39), (4, 62, 39), (12, 78, 50)] {
                    let offset = rows.iter().find(|r| r.row_index == row).unwrap().start + 17;
                    assert_eq!(a.data[offset], old);
                    assert_eq!(b.data[offset], new);
                    a.data[offset] = new;
                }
            }
            assert_eq!(a.data, b.data, "native manual {stem} program {p}");
        }
    }
    if env::var_os("LIBXGWX_COIL_SAVE_CAPTURE").is_some() {
        for (doc, stem) in [(&deleted, "CO24DELS"), (&restored, "CO24REFS")] {
            let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
            assert_eq!(native.ladder_programs().len(), 7);
            for (p, (a, b)) in doc
                .ladder_programs()
                .into_iter()
                .zip(native.ladder_programs())
                .enumerate()
            {
                assert_eq!(
                    a.unwrap().data,
                    b.unwrap().data,
                    "native Save As {stem} program {p}"
                );
            }
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "requires the supplied IEC project and disabled-identifier native captures"]
fn xgi_disabled_identifier_native_capture() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture =
        std::path::PathBuf::from(env::var("LIBXGWX_DISABLED_IDENTIFIER_CAPTURE").unwrap());
    let local_symbols = |doc: &XgwxDocument| {
        doc.iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    };
    let mut deletes = Vec::new();
    let mut refills = Vec::new();
    for (row, name) in [(2, "스위치_1"), (6, "스위치_2")] {
        let program = source.ladder_programs().remove(0).unwrap();
        let contact = program
            .iec_record_frames()
            .unwrap()
            .into_iter()
            .find(|r| r.row_index == row && r.kind == IecRecordKind::Contact(8))
            .unwrap();
        let mut deleted = source.clone();
        deleted
            .delete_iec_ld_contact(0, contact.offset, 1, "RISING", name)
            .unwrap();
        let mut restored = deleted.clone();
        restored
            .insert_iec_ld_single_element(0, row, 1, "contact", "RISING", name)
            .unwrap();
        assert_eq!(local_symbols(&restored), local_symbols(&source));
        for (a, b) in restored
            .ladder_programs()
            .into_iter()
            .zip(source.ladder_programs())
        {
            assert_eq!(
                a.unwrap().data,
                b.unwrap().data,
                "refill recovers source payload"
            );
        }
        deletes.push(deleted);
        refills.push(restored);
    }
    assert_eq!(
        refills[0].to_bytes().unwrap(),
        refills[1].to_bytes().unwrap()
    );
    for (doc, stem) in [(&deletes[0], "SW1NDEL"), (&refills[0], "SW1NUND")] {
        let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
        let mut native_symbols = local_symbols(&native);
        let source_symbols = local_symbols(&source);
        assert_eq!(native_symbols.len(), source_symbols.len());
        // Native Delete/Undo rewrites lighting LocalVar records. Each preceding
        // record becomes 22 bytes shorter; declaration and allocation fields
        // remain identical. Check this captured layout change explicitly.
        assert_eq!(native_symbols[0].len(), source_symbols[0].len());
        for (index, (native, source)) in native_symbols[0]
            .iter_mut()
            .zip(&source_symbols[0])
            .enumerate()
        {
            assert_eq!(native.record_offset + index * 22, source.record_offset);
            native.record_offset = source.record_offset;
        }
        assert_eq!(
            native_symbols, source_symbols,
            "native must not declare switch variables"
        );
        assert_eq!(native.ladder_programs().len(), 7);
        for (p, (a, b)) in doc
            .ladder_programs()
            .into_iter()
            .zip(native.ladder_programs())
            .enumerate()
        {
            let (mut a, b) = (a.unwrap(), b.unwrap());
            if p == 0 {
                let rows = a.iec_row_frames().unwrap();
                for (row, old, new) in [(22, 52, 39), (26, 52, 50)] {
                    let offset = rows.iter().find(|r| r.row_index == row).unwrap().start + 17;
                    assert_eq!(a.data[offset], old);
                    assert_eq!(b.data[offset], new);
                    a.data[offset] = new;
                }
            }
            assert_eq!(a.data, b.data, "native Delete/Undo {stem} program {p}");
        }
    }
    if env::var_os("LIBXGWX_DISABLED_IDENTIFIER_SAVE_CAPTURE").is_some() {
        for (doc, stem) in [
            (&deletes[0], "SW1DELS"),
            (&deletes[1], "SW2DELS"),
            (&refills[0], "SWREFS"),
        ] {
            let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
            assert_eq!(local_symbols(&native), local_symbols(&source));
            assert_eq!(native.ladder_programs().len(), 7);
            for (p, (a, b)) in doc
                .ladder_programs()
                .into_iter()
                .zip(native.ladder_programs())
                .enumerate()
            {
                assert_eq!(
                    a.unwrap().data,
                    b.unwrap().data,
                    "native Save As {stem} program {p}"
                );
            }
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "requires the supplied IEC project and native chain comparison refill captures"]
fn xgi_chain_comparison_refill_native_capture() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_CHAIN_REFILL_CAPTURE").unwrap());
    let block_at = |doc: &XgwxDocument| {
        doc.ladder_programs()
            .remove(0)
            .unwrap()
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == 67 && b.raw_x == 16)
            .unwrap()
    };
    let block = block_at(&source);
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_eq_chain_head(0, block.record_offset, "EQ")
        .unwrap();
    let mut restored = deleted.clone();
    let args = ["%MW10".to_owned(), "0".to_owned(), "%MX0".to_owned()];
    restored
        .insert_iec_ld_function(0, 67, 16, "EQ", &args)
        .unwrap();
    let symbols = |doc: &XgwxDocument| {
        doc.iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    };
    assert_eq!(symbols(&source), symbols(&restored));
    let restored_bytes = restored.to_bytes().unwrap();
    let mut removed_again = restored.clone();
    removed_again
        .delete_iec_ld_scalar_chain_function(0, block_at(&restored).record_offset, "EQ")
        .unwrap();
    for (candidate, stem) in [(&restored, "CH0NREF"), (&removed_again, "CH0NROOM")] {
        let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
        assert_eq!(native.ladder_programs().len(), 7);
        for (p, (a, b)) in candidate
            .ladder_programs()
            .into_iter()
            .zip(native.ladder_programs())
            .enumerate()
        {
            let (mut a, b) = (a.unwrap(), b.unwrap());
            if p == 0 {
                let rows = a.iec_row_frames().unwrap();
                for (row, old, new) in [(22, 52, 39), (26, 52, 50), (76, 56, 39)] {
                    let offset = rows.iter().find(|r| r.row_index == row).unwrap().start + 17;
                    assert_eq!(a.data[offset], old);
                    assert_eq!(b.data[offset], new);
                    a.data[offset] = new;
                }
            }
            assert_eq!(a.data, b.data, "native {stem} program {p}");
        }
    }
    removed_again
        .insert_iec_ld_function(0, 67, 16, "EQ", &args)
        .unwrap();
    assert_eq!(
        removed_again.to_bytes().unwrap(),
        restored_bytes,
        "repeat Delete/refill"
    );
    for bad in ["%MW0", "_ON"] {
        let before = deleted.to_bytes().unwrap();
        assert!(
            deleted
                .insert_iec_ld_function(
                    0,
                    67,
                    16,
                    "EQ",
                    &[args[0].clone(), args[1].clone(), bad.to_owned()]
                )
                .is_err()
        );
        assert_eq!(
            deleted.to_bytes().unwrap(),
            before,
            "rejected destination must not shift rows"
        );
    }
    if let Some(output) = env::var_os("LIBXGWX_CHAIN_REFILL_OUTPUT") {
        let output = std::path::PathBuf::from(output);
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(output.join("CH0GEN.xgwx"), restored_bytes).unwrap();
        let mut removed = restored.clone();
        removed
            .delete_iec_ld_scalar_chain_function(0, block_at(&restored).record_offset, "EQ")
            .unwrap();
        std::fs::write(output.join("CH0GDEL.xgwx"), removed.to_bytes().unwrap()).unwrap();
    }
    if env::var_os("LIBXGWX_CHAIN_REFILL_SAVE_CAPTURE").is_some() {
        let mut removed = restored.clone();
        removed
            .delete_iec_ld_scalar_chain_function(0, block_at(&restored).record_offset, "EQ")
            .unwrap();
        for (candidate, stem) in [(&restored, "CH0GENS"), (&removed, "CH0GDELS")] {
            let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
            assert_eq!(native.ladder_programs().len(), 7);
            assert_eq!(
                symbols(candidate),
                symbols(&native),
                "native Save As declarations {stem}"
            );
            for (p, (a, b)) in candidate
                .ladder_programs()
                .into_iter()
                .zip(native.ladder_programs())
                .enumerate()
            {
                assert_eq!(
                    a.unwrap().data,
                    b.unwrap().data,
                    "native Save As {stem} program {p}"
                );
            }
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "requires supplied IEC project and native heating head refill captures"]
fn xgi_heating_head_refill_native_capture() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_CHAIN_REFILL_CAPTURE").unwrap());
    let block_at = |doc: &XgwxDocument| {
        doc.ladder_programs()
            .remove(6)
            .unwrap()
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == 46 && b.raw_x == 19)
            .unwrap()
    };
    let symbols = |doc: &XgwxDocument| {
        doc.iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    };
    let mut deleted = source.clone();
    deleted
        .delete_iec_ld_heating_chain_head(6, block_at(&source).record_offset, "EQ")
        .unwrap();
    let args = ["%MW129".to_owned(), "1".to_owned(), "%MX729".to_owned()];
    let mut restored = deleted.clone();
    restored
        .insert_iec_ld_function(6, 46, 19, "EQ", &args)
        .unwrap();
    assert_eq!(symbols(&source), symbols(&restored));
    let restored_bytes = restored.to_bytes().unwrap();
    let mut removed = restored.clone();
    removed
        .delete_iec_ld_scalar_chain_function(6, block_at(&restored).record_offset, "EQ")
        .unwrap();
    for (candidate, stem) in [(&restored, "CH6NREF"), (&removed, "CH6NROOM")] {
        let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
        assert_eq!(native.ladder_programs().len(), 7);
        for (p, (a, b)) in candidate
            .ladder_programs()
            .into_iter()
            .zip(native.ladder_programs())
            .enumerate()
        {
            let (mut a, b) = (a.unwrap(), b.unwrap());
            if p == 6 {
                let rows = a.iec_row_frames().unwrap();
                for (row, old, new) in [
                    (3, 62, 39),
                    (4, 62, 39),
                    (12, 78, 50),
                    (50, 61, 39),
                    (51, 68, 39),
                    (54, 61, 39),
                    (55, 68, 39),
                ] {
                    let offset = rows.iter().find(|r| r.row_index == row).unwrap().start + 17;
                    assert_eq!(a.data[offset], old);
                    assert_eq!(b.data[offset], new);
                    a.data[offset] = new;
                }
            }
            assert_eq!(a.data, b.data, "native {stem} program {p}");
        }
    }
    let removed_bytes = removed.to_bytes().unwrap();
    removed
        .insert_iec_ld_function(6, 46, 19, "EQ", &args)
        .unwrap();
    assert_eq!(
        removed.to_bytes().unwrap(),
        restored_bytes,
        "repeat Delete/refill"
    );
    for bad in ["%MW0", "_ON"] {
        let before = deleted.to_bytes().unwrap();
        assert!(
            deleted
                .insert_iec_ld_function(
                    6,
                    46,
                    19,
                    "EQ",
                    &[args[0].clone(), args[1].clone(), bad.to_owned()]
                )
                .is_err()
        );
        assert_eq!(
            deleted.to_bytes().unwrap(),
            before,
            "rejected destination must not shift rows"
        );
    }
    if let Some(output) = env::var_os("LIBXGWX_CHAIN_REFILL_OUTPUT") {
        let output = std::path::PathBuf::from(output);
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(output.join("CH6GEN.xgwx"), restored_bytes).unwrap();
        std::fs::write(output.join("CH6GDEL.xgwx"), removed_bytes).unwrap();
    }
    if env::var_os("LIBXGWX_CHAIN_REFILL_SAVE_CAPTURE").is_some() {
        let mut removed = restored.clone();
        removed
            .delete_iec_ld_scalar_chain_function(6, block_at(&restored).record_offset, "EQ")
            .unwrap();
        for (candidate, stem) in [(&restored, "CH6GENS"), (&removed, "CH6GDELS")] {
            let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
            assert_eq!(native.ladder_programs().len(), 7);
            assert_eq!(
                symbols(candidate),
                symbols(&native),
                "native Save As symbols {stem}"
            );
            for (p, (a, b)) in candidate
                .ladder_programs()
                .into_iter()
                .zip(native.ladder_programs())
                .enumerate()
            {
                assert_eq!(
                    a.unwrap().data,
                    b.unwrap().data,
                    "native Save As {stem} program {p}"
                );
            }
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "requires supplied IEC project and native heating middle refill captures"]
fn xgi_heating_middle_refill_native_capture() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_CHAIN_REFILL_CAPTURE").unwrap());
    let symbols = |doc: &XgwxDocument| {
        doc.iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    };
    for (row, value, output, heights) in [
        (
            50,
            "2",
            "%MX730",
            [
                (3, 62, 39),
                (4, 62, 39),
                (12, 78, 50),
                (54, 61, 39),
                (55, 68, 39),
                (58, 61, 39),
                (59, 68, 39),
            ],
        ),
        (
            54,
            "3",
            "%MX731",
            [
                (3, 62, 39),
                (4, 62, 39),
                (12, 78, 50),
                (58, 61, 39),
                (59, 68, 39),
                (62, 73, 39),
                (63, 74, 39),
            ],
        ),
    ] {
        let block_at = |doc: &XgwxDocument| {
            doc.ladder_programs()
                .remove(6)
                .unwrap()
                .iec_function_blocks()
                .unwrap()
                .into_iter()
                .find(|b| b.row_index == row && b.raw_x == 19)
                .unwrap()
        };
        let mut deleted = source.clone();
        deleted
            .delete_iec_ld_heating_chain_middle(6, block_at(&source).record_offset, "EQ")
            .unwrap();
        let args = ["%MW129".to_owned(), value.to_owned(), output.to_owned()];
        let mut restored = deleted.clone();
        restored
            .insert_iec_ld_function(6, row, 19, "EQ", &args)
            .unwrap();
        assert_eq!(symbols(&source), symbols(&restored));
        let restored_bytes = restored.to_bytes().unwrap();
        let mut removed = restored.clone();
        removed
            .delete_iec_ld_scalar_chain_function(6, block_at(&restored).record_offset, "EQ")
            .unwrap();
        for (candidate, suffix) in [(&restored, "NREF"), (&removed, "NROOM")] {
            let stem = format!("M{row}{suffix}");
            let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
            assert_eq!(native.ladder_programs().len(), 7);
            for (p, (a, b)) in candidate
                .ladder_programs()
                .into_iter()
                .zip(native.ladder_programs())
                .enumerate()
            {
                let (mut a, b) = (a.unwrap(), b.unwrap());
                if p == 6 {
                    let frames = a.iec_row_frames().unwrap();
                    for (y, old, new) in heights {
                        let offset = frames.iter().find(|r| r.row_index == y).unwrap().start + 17;
                        assert_eq!(a.data[offset], old);
                        assert_eq!(b.data[offset], new);
                        a.data[offset] = new;
                    }
                }
                assert_eq!(a.data, b.data, "native {stem} program {p}");
            }
        }
        let removed_bytes = removed.to_bytes().unwrap();
        removed
            .insert_iec_ld_function(6, row, 19, "EQ", &args)
            .unwrap();
        assert_eq!(
            removed.to_bytes().unwrap(),
            restored_bytes,
            "repeat Delete/refill at {row}"
        );
        for bad in ["%MW0", "_ON"] {
            let before = deleted.to_bytes().unwrap();
            assert!(
                deleted
                    .insert_iec_ld_function(
                        6,
                        row,
                        19,
                        "EQ",
                        &[args[0].clone(), args[1].clone(), bad.to_owned()]
                    )
                    .is_err()
            );
            assert_eq!(deleted.to_bytes().unwrap(), before);
        }
        if let Some(output) = env::var_os("LIBXGWX_CHAIN_REFILL_OUTPUT") {
            let output = std::path::PathBuf::from(output);
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(output.join(format!("M{row}GEN.xgwx")), restored_bytes).unwrap();
            std::fs::write(output.join(format!("M{row}GDEL.xgwx")), removed_bytes).unwrap();
        }
        if env::var_os("LIBXGWX_CHAIN_REFILL_SAVE_CAPTURE").is_some() {
            let mut removed = restored.clone();
            removed
                .delete_iec_ld_scalar_chain_function(6, block_at(&restored).record_offset, "EQ")
                .unwrap();
            for (candidate, suffix) in [(&restored, "GENS"), (&removed, "GDELS")] {
                let stem = format!("M{row}{suffix}");
                let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
                assert_eq!(native.ladder_programs().len(), 7);
                assert_eq!(
                    symbols(candidate),
                    symbols(&native),
                    "native Save As symbols {stem}"
                );
                for (p, (a, b)) in candidate
                    .ladder_programs()
                    .into_iter()
                    .zip(native.ladder_programs())
                    .enumerate()
                {
                    assert_eq!(
                        a.unwrap().data,
                        b.unwrap().data,
                        "native Save As {stem} program {p}"
                    );
                }
            }
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "requires supplied IEC project and native heating outer refill captures"]
fn xgi_heating_outer_refill_native_capture() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_CHAIN_REFILL_CAPTURE").unwrap());
    let symbols = |doc: &XgwxDocument| {
        doc.iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    };
    for (row, value, output, heights) in [(
        58,
        "0",
        "%MX735",
        [
            (3, 62, 39),
            (4, 62, 39),
            (12, 78, 50),
            (62, 73, 39),
            (63, 74, 39),
            (66, 52, 39),
            (67, 56, 39),
        ],
    )] {
        let block_at = |doc: &XgwxDocument| {
            doc.ladder_programs()
                .remove(6)
                .unwrap()
                .iec_function_blocks()
                .unwrap()
                .into_iter()
                .find(|b| b.row_index == row && b.raw_x == 19)
                .unwrap()
        };
        let mut deleted = source.clone();
        deleted
            .delete_iec_ld_heating_chain_x3_eq_repaired(6, block_at(&source).record_offset, "EQ")
            .unwrap();
        let args = ["%MW129".to_owned(), value.to_owned(), output.to_owned()];
        let mut restored = deleted.clone();
        restored
            .insert_iec_ld_function(6, row, 19, "EQ", &args)
            .unwrap();
        assert_eq!(symbols(&source), symbols(&restored));
        let restored_bytes = restored.to_bytes().unwrap();
        let mut removed = restored.clone();
        removed
            .delete_iec_ld_scalar_chain_function(6, block_at(&restored).record_offset, "EQ")
            .unwrap();
        for (candidate, suffix) in [(&restored, "NREF"), (&removed, "NROOM")] {
            let stem = format!("M{row}{suffix}");
            let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
            assert_eq!(native.ladder_programs().len(), 7);
            for (p, (a, b)) in candidate
                .ladder_programs()
                .into_iter()
                .zip(native.ladder_programs())
                .enumerate()
            {
                let (mut a, b) = (a.unwrap(), b.unwrap());
                if p == 6 {
                    let frames = a.iec_row_frames().unwrap();
                    for (y, old, new) in heights {
                        let offset = frames.iter().find(|r| r.row_index == y).unwrap().start + 17;
                        assert_eq!(a.data[offset], old);
                        assert_eq!(b.data[offset], new);
                        a.data[offset] = new;
                    }
                }
                assert_eq!(a.data, b.data, "native {stem} program {p}");
            }
        }
        let removed_bytes = removed.to_bytes().unwrap();
        removed
            .insert_iec_ld_function(6, row, 19, "EQ", &args)
            .unwrap();
        assert_eq!(
            removed.to_bytes().unwrap(),
            restored_bytes,
            "repeat Delete/refill at {row}"
        );
        for bad in ["%MW0", "_ON"] {
            let before = deleted.to_bytes().unwrap();
            assert!(
                deleted
                    .insert_iec_ld_function(
                        6,
                        row,
                        19,
                        "EQ",
                        &[args[0].clone(), args[1].clone(), bad.to_owned()]
                    )
                    .is_err()
            );
            assert_eq!(deleted.to_bytes().unwrap(), before);
        }
        if let Some(output) = env::var_os("LIBXGWX_CHAIN_REFILL_OUTPUT") {
            let output = std::path::PathBuf::from(output);
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(output.join(format!("M{row}GEN.xgwx")), restored_bytes).unwrap();
            std::fs::write(output.join(format!("M{row}GDEL.xgwx")), removed_bytes).unwrap();
        }
        if env::var_os("LIBXGWX_CHAIN_REFILL_SAVE_CAPTURE").is_some() {
            let mut removed = restored.clone();
            removed
                .delete_iec_ld_scalar_chain_function(6, block_at(&restored).record_offset, "EQ")
                .unwrap();
            for (candidate, suffix) in [(&restored, "GENS"), (&removed, "GDELS")] {
                let stem = format!("M{row}{suffix}");
                let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
                assert_eq!(native.ladder_programs().len(), 7);
                assert_eq!(
                    symbols(candidate),
                    symbols(&native),
                    "native Save As symbols {stem}"
                );
                for (p, (a, b)) in candidate
                    .ladder_programs()
                    .into_iter()
                    .zip(native.ladder_programs())
                    .enumerate()
                {
                    assert_eq!(
                        a.unwrap().data,
                        b.unwrap().data,
                        "native Save As {stem} program {p}"
                    );
                }
            }
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "requires supplied IEC project and native heating contact refill captures"]
fn xgi_heating_contact_refill_native_capture() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_CHAIN_REFILL_CAPTURE").unwrap());
    let symbols = |doc: &XgwxDocument| {
        doc.iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    };
    for (row, value, output, heights) in [(
        62,
        "1",
        "%MX732",
        [
            (3, 62, 39),
            (4, 62, 39),
            (12, 78, 50),
            (66, 52, 39),
            (67, 56, 39),
            (70, 52, 39),
            (71, 56, 39),
        ],
    )] {
        let block_at = |doc: &XgwxDocument| {
            doc.ladder_programs()
                .remove(6)
                .unwrap()
                .iec_function_blocks()
                .unwrap()
                .into_iter()
                .find(|b| b.row_index == row && b.raw_x == 19)
                .unwrap()
        };
        let mut deleted = source.clone();
        deleted
            .delete_iec_ld_heating_chain_contact_eq(6, block_at(&source).record_offset, "EQ")
            .unwrap();
        let args = ["%MW129".to_owned(), value.to_owned(), output.to_owned()];
        let mut restored = deleted.clone();
        restored
            .insert_iec_ld_function(6, row, 19, "EQ", &args)
            .unwrap();
        assert_eq!(symbols(&source), symbols(&restored));
        let restored_bytes = restored.to_bytes().unwrap();
        let mut removed = restored.clone();
        removed
            .delete_iec_ld_scalar_chain_function(6, block_at(&restored).record_offset, "EQ")
            .unwrap();
        for (candidate, suffix) in [(&restored, "NREF"), (&removed, "NROOM")] {
            let stem = format!("M{row}{suffix}");
            let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
            assert_eq!(native.ladder_programs().len(), 7);
            for (p, (a, b)) in candidate
                .ladder_programs()
                .into_iter()
                .zip(native.ladder_programs())
                .enumerate()
            {
                let (mut a, b) = (a.unwrap(), b.unwrap());
                if p == 6 {
                    let frames = a.iec_row_frames().unwrap();
                    for (y, old, new) in heights {
                        let offset = frames.iter().find(|r| r.row_index == y).unwrap().start + 17;
                        assert_eq!(a.data[offset], old);
                        assert_eq!(b.data[offset], new);
                        a.data[offset] = new;
                    }
                }
                assert_eq!(a.data, b.data, "native {stem} program {p}");
            }
        }
        let removed_bytes = removed.to_bytes().unwrap();
        removed
            .insert_iec_ld_function(6, row, 19, "EQ", &args)
            .unwrap();
        assert_eq!(
            removed.to_bytes().unwrap(),
            restored_bytes,
            "repeat Delete/refill at {row}"
        );
        for bad in ["%MW0", "_ON"] {
            let before = deleted.to_bytes().unwrap();
            assert!(
                deleted
                    .insert_iec_ld_function(
                        6,
                        row,
                        19,
                        "EQ",
                        &[args[0].clone(), args[1].clone(), bad.to_owned()]
                    )
                    .is_err()
            );
            assert_eq!(deleted.to_bytes().unwrap(), before);
        }
        if let Some(output) = env::var_os("LIBXGWX_CHAIN_REFILL_OUTPUT") {
            let output = std::path::PathBuf::from(output);
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(output.join(format!("M{row}GEN.xgwx")), restored_bytes).unwrap();
            std::fs::write(output.join(format!("M{row}GDEL.xgwx")), removed_bytes).unwrap();
        }
        if env::var_os("LIBXGWX_CHAIN_REFILL_SAVE_CAPTURE").is_some() {
            let mut removed = restored.clone();
            removed
                .delete_iec_ld_scalar_chain_function(6, block_at(&restored).record_offset, "EQ")
                .unwrap();
            for (candidate, suffix) in [(&restored, "GENS"), (&removed, "GDELS")] {
                let stem = format!("M{row}{suffix}");
                let native = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
                assert_eq!(native.ladder_programs().len(), 7);
                assert_eq!(
                    symbols(candidate),
                    symbols(&native),
                    "native Save As symbols {stem}"
                );
                for (p, (a, b)) in candidate
                    .ladder_programs()
                    .into_iter()
                    .zip(native.ladder_programs())
                    .enumerate()
                {
                    assert_eq!(
                        a.unwrap().data,
                        b.unwrap().data,
                        "native Save As {stem} program {p}"
                    );
                }
            }
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "requires supplied IEC project and native combined contact/comparison captures"]
fn xgi_combined_contact_comparison_native_capture() {
    let capture = std::path::PathBuf::from(env::var("LIBXGWX_CHAIN_REFILL_CAPTURE").unwrap());
    let source = XgwxDocument::from_path(capture.join("FD6R62X19.xgwx")).unwrap();
    let generated = XgwxDocument::from_path(capture.join("M62GEN.xgwx")).unwrap();
    let symbols = |doc: &XgwxDocument| {
        doc.iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    };
    let native_equal = |candidate: &XgwxDocument, stem: &str| {
        let native =
            XgwxDocument::from_path(capture.join(format!("combined/{stem}.xgwx"))).unwrap();
        for (p, (a, b)) in candidate
            .ladder_programs()
            .into_iter()
            .zip(native.ladder_programs())
            .enumerate()
        {
            let (mut a, b) = (a.unwrap(), b.unwrap());
            if p == 6 {
                let frames = a.iec_row_frames().unwrap();
                for (row, old, new) in [
                    (3, 62, 39),
                    (4, 62, 39),
                    (12, 78, 50),
                    (66, 52, 39),
                    (67, 56, 39),
                    (70, 52, 39),
                    (71, 56, 39),
                ] {
                    let at = frames.iter().find(|r| r.row_index == row).unwrap().start + 17;
                    assert_eq!(a.data[at], old);
                    assert_eq!(b.data[at], new);
                    a.data[at] = new;
                }
            }
            assert_eq!(a.data, b.data, "combined native program {p}");
        }
    };
    for mask in 1..=7 {
        let mut doc = source.clone();
        let mut deleted = Vec::new();
        for (bit, x) in [(1, 7), (2, 10), (4, 13)] {
            if mask & bit == 0 {
                continue;
            }
            let p = doc.ladder_programs().remove(6).unwrap();
            let r = p
                .iec_record_frames()
                .unwrap()
                .into_iter()
                .find(|r| {
                    r.row_index == 62
                        && matches!(r.kind, IecRecordKind::Contact(_))
                        && p.data[r.offset + 5] == x
                })
                .unwrap();
            let IecRecordKind::Contact(code) = r.kind else {
                unreachable!()
            };
            let kind = [
                "NO",
                "NC",
                "RISING",
                "FALLING",
                "NEGATED_RISING",
                "NEGATED_FALLING",
            ][usize::from(code - 6)];
            let value = String::from_utf16(
                &p.data[r.offset + 19..r.end]
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    .collect::<Vec<_>>(),
            )
            .unwrap();
            doc.delete_iec_ld_contact(6, r.offset, x, kind, &value)
                .unwrap();
            deleted.push((x, kind, value));
        }
        let args = ["%MW129".to_owned(), "1".to_owned(), "%MX732".to_owned()];
        doc.insert_iec_ld_function(6, 62, 19, "EQ", &args).unwrap();
        let placed = doc.to_bytes().unwrap();
        let block = doc
            .ladder_programs()
            .remove(6)
            .unwrap()
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == 62 && b.raw_x == 19)
            .unwrap();
        doc.delete_iec_ld_scalar_chain_function(6, block.record_offset, "EQ")
            .unwrap();
        doc.insert_iec_ld_function(6, 62, 19, "EQ", &args).unwrap();
        assert_eq!(
            doc.to_bytes().unwrap(),
            placed,
            "repeat with missing contacts mask {mask}"
        );
        if matches!(mask, 1 | 4) {
            native_equal(&doc, if mask == 1 { "C62N7" } else { "C62N13" });
        }
        for (x, kind, value) in deleted {
            doc.insert_iec_ld_single_element(6, 62, x, "contact", kind, &value)
                .unwrap();
        }
        if mask == 1 {
            native_equal(&doc, "C62N7F");
        }
        assert_eq!(symbols(&doc), symbols(&source));
        for (a, b) in doc
            .ladder_programs()
            .into_iter()
            .zip(generated.ladder_programs())
        {
            assert_eq!(
                a.unwrap().data,
                b.unwrap().data,
                "completed combined mask {mask}"
            );
        }
    }
    if env::var_os("LIBXGWX_COMBINED_SAVE_CAPTURE").is_some() {
        for stem in ["C62R7", "C62G7", "C62R13", "C62G13"] {
            let candidate =
                XgwxDocument::from_path(capture.join(format!("combined/{stem}.xgwx"))).unwrap();
            let native =
                XgwxDocument::from_path(capture.join(format!("combined/{stem}S.xgwx"))).unwrap();
            assert_eq!(
                symbols(&candidate),
                symbols(&native),
                "native combined Save As locals {stem}"
            );
            assert_eq!(native.ladder_programs().len(), 7);
            for (p, (a, b)) in candidate
                .ladder_programs()
                .into_iter()
                .zip(native.ladder_programs())
                .enumerate()
            {
                assert_eq!(
                    a.unwrap().data,
                    b.unwrap().data,
                    "native combined Save As {stem} program {p}"
                );
            }
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "requires external XGI project and optional native shifted captures"]
fn xgi_shifted_heating_comparison_deletion() {
    use std::{env, path::PathBuf};
    type Delete = fn(&mut XgwxDocument, usize, usize, &str) -> Result<(), XgwxError>;
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let routes: [(u16, Delete); 5] = [
        (46, XgwxDocument::delete_iec_ld_heating_chain_head),
        (50, XgwxDocument::delete_iec_ld_heating_chain_middle),
        (54, XgwxDocument::delete_iec_ld_heating_chain_middle),
        (58, XgwxDocument::delete_iec_ld_heating_chain_x3_eq_repaired),
        (62, XgwxDocument::delete_iec_ld_heating_chain_contact_eq),
    ];
    let offset = |document: &XgwxDocument, row| {
        document
            .ladder_programs()
            .remove(6)
            .unwrap()
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|block| block.row_index == row && block.raw_x == 19)
            .unwrap()
            .record_offset
    };
    let locals = |document: &XgwxDocument| {
        document
            .iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    };
    let payloads = |document: &XgwxDocument| {
        document
            .ladder_programs()
            .into_iter()
            .map(|p| p.unwrap().data)
            .collect::<Vec<_>>()
    };
    for extra_comment in [false, true] {
        for (row, remove) in routes {
            let mut original_delete = source.clone();
            remove(&mut original_delete, 6, offset(&source, row), "EQ").unwrap();
            original_delete.insert_iec_ld_blank_row(6, 45).unwrap();
            if extra_comment {
                original_delete
                    .insert_iec_ld_comment(6, 46, "shift audit")
                    .unwrap();
            }
            let mut shifted = source.clone();
            shifted.insert_iec_ld_blank_row(6, 45).unwrap();
            if extra_comment {
                shifted.insert_iec_ld_comment(6, 46, "shift audit").unwrap();
            }
            let before = shifted.to_bytes().unwrap();
            let block_offset = offset(&shifted, row + 1);
            assert!(remove(&mut shifted, 6, block_offset, "NE").is_err());
            assert_eq!(shifted.to_bytes().unwrap(), before, "wrong-name rejection");
            remove(&mut shifted, 6, block_offset, "EQ").unwrap();
            assert_eq!(
                payloads(&shifted),
                payloads(&original_delete),
                "shift/delete commute L{row}, comment={extra_comment}"
            );
            assert_eq!(locals(&shifted), locals(&source));
        }
    }
    if let Some(capture) = env::var_os("LIBXGWX_SHIFTED_SAVE_CAPTURE") {
        let capture = PathBuf::from(capture);
        for stem in [
            "SHIFT6", "SHD47", "SHR47", "SHD51", "SHR51", "SHD55", "SHR55", "SHD59", "SHR59",
            "SHD63", "SHR63",
        ] {
            let generated = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
            let native = XgwxDocument::from_path(capture.join(format!("{stem}S.xgwx"))).unwrap();
            assert_eq!(payloads(&generated).len(), 7);
            assert_eq!(
                payloads(&generated),
                payloads(&native),
                "shifted native Save As {stem}"
            );
            assert_eq!(
                locals(&generated),
                locals(&native),
                "shifted native locals {stem}"
            );
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "requires external XGI project for contact-kind lifecycle coverage"]
fn xgi_contact_kind_comparison_lifecycle() {
    use std::env;
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let payloads = |document: &XgwxDocument| {
        document
            .ladder_programs()
            .into_iter()
            .map(|p| p.unwrap().data)
            .collect::<Vec<_>>()
    };
    let locals = |document: &XgwxDocument| {
        document
            .iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    };
    let offset = |document: &XgwxDocument, row| {
        document
            .ladder_programs()
            .remove(6)
            .unwrap()
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row && b.raw_x == 19)
            .unwrap()
            .record_offset
    };
    let contact = |document: &XgwxDocument, row, x| {
        let program = document.ladder_programs().remove(6).unwrap();
        program
            .iec_record_frames()
            .unwrap()
            .into_iter()
            .find(|r| {
                r.row_index == row
                    && matches!(r.kind, IecRecordKind::Contact(6..=11))
                    && program.data[r.offset + 5] == x
            })
            .unwrap()
            .offset
            + 15
    };
    let args = vec!["%MW129".into(), "1".into(), "%MX732".into()];
    for shifted in [false, true] {
        let delta = u16::from(shifted);
        let mut base = source.clone();
        if shifted {
            base.insert_iec_ld_blank_row(6, 45).unwrap();
        }
        let mut deleted_base = base.clone();
        deleted_base
            .delete_iec_ld_heating_chain_contact_eq(6, offset(&base, 62 + delta), "EQ")
            .unwrap();
        for (row, x, original_kind, retained) in [
            (62, 7, "NO", true),
            (62, 10, "NC", true),
            (62, 13, "NC", true),
            (63, 7, "NO", false),
        ] {
            for kind in [
                "NO",
                "NC",
                "RISING",
                "FALLING",
                "NEGATED_RISING",
                "NEGATED_FALLING",
            ] {
                let mut edited = base.clone();
                edited
                    .update_iec_ld_contact_kind(
                        6,
                        contact(&edited, row + delta, x),
                        original_kind,
                        kind,
                    )
                    .unwrap();
                let block_offset = offset(&edited, 62 + delta);
                let before = edited.to_bytes().unwrap();
                assert!(
                    edited
                        .delete_iec_ld_heating_chain_contact_eq(6, block_offset, "NE")
                        .is_err()
                );
                assert_eq!(edited.to_bytes().unwrap(), before);
                edited
                    .delete_iec_ld_heating_chain_contact_eq(6, block_offset, "EQ")
                    .unwrap();
                let mut expected = deleted_base.clone();
                if retained {
                    expected
                        .update_iec_ld_contact_kind(
                            6,
                            contact(&expected, row + delta, x),
                            original_kind,
                            kind,
                        )
                        .unwrap();
                }
                assert_eq!(
                    payloads(&edited),
                    payloads(&expected),
                    "kind/delete commute L{row}, x{x}, {kind}, shifted={shifted}"
                );
                assert_eq!(locals(&edited), locals(&source));
                edited
                    .insert_iec_ld_function(6, 62 + delta, 19, "EQ", &args)
                    .unwrap();
                let filled = edited.to_bytes().unwrap();
                let block_offset = offset(&edited, 62 + delta);
                edited
                    .delete_iec_ld_scalar_chain_function(6, block_offset, "EQ")
                    .unwrap();
                edited
                    .insert_iec_ld_function(6, 62 + delta, 19, "EQ", &args)
                    .unwrap();
                assert_eq!(edited.to_bytes().unwrap(), filled);
            }
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "requires external XGI project for contact-first comparison edits"]
fn xgi_contact_first_comparison_lifecycle() {
    use std::{env, fs, path::PathBuf};
    let source = XgwxDocument::from_path(env::var("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let output = env::var_os("LIBXGWX_CONTACT_FIRST_OUTPUT").map(PathBuf::from);
    if let Some(output) = &output {
        fs::create_dir_all(output).unwrap();
    }
    let payloads = |d: &XgwxDocument| {
        d.ladder_programs()
            .into_iter()
            .map(|p| p.unwrap().data)
            .collect::<Vec<_>>()
    };
    let locals = |d: &XgwxDocument| {
        d.iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    };
    let block = |d: &XgwxDocument, row| {
        d.ladder_programs()
            .remove(6)
            .unwrap()
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == row && b.raw_x == 19)
            .unwrap()
            .record_offset
    };
    let contact = |d: &XgwxDocument, row, x| {
        let p = d.ladder_programs().remove(6).unwrap();
        let r = p
            .iec_record_frames()
            .unwrap()
            .into_iter()
            .find(|r| {
                r.row_index == row
                    && matches!(r.kind, IecRecordKind::Contact(6..=11))
                    && p.data[r.offset + 5] == x
            })
            .unwrap();
        let IecRecordKind::Contact(code) = r.kind else {
            unreachable!()
        };
        let kind = [
            "NO",
            "NC",
            "RISING",
            "FALLING",
            "NEGATED_RISING",
            "NEGATED_FALLING",
        ][usize::from(code - 6)];
        let variable = String::from_utf16(
            &p.data[r.offset + 19..r.end]
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        (r.offset, kind, variable)
    };
    let args = vec!["%MW129".into(), "1".into(), "%MX732".into()];
    for shifted in [false, true] {
        let delta = u16::from(shifted);
        let row = 62 + delta;
        let mut base = source.clone();
        if shifted {
            base.insert_iec_ld_blank_row(6, 45).unwrap();
        }
        let sites = [(row, 7), (row, 10), (row, 13), (row + 1, 7)];
        let mut comparison_first = base.clone();
        comparison_first
            .delete_iec_ld_heating_chain_contact_eq(6, block(&base, row), "EQ")
            .unwrap();
        for mask in 1..16 {
            let emit = |state, d: &XgwxDocument| {
                if !shifted && let Some(output) = &output {
                    d.write_to(output.join(format!("A{mask}{state}.xgwx")))
                        .unwrap();
                }
            };
            let mut edited = base.clone();
            let mut expected = comparison_first.clone();
            for (index, &(r, x)) in sites.iter().enumerate() {
                if mask & (1 << index) == 0 {
                    continue;
                }
                let (offset, kind, variable) = contact(&edited, r, x);
                edited
                    .delete_iec_ld_contact(6, offset, x, kind, &variable)
                    .unwrap();
                if index < 3 {
                    let (offset, kind, variable) = contact(&expected, r, x);
                    expected
                        .delete_iec_ld_contact(6, offset, x, kind, &variable)
                        .unwrap();
                }
            }
            emit("C", &edited);
            let before = edited.to_bytes().unwrap();
            let offset = block(&edited, row);
            assert!(
                edited
                    .delete_iec_ld_heating_chain_contact_eq(6, offset, "NE")
                    .is_err()
            );
            assert_eq!(edited.to_bytes().unwrap(), before);
            edited
                .delete_iec_ld_heating_chain_contact_eq(6, offset, "EQ")
                .unwrap();
            let mut expected_payloads = payloads(&expected);
            if mask & 7 == 7 {
                // Native Delete Line prunes the x4 feed after the last top
                // contact has gone; comparison-first contact deletion retains it.
                let p = expected.ladder_programs().remove(6).unwrap();
                let frame = p
                    .iec_row_frames()
                    .unwrap()
                    .into_iter()
                    .find(|r| r.row_index == row)
                    .unwrap();
                let wire = p
                    .iec_record_frames()
                    .unwrap()
                    .into_iter()
                    .find(|r| r.row_index == row && r.kind == IecRecordKind::ShortWire)
                    .unwrap();
                assert_eq!(frame.record_count, 3);
                assert_eq!(wire.end - wire.offset, 15);
                assert_eq!(p.data[wire.offset + 5], 4);
                expected_payloads[6][frame.start + 33..frame.start + 35]
                    .copy_from_slice(&2u16.to_le_bytes());
                expected_payloads[6].drain(wire.offset..wire.end);
            }
            assert_eq!(
                payloads(&edited),
                expected_payloads,
                "native contact cleanup mask {mask}, shifted {shifted}"
            );
            assert_eq!(locals(&edited), locals(&base));
            emit("D", &edited);
            edited
                .insert_iec_ld_function(6, row, 19, "EQ", &args)
                .unwrap();
            expected
                .insert_iec_ld_function(6, row, 19, "EQ", &args)
                .unwrap();
            assert_eq!(payloads(&edited), payloads(&expected), "refill mask {mask}");
            emit("R", &edited);
            let filled = edited.to_bytes().unwrap();
            edited
                .delete_iec_ld_scalar_chain_function(6, block(&edited, row), "EQ")
                .unwrap();
            edited
                .insert_iec_ld_function(6, row, 19, "EQ", &args)
                .unwrap();
            assert_eq!(
                edited.to_bytes().unwrap(),
                filled,
                "repeated refill mask {mask}, shifted {shifted}"
            );
            for (index, &(r, x)) in sites.iter().take(3).enumerate() {
                if mask & (1 << index) != 0 {
                    let (_, kind, variable) = contact(&base, r, x);
                    edited
                        .insert_iec_ld_single_element(6, r, x, "contact", kind, &variable)
                        .unwrap();
                }
            }
            assert!(
                edited
                    .ladder_programs()
                    .remove(6)
                    .unwrap()
                    .iec_circuit_graph()
                    .is_some()
            );
            assert_eq!(locals(&edited), locals(&base));
            emit("F", &edited);
        }
    }
}

#[cfg(feature = "write")]
#[test]
#[ignore = "requires native contact-first captures and the source fixture"]
fn xgi_contact_first_native_refill_lifecycle() {
    use std::{env, path::PathBuf};
    let capture = PathBuf::from(env::var_os("LIBXGWX_CONTACT_FIRST_SAVE_CAPTURE").unwrap());
    let source =
        XgwxDocument::from_path(env::var_os("LIBXGWX_SMARTHOME_FIXTURE").unwrap()).unwrap();
    let original = source.ladder_programs().remove(6).unwrap();
    let elements = crate::iec_ld::element_operands(&original);
    let records = original.iec_record_frames().unwrap();
    let sites = [7, 10, 13].map(|x| {
        let record = records
            .iter()
            .find(|r| {
                r.row_index == 62
                    && matches!(r.kind, IecRecordKind::Contact(6..=11))
                    && original.data[r.offset + 5] == x
            })
            .unwrap();
        let value = elements
            .iter()
            .find(|e| e.string.offset == record.offset + 15)
            .unwrap()
            .string
            .value
            .clone();
        (
            x,
            if record.kind == IecRecordKind::Contact(6) {
                "NO"
            } else {
                "NC"
            },
            value,
        )
    });
    let args = vec!["%MW129".into(), "1".into(), "%MX732".into()];
    for mask in [1, 8, 7] {
        let mut edited = XgwxDocument::from_path(capture.join(format!("A{mask}NS.xgwx"))).unwrap();
        let locals = edited
            .iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        edited
            .insert_iec_ld_function(6, 62, 19, "EQ", &args)
            .unwrap();
        edited
            .write_to(capture.join(format!("N{mask}R.xgwx")))
            .unwrap();
        let filled = edited.to_bytes().unwrap();
        let offset = edited
            .ladder_programs()
            .remove(6)
            .unwrap()
            .iec_function_blocks()
            .unwrap()
            .into_iter()
            .find(|b| b.row_index == 62 && b.raw_x == 19)
            .unwrap()
            .record_offset;
        edited
            .delete_iec_ld_scalar_chain_function(6, offset, "EQ")
            .unwrap();
        edited
            .insert_iec_ld_function(6, 62, 19, "EQ", &args)
            .unwrap();
        assert_eq!(
            edited.to_bytes().unwrap(),
            filled,
            "native repeat mask {mask}"
        );
        for (index, (x, kind, value)) in sites.iter().enumerate() {
            if mask & (1 << index) != 0 {
                edited
                    .insert_iec_ld_single_element(6, 62, *x, "contact", kind, value)
                    .unwrap();
            }
        }
        assert!(
            edited
                .ladder_programs()
                .remove(6)
                .unwrap()
                .iec_circuit_graph()
                .is_some()
        );
        assert_eq!(
            edited
                .iec_local_symbols()
                .into_iter()
                .map(Result::unwrap)
                .collect::<Vec<_>>(),
            locals
        );
        edited
            .write_to(capture.join(format!("N{mask}F.xgwx")))
            .unwrap();
    }
}

#[test]
#[ignore = "requires twenty-one native contact-first Save As captures"]
fn xgi_contact_first_native_save_capture() {
    use std::{env, path::PathBuf};
    let capture = PathBuf::from(
        env::var_os("LIBXGWX_CONTACT_FIRST_SAVE_CAPTURE")
            .expect("set native contact-first capture directory"),
    );
    for mask in [1, 2, 4, 7, 8] {
        for state in ["D", "R", "F"] {
            let stem = format!("A{mask}{state}");
            let generated = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
            // Preserve A7DS as pre-cleanup evidence; B7DS contains the fix.
            let native_stem = if mask == 7 && state == "D" {
                "B7D"
            } else {
                &stem
            };
            let native =
                XgwxDocument::from_path(capture.join(format!("{native_stem}S.xgwx"))).unwrap();
            let programs = |d: &XgwxDocument| {
                d.ladder_programs()
                    .into_iter()
                    .map(Result::unwrap)
                    .collect::<Vec<_>>()
            };
            let old = programs(&generated);
            let new = programs(&native);
            assert_eq!(old.len(), 7);
            assert_eq!(new.len(), 7);
            for (index, (before, after)) in old.iter().zip(&new).enumerate() {
                assert_eq!(before.data, after.data, "native {stem} program {index}");
                if state == "F" {
                    assert!(
                        after.iec_circuit_graph().is_some(),
                        "completed {stem} program {index}"
                    );
                }
            }
            let locals = |d: &XgwxDocument| {
                d.iec_local_symbols()
                    .into_iter()
                    .map(Result::unwrap)
                    .collect::<Vec<_>>()
            };
            assert_eq!(locals(&generated), locals(&native), "native locals {stem}");
        }
    }
    for mask in [1, 8, 7] {
        for state in ["R", "F"] {
            let stem = format!("N{mask}{state}");
            let generated = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
            let native = XgwxDocument::from_path(capture.join(format!("{stem}S.xgwx"))).unwrap();
            let payloads = |d: &XgwxDocument| {
                d.ladder_programs()
                    .into_iter()
                    .map(|p| p.unwrap().data)
                    .collect::<Vec<_>>()
            };
            assert_eq!(payloads(&generated).len(), 7);
            assert_eq!(payloads(&native).len(), 7);
            assert_eq!(
                payloads(&generated),
                payloads(&native),
                "native imported refill {stem}"
            );
            let locals = |d: &XgwxDocument| {
                d.iec_local_symbols()
                    .into_iter()
                    .map(Result::unwrap)
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                locals(&generated),
                locals(&native),
                "native imported locals {stem}"
            );
            if state == "F" {
                assert!(
                    native
                        .ladder_programs()
                        .remove(6)
                        .unwrap()
                        .iec_circuit_graph()
                        .is_some()
                );
            }
        }
    }
}

#[test]
#[ignore = "requires native contact-first Delete Line captures"]
fn xgi_contact_first_native_delete_capture() {
    use std::{env, path::PathBuf};
    let capture = PathBuf::from(
        env::var_os("LIBXGWX_CONTACT_FIRST_SAVE_CAPTURE")
            .expect("set native contact-first capture directory"),
    );
    let payloads = |d: &XgwxDocument| {
        d.ladder_programs()
            .into_iter()
            .map(|p| p.unwrap().data)
            .collect::<Vec<_>>()
    };
    let locals = |d: &XgwxDocument| {
        d.iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    };
    for mask in [1, 8, 7] {
        let generated = XgwxDocument::from_path(capture.join(format!("A{mask}D.xgwx"))).unwrap();
        let native = XgwxDocument::from_path(capture.join(format!("A{mask}NS.xgwx"))).unwrap();
        let mut expected_locals = locals(&generated);
        let observed_locals = locals(&native);
        assert_eq!(expected_locals.len(), 7);
        assert_eq!(expected_locals[6].len(), 3);
        assert_eq!(observed_locals[6].len(), 3);
        for (index, old, new) in [(1, 124, 102), (2, 244, 200)] {
            assert_eq!(expected_locals[6][index].record_offset, old);
            assert_eq!(observed_locals[6][index].record_offset, new);
            expected_locals[6][index].record_offset = new;
        }
        assert_eq!(
            expected_locals, observed_locals,
            "manual locals mask {mask}"
        );
        let mut expected = payloads(&generated);
        let observed = payloads(&native);
        assert_eq!(expected.len(), 7);
        assert_eq!(observed.len(), 7);
        let program = generated.ladder_programs().remove(6).unwrap();
        let rows = program.iec_row_frames().unwrap();
        for (row, old, new) in [
            (3, 62, 39),
            (4, 62, 39),
            (12, 78, 50),
            (62, 39, 73),
            (65, 52, 39),
            (66, 56, 39),
        ] {
            let offset = rows.iter().find(|r| r.row_index == row).unwrap().start + 17;
            assert_eq!(expected[6][offset], old, "generated L{row} height");
            expected[6][offset] = new;
        }
        if mask == 7 {
            // The writer must reproduce native's depleted two-record feed.
            let row = rows.iter().find(|r| r.row_index == 62).unwrap();
            assert_eq!(row.record_count, 2);
            let records = program.iec_record_frames().unwrap();
            assert!(
                !records
                    .iter()
                    .any(|r| r.row_index == 62 && r.kind == IecRecordKind::ShortWire)
            );
        }
        assert_eq!(
            expected, observed,
            "native contact-first Delete Line mask {mask}"
        );
    }
}

#[test]
#[ignore = "requires native Save As captures for all six contact kinds"]
fn xgi_contact_kind_native_save_capture() {
    use std::{env, path::PathBuf};
    let payloads = |document: &XgwxDocument| {
        document
            .ladder_programs()
            .into_iter()
            .map(|p| p.unwrap().data)
            .collect::<Vec<_>>()
    };
    let locals = |document: &XgwxDocument| {
        document
            .iec_local_symbols()
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    };
    let capture = PathBuf::from(
        env::var_os("LIBXGWX_CONTACT_KIND_SAVE_CAPTURE")
            .expect("set native contact-kind capture directory"),
    );
    for code in 6..=11 {
        for state in ["C", "D", "R"] {
            let stem = format!("K{code}{state}");
            let generated = XgwxDocument::from_path(capture.join(format!("{stem}.xgwx"))).unwrap();
            let native = XgwxDocument::from_path(capture.join(format!("{stem}S.xgwx"))).unwrap();
            assert_eq!(payloads(&generated).len(), 7);
            assert_eq!(
                payloads(&generated),
                payloads(&native),
                "native contact-kind Save As {stem}"
            );
            assert_eq!(
                locals(&generated),
                locals(&native),
                "native contact-kind locals {stem}"
            );
        }
    }
    // Direct native Delete Line refreshes these six cached row heights.
    // Assert the exact captured changes before comparing every remaining byte.
    let generated = XgwxDocument::from_path(capture.join("K6D.xgwx")).unwrap();
    let native = XgwxDocument::from_path(capture.join("K6NS.xgwx")).unwrap();
    // Native interactive editing also rewrites the local-symbol record framing.
    // Only these two offsets change; all symbol values and allocations match.
    let mut expected_locals = locals(&generated);
    let observed_locals = locals(&native);
    assert_eq!(expected_locals[6].len(), 3);
    assert_eq!(observed_locals[6].len(), 3);
    for (index, old, new) in [(1, 124, 102), (2, 244, 200)] {
        assert_eq!(expected_locals[6][index].record_offset, old);
        assert_eq!(observed_locals[6][index].record_offset, new);
        expected_locals[6][index].record_offset = new;
    }
    assert_eq!(expected_locals, observed_locals);
    let mut expected = payloads(&generated);
    let observed = payloads(&native);
    let program = generated.ladder_programs().remove(6).unwrap();
    let rows = program.iec_row_frames().unwrap();
    for (row, old, new) in [
        (3, 62, 39),
        (4, 62, 39),
        (12, 78, 50),
        (62, 39, 73),
        (65, 52, 39),
        (66, 56, 39),
    ] {
        let offset = rows.iter().find(|r| r.row_index == row).unwrap().start + 17;
        assert_eq!(expected[6][offset], old, "generated L{row} height");
        assert_eq!(observed[6][offset], new, "native L{row} height");
        expected[6][offset] = new;
    }
    assert_eq!(
        expected, observed,
        "native manual contact-fed comparison deletion"
    );
}

#[cfg(feature = "write")]
#[test]
#[ignore = "set LIBXGWX_IEC_OPEN_FIXTURE to a native-saved IEC project with exposed endpoints"]
fn xgi_fresh_functions_preserve_unrelated_open_networks() {
    let source = XgwxDocument::from_path(env::var("LIBXGWX_IEC_OPEN_FIXTURE").unwrap()).unwrap();
    let before = source.ladder_programs().remove(0).unwrap();
    let layout = before.iec_circuit_layout().unwrap();
    assert!(!layout.open_branch_endpoints.is_empty());
    let row = u16::from_le_bytes(before.data[4..6].try_into().unwrap()) + 2;
    for (name, operands) in [
        ("MOVE", vec!["1".into(), "%MW100".into()]),
        ("ADD", vec!["1".into(), "2".into(), "%MW100".into()]),
        ("EQ", vec!["1".into(), "2".into(), "%MX100".into()]),
    ] {
        let mut edited = source.clone();
        edited
            .insert_iec_ld_function(0, row, 4, name, &operands)
            .unwrap();
        let after = edited.ladder_programs().remove(0).unwrap();
        assert_eq!(&before.data[8..], &after.data[8..before.data.len()]);
        let observed = after.iec_circuit_layout().unwrap();
        assert_eq!(observed.open_branch_endpoints, layout.open_branch_endpoints);
        assert_eq!(
            observed.function_bindings[..layout.function_bindings.len()],
            layout.function_bindings
        );
    }
    let mut rejected = source.clone();
    let bytes = rejected.to_bytes().unwrap();
    assert!(
        rejected
            .insert_iec_ld_function(0, 69, 4, "MOVE", &["1".into(), "%MW100".into()])
            .is_err()
    );
    assert_eq!(
        bytes,
        rejected.to_bytes().unwrap(),
        "rejected overlap is atomic"
    );
}

#[cfg(feature = "write")]
#[test]
fn documented_instruction_cpu_rejections_are_atomic() {
    let mut doc = XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
    doc.select_cpu("XGK-CPUH").unwrap();
    let original = doc.to_bytes().unwrap();
    let operands = vec!["0".into(), "D100".into()];
    assert!(doc.insert_ladder_instruction(0, 52, "INLATCH", &operands).is_err());
    assert_eq!(doc.to_bytes().unwrap(), original);
    doc.insert_ladder_instruction(0, 52, "MOV", &operands).unwrap();
    let program = doc.ladder_programs().remove(0).unwrap();
    let instruction = program.strings.iter().find(|s| s.value == "MOV,0,D100").unwrap();
    let (offset, expected) = (instruction.offset, instruction.value.clone());
    let before = doc.to_bytes().unwrap();
    assert!(doc.update_ladder_cell_text(0, offset, &expected, "INLATCH,0,D100").is_err());
    assert_eq!(doc.to_bytes().unwrap(), before);
    doc.select_cpu("XGK-CPUHN").unwrap();
    doc.update_ladder_cell_text(0, offset, &expected, "INLATCH,0,D100").unwrap();
    // Existing commands can still have their operands repaired after a CPU change.
    doc.select_cpu("XGK-CPUH").unwrap();
    doc.update_ladder_cell_text(0, offset, "INLATCH,0,D100", "INLATCH,0,D200").unwrap();
}

#[cfg(feature = "write")]
#[test]
fn verified_serialization_detects_inconsistent_xml_tree() {
    let source = std::fs::read("fixtures/elements.xgwx").unwrap();
    let mut doc = XgwxDocument::parse(&source).unwrap();
    assert_eq!(doc.to_verified_bytes().unwrap(), source);
    // A caller changing the public XML without updating its parsed tree must
    // not receive bytes marked as verified.
    doc.xml = doc.xml.replace("<Project ", "<Project Unexpected=\"changed\" ");
    assert!(matches!(doc.to_verified_bytes(), Err(XgwxError::RewriteVerificationFailed)));
}

#[cfg(feature = "write")]
fn browser_iec_test_document() -> XgwxDocument {
    use base64::Engine;
    fn element(code: u8, x: u8, name: &str) -> Vec<u8> {
        let mut bytes = vec![0xff, code, 0, 0, 0, x, 0, 0, 0, 1, 0, if code >= 0x0e {0x20} else {0}, 0, 0, 0];
        bytes.extend_from_slice(&[0xff, 0xfe, 0xff, name.encode_utf16().count() as u8]);
        for unit in name.encode_utf16() {bytes.extend_from_slice(&unit.to_le_bytes());}
        bytes
    }
    let mut contacts = vec![0,0,0,0,1,0,1,0, 0,0,0,0,0,0,0,0,1,0];
    let mut row = crate::iec_function_write::row_header(0);
    row[33..35].copy_from_slice(&3u16.to_le_bytes());
    contacts.extend(row); contacts.extend(element(6,1,"SwitchA"));
    contacts.extend(crate::iec_function_write::wire(0,4,91));
    contacts.extend(element(14,94,"LampA"));
    let mut blank = vec![0,0,0,0,1,0,1,0, 0,0,0,0,0,0,0,0,1,0];
    blank.extend(crate::iec_function_write::row_header(0));
    let xml = format!("<Project GUID=\"browser-qa\">Independent IEC QA<Unknown Keep=\"exact\">OPAQUE</Unknown><Program Comment=\"Contact demo\">Signals<LocalVar><Symbols Count=\"0\"> </Symbols></LocalVar><ProgramData Version=\"LD VER 1.1\" ProjectType=\"2\" Compressed=\"false\">{}</ProgramData></Program><Program Comment=\"Function demo\">Math<LocalVar><Symbols Count=\"0\"> </Symbols></LocalVar><ProgramData Version=\"LD VER 1.1\" ProjectType=\"2\" Compressed=\"false\">{}</ProgramData></Program></Project>",base64::engine::general_purpose::STANDARD.encode(contacts),base64::engine::general_purpose::STANDARD.encode(blank));
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());encoder.write_all(xml.as_bytes()).unwrap();let gzip=encoder.finish().unwrap();
    let mut header=vec![0;138];header[..6].copy_from_slice(&[88,71,255,254,255,20]);
    for (i,u) in "XG5000 WORKSPACE FILE".encode_utf16().enumerate(){header[6+i*2..8+i*2].copy_from_slice(&u.to_le_bytes());}
    header[46..50].copy_from_slice(&1u32.to_le_bytes());let aligned=(gzip.len()+3)&!3;
    header[134..138].copy_from_slice(&(aligned as u32).to_le_bytes());
    header.extend(gzip);header.resize(138+aligned,0);header.extend(b"independent-opaque-trailer");
    let mut doc = XgwxDocument::parse(&header).unwrap();
    for (name,kind) in [("SwitchA","BOOL"),("SwitchB","BOOL"),("LongerSwitch","BOOL"),("LampA","BOOL"),("LampB","BOOL"),("NumberA","INT"),("InputOnly","BOOL")] {
        doc.insert_iec_local_symbol(0,name,kind,"").unwrap();
    }
    let input_index = doc.iec_local_symbols().remove(0).unwrap().iter().position(|s|s.name=="InputOnly").unwrap();
    doc.update_iec_local_symbol_address(0,input_index,"InputOnly","","%IX0.0.0").unwrap();
    for name in ["NumberA","NumberB","NumberC","NumberD"] {doc.insert_iec_local_symbol(1,name,"INT","").unwrap();}
    doc.insert_iec_local_symbol(1,"BoolFlag","BOOL","").unwrap();
    doc.insert_iec_ld_function(1,0,10,"ADD",&["NumberA".into(),"NumberB".into(),"NumberC".into()]).unwrap();
    XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap()
}

#[cfg(feature = "write")]
#[test]
fn browser_iec_edits_preserve_exact_non_target_data_and_reject_stale_edits() {
    let source=browser_iec_test_document();
    let before=source.ladder_programs().into_iter().collect::<Result<Vec<_>,_>>().unwrap();
    assert!(before[0].iec_circuit_graph().is_some());
    let contact=crate::iec_ld::element_operands(&before[0]).into_iter().find(|e|e.record_code==6).unwrap();
    let mut edited=source.clone();
    let mut patch=BrowserIecPatch{operation:"kind".into(),offset:contact.string.offset,expected_kind:"NO".into(),expected_value:"SwitchA".into(),replacement:"NC".into()};
    edited.edit_browser_iec(0,&patch).unwrap();
    assert_eq!(edited.ladder_programs().remove(1).unwrap().data,before[1].data);
    let saved=edited.to_verified_bytes().unwrap();
    assert!(edited.edit_browser_iec(0,&patch).is_err());assert_eq!(edited.to_verified_bytes().unwrap(),saved);
    patch.expected_kind="NC".into();patch.operation="elementOperand".into();patch.replacement="LongerSwitch".into();
    edited.edit_browser_iec(0,&patch).unwrap();
    assert_eq!(edited.ladder_programs().remove(1).unwrap().data,before[1].data);
    let snapshot=edited.clone();patch.expected_value="LongerSwitch".into();patch.replacement="NumberA".into();
    assert!(edited.edit_browser_iec(0,&patch).is_err());assert_eq!(edited,snapshot);
    patch.replacement="Undeclared".into();assert!(edited.edit_browser_iec(0,&patch).is_err());assert_eq!(edited,snapshot);
    patch.offset=usize::MAX;assert!(edited.edit_browser_iec(0,&patch).is_err());assert_eq!(edited,snapshot);
    let coil=crate::iec_ld::element_operands(&before[0]).into_iter().find(|e|e.record_code==14).unwrap();
    let readonly=BrowserIecPatch{operation:"elementOperand".into(),offset:coil.string.offset,expected_kind:"OUTPUT".into(),expected_value:"LampA".into(),replacement:"InputOnly".into()};
    assert!(edited.edit_browser_iec(0,&readonly).is_err());assert_eq!(edited,snapshot);
    let function=crate::iec_ld::function_operands(&before[1]).into_iter().find(|e|e.value=="NumberA").unwrap();
    let p=BrowserIecPatch{operation:"functionOperand".into(),offset:function.offset,expected_kind:"ADD".into(),expected_value:"NumberA".into(),replacement:"NumberD".into()};
    edited.edit_browser_iec(1,&p).unwrap();
    let roundtrip=XgwxDocument::parse(&edited.to_verified_bytes().unwrap()).unwrap();
    assert!(crate::iec_ld::function_operands(&roundtrip.ladder_programs().remove(1).unwrap()).iter().any(|e|e.value=="NumberD"));
    // Optional QA export is independently constructed, never third-party data.
    if let Ok(path)=env::var("LIBXGWX_BROWSER_QA_OUT") {std::fs::write(path,source.to_verified_bytes().unwrap()).unwrap();}
}

#[cfg(feature = "write")]
#[test]
fn browser_hardware_edits_are_bounded_and_atomic() {
    let input = crate::xgk_module_catalog().iter().find(|e| e.model == "XGI-D24A/B").unwrap();
    let analog = crate::xgk_module_catalog().iter().find(|e| e.model == "XGF-AD8A").unwrap();
    let xml = format!(r#"<Project>Hardware QA<Configuration Type="17">PLC</Configuration><Parameter Type="IO PARAMETER"><Module Base="0" Slot="1" Id="{}" SubType="0" Name="{}" Comment="Original" Details="{}"/><Module Base="1" Slot="2" Id="{}" SubType="0" Name="{}" Comment="Analog" Details="{}"/><BaseInfo><Base Base="0" SlotCount="4"/><Base Base="1" SlotCount="6"/></BaseInfo></Parameter><Program Comment="Keep">Main<ProgramData dt="bin.base64">YWJj</ProgramData></Program><Unknown Keep="exact"/></Project>"#, input.id, input.name, input.details, analog.id, analog.name, analog.details);
    let mut doc = browser_iec_test_document();doc.xml=xml.clone();doc.root=parse_xml(&xml).unwrap();
    let patch = |op:&str,base,slot,expected:String,replacement:&str,key,index| BrowserHardwarePatch {
        operation:op.into(),base,slot,expected_value:expected,replacement:replacement.into(),key,index
    };
    doc.edit_browser_hardware(&patch("slotCount",0,None,"4".into(),"6",None,None)).unwrap();
    assert_eq!(doc.xml,xml.replace("Base=\"0\" SlotCount=\"4\"", "Base=\"0\" SlotCount=\"6\""));
    let before = doc.xml.clone();
    doc.edit_browser_hardware(&patch("comment",0,Some(1),"Original".into(),"Changed & safe",None,None)).unwrap();
    assert_eq!(doc.xml,before.replace("Comment=\"Original\"", "Comment=\"Changed &amp; safe\""));
    let before = doc.xml.clone();
    doc.edit_browser_hardware(&patch("inputFilter",0,Some(1),input.details.into(),"5",None,None)).unwrap();
    assert_eq!(doc.xml,before.replacen(&format!("Details=\"{}\"",input.details),&format!("Details=\"05{}\"",&input.details[2..]),1));
    let before = doc.xml.clone();
    doc.edit_browser_hardware(&patch("option",1,Some(2),analog.details.into(),"1",Some("channelOperation".into()),Some(3))).unwrap();
    let details=doc.modules().into_iter().find(|m|m.base==Some(1)).unwrap().details.unwrap();
    assert_eq!(&details[..2],"08");assert_eq!(&details[2..],&analog.details[2..]);
    assert_eq!(doc.xml,before.replace(analog.details,&details));
    let saved=XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();assert_eq!(saved.xml,doc.xml);
    let original=doc.to_verified_bytes().unwrap();
    for p in [patch("slotCount",0,None,"4".into(),"8",None,None),patch("slotCount",0,None,"6".into(),"2",None,None),patch("comment",99,Some(1),"Original".into(),"Bad",None,None),patch("inputFilter",0,Some(1),"0500000000000000".into(),"255",None,None),patch("option",1,Some(2),details.clone(),"99",Some("channelOperation".into()),Some(3))] {
        assert!(doc.edit_browser_hardware(&p).is_err());assert_eq!(doc.to_verified_bytes().unwrap(),original);
    }
    let unverified=doc.xml.replace("Type=\"17\"","Type=\"100\"");let mut other=doc.clone();other.xml=unverified.clone();other.root=parse_xml(&unverified).unwrap();
    assert!(other.edit_browser_hardware(&patch("slotCount",0,None,"6".into(),"8",None,None)).is_err());
    assert!(other.edit_browser_hardware(&patch("inputFilter",0,Some(1),"0500000000000000".into(),"10",None,None)).is_err());
}

#[cfg(feature = "write")]
#[test]
fn browser_network_metadata_is_bounded_and_atomic() {
    let xml=r#"<Project><Network Name="LAN" Type="Ethernet" NetworkType="FEnet"><NetworkModule Base="0" Slot="1" ConfigName="PLC" Alias="Station" Description="Original"><Opaque>KEEP</Opaque></NetworkModule></Network><Network Name="Other"/><Program>Keep<ProgramData dt="bin.base64">YWJj</ProgramData></Program></Project>"#;
    let mut doc=browser_iec_test_document();doc.xml=xml.into();doc.root=parse_xml(xml).unwrap();
    let mut p=BrowserNetworkPatch {network_index:0,module:false,base:None,slot:None,field:"name".into(),expected_value:"LAN".into(),replacement:"Plant & line".into()};
    doc.edit_browser_network(&p).unwrap();assert_eq!(doc.xml,xml.replace("Name=\"LAN\"","Name=\"Plant &amp; line\""));
    let saved=doc.clone();assert!(doc.edit_browser_network(&p).is_err());assert_eq!(doc,saved);
    p.module=true;p.base=Some(0);p.slot=Some(1);p.field="alias".into();p.expected_value="Station".into();p.replacement="New station".into();
    doc.edit_browser_network(&p).unwrap();let saved=doc.clone();
    p.field="ipAddress".into();assert!(doc.edit_browser_network(&p).is_err());assert_eq!(doc,saved);
    p.field="alias".into();p.network_index=1;assert!(doc.edit_browser_network(&p).is_err());assert_eq!(doc,saved);
    let bytes=doc.to_verified_bytes().unwrap();let next=XgwxDocument::parse(&bytes).unwrap();assert_eq!(next.xml,doc.xml);
}
