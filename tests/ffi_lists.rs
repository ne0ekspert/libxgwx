#![cfg(all(feature = "ffi", not(target_arch = "wasm32")))]
use serde_json::{Value, json};
use std::ffi::CStr;
use std::ptr;
use xgwx::ffi::*;

struct Document(*mut XgwxHandle);
impl Document {
    fn parse(bytes: &[u8]) -> Self {
        let mut doc = ptr::null_mut();
        assert_ok(unsafe { xgwx_document_parse(bytes.as_ptr(), bytes.len(), &mut doc) });
        Self(doc)
    }
    fn list(&self, selector: u32) -> Vec<Value> {
        let mut out = XgwxBuffer::default();
        assert_ok(unsafe { xgwx_document_list_json(self.0, selector, &mut out) });
        let value: Value = serde_json::from_slice(&take_buffer(out)).unwrap();
        assert_eq!(value["schema_version"], 1);
        value["items"].as_array().unwrap().clone()
    }
    #[cfg(feature = "write")]
    fn apply(&self, edits: Value) -> i32 {
        let bytes = serde_json::to_vec(&json!({"schema_version":1,"edits":edits})).unwrap();
        unsafe { xgwx_document_apply_edits_json(self.0, bytes.as_ptr(), bytes.len()) }
    }
    #[cfg(feature = "write")]
    fn bytes(&self) -> Vec<u8> {
        let mut out = XgwxBuffer::default();
        assert_ok(unsafe { xgwx_document_serialize(self.0, &mut out) });
        take_buffer(out)
    }
}
impl Drop for Document {
    fn drop(&mut self) {
        unsafe { xgwx_document_free(self.0) };
    }
}
fn assert_ok(status: i32) {
    assert_eq!(
        status,
        XGWX_OK,
        "{}",
        unsafe { CStr::from_ptr(xgwx_last_error()) }.to_string_lossy()
    );
}
fn take_buffer(mut buffer: XgwxBuffer) -> Vec<u8> {
    let bytes = if buffer.len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) }.to_vec()
    };
    unsafe { xgwx_buffer_free(&mut buffer) };
    bytes
}

#[test]
fn dedicated_lists_match_native_readers_and_reject_invalid_selectors() {
    let input = include_bytes!("../fixtures/elements.xgwx");
    let doc = Document::parse(input);
    let native = xgwx::XgwxDocument::parse(input).unwrap();
    for (selector, expected) in [
        (
            XGWX_LIST_MODULES,
            serde_json::to_value(native.modules()).unwrap(),
        ),
        (
            XGWX_LIST_NETWORKS,
            serde_json::to_value(native.networks()).unwrap(),
        ),
        (
            XGWX_LIST_PROGRAMS,
            serde_json::to_value(native.programs()).unwrap(),
        ),
        (
            XGWX_LIST_VARIABLES,
            serde_json::to_value(native.variables().unwrap()).unwrap(),
        ),
        (
            XGWX_LIST_NETWORK_MODULES,
            serde_json::to_value(native.network_modules()).unwrap(),
        ),
    ] {
        assert_eq!(Value::Array(doc.list(selector)), expected);
    }
    let mut out = XgwxBuffer::default();
    assert_eq!(
        unsafe { xgwx_document_list_json(doc.0, 999, &mut out) },
        XGWX_INVALID_ARGUMENT
    );
    assert!(out.data.is_null() && out.len == 0);
    assert_eq!(
        unsafe { xgwx_document_list_json(doc.0, XGWX_LIST_MODULES, ptr::null_mut()) },
        XGWX_INVALID_ARGUMENT
    );
}

#[cfg(feature = "write")]
#[test]
fn edits_all_four_lists_without_touching_program_payloads() {
    let input = include_bytes!("../fixtures/elements.xgwx");
    let doc = Document::parse(input);
    let module = doc.list(XGWX_LIST_MODULES).remove(0);
    let network = doc.list(XGWX_LIST_NETWORKS).remove(0);
    let program = doc.list(XGWX_LIST_PROGRAMS).remove(0);
    let variable = doc.list(XGWX_LIST_VARIABLES).remove(0);
    assert_ok(doc.apply(json!([
        {"operation":"update_module","base":module["base"],"slot":module["slot"],"expected_name":module["name"],"patch":{"comment":"C module comment"}},
        {"operation":"update_network","network_index":0,"expected_name":network["name"],"patch":{"name":"C network"}},
        {"operation":"update_program","program_index":0,"expected_object_id":program["object_id"],"patch":{"name":"CProgram","comment":"C program comment"}},
        {"operation":"update_variable","variable_index":0,"expected_name":variable["name"],"patch":{"name":"CVariable","description":"C global description"}},
    ])));
    assert_eq!(
        doc.list(XGWX_LIST_MODULES)[0]["comment"],
        "C module comment"
    );
    assert_eq!(doc.list(XGWX_LIST_NETWORKS)[0]["name"], "C network");
    assert_eq!(doc.list(XGWX_LIST_PROGRAMS)[0]["name"], "CProgram");
    assert_eq!(doc.list(XGWX_LIST_VARIABLES)[0]["name"], "CVariable");
    let original = xgwx::XgwxDocument::parse(input).unwrap();
    let edited = xgwx::XgwxDocument::parse(&doc.bytes()).unwrap();
    let payloads = |d: &xgwx::XgwxDocument| {
        d.ladder_programs()
            .into_iter()
            .map(|p| p.unwrap().data)
            .collect::<Vec<_>>()
    };
    assert_eq!(payloads(&original), payloads(&edited));
    assert_eq!(original.trailer, edited.trailer);
}

#[cfg(feature = "write")]
#[test]
fn stale_guards_and_schema_errors_roll_back_the_entire_batch() {
    let doc = Document::parse(include_bytes!("../fixtures/elements.xgwx"));
    let module = doc.list(XGWX_LIST_MODULES).remove(0);
    let original = doc.bytes();
    let first = json!({"operation":"update_module","base":module["base"],"slot":module["slot"],"expected_name":module["name"],"patch":{"comment":"must roll back"}});
    for bad in [
        json!({"operation":"update_module","base":module["base"],"slot":module["slot"],"expected_name":"changed model","patch":{"comment":"no"}}),
        json!({"operation":"update_network","network_index":0,"expected_name":"changed network","patch":{"name":"no"}}),
        json!({"operation":"update_program","program_index":0,"expected_object_id":"changed identity","patch":{"name":"no"}}),
        json!({"operation":"update_variable","variable_index":0,"expected_name":"changed variable","patch":{"description":"no"}}),
    ] {
        assert_eq!(doc.apply(json!([first.clone(), bad])), XGWX_OPERATION_ERROR);
        assert!(
            unsafe { CStr::from_ptr(xgwx_last_error()) }
                .to_string_lossy()
                .contains("edit 1:")
        );
        assert_eq!(doc.bytes(), original);
    }
    for request in [
        json!({"schema_version":2,"edits":[]}),
        json!({"schema_version":1,"edits":[],"ignored":"bad"}),
        json!({"schema_version":1,"edits":[{"operation":"unknown"}]}),
        json!({"schema_version":1,"edits":[{"operation":"update_module","base":0,"slot":0,"expected_name":"x","patch":{"typo":"bad"}}]}),
    ] {
        let bytes = serde_json::to_vec(&request).unwrap();
        assert_eq!(
            unsafe { xgwx_document_apply_edits_json(doc.0, bytes.as_ptr(), bytes.len()) },
            XGWX_INVALID_ARGUMENT
        );
        assert_eq!(doc.bytes(), original);
    }
}

#[cfg(feature = "write")]
#[test]
fn module_lifecycle_and_verified_options_use_the_native_catalog() {
    let doc = Document::parse(include_bytes!("../fixtures/empty-projects/new-xgk.xgwx"));
    assert_ok(
        doc.apply(json!([{"operation":"insert_module","base":0,"slot":0,"model":"XGF-RD8A"}])),
    );
    let initial_name = doc.list(XGWX_LIST_MODULES)[0]["name"].clone();
    assert_ok(doc.apply(json!([{"operation":"select_module","base":0,"slot":0,"expected_name":initial_name,"model":"XGF-AD8A"}])));
    let expected_name = doc.list(XGWX_LIST_MODULES)[0]["name"].clone();
    let mut options = XgwxBuffer::default();
    assert_ok(unsafe { xgwx_document_module_options_json(doc.0, 0, 0, &mut options) });
    let options: Value = serde_json::from_slice(&take_buffer(options)).unwrap();
    let option = &options["items"][0];
    let mut catalog = XgwxBuffer::default();
    assert_ok(unsafe { xgwx_module_catalog_json(&mut catalog) });
    let catalog: Value = serde_json::from_slice(&take_buffer(catalog)).unwrap();
    let module = catalog["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["model"] == "XGF-AD8A")
        .unwrap();
    let values = module["options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["key"] == option["key"])
        .unwrap();
    let value = values["values"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["value"] != option["value"])
        .unwrap()["value"]
        .clone();
    assert_eq!(expected_name, module["name"]);
    assert_ok(doc.apply(json!([{"operation":"set_module_option","base":0,"slot":0,"expected_name":expected_name,"key":option["key"],"index":option["index"],"expected_value":option["value"],"value":value}])));
    let current = doc.bytes();
    assert_eq!(doc.apply(json!([{"operation":"set_module_option","base":0,"slot":0,"expected_name":expected_name,"key":option["key"],"index":option["index"],"expected_value":option["value"],"value":value}])), XGWX_OPERATION_ERROR);
    assert_eq!(doc.bytes(), current);
    assert_ok(doc.apply(
        json!([{"operation":"delete_module","base":0,"slot":0,"expected_name":expected_name}]),
    ));
    assert!(doc.list(XGWX_LIST_MODULES).is_empty());
}

#[cfg(feature = "write")]
#[test]
fn linked_network_module_edits_keep_protocol_configuration_intact() {
    let doc = Document::parse(include_bytes!("../fixtures/empty-projects/new-xgk.xgwx"));
    assert_ok(
        doc.apply(json!([{"operation":"insert_module","base":0,"slot":0,"model":"XGL-EFMT(B)"}])),
    );
    let module = doc.list(XGWX_LIST_NETWORK_MODULES).remove(0);
    let before = xgwx::XgwxDocument::parse(&doc.bytes()).unwrap();
    assert_ok(doc.apply(json!([{"operation":"update_network_module","base":0,"slot":0,"expected_config_name":module["config_name"],"patch":{"alias":"C device alias"}}])));
    assert_eq!(
        doc.list(XGWX_LIST_NETWORK_MODULES)[0]["alias"],
        "C device alias"
    );
    let after = xgwx::XgwxDocument::parse(&doc.bytes()).unwrap();
    assert_eq!(before.fenet_config_infos(), after.fenet_config_infos());
    let current = doc.bytes();
    assert_eq!(doc.apply(json!([{"operation":"update_network_module","base":0,"slot":0,"expected_config_name":"stale","patch":{"alias":"no"}}])),XGWX_OPERATION_ERROR);
    assert_eq!(doc.bytes(), current);
}

#[cfg(feature = "write")]
#[test]
fn input_filter_edits_validate_the_current_native_value() {
    let doc = Document::parse(include_bytes!("../fixtures/elements.xgwx"));
    let module = doc
        .list(XGWX_LIST_MODULES)
        .into_iter()
        .find(|m| !m["input_filter_raw"].is_null())
        .unwrap();
    let edit = json!({"operation":"set_module_input_filter","base":module["base"],"slot":module["slot"],"expected_name":module["name"],"expected_value":module["input_filter_raw"],"value":5});
    assert_ok(doc.apply(json!([edit.clone()])));
    let edited = doc
        .list(XGWX_LIST_MODULES)
        .into_iter()
        .find(|m| m["base"] == module["base"] && m["slot"] == module["slot"])
        .unwrap();
    assert_eq!(edited["input_filter_raw"], 5);
    let current = doc.bytes();
    assert_eq!(doc.apply(json!([edit])), XGWX_OPERATION_ERROR);
    assert_eq!(doc.bytes(), current);
}

#[cfg(feature = "write")]
#[test]
fn program_lifecycle_and_source_edits_follow_stable_identities() {
    let mut handle = ptr::null_mut();
    assert_ok(unsafe {
        xgwx_project_create(b"XGI-CPUE".as_ptr(), 8, b"ST".as_ptr(), 2, &mut handle)
    });
    let doc = Document(handle);
    let first = doc.list(XGWX_LIST_PROGRAMS)[0]["object_id"].clone();
    let id = "11111111-1111-4111-8111-111111111111";
    assert_ok(doc.apply(json!([{"operation":"create_program","name":"CSource","language":"ST","object_id":id,"symbol_id":"22222222-2222-4222-8222-222222222222"}])));
    assert_eq!(doc.list(XGWX_LIST_PROGRAMS).len(), 2);
    assert_ok(doc.apply(json!([
        {"operation":"edit_text_program","program_index":1,"expected_object_id":id,"expected_language":"ST","expected_source":"","source":"(* C API 😀 *)\r\n"},
        {"operation":"move_program","from":1,"to":0,"expected_object_id":id,"expected_target_id":first},
    ])));
    assert_eq!(doc.list(XGWX_LIST_PROGRAMS)[0]["object_id"], id);
    assert_eq!(
        doc.list(XGWX_LIST_TEXT_PROGRAMS)[0]["source"],
        "(* C API 😀 *)\r\n"
    );
    assert_ok(
        doc.apply(
            json!([{"operation":"delete_program","program_index":0,"expected_object_id":id}]),
        ),
    );
    assert_eq!(doc.list(XGWX_LIST_PROGRAMS).len(), 1);
    assert_eq!(doc.list(XGWX_LIST_PROGRAMS)[0]["object_id"], first);
}
