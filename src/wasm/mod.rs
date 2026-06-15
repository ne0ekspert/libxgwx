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

        // Avoid eager payload and ladder decoding in the browser summary path.
        // These operations can decode unbounded attacker-controlled data and are
        // not required for the lightweight metadata shown in the web demo.
        let decoded_payload_count = 0;
        let decoded_payload_errors = 0;
        let ladder_program_count = 0;
        let ladder_errors = 0;
        warnings.push(
            "payload and ladder decode skipped in browser summary to avoid unbounded decoding; zero counts here do not imply absence"
                .to_owned(),
        );
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
            ladder: Vec::new(),
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
            hsc,
            position: position_parameters
                .into_iter()
                .map(WasmPositionSummary::from_position)
                .collect(),
            pid: WasmPidSummary {
                cal_parameters: pid_cal.len(),
                tune_parameters: pid_tune.len(),
                cal_loops: pid_cal.iter().map(|parameter| parameter.loops.len()).sum(),
                tune_loops: pid_tune.iter().map(|parameter| parameter.loops.len()).sum(),
            },
            warnings,
        }
    }
}
