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
pub(super) struct WasmHardwareSummary {
    pub(super) bases: Vec<WasmBaseSummary>,
    pub(super) modules: Vec<WasmModuleSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmBaseSummary {
    pub(super) base: Option<u32>,
    pub(super) slot_count: Option<u32>,
}

impl WasmBaseSummary {
    pub(super) fn from_base(base: BaseSummary) -> Self {
        Self {
            base: base.base,
            slot_count: base.slot_count,
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
        }
    }
}
