use flate2::{Compression, write::GzEncoder};
use std::io::Write;
use xgwx::{ParameterData, ParameterKind, XgwxDocument, XgwxError};
fn document(xml: &str) -> XgwxDocument {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    XgwxDocument::parse(&bytes).unwrap()
}
#[test]
fn repeated_plc_names_keep_parameter_ownership_and_document_indices() {
    let payload = "0".repeat(448);
    let doc = document(&format!(
        r#"<Project><Parameter Type="OTHER"/><Configuration>PLC<Parameter Type="HSC PARAMETER" PAYLOAD="{payload}"/><Parameter Type="PID CAL PARAMETER"><EmbedPidLoopData TargetValue="11"/></Parameter></Configuration><Configuration>PLC<Parameter Type="PID CAL PARAMETER"><EmbedPidLoopData TargetValue="22"/></Parameter><Parameter Type="POSITION PARAMETER"/></Configuration></Project>"#
    ));
    let a = doc.configuration().get(0).unwrap().parameters();
    let b = doc.configuration().get(1).unwrap().parameters();
    assert_eq!(
        a.list().iter().map(|p| p.index()).collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(
        b.list().iter().map(|p| p.index()).collect::<Vec<_>>(),
        [3, 4]
    );
    assert_eq!(a.pid().calculation()[0].loops[0].target_value, Some(11));
    assert_eq!(b.pid().calculation()[0].loops[0].target_value, Some(22));
    assert!(b.hsc().is_empty());
    assert_eq!(a.hsc()[0].as_ref().unwrap().channels.len(), 4);
    assert!(matches!(
        a.get(3),
        Err(XgwxError::ParameterIndexNotFound { index: 3 })
    ));
    assert_eq!(b.get(3).unwrap().configuration().unwrap().index(), 1);
    assert!(doc.parameter().get(0).unwrap().configuration().is_none());
    assert_eq!(
        doc.parameter()
            .of_kind(ParameterKind::PidCalculation)
            .iter()
            .map(|p| p.index())
            .collect::<Vec<_>>(),
        [2, 3]
    );
}
#[test]
fn malformed_hsc_does_not_hide_other_records_or_raw_values() {
    let doc = document(
        r#"<Project><Parameter Type="HSC PARAMETER" PAYLOAD="XX" Custom="keep"/><Parameter Type="HSC PARAMETER" PAYLOAD="0000"/><Parameter Type="PID TUNE PARAMETER"><PidTunLoopData TargetValue="7"/></Parameter></Project>"#,
    );
    let before = doc.clone();
    let results = doc.parameter().hsc();
    assert!(results[0].is_err());
    assert!(results[1].is_ok());
    assert!(doc.parameter().get(0).unwrap().decode().is_err());
    assert!(
        doc.parameter()
            .get(0)
            .unwrap()
            .summary()
            .attributes
            .iter()
            .any(|a| a.name == "Custom" && a.value == "keep")
    );
    assert_eq!(
        doc.parameter().pid().tuning()[0].loops[0].target_value,
        Some(7)
    );
    assert_eq!(doc, before);
}
#[test]
fn selected_pid_and_position_decoding_preserves_sparse_native_fields() {
    let doc = document(
        r#"<Project><Parameter Type="POSITION PARAMETER"><HSC_PRM_XGB_AXIS_GROUP NUM_AXIS_DATA="1"><HSC_PRM_XGB_AXIS_DATA NUM_STEP_DATA="1"><HSC_PRM_XGB_AXIS_STEP_DATA TargetPosition="42"/></HSC_PRM_XGB_AXIS_DATA><HSC_PRM_XGB_AXIS_DATA/><HSC_PRM_XGB_AXIS_PARAM/><HSC_PRM_XGB_AXIS_PARAM/><HSC_PRM_XGB_AXIS_PARAM/></HSC_PRM_XGB_AXIS_GROUP></Parameter><Parameter Type="PID CAL PARAMETER" Custom="opaque"><EmbedPidLoopData TargetValue="-1" ForwardPWM="31"/><EmbedPidLoopData/></Parameter><Parameter Type="PID TUNE PARAMETER"><PidTunLoopData SetPWMAtPoint="47"/></Parameter></Project>"#,
    );
    let ParameterData::Position(position) = doc.parameter().get(0).unwrap().decode().unwrap()
    else {
        panic!("position")
    };
    assert_eq!(position.axis_count, Some(1));
    assert_eq!(position.axes.len(), 3);
    assert_eq!(position.axes[0].steps.len(), 1);
    assert_eq!(position.axes[0].steps[0].target_position, Some(42));
    assert!(position.axes[2].parameter.is_some());
    let ParameterData::PidCalculation(calculation) =
        doc.parameter().get(1).unwrap().decode().unwrap()
    else {
        panic!("calculation")
    };
    assert_eq!(calculation.loops[0].target_value, Some(-1));
    assert_eq!(calculation.loops[0].forward_pwm, Some(31));
    assert_eq!(calculation.loops[1].loop_index, 1);
    assert!(calculation.loops[1].target_value.is_none());
    assert!(
        calculation
            .attributes
            .iter()
            .any(|a| a.name == "Custom" && a.value == "opaque")
    );
    let ParameterData::PidTuning(tuning) = doc.parameter().get(2).unwrap().decode().unwrap() else {
        panic!("tuning")
    };
    assert_eq!(tuning.loops[0].set_pwm_at_point, Some(47));
}
#[test]
fn general_unknown_and_missing_parameter_types_remain_visible() {
    let doc = document(
        r#"<Project><Parameter Type="FUTURE PARAMETER" Raw="123"><VendorPayload Details="opaque"/></Parameter><Parameter/></Project>"#,
    );
    let selected = doc.parameter().get(0).unwrap();
    assert_eq!(selected.kind(), ParameterKind::Other);
    assert_eq!(selected.type_name(), Some("FUTURE PARAMETER"));
    let ParameterData::Other(summary) = selected.decode().unwrap() else {
        panic!("unknown preserved")
    };
    assert_eq!(summary.sections[0].name, "VendorPayload");
    assert!(
        summary
            .attributes
            .iter()
            .any(|a| a.name == "Raw" && a.value == "123")
    );
    assert_eq!(doc.parameter().get(1).unwrap().type_name(), None);
    assert_eq!(doc.parameter().summaries().len(), 2);
    assert!(matches!(
        doc.parameter().get(2),
        Err(XgwxError::ParameterIndexNotFound { index: 2 })
    ));
}
#[test]
fn empty_collections_and_nested_configuration_boundaries_are_explicit() {
    let empty = document("<Project/>");
    assert!(empty.parameter().list().is_empty());
    assert!(empty.parameter().hsc().is_empty());
    assert!(empty.parameter().position().is_empty());
    assert!(empty.parameter().pid().calculation().is_empty());
    assert!(empty.parameter().pid().tuning().is_empty());
    let doc = document(
        "<Project><Configuration><Parameter/><Configuration><Parameter/></Configuration><Parameter/></Configuration><Parameter/></Project>",
    );
    assert_eq!(
        doc.parameter()
            .list()
            .iter()
            .map(|p| p.configuration_index())
            .collect::<Vec<_>>(),
        [Some(0), Some(1), Some(0), None]
    );
    assert_eq!(
        doc.configuration()
            .get(0)
            .unwrap()
            .parameters()
            .list()
            .len(),
        2
    );
}
