//! Strict snake_case C transport DTOs. Native WASM serialization is not used.
use serde::{Deserialize, Serialize};

pub(super) use super::inspection_types::*;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LadderEditKind {
    NormallyOpen,
    NormallyClosed,
    AddressedRisingPulse,
    AddressedFallingPulse,
    AddressedRisingPulseNot,
    AddressedFallingPulseNot,
    Inverse,
    RisingPulse,
    FallingPulse,
    Output,
    InverseOutput,
    Set,
    Reset,
    RisingPulseOutput,
    FallingPulseOutput,
}

impl From<LadderEditKind> for crate::LadderEditKind {
    fn from(value: LadderEditKind) -> Self {
        match value {
            LadderEditKind::NormallyOpen => crate::LadderEditKind::NormallyOpen,
            LadderEditKind::NormallyClosed => crate::LadderEditKind::NormallyClosed,
            LadderEditKind::AddressedRisingPulse => crate::LadderEditKind::AddressedRisingPulse,
            LadderEditKind::AddressedFallingPulse => crate::LadderEditKind::AddressedFallingPulse,
            LadderEditKind::AddressedRisingPulseNot => {
                crate::LadderEditKind::AddressedRisingPulseNot
            }
            LadderEditKind::AddressedFallingPulseNot => {
                crate::LadderEditKind::AddressedFallingPulseNot
            }
            LadderEditKind::Inverse => crate::LadderEditKind::Inverse,
            LadderEditKind::RisingPulse => crate::LadderEditKind::RisingPulse,
            LadderEditKind::FallingPulse => crate::LadderEditKind::FallingPulse,
            LadderEditKind::Output => crate::LadderEditKind::Output,
            LadderEditKind::InverseOutput => crate::LadderEditKind::InverseOutput,
            LadderEditKind::Set => crate::LadderEditKind::Set,
            LadderEditKind::Reset => crate::LadderEditKind::Reset,
            LadderEditKind::RisingPulseOutput => crate::LadderEditKind::RisingPulseOutput,
            LadderEditKind::FallingPulseOutput => crate::LadderEditKind::FallingPulseOutput,
        }
    }
}

impl From<crate::LadderEditKind> for LadderEditKind {
    fn from(value: crate::LadderEditKind) -> Self {
        match value {
            crate::LadderEditKind::NormallyOpen => LadderEditKind::NormallyOpen,
            crate::LadderEditKind::NormallyClosed => LadderEditKind::NormallyClosed,
            crate::LadderEditKind::AddressedRisingPulse => LadderEditKind::AddressedRisingPulse,
            crate::LadderEditKind::AddressedFallingPulse => LadderEditKind::AddressedFallingPulse,
            crate::LadderEditKind::AddressedRisingPulseNot => {
                LadderEditKind::AddressedRisingPulseNot
            }
            crate::LadderEditKind::AddressedFallingPulseNot => {
                LadderEditKind::AddressedFallingPulseNot
            }
            crate::LadderEditKind::Inverse => LadderEditKind::Inverse,
            crate::LadderEditKind::RisingPulse => LadderEditKind::RisingPulse,
            crate::LadderEditKind::FallingPulse => LadderEditKind::FallingPulse,
            crate::LadderEditKind::Output => LadderEditKind::Output,
            crate::LadderEditKind::InverseOutput => LadderEditKind::InverseOutput,
            crate::LadderEditKind::Set => LadderEditKind::Set,
            crate::LadderEditKind::Reset => LadderEditKind::Reset,
            crate::LadderEditKind::RisingPulseOutput => LadderEditKind::RisingPulseOutput,
            crate::LadderEditKind::FallingPulseOutput => LadderEditKind::FallingPulseOutput,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LadderCommentKind {
    Rung,
    Output,
}

impl From<LadderCommentKind> for crate::LadderCommentKind {
    fn from(value: LadderCommentKind) -> Self {
        match value {
            LadderCommentKind::Rung => crate::LadderCommentKind::Rung,
            LadderCommentKind::Output => crate::LadderCommentKind::Output,
        }
    }
}

impl From<crate::LadderCommentKind> for LadderCommentKind {
    fn from(value: crate::LadderCommentKind) -> Self {
        match value {
            crate::LadderCommentKind::Rung => LadderCommentKind::Rung,
            crate::LadderCommentKind::Output => LadderCommentKind::Output,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModulePatch {
    pub id: Option<u32>,
    pub sub_type: Option<u32>,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub details: Option<String>,
}

impl From<ModulePatch> for crate::ModulePatch {
    fn from(value: ModulePatch) -> Self {
        Self {
            id: value.id,
            sub_type: value.sub_type,
            name: value.name,
            comment: value.comment,
            details: value.details,
        }
    }
}

impl From<crate::ModulePatch> for ModulePatch {
    fn from(value: crate::ModulePatch) -> Self {
        Self {
            id: value.id,
            sub_type: value.sub_type,
            name: value.name,
            comment: value.comment,
            details: value.details,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProgramPatch {
    pub name: Option<String>,
    pub task: Option<String>,
    pub version: Option<u32>,
    pub kind: Option<u32>,
    pub instance_name: Option<String>,
    pub comment: Option<String>,
}

impl From<ProgramPatch> for crate::ProgramPatch {
    fn from(value: ProgramPatch) -> Self {
        Self {
            name: value.name,
            task: value.task,
            version: value.version,
            kind: value.kind,
            instance_name: value.instance_name,
            comment: value.comment,
        }
    }
}

impl From<crate::ProgramPatch> for ProgramPatch {
    fn from(value: crate::ProgramPatch) -> Self {
        Self {
            name: value.name,
            task: value.task,
            version: value.version,
            kind: value.kind,
            instance_name: value.instance_name,
            comment: value.comment,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VariablePatch {
    pub name: Option<String>,
    pub address_area: Option<String>,
    pub address_number: Option<u32>,
    pub data_type: Option<String>,
    pub description: Option<String>,
}

impl From<VariablePatch> for crate::VariablePatch {
    fn from(value: VariablePatch) -> Self {
        Self {
            name: value.name,
            address_area: value.address_area,
            address_number: value.address_number,
            data_type: value.data_type,
            description: value.description,
        }
    }
}

impl From<crate::VariablePatch> for VariablePatch {
    fn from(value: crate::VariablePatch) -> Self {
        Self {
            name: value.name,
            address_area: value.address_area,
            address_number: value.address_number,
            data_type: value.data_type,
            description: value.description,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NetworkPatch {
    pub name: Option<String>,
    pub type_name: Option<String>,
    pub network_type: Option<String>,
}

impl From<NetworkPatch> for crate::NetworkPatch {
    fn from(value: NetworkPatch) -> Self {
        Self {
            name: value.name,
            type_name: value.type_name,
            network_type: value.network_type,
        }
    }
}

impl From<crate::NetworkPatch> for NetworkPatch {
    fn from(value: crate::NetworkPatch) -> Self {
        Self {
            name: value.name,
            type_name: value.type_name,
            network_type: value.network_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NetworkModulePatch {
    pub config_name: Option<String>,
    pub alias: Option<String>,
    pub description: Option<String>,
}

impl From<NetworkModulePatch> for crate::NetworkModulePatch {
    fn from(value: NetworkModulePatch) -> Self {
        Self {
            config_name: value.config_name,
            alias: value.alias,
            description: value.description,
        }
    }
}

impl From<crate::NetworkModulePatch> for NetworkModulePatch {
    fn from(value: crate::NetworkModulePatch) -> Self {
        Self {
            config_name: value.config_name,
            alias: value.alias,
            description: value.description,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BrowserNetworkPatch {
    pub network_index: usize,
    pub module: bool,
    pub base: Option<u32>,
    pub slot: Option<u32>,
    pub field: String,
    pub expected_value: String,
    pub replacement: String,
}

impl From<BrowserNetworkPatch> for crate::BrowserNetworkPatch {
    fn from(value: BrowserNetworkPatch) -> Self {
        Self {
            network_index: value.network_index,
            module: value.module,
            base: value.base,
            slot: value.slot,
            field: value.field,
            expected_value: value.expected_value,
            replacement: value.replacement,
        }
    }
}

impl From<crate::BrowserNetworkPatch> for BrowserNetworkPatch {
    fn from(value: crate::BrowserNetworkPatch) -> Self {
        Self {
            network_index: value.network_index,
            module: value.module,
            base: value.base,
            slot: value.slot,
            field: value.field,
            expected_value: value.expected_value,
            replacement: value.replacement,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FenetFieldPatch {
    pub base: u32,
    pub slot: u32,
    pub field: String,
    pub expected_value: String,
    pub replacement: String,
}

impl From<FenetFieldPatch> for crate::FenetFieldPatch {
    fn from(value: FenetFieldPatch) -> Self {
        Self {
            base: value.base,
            slot: value.slot,
            field: value.field,
            expected_value: value.expected_value,
            replacement: value.replacement,
        }
    }
}

impl From<crate::FenetFieldPatch> for FenetFieldPatch {
    fn from(value: crate::FenetFieldPatch) -> Self {
        Self {
            base: value.base,
            slot: value.slot,
            field: value.field,
            expected_value: value.expected_value,
            replacement: value.replacement,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BrowserHardwarePatch {
    pub operation: String,
    pub base: u32,
    pub slot: Option<u32>,
    pub expected_value: String,
    pub replacement: String,
    pub key: Option<String>,
    pub index: Option<u32>,
}

impl From<BrowserHardwarePatch> for crate::BrowserHardwarePatch {
    fn from(value: BrowserHardwarePatch) -> Self {
        Self {
            operation: value.operation,
            base: value.base,
            slot: value.slot,
            expected_value: value.expected_value,
            replacement: value.replacement,
            key: value.key,
            index: value.index,
        }
    }
}

impl From<crate::BrowserHardwarePatch> for BrowserHardwarePatch {
    fn from(value: crate::BrowserHardwarePatch) -> Self {
        Self {
            operation: value.operation,
            base: value.base,
            slot: value.slot,
            expected_value: value.expected_value,
            replacement: value.replacement,
            key: value.key,
            index: value.index,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BrowserIecPatch {
    pub operation: String,
    pub offset: usize,
    pub expected_kind: String,
    pub expected_value: String,
    pub replacement: String,
}

impl From<BrowserIecPatch> for crate::BrowserIecPatch {
    fn from(value: BrowserIecPatch) -> Self {
        Self {
            operation: value.operation,
            offset: value.offset,
            expected_kind: value.expected_kind,
            expected_value: value.expected_value,
            replacement: value.replacement,
        }
    }
}

impl From<crate::BrowserIecPatch> for BrowserIecPatch {
    fn from(value: crate::BrowserIecPatch) -> Self {
        Self {
            operation: value.operation,
            offset: value.offset,
            expected_kind: value.expected_kind,
            expected_value: value.expected_value,
            replacement: value.replacement,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NewProgram {
    pub name: String,
    pub language: String,
    pub object_id: String,
    pub symbol_id: String,
}

impl From<NewProgram> for crate::NewProgram {
    fn from(value: NewProgram) -> Self {
        Self {
            name: value.name,
            language: value.language,
            object_id: value.object_id,
            symbol_id: value.symbol_id,
        }
    }
}

impl From<crate::NewProgram> for NewProgram {
    fn from(value: crate::NewProgram) -> Self {
        Self {
            name: value.name,
            language: value.language,
            object_id: value.object_id,
            symbol_id: value.symbol_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CnetFieldEdit {
    pub port_index: usize,
    pub field: String,
    pub expected_value: String,
    pub replacement: String,
}

impl From<CnetFieldEdit> for crate::CnetFieldEdit {
    fn from(value: CnetFieldEdit) -> Self {
        Self {
            port_index: value.port_index,
            field: value.field,
            expected_value: value.expected_value,
            replacement: value.replacement,
        }
    }
}

impl From<crate::CnetFieldEdit> for CnetFieldEdit {
    fn from(value: crate::CnetFieldEdit) -> Self {
        Self {
            port_index: value.port_index,
            field: value.field,
            expected_value: value.expected_value,
            replacement: value.replacement,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CnetSettingsPatch {
    pub base: u32,
    pub slot: u32,
    pub changes: Vec<CnetFieldEdit>,
}

impl From<CnetSettingsPatch> for crate::CnetSettingsPatch {
    fn from(value: CnetSettingsPatch) -> Self {
        Self {
            base: value.base,
            slot: value.slot,
            changes: value.changes.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::CnetSettingsPatch> for CnetSettingsPatch {
    fn from(value: crate::CnetSettingsPatch) -> Self {
        Self {
            base: value.base,
            slot: value.slot,
            changes: value.changes.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SfcEntityPatch {
    pub program_index: usize,
    pub block_index: usize,
    pub entity_index: usize,
    pub expected_type: u32,
    pub expected_row: u32,
    pub expected_column: u32,
    pub field: String,
    pub expected_value: String,
    pub replacement: String,
}

impl From<SfcEntityPatch> for crate::SfcEntityPatch {
    fn from(value: SfcEntityPatch) -> Self {
        Self {
            program_index: value.program_index,
            block_index: value.block_index,
            entity_index: value.entity_index,
            expected_type: value.expected_type,
            expected_row: value.expected_row,
            expected_column: value.expected_column,
            field: value.field,
            expected_value: value.expected_value,
            replacement: value.replacement,
        }
    }
}

impl From<crate::SfcEntityPatch> for SfcEntityPatch {
    fn from(value: crate::SfcEntityPatch) -> Self {
        Self {
            program_index: value.program_index,
            block_index: value.block_index,
            entity_index: value.entity_index,
            expected_type: value.expected_type,
            expected_row: value.expected_row,
            expected_column: value.expected_column,
            field: value.field,
            expected_value: value.expected_value,
            replacement: value.replacement,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SfcSequencePatch {
    pub program_index: usize,
    pub block_index: usize,
    pub expected_entities: Vec<SfcEntity>,
    pub rows: Vec<SfcRow>,
    #[serde(default)]
    pub expected_rows: Option<Vec<SfcRow>>,
}

impl From<SfcSequencePatch> for crate::SfcSequencePatch {
    fn from(value: SfcSequencePatch) -> Self {
        Self {
            program_index: value.program_index,
            block_index: value.block_index,
            expected_entities: value
                .expected_entities
                .into_iter()
                .map(Into::into)
                .collect(),
            rows: value.rows.into_iter().map(Into::into).collect(),
            expected_rows: value
                .expected_rows
                .map(|items| items.into_iter().map(Into::into).collect()),
        }
    }
}

impl From<crate::SfcSequencePatch> for SfcSequencePatch {
    fn from(value: crate::SfcSequencePatch) -> Self {
        Self {
            program_index: value.program_index,
            block_index: value.block_index,
            expected_entities: value
                .expected_entities
                .into_iter()
                .map(Into::into)
                .collect(),
            rows: value.rows.into_iter().map(Into::into).collect(),
            expected_rows: value
                .expected_rows
                .map(|items| items.into_iter().map(Into::into).collect()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SfcVariablePatch {
    pub program_index: usize,
    pub expected_variables: Vec<SfcVariable>,
    pub name: String,
    pub data_type: String,
    pub description: String,
    pub remove: bool,
    #[serde(default)]
    pub declaration: Option<SfcDeclaration>,
    #[serde(default)]
    pub update: bool,
}

impl From<SfcVariablePatch> for crate::SfcVariablePatch {
    fn from(value: SfcVariablePatch) -> Self {
        Self {
            program_index: value.program_index,
            expected_variables: value
                .expected_variables
                .into_iter()
                .map(Into::into)
                .collect(),
            name: value.name,
            data_type: value.data_type,
            description: value.description,
            remove: value.remove,
            declaration: value.declaration.map(Into::into),
            update: value.update,
        }
    }
}

impl From<crate::SfcVariablePatch> for SfcVariablePatch {
    fn from(value: crate::SfcVariablePatch) -> Self {
        Self {
            program_index: value.program_index,
            expected_variables: value
                .expected_variables
                .into_iter()
                .map(Into::into)
                .collect(),
            name: value.name,
            data_type: value.data_type,
            description: value.description,
            remove: value.remove,
            declaration: value.declaration.map(Into::into),
            update: value.update,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LadderCellEdit {
    pub raw_y: u32,
    pub column: u8,
    pub expected: Option<LadderEditElement>,
    pub replacement: Option<LadderEditElement>,
}

impl From<LadderCellEdit> for crate::LadderCellEdit {
    fn from(value: LadderCellEdit) -> Self {
        Self {
            raw_y: value.raw_y,
            column: value.column,
            expected: value.expected.map(Into::into),
            replacement: value.replacement.map(Into::into),
        }
    }
}

impl From<crate::LadderCellEdit> for LadderCellEdit {
    fn from(value: crate::LadderCellEdit) -> Self {
        Self {
            raw_y: value.raw_y,
            column: value.column,
            expected: value.expected.map(Into::into),
            replacement: value.replacement.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LadderCommentEdit {
    pub kind: LadderCommentKind,
    pub raw_y: u32,
    pub expected: Option<String>,
    pub replacement: String,
}

impl From<LadderCommentEdit> for crate::LadderCommentEdit {
    fn from(value: LadderCommentEdit) -> Self {
        Self {
            kind: value.kind.into(),
            raw_y: value.raw_y,
            expected: value.expected,
            replacement: value.replacement,
        }
    }
}

impl From<crate::LadderCommentEdit> for LadderCommentEdit {
    fn from(value: crate::LadderCommentEdit) -> Self {
        Self {
            kind: value.kind.into(),
            raw_y: value.raw_y,
            expected: value.expected,
            replacement: value.replacement,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LadderBranchEdit {
    pub raw_y: u32,
    pub boundary: u8,
    pub expected: bool,
    pub present: bool,
}

impl From<LadderBranchEdit> for crate::LadderBranchEdit {
    fn from(value: LadderBranchEdit) -> Self {
        Self {
            raw_y: value.raw_y,
            boundary: value.boundary,
            expected: value.expected,
            present: value.present,
        }
    }
}

impl From<crate::LadderBranchEdit> for LadderBranchEdit {
    fn from(value: crate::LadderBranchEdit) -> Self {
        Self {
            raw_y: value.raw_y,
            boundary: value.boundary,
            expected: value.expected,
            present: value.present,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LadderEditElement {
    pub kind: LadderEditKind,
    pub operand: String,
}

impl From<LadderEditElement> for crate::LadderEditElement {
    fn from(value: LadderEditElement) -> Self {
        Self {
            kind: value.kind.into(),
            operand: value.operand,
        }
    }
}

impl From<crate::LadderEditElement> for LadderEditElement {
    fn from(value: crate::LadderEditElement) -> Self {
        Self {
            kind: value.kind.into(),
            operand: value.operand,
        }
    }
}

/// Guarded standalone ST/IL source replacement; protocol fields remain snake_case.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TextProgramPatch {
    pub program_index: usize,
    pub expected_object_id: String,
    pub expected_language: String,
    pub expected_source: String,
    pub source: String,
}
impl From<TextProgramPatch> for crate::TextProgramPatch {
    fn from(value: TextProgramPatch) -> Self {
        Self {
            program_index: value.program_index,
            expected_object_id: value.expected_object_id,
            expected_language: value.expected_language,
            expected_source: value.expected_source,
            source: value.source,
        }
    }
}

#[cfg(feature = "il")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VendorIlPatch {
    pub program_index: usize,
    pub expected_object_id: String,
    pub expected_source: String,
    pub source: String,
}
#[cfg(feature = "il")]
impl From<VendorIlPatch> for crate::VendorIlPatch {
    fn from(value: VendorIlPatch) -> Self {
        Self {
            program_index: value.program_index,
            expected_object_id: value.expected_object_id,
            expected_source: value.expected_source,
            source: value.source,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OpcUaTag {
    pub name: String,
    pub data_type: String,
    pub address_area: String,
    pub address_number: u32,
    pub comment: String,
}
impl From<OpcUaTag> for crate::OpcUaTag {
    fn from(t: OpcUaTag) -> Self {
        Self {
            name: t.name,
            data_type: t.data_type,
            address_area: t.address_area,
            address_number: t.address_number,
            comment: t.comment,
        }
    }
}
impl From<crate::OpcUaTag> for OpcUaTag {
    fn from(t: crate::OpcUaTag) -> Self {
        Self {
            name: t.name,
            data_type: t.data_type,
            address_area: t.address_area,
            address_number: t.address_number,
            comment: t.comment,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpcUaFieldEdit {
    pub field: String,
    pub expected_value: String,
    pub replacement: String,
}
impl From<OpcUaFieldEdit> for crate::OpcUaFieldEdit {
    fn from(v: OpcUaFieldEdit) -> Self {
        Self {
            field: v.field,
            expected_value: v.expected_value,
            replacement: v.replacement,
        }
    }
}
