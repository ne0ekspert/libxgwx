use crate::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmHeaderSummary {
    pub(super) label: Option<String>,
    pub(super) gzip_offset: usize,
    pub(super) header_bytes: usize,
    pub(super) trailer_bytes: usize,
    pub(super) compressed_size_hint: Option<u32>,
}

impl WasmHeaderSummary {
    pub(super) fn from_header(header: &XgwxHeader, trailer_bytes: usize) -> Self {
        Self {
            label: header.label.clone(),
            gzip_offset: header.gzip_offset,
            header_bytes: header.raw.len(),
            trailer_bytes,
            compressed_size_hint: header.compressed_size_hint,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmProjectSummary {
    pub(super) name: Option<String>,
    pub(super) file_version: Option<String>,
    pub(super) comment: Option<String>,
    pub(super) guid: Option<String>,
    pub(super) file_last_write_time: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmCounts {
    pub(super) configurations: usize,
    pub(super) networks: usize,
    pub(super) modules: usize,
    pub(super) programs: usize,
    pub(super) variables: Option<usize>,
    pub(super) decoded_payloads: usize,
    pub(super) decoded_payload_errors: usize,
    pub(super) ladder_programs: usize,
    pub(super) ladder_errors: usize,
    pub(super) cnet_modules: usize,
    pub(super) fenet_modules: usize,
    pub(super) hsc_parameters: usize,
    pub(super) position_parameters: usize,
    pub(super) pid_cal_parameters: usize,
    pub(super) pid_tune_parameters: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmProgramSummary {
    pub(super) configuration_index: Option<usize>,
    pub(super) name: Option<String>,
    pub(super) task: Option<String>,
    pub(super) kind: Option<u32>,
    pub(super) version: Option<u32>,
    pub(super) object_id: Option<String>,
    pub(super) comment: Option<String>,
}

impl WasmProgramSummary {
    pub(super) fn from_program(program: ProgramSummary) -> Self {
        Self {
            configuration_index: None,
            name: program.name,
            task: program.task,
            kind: program.kind,
            version: program.version,
            object_id: program.object_id,
            comment: program.comment,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmVariableSummary {
    pub(super) name: Option<String>,
    pub(super) address: Option<String>,
    pub(super) address_area: Option<String>,
    pub(super) address_number: Option<u32>,
    pub(super) data_type: Option<String>,
    pub(super) scope: Option<String>,
    pub(super) description: Option<String>,
    pub(super) comment: Option<String>,
    pub(super) source_ref: Option<String>,
    pub(super) range: Option<String>,
    pub(super) format_version: Option<String>,
}

impl WasmVariableSummary {
    pub(super) fn from_variable(variable: VariableSummary) -> Self {
        Self {
            name: variable.name,
            address: variable.address,
            address_area: variable.address_area,
            address_number: variable.address_number,
            data_type: variable.data_type.or(variable.type_name),
            scope: variable.scope,
            description: variable.description,
            comment: variable.comment,
            source_ref: variable.source_ref,
            range: variable.range,
            format_version: variable.format_version,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecLocalSymbolSummary {
    pub(super) record_offset: usize,
    pub(super) name: String,
    pub(super) address: Option<String>,
    pub(super) type_reference: Option<String>,
    pub(super) data_type_code: u32,
    pub(super) data_type: Option<String>,
    pub(super) storage_class: String,
    pub(super) allocation_number: Option<u32>,
    pub(super) allocation_width: Option<u32>,
    pub(super) description: Option<String>,
    pub(super) is_instance: bool,
}

impl WasmIecLocalSymbolSummary {
    pub(super) fn from_symbol(symbol: IecLocalSymbol) -> Self {
        Self {
            record_offset: symbol.record_offset,
            name: symbol.name,
            address: symbol.address,
            type_reference: symbol.type_reference,
            data_type_code: symbol.data_type_code,
            data_type: symbol.data_type,
            storage_class: symbol.storage_class,
            allocation_number: symbol.allocation_number,
            allocation_width: symbol.allocation_width,
            description: symbol.description,
            is_instance: symbol.is_instance,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmHardwareSummary {
    pub(super) cpu_profile: Option<&'static CpuHardwareProfile>,
    pub(super) bases: Vec<WasmBaseSummary>,
    pub(super) modules: Vec<WasmModuleSummary>,
    pub(super) typed_modules: Vec<WasmTypedModule>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmBaseSummary {
    pub(super) base: Option<u32>,
    pub(super) slot_count: Option<u32>,
    pub(super) base_index: usize,
    pub(super) configuration_index: Option<usize>,
    pub(super) modules: Vec<WasmTypedModule>,
}

impl WasmBaseSummary {
    pub(super) fn from_base(base: crate::Base<'_>) -> Self {
        Self {
            base: base.number(),
            slot_count: base.slot_count(),
            base_index: base.index(),
            configuration_index: base.configuration_index(),
            modules: base
                .modules()
                .into_iter()
                .map(WasmTypedModule::from_module)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmModuleSummary {
    pub(super) base: Option<u32>,
    pub(super) slot: Option<u32>,
    pub(super) id: Option<u32>,
    pub(super) sub_type: Option<u32>,
    pub(super) name: Option<String>,
    pub(super) comment: Option<String>,
    pub(super) details: Option<String>,
    pub(super) input_filter_raw: Option<u8>,
    pub(super) input_filter: Option<String>,
}

impl WasmModuleSummary {
    pub(super) fn from_module(module: ModuleSummary) -> Self {
        Self {
            base: module.base,
            slot: module.slot,
            id: module.id,
            sub_type: module.sub_type,
            name: module.name,
            comment: module.comment,
            details: module.details,
            input_filter_raw: module.input_filter_raw,
            input_filter: module.input_filter.map(|value| value.to_string()),
        }
    }
}

// Keep typed membership separate from installed edit coordinates.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmTypedModule {
    kind: &'static str,
    io_kind: Option<String>,
    configuration_index: Option<usize>,
    hardware: Option<WasmModuleSummary>,
    network: Option<super::network::WasmNetworkModuleSummary>,
    configuration: Option<serde_json::Value>,
}
impl WasmTypedModule {
    pub(super) fn from_module(module: crate::Module) -> Self {
        match module {
            crate::Module::Io(io) => Self {
                kind: "io",
                io_kind: Some(format!("{:?}", io.kind)),
                configuration_index: io.base.configuration_index,
                hardware: Some(WasmModuleSummary::from_module(io.base.hardware)),
                network: None,
                configuration: None,
            },
            crate::Module::Other(hw) => Self {
                kind: "other",
                io_kind: None,
                configuration_index: hw.configuration_index,
                hardware: Some(WasmModuleSummary::from_module(hw.hardware)),
                network: None,
                configuration: None,
            },
            crate::Module::Network(net) => Self {
                kind: "network",
                io_kind: None,
                configuration_index: net.configuration_index,
                hardware: net
                    .base
                    .map(|hw| WasmModuleSummary::from_module(hw.hardware)),
                network: Some(super::network::WasmNetworkModuleSummary::from_module(
                    net.metadata,
                )),
                configuration: Some(match net.configuration {
                    crate::ConfigurationMatch::Missing => serde_json::json!({"status":"missing"}),
                    crate::ConfigurationMatch::Matched(config) => {
                        serde_json::json!({"status":"matched", "settings": network_settings(config)})
                    }
                    crate::ConfigurationMatch::Ambiguous(configs) => {
                        serde_json::json!({"status":"ambiguous", "candidates": configs.into_iter().map(network_settings).collect::<Vec<_>>()})
                    }
                }),
            },
        }
    }
}
fn network_settings(config: crate::NetworkConfiguration) -> serde_json::Value {
    use super::network::*;
    match config {
        crate::NetworkConfiguration::Cnet(c) => {
            serde_json::json!({"kind":"cnet", "value": WasmCnetSummary::from_cnet(&c)})
        }
        crate::NetworkConfiguration::Fenet(c) => {
            serde_json::json!({"kind":"fenet", "value": WasmFenetSummary::from_fenet(&c)})
        }
        crate::NetworkConfiguration::Other(c) => {
            serde_json::json!({"kind":"other", "value": WasmXgpdSummary::from_xgpd(c)})
        }
    }
}
