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
    let configurations = doc.configurations();
    doc.task()
        .inspections()
        .into_iter()
        .map(|task| {
            let index = task.index;
            let configuration = task.configuration_index.and_then(|i| configurations.get(i));
            let (program_indices, membership_error) = match task.program_indices {
                Ok(programs) => (programs, None),
                Err(error) => (Vec::new(), Some(error)),
            };
            WasmTask {
                index,
                configuration_index: task.configuration_index,
                configuration_name: configuration.as_ref().and_then(|c| c.name.clone()),
                creation_supported: configuration
                    .is_some_and(|c| crate::task_api::task_creation_supported(c.type_code)),
                device_syntax: if configuration
                    .as_ref()
                    .and_then(|c| c.type_code)
                    .is_some_and(|t| t >= 100)
                {
                    "iec"
                } else {
                    "xgk"
                },
                definition: task.definition,
                expected: task.expected,
                settings: task.settings,
                program_indices,
                membership_error,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        time::{Duration, Instant},
    };

    fn document(count: usize) -> XgwxDocument {
        let tasks = (0..count)
            .map(|i| {
                format!("<Task Kind='2' Priority='3' TaskIndex='1' Device='100'>Task{i}</Task>")
            })
            .collect::<String>();
        let xml = format!(
            "<Project><Configuration Type='1' GUID='native-guid'><Tasks>{tasks}</Tasks></Configuration></Project>"
        );
        document_xml(&xml)
    }

    fn document_xml(xml: &str) -> XgwxDocument {
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gzip.write_all(xml.as_bytes()).unwrap();
        let mut bytes = b"XG".to_vec();
        bytes.extend(gzip.finish().unwrap());
        XgwxDocument::parse(&bytes).unwrap()
    }

    #[test]
    fn task_summary_fields_match_selected_task_contracts() {
        for doc in [
            XgwxDocument::parse(include_bytes!("../../fixtures/tasks/xgk-native.xgwx")).unwrap(),
            XgwxDocument::parse(include_bytes!("../../fixtures/tasks/xgi-native.xgwx")).unwrap(),
            document_xml(
                "<Project><Task>orphan</Task><Configuration Type='999' GUID='a'>Unknown<Tasks><Task>A</Task><Task>A</Task></Tasks></Configuration><Configuration Type=' 100 ' GUID='b'>IEC<Tasks><Task>B</Task></Tasks><Program Task='B'/></Configuration></Project>",
            ),
        ] {
            let configurations = doc.configurations();
            for row in summaries(&doc) {
                let task = doc.task().get(row.index).unwrap();
                let owner = task.configuration_index();
                let configuration = owner.and_then(|index| configurations.get(index));
                assert_eq!(row.configuration_index, owner);
                assert_eq!(
                    row.configuration_name,
                    configuration.and_then(|c| c.name.clone())
                );
                assert_eq!(
                    row.creation_supported,
                    owner.is_some_and(|index| doc.task().can_create(index))
                );
                assert_eq!(
                    row.device_syntax,
                    if configuration
                        .and_then(|c| c.type_code)
                        .is_some_and(|t| t >= 100)
                    {
                        "iec"
                    } else {
                        "xgk"
                    }
                );
                assert_eq!(row.definition, task.summary());
                assert_eq!(row.expected, task.snapshot().ok());
                assert_eq!(row.settings, task.settings().ok());
                match task.programs() {
                    Ok(programs) => {
                        assert_eq!(
                            row.program_indices,
                            programs.iter().map(|p| p.index()).collect::<Vec<_>>()
                        );
                        assert!(row.membership_error.is_none());
                    }
                    Err(error) => {
                        assert!(row.program_indices.is_empty());
                        assert_eq!(row.membership_error, Some(error.to_string()));
                    }
                }
            }
        }
    }

    #[test]
    fn task_summary_scaling_does_not_repeat_document_queries() {
        for count in [500, 1_000] {
            let doc = document(count);
            let start = Instant::now();
            let tasks = summaries(&doc);
            let elapsed = start.elapsed();
            eprintln!("{count} task summaries: {elapsed:?}");
            assert!(
                elapsed < Duration::from_secs(5),
                "summaries took {elapsed:?}"
            );
            assert_eq!(tasks.len(), count);
            assert!(
                tasks
                    .iter()
                    .enumerate()
                    .all(|(index, task)| task.index == index
                        && task.expected.is_some()
                        && task.settings.is_some()
                        && task.membership_error.is_none())
            );
        }
    }

    #[test]
    fn task_summary_large_browser_input_keeps_all_tasks() {
        let doc = document(20_000);
        let start = Instant::now();
        let summary = super::super::WasmDocumentSummary::from_document(&doc);
        let elapsed = start.elapsed();
        eprintln!("20000-task browser summary: {elapsed:?}");
        assert!(
            elapsed < Duration::from_secs(10),
            "summary took {elapsed:?}"
        );
        assert_eq!(summary.tasks.len(), 20_000);
        assert_eq!(summary.tasks[19_999].index, 19_999);
        assert_eq!(
            summary.tasks[19_999].definition.name.as_deref(),
            Some("Task19999")
        );
    }
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
