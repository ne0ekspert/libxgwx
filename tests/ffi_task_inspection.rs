#![cfg(all(feature = "ffi", not(target_arch = "wasm32")))]
use flate2::{Compression, write::GzEncoder};
use serde_json::{Value, json};
use std::{
    ffi::CStr,
    io::Write,
    ptr,
    time::{Duration, Instant},
};
use xgwx::ffi::*;

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
    fn xml(xml: &str) -> Self {
        let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
        gzip.write_all(xml.as_bytes()).unwrap();
        let mut bytes = b"XG".to_vec();
        bytes.extend(gzip.finish().unwrap());
        let mut doc = ptr::null_mut();
        ok(unsafe { xgwx_document_parse(bytes.as_ptr(), bytes.len(), &mut doc) });
        Self(doc)
    }
    fn tasks(&self, offset: usize, limit: usize) -> Value {
        let request =
            serde_json::to_vec(&json!({"section":"tasks", "offset":offset, "limit":limit}))
                .unwrap();
        let mut buffer = XgwxBuffer::default();
        ok(unsafe {
            xgwx_document_inspect_section_json(self.0, request.as_ptr(), request.len(), &mut buffer)
        });
        let json: Value =
            unsafe { serde_json::from_slice(std::slice::from_raw_parts(buffer.data, buffer.len)) }
                .unwrap();
        unsafe { xgwx_buffer_free(&mut buffer) };
        json["data"]["tasks"].clone()
    }
}

#[test]
fn large_task_pages_keep_global_membership_indices_and_cursor_contract() {
    let records = (0..20_000)
        .map(|index| {
            format!("<Task Kind='2' Priority='3' TaskIndex='1' Device='100'>T{index}</Task>")
        })
        .collect::<String>();
    let programs = (0..20_000)
        .map(|index| format!("<Program Task='T{index}'/>"))
        .collect::<String>();
    let doc = Document::xml(&format!(
        "<Project><Configuration GUID='native'><Tasks>{records}</Tasks>{programs}</Configuration></Project>"
    ));
    for (offset, limit, length, next) in [
        (0, 1, 1, Some(1)),
        (0, 1000, 1000, Some(1000)),
        (19_999, 1, 1, None),
        (20_000, 1, 0, None),
        (usize::MAX, 1, 0, None),
    ] {
        let start = Instant::now();
        let page = doc.tasks(offset, limit);
        let elapsed = start.elapsed();
        eprintln!("20000 task/program pairs, offset {offset}, limit {limit}: {elapsed:?}");
        assert!(elapsed < Duration::from_secs(10), "page took {elapsed:?}");
        assert_eq!(page["total"], 20_000);
        assert_eq!(page["offset"], offset);
        assert_eq!(page["limit"], limit);
        assert_eq!(page["next_offset"], json!(next));
        let items = page["items"].as_array().unwrap();
        assert_eq!(items.len(), length);
        for (local, item) in items.iter().enumerate() {
            assert_eq!(item["index"], offset + local);
            assert_eq!(item["program_indices"], json!([offset + local]));
            assert!(item["membership_error"].is_null());
        }
    }
    assert_eq!(doc.tasks(0, 0)["limit"], 1);
    assert_eq!(doc.tasks(0, usize::MAX)["limit"], 1000);
}

#[test]
fn invalid_task_membership_stays_null_and_does_not_hide_snapshots() {
    let doc = Document::xml(
        "<Project><Task>orphan</Task><Configuration GUID='native'><Tasks><Task>A</Task><Task>A</Task><Task>B</Task></Tasks><Program Task='A'/></Configuration></Project>",
    );
    let page = doc.tasks(0, 100);
    let items = page["items"].as_array().unwrap();
    assert_eq!(items.len(), 4);
    for item in &items[..3] {
        assert!(item["program_indices"].is_null());
        assert!(item["membership_error"].is_string());
    }
    assert!(items[0]["configuration_index"].is_null());
    assert!(items[0]["expected"].is_null());
    assert_eq!(items[1]["expected"]["configuration_guid"], "native");
    assert_eq!(items[3]["program_indices"], json!([]));
    assert!(items[3]["membership_error"].is_null());
}
