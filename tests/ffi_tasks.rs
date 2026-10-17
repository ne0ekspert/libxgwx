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
            serde_json::to_vec(&json!({"section":"tasks","offset":offset,"limit":limit})).unwrap();
        let mut out = XgwxBuffer::default();
        ok(unsafe {
            xgwx_document_inspect_section_json(self.0, req.as_ptr(), req.len(), &mut out)
        });
        serde_json::from_slice::<Value>(&take(out)).unwrap()["data"]["tasks"].clone()
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
fn settings(name: &str) -> Value {
    json!({"name":name,"priority":3,"task_number":1,"trigger":{"kind":"periodic","interval_ms":250}})
}
#[test]
fn ffi_task_lifecycle_uses_inspected_snapshots_and_keeps_membership() {
    let source = include_bytes!("../fixtures/tasks/xgk-native.xgwx");
    let doc = Document::parse(source);
    let native = XgwxDocument::parse(source).unwrap();
    let p = native.program().get(0).unwrap().summary();
    let page = doc.inspect(1, 1);
    assert_eq!(page["items"][0]["index"], 1);
    assert_eq!(page["total"], 5);
    ok(doc.apply(
        json!([{"operation":"create_task","configuration_index":0,"settings":settings("Fresh")}]),
    ));
    let expected = doc.inspect(5, 1)["items"][0]["expected"].clone();
    assert_eq!(
        expected["configuration_guid"],
        native
            .configuration()
            .get(0)
            .unwrap()
            .summary()
            .guid
            .unwrap()
    );
    ok(doc.apply(json!([{"operation":"assign_task_program","task_index":5,"expected":expected,"program_index":0,"expected_object_id":p.object_id,"expected_task":p.task}])));
    let bytes = doc.bytes();
    assert_ne!(
        doc.apply(json!([{"operation":"delete_task","task_index":5,"expected":expected}])),
        XGWX_OK
    );
    assert_eq!(doc.bytes(), bytes);
    ok(doc.apply(json!([{"operation":"update_task","task_index":5,"expected":expected,"settings":settings("Changed")}])));
    assert_eq!(
        XgwxDocument::parse(&doc.bytes())
            .unwrap()
            .program()
            .get(0)
            .unwrap()
            .summary()
            .task
            .as_deref(),
        Some("Changed")
    );
    assert_ne!(
        doc.apply(
            json!([{"operation":"rename_task","task_index":5,"expected":expected,"name":"stale"}])
        ),
        XGWX_OK
    );
    let expected = doc.inspect(5, 1)["items"][0]["expected"].clone();
    ok(doc.apply(
        json!([{"operation":"rename_task","task_index":5,"expected":expected,"name":"Worker"}]),
    ));
    let scan = doc.inspect(0, 1)["items"][0]["expected"].clone();
    ok(doc.apply(json!([{"operation":"assign_task_program","task_index":0,"expected":scan,"program_index":0,"expected_object_id":p.object_id,"expected_task":"Worker"}])));
    let expected = doc.inspect(5, 1)["items"][0]["expected"].clone();
    ok(doc.apply(json!([{"operation":"delete_task","task_index":5,"expected":expected}])));
    assert_eq!(doc.inspect(0, 100)["total"], 5);
}
#[test]
fn ffi_bad_fields_and_late_failure_roll_back_the_entire_transaction() {
    let doc = Document::parse(include_bytes!("../fixtures/tasks/xgk-native.xgwx"));
    let before = doc.bytes();
    let mut bad = settings("Fresh");
    bad["hidden"] = json!(true);
    for edits in [
        json!([{"operation":"create_task","configuration_index":0,"settings":bad}]),
        json!([{"operation":"create_task","configuration_index":0,"settings":settings("Fresh")},
            {"operation":"create_task","configuration_index":0,"settings":settings("DuplicateNumber")}]),
    ] {
        assert_ne!(doc.apply(edits), XGWX_OK);
        assert_eq!(doc.bytes(), before);
    }
}
