use base64::Engine;
use flate2::{Compression, write::GzEncoder};
use std::io::Write;
use xgwx::{XgwxDocument, XgwxError};

fn field(bytes: &mut Vec<u8>, value: &str) {
    let count = value.encode_utf16().count();
    assert!(count <= 255);
    bytes.extend([0xff, 0xfe, 0xff, count as u8]);
    bytes.extend(value.encode_utf16().flat_map(u16::to_le_bytes));
}

fn array(name: &str, bounds: &str, maps: &[Vec<(&str, &str)>; 6]) -> Vec<u8> {
    let mut bytes = Vec::new();
    field(&mut bytes, "PB50");
    field(&mut bytes, name);
    bytes.extend(1u32.to_le_bytes());
    bytes.extend(22u32.to_le_bytes());
    field(&mut bytes, &format!("ARRAY[{bounds}] OF BOOL"));
    bytes.extend(0u32.to_le_bytes());
    for _ in 0..4 {
        field(&mut bytes, "");
    }
    bytes.extend(u32::MAX.to_le_bytes());
    bytes.extend(0u32.to_le_bytes());
    bytes.extend(u32::MAX.to_le_bytes());
    for entries in maps {
        bytes.extend((entries.len() as u32).to_le_bytes());
        for (key, value) in entries {
            field(&mut bytes, key);
            field(&mut bytes, value);
        }
    }
    field(&mut bytes, "");
    field(&mut bytes, "");
    bytes
}

fn document(records: &[u8], count: &str, compressed: bool) -> XgwxDocument {
    let payload = if compressed {
        let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::best());
        encoder.write_all(records).unwrap();
        encoder.finish().unwrap()
    } else {
        records.to_vec()
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(payload);
    let xml = format!(
        "<Project><Program Kind='4'><Body><ST_Program Version='Ver 1.1' StepCount='0'><CodeList CodeCount='0'/><Breakpoints/><Bookmarks/></ST_Program></Body><LocalVar><Symbols Count='{count}' Compressed='{}'>{encoded}</Symbols></LocalVar></Program></Project>",
        u8::from(compressed)
    );
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    XgwxDocument::parse(&bytes).unwrap()
}

#[test]
fn many_maximum_capacity_arrays_with_empty_initializers_remain_inspectable() {
    let records: Vec<_> = (0..1024)
        .flat_map(|i| array(&format!("Array{i}"), "0..65535", &Default::default()))
        .collect();
    let doc = document(&records, "1024", true);
    let variables = doc.variable().sfc_variables(0).unwrap();
    assert_eq!(variables.len(), 1024);
    assert!(variables.iter().all(|v| {
        let declaration = v.declaration.as_ref().unwrap();
        declaration.initial_value.is_empty() && declaration.dimensions[0].upper == 65535
    }));
    let programs = doc.program().text_programs();
    assert_eq!(programs[0].variables.len(), 1024);
    assert!(programs[0].variables_error.is_none());
}

#[test]
fn canonical_prefixes_keep_values_for_large_and_multidimensional_arrays() {
    for (bounds, entries, expected) in [
        ("0..65535", vec![("Values[0]", "1")], "1"),
        (
            "-2..-1,5..6",
            vec![
                ("Values[-1,5]", "0"),
                ("Values[-2,6]", "1"),
                ("Values[-2,5]", "1"),
            ],
            "2(1),0",
        ),
        ("0..1", vec![("Values[1]", "0"), ("Values[0]", "1")], "1,0"),
        ("0..255,0..255", vec![("Values[0,0]", "1")], "1"),
    ] {
        let mut maps: [Vec<(&str, &str)>; 6] = Default::default();
        maps[2] = entries;
        for compressed in [false, true] {
            let doc = document(&array("Values", bounds, &maps), "1", compressed);
            assert_eq!(
                doc.variable().sfc_variables(0).unwrap()[0]
                    .declaration
                    .as_ref()
                    .unwrap()
                    .initial_value,
                expected
            );
        }
    }
}

#[test]
fn gaps_foreign_alias_duplicate_and_out_of_range_keys_remain_errors() {
    for entries in [
        vec![("Values[1]", "1")],
        vec![("Values[0]", "1"), ("Values[2]", "0")],
        vec![("Other[0]", "1")],
        vec![("Values[00]", "1")],
        vec![("Values[65536]", "1")],
        vec![("Values[0]", "1"), ("Values[0]", "0")],
    ] {
        let mut maps: [Vec<(&str, &str)>; 6] = Default::default();
        maps[2] = entries;
        let doc = document(&array("Values", "0..65535", &maps), "1", true);
        assert!(doc.variable().sfc_variables(0).is_err());
    }
}

#[test]
fn unsupported_maps_and_oversized_initializer_counts_remain_errors() {
    for map in [0, 1, 3, 4, 5] {
        let mut maps: [Vec<(&str, &str)>; 6] = Default::default();
        maps[map] = vec![("Values[0]", "1")];
        assert!(
            document(&array("Values", "0..65535", &maps), "1", false)
                .variable()
                .sfc_variables(0)
                .is_err()
        );
    }
    let mut maps: [Vec<(&str, &str)>; 6] = Default::default();
    maps[2] = vec![("Values[0]", "1"), ("Values[1]", "0")];
    assert!(
        document(&array("Values", "0..0", &maps), "1", false)
            .variable()
            .sfc_variables(0)
            .is_err()
    );
}

#[test]
fn declared_symbol_count_is_checked_before_array_metadata() {
    // The malformed bounds would fail metadata decoding if it ran first.
    let records = array("Values", "invalid", &Default::default());
    for count in ["0", "2", "invalid"] {
        assert!(matches!(
            document(&records, count, true).variable().sfc_variables(0),
            Err(XgwxError::InvalidVariableRecord { index: 0 })
        ));
    }
}
