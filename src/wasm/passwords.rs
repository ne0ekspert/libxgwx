use super::*;
use std::cell::RefCell;
thread_local! { static SESSIONS: RefCell<Vec<(Vec<u8>, XgwxDocument)>> = const { RefCell::new(Vec::new()) }; }

pub(super) fn parse_in_wasm_session(bytes: &[u8]) -> Result<XgwxDocument, XgwxError> {
    if let Some(doc) = SESSIONS.with(|sessions| {
        sessions
            .borrow()
            .iter()
            .rev()
            .find(|(known, _)| known.as_slice() == bytes)
            .map(|(_, doc)| doc.clone())
    }) {
        return Ok(doc);
    }
    let mut result = XgwxDocument::parse(bytes);
    SESSIONS.with(|sessions| {
        for (_, previous) in sessions.borrow().iter().rev() {
            if let Ok(candidate) = XgwxDocument::parse_in_session(bytes, previous)
                && result.as_ref().map_or(true, |doc| {
                    candidate.opened_programs.len() > doc.opened_programs.len()
                })
            {
                result = Ok(candidate);
            }
        }
    });
    result
}
fn remember(doc: &XgwxDocument, bytes: &[u8]) -> Result<(), XgwxError> {
    if !doc.project().password_protected().unwrap_or(false) && doc.opened_programs.is_empty() {
        return Ok(());
    }
    // Serializers preserve the original document header in memory. Cache the
    // exact output's header/gzip metadata, not its pre-serialization snapshot.
    let normalized = XgwxDocument::parse_in_session(bytes, doc)?;
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.borrow_mut();
        if sessions
            .last()
            .is_some_and(|(known, last)| known == bytes && last == doc)
        {
            return;
        }
        sessions.push((bytes.to_vec(), normalized));
        if sessions.len() > 16 {
            sessions.remove(0);
        }
    });
    Ok(())
}
#[cfg(feature = "write")]
pub(super) fn serialize_in_wasm_session(
    doc: &XgwxDocument,
    verified: bool,
) -> Result<Vec<u8>, XgwxError> {
    let bytes = if verified {
        doc.to_verified_bytes()?
    } else {
        doc.to_bytes()?
    };
    remember(doc, &bytes)?;
    Ok(bytes)
}
/// Forget authentication when closing or switching files. No passwords persist.
#[wasm_bindgen]
pub fn clear_xgwx_password_session() {
    SESSIONS.with(|s| s.borrow_mut().clear());
}

/// Authenticate a project (no index) or program without removing protection.
#[wasm_bindgen]
pub fn unlock_xgwx_password(
    bytes: &[u8],
    password: &str,
    program_index: Option<usize>,
) -> Result<JsValue, JsValue> {
    let mut doc = if program_index.is_some() {
        parse_in_wasm_session(bytes)
    } else {
        XgwxDocument::parse_with_password(bytes, password)
    }
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    if let Some(index) = program_index {
        doc.unlock_program(index, password)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
    }
    let json = serde_json::to_string(&WasmDocumentSummary::from_document(&doc))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let value = js_sys::JSON::parse(&json)?;
    remember(&doc, bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(value)
}
/// Set/change/remove protection. Null replacement removes; empty is invalid.
#[cfg(feature = "write")]
#[wasm_bindgen]
pub fn set_xgwx_password(
    bytes: &[u8],
    program_index: Option<usize>,
    current: Option<String>,
    replacement: Option<String>,
) -> Result<Vec<u8>, JsValue> {
    let mut doc = parse_in_wasm_session(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let result = if let Some(index) = program_index {
        doc.program_mut()
            .get(index)
            .and_then(|mut p| p.set_password(current.as_deref(), replacement.as_deref()))
    } else {
        doc.project_mut()
            .set_password(current.as_deref(), replacement.as_deref())
    };
    result.map_err(|e| JsValue::from_str(&e.to_string()))?;
    serialize_in_wasm_session(&doc, true).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmPasswordProtection {
    project_protected: bool,
    programs: Vec<WasmProgramProtection>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmProgramProtection {
    index: usize,
    protected: bool,
    locked: bool,
}
impl WasmPasswordProtection {
    pub(super) fn from_document(doc: &XgwxDocument) -> Self {
        Self {
            project_protected: doc.project().password_protected().unwrap_or(false),
            programs: (0..doc.program().programs().len())
                .map(|index| {
                    let p = doc.program().get(index).unwrap();
                    WasmProgramProtection {
                        index,
                        protected: p.password_protected(),
                        locked: p.is_locked(),
                    }
                })
                .collect(),
        }
    }
}
