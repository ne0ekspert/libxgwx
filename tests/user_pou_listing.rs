use flate2::{Compression, write::GzEncoder};
use std::io::Write;
use std::time::{Duration, Instant};
use xgwx::XgwxDocument;

fn document(definitions: usize, nested: bool) -> XgwxDocument {
    let entry = if nested {
        "<UserFunction><UserFunction/></UserFunction>"
    } else {
        "<UserFunction/>"
    };
    let xml = format!(
        "<Project><Configuration Type=\"100\"><UserFunctions>{}</UserFunctions></Configuration></Project>",
        entry.repeat(definitions)
    );
    from_xml(&xml)
}

fn from_xml(xml: &str) -> XgwxDocument {
    XgwxDocument::parse(&xml_bytes(xml)).unwrap()
}

fn xml_bytes(xml: &str) -> Vec<u8> {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    bytes
}

#[test]
fn listing_scaling_workload() {
    for count in [10_000, 20_000] {
        let doc = document(count, false);
        let start = Instant::now();
        let entries = doc.user_pou().list();
        let elapsed = start.elapsed();
        eprintln!("{count} definitions: {elapsed:?}");
        // Generous headroom over the linear path, which takes milliseconds in debug
        // builds. The repeated document scan took over 20 seconds at 20,000 entries.
        assert!(elapsed < Duration::from_secs(5), "listing took {elapsed:?}");
        assert_eq!(entries.len(), count);
        assert!(
            entries.iter().enumerate().all(|(i, p)| p.index == i
                && p.kind.is_none()
                && !p.editable
                && p.reason.is_some())
        );
    }
}

#[cfg(feature = "ffi")]
#[test]
fn large_list_is_available_through_ffi_listing_and_inspection() {
    use std::ptr;
    use xgwx::ffi::*;

    let bytes = xml_bytes(&format!(
        "<Project><Configuration Type=\"100\"><UserFunctions>{}</UserFunctions></Configuration></Project>",
        "<UserFunction/>".repeat(100_000)
    ));
    unsafe {
        let mut handle = ptr::null_mut();
        assert_eq!(
            xgwx_document_parse(bytes.as_ptr(), bytes.len(), &mut handle),
            XGWX_OK
        );
        for inspect in [false, true] {
            let mut buffer = XgwxBuffer::default();
            let status = if inspect {
                xgwx_document_inspect_json(handle, &mut buffer)
            } else {
                xgwx_document_list_json(handle, XGWX_LIST_USER_POUS, &mut buffer)
            };
            assert_eq!(status, XGWX_OK);
            let json: serde_json::Value =
                serde_json::from_slice(std::slice::from_raw_parts(buffer.data, buffer.len))
                    .unwrap();
            xgwx_buffer_free(&mut buffer);
            let entries = &json[if inspect { "user_pous" } else { "items" }];
            assert_eq!(entries.as_array().unwrap().len(), 100_000);
            assert_eq!(entries[99_999]["index"], 99_999);
            assert_eq!(entries[99_999]["editable"], false);
        }
        xgwx_document_free(handle);
    }
}

#[test]
fn large_flat_and_nested_lists_retain_all_read_only_entries() {
    for (count, nested) in [(100_000, false), (50_000, true)] {
        let doc = document(count, nested);
        assert!(doc.user_pou().can_create());
        let entries = doc.user_pou().list();
        assert_eq!(entries.len(), 100_000);
        assert!(
            entries
                .iter()
                .enumerate()
                .all(|(i, p)| p.index == i && !p.editable && p.reason.is_some())
        );
    }
}

#[test]
fn capability_and_native_snapshots_remain_consistent() {
    let original = XgwxDocument::parse(include_bytes!(
        "../fixtures/user-pous/native-roundtrip.xgwx"
    ))
    .unwrap();
    let expected = original.user_pou().list();
    assert!(expected.iter().all(|p| p.editable));
    assert_eq!(expected.len(), 2);

    for configurations in [
        "",
        "<Configuration Type=\"999\"/>",
        "<Configuration Type=\"100\"/><Configuration Type=\"100\"/>",
    ] {
        // Exercise the same native definitions with different document-wide capabilities.
        let xml = original.container().xml();
        let start = xml.find("<UserFunctions").unwrap();
        let end = xml[start..].find("</UserFunctions>").unwrap() + start + "</UserFunctions>".len();
        let xml = format!("<Project>{configurations}{}</Project>", &xml[start..end]);
        let doc = from_xml(&xml);
        assert!(!doc.user_pou().can_create());
        let mut actual = doc.user_pou().list();
        assert!(actual.iter().all(|p| !p.editable && p.reason.is_some()));
        for p in &mut actual {
            p.editable = true;
            p.reason = None;
        }
        assert_eq!(actual, expected);
    }
}
