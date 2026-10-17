#![cfg(all(feature = "ffi", not(target_arch = "wasm32")))]
use std::{ffi::CStr, ptr};
use xgwx::ffi::*;
const PROJECT: &[u8] = include_bytes!("../fixtures/passwords/xgk-project-ascii.xgwx");
const PROGRAM: &[u8] = include_bytes!("../fixtures/passwords/xgi-st-native.xgwx");
#[test]
fn c_password_open_unlock_and_output_contracts() {
    unsafe {
        let mut doc = ptr::null_mut();
        assert_eq!(
            xgwx_document_parse(PROJECT.as_ptr(), PROJECT.len(), &mut doc),
            XGWX_PASSWORD_REQUIRED
        );
        assert!(doc.is_null());
        let wrong = b"wrong";
        assert_eq!(
            xgwx_document_parse_with_password(
                PROJECT.as_ptr(),
                PROJECT.len(),
                wrong.as_ptr(),
                wrong.len(),
                &mut doc
            ),
            XGWX_PASSWORD_INCORRECT
        );
        assert!(
            !CStr::from_ptr(xgwx_last_error())
                .to_str()
                .unwrap()
                .contains("wrong")
        );
        assert!(doc.is_null());
        let pass = b"Ab1! x";
        assert_eq!(
            xgwx_document_parse_with_password(
                PROJECT.as_ptr(),
                PROJECT.len(),
                pass.as_ptr(),
                pass.len(),
                &mut doc
            ),
            0
        );
        xgwx_document_free(doc);
        doc = ptr::null_mut();
        assert_eq!(
            xgwx_document_parse(PROGRAM.as_ptr(), PROGRAM.len(), &mut doc),
            0
        );
        assert_eq!(
            xgwx_document_unlock_program(doc, 0, wrong.as_ptr(), wrong.len()),
            XGWX_PASSWORD_INCORRECT
        );
        assert_eq!(
            xgwx_document_unlock_program(doc, 0, pass.as_ptr(), pass.len()),
            0
        );
        let mut out = XgwxBuffer::default();
        assert_eq!(xgwx_document_passwords_json(doc, &mut out), 0);
        let value: serde_json::Value =
            serde_json::from_slice(std::slice::from_raw_parts(out.data, out.len)).unwrap();
        assert_eq!(value["programs"][0]["protected"], true);
        assert_eq!(value["programs"][0]["locked"], false);
        xgwx_buffer_free(&mut out);
        #[cfg(feature = "write")]
        {
            assert_eq!(
                xgwx_document_set_program_password(
                    doc,
                    0,
                    ptr::null(),
                    1,
                    pass.as_ptr(),
                    pass.len()
                ),
                XGWX_INVALID_ARGUMENT
            );
            assert_eq!(
                xgwx_document_set_program_password(
                    doc,
                    0,
                    pass.as_ptr(),
                    pass.len(),
                    b"".as_ptr(),
                    0
                ),
                XGWX_OPERATION_ERROR
            );
            let next = b"12345678";
            assert_eq!(
                xgwx_document_set_program_password(
                    doc,
                    0,
                    pass.as_ptr(),
                    pass.len(),
                    next.as_ptr(),
                    next.len()
                ),
                0
            );
            assert_eq!(
                xgwx_document_set_program_password(
                    doc,
                    0,
                    next.as_ptr(),
                    next.len(),
                    ptr::null(),
                    0
                ),
                0
            );
            assert_eq!(
                xgwx_document_set_project_password(doc, ptr::null(), 0, pass.as_ptr(), pass.len()),
                0
            );
            assert_eq!(xgwx_document_serialize(doc, &mut out), 0);
            let mut reopened = ptr::null_mut();
            assert_eq!(
                xgwx_document_parse(out.data, out.len, &mut reopened),
                XGWX_PASSWORD_REQUIRED
            );
            assert_eq!(
                xgwx_document_parse_with_password(
                    out.data,
                    out.len,
                    pass.as_ptr(),
                    pass.len(),
                    &mut reopened
                ),
                0
            );
            assert_eq!(
                xgwx_document_set_project_password(
                    reopened,
                    pass.as_ptr(),
                    pass.len(),
                    ptr::null(),
                    0
                ),
                0
            );
            xgwx_document_free(reopened);
            xgwx_buffer_free(&mut out);
        }
        #[cfg(not(feature = "write"))]
        assert_eq!(
            xgwx_document_set_program_password(doc, 0, ptr::null(), 0, ptr::null(), 0),
            XGWX_FEATURE_UNAVAILABLE
        );
        xgwx_document_free(doc);
    }
}
