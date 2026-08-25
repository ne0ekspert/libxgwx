use wasm_bindgen::prelude::*;

use crate::*;
use serde::Serialize;

mod ladder;
mod network;
mod parameters;
mod project;

use ladder::*;
use network::*;
use parameters::*;
use project::*;

const MAX_WASM_VARIABLES: usize = 65536;
const MAX_WASM_LADDER_PROGRAMS: usize = 64;
const MAX_WASM_LADDER_DECODED_BYTES: usize = 32 * 1024 * 1024;
const MAX_WASM_LADDER_ITEMS: usize = 100_000;

/// Parse `.xgwx` bytes and return a browser-friendly JavaScript summary.
#[wasm_bindgen]
pub fn parse_xgwx(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let doc = XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    let summary = WasmDocumentSummary::from_document(&doc);
    let json = serde_json::to_string(&summary).map_err(|error| {
        JsValue::from_str(&format!("failed to serialize xgwx summary: {error}"))
    })?;
    js_sys::JSON::parse(&json)
}

/// Return category and description metadata for known ladder mnemonics.
#[wasm_bindgen(js_name = known_ladder_mnemonics)]
pub fn known_ladder_mnemonics_wasm() -> Result<JsValue, JsValue> {
    let mnemonics = crate::known_ladder_mnemonics()
        .iter()
        .copied()
        .map(WasmLadderMnemonicSummary::from_info)
        .collect::<Vec<_>>();
    let json = serde_json::to_string(&mnemonics).map_err(|error| {
        JsValue::from_str(&format!(
            "failed to serialize ladder mnemonic metadata: {error}"
        ))
    })?;
    js_sys::JSON::parse(&json)
}

/// Apply supported module attribute changes and return rewritten `.xgwx` bytes.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_module)]
pub fn update_xgwx_module_wasm(
    bytes: &[u8],
    base: u32,
    slot: u32,
    patch: JsValue,
) -> Result<Vec<u8>, JsValue> {
    let json = js_sys::JSON::stringify(&patch)?
        .as_string()
        .ok_or_else(|| JsValue::from_str("module patch is not JSON-serializable"))?;
    let patch = serde_json::from_str::<ModulePatch>(&json)
        .map_err(|error| JsValue::from_str(&format!("invalid module patch: {error}")))?;
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_module(base, slot, &patch)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Update an XGI-D24A/B input filter and return rewritten `.xgwx` bytes.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = set_xgwx_module_input_filter)]
pub fn set_xgwx_module_input_filter_wasm(
    bytes: &[u8],
    base: u32,
    slot: u32,
    raw_filter: u8,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.set_module_input_filter(base, slot, ModuleInputFilter::from_raw(raw_filter))
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmLadderMnemonicSummary {
    mnemonic: &'static str,
    category: &'static str,
    description: &'static str,
}

impl WasmLadderMnemonicSummary {
    fn from_info(info: LadderMnemonicInfo) -> Self {
        Self {
            mnemonic: info.mnemonic,
            category: info.category.label(),
            description: info.description,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmDocumentSummary {
    header: WasmHeaderSummary,
    project: WasmProjectSummary,
    counts: WasmCounts,
    programs: Vec<WasmProgramSummary>,
    variables: Vec<WasmVariableSummary>,
    hardware: WasmHardwareSummary,
    ladder: Vec<WasmLadderProgramSummary>,
    networks: Vec<WasmNetworkSummary>,
    cnet: Vec<WasmCnetSummary>,
    fenet: Vec<WasmFenetSummary>,
    parameters: Vec<WasmParameterSummary>,
    safety_comm: Option<WasmSafetyCommSummary>,
    hsc: Vec<WasmHscSummary>,
    position: Vec<WasmPositionSummary>,
    pid: WasmPidSummary,
    warnings: Vec<String>,
}

impl WasmDocumentSummary {
    fn from_document(doc: &XgwxDocument) -> Self {
        let mut warnings = Vec::new();
        let project = doc.project_info();
        let configurations = doc.configurations();
        let networks = doc.networks();
        let bases = doc.bases();
        let modules = doc.modules();
        let programs = doc.programs();
        let parameters = doc.parameters();
        let safety_comm = doc.safety_comm();
        let position_parameters = doc.position_parameters();
        let variable_summaries = match doc.variables() {
            Ok(variables) => Some(variables),
            Err(error) => {
                warnings.push(format!("variables: {error}"));
                None
            }
        };
        let variable_count = variable_summaries.as_ref().map(Vec::len);
        let variables_for_output = variable_summaries
            .map(|variables| {
                variables
                    .into_iter()
                    .take(MAX_WASM_VARIABLES)
                    .map(WasmVariableSummary::from_variable)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if let Some(total) = variable_count
            && total > MAX_WASM_VARIABLES
        {
            warnings.push(format!(
                "variables: truncated to {MAX_WASM_VARIABLES} entries from {total}"
            ));
        }

        // The web UI does not display the general payload inventory. Ladder
        // programs are decoded separately below under browser-specific budgets.
        let decoded_payload_count = 0;
        let decoded_payload_errors = 0;
        warnings.push(
            "payload decode skipped in browser summary; zero counts here do not imply absence"
                .to_owned(),
        );
        let (ladder, ladder_errors) = decode_browser_ladder(doc, programs.len(), &mut warnings);
        let ladder_program_count = ladder.len();
        let hsc = doc
            .hsc_parameters()
            .into_iter()
            .filter_map(|result| match result {
                Ok(parameter) => Some(WasmHscSummary::from_parameter(parameter)),
                Err(error) => {
                    warnings.push(format!("hsc: {error}"));
                    None
                }
            })
            .collect::<Vec<_>>();
        let pid_cal = doc.pid_cal_parameters();
        let pid_tune = doc.pid_tune_parameters();
        let cnet_configs = doc.cnet_config_infos();
        let fenet_configs = doc.fenet_config_infos();

        Self {
            header: WasmHeaderSummary::from_header(&doc.header, doc.trailer.len()),
            project: WasmProjectSummary {
                name: project.name,
                file_version: project.file_version,
                comment: project.comment,
                guid: project.guid,
                file_last_write_time: project.file_last_write_time,
            },
            counts: WasmCounts {
                configurations: configurations.len(),
                networks: networks.len(),
                modules: modules.len(),
                programs: programs.len(),
                variables: variable_count,
                decoded_payloads: decoded_payload_count,
                decoded_payload_errors,
                ladder_programs: ladder_program_count,
                ladder_errors,
                cnet_modules: cnet_configs.len(),
                fenet_modules: fenet_configs.len(),
                hsc_parameters: hsc.len(),
                position_parameters: position_parameters.len(),
                pid_cal_parameters: pid_cal.len(),
                pid_tune_parameters: pid_tune.len(),
            },
            programs: programs
                .into_iter()
                .map(WasmProgramSummary::from_program)
                .collect(),
            variables: variables_for_output,
            hardware: WasmHardwareSummary {
                bases: bases.into_iter().map(WasmBaseSummary::from_base).collect(),
                modules: modules
                    .into_iter()
                    .map(WasmModuleSummary::from_module)
                    .collect(),
            },
            ladder,
            networks: networks
                .into_iter()
                .map(WasmNetworkSummary::from_network)
                .collect(),
            cnet: cnet_configs
                .iter()
                .map(WasmCnetSummary::from_cnet)
                .collect(),
            fenet: fenet_configs
                .iter()
                .map(WasmFenetSummary::from_fenet)
                .collect(),
            parameters: parameters
                .into_iter()
                .map(WasmParameterSummary::from_parameter)
                .collect(),
            safety_comm: safety_comm.map(WasmSafetyCommSummary::from_safety),
            hsc,
            position: position_parameters
                .into_iter()
                .map(WasmPositionSummary::from_position)
                .collect(),
            pid: WasmPidSummary::from_parameters(pid_cal, pid_tune),
            warnings,
        }
    }
}

fn decode_browser_ladder(
    doc: &XgwxDocument,
    program_count: usize,
    warnings: &mut Vec<String>,
) -> (Vec<WasmLadderProgramSummary>, usize) {
    let mut ladder = Vec::new();
    let mut errors = 0;
    let mut decoded_bytes = 0usize;
    let mut item_count = 0usize;

    for (program_index, element) in doc
        .root
        .descendants_named("Program")
        .take(MAX_WASM_LADDER_PROGRAMS)
        .enumerate()
    {
        let program = match LadderProgramData::from_program_element(element) {
            Ok(program) => program,
            Err(error) => {
                errors += 1;
                warnings.push(format!("ladder program {program_index}: {error}"));
                continue;
            }
        };
        let program_items = WasmLadderProgramSummary::source_item_count(&program);

        if decoded_bytes.saturating_add(program.decoded_len) > MAX_WASM_LADDER_DECODED_BYTES {
            warnings.push(format!(
                "ladder program {program_index}: omitted after reaching the {MAX_WASM_LADDER_DECODED_BYTES}-byte browser decode budget"
            ));
            break;
        }
        if item_count.saturating_add(program_items) > MAX_WASM_LADDER_ITEMS {
            warnings.push(format!(
                "ladder program {program_index}: omitted after reaching the {MAX_WASM_LADDER_ITEMS}-item browser summary budget"
            ));
            break;
        }

        decoded_bytes += program.decoded_len;
        item_count += program_items;
        ladder.push(WasmLadderProgramSummary::from_program(
            program_index,
            &program,
        ));
    }

    if program_count > MAX_WASM_LADDER_PROGRAMS {
        warnings.push(format!(
            "ladder: limited to the first {MAX_WASM_LADDER_PROGRAMS} of {program_count} programs"
        ));
    }

    (ladder, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_summary_preserves_parameter_details() {
        let doc =
            XgwxDocument::from_path("fixtures/XGB_Enet02.xgwx").expect("fixture should parse");
        let parameter_count = doc.parameters().len();
        let summary = WasmDocumentSummary::from_document(&doc);

        assert_eq!(summary.parameters.len(), parameter_count);
        let basic = summary
            .parameters
            .iter()
            .find(|parameter| parameter.parameter_type.as_deref() == Some("BASIC PARAMETER"))
            .expect("basic parameter should be present");
        assert!(
            basic
                .sections
                .iter()
                .any(|section| !section.attributes.is_empty())
        );

        assert!(
            summary
                .hsc
                .iter()
                .any(|parameter| !parameter.channels.is_empty())
        );
        assert!(
            summary
                .position
                .iter()
                .any(|parameter| { parameter.axes.iter().any(|axis| axis.parameter.is_some()) })
        );
        assert!(!summary.pid.calculation.is_empty());
        assert!(!summary.pid.tuning.is_empty());

        let json = serde_json::to_value(&summary).expect("summary should serialize");
        assert!(json.pointer("/parameters/0/sections").is_some());
        assert!(json.pointer("/position/0/axes/0/parameter").is_some());
        assert!(json.pointer("/pid/calculation/0/loops").is_some());
        assert!(json.pointer("/pid/tuning/0/loops").is_some());
        assert!(json.pointer("/cnet/0/ports/0/rxTimeout").is_some());
        assert!(json.pointer("/fenet/0/ipAddress2").is_some());

        let safety_doc =
            XgwxDocument::from_path("fixtures/elements.xgwx").expect("fixture should parse");
        let safety_summary = WasmDocumentSummary::from_document(&safety_doc);
        assert!(safety_summary.safety_comm.is_some());
        let safety_json = serde_json::to_value(&safety_summary).expect("summary should serialize");
        assert!(safety_json.pointer("/safetyComm/channels").is_some());
    }

    #[test]
    fn browser_summary_includes_drawable_ladder_data() {
        let doc =
            XgwxDocument::from_path("fixtures/elements.xgwx").expect("ladder fixture should parse");
        let summary = WasmDocumentSummary::from_document(&doc);

        assert_eq!(summary.counts.ladder_programs, 1);
        assert_eq!(summary.counts.ladder_errors, 0);
        assert_eq!(summary.ladder.len(), 1);
        assert!(!summary.ladder[0].rungs.is_empty());
        assert!(!summary.ladder[0].cells.is_empty());

        let json = serde_json::to_value(&summary).expect("summary should serialize");
        assert!(json.pointer("/ladder/0/cells/0/rawX").is_some());
        assert!(json.pointer("/ladder/0/cells/0/rawY").is_some());
    }
}
