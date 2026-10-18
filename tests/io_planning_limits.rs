#![cfg(feature = "write")]
use flate2::{Compression, write::GzEncoder};
use std::io::Write;
use xgwx::{XgwxDocument, XgwxError};

fn field(out: &mut Vec<u8>, value: &str) {
    assert!(value.encode_utf16().count() <= 255);
    out.extend_from_slice(&[255, 254, 255, value.encode_utf16().count() as u8]);
    for unit in value.encode_utf16() {
        out.extend_from_slice(&unit.to_le_bytes());
    }
}

fn symbols(count: usize, name: impl Fn(usize) -> String) -> String {
    use base64::Engine;
    let mut data = Vec::new();
    for i in 0..count {
        for value in ["SV5.0", &name(i), "P"] {
            field(&mut data, value);
        }
        data.extend_from_slice(&0u32.to_le_bytes());
        for value in ["LWORD", "", ""] {
            field(&mut data, value);
        }
        data.extend_from_slice(&0u32.to_le_bytes());
        field(&mut data, "[0:0]");
    }
    format!(
        "<Symbols Count='{count}'>{}</Symbols>",
        base64::engine::general_purpose::STANDARD.encode(data)
    )
}

fn parse_xml(xml: &str) -> XgwxDocument {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    XgwxDocument::parse(&bytes).unwrap()
}

fn document(cpu: u32, bases: usize, symbols: &str) -> XgwxDocument {
    let hardware: String = (0..bases)
        .map(|base| {
            let modules: String = (0..12)
                .map(|slot| format!("<Module Id='42243' SubType='0' Base='{base}' Slot='{slot}'/>"))
                .collect();
            format!("<Base Base='{base}' SlotCount='12'>{modules}</Base>")
        })
        .collect();
    let xml = format!(
        "<Project><Configuration Type='{cpu}'>PLC<XGTBasicParam IO_POINT_METHOD='0'/>{hardware}</Configuration>{symbols}</Project>"
    );
    parse_xml(&xml)
}

#[test]
fn planner_rejects_impossible_geometry_before_allocating_rows() {
    let doc = document(0, 9, "<Symbols Count='0'/>");
    match doc.variable().preview_io_variables() {
        Err(_) => {}
        Ok(rows) => panic!(
            "accepted impossible geometry and allocated {} rows",
            rows.len()
        ),
    }
}

#[test]
fn planning_limits_apply_before_symbol_payload_decoding() {
    let doc = document(0, 9, "<Symbols Count='1'>invalid base64</Symbols>");
    assert!(matches!(
        doc.variable().preview_io_variables(),
        Err(XgwxError::ResourceLimitExceeded { .. }) | Err(XgwxError::CpuHardwareLimit { .. })
    ));
}

#[test]
fn scoped_preview_and_generation_cannot_bypass_whole_project_limits() {
    let mut doc = document(0, 9, "<Symbols Count='0'/>");
    let original = doc.clone();
    assert!(
        doc.variable()
            .preview_io_variables_for(Some(0), Some(0))
            .is_err()
    );
    assert!(
        doc.variable_mut()
            .generate_io_variables_for(Some(0), Some(0))
            .is_err()
    );
    assert_eq!(doc, original);
}

#[test]
fn largest_supported_geometry_and_cpu_specific_limits_are_preserved() {
    let doc = document(0, 8, "<Symbols Count='0'/>");
    assert_eq!(doc.variable().preview_io_variables().unwrap().len(), 6144);
    let doc = document(17, 5, "<Symbols Count='0'/>");
    assert!(doc.variable().preview_io_variables().is_err());
}

#[test]
fn native_empty_reserved_bases_remain_accepted() {
    let doc =
        XgwxDocument::parse(include_bytes!("../fixtures/empty-projects/new-xgk.xgwx")).unwrap();
    assert!(doc.variable().preview_io_variables().unwrap().is_empty());
}

#[test]
fn collision_associations_are_bounded_before_owned_rows() {
    let doc = document(0, 1, &symbols(1100, |i| format!("Existing{i}")));
    assert!(matches!(
        doc.variable().preview_io_variables(),
        Err(XgwxError::ResourceLimitExceeded {
            resource: "I/O collision associations",
            limit: 65_536
        })
    ));
}

#[test]
fn repeated_long_names_are_bounded_by_copied_bytes() {
    let long = "가".repeat(250);
    let doc = document(0, 1, &symbols(350, |i| format!("{i}{long}")));
    assert!(matches!(
        doc.variable().preview_io_variables(),
        Err(XgwxError::ResourceLimitExceeded {
            resource: "I/O collision name bytes",
            limit: 16_777_216
        })
    ));
}

#[test]
fn scoped_generation_cannot_bypass_collision_budget_or_mutate_on_error() {
    let mut doc = document(0, 1, &symbols(1100, |i| format!("Existing{i}")));
    let before = doc.clone();
    assert!(
        doc.variable()
            .preview_io_variables_for(Some(0), Some(1))
            .is_err()
    );
    assert!(
        doc.variable_mut()
            .generate_io_variables_for(Some(0), Some(1))
            .is_err()
    );
    assert_eq!(doc, before);
}

#[test]
fn oversized_raw_hardware_inventories_fail_before_summary_cloning() {
    let bases = "<Base Base='0' SlotCount='12'/>".repeat(20_000);
    let doc = parse_xml(&format!(
        "<Project><Configuration Type='0'><XGTBasicParam IO_POINT_METHOD='0'/>{bases}</Configuration></Project>"
    ));
    assert!(matches!(
        doc.variable().preview_io_variables(),
        Err(XgwxError::ResourceLimitExceeded {
            resource: "I/O generation bases",
            limit: 8
        })
    ));
    assert!(matches!(
        doc.variable().preview_io_variables_for(Some(0), Some(0)),
        Err(XgwxError::ResourceLimitExceeded {
            resource: "I/O generation bases",
            limit: 8
        })
    ));

    let modules = "<Module Id='42243' SubType='0' Base='0' Slot='0'/>".repeat(20_000);
    let doc = parse_xml(&format!(
        "<Project><Configuration Type='0'><XGTBasicParam IO_POINT_METHOD='0'/><Base Base='0' SlotCount='12'>{modules}</Base></Configuration></Project>"
    ));
    assert!(matches!(
        doc.variable().preview_io_variables(),
        Err(XgwxError::ResourceLimitExceeded {
            resource: "I/O generation modules",
            limit: 96
        })
    ));
}
