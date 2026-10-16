#![cfg(feature = "write")]
use xgwx::{NetworkFieldEdit, OpcUaFieldEdit, XgwxDocument};
fn project() -> XgwxDocument {
    let mut doc = xgwx::create_project("XGK-CPUSN", "LD").unwrap();
    doc.base_mut()
        .by_number(0)
        .unwrap()
        .insert_module(0, "XGL-EOPCT")
        .unwrap();
    doc
}
fn edit(field: &str, old: &str, new: &str) -> OpcUaFieldEdit {
    OpcUaFieldEdit {
        field: field.into(),
        expected_value: old.into(),
        replacement: new.into(),
    }
}
fn apply(doc: &mut XgwxDocument, changes: &[OpcUaFieldEdit]) -> Result<(), xgwx::XgwxError> {
    doc.network_mut()
        .module_at(0, 0)?
        .opcua()?
        .edit_settings(changes)
}
#[test]
fn insert_edit_reparse_and_delete_eopct() {
    let mut doc = project();
    let before = doc.clone();
    let configs = doc.network().opcua_config_infos();
    assert_eq!(configs.len(), 1);
    assert_eq!(
        configs[0].endpoint.as_deref(),
        Some("opc.tcp://192.168.1.2:4840")
    );
    doc.network_mut().module_at(0, 0).unwrap().opcua().unwrap();
    assert_eq!(doc, before, "selecting a server must not mutate it");
    apply(
        &mut doc,
        &[
            edit("port", "4840", "4841"),
            edit("publishingInterval", "200", "20"),
            edit("samplingInterval", "200", "20"),
            edit(
                "applicationName",
                &configs[0]
                    .fields
                    .iter()
                    .find(|s| s.field == "applicationName")
                    .unwrap()
                    .value,
                "Plant & Line <A>",
            ),
        ],
    )
    .unwrap();
    let bytes = doc.to_verified_bytes().unwrap();
    let mut doc = XgwxDocument::parse(&bytes).unwrap();
    let config = &doc.network().opcua_config_infos()[0];
    assert_eq!(
        config.endpoint.as_deref(),
        Some("opc.tcp://192.168.1.2:4841")
    );
    assert!(config.fields.iter().any(|s| s.value == "Plant & Line <A>"));
    doc.network_mut()
        .module_at(0, 0)
        .unwrap()
        .fenet()
        .unwrap()
        .edit_field(&NetworkFieldEdit {
            field: "ipAddress".into(),
            expected_value: "192.168.1.2".into(),
            replacement: "10.2.3.4".into(),
        })
        .unwrap();
    assert_eq!(
        doc.network().opcua_config_infos()[0].endpoint.as_deref(),
        Some("opc.tcp://10.2.3.4:4841")
    );
    assert_eq!(before.program().programs(), doc.program().programs());
    doc.module_mut().at(0, 0).unwrap().delete().unwrap();
    assert!(doc.network().opcua_config_infos().is_empty());
}
#[test]
fn invalid_and_stale_edits_are_atomic() {
    let mut doc = project();
    let before = doc.clone();
    for change in [
        edit("port", "4840", "502"),
        edit("port", "4840", "65536"),
        edit("port", "4840", "0"),
        edit("samplingInterval", "200", "19"),
        edit("samplingInterval", "200", "1001"),
        edit("nodeIdType", "0", "2"),
        edit("port", "9999", "4841"),
        edit("certificate", "", "x"),
        edit("applicationName", "", "\0"),
    ] {
        assert!(apply(&mut doc, &[edit("port", "4840", "4842"), change]).is_err());
        assert_eq!(doc, before);
    }
}

#[test]
fn native_exported_tags_support_create_update_delete_and_stale_guards() {
    use xgwx::{OpcUaTag, OpcUaTagPatch};
    let mut doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/networks/eopct-tags-native.xgwx"
    ))
    .unwrap();
    let original = doc.clone();
    let tags = doc.network().opcua_tags(0).unwrap();
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0].name, "PumpReady");
    assert_eq!(tags[0].data_type, "BIT");
    assert_eq!(tags[1].address_number, 123);
    let mut tag = tags[1].clone();
    tag.name = "TankLevelEdited".into();
    tag.address_number = 456;
    tag.comment = "Tank & line <B> 수위".into();
    let patch = OpcUaTagPatch {
        configuration_index: 0,
        expected_tags: tags.clone(),
        index: Some(1),
        tag: Some(tag.clone()),
    };
    doc.network_mut().edit_opcua_tag(&patch).unwrap();
    assert_eq!(
        doc.network().opcua_tags(0).unwrap(),
        vec![tags[0].clone(), tag]
    );
    assert!(doc.network_mut().edit_opcua_tag(&patch).is_err());
    let before_add = doc.network().opcua_tags(0).unwrap();
    doc.network_mut()
        .edit_opcua_tag(&OpcUaTagPatch {
            configuration_index: 0,
            expected_tags: before_add.clone(),
            index: None,
            tag: Some(OpcUaTag {
                name: "NewWord".into(),
                data_type: "WORD".into(),
                address_area: "D".into(),
                address_number: 100,
                comment: "New node".into(),
            }),
        })
        .unwrap();
    let bytes = doc.to_verified_bytes().unwrap();
    let mut doc = XgwxDocument::parse(&bytes).unwrap();
    let after_add = doc.network().opcua_tags(0).unwrap();
    assert_eq!(after_add.len(), 3);
    doc.network_mut()
        .edit_opcua_tag(&OpcUaTagPatch {
            configuration_index: 0,
            expected_tags: after_add,
            index: Some(2),
            tag: None,
        })
        .unwrap();
    assert_eq!(doc.network().opcua_tags(0).unwrap(), before_add);
    assert_eq!(
        doc.variable()
            .list()
            .unwrap()
            .iter()
            .map(|v| v.summary().clone())
            .collect::<Vec<_>>(),
        original
            .variable()
            .list()
            .unwrap()
            .iter()
            .map(|v| v.summary().clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(doc.program().programs(), original.program().programs());
    let mut empty = project();
    assert!(empty.network().opcua_tags(0).unwrap().is_empty());
    empty
        .network_mut()
        .edit_opcua_tag(&OpcUaTagPatch {
            configuration_index: 0,
            expected_tags: vec![],
            index: None,
            tag: Some(tags[1].clone()),
        })
        .unwrap();
    assert_eq!(
        empty.network().opcua_tags(0).unwrap(),
        vec![tags[1].clone()]
    );
}

#[test]
fn native_scalar_device_offsets_are_preserved() {
    let mut doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/networks/eopct-scalars-native.xgwx"
    ))
    .unwrap();
    let tags = doc.network().opcua_tags(0).unwrap();
    assert_eq!(
        tags.iter()
            .map(|t| t.data_type.as_str())
            .collect::<Vec<_>>(),
        ["BIT", "WORD", "BYTE", "DWORD", "LWORD"]
    );
    assert_eq!(tags[2].address_number, 16, "BYTE M00010 uses a bit offset");
    for index in 0..tags.len() {
        let expected_tags = doc.network().opcua_tags(0).unwrap();
        let mut tag = expected_tags[index].clone();
        tag.comment.push_str(" edited");
        doc.network_mut()
            .edit_opcua_tag(&xgwx::OpcUaTagPatch {
                configuration_index: 0,
                expected_tags,
                index: Some(index),
                tag: Some(tag),
            })
            .unwrap();
    }
    assert_eq!(
        XgwxDocument::parse(&doc.to_verified_bytes().unwrap())
            .unwrap()
            .network()
            .opcua_tags(0)
            .unwrap()
            .len(),
        5
    );
}

#[test]
fn opcua_identity_and_unknown_attributes_are_guarded() {
    let mut doc = project();
    let xml = doc.container().xml().replace(
        "<OPCUA_Basic ",
        "<OPCUA_Basic FutureFlag=\"opaque&amp;value\" ",
    );
    doc.container_mut().replace_xml(xml).unwrap();
    apply(&mut doc, &[edit("port", "4840", "4841")]).unwrap();
    assert!(
        doc.container()
            .xml()
            .contains("FutureFlag=\"opaque&amp;value\"")
    );
    let xml = doc
        .container()
        .xml()
        .replace("ConfigName=\"LSPLC\"", "ConfigName=\"OtherPLC\"");
    doc.container_mut().replace_xml(xml).unwrap();
    let before = doc.clone();
    assert!(apply(&mut doc, &[edit("port", "4841", "4842")]).is_err());
    assert_eq!(doc, before);
}

#[test]
fn related_intervals_and_tag_address_alignment_are_validated_atomically() {
    let mut doc = project();
    let before = doc.clone();
    assert!(apply(&mut doc, &[edit("publishingInterval", "200", "100")]).is_err());
    assert_eq!(doc, before);
    apply(
        &mut doc,
        &[
            edit("publishingInterval", "200", "100"),
            edit("samplingInterval", "200", "100"),
        ],
    )
    .unwrap();
    let tags = doc.network().opcua_tags(0).unwrap();
    for (data_type, area, number) in [
        ("BYTE", "D", 10),
        ("BIT", "D", 10),
        ("WORD", "M", 17),
        ("DWORD", "D", 32767),
    ] {
        let before = doc.clone();
        assert!(
            doc.network_mut()
                .edit_opcua_tag(&xgwx::OpcUaTagPatch {
                    configuration_index: 0,
                    expected_tags: tags.clone(),
                    index: None,
                    tag: Some(xgwx::OpcUaTag {
                        name: "Invalid".into(),
                        data_type: data_type.into(),
                        address_area: area.into(),
                        address_number: number,
                        comment: "".into()
                    })
                })
                .is_err()
        );
        assert_eq!(doc, before);
    }
}

#[test]
fn generated_options_and_scalar_nodes_match_the_native_resave() {
    let native = XgwxDocument::parse(include_bytes!(
        "../fixtures/networks/eopct-generated-resaved.xgwx"
    ))
    .unwrap();
    let mut generated = project();
    apply(
        &mut generated,
        &[
            edit("port", "4840", "4841"),
            edit("publishingInterval", "200", "100"),
            edit("samplingInterval", "200", "100"),
            edit("nodeIdType", "0", "1"),
        ],
    )
    .unwrap();
    for tag in native.network().opcua_tags(0).unwrap() {
        let expected_tags = generated.network().opcua_tags(0).unwrap();
        generated
            .network_mut()
            .edit_opcua_tag(&xgwx::OpcUaTagPatch {
                configuration_index: 0,
                expected_tags,
                index: None,
                tag: Some(tag),
            })
            .unwrap();
    }
    let generated = XgwxDocument::parse(&generated.to_verified_bytes().unwrap()).unwrap();
    assert_eq!(
        generated.network().opcua_config_infos(),
        native.network().opcua_config_infos()
    );
    assert_eq!(
        generated.network().opcua_tags(0).unwrap(),
        native.network().opcua_tags(0).unwrap()
    );
}

#[test]
fn unknown_tag_children_are_rejected_without_loss() {
    let mut doc = project();
    let xml = doc.container().xml().replace(
        "<EIPTags Count=\"0\"></EIPTags>",
        "<EIPTags Count=\"0\"><FutureField Value=\"preserve\"/></EIPTags>",
    );
    doc.container_mut().replace_xml(xml).unwrap();
    let before = doc.clone();
    assert!(doc.network().opcua_tags(0).is_err());
    assert!(
        doc.network_mut()
            .edit_opcua_tag(&xgwx::OpcUaTagPatch {
                configuration_index: 0,
                expected_tags: vec![],
                index: None,
                tag: Some(xgwx::OpcUaTag {
                    name: "Value".into(),
                    data_type: "WORD".into(),
                    address_area: "D".into(),
                    address_number: 0,
                    comment: "".into()
                })
            })
            .is_err()
    );
    assert_eq!(doc, before);
}

#[test]
fn explicit_signed_and_unsigned_integer_types_survive_edits_and_reopen() {
    use xgwx::{OpcUaTag, OpcUaTagPatch};
    let mut doc = project();
    for (index, data_type) in [
        "SINT", "USINT", "INT", "UINT", "DINT", "UDINT", "LINT", "ULINT",
    ]
    .into_iter()
    .enumerate()
    {
        let byte = matches!(data_type, "SINT" | "USINT");
        let tag = OpcUaTag {
            name: format!("Integer{index}"),
            data_type: data_type.into(),
            address_area: if byte { "M" } else { "D" }.into(),
            address_number: if byte {
                16 + (index as u32) * 8
            } else {
                100 + (index as u32) * 4
            },
            comment: "Explicit integer type".into(),
        };
        let expected_tags = doc.network().opcua_tags(0).unwrap();
        doc.network_mut()
            .edit_opcua_tag(&OpcUaTagPatch {
                configuration_index: 0,
                expected_tags,
                index: None,
                tag: Some(tag),
            })
            .unwrap();
    }
    let mut doc = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
    let tags = doc.network().opcua_tags(0).unwrap();
    assert_eq!(
        tags.iter()
            .map(|t| t.data_type.as_str())
            .collect::<Vec<_>>(),
        [
            "SINT", "USINT", "INT", "UINT", "DINT", "UDINT", "LINT", "ULINT"
        ]
    );
    for (index, replacement) in [
        (0, "USINT"),
        (1, "SINT"),
        (2, "UINT"),
        (3, "INT"),
        (4, "UDINT"),
        (5, "DINT"),
        (6, "ULINT"),
        (7, "LINT"),
    ] {
        let expected_tags = doc.network().opcua_tags(0).unwrap();
        let mut tag = expected_tags[index].clone();
        tag.data_type = replacement.into();
        doc.network_mut()
            .edit_opcua_tag(&OpcUaTagPatch {
                configuration_index: 0,
                expected_tags,
                index: Some(index),
                tag: Some(tag),
            })
            .unwrap();
    }
    let doc = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
    assert_eq!(
        doc.network()
            .opcua_tags(0)
            .unwrap()
            .iter()
            .map(|t| t.data_type.as_str())
            .collect::<Vec<_>>(),
        [
            "USINT", "SINT", "UINT", "INT", "UDINT", "DINT", "ULINT", "LINT"
        ]
    );
}

#[test]
fn integer_aliases_keep_width_and_address_constraints() {
    for (data_type, area, number) in [
        ("SINT", "D", 0),
        ("USINT", "M", 1),
        ("INT", "M", 17),
        ("UINT", "M", 17),
        ("DINT", "D", 32767),
        ("UDINT", "D", 32767),
        ("LINT", "D", 32765),
        ("ULINT", "D", 32765),
    ] {
        let mut doc = project();
        let before = doc.clone();
        assert!(
            doc.network_mut()
                .edit_opcua_tag(&xgwx::OpcUaTagPatch {
                    configuration_index: 0,
                    expected_tags: vec![],
                    index: None,
                    tag: Some(xgwx::OpcUaTag {
                        name: "Invalid".into(),
                        data_type: data_type.into(),
                        address_area: area.into(),
                        address_number: number,
                        comment: "".into()
                    })
                })
                .is_err(),
            "{data_type}"
        );
        assert_eq!(doc, before);
    }
}

#[test]
fn native_integer_resave_retains_types_and_remains_editable() {
    let mut doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/networks/eopct-integers-resaved.xgwx"
    ))
    .unwrap();
    let tags = doc.network().opcua_tags(0).unwrap();
    assert_eq!(
        tags[5..]
            .iter()
            .map(|t| t.data_type.as_str())
            .collect::<Vec<_>>(),
        [
            "SINT", "USINT", "INT", "UINT", "DINT", "UDINT", "LINT", "ULINT"
        ]
    );
    for index in 5..tags.len() {
        let expected_tags = doc.network().opcua_tags(0).unwrap();
        let mut tag = expected_tags[index].clone();
        tag.comment.push_str(" edited");
        doc.network_mut()
            .edit_opcua_tag(&xgwx::OpcUaTagPatch {
                configuration_index: 0,
                expected_tags,
                index: Some(index),
                tag: Some(tag),
            })
            .unwrap();
    }
    let reopened = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
    let mut expected = tags;
    for tag in &mut expected[5..] {
        tag.comment.push_str(" edited");
    }
    assert_eq!(reopened.network().opcua_tags(0).unwrap(), expected);
}
