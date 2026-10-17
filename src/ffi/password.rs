use super::*;

pub(super) fn failure(error: crate::XgwxError) -> Failure {
    let code = match &error {
        crate::XgwxError::Password(crate::PasswordError::Required(_)) => XGWX_PASSWORD_REQUIRED,
        crate::XgwxError::Password(crate::PasswordError::Incorrect(_)) => XGWX_PASSWORD_INCORRECT,
        crate::XgwxError::Password(_) => XGWX_OPERATION_ERROR,
        _ => XGWX_OPERATION_ERROR,
    };
    Failure(code, error.to_string())
}

pub(super) fn parse_failure(error: crate::XgwxError) -> Failure {
    match &error {
        crate::XgwxError::Password(_) => failure(error),
        _ => Failure(XGWX_PARSE_ERROR, error.to_string()),
    }
}
#[cfg(feature = "write")]
unsafe fn optional_text<'a>(data: *const u8, len: usize) -> Result<Option<&'a str>> {
    if data.is_null() {
        if len != 0 {
            return Err(invalid("NULL password must have zero length"));
        }
        Ok(None)
    } else {
        // SAFETY: Forward the foreign caller's readable UTF-8 span contract.
        unsafe { text(data, len).map(Some) }
    }
}

/// Parse with a project password; program blocks remain locked.
/// # Safety
/// Inputs must be readable UTF-8/byte spans; out must be a writable NULL slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_parse_with_password(
    data: *const u8,
    len: usize,
    password: *const u8,
    password_len: usize,
    out: *mut *mut XgwxHandle,
) -> i32 {
    call(|| unsafe {
        document_output(out)?;
        let doc =
            XgwxDocument::parse_with_password(input(data, len)?, text(password, password_len)?)
                .map_err(parse_failure)?;
        *out = Box::into_raw(Box::new(doc)).cast();
        Ok(())
    })
}

/// Authenticate a program without changing its stored password.
/// # Safety
/// doc must be a live exclusive handle and password a readable UTF-8 span.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_unlock_program(
    doc: *mut XgwxHandle,
    index: usize,
    password: *const u8,
    password_len: usize,
) -> i32 {
    call(|| unsafe {
        handle_mut(doc)?
            .unlock_program(index, text(password, password_len)?)
            .map_err(failure)
    })
}

/// Return password status only, without credential bytes.
/// # Safety
/// doc must be a live shared handle and out a writable empty buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_passwords_json(
    doc: *const XgwxHandle,
    out: *mut XgwxBuffer,
) -> i32 {
    call(|| unsafe {
        buffer_output(out)?;
        let doc = handle(doc)?;
        let programs: Vec<_> = doc.root.descendants_named("Program").enumerate().map(|(index, program)| {
            serde_json::json!({"index":index,"protected":!program.attribute("Encrytption").unwrap_or("").is_empty(),"locked":doc.program_element_is_locked(program)})
        }).collect();
        let value = serde_json::json!({"schema_version":1,"project_protected":doc.project().password_protected().map_err(failure)?,"programs":programs});
        write_buffer(out, json_bytes(value)?);
        Ok(())
    })
}

/// Set/change/remove a project password. NULL+0 means absent/removal.
/// # Safety
/// doc must be an exclusive live handle; non-NULL inputs are UTF-8 spans.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_set_project_password(
    doc: *mut XgwxHandle,
    current: *const u8,
    current_len: usize,
    replacement: *const u8,
    replacement_len: usize,
) -> i32 {
    call(|| {
        #[cfg(feature = "write")]
        unsafe {
            handle_mut(doc)?
                .project_mut()
                .set_password(
                    optional_text(current, current_len)?,
                    optional_text(replacement, replacement_len)?,
                )
                .map_err(failure)
        }
        #[cfg(not(feature = "write"))]
        {
            let _ = (doc, current, current_len, replacement, replacement_len);
            Err(Failure(
                XGWX_FEATURE_UNAVAILABLE,
                "write feature required".into(),
            ))
        }
    })
}

/// Set/change/remove a program password. Existing protection needs current.
/// # Safety
/// doc must be an exclusive live handle; non-NULL inputs are UTF-8 spans.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xgwx_document_set_program_password(
    doc: *mut XgwxHandle,
    index: usize,
    current: *const u8,
    current_len: usize,
    replacement: *const u8,
    replacement_len: usize,
) -> i32 {
    call(|| {
        #[cfg(feature = "write")]
        unsafe {
            handle_mut(doc)?
                .program_mut()
                .get(index)
                .map_err(operation)?
                .set_password(
                    optional_text(current, current_len)?,
                    optional_text(replacement, replacement_len)?,
                )
                .map_err(failure)
        }
        #[cfg(not(feature = "write"))]
        {
            let _ = (
                doc,
                index,
                current,
                current_len,
                replacement,
                replacement_len,
            );
            Err(Failure(
                XGWX_FEATURE_UNAVAILABLE,
                "write feature required".into(),
            ))
        }
    })
}
