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

const HSC_COUNTER_MODE_VALUES: &[ModuleOptionValue] = &[
    ModuleOptionValue {
        label: "리니어 카운터",
        value: 0,
    },
    ModuleOptionValue {
        label: "링 카운터",
        value: 1,
    },
];

const HSC_PULSE_INPUT_MODE_VALUES: &[ModuleOptionValue] = &[
    ModuleOptionValue {
        label: "2상1체배",
        value: 0,
    },
    ModuleOptionValue {
        label: "2상2체배",
        value: 1,
    },
    ModuleOptionValue {
        label: "2상4체배",
        value: 2,
    },
    ModuleOptionValue {
        label: "CW/CCW",
        value: 3,
    },
    ModuleOptionValue {
        label: "A상입력1체배",
        value: 4,
    },
    ModuleOptionValue {
        label: "1상입력2체배",
        value: 5,
    },
    ModuleOptionValue {
        label: "1상2입력1체배",
        value: 6,
    },
    ModuleOptionValue {
        label: "1상2입력2체배",
        value: 7,
    },
];

const HSC_COMPARE_MODE_VALUES: &[ModuleOptionValue] = &[
    ModuleOptionValue {
        label: "(단일비교)작다",
        value: 0,
    },
    ModuleOptionValue {
        label: "(단일비교)같다",
        value: 1,
    },
    ModuleOptionValue {
        label: "(단일비교)크다",
        value: 2,
    },
    ModuleOptionValue {
        label: "(단일비교)같거나크다",
        value: 3,
    },
    ModuleOptionValue {
        label: "(단일비교)같거나작다",
        value: 4,
    },
    ModuleOptionValue {
        label: "(구간비교)포함",
        value: 5,
    },
    ModuleOptionValue {
        label: "(구간비교)제외",
        value: 6,
    },
];

const HSC_OUTPUT_STATE_VALUES: &[ModuleOptionValue] = &[
    ModuleOptionValue {
        label: "출력금지",
        value: 0,
    },
    ModuleOptionValue {
        label: "출력유지",
        value: 1,
    },
];

const HSC_AUXILIARY_FUNCTION_VALUES: &[ModuleOptionValue] = &[
    ModuleOptionValue {
        label: "사용안함",
        value: 0,
    },
    ModuleOptionValue {
        label: "카운트클리어",
        value: 1,
    },
    ModuleOptionValue {
        label: "카운트래치",
        value: 2,
    },
    ModuleOptionValue {
        label: "구간카운트",
        value: 3,
    },
    ModuleOptionValue {
        label: "입력 주파수측정",
        value: 4,
    },
    ModuleOptionValue {
        label: "단위 시간당 회전수측정",
        value: 5,
    },
    ModuleOptionValue {
        label: "카운트금지",
        value: 6,
    },
];

const HSC_OUTPUT_LEVEL_VALUES: &[ModuleOptionValue] = &[
    ModuleOptionValue {
        label: "Low Active",
        value: 0,
    },
    ModuleOptionValue {
        label: "High Active",
        value: 1,
    },
];

const HSC_INPUT_FILTER_VALUES: &[ModuleOptionValue] = &[
    ModuleOptionValue {
        label: "사용안함",
        value: 0,
    },
    ModuleOptionValue {
        label: "100 kPPS",
        value: 1,
    },
    ModuleOptionValue {
        label: "10 kPPS",
        value: 2,
    },
    ModuleOptionValue {
        label: "1 kPPS",
        value: 3,
    },
    ModuleOptionValue {
        label: "0.1 kPPS",
        value: 4,
    },
];

const HSC_2_CHANNEL_OPTIONS: &[ModuleOptionEntry] = &[
    ModuleOptionEntry {
        key: "counterMode",
        label: "카운터 모드",
        section: "",
        scope: "channel",
        count: 2,
        encoding: ModuleOptionEncoding {
            kind: "per-channel",
            offset: 0,
            second_offset: 0,
            stride: 100,
            bits: 32,
            shift_base: 0,
            channels_per_word: 0,
            bank_stride: 0,
        },
        values: HSC_COUNTER_MODE_VALUES,
    },
    ModuleOptionEntry {
        key: "pulseInputMode",
        label: "펄스 입력 모드",
        section: "",
        scope: "channel",
        count: 2,
        encoding: ModuleOptionEncoding {
            kind: "per-channel",
            offset: 4,
            second_offset: 0,
            stride: 100,
            bits: 32,
            shift_base: 0,
            channels_per_word: 0,
            bank_stride: 0,
        },
        values: HSC_PULSE_INPUT_MODE_VALUES,
    },
    ModuleOptionEntry {
        key: "compareOutput0Mode",
        label: "비교출력 0 모드",
        section: "",
        scope: "channel",
        count: 2,
        encoding: ModuleOptionEncoding {
            kind: "per-channel",
            offset: 32,
            second_offset: 0,
            stride: 100,
            bits: 32,
            shift_base: 0,
            channels_per_word: 0,
            bank_stride: 0,
        },
        values: HSC_COMPARE_MODE_VALUES,
    },
    ModuleOptionEntry {
        key: "compareOutput1Mode",
        label: "비교출력 1 모드",
        section: "",
        scope: "channel",
        count: 2,
        encoding: ModuleOptionEncoding {
            kind: "per-channel",
            offset: 36,
            second_offset: 0,
            stride: 100,
            bits: 32,
            shift_base: 0,
            channels_per_word: 0,
            bank_stride: 0,
        },
        values: HSC_COMPARE_MODE_VALUES,
    },
    ModuleOptionEntry {
        key: "outputStateSetting",
        label: "출력상태 설정",
        section: "",
        scope: "module",
        count: 1,
        encoding: ModuleOptionEncoding {
            kind: "scalar",
            offset: 200,
            second_offset: 0,
            stride: 0,
            bits: 32,
            shift_base: 0,
            channels_per_word: 0,
            bank_stride: 0,
        },
        values: HSC_OUTPUT_STATE_VALUES,
    },
    ModuleOptionEntry {
        key: "auxiliaryFunctionMode",
        label: "부가기능 모드",
        section: "",
        scope: "channel",
        count: 2,
        encoding: ModuleOptionEncoding {
            kind: "per-channel",
            offset: 72,
            second_offset: 0,
            stride: 100,
            bits: 32,
            shift_base: 0,
            channels_per_word: 0,
            bank_stride: 0,
        },
        values: HSC_AUXILIARY_FUNCTION_VALUES,
    },
];

const HSC_8_CHANNEL_OPTIONS: &[ModuleOptionEntry] = &[
    ModuleOptionEntry {
        key: "counterMode",
        label: "카운터 모드",
        section: "",
        scope: "channel",
        count: 8,
        encoding: ModuleOptionEncoding {
            kind: "packed-bytes",
            offset: 0,
            second_offset: 0,
            stride: 0,
            bits: 1,
            shift_base: 0,
            channels_per_word: 8,
            bank_stride: 1,
        },
        values: HSC_COUNTER_MODE_VALUES,
    },
    ModuleOptionEntry {
        key: "pulseInputMode",
        label: "펄스 입력 모드",
        section: "",
        scope: "channel",
        count: 8,
        encoding: ModuleOptionEncoding {
            kind: "packed-bytes",
            offset: 4,
            second_offset: 0,
            stride: 0,
            bits: 4,
            shift_base: 0,
            channels_per_word: 2,
            bank_stride: 1,
        },
        values: HSC_PULSE_INPUT_MODE_VALUES,
    },
    ModuleOptionEntry {
        key: "compareOutputMode",
        label: "비교 출력 모드",
        section: "",
        scope: "channel",
        count: 8,
        encoding: ModuleOptionEncoding {
            kind: "packed-bytes",
            offset: 8,
            second_offset: 0,
            stride: 0,
            bits: 4,
            shift_base: 0,
            channels_per_word: 2,
            bank_stride: 1,
        },
        values: HSC_COMPARE_MODE_VALUES,
    },
    ModuleOptionEntry {
        key: "outputStateSetting",
        label: "출력상태 설정",
        section: "",
        scope: "channel",
        count: 8,
        encoding: ModuleOptionEncoding {
            kind: "packed-bytes",
            offset: 16,
            second_offset: 0,
            stride: 0,
            bits: 1,
            shift_base: 0,
            channels_per_word: 8,
            bank_stride: 1,
        },
        values: HSC_OUTPUT_STATE_VALUES,
    },
    ModuleOptionEntry {
        key: "inputFilter",
        label: "입력 필터 값",
        section: "",
        scope: "channel",
        count: 8,
        encoding: ModuleOptionEncoding {
            kind: "packed-bytes",
            offset: 12,
            second_offset: 0,
            stride: 0,
            bits: 4,
            shift_base: 0,
            channels_per_word: 2,
            bank_stride: 1,
        },
        values: HSC_INPUT_FILTER_VALUES,
    },
    ModuleOptionEntry {
        key: "auxiliaryFunctionMode",
        label: "부가기능 모드",
        section: "",
        scope: "channel",
        count: 8,
        encoding: ModuleOptionEncoding {
            kind: "packed-bytes",
            offset: 24,
            second_offset: 0,
            stride: 0,
            bits: 4,
            shift_base: 0,
            channels_per_word: 2,
            bank_stride: 1,
        },
        values: HSC_AUXILIARY_FUNCTION_VALUES,
    },
    ModuleOptionEntry {
        key: "pulseInputLevel",
        label: "펄스입력레벨",
        section: "",
        scope: "channel",
        count: 8,
        encoding: ModuleOptionEncoding {
            kind: "packed-bytes",
            offset: 28,
            second_offset: 0,
            stride: 0,
            bits: 1,
            shift_base: 0,
            channels_per_word: 8,
            bank_stride: 1,
        },
        values: HSC_OUTPUT_LEVEL_VALUES,
    },
];

/// Return the XGK modules supported by the latest stable catalog.
pub fn xgk_module_catalog() -> &'static [ModuleCatalogEntry] {
    static CATALOG: std::sync::OnceLock<Vec<ModuleCatalogEntry>> = std::sync::OnceLock::new();
    CATALOG
        .get_or_init(|| {
            XGK_MODULE_RECORDS
                .iter()
                .copied()
                .map(|mut entry| {
                    entry.options = match entry.model {
                        "XGF-HD2A" | "XGF-HO2A" => HSC_2_CHANNEL_OPTIONS,
                        "XGF-HO8A" => HSC_8_CHANNEL_OPTIONS,
                        _ => entry.options,
                    };
                    entry
                })
                .collect()
        })
        .as_slice()
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
