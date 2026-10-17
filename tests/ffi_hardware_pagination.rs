#![cfg(all(feature = "ffi", not(target_arch = "wasm32")))]

use flate2::{Compression, write::GzEncoder};
use serde_json::{Value, json};
#[cfg(feature = "write")]
use std::time::{Duration, Instant};
use std::{ffi::CStr, io::Write, ptr};
use xgwx::{XgwxDocument, ffi::*};

struct Document(*mut XgwxHandle);
impl Drop for Document {
    fn drop(&mut self) {
        unsafe { xgwx_document_free(self.0) };
    }
}
fn ok(status: i32) {
    assert_eq!(
        status,
        XGWX_OK,
        "{}",
        unsafe { CStr::from_ptr(xgwx_last_error()) }.to_string_lossy()
    );
}
impl Document {
    fn parse(bytes: &[u8]) -> Self {
        let mut handle = ptr::null_mut();
        ok(unsafe { xgwx_document_parse(bytes.as_ptr(), bytes.len(), &mut handle) });
        Self(handle)
    }
    fn xml(xml: &str) -> Self {
        Self::parse(&xml_bytes(xml))
    }
    fn hardware(&self, offset: usize, limit: usize) -> Value {
        let request =
            serde_json::to_vec(&json!({"section":"hardware", "offset":offset, "limit":limit}))
                .unwrap();
        let mut buffer = XgwxBuffer::default();
        ok(unsafe {
            xgwx_document_inspect_section_json(self.0, request.as_ptr(), request.len(), &mut buffer)
        });
        let result = unsafe {
            serde_json::from_slice::<Value>(std::slice::from_raw_parts(buffer.data, buffer.len))
        }
        .unwrap();
        unsafe { xgwx_buffer_free(&mut buffer) };
        result["data"].clone()
    }
}

fn xml_bytes(xml: &str) -> Vec<u8> {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    bytes
}

#[cfg(feature = "write")]
#[test]
fn large_duplicate_inventory_is_bounded_at_both_page_limits() {
    let doc = Document::xml(&format!(
        "<Project>{}<Module Base=\"0\" Slot=\"1\"/></Project>",
        "<Module Base=\"0\" Slot=\"0\"/>".repeat(100_000)
    ));
    for (offset, limit, length, next) in [
        (0, 1, 1, Some(1)),
        (0, 1000, 1000, Some(1000)),
        (100_000, 1, 1, None),
        (100_001, 1, 0, None),
    ] {
        let start = Instant::now();
        let hardware = doc.hardware(offset, limit);
        let elapsed = start.elapsed();
        eprintln!("100001 modules, offset {offset}, limit {limit}: {elapsed:?}");
        assert!(
            elapsed < Duration::from_secs(10),
            "inspection took {elapsed:?}"
        );
        let page = &hardware["modules"];
        assert_eq!(page["total"], 100_001);
        assert_eq!(page["items"].as_array().unwrap().len(), length);
        assert_eq!(page["next_offset"], json!(next));
        if length > 0 {
            assert_eq!(page["items"][0]["module_index"], offset);
        }
    }
}

#[cfg(feature = "write")]
#[test]
fn selected_options_keep_native_success_and_error_precedence() {
    let fixture = XgwxDocument::parse(include_bytes!("../fixtures/elements.xgwx")).unwrap();
    let mut created = xgwx::create_project("XGK-CPUSN", "LD").unwrap();
    created
        .base_mut()
        .by_number(0)
        .unwrap()
        .insert_module(0, "XGI-D24A/B")
        .unwrap();
    assert!(created.module().at(0, 0).unwrap().option_values().is_ok());
    for xml in [
        fixture.container().xml(),
        created.container().xml(),
        "<Project><Module/><Module Base=\"0\"/></Project>",
        "<Project><Configuration><Module Base=\"0\" Slot=\"0\"/></Configuration><Configuration><Module Base=\" 00 \" Slot=\"00\"/></Configuration></Project>",
        "<Project><Configuration Type=\"100\"><Base Base=\"1\"><Module Base=\"0\" Slot=\"0\"/></Base></Configuration></Project>",
        "<Project><Configuration Type=\"999\"><Module Base=\"0\" Slot=\"0\"/></Configuration></Project>",
    ] {
        let bytes = xml_bytes(xml);
        let native = XgwxDocument::parse(&bytes).unwrap();
        let doc = Document::parse(&bytes);
        for (index, summary) in native.module().summaries().iter().enumerate() {
            let expected = match (summary.base, summary.slot) {
                (Some(base), Some(slot)) => match native
                    .module()
                    .at(base, slot)
                    .and_then(|m| m.option_values())
                {
                    Ok(options) => {
                        json!({"status":"decoded", "items":options.iter().map(|o| json!({"key":o.key,"index":o.index,"value":o.value})).collect::<Vec<_>>()})
                    }
                    Err(error) => json!({"status":"unsupported", "error":error.to_string()}),
                },
                _ => json!({"status":"unsupported", "error":"Module has no base/slot identity"}),
            };
            let item = doc.hardware(index, 1)["modules"]["items"][0].clone();
            assert_eq!(item["module_index"], index);
            assert_eq!(item["option_values"], expected);
        }
    }
}

#[cfg(feature = "write")]
#[test]
fn small_hardware_pages_do_not_enrich_the_entire_inventory() {
    for count in [1_000, 2_000] {
        let doc = Document::xml(&format!(
            "<Project>{}</Project>",
            "<Module Base=\"0\" Slot=\"0\"/>".repeat(count)
        ));
        let start = Instant::now();
        let hardware = doc.hardware(0, 1);
        let elapsed = start.elapsed();
        eprintln!("{count} modules, limit 1: {elapsed:?}");
        assert!(
            elapsed < Duration::from_secs(5),
            "inspection took {elapsed:?}"
        );
        assert_eq!(hardware["modules"]["total"], count);
        assert_eq!(hardware["modules"]["items"].as_array().unwrap().len(), 1);
        assert_eq!(
            hardware["modules"]["items"][0]["option_values"]["error"],
            xgwx::XgwxError::AmbiguousModule { base: 0, slot: 0 }.to_string()
        );
    }
}

#[test]
fn hardware_pages_match_full_native_fixture_and_keep_absolute_indices() {
    let bytes = include_bytes!("../fixtures/elements.xgwx");
    let native = XgwxDocument::parse(bytes).unwrap();
    let doc = Document::parse(bytes);
    let full = doc.hardware(0, 1000);
    let count = native.module().summaries().len();
    assert!(count > 1 && count < 1000);
    for offset in [0, 1, count - 1, count, count + 1, usize::MAX] {
        let page = doc.hardware(offset, 1);
        for key in ["configurations", "bases", "typed_modules", "modules"] {
            let expected = full[key]["items"]
                .as_array()
                .unwrap()
                .iter()
                .skip(offset)
                .take(1)
                .cloned()
                .collect::<Vec<_>>();
            // Base contents use the same independent offset/limit as the outer page.
            if key != "bases" {
                assert_eq!(page[key]["items"], json!(expected), "{key}, {offset}");
            }
            assert_eq!(page[key]["total"], full[key]["total"]);
            assert_eq!(page[key]["offset"], json!(offset));
        }
    }
    assert_eq!(doc.hardware(0, 0)["modules"]["limit"], 1);
    assert_eq!(doc.hardware(0, usize::MAX)["modules"]["limit"], 1000);
}
