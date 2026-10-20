#![cfg(all(feature = "ffi", not(target_arch = "wasm32")))]
use serde_json::Value;
#[cfg(feature = "write")]
use serde_json::json;
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
            serde_json::to_value(native.module().summaries()).unwrap(),
        ),
        (
            XGWX_LIST_NETWORKS,
            serde_json::to_value(native.network().list()).unwrap(),
        ),
        (
            XGWX_LIST_PROGRAMS,
            serde_json::to_value(native.program().programs()).unwrap(),
        ),
        (
            XGWX_LIST_VARIABLES,
            serde_json::to_value(native.variable().variables().unwrap()).unwrap(),
        ),
        (
            XGWX_LIST_NETWORK_MODULES,
            serde_json::to_value(native.network().module_summaries()).unwrap(),
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
        d.program()
            .ladder_programs()
            .into_iter()
            .map(|p| p.unwrap().data)
            .collect::<Vec<_>>()
    };
    assert_eq!(payloads(&original), payloads(&edited));
    assert_eq!(original.container().trailer(), edited.container().trailer());
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
    assert_eq!(
        before.network().fenet_config_infos(),
        after.network().fenet_config_infos()
    );
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

#[test]
fn user_pou_list_is_snake_case_and_available_without_write() {
    let doc = Document::parse(include_bytes!(
        "../fixtures/user-pous/native-roundtrip.xgwx"
    ));
    let items = doc.list(XGWX_LIST_USER_POUS);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["kind"], "function");
    assert_eq!(items[1]["kind"], "function_block");
    assert!(items.iter().all(|p| p["editable"] == true));
    assert!(items[0]["object_id"].is_string());
    assert!(items[0].get("objectId").is_none());
    assert_eq!(items[0]["return_type"], "DINT");
    assert!(
        items[0]["variables"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["data_type"].is_string())
    );
    let mut out = XgwxBuffer::default();
    assert_ok(unsafe { xgwx_document_inspect_json(doc.0, &mut out) });
    let summary: Value = serde_json::from_slice(&take_buffer(out)).unwrap();
    assert_eq!(summary["user_pous"], Value::Array(items));
    assert_eq!(summary["user_pou_creation"], true);
}

#[cfg(feature = "write")]
#[test]
fn iec_block_insert_and_wire_transactions_use_program_identity_and_roll_back() {
    let input = include_bytes!("../fixtures/iec-blocks/empty-with-instances.xgwx");
    let doc = Document::parse(input);
    let parsed = xgwx::XgwxDocument::parse(input).unwrap();
    let identity = parsed.program().list()[0].object_id.clone().unwrap();
    assert_ok(doc.apply(json!([{"operation":"insert_iec_ld_function","program_index":0,"expected_object_id":identity,"row_index":0,"raw_x":4,"function_name":"CTUD_DINT","operands":["BothCounter","FALSE","FALSE","FALSE","DINT#10"]}])));
    let d = xgwx::XgwxDocument::parse(&doc.bytes()).unwrap();
    let b = d
        .program()
        .ladder_programs()
        .remove(0)
        .unwrap()
        .iec_function_blocks()
        .unwrap()
        .remove(0);
    let wire = json!({"operation":"insert_iec_ld_function_output_wire","program_index":0,"expected_object_id":identity,"block_offset":b.record_offset,"expected_name":"CTUD_DINT","pin_name":"QD","start_x":7});
    let before = doc.bytes();
    assert_eq!(
        doc.apply(json!([wire.clone(), wire.clone()])),
        XGWX_OPERATION_ERROR
    );
    assert_eq!(doc.bytes(), before);
    assert_ok(doc.apply(json!([wire.clone()])));
    let before = doc.bytes();
    let mut stale = wire;
    stale["expected_object_id"] = json!("stale");
    assert_eq!(doc.apply(json!([stale])), XGWX_OPERATION_ERROR);
    assert_eq!(doc.bytes(), before);
}

impl Document {
    fn section(&self, section: &str, index: Option<usize>) -> Value {
        self.section_page(section, index, 0, 1000)
    }
    fn section_page(
        &self,
        section: &str,
        index: Option<usize>,
        offset: usize,
        limit: usize,
    ) -> Value {
        let request = serde_json::to_vec(
            &serde_json::json!({"section":section,"program_index":index,"offset":offset,"limit":limit}),
        )
        .unwrap();
        let mut out = XgwxBuffer::default();
        assert_ok(unsafe {
            xgwx_document_inspect_section_json(self.0, request.as_ptr(), request.len(), &mut out)
        });
        serde_json::from_slice::<Value>(&take_buffer(out)).unwrap()["data"].clone()
    }
}

#[test]
fn structured_inspection_is_available_without_writers() {
    let doc = Document::parse(include_bytes!("../fixtures/elements.xgwx"));
    assert!(
        doc.section("overview", None)["sections"]
            .as_array()
            .unwrap()
            .len()
            > 20
    );
    let details = doc.section("program", Some(0));
    assert!(details["details"]["cells"]["total"].as_u64().unwrap() > 0);
    assert_eq!(details["details"]["labels"]["total"], 0);
    let request = br#"{"section":"overview","programIndex":0}"#;
    let mut out = XgwxBuffer::default();
    assert_eq!(
        unsafe {
            xgwx_document_inspect_section_json(doc.0, request.as_ptr(), request.len(), &mut out)
        },
        XGWX_INVALID_ARGUMENT
    );
    assert!(out.data.is_null());
}

#[cfg(feature = "write")]
#[test]
fn extended_c_envelopes_reach_guarded_operations_and_preserve_the_document() {
    let doc = Document::parse(include_bytes!("../fixtures/empty-projects/new-xgk.xgwx"));
    let before = doc.bytes();
    let cases: Value =
        serde_json::from_str(include_str!("ffi-extended-preconditions.json")).unwrap();
    assert_eq!(cases.as_object().unwrap().len(), 110);
    for (name, args) in cases.as_object().unwrap() {
        let mut args = args.clone();
        args["operation"] = json!(name);
        let status = doc.apply(json!([args]));
        if name == "generate_io_variables" && status == XGWX_OK {
            continue;
        }
        assert!(
            status == XGWX_OPERATION_ERROR || status == XGWX_FEATURE_UNAVAILABLE,
            "{name}: status={status}, {}",
            unsafe { CStr::from_ptr(xgwx_last_error()) }.to_string_lossy()
        );
        assert_eq!(doc.bytes(), before, "{name}");
    }
}

#[cfg(feature = "write")]
#[test]
fn labels_c_roundtrip_identity_and_atomic_batches() {
    for cpu in ["XGK-CPUSN", "XGI-CPUE"] {
        let native = xgwx::create_project(cpu, "LD").unwrap();
        let doc = Document::parse(&native.to_verified_bytes().unwrap());
        let id = doc.list(XGWX_LIST_PROGRAMS)[0]["object_id"].clone();
        let insert = json!({"operation":"edit_ld_label","program_index":0,"expected_object_id":id,"raw_y":0,"expected":null,"replacement":"Start"});
        assert_ok(doc.apply(json!([insert])));
        let details = doc.section("program", Some(0));
        assert_eq!(details["details"]["labels"]["items"][0]["name"], "Start");
        let before = doc.bytes();
        assert_eq!(doc.apply(json!([
            {"operation":"edit_ld_label","program_index":0,"expected_object_id":id,"raw_y":0,"expected":"Start","replacement":"Next"},
            {"operation":"edit_ld_label","program_index":0,"expected_object_id":"stale","raw_y":0,"expected":"Next","replacement":"Fail"}
        ])),XGWX_OPERATION_ERROR);
        assert_eq!(doc.bytes(), before);
        assert_ok(doc.apply(json!([{"operation":"edit_ld_label","program_index":0,"expected_object_id":id,"raw_y":0,"expected":"Start","replacement":null}])));
        assert_eq!(
            doc.section("program", Some(0))["details"]["labels"]["total"],
            0
        );
    }
}

#[cfg(feature = "write")]
#[test]
fn opcua_c_settings_tags_and_stale_snapshot_roundtrip() {
    let doc = Document::parse(include_bytes!(
        "../fixtures/networks/eopct-tags-native.xgwx"
    ));
    let networks = doc.section("networks", None);
    let first = doc.section_page("networks.opcua_tags.0", None, 0, 1);
    assert_eq!(first["tags"]["next_offset"], 1);
    let second = doc.section_page("networks.opcua_tags.0", None, 1, 1);
    assert_eq!(
        second["tags"]["items"][0],
        networks["opcua_tags"]["items"][0]["tags"]["items"][1]
    );
    let config = &networks["opcua"]["items"][0];
    let base = config["base"].clone();
    let slot = config["slot"].clone();
    let old = config["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["field"] == "port")
        .unwrap()["value"]
        .clone();
    let edit = json!({"operation":"edit_opcua_settings","base":base,"slot":slot,"changes":[{"field":"port","expected_value":old,"replacement":"4841"}]});
    assert_ok(doc.apply(json!([edit])));
    assert!(
        doc.section("networks", None)["opcua"]["items"][0]["endpoint"]
            .as_str()
            .unwrap()
            .ends_with(":4841")
    );
    let tags = networks["opcua_tags"]["items"][0]["tags"]["items"].clone();
    let mut tag = tags[1].clone();
    tag["name"] = json!("EditedTag");
    let edit = json!({"operation":"edit_opcua_tag","configuration_index":0,"expected_tags":tags,"index":1,"tag":tag});
    assert_ok(doc.apply(json!([edit])));
    let before = doc.bytes();
    assert_eq!(doc.apply(json!([edit])), XGWX_OPERATION_ERROR);
    assert_eq!(doc.bytes(), before);
    let reparsed = Document::parse(&before);
    assert_eq!(
        reparsed.section("networks", None)["opcua_tags"]["items"][0]["tags"]["items"][1]["name"],
        "EditedTag"
    );
}

#[cfg(feature = "write")]
#[test]
fn sfc_and_text_declarations_use_exact_c_inspection_snapshots() {
    let doc = Document::parse(include_bytes!("../fixtures/sfc/native-loop.xgwx"));
    let id = doc.list(XGWX_LIST_PROGRAMS)[0]["object_id"].clone();
    assert_ok(doc.apply(json!([{"operation":"edit_sfc_entity","expected_object_id":id,"patch":{
        "program_index":0,"block_index":0,"entity_index":3,"expected_type":0,"expected_row":3,"expected_column":0,
        "field":"comment","expected_value":"Ready for cycle","replacement":"C chart"}}])));
    let block = doc.section("sfc.block.0", Some(0));
    let mut rows = block["editable_rows"]["items"].clone();
    let step = rows
        .as_array()
        .unwrap()
        .iter()
        .position(|r| r["kind"] == "step")
        .unwrap();
    rows[step]["comment"] = json!("C sequence");
    assert_ok(doc.apply(json!([{"operation":"replace_sfc_sequence","expected_object_id":id,"patch":{
        "program_index":0,"block_index":0,"expected_entities":block["entities"]["items"],"expected_rows":block["editable_rows"]["items"],"rows":rows}}])));
    let vars = doc.section("sfc.variables", Some(0))["items"].clone();
    assert_ok(doc.apply(json!([{"operation":"edit_sfc_variable","expected_object_id":id,"patch":{
        "program_index":0,"expected_variables":vars,"name":"CFlag","data_type":"BOOL","description":"C variable","remove":false}}])));
    assert!(
        doc.section("sfc.variables", Some(0))["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["name"] == "CFlag")
    );
    let native = xgwx::create_project("XGI-CPUE", "ST").unwrap();
    let doc = Document::parse(&native.to_verified_bytes().unwrap());
    let text = doc.section("text.variables", Some(0));
    assert_ok(doc.apply(json!([{"operation":"edit_text_variable","expected_object_id":text["object_id"],"patch":{
        "program_index":0,"expected_variables":text["variables"]["items"],"name":"CFlag","data_type":"BOOL","description":"C variable","remove":false}}])));
    assert!(
        doc.section("text.variables", Some(0))["variables"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["name"] == "CFlag")
    );
}

#[cfg(feature = "write")]
#[test]
fn legacy_cpu_operation_cannot_implicitly_choose_the_first_of_multiple_plcs() {
    let mut native = xgwx::create_project("XGK-CPUH", "LD").unwrap();
    let xml = native.container().xml();
    let parsed = roxmltree::Document::parse(xml).unwrap();
    let plc = parsed
        .descendants()
        .find(|node| node.has_tag_name("Configuration"))
        .unwrap();
    let mut combined = xml.to_owned();
    combined.insert_str(plc.range().end, &xml[plc.range()]);
    native.container_mut().replace_xml(combined).unwrap();
    let input = native.to_verified_bytes().unwrap();
    let doc = Document::parse(&input);
    for model in ["XGK-CPUH", "XGK-CPUUN"] {
        assert_eq!(
            doc.apply(json!([{"operation":"select_cpu","model":model}])),
            XGWX_OPERATION_ERROR
        );
        assert_eq!(doc.bytes(), input);
    }
}
