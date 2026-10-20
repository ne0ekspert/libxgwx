#![cfg(feature = "write")]
use flate2::{Compression, write::GzEncoder};
use std::io::Write;
use xgwx::{
    NativeParameterChange as Change, NativeParameterTarget as Target, ParameterKind, XgwxDocument,
};

fn document(xml: &str) -> XgwxDocument {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    XgwxDocument::parse(&bytes).unwrap()
}
fn native(name: &str) -> XgwxDocument {
    let bytes = std::fs::read(format!(
        "{}/fixtures/{name}.xgwx",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    XgwxDocument::parse(&bytes).unwrap()
}
fn change(
    doc: &XgwxDocument,
    index: usize,
    target: Target,
    name: &str,
    replacement: i64,
) -> Change {
    let field = doc
        .parameter()
        .get(index)
        .unwrap()
        .native_fields()
        .into_iter()
        .find(|f| f.target == target && f.name == name)
        .unwrap();
    Change {
        target,
        name: name.into(),
        expected: field.value,
        replacement,
    }
}
fn index(doc: &XgwxDocument, kind: ParameterKind) -> usize {
    doc.parameter().of_kind(kind)[0].index()
}

#[test]
fn all_native_fields_edit_and_serialize_on_both_xgb_fixtures() {
    for file in ["XGB_Enet01", "XGB_Enet02"] {
        for kind in [
            ParameterKind::Hsc,
            ParameterKind::PidCalculation,
            ParameterKind::PidTuning,
            ParameterKind::Position,
        ] {
            let mut doc = native(file);
            let i = index(&doc, kind);
            let before = doc.clone();
            let fields = doc.parameter().get(i).unwrap().native_fields();
            assert!(!fields.is_empty(), "{file} {kind:?}");
            let changes: Vec<_> = fields
                .iter()
                .map(|f| {
                    let current = f.value.parse::<i64>().unwrap();
                    Change {
                        target: f.target,
                        name: f.name.clone(),
                        expected: f.value.clone(),
                        replacement: if current == f.maximum {
                            f.minimum
                        } else {
                            current + 1
                        },
                    }
                })
                .collect();
            doc.parameter_mut()
                .get(i)
                .unwrap()
                .edit_native(&changes)
                .unwrap();
            let bytes = doc.to_verified_bytes().unwrap();
            let reparsed = XgwxDocument::parse(&bytes).unwrap();
            let actual = reparsed.parameter().get(i).unwrap().native_fields();
            assert_eq!(actual.len(), changes.len());
            for (actual, change) in actual.iter().zip(&changes) {
                assert_eq!(
                    (&actual.target, &actual.name, &actual.value),
                    (
                        &change.target,
                        &change.name,
                        &change.replacement.to_string()
                    )
                );
            }
            // Every other parameter subtree stays exactly equal, including unknown data.
            for other in before
                .parameter()
                .list()
                .into_iter()
                .filter(|p| p.index() != i)
            {
                assert_eq!(
                    other.decode().unwrap(),
                    reparsed
                        .parameter()
                        .get(other.index())
                        .unwrap()
                        .decode()
                        .unwrap()
                );
            }
            // An empty or numerically identical edit must retain the original container.
            let noop: Vec<_> = actual
                .iter()
                .map(|f| Change {
                    target: f.target,
                    name: f.name.clone(),
                    expected: f.value.clone(),
                    replacement: f.value.parse().unwrap(),
                })
                .collect();
            let mut noop_doc = reparsed.clone();
            noop_doc
                .parameter_mut()
                .get(i)
                .unwrap()
                .edit_native(&noop)
                .unwrap();
            assert_eq!(noop_doc.to_verified_bytes().unwrap(), bytes);
            noop_doc
                .parameter_mut()
                .get(i)
                .unwrap()
                .edit_native(&[])
                .unwrap();
            assert_eq!(noop_doc, reparsed);
        }
    }
}

#[test]
fn edits_address_exact_plc_loop_axis_and_step_without_rewriting_other_xml() {
    let native = native("XGB_Enet01");
    let xml = roxmltree::Document::parse(native.container().xml()).unwrap();
    let parameter = xml
        .descendants()
        .find(|n| n.has_tag_name("Parameter") && n.attribute("Type") == Some("POSITION PARAMETER"))
        .unwrap();
    let fragment = &native.container().xml()[parameter.range()];
    let mut doc = document(&format!(
        "<Project><Parameter Type='OTHER'/><Configuration Type='2'>{fragment}</Configuration><Configuration Type='2'>{fragment}</Configuration></Project>"
    ));
    let before = doc.container().xml().to_owned();
    let i = 2;
    let change = change(
        &doc,
        i,
        Target::PositionStep {
            axis_index: 1,
            step_index: 29,
        },
        "TargetPosition",
        -123456,
    );
    let xml = roxmltree::Document::parse(&before).unwrap();
    let param = xml
        .descendants()
        .filter(|n| n.has_tag_name("Parameter"))
        .nth(i)
        .unwrap();
    let step = param
        .descendants()
        .filter(|n| n.has_tag_name("HSC_PRM_XGB_AXIS_DATA"))
        .nth(1)
        .unwrap()
        .children()
        .filter(|n| n.has_tag_name("HSC_PRM_XGB_AXIS_STEP_DATA"))
        .nth(29)
        .unwrap();
    let range = step
        .attributes()
        .find(|a| a.name() == "TargetPosition")
        .unwrap()
        .range_value();
    let mut expected = before.clone();
    expected.replace_range(range, "-123456");
    doc.parameter_mut()
        .get(i)
        .unwrap()
        .edit_native(&[change])
        .unwrap();
    assert_eq!(doc.container().xml(), expected);
    assert_eq!(
        doc.parameter().get(i).unwrap().configuration_index(),
        Some(1)
    );
}

#[test]
fn hsc_updates_only_selected_nibble_or_little_endian_word() {
    let mut payload = vec![0xABu8; 448];
    payload.fill(b'0');
    // Valid time/revolution words, all channels. Keep nonzero unknown bytes.
    for channel in 0..4 {
        payload[(channel * 56 + 44) * 2..(channel * 56 + 48) * 2].copy_from_slice(b"01000100");
    }
    payload[0] = b'a';
    let raw = String::from_utf8(payload).unwrap();
    let raw = raw
        .as_bytes()
        .chunks(16)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect::<Vec<_>>()
        .join(" \t");
    let mut doc = document(&format!(
        "<Project><Parameter Type='HSC PARAMETER' PAYLOAD_ASC_LENGTH='448' PAYLOAD='{raw}' Custom='opaque'/></Project>"
    ));
    let before = doc.clone();
    let changes = [
        change(
            &doc,
            0,
            Target::HscChannel { channel: 3 },
            "counter_mode",
            1,
        ),
        change(
            &doc,
            0,
            Target::HscChannel { channel: 2 },
            "unit_time_ms",
            0x1234,
        ),
        change(
            &doc,
            0,
            Target::HscChannel { channel: 1 },
            "pulse_input_mode",
            3,
        ),
        change(
            &doc,
            0,
            Target::HscChannel { channel: 3 },
            "pulses_per_revolution",
            60000,
        ),
    ];
    doc.parameter_mut()
        .get(0)
        .unwrap()
        .edit_native(&changes)
        .unwrap();
    let old = before.parameter().hsc().remove(0).unwrap();
    let new = doc.parameter().hsc().remove(0).unwrap();
    let mut expected = old.payload_bytes;
    expected[2] = 0x10; // Channel 3 mode is high nibble of byte 2.
    expected[3] = 0x30; // Channel 1 pulse mode is high nibble of byte 3.
    expected[214..216].copy_from_slice(&[0x60, 0xEA]); // Channel 3, 60000 LE.
    expected[156..158].copy_from_slice(&[0x34, 0x12]); // 2*56 + 44.
    assert_eq!(new.payload_bytes, expected);
    assert_eq!(new.initial_unknown_nibble, Some(10));
    let old = old
        .payload
        .bytes()
        .filter(|b| b.is_ascii_whitespace())
        .collect::<Vec<_>>();
    let new = new
        .payload
        .bytes()
        .filter(|b| b.is_ascii_whitespace())
        .collect::<Vec<_>>();
    assert_eq!(old, new);
}

#[test]
fn invalid_batches_rollback_for_every_family() {
    for kind in [
        ParameterKind::Hsc,
        ParameterKind::PidCalculation,
        ParameterKind::PidTuning,
        ParameterKind::Position,
    ] {
        let baseline = native("XGB_Enet01");
        let i = index(&baseline, kind);
        let fields = baseline.parameter().get(i).unwrap().native_fields();
        let f = &fields[0];
        let valid = change(&baseline, i, f.target, &f.name, 1);
        let mut stale = valid.clone();
        stale.expected = "stale".into();
        let mut unknown = valid.clone();
        unknown.name = "Reserved_0".into();
        let mut low = valid.clone();
        low.replacement = f.minimum - 1;
        let mut high = valid.clone();
        high.replacement = f.maximum + 1;
        let mut wrong_target = valid.clone();
        wrong_target.target = Target::HscChannel { channel: 999 };
        for invalid in [stale, unknown, low, high, wrong_target, valid.clone()] {
            let mut doc = baseline.clone();
            assert!(
                doc.parameter_mut()
                    .get(i)
                    .unwrap()
                    .edit_native(&[valid.clone(), invalid])
                    .is_err()
            );
            assert_eq!(doc, baseline);
        }
    }
}

#[test]
fn unknown_layouts_and_tentative_hsc_fields_remain_read_only() {
    let baseline = native("XGB_Enet01");
    for (from, to, kind) in [
        (
            "Header_0=\"17736\"",
            "Header_0=\"17737\"",
            ParameterKind::PidCalculation,
        ),
        ("CheckSum=\"0\"", "CheckSum=\"1\"", ParameterKind::PidTuning),
        (
            "NUM_AXIS_DATA=\"2\"",
            "NUM_AXIS_DATA=\"3\"",
            ParameterKind::Position,
        ),
        (
            "NUM_STEP_DATA=\"30\"",
            "NUM_STEP_DATA=\"31\"",
            ParameterKind::Position,
        ),
        (
            "PAYLOAD_ASC_LENGTH=\"448\"",
            "PAYLOAD_ASC_LENGTH=\"446\"",
            ParameterKind::Hsc,
        ),
    ] {
        let xml = baseline.container().xml().replace(from, to);
        assert_ne!(xml, baseline.container().xml());
        let mut doc = document(&xml);
        let i = index(&doc, kind);
        assert!(doc.parameter().get(i).unwrap().native_fields().is_empty());
        let before = doc.clone();
        assert!(
            doc.parameter_mut()
                .get(i)
                .unwrap()
                .edit_native(&[])
                .is_err()
        );
        assert_eq!(doc, before);
    }
    let fields = baseline
        .parameter()
        .get(index(&baseline, ParameterKind::Hsc))
        .unwrap()
        .native_fields();
    assert!(fields.iter().all(|f| {
        [
            "counter_mode",
            "pulse_input_mode",
            "unit_time_ms",
            "pulses_per_revolution",
        ]
        .contains(&f.name.as_str())
    }));
    for payload in [
        "XX".into(),
        "00".into(),
        "\u{2003}".to_string() + &"0".repeat(448),
    ] {
        let doc = document(&format!(
            "<Project><Parameter Type='HSC PARAMETER' PAYLOAD_ASC_LENGTH='448' PAYLOAD='{payload}'/></Project>"
        ));
        assert!(doc.parameter().get(0).unwrap().native_fields().is_empty());
    }
}

#[cfg(feature = "ffi")]
#[test]
fn c_api_native_inspection_edit_and_transaction_rollback_match_rust() {
    use serde_json::json;
    use std::{ffi::CStr, ptr};
    use xgwx::ffi::*;
    fn ok(status: i32) {
        assert_eq!(
            status,
            XGWX_OK,
            "{}",
            unsafe { CStr::from_ptr(xgwx_last_error()) }.to_string_lossy()
        );
    }
    fn take(mut out: XgwxBuffer) -> Vec<u8> {
        let bytes = unsafe { std::slice::from_raw_parts(out.data, out.len) }.to_vec();
        unsafe { xgwx_buffer_free(&mut out) };
        bytes
    }
    fn serialize(handle: *mut XgwxHandle) -> Vec<u8> {
        let mut out = XgwxBuffer::default();
        ok(unsafe { xgwx_document_serialize(handle, &mut out) });
        take(out)
    }
    let original = native("XGB_Enet01");
    let bytes = original.to_verified_bytes().unwrap();
    let mut handle = ptr::null_mut();
    ok(unsafe { xgwx_document_parse(bytes.as_ptr(), bytes.len(), &mut handle) });
    let request =
        serde_json::to_vec(&json!({"section":"parameters", "offset":0, "limit":100})).unwrap();
    let mut out = XgwxBuffer::default();
    ok(unsafe {
        xgwx_document_inspect_section_json(handle, request.as_ptr(), request.len(), &mut out)
    });
    let inspection: serde_json::Value = serde_json::from_slice(&take(out)).unwrap();
    let items = inspection["data"]["parameters"]["items"]
        .as_array()
        .unwrap();
    let mut expected = original.clone();
    let mut edits = Vec::new();
    for kind in [
        ParameterKind::Hsc,
        ParameterKind::PidCalculation,
        ParameterKind::PidTuning,
        ParameterKind::Position,
    ] {
        let i = index(&original, kind);
        let fields = original.parameter().get(i).unwrap().native_fields();
        assert_eq!(
            items.iter().find(|p| p["parameter_index"] == i).unwrap()["native_fields"],
            serde_json::to_value(&fields).unwrap()
        );
        let f = &fields[0];
        let c = change(
            &original,
            i,
            f.target,
            &f.name,
            if f.value == "0" { 1 } else { 0 },
        );
        expected
            .parameter_mut()
            .get(i)
            .unwrap()
            .edit_native(std::slice::from_ref(&c))
            .unwrap();
        edits.push(
            json!({"operation":"edit_native_parameters", "parameter_index":i, "changes":[c]}),
        );
    }
    let request = serde_json::to_vec(&json!({"schema_version":1,"edits":edits})).unwrap();
    ok(unsafe { xgwx_document_apply_edits_json(handle, request.as_ptr(), request.len()) });
    assert_eq!(serialize(handle), expected.to_verified_bytes().unwrap());
    let before = serialize(handle);
    // A valid first operation must roll back when a later stale operation fails.
    let i = index(&expected, ParameterKind::PidCalculation);
    let c = change(
        &expected,
        i,
        Target::PidCalculationLoop { loop_index: 15 },
        "TargetValue",
        123,
    );
    let edit = json!({"operation":"edit_native_parameters", "parameter_index":i, "changes":[c]});
    let request =
        serde_json::to_vec(&json!({"schema_version":1,"edits":[edit.clone(), edit.clone()]}))
            .unwrap();
    assert_ne!(
        unsafe { xgwx_document_apply_edits_json(handle, request.as_ptr(), request.len()) },
        XGWX_OK
    );
    assert_eq!(serialize(handle), before);
    for target in [true, false] {
        let mut invalid = edit.clone();
        if target {
            invalid["changes"][0]["target"]["unexpected"] = json!(true);
        } else {
            invalid["changes"][0]["unexpected"] = json!(true);
        }
        let request = serde_json::to_vec(&json!({"schema_version":1,"edits":[invalid]})).unwrap();
        assert_ne!(
            unsafe { xgwx_document_apply_edits_json(handle, request.as_ptr(), request.len()) },
            XGWX_OK
        );
        assert_eq!(serialize(handle), before);
    }
    unsafe { xgwx_document_free(handle) };
}

#[test]
fn malformed_ambiguous_and_wrong_cpu_layouts_are_not_advertised() {
    let baseline = native("XGB_Enet01");
    let xml = baseline.container().xml();
    let parsed = roxmltree::Document::parse(xml).unwrap();
    let parameter = parsed
        .descendants()
        .find(|n| n.has_tag_name("Parameter") && n.attribute("Type") == Some("PID CAL PARAMETER"))
        .unwrap();
    let fragment = &xml[parameter.range()];
    for body in [
        fragment.replace(
            "</Parameter>",
            "<EmbedPidLoopData TargetValue='123'/></Parameter>",
        ),
        fragment.replace(
            "</Parameter>",
            "<Vendor><EmbedPidLoopData TargetValue='123'/></Vendor></Parameter>",
        ),
        fragment.replace("</Parameter>", "<EmbedPidLoopDataList/></Parameter>"),
        fragment
            .replace("<EmbedPidLoopData ", "<VendorData ")
            .replace("</EmbedPidLoopData>", "</VendorData>"),
    ] {
        let doc = document(&format!("<Project>{body}</Project>"));
        assert!(doc.parameter().get(0).unwrap().native_fields().is_empty());
    }
    for cpu in ["100", "999999"] {
        let doc = document(&format!(
            "<Project><Configuration Type='{cpu}'>{fragment}</Configuration></Project>"
        ));
        assert!(doc.parameter().get(0).unwrap().native_fields().is_empty());
    }
    let mut doc = document("<Project><Parameter Type='OTHER'/></Project>");
    let before = doc.clone();
    assert!(
        doc.parameter_mut()
            .get(0)
            .unwrap()
            .edit_native(&[])
            .is_err()
    );
    assert_eq!(doc, before);
    assert!(doc.parameter_mut().get(1).is_err());
}

#[cfg(any(feature = "ffi", feature = "wasm"))]
#[test]
fn native_change_json_rejects_unknown_keys_for_every_target_kind() {
    use serde_json::json;
    for target in [
        json!({"kind":"safety_communication"}),
        json!({"kind":"hsc_channel","channel":0}),
        json!({"kind":"pid_calculation"}),
        json!({"kind":"pid_calculation_loop","loop_index":0}),
        json!({"kind":"pid_tuning"}),
        json!({"kind":"pid_tuning_loop","loop_index":0}),
        json!({"kind":"position_axis","axis_index":0}),
        json!({"kind":"position_step","axis_index":0,"step_index":0}),
    ] {
        let valid = json!({"target":target,"name":"unused","expected":"0","replacement":1});
        assert!(serde_json::from_value::<Change>(valid.clone()).is_ok());
        let mut invalid = valid.clone();
        invalid["target"]["extra"] = json!(true);
        assert!(
            serde_json::from_value::<Change>(invalid).is_err(),
            "{target}"
        );
        let mut invalid = valid;
        invalid["extra"] = json!(true);
        assert!(serde_json::from_value::<Change>(invalid).is_err());
    }
}

#[test]
fn xg5000_resave_retains_all_supported_native_fields() {
    let original = native("native-parameters/generated");
    let resaved = native("native-parameters/resaved");
    for (kind, expected_count) in [
        (ParameterKind::Hsc, 16),
        (ParameterKind::PidCalculation, 342),
        (ParameterKind::PidTuning, 114),
        (ParameterKind::Position, 792),
    ] {
        let expected = original.parameter().of_kind(kind)[0].native_fields();
        let actual = resaved.parameter().of_kind(kind)[0].native_fields();
        assert_eq!(expected.len(), expected_count);
        assert_eq!(expected, actual, "native Save As changed {kind:?}");
    }
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/native-parameters/resaved.xgwx"
    ))
    .unwrap();
    assert_eq!(resaved.to_verified_bytes().unwrap(), bytes);
}

#[test]
fn legacy_fenet_addresses_use_little_endian_and_reject_invalid_masks_atomically() {
    let mut doc = native("empty-projects/new-xgk");
    let record = doc
        .parameter()
        .list()
        .into_iter()
        .find(|p| p.type_name() == Some("FENET PARAMETER"))
        .unwrap();
    let index = record.index();
    assert_eq!(record.native_fields().len(), 8);
    let original = doc.container().xml().to_owned();
    let target = Target::SafetyCommunication {};
    let ip = change(
        &doc,
        index,
        target,
        "IPAddress",
        u32::from_le_bytes([10, 20, 30, 40]) as i64,
    );
    let invalid = change(
        &doc,
        index,
        target,
        "Subnet",
        u32::from_le_bytes([255, 0, 255, 0]) as i64,
    );
    assert!(
        doc.parameter_mut()
            .get(index)
            .unwrap()
            .edit_native(&[ip.clone(), invalid])
            .is_err()
    );
    assert_eq!(doc.container().xml(), original);
    doc.parameter_mut()
        .get(index)
        .unwrap()
        .edit_native(&[ip.clone()])
        .unwrap();
    assert!(
        doc.parameter_mut()
            .get(index)
            .unwrap()
            .edit_native(&[ip])
            .is_err()
    );
    let xml = doc.container().xml().to_owned();
    assert_eq!(
        xml,
        original.replacen(
            "IPAddress=\"1861920960\"",
            &format!("IPAddress=\"{}\"", u32::from_le_bytes([10, 20, 30, 40])),
            1
        )
    );
    XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
}

#[test]
fn io_parameter_membership_uses_exact_records_across_repeated_plc_coordinates() {
    let doc = document(
        r#"<Project><Configuration Type="17"><Parameter Type="IO PARAMETER"><Module Base="0" Slot="0" Id="1"/><BaseInfo><Base Base="0" SlotCount="4"/></BaseInfo></Parameter><Parameter Type="IO PARAMETER"><Module Base="0" Slot="0" Id="2"/></Parameter></Configuration><Configuration Type="17"><Parameter Type="IO PARAMETER"><Module Base="0" Slot="0" Id="3"/><BaseInfo><Base Base="0" SlotCount="6"/></BaseInfo></Parameter><Parameter Type="FENET PARAMETER"><Safety_Comm IPAddress="1" Subnet="0" Gateway="0"/></Parameter></Configuration></Project>"#,
    );
    let parameters = doc.parameter().list();
    assert_eq!(parameters[0].io_module_indices(), vec![0]);
    assert_eq!(parameters[1].io_module_indices(), vec![1]);
    assert_eq!(parameters[2].io_module_indices(), vec![2]);
    assert_eq!(parameters[0].io_base_indices(), vec![0]);
    assert_eq!(parameters[2].io_base_indices(), vec![1]);
    assert!(parameters[3].io_module_indices().is_empty());
}

#[test]
fn ambiguous_legacy_fenet_records_are_readonly() {
    for body in [
        "<Safety_Comm IPAddress='1'/><Safety_Comm IPAddress='2'/>",
        "<Unknown><Safety_Comm IPAddress='1'/></Unknown>",
    ] {
        let doc = document(&format!(
            "<Project><Parameter Type='FENET PARAMETER'>{body}</Parameter></Project>"
        ));
        assert!(doc.parameter().get(0).unwrap().native_fields().is_empty());
    }
}

#[test]
fn legacy_fenet_native_dialog_bounds_and_driver_codes_are_guarded() {
    let original = native("empty-projects/new-xgk");
    let index = original
        .parameter()
        .list()
        .into_iter()
        .find(|p| p.type_name() == Some("FENET PARAMETER"))
        .unwrap()
        .index();
    for (name, value) in [
        ("RcvWaitTime", 1),
        ("RcvWaitTime", 256),
        ("RetransTime", 0),
        ("RetransTime", 601),
        ("GlofaSocketCnt", 0),
        ("GlofaSocketCnt", 5),
        ("DriverType", 3),
        ("EnableHostTable", 2),
    ] {
        let mut doc = original.clone();
        let edit = change(&doc, index, Target::SafetyCommunication {}, name, value);
        assert!(
            doc.parameter_mut()
                .get(index)
                .unwrap()
                .edit_native(&[edit])
                .is_err(),
            "{name} {value}"
        );
        assert_eq!(doc, original);
    }
    let doc = document(
        "<Project><Configuration Type='100'><Parameter Type='FENET PARAMETER'><Safety_Comm IPAddress='1'/></Parameter></Configuration></Project>",
    );
    assert!(doc.parameter().get(0).unwrap().native_fields().is_empty());
}

#[test]
fn native_fenet_io_resave_retains_settings_channels_and_program_payload() {
    let generated = native("fenet-io-parameters/generated");
    let resaved = native("fenet-io-parameters/resaved");
    let fields = |doc: &XgwxDocument| {
        doc.parameter()
            .list()
            .into_iter()
            .find(|p| p.type_name() == Some("FENET PARAMETER"))
            .unwrap()
            .native_fields()
    };
    assert_eq!(fields(&generated), fields(&resaved));
    assert_eq!(generated.module().summaries(), resaved.module().summaries());
    assert_eq!(
        generated
            .program()
            .ladder_programs()
            .remove(0)
            .unwrap()
            .data,
        resaved.program().ladder_programs().remove(0).unwrap().data
    );
    let xml_a = roxmltree::Document::parse(generated.container().xml()).unwrap();
    let xml_b = roxmltree::Document::parse(resaved.container().xml()).unwrap();
    for name in [
        "Base",
        "Safety_Comm_DI",
        "Safety_Comm_DO",
        "Safety_Comm_AI",
        "Safety_Comm_AO",
        "Safety_Comm_HostIp",
    ] {
        let attrs = |xml: &roxmltree::Document<'_>| {
            xml.descendants()
                .filter(|n| n.has_tag_name(name))
                .map(|n| {
                    n.attributes()
                        .map(|a| (a.name().to_owned(), a.value().to_owned()))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(attrs(&xml_a), attrs(&xml_b), "{name}");
    }
    let modbus = native("fenet-io-parameters/modbus-native");
    assert_eq!(
        fields(&modbus)
            .iter()
            .find(|f| f.name == "DriverType")
            .unwrap()
            .value,
        "5"
    );
    assert_eq!(generated.module().summaries(), modbus.module().summaries());
    let mut changed = generated.clone();
    let index = changed
        .parameter()
        .list()
        .into_iter()
        .find(|p| p.type_name() == Some("FENET PARAMETER"))
        .unwrap()
        .index();
    let driver = change(
        &changed,
        index,
        Target::SafetyCommunication {},
        "DriverType",
        5,
    );
    changed
        .parameter_mut()
        .get(index)
        .unwrap()
        .edit_native(&[driver])
        .unwrap();
    assert_eq!(fields(&changed), fields(&modbus));
}
