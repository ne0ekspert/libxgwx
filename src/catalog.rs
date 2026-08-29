use crate::XgwxError;

/// One selectable XGK module and the default values XG5000 assigns to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct ModuleCatalogEntry {
    pub model: &'static str,
    pub category: &'static str,
    pub id: u32,
    pub sub_type: u32,
    pub name: &'static str,
    pub details: &'static str,
    /// Number of consecutive physical base slots occupied by the module.
    pub slot_span: u32,
    pub status: &'static str,
    /// Dropdown-style options whose byte encodings were verified in XG5000.
    pub options: &'static [ModuleOptionEntry],
    /// All options observed in the module dialog, including read-only fields
    /// whose `Details` encoding has not been mapped safely yet.
    pub visible_options: &'static [ModuleVisibleOption],
}

/// One option observed in a module dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct ModuleVisibleOption {
    pub key: &'static str,
    pub label: &'static str,
    pub section: &'static str,
    /// `module`, `channel`, `group`, `file`, or `fileData`.
    pub scope: &'static str,
    /// Number of child items per parent for nested scopes such as `fileData`.
    /// Zero for flat scopes.
    pub items_per_parent: u32,
    pub count: u32,
    /// Human-readable default captured from the module dialog.
    pub default_value: &'static str,
    /// All labels observed in a dropdown. Empty for numeric/text fields.
    pub choices: &'static [&'static str],
}

/// One verified dropdown option for a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct ModuleOptionEntry {
    /// Stable key. Sectioned modules use keys such as `input.channelOperation`.
    pub key: &'static str,
    pub label: &'static str,
    pub section: &'static str,
    /// `module`, `channel`, or `group`.
    pub scope: &'static str,
    pub count: u32,
    pub encoding: ModuleOptionEncoding,
    pub values: &'static [ModuleOptionValue],
}

/// One label/value pair exposed by XG5000.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct ModuleOptionValue {
    pub label: &'static str,
    pub value: u32,
}

/// Normalized storage metadata for a verified module option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct ModuleOptionEncoding {
    pub kind: &'static str,
    pub offset: u32,
    pub second_offset: u32,
    pub stride: u32,
    pub bits: u32,
    pub shift_base: u32,
    pub channels_per_word: u32,
    pub bank_stride: u32,
}

include!("catalog_data.rs");

/// Return the XGK modules supported by the latest stable catalog.
pub fn xgk_module_catalog() -> &'static [ModuleCatalogEntry] {
    XGK_MODULE_RECORDS
}

pub(crate) fn find_xgk_module(model: &str) -> Result<&'static ModuleCatalogEntry, XgwxError> {
    xgk_module_catalog()
        .iter()
        .find(|entry| entry.model.eq_ignore_ascii_case(model))
        .ok_or_else(|| XgwxError::UnknownModuleCatalogModel {
            model: model.to_owned(),
        })
}

pub(crate) fn find_xgk_module_option(
    entry: &'static ModuleCatalogEntry,
    key: &str,
) -> Result<&'static ModuleOptionEntry, XgwxError> {
    entry
        .options
        .iter()
        .find(|option| option.key == key)
        .ok_or_else(|| XgwxError::UnknownModuleOption {
            model: entry.model.to_owned(),
            key: key.to_owned(),
        })
}
