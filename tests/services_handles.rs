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
fn repeated_native_link_selectors_do_not_merge_plcs_or_blocks() {
    let doc = document(
        r#"<Project><Configuration>PLC<XGPD_HS_LINK Index="0"><XGPD_HS_LINK_BLK BlockIndex="0" TxAddress="A"/></XGPD_HS_LINK><Safety_Comm RcvWaitTime="11"/><TrendMonitoring Custom="A"/></Configuration><Configuration>PLC<XGPD_HS_LINK Index="0"><XGPD_HS_LINK_BLK BlockIndex="0" TxAddress="B"/></XGPD_HS_LINK><Safety_Comm RcvWaitTime="22"/><TrendMonitoring Custom="B"/></Configuration></Project>"#,
    );
    let a = doc.configuration().get(0).unwrap();
    let b = doc.configuration().get(1).unwrap();
    assert_eq!(
        a.high_speed_link().get(0).unwrap().blocks()[0]
            .summary()
            .tx_address
            .as_deref(),
        Some("A")
    );
    assert_eq!(
        b.high_speed_link().get(1).unwrap().blocks()[0]
            .summary()
            .tx_address
            .as_deref(),
        Some("B")
    );
    assert!(a.high_speed_link().get(1).is_err());
    assert!(a.high_speed_link().block(1).is_err());
    assert_eq!(
        b.high_speed_link()
            .block(1)
            .unwrap()
            .link()
            .unwrap()
            .index(),
        1
    );
    assert_eq!(
        b.high_speed_link()
            .block(1)
            .unwrap()
            .configuration()
            .unwrap()
            .index(),
        1
    );
    assert_eq!(
        a.safety().first().unwrap().summary().rcv_wait_time,
        Some(11)
    );
    assert_eq!(b.safety().get(1).unwrap().summary().rcv_wait_time, Some(22));
    assert!(a.safety().get(1).is_err());
    assert_eq!(
        b.trend().get(1).unwrap().configuration().unwrap().index(),
        1
    );
    assert!(a.trend().get(1).is_err());
    assert_eq!(doc.safety().list().len(), 2);
    assert_eq!(doc.trend().list().len(), 2);
}
#[test]
fn block_parentage_respects_nested_links_configuration_boundaries_and_orphans() {
    let doc = document(
        "<Project><XGPD_HS_LINK_BLK/><XGPD_HS_LINK><Wrapper><XGPD_HS_LINK_BLK/></Wrapper><XGPD_HS_LINK><XGPD_HS_LINK_BLK/></XGPD_HS_LINK><Configuration><XGPD_HS_LINK_BLK/></Configuration><XGPD_HS_LINK_BLK/></XGPD_HS_LINK></Project>",
    );
    let blocks = doc.high_speed_link().blocks();
    assert_eq!(
        blocks
            .iter()
            .map(|b| b.link().map(|link| link.index()))
            .collect::<Vec<_>>(),
        [None, Some(0), Some(1), None, Some(0)]
    );
    assert_eq!(
        doc.high_speed_link()
            .get(0)
            .unwrap()
            .blocks()
            .iter()
            .map(|b| b.index())
            .collect::<Vec<_>>(),
        [1, 4]
    );
    assert_eq!(doc.high_speed_link().get(1).unwrap().blocks()[0].index(), 2);
    assert_eq!(
        doc.configuration()
            .get(0)
            .unwrap()
            .high_speed_link()
            .blocks()[0]
            .index(),
        3
    );
    assert!(blocks[0].configuration_index().is_none());
    assert_eq!(blocks[3].configuration_index(), Some(0));
}
#[test]
fn safety_channels_and_unknown_trend_sections_are_preserved() {
    let doc = document(
        r#"<Project><Safety_Comm IPAddress="67305985" Extra="keep"><FutureChannel Device="77" Addr="42" Custom="opaque"/></Safety_Comm><TrendMonitoring Extra="keep"><TraceConfiguration/><TraceConfiguration Custom="second"/><GraphConfiguration/><WindowConfiguration/><FutureConfiguration Details="opaque"/></TrendMonitoring></Project>"#,
    );
    let before = doc.clone();
    let safety = doc.safety().get(0).unwrap();
    assert_eq!(safety.summary().ip_address.as_deref(), Some("1.2.3.4"));
    assert_eq!(safety.channels()[0].name, "FutureChannel");
    assert!(
        safety.channels()[0]
            .attributes
            .iter()
            .any(|a| a.name == "Custom" && a.value == "opaque")
    );
    let trend = doc.trend().get(0).unwrap();
    assert_eq!(trend.sections().len(), 5);
    assert!(trend.summary().trace_configuration.is_some());
    assert_eq!(trend.sections()[4].name, "FutureConfiguration");
    assert!(trend.configuration().is_none());
    assert!(safety.configuration().is_none());
    assert_eq!(doc, before);
}
#[test]
fn multiple_records_keep_first_semantics_and_empty_collections_are_explicit() {
    let doc = document(
        "<Project><Safety_Comm RcvWaitTime=\"1\"/><Safety_Comm RcvWaitTime=\"2\"/><TrendMonitoring/><TrendMonitoring/></Project>",
    );
    assert_eq!(doc.safety().summaries().len(), 2);
    assert_eq!(doc.trend().summaries().len(), 2);
    assert_eq!(
        doc.safety().first().unwrap().summary().rcv_wait_time,
        Some(1)
    );
    assert_eq!(
        doc.safety().get(1).unwrap().summary().rcv_wait_time,
        Some(2)
    );
    assert_eq!(doc.trend().first().unwrap().index(), 0);
    let empty = document("<Project/>");
    assert!(empty.high_speed_link().list().is_empty());
    assert!(empty.high_speed_link().blocks().is_empty());
    assert!(empty.safety().first().is_none());
    assert!(empty.trend().first().is_none());
    assert!(empty.high_speed_link().get(0).is_err());
    assert!(empty.safety().get(0).is_err());
    assert!(empty.trend().get(0).is_err());
}
