use flate2::{Compression, write::GzEncoder};
use std::{
    io::Write,
    time::{Duration, Instant},
};
use xgwx::XgwxDocument;

fn document(xml: &str) -> XgwxDocument {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    XgwxDocument::parse(&bytes).unwrap()
}

#[test]
fn parameter_inspection_does_not_rescan_configuration_for_each_record() {
    let entry = "<Parameter Type='BASIC PARAMETER'><XGIBasicParam/></Parameter>";
    for count in [2_500, 5_000] {
        let doc = document(&format!(
            "<Project><Configuration Type='100'>{}</Configuration></Project>",
            entry.repeat(count)
        ));
        let start = Instant::now();
        let records = doc.parameter().list();
        assert_eq!(records.len(), count);
        assert!(
            records
                .into_iter()
                .all(|p| p.basic_operation_fields().is_empty())
        );
        let elapsed = start.elapsed();
        eprintln!("{count} parameter controls: {elapsed:?}");
        assert!(
            elapsed < Duration::from_secs(2),
            "inspection took {elapsed:?}"
        );
    }
}

#[test]
fn large_supported_and_unsupported_parameter_batches_remain_complete() {
    for section in [
        "<XGIBasicParam/>",
        "<XGIBasicParam bp_ver='100' size='400' SCAN_WD_TIME_0='500'/>",
    ] {
        let count = 50_000;
        let doc = document(&format!(
            "<Project><Configuration Type='100'>{}</Configuration></Project>",
            format!("<Parameter Type='BASIC PARAMETER'>{section}</Parameter>").repeat(count)
        ));
        let start = Instant::now();
        let records = doc.parameter().list();
        assert_eq!(records.len(), count);
        for (index, record) in records.into_iter().enumerate() {
            assert_eq!(record.index(), index);
            assert_eq!(record.configuration_index(), Some(0));
            let fields = record.basic_operation_fields();
            assert_eq!(fields.len(), usize::from(section.contains("bp_ver")));
            if let Some(field) = fields.first() {
                assert_eq!((&*field.name, &*field.value), ("SCAN_WD_TIME_0", "500"));
            }
        }
        let elapsed = start.elapsed();
        eprintln!("50000 {section} parameter controls: {elapsed:?}");
        assert!(
            elapsed < Duration::from_secs(10),
            "inspection took {elapsed:?}"
        );
    }
}

#[test]
fn basic_controls_follow_exact_nearest_cpu_and_configuration_scope() {
    let parameter = "<Parameter Type='BASIC PARAMETER'><XGIBasicParam bp_ver='100' size='400' SCAN_WD_TIME_0='500'/></Parameter>";
    let doc = document(&format!(
        "<Project>{parameter}<Configuration Type=' 100 '>{parameter}<Configuration Type='1'>{parameter}</Configuration>{parameter}<Configuration>{parameter}</Configuration>{parameter}</Configuration><Configuration Type='101'>{parameter}</Configuration><Configuration Type='999999'>{parameter}</Configuration>{parameter}</Project>"
    ));
    let owners = [
        None,
        Some(0),
        Some(1),
        Some(0),
        Some(2),
        Some(0),
        Some(3),
        Some(4),
        None,
    ];
    let records = doc.parameter().list();
    assert_eq!(records.len(), owners.len());
    for (index, record) in records.into_iter().enumerate() {
        assert_eq!(record.index(), index);
        assert_eq!(record.configuration_index(), owners[index]);
        let expected = record
            .configuration()
            .and_then(|c| c.cpu())
            .is_some_and(|cpu| cpu.family == "XGI");
        assert_eq!(record.basic_operation_fields().len(), usize::from(expected));
    }
    let scoped = doc.configuration().get(0).unwrap().parameters().list();
    assert_eq!(
        scoped.iter().map(|p| p.index()).collect::<Vec<_>>(),
        [1, 3, 5]
    );
    assert!(
        scoped
            .into_iter()
            .all(|p| p.basic_operation_fields().len() == 1)
    );
}

#[test]
fn cpu_catalog_and_type_parsing_keep_operation_controls_compatible() {
    for cpu_type in [
        "100",
        " 102 ",
        "104",
        "106",
        "107",
        "110",
        "111",
        "1",
        "101",
        "103",
        "108",
        "109",
        "112",
        "999999",
        "garbage",
        "4294967296",
        "",
    ] {
        let doc = document(&format!(
            "<Project><Configuration Type='{cpu_type}'><Parameter Type='BASIC PARAMETER'><XGIBasicParam bp_ver='100' size='400' SCAN_WD_TIME_0='500'/></Parameter><Parameter Type='BASIC PARAMETER'><XGTBasicParam bp_ver='100' size='400' SCAN_WD_TIME='500'/></Parameter></Configuration></Project>"
        ));
        let expected = doc
            .configuration()
            .get(0)
            .unwrap()
            .cpu()
            .is_some_and(|cpu| cpu.family == "XGI");
        let records = doc.parameter().list();
        assert_eq!(
            records[0].basic_operation_fields().len(),
            usize::from(expected),
            "Type={cpu_type}"
        );
        // XGT eligibility is defined by its captured section layout, independently of CPU family.
        assert_eq!(records[1].basic_operation_fields().len(), 1);
        #[cfg(feature = "write")]
        {
            let before = doc.clone();
            let mut edited = doc;
            let result = edited
                .parameter_mut()
                .get(0)
                .unwrap()
                .edit_basic_operation(&[("SCAN_WD_TIME_0", "500", 600)]);
            assert_eq!(result.is_ok(), expected, "Type={cpu_type}");
            if !expected {
                assert_eq!(edited, before);
            }
        }
    }
}
