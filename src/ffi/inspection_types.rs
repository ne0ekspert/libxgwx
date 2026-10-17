use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModuleInputFilter {
    Default,
    Ms1,
    Ms3,
    Ms5,
    Ms10,
    Ms20,
    Ms70,
    Ms100,
    Unknown(u8),
}

impl From<ModuleInputFilter> for crate::ModuleInputFilter {
    fn from(value: ModuleInputFilter) -> Self {
        match value {
            ModuleInputFilter::Default => crate::ModuleInputFilter::Default,
            ModuleInputFilter::Ms1 => crate::ModuleInputFilter::Ms1,
            ModuleInputFilter::Ms3 => crate::ModuleInputFilter::Ms3,
            ModuleInputFilter::Ms5 => crate::ModuleInputFilter::Ms5,
            ModuleInputFilter::Ms10 => crate::ModuleInputFilter::Ms10,
            ModuleInputFilter::Ms20 => crate::ModuleInputFilter::Ms20,
            ModuleInputFilter::Ms70 => crate::ModuleInputFilter::Ms70,
            ModuleInputFilter::Ms100 => crate::ModuleInputFilter::Ms100,
            ModuleInputFilter::Unknown(value) => crate::ModuleInputFilter::Unknown(value),
        }
    }
}

impl From<crate::ModuleInputFilter> for ModuleInputFilter {
    fn from(value: crate::ModuleInputFilter) -> Self {
        match value {
            crate::ModuleInputFilter::Default => ModuleInputFilter::Default,
            crate::ModuleInputFilter::Ms1 => ModuleInputFilter::Ms1,
            crate::ModuleInputFilter::Ms3 => ModuleInputFilter::Ms3,
            crate::ModuleInputFilter::Ms5 => ModuleInputFilter::Ms5,
            crate::ModuleInputFilter::Ms10 => ModuleInputFilter::Ms10,
            crate::ModuleInputFilter::Ms20 => ModuleInputFilter::Ms20,
            crate::ModuleInputFilter::Ms70 => ModuleInputFilter::Ms70,
            crate::ModuleInputFilter::Ms100 => ModuleInputFilter::Ms100,
            crate::ModuleInputFilter::Unknown(value) => ModuleInputFilter::Unknown(value),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SfcEntity {
    pub entity_index: usize,
    pub type_code: Option<u32>,
    pub row: Option<u32>,
    pub column: Option<u32>,
    pub properties: BTreeMap<String, BTreeMap<String, String>>,
}

impl From<SfcEntity> for crate::SfcEntity {
    fn from(value: SfcEntity) -> Self {
        Self {
            entity_index: value.entity_index,
            type_code: value.type_code,
            row: value.row,
            column: value.column,
            properties: value.properties,
        }
    }
}

impl From<crate::SfcEntity> for SfcEntity {
    fn from(value: crate::SfcEntity) -> Self {
        Self {
            entity_index: value.entity_index,
            type_code: value.type_code,
            row: value.row,
            column: value.column,
            properties: value.properties,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SfcRow {
    pub kind: String,
    pub title: String,
    pub comment: String,
    pub initial: bool,
    pub action: Option<String>,
    #[serde(default)]
    pub action_qualifier: Option<String>,
    #[serde(default)]
    pub action_time: Option<String>,
    #[serde(default)]
    pub action_code: Option<String>,
    #[serde(default)]
    pub transition_code: Option<String>,
    #[serde(default)]
    pub position: Option<SfcPosition>,
    #[serde(default)]
    pub branch_end: Option<u32>,
}

impl From<SfcRow> for crate::SfcRow {
    fn from(value: SfcRow) -> Self {
        Self {
            kind: value.kind,
            title: value.title,
            comment: value.comment,
            initial: value.initial,
            action: value.action,
            action_qualifier: value.action_qualifier,
            action_time: value.action_time,
            action_code: value.action_code,
            transition_code: value.transition_code,
            position: value.position.map(Into::into),
            branch_end: value.branch_end,
        }
    }
}

impl From<crate::SfcRow> for SfcRow {
    fn from(value: crate::SfcRow) -> Self {
        Self {
            kind: value.kind,
            title: value.title,
            comment: value.comment,
            initial: value.initial,
            action: value.action,
            action_qualifier: value.action_qualifier,
            action_time: value.action_time,
            action_code: value.action_code,
            transition_code: value.transition_code,
            position: value.position.map(Into::into),
            branch_end: value.branch_end,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SfcPosition {
    pub row: u32,
    pub column: u32,
}

impl From<SfcPosition> for crate::SfcPosition {
    fn from(value: SfcPosition) -> Self {
        Self {
            row: value.row,
            column: value.column,
        }
    }
}

impl From<crate::SfcPosition> for SfcPosition {
    fn from(value: crate::SfcPosition) -> Self {
        Self {
            row: value.row,
            column: value.column,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SfcVariable {
    pub name: String,
    pub data_type: String,
    pub description: String,
    pub system: bool,
    #[serde(default)]
    pub declaration: Option<SfcDeclaration>,
}

impl From<SfcVariable> for crate::SfcVariable {
    fn from(value: SfcVariable) -> Self {
        Self {
            name: value.name,
            data_type: value.data_type,
            description: value.description,
            system: value.system,
            declaration: value.declaration.map(Into::into),
        }
    }
}

impl From<crate::SfcVariable> for SfcVariable {
    fn from(value: crate::SfcVariable) -> Self {
        Self {
            name: value.name,
            data_type: value.data_type,
            description: value.description,
            system: value.system,
            declaration: value.declaration.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SfcArrayBound {
    pub lower: i32,
    pub upper: i32,
}

impl From<SfcArrayBound> for crate::SfcArrayBound {
    fn from(value: SfcArrayBound) -> Self {
        Self {
            lower: value.lower,
            upper: value.upper,
        }
    }
}

impl From<crate::SfcArrayBound> for SfcArrayBound {
    fn from(value: crate::SfcArrayBound) -> Self {
        Self {
            lower: value.lower,
            upper: value.upper,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SfcDeclaration {
    #[serde(default)]
    pub dimensions: Vec<SfcArrayBound>,
    #[serde(default)]
    pub initial_value: String,
    #[serde(default)]
    pub retain: bool,
}

impl From<SfcDeclaration> for crate::SfcDeclaration {
    fn from(value: SfcDeclaration) -> Self {
        Self {
            dimensions: value.dimensions.into_iter().map(Into::into).collect(),
            initial_value: value.initial_value,
            retain: value.retain,
        }
    }
}

impl From<crate::SfcDeclaration> for SfcDeclaration {
    fn from(value: crate::SfcDeclaration) -> Self {
        Self {
            dimensions: value.dimensions.into_iter().map(Into::into).collect(),
            initial_value: value.initial_value,
            retain: value.retain,
        }
    }
}
