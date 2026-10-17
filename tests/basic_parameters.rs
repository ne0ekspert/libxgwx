#![cfg(feature = "write")]
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
fn fixture() -> XgwxDocument {
    document(
        r#"<Project><Parameter Type="OTHER"/><Configuration><Parameter Type="BASIC PARAMETER"><XGTBasicParam bp_ver="100" size="400" STATIC_PERIOD_TIME="10" SCAN_WD_TIME="500" STATIC_PERIOD_MODE="0" DEBUG_OUTPUT_PARAMETER="0" Unknown="keep &amp; preserve"/></Parameter></Configuration><Configuration><Parameter Type="BASIC PARAMETER"><XGTBasicParam bp_ver="100" size="400" STATIC_PERIOD_TIME="20" SCAN_WD_TIME="600"/></Parameter></Configuration></Project>"#,
    )
}
#[test]
fn timing_patch_preserves_other_plcs_unknown_fields_and_native_xml() {
    let mut doc = fixture();
    let before = doc.container().xml().to_owned();
    assert_eq!(
        doc.parameter().get(2).unwrap().configuration_index(),
        Some(1)
    );
    doc.parameter_mut()
        .get(2)
        .unwrap()
        .edit_basic_timing(&[
            ("STATIC_PERIOD_TIME", "20", 999),
            ("SCAN_WD_TIME", "600", 10),
        ])
        .unwrap();
    assert_eq!(
        doc.container().xml(),
        before
            .replace("STATIC_PERIOD_TIME=\"20\"", "STATIC_PERIOD_TIME=\"999\"")
            .replace("SCAN_WD_TIME=\"600\"", "SCAN_WD_TIME=\"10\"")
    );
}
#[test]
fn created_workspace_timings_survive_verified_container_roundtrip() {
    let mut doc = xgwx::create_project("XGK-CPUSN", "LD").unwrap();
    let parameter = doc
        .parameter()
        .list()
        .into_iter()
        .find(|p| !p.basic_timing_fields().is_empty())
        .unwrap();
    let index = parameter.index();
    let fields = parameter.basic_timing_fields();
    let expected = fields
        .iter()
        .find(|f| f.name == "SCAN_WD_TIME")
        .unwrap()
        .value
        .clone();
    doc.parameter_mut()
        .get(index)
        .unwrap()
        .edit_basic_timing(&[("SCAN_WD_TIME", &expected, 750)])
        .unwrap();
    let reopened = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
    assert_eq!(
        reopened.parameter().summaries(),
        doc.parameter().summaries()
    );
}
#[test]
fn invalid_stale_duplicate_or_unknown_fields_reject_the_entire_patch() {
    let source = fixture();
    for bad in [
        ("STATIC_PERIOD_TIME", "10", 0),
        ("STATIC_PERIOD_TIME", "10", 1000),
        ("STATIC_PERIOD_TIME", "stale", 5),
        ("SCAN_WD_TIME", "500", 9),
        ("SCAN_WD_TIME", "500", 1001),
        ("RESET_DISABLE", "0", 1),
        ("SCAN_WD_TIME", "500", 100),
    ] {
        let mut doc = source.clone();
        // The final case deliberately repeats the valid first field.
        assert!(
            doc.parameter_mut()
                .get(1)
                .unwrap()
                .edit_basic_timing(&[("SCAN_WD_TIME", "500", 1000), bad])
                .is_err()
        );
        assert_eq!(doc, source);
    }
    assert!(source.clone().parameter_mut().get(99).is_err());
}
#[test]
fn manual_boundaries_and_no_op_edits_are_supported() {
    let source = fixture();
    for (cycle, watchdog) in [(1, 10), (999, 1000)] {
        let mut doc = source.clone();
        doc.parameter_mut()
            .get(1)
            .unwrap()
            .edit_basic_timing(&[
                ("STATIC_PERIOD_TIME", "10", cycle),
                ("SCAN_WD_TIME", "500", watchdog),
            ])
            .unwrap();
    }
    let mut doc = source.clone();
    doc.parameter_mut()
        .get(1)
        .unwrap()
        .edit_basic_timing(&[("STATIC_PERIOD_TIME", "10", 10)])
        .unwrap();
    assert_eq!(doc, source);
    assert_eq!(doc.to_bytes().unwrap(), source.to_bytes().unwrap());
}
#[test]
fn unsupported_or_ambiguous_layouts_remain_read_only() {
    for section in [
        r#"<XGIBasicParam bp_ver="100" size="400" STATIC_PERIOD_TIME="10"/>"#,
        r#"<XGTBasicParam bp_ver="101" size="400" STATIC_PERIOD_TIME="10"/>"#,
        r#"<XGTBasicParam bp_ver="100" size="401" STATIC_PERIOD_TIME="10"/>"#,
        r#"<XGTBasicParam bp_ver="100" size="400" STATIC_PERIOD_TIME="10"/><Other/>"#,
    ] {
        let mut doc = document(&format!(
            r#"<Project><Parameter Type="BASIC PARAMETER">{section}</Parameter></Project>"#
        ));
        let before = doc.clone();
        assert!(
            doc.parameter()
                .get(0)
                .unwrap()
                .basic_timing_fields()
                .is_empty()
        );
        assert!(
            doc.parameter_mut()
                .get(0)
                .unwrap()
                .edit_basic_timing(&[("STATIC_PERIOD_TIME", "10", 5)])
                .is_err()
        );
        assert_eq!(doc, before);
    }
}

fn basic_index(doc: &XgwxDocument) -> usize {
    doc.parameter()
        .list()
        .into_iter()
        .find(|p| !p.basic_operation_fields().is_empty())
        .unwrap()
        .index()
}
#[test]
fn captured_switches_use_native_encodings_including_inverse_debug_output() {
    use xgwx::BasicParameterControl;
    for source in [
        include_bytes!("../fixtures/basic-parameters/xgk-native-controls.xgwx").as_slice(),
        include_bytes!("../fixtures/basic-parameters/xgi-native-overall.xgwx").as_slice(),
    ] {
        let mut doc = XgwxDocument::parse(source).unwrap();
        let index = basic_index(&doc);
        let fields = doc.parameter().get(index).unwrap().basic_operation_fields();
        let debug = fields
            .iter()
            .find(|f| f.name == "DEBUG_OUTPUT_PARAMETER")
            .unwrap();
        assert_eq!(
            debug.control,
            BasicParameterControl::Checkbox {
                checked: 0,
                unchecked: 43536
            }
        );
        let changes = fields
            .iter()
            .filter_map(|f| {
                if let BasicParameterControl::Checkbox { checked, unchecked } = f.control {
                    Some((
                        f.name.as_str(),
                        f.value.as_str(),
                        if f.name.starts_with("O_") {
                            checked
                        } else {
                            unchecked
                        },
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        doc.parameter_mut()
            .get(index)
            .unwrap()
            .edit_basic_operation(&changes)
            .unwrap();
        let fields = doc.parameter().get(index).unwrap().basic_operation_fields();
        assert_eq!(
            fields
                .iter()
                .find(|f| f.name == "DEBUG_OUTPUT_PARAMETER")
                .unwrap()
                .value,
            "43536"
        );
        assert_eq!(
            fields
                .iter()
                .find(|f| f.name == "O_RESET_DISABLE")
                .unwrap()
                .value,
            "1"
        );
        assert_eq!(
            fields
                .iter()
                .find(|f| f.name == "RESET_DISABLE")
                .unwrap()
                .value,
            "0"
        );
    }
}
#[test]
fn illegal_choices_and_inconsistent_switch_dependencies_reject_atomically() {
    let source = xgwx::create_project("XGK-CPUSN", "LD").unwrap();
    for (name, expected, value) in [
        ("STANDARD_INPUT_FILTER_TIME", "3", 0),
        ("STANDARD_INPUT_FILTER_TIME", "3", 2),
        ("DEBUG_OUTPUT_PARAMETER", "0", 1),
        ("SOE_RETAIN_HISTORY", "43544", 1),
        ("RESET_DISABLE", "0", 1),
    ] {
        let mut doc = source.clone();
        let index = basic_index(&doc);
        assert!(
            doc.parameter_mut()
                .get(index)
                .unwrap()
                .edit_basic_operation(&[("SCAN_WD_TIME", "500", 600), (name, expected, value)])
                .is_err()
        );
        assert_eq!(doc, source);
    }
    for value in [1, 3, 5, 10, 20, 70, 100] {
        let mut doc = source.clone();
        let index = basic_index(&doc);
        doc.parameter_mut()
            .get(index)
            .unwrap()
            .edit_basic_operation(&[("STANDARD_INPUT_FILTER_TIME", "3", value)])
            .unwrap();
    }
}
#[test]
fn iec_watchdog_edit_preserves_hidden_slots_and_restart_encoding() {
    let mut doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/basic-parameters/xgi-native-overall.xgwx"
    ))
    .unwrap();
    let index = basic_index(&doc);
    let before = doc.parameter().get(index).unwrap().summary();
    doc.parameter_mut()
        .get(index)
        .unwrap()
        .edit_basic_operation(&[
            ("SCAN_WD_TIME_0", "678", 10),
            ("RESTART_METHOD", "0", 43521),
        ])
        .unwrap();
    let after = doc.parameter().get(index).unwrap().summary();
    for slot in 1..4 {
        let key = format!("SCAN_WD_TIME_{slot}");
        assert_eq!(
            before.sections[0].attributes.iter().find(|a| a.name == key),
            after.sections[0].attributes.iter().find(|a| a.name == key)
        );
    }
    assert!(
        !doc.parameter()
            .get(index)
            .unwrap()
            .basic_operation_fields()
            .iter()
            .any(|f| [
                "IO_POINT_METHOD",
                "ERROR_MODE_LATCH_PARAMETER",
                "SCAN_WD_TIME_1"
            ]
            .contains(&f.name.as_str()))
    );
    let reopened = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
    assert_eq!(
        reopened.parameter().summaries(),
        doc.parameter().summaries()
    );
}
#[test]
fn unknown_switch_codes_remain_preserved_and_read_only() {
    let mut doc = document(
        r#"<Project><Parameter Type="BASIC PARAMETER"><XGTBasicParam bp_ver="100" size="400" DEBUG_OUTPUT_PARAMETER="1234" SCAN_WD_TIME="500"/></Parameter></Project>"#,
    );
    assert!(
        !doc.parameter()
            .get(0)
            .unwrap()
            .basic_operation_fields()
            .iter()
            .any(|f| f.name == "DEBUG_OUTPUT_PARAMETER")
    );
    let before = doc.clone();
    assert!(
        doc.parameter_mut()
            .get(0)
            .unwrap()
            .edit_basic_operation(&[("DEBUG_OUTPUT_PARAMETER", "1234", 0)])
            .is_err()
    );
    assert_eq!(doc, before);
}

#[test]
fn native_acceptance_pairs_retain_all_fields_and_semantic_control_states() {
    use xgwx::BasicParameterControl;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/basic-parameters");
    for family in ["xgk", "xgi"] {
        for state in ["on", "off", "overall"] {
            let prefix = format!("{family}-{state}");
            let source = XgwxDocument::parse(
                &std::fs::read(root.join(format!("{prefix}-generated.xgwx"))).unwrap(),
            )
            .unwrap();
            let native = XgwxDocument::parse(
                &std::fs::read(root.join(format!("{prefix}-resaved.xgwx"))).unwrap(),
            )
            .unwrap();
            let a = source.parameter().get(basic_index(&source)).unwrap();
            let b = native.parameter().get(basic_index(&native)).unwrap();
            assert_eq!(a.summary(), b.summary());
            let fields = b.basic_operation_fields();
            assert!(fields.len() >= 14);
            for field in fields {
                if let BasicParameterControl::Checkbox { checked, unchecked } = field.control {
                    let on = state == "on" || (state == "overall" && field.name.starts_with("O_"));
                    assert_eq!(
                        field.value.parse::<u32>().unwrap(),
                        if on { checked } else { unchecked },
                        "{prefix} {}",
                        field.name
                    );
                }
            }
        }
    }
}
