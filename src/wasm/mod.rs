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
        let decoded_payloads = doc.decoded_payloads();
        let mut decoded_payload_errors = 0;
        for error in decoded_payloads
            .iter()
            .filter_map(|payload| payload.as_ref().err())
        {
            decoded_payload_errors += 1;
            warnings.push(format!("payload: {error}"));
        }
        let ladder_programs = doc.ladder_programs();
        warnings.extend(
            ladder_programs
                .iter()
                .filter_map(|program| program.as_ref().err())
                .map(|error| format!("ladder: {error}")),
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
                variables: variable_summaries.as_ref().map(Vec::len),
                decoded_payloads: decoded_payloads.len(),
                decoded_payload_errors,
                ladder_programs: ladder_programs
                    .iter()
                    .filter(|program| program.is_ok())
                    .count(),
                ladder_errors: ladder_programs
                    .iter()
                    .filter(|program| program.is_err())
                    .count(),
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
            variables: variable_summaries
                .unwrap_or_default()
                .into_iter()
                .map(WasmVariableSummary::from_variable)
                .collect(),
            hardware: WasmHardwareSummary {
                bases: bases.into_iter().map(WasmBaseSummary::from_base).collect(),
                modules: modules
                    .into_iter()
                    .map(WasmModuleSummary::from_module)
                    .collect(),
            },
            ladder: ladder_programs
                .iter()
                .enumerate()
                .filter_map(|(index, program)| {
                    program
                        .as_ref()
                        .ok()
                        .map(|program| WasmLadderProgramSummary::from_program(index, program))
                })
                .collect(),
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
