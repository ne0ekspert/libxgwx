#![cfg(all(feature = "ffi", feature = "write", not(target_arch = "wasm32")))]
use serde_json::json;
use std::{ffi::CStr, ptr};
use xgwx::{XgwxDocument, ffi::*};
#[test]
fn ffi_global_lifecycle_and_batch_rollback() {
    unsafe {
        let source = include_bytes!("../fixtures/empty-projects/new-xgk.xgwx");
        let mut doc = ptr::null_mut();
        assert_eq!(
            xgwx_document_parse(source.as_ptr(), source.len(), &mut doc),
            XGWX_OK
        );
        let apply = |edits: serde_json::Value| {
            let json = serde_json::to_vec(&json!({"schema_version":1,"edits":edits})).unwrap();
            xgwx_document_apply_edits_json(doc, json.as_ptr(), json.len())
        };
        let inspect = || {
            let mut out = XgwxBuffer::default();
            assert_eq!(xgwx_document_serialize(doc, &mut out), XGWX_OK);
            let bytes = std::slice::from_raw_parts(out.data, out.len).to_vec();
            xgwx_buffer_free(&mut out);
            XgwxDocument::parse(&bytes)
                .unwrap()
                .variable()
                .globals()
                .unwrap()
        };
        let insert = json!({"operation":"insert_variable","name":"Ready","data_type":"BIT","address_area":"M","address_number":42,"description":"Global ready"});
        assert_ne!(
            apply(
                json!([insert,{"operation":"delete_variable","variable_index":0,"expected_name":"Stale"}])
            ),
            XGWX_OK
        );
        assert!(inspect().is_empty());
        assert_eq!(
            apply(json!([insert])),
            XGWX_OK,
            "{}",
            CStr::from_ptr(xgwx_last_error()).to_string_lossy()
        );
        assert_eq!(inspect()[0].name.as_deref(), Some("Ready"));
        assert_eq!(
            apply(
                json!([{"operation":"delete_variable","variable_index":0,"expected_name":"Ready"}])
            ),
            XGWX_OK
        );
        assert!(inspect().is_empty());
        xgwx_document_free(doc);
    }
}
