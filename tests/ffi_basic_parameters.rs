#![cfg(all(feature = "ffi", feature = "write", not(target_arch = "wasm32")))]
use serde_json::{Value, json};
use std::{ffi::CStr, ptr};
use xgwx::{XgwxDocument, ffi::*};
struct Document(*mut XgwxHandle);
impl Drop for Document {
    fn drop(&mut self) {
        unsafe { xgwx_document_free(self.0) }
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
fn take(mut buffer: XgwxBuffer) -> Vec<u8> {
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) }.to_vec();
    unsafe { xgwx_buffer_free(&mut buffer) };
    bytes
}
impl Document {
    fn parse(bytes: &[u8]) -> Self {
        let mut doc = ptr::null_mut();
        ok(unsafe { xgwx_document_parse(bytes.as_ptr(), bytes.len(), &mut doc) });
        Self(doc)
    }
    fn inspect(&self, offset: usize, limit: usize) -> Value {
        let req =
            serde_json::to_vec(&json!({"section":"parameters","offset":offset,"limit":limit}))
                .unwrap();
        let mut out = XgwxBuffer::default();
        ok(unsafe {
            xgwx_document_inspect_section_json(self.0, req.as_ptr(), req.len(), &mut out)
        });
        serde_json::from_slice::<Value>(&take(out)).unwrap()["data"].clone()
    }
    fn apply(&self, edits: Value) -> i32 {
        let req = serde_json::to_vec(&json!({"schema_version":1,"edits":edits})).unwrap();
        unsafe { xgwx_document_apply_edits_json(self.0, req.as_ptr(), req.len()) }
    }
    fn bytes(&self) -> Vec<u8> {
        let mut out = XgwxBuffer::default();
        ok(unsafe { xgwx_document_serialize(self.0, &mut out) });
        take(out)
    }
}
fn native(model: &str, state: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/fixtures/basic-parameters/{model}-{state}-resaved.xgwx",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}
#[test]
fn ffi_native_control_transitions_roundtrip_and_keep_other_xml() {
    for model in ["xgk", "xgi"] {
        for (from, to) in [("off", "on"), ("on", "off"), ("off", "overall")] {
            let bytes = native(model, from);
            let before = XgwxDocument::parse(&bytes).unwrap();
            let target = XgwxDocument::parse(&native(model, to)).unwrap();
            let source = before
                .parameter()
                .list()
                .into_iter()
                .find(|p| !p.basic_operation_fields().is_empty())
                .unwrap();
            let desired = target
                .parameter()
                .get(source.index())
                .unwrap()
                .basic_operation_fields();
            let changes: Vec<Value> = source.basic_operation_fields().iter().filter_map(|f| {
                let t = desired.iter().find(|t| t.name==f.name).unwrap();
                (f.value!=t.value).then(||json!({"name":f.name,"expected":f.value,"replacement":t.value.parse::<u32>().unwrap()}))
            }).collect();
            assert!(!changes.is_empty());
            let doc = Document::parse(&bytes);
            let inspected = doc.inspect(0, 100);
            let item = inspected["parameters"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["parameter_index"] == source.index())
                .unwrap();
            assert_eq!(
                item["basic_operation_fields"].as_array().unwrap().len(),
                source.basic_operation_fields().len()
            );
            let edit = json!({"operation":"edit_basic_operation","parameter_index":source.index(),"changes":changes});
            ok(doc.apply(json!([edit.clone()])));
            let saved = doc.bytes();
            let mut expected = before.clone();
            let owned: Vec<_> = changes
                .iter()
                .map(|c| {
                    (
                        c["name"].as_str().unwrap(),
                        c["expected"].as_str().unwrap(),
                        c["replacement"].as_u64().unwrap() as u32,
                    )
                })
                .collect();
            expected
                .parameter_mut()
                .get(source.index())
                .unwrap()
                .edit_basic_operation(&owned)
                .unwrap();
            assert_eq!(
                XgwxDocument::parse(&saved).unwrap().container().xml(),
                expected.container().xml()
            );
            let reopened = Document::parse(&saved);
            assert_eq!(reopened.inspect(0, 100), doc.inspect(0, 100));
            assert_ne!(doc.apply(json!([edit])), XGWX_OK);
            assert_eq!(doc.bytes(), saved);
            assert_eq!(doc.inspect(999, 1)["parameters"]["items"], json!([]));
        }
    }
}
#[test]
fn ffi_rejections_and_batch_rollback() {
    let doc = Document::parse(&native("xgk", "off"));
    let state = doc.inspect(0, 100);
    let item = state["parameters"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| !p["basic_operation_fields"].as_array().unwrap().is_empty())
        .unwrap();
    let index = item["parameter_index"].clone();
    let field = item["basic_operation_fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "SCAN_WD_TIME")
        .unwrap();
    let valid = json!({"operation":"edit_basic_timing","parameter_index":index,"changes":[{"name":"SCAN_WD_TIME","expected":field["value"],"replacement":650}]});
    let snapshot = doc.bytes();
    for changes in [
        json!([{"name":"SCAN_WD_TIME","expected":field["value"],"replacement":1001}]),
        json!([{"name":"SCAN_WD_TIME","expected":field["value"],"replacement":650,"extra":true}]),
        json!([{"name":"RESET_DISABLE","expected":"0","replacement":1}]),
        json!([{"name":"STANDARD_INPUT_FILTER_TIME","expected":"1","replacement":0}]),
        json!([{"name":"SCAN_WD_TIME","expected":field["value"],"replacement":650},{"name":"SCAN_WD_TIME","expected":field["value"],"replacement":500}]),
    ] {
        assert_ne!(doc.apply(json!([{"operation":"edit_basic_operation","parameter_index":index,"changes":changes}])),XGWX_OK);
        assert_eq!(doc.bytes(), snapshot);
    }
    assert_ne!(doc.apply(json!([valid.clone(),{"operation":"edit_basic_operation","parameter_index":99999,"changes":[]}])),XGWX_OK);
    assert_eq!(doc.bytes(), snapshot);
    ok(doc.apply(json!([valid])));
}
