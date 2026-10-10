//! Native C ABI for language bindings. Enable `ffi`; add `write` for project
//! creation, guarded edits and verified serialization, and `il` for conversion.
//!
//! The declarations in `include/xgwx.h` are the ABI contract. Documents are
//! opaque handles. Returned buffers belong to Rust and must be released through
//! [`xgwx_buffer_free`]. UTF-8 inputs and outputs use explicit byte lengths;
//! buffers are not NUL-terminated. No function performs filesystem I/O.
//!
//! Fallible calls return an integer status. On failure, copy the thread-local
//! [`xgwx_last_error`] message before the next fallible call on that thread.
//! Rust unwinding panics become [`XGWX_PANIC`]; aborts and allocation failures
//! cannot be recovered. Invalid foreign pointers remain caller errors.
//!
//! A handle must not be freed or used concurrently with any other call involving
//! that handle. Initialize output handles to NULL and output buffers to zero.
//! Failed calls preserve the document and leave output slots unchanged.

use crate::XgwxDocument;
use std::cell::RefCell;
use std::ffi::{CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

#[cfg(feature = "write")]
mod edits;

/// Select the installed hardware module list.
pub const XGWX_LIST_MODULES: u32 = 1;
/// Select the network list.
pub const XGWX_LIST_NETWORKS: u32 = 2;
/// Select program metadata in document order.
pub const XGWX_LIST_PROGRAMS: u32 = 3;
/// Select global variables in symbol-table order.
pub const XGWX_LIST_VARIABLES: u32 = 4;
/// Select linked network modules.
pub const XGWX_LIST_NETWORK_MODULES: u32 = 5;
/// Select standalone ST/IL source and editability information.
pub const XGWX_LIST_TEXT_PROGRAMS: u32 = 6;

fn json_bytes(value: impl serde::Serialize) -> Result<Vec<u8>> {
    serde_json::to_vec(&value).map_err(operation)
}

/// Successful call.
pub const XGWX_OK: i32 = 0;
/// Null pointer, nonempty output, invalid UTF-8 or invalid argument.
pub const XGWX_INVALID_ARGUMENT: i32 = 1;
/// Invalid workspace container.
pub const XGWX_PARSE_ERROR: i32 = 2;
/// A guarded operation, conversion or serialization was rejected.
pub const XGWX_OPERATION_ERROR: i32 = 3;
/// This build does not enable the required optional feature.
pub const XGWX_FEATURE_UNAVAILABLE: i32 = 4;
/// Rust panicked while performing the request.
pub const XGWX_PANIC: i32 = 255;

/// Rust-owned bytes, matching `xgwx_buffer` in the C header.
#[repr(C)]
pub struct XgwxBuffer {
    pub data: *mut u8,
    pub len: usize,
}

impl Default for XgwxBuffer {
    fn default() -> Self {
        Self {
            data: ptr::null_mut(),
            len: 0,
        }
    }
}

/// Opaque document handle. C callers must not access its representation.
#[repr(C)]
pub struct XgwxHandle {
    _private: [u8; 0],
}

thread_local! {
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::default());
}

struct Failure(i32, String);
type Result<T> = std::result::Result<T, Failure>;

fn invalid(message: &str) -> Failure {
    Failure(XGWX_INVALID_ARGUMENT, message.into())
}

fn operation(error: impl std::fmt::Display) -> Failure {
    Failure(XGWX_OPERATION_ERROR, error.to_string())
}

fn call(f: impl FnOnce() -> Result<()>) -> i32 {
    let result = catch_unwind(AssertUnwindSafe(f));
    let (status, message) = match result {
        Ok(Ok(())) => (XGWX_OK, String::new()),
        Ok(Err(Failure(status, message))) => (status, message),
        Err(_) => (XGWX_PANIC, "Rust panic in libxgwx C API".into()),
    };
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = CString::new(message.replace('\0', "\\0")).unwrap();
    });
    status
}

unsafe fn input<'a>(data: *const u8, len: usize) -> Result<&'a [u8]> {
    if len > isize::MAX as usize {
        return Err(invalid("input length exceeds isize::MAX"));
    }
    if len == 0 {
        return Ok(&[]);
    }
    if data.is_null() {
        return Err(invalid("nonempty input has a NULL pointer"));
    }
    // SAFETY: The foreign caller supplies a readable allocation of len bytes.
    Ok(unsafe { std::slice::from_raw_parts(data, len) })
}

unsafe fn text<'a>(data: *const u8, len: usize) -> Result<&'a str> {
    // SAFETY: Forwarding the caller's input contract.
    let bytes = unsafe { input(data, len)? };
    std::str::from_utf8(bytes).map_err(|_| invalid("text must be valid UTF-8"))
}

unsafe fn handle<'a>(doc: *const XgwxHandle) -> Result<&'a XgwxDocument> {
    // SAFETY: Non-NULL document pointers must denote live handles from this API.
    unsafe { doc.cast::<XgwxDocument>().as_ref() }.ok_or_else(|| invalid("document is NULL"))
}

unsafe fn handle_mut<'a>(doc: *mut XgwxHandle) -> Result<&'a mut XgwxDocument> {
    // SAFETY: The caller grants exclusive access to the live handle.
    unsafe { doc.cast::<XgwxDocument>().as_mut() }.ok_or_else(|| invalid("document is NULL"))
}

unsafe fn document_output(out: *mut *mut XgwxHandle) -> Result<()> {
    if out.is_null() {
        return Err(invalid("document output is NULL"));
    }
    // SAFETY: The caller initializes a writable pointer slot to NULL.
    if !unsafe { *out }.is_null() {
        return Err(invalid("document output must initially be NULL"));
    }
    Ok(())
}

unsafe fn buffer_output(out: *mut XgwxBuffer) -> Result<()> {
    // SAFETY: The caller supplies a writable, initialized buffer struct.
    let out = unsafe { out.as_ref() }.ok_or_else(|| invalid("buffer output is NULL"))?;
    if !out.data.is_null() || out.len != 0 {
        return Err(invalid("buffer output must initially be empty"));
    }
    Ok(())
}

unsafe fn write_buffer(out: *mut XgwxBuffer, bytes: Vec<u8>) {
    if bytes.is_empty() {
        return;
    }
    let bytes = bytes.into_boxed_slice();
    let len = bytes.len();
    let data = Box::into_raw(bytes).cast::<u8>();
    // SAFETY: The caller validated an empty, writable output before allocation.
    unsafe { *out = XgwxBuffer { data, len } };
}

/// Return the C ABI version, currently 1.
#[unsafe(no_mangle)]
pub extern "C" fn xgwx_abi_version() -> u32 {
    1
}

/// Return feature bits: bit 0 = `write`, bit 1 = `il`.
#[unsafe(no_mangle)]
pub extern "C" fn xgwx_features() -> u32 {
    u32::from(cfg!(feature = "write")) | (u32::from(cfg!(feature = "il")) << 1)
}

/// Borrow the last error as a NUL-terminated UTF-8 string, empty after success.
/// The pointer is valid until the next fallible call on this thread or thread exit.
#[unsafe(no_mangle)]
pub extern "C" fn xgwx_last_error() -> *const c_char {
    LAST_ERROR.with(|slot| slot.borrow().as_ptr())
}

/// Parse a workspace from bytes, copying all retained data into a new handle.
///
/// # Safety
/// `data` must be readable for `len` bytes; NULL is allowed only for zero length.
/// `out` must point to a writable handle slot initialized to NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_parse(
    data: *const u8,
    len: usize,
    out: *mut *mut XgwxHandle,
) -> i32 {
    call(|| {
        // SAFETY: All foreign memory follows the documented caller contract.
        unsafe {
            document_output(out)?;
            let doc = XgwxDocument::parse(input(data, len)?)
                .map_err(|e| Failure(XGWX_PARSE_ERROR, e.to_string()))?;
            *out = Box::into_raw(Box::new(doc)).cast();
        }
        Ok(())
    })
}

/// Release a document. NULL is a no-op.
///
/// # Safety
/// A non-NULL pointer must be a live handle returned by this API, with no active
/// calls or references. Release it exactly once and discard the pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_free(doc: *mut XgwxHandle) {
    if !doc.is_null() {
        // SAFETY: Ownership of a live Box is returned by the foreign caller.
        drop(unsafe { Box::from_raw(doc.cast::<XgwxDocument>()) });
    }
}

/// Release a Rust-owned buffer and reset its fields. NULL and empty are no-ops.
///
/// # Safety
/// `buffer` must point to an initialized writable struct; any nonempty contents
/// must be an unchanged buffer returned by this API, not a copied ownership token.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_buffer_free(buffer: *mut XgwxBuffer) {
    // SAFETY: The caller supplies a writable buffer struct, or NULL.
    if let Some(buffer) = unsafe { buffer.as_mut() } {
        if !buffer.data.is_null() {
            let slice = ptr::slice_from_raw_parts_mut(buffer.data, buffer.len);
            // SAFETY: Reconstituting the Box<[u8]> allocated by write_buffer.
            drop(unsafe { Box::from_raw(slice) });
        }
        *buffer = XgwxBuffer::default();
    }
}

/// Return a UTF-8 JSON snapshot with `schema_version: 1` and snake_case fields.
/// Includes project, configurations, bases, modules, tasks, programs, networks,
/// network modules, globals, and supported program languages. Variable decoding
/// failures are reported in `variables_error`, while the other sections remain available.
///
/// # Safety
/// `doc` must be a live handle and `out` a writable zero-initialized buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_inspect_json(
    doc: *const XgwxHandle,
    out: *mut XgwxBuffer,
) -> i32 {
    call(|| {
        // SAFETY: Forwarding the documented handle and output contracts.
        unsafe {
            buffer_output(out)?;
            let doc = handle(doc)?;
            let (variables, variables_error) = match doc.variables() {
                Ok(v) => (Some(v), None),
                Err(e) => (None, Some(e.to_string())),
            };
            let summary = serde_json::json!({
                "schema_version": 1,
                "project": doc.project_info(),
                "configurations": doc.configurations(),
                "bases": doc.bases(),
                "modules": doc.modules(),
                "tasks": doc.tasks(),
                "programs": doc.programs(),
                "networks": doc.networks(),
                "network_modules": doc.network_modules(),
                "variables": variables,
                "variables_error": variables_error,
                "program_languages": doc.program_languages(),
            });
            write_buffer(out, serde_json::to_vec(&summary).map_err(operation)?);
        }
        Ok(())
    })
}

/// Copy the full decompressed XML as UTF-8 bytes.
///
/// # Safety
/// `doc` must be a live handle and `out` a writable zero-initialized buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_xml(doc: *const XgwxHandle, out: *mut XgwxBuffer) -> i32 {
    call(|| {
        // SAFETY: Forwarding the documented handle and output contracts.
        unsafe {
            buffer_output(out)?;
            write_buffer(out, handle(doc)?.xml.as_bytes().to_vec());
        }
        Ok(())
    })
}

/// Load one selected list as `{schema_version: 1, items: [...]}` JSON.
/// Globals that cannot be decoded return an operation error with no output.
///
/// # Safety
/// `doc` must be a live handle and `out` a writable zero-initialized buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_list_json(
    doc: *const XgwxHandle,
    list: u32,
    out: *mut XgwxBuffer,
) -> i32 {
    call(|| {
        // SAFETY: Forwarding the documented handle and output contracts.
        unsafe {
            buffer_output(out)?;
            let doc = handle(doc)?;
            let items = match list {
                XGWX_LIST_MODULES => serde_json::to_value(doc.modules()),
                XGWX_LIST_NETWORKS => serde_json::to_value(doc.networks()),
                XGWX_LIST_PROGRAMS => serde_json::to_value(doc.programs()),
                XGWX_LIST_VARIABLES => serde_json::to_value(doc.variables().map_err(operation)?),
                XGWX_LIST_NETWORK_MODULES => serde_json::to_value(doc.network_modules()),
                XGWX_LIST_TEXT_PROGRAMS => Ok(serde_json::Value::Array(
                    doc.text_programs()
                        .into_iter()
                        .map(|p| {
                            serde_json::json!({
                                "program_index": p.program_index, "object_id": p.object_id,
                                "language": p.language, "source": p.source, "editable": p.editable,
                                "reason": p.reason, "variables_error": p.variables_error,
                                "variables": p.variables.into_iter().map(|v| serde_json::json!({
                                    "name": v.name, "data_type": v.data_type,
                                    "description": v.description, "system": v.system,
                                })).collect::<Vec<_>>(),
                            })
                        })
                        .collect(),
                )),
                _ => return Err(invalid("unknown list selector")),
            }
            .map_err(operation)?;
            write_buffer(
                out,
                json_bytes(serde_json::json!({"schema_version": 1, "items": items}))?,
            );
        }
        Ok(())
    })
}

/// Load selectable XGK modules and their verified/read-only options. Requires `write`.
///
/// # Safety
/// `out` must point to a writable zero-initialized buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_module_catalog_json(out: *mut XgwxBuffer) -> i32 {
    call(|| {
        // SAFETY: Forwarding the documented output contract.
        unsafe {
            buffer_output(out)?;
            #[cfg(feature = "write")]
            {
                let items = crate::xgk_module_catalog().iter().map(|m| serde_json::json!({
                    "model": m.model, "category": m.category, "id": m.id,
                    "sub_type": m.sub_type, "name": m.name, "slot_span": m.slot_span,
                    "status": m.status,
                    "options": m.options.iter().map(|o| serde_json::json!({
                        "key": o.key, "label": o.label, "section": o.section,
                        "scope": o.scope, "count": o.count,
                        "values": o.values.iter().map(|v| serde_json::json!({"label": v.label, "value": v.value})).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                    "visible_options": m.visible_options.iter().map(|o| serde_json::json!({
                        "key": o.key, "label": o.label, "section": o.section, "scope": o.scope,
                        "items_per_parent": o.items_per_parent, "count": o.count,
                        "default_value": o.default_value, "choices": o.choices,
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>();
                write_buffer(
                    out,
                    json_bytes(serde_json::json!({"schema_version": 1, "items": items}))?,
                );
            }
            #[cfg(not(feature = "write"))]
            return Err(Failure(
                XGWX_FEATURE_UNAVAILABLE,
                "enable the write feature".into(),
            ));
        }
        #[allow(unreachable_code)]
        Ok(())
    })
}

/// Read the current verified options of one installed module. Requires `write`.
///
/// # Safety
/// `doc` must be a live handle and `out` a writable zero-initialized buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_module_options_json(
    doc: *const XgwxHandle,
    base: u32,
    slot: u32,
    out: *mut XgwxBuffer,
) -> i32 {
    call(|| {
        // SAFETY: Forwarding the documented handle and output contracts.
        unsafe {
            buffer_output(out)?;
            let doc = handle(doc)?;
            #[cfg(feature = "write")]
            {
                let items = doc
                    .module_option_values(base, slot)
                    .map_err(operation)?
                    .into_iter()
                    .map(|v| serde_json::json!({"key": v.key, "index": v.index, "value": v.value}))
                    .collect::<Vec<_>>();
                write_buffer(
                    out,
                    json_bytes(serde_json::json!({"schema_version": 1, "items": items}))?,
                );
            }
            #[cfg(not(feature = "write"))]
            {
                let _ = (doc, base, slot);
                return Err(Failure(
                    XGWX_FEATURE_UNAVAILABLE,
                    "enable the write feature".into(),
                ));
            }
        }
        #[allow(unreachable_code)]
        Ok(())
    })
}

/// Apply a schema-versioned JSON edit batch atomically. Requires `write`.
/// The strict snake_case request format is documented in `docs/c-api.md`.
/// Stale selection guards and final verified serialization must all succeed
/// before the handle changes. The call does not write any files.
///
/// # Safety
/// `doc` must be exclusively accessible and `json` readable for `len` UTF-8 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_apply_edits_json(
    doc: *mut XgwxHandle,
    json: *const u8,
    len: usize,
) -> i32 {
    call(|| {
        // SAFETY: Forwarding the documented handle and text contracts.
        unsafe {
            let doc = handle_mut(doc)?;
            let request = text(json, len)?;
            #[cfg(feature = "write")]
            edits::apply(doc, request)?;
            #[cfg(not(feature = "write"))]
            {
                let _ = (doc, request);
                return Err(Failure(
                    XGWX_FEATURE_UNAVAILABLE,
                    "enable the write feature".into(),
                ));
            }
        }
        #[allow(unreachable_code)]
        Ok(())
    })
}

/// Return a verified workspace container. Requires `write`.
///
/// # Safety
/// `doc` must be a live handle and `out` a writable zero-initialized buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_serialize(
    doc: *const XgwxHandle,
    out: *mut XgwxBuffer,
) -> i32 {
    call(|| {
        // SAFETY: Forwarding the documented handle and output contracts.
        unsafe {
            buffer_output(out)?;
            let doc = handle(doc)?;
            #[cfg(feature = "write")]
            write_buffer(out, doc.to_verified_bytes().map_err(operation)?);
            #[cfg(not(feature = "write"))]
            {
                let _ = doc;
                return Err(Failure(
                    XGWX_FEATURE_UNAVAILABLE,
                    "enable the write feature".into(),
                ));
            }
        }
        #[allow(unreachable_code)]
        Ok(())
    })
}

/// Create a blank native project for the selected CPU and language. Requires `write`.
///
/// # Safety
/// Both text inputs must be readable UTF-8 byte spans. `out` must point to a
/// writable handle slot initialized to NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_project_create(
    cpu: *const u8,
    cpu_len: usize,
    language: *const u8,
    language_len: usize,
    out: *mut *mut XgwxHandle,
) -> i32 {
    call(|| {
        // SAFETY: Forwarding the documented text and output contracts.
        unsafe {
            document_output(out)?;
            let cpu = text(cpu, cpu_len)?;
            let language = text(language, language_len)?;
            #[cfg(feature = "write")]
            {
                let doc = crate::create_project(cpu, language).map_err(operation)?;
                *out = Box::into_raw(Box::new(doc)).cast();
            }
            #[cfg(not(feature = "write"))]
            {
                let _ = (cpu, language);
                return Err(Failure(
                    XGWX_FEATURE_UNAVAILABLE,
                    "enable the write feature".into(),
                ));
            }
        }
        #[allow(unreachable_code)]
        Ok(())
    })
}

/// Rename a project only if its current name equals `expected`. Requires `write`.
///
/// # Safety
/// `doc` must be exclusively accessible. Both text inputs must be readable UTF-8 spans.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_rename_project(
    doc: *mut XgwxHandle,
    expected: *const u8,
    expected_len: usize,
    name: *const u8,
    name_len: usize,
) -> i32 {
    call(|| {
        // SAFETY: Forwarding the documented handle and text contracts.
        unsafe {
            let doc = handle_mut(doc)?;
            let expected = text(expected, expected_len)?;
            let name = text(name, name_len)?;
            #[cfg(feature = "write")]
            {
                let mut candidate = doc.clone();
                candidate
                    .rename_project(expected, name)
                    .map_err(operation)?;
                *doc = candidate;
            }
            #[cfg(not(feature = "write"))]
            {
                let _ = (doc, expected, name);
                return Err(Failure(
                    XGWX_FEATURE_UNAVAILABLE,
                    "enable the write feature".into(),
                ));
            }
        }
        #[allow(unreachable_code)]
        Ok(())
    })
}

/// Perform the library's guarded CPU selection. Requires `write`.
///
/// # Safety
/// `doc` must be exclusively accessible; `cpu` must be a readable UTF-8 span.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_select_cpu(
    doc: *mut XgwxHandle,
    cpu: *const u8,
    cpu_len: usize,
) -> i32 {
    call(|| {
        // SAFETY: Forwarding the documented handle and text contracts.
        unsafe {
            let doc = handle_mut(doc)?;
            let cpu = text(cpu, cpu_len)?;
            #[cfg(feature = "write")]
            {
                let mut candidate = doc.clone();
                candidate.select_cpu(cpu).map_err(operation)?;
                *doc = candidate;
            }
            #[cfg(not(feature = "write"))]
            {
                let _ = (doc, cpu);
                return Err(Failure(
                    XGWX_FEATURE_UNAVAILABLE,
                    "enable the write feature".into(),
                ));
            }
        }
        #[allow(unreachable_code)]
        Ok(())
    })
}

/// Convert one ladder program to UTF-8 IL text. Requires `il`.
///
/// # Safety
/// `doc` must be a live handle and `out` a writable zero-initialized buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_program_il(
    doc: *const XgwxHandle,
    program_index: usize,
    out: *mut XgwxBuffer,
) -> i32 {
    call(|| {
        // SAFETY: Forwarding the documented handle and output contracts.
        unsafe {
            buffer_output(out)?;
            let doc = handle(doc)?;
            #[cfg(feature = "il")]
            {
                let program = doc
                    .ladder_program(program_index)
                    .ok_or_else(|| invalid("program index is out of range"))?
                    .map_err(operation)?;
                write_buffer(
                    out,
                    program.to_il().map_err(operation)?.to_string().into_bytes(),
                );
            }
            #[cfg(not(feature = "il"))]
            {
                let _ = (doc, program_index);
                return Err(Failure(
                    XGWX_FEATURE_UNAVAILABLE,
                    "enable the il feature".into(),
                ));
            }
        }
        #[allow(unreachable_code)]
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panics_are_contained_and_errors_are_thread_local() {
        assert_eq!(call(|| panic!("test panic")), XGWX_PANIC);
        let original = unsafe { std::ffi::CStr::from_ptr(xgwx_last_error()) }.to_owned();
        std::thread::spawn(|| {
            assert_eq!(call(|| Err(invalid("other thread"))), XGWX_INVALID_ARGUMENT);
        })
        .join()
        .unwrap();
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(xgwx_last_error()) },
            original.as_c_str()
        );
        assert_eq!(call(|| Ok(())), XGWX_OK);
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(xgwx_last_error()) }.to_bytes(),
            b""
        );
    }
}
