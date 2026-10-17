//! Task inspection and guarded independent WASM mutations.
use super::*;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmTask {
    index: usize,
    configuration_index: Option<usize>,
    configuration_name: Option<String>,
    creation_supported: bool,
    device_syntax: &'static str,
    definition: TaskSummary,
    expected: Option<TaskSnapshot>,
    settings: Option<TaskSettings>,
    program_indices: Vec<usize>,
    membership_error: Option<String>,
}
pub(super) fn summaries(doc: &XgwxDocument) -> Vec<WasmTask> {
    doc.task()
        .list()
        .into_iter()
        .enumerate()
        .map(|(index, definition)| {
            let task = doc.task().get(index).unwrap();
            let configuration = task
                .configuration_index()
                .and_then(|i| doc.configurations().get(i).cloned());
            let (program_indices, membership_error) = match task.programs() {
                Ok(programs) => (programs.iter().map(|p| p.index()).collect(), None),
                Err(e) => (Vec::new(), Some(e.to_string())),
            };
            WasmTask {
                index,
                configuration_index: task.configuration_index(),
                configuration_name: configuration.as_ref().and_then(|c| c.name.clone()),
                creation_supported: task
                    .configuration_index()
                    .is_some_and(|i| doc.task().can_create(i)),
                device_syntax: if configuration
                    .as_ref()
                    .and_then(|c| c.type_code)
                    .is_some_and(|t| t >= 100)
                {
                    "iec"
                } else {
                    "xgk"
                },
                definition,
                expected: task.snapshot().ok(),
                settings: task.settings().ok(),
                program_indices,
                membership_error,
            }
        })
        .collect()
}
#[cfg(feature = "write")]
fn decode<T: serde::de::DeserializeOwned>(value: &JsValue) -> Result<T, JsValue> {
    let json = js_sys::JSON::stringify(value)?
        .as_string()
        .ok_or_else(|| JsValue::from_str("task request is not JSON-serializable"))?;
    serde_json::from_str(&json).map_err(|e| JsValue::from_str(&e.to_string()))
}
#[cfg(feature = "write")]
fn edit(
    bytes: &[u8],
    change: impl FnOnce(&mut XgwxDocument) -> Result<(), XgwxError>,
) -> Result<Vec<u8>, JsValue> {
    let mut doc = parse_in_wasm_session(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    change(&mut doc).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serialize_in_wasm_session(&doc, true).map_err(|e| JsValue::from_str(&e.to_string()))
}
/// Create a task using native scheduling settings in the selected PLC.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = create_xgwx_task)]
pub fn create_xgwx_task(
    bytes: &[u8],
    configuration_index: usize,
    settings: JsValue,
) -> Result<Vec<u8>, JsValue> {
    let settings: TaskSettings = decode(&settings)?;
    edit(bytes, |doc| {
        doc.task_mut().create(configuration_index, &settings)
    })
}
/// Update a task only if its complete inspected definition is unchanged.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_task)]
pub fn update_xgwx_task(
    bytes: &[u8],
    task_index: usize,
    expected: JsValue,
    settings: JsValue,
) -> Result<Vec<u8>, JsValue> {
    let expected: TaskSnapshot = decode(&expected)?;
    let settings: TaskSettings = decode(&settings)?;
    edit(bytes, |doc| {
        doc.task_mut()
            .get_if(task_index, &expected)?
            .update(&settings)
    })
}
/// Rename a user task and its program references atomically.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = rename_xgwx_task)]
pub fn rename_xgwx_task(
    bytes: &[u8],
    task_index: usize,
    expected: JsValue,
    name: &str,
) -> Result<Vec<u8>, JsValue> {
    let expected: TaskSnapshot = decode(&expected)?;
    edit(bytes, |doc| {
        doc.task_mut().get_if(task_index, &expected)?.rename(name)
    })
}
/// Delete an empty user task, preserving its programs by rejecting populated tasks.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_task)]
pub fn delete_xgwx_task(
    bytes: &[u8],
    task_index: usize,
    expected: JsValue,
) -> Result<Vec<u8>, JsValue> {
    let expected: TaskSnapshot = decode(&expected)?;
    edit(bytes, |doc| {
        doc.task_mut().get_if(task_index, &expected)?.delete()
    })
}
/// Assign an identified program to a task in the same PLC.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = assign_xgwx_task_program)]
pub fn assign_xgwx_task_program(
    bytes: &[u8],
    task_index: usize,
    expected: JsValue,
    program_index: usize,
    expected_object_id: &str,
    expected_task: &str,
) -> Result<Vec<u8>, JsValue> {
    let expected: TaskSnapshot = decode(&expected)?;
    edit(bytes, |doc| {
        doc.task_mut()
            .get_if(task_index, &expected)?
            .assign_program(program_index, expected_object_id, expected_task)
    })
}
/// Create a native empty program directly in the selected task.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = create_xgwx_task_program)]
pub fn create_xgwx_task_program(
    bytes: &[u8],
    task_index: usize,
    expected: JsValue,
    program: JsValue,
) -> Result<Vec<u8>, JsValue> {
    let expected: TaskSnapshot = decode(&expected)?;
    let program: NewProgram = decode(&program)?;
    edit(bytes, |doc| {
        doc.task_mut()
            .get_if(task_index, &expected)?
            .create_program(&program)
    })
}
