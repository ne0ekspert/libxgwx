#[cfg(feature = "write")]
use crate::XgwxError;

/// CPU family used for compatibility validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "UPPERCASE"))]
pub enum CpuFamily {
    Xgk,
    Xgb,
    Xgr,
    Xgi,
}

impl CpuFamily {
    /// Return the native uppercase family name used in protocol output.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Xgk => "XGK",
            Self::Xgb => "XGB",
            Self::Xgr => "XGR",
            Self::Xgi => "XGI",
        }
    }
}

impl std::fmt::Display for CpuFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One CPU model selectable in an XG5000 project configuration.
///
/// `type_code` is the numeric value stored in the `<Configuration Type>`
/// attribute. The mapping was taken from XG5000's CPU hardware catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct CpuCatalogEntry {
    /// Cataloged native model name.
    pub model: &'static str,
    /// CPU family used for compatibility validation.
    pub family: CpuFamily,
    /// Native Configuration Type value.
    pub type_code: u32,
    /// Maximum supported base count; valid base numbers are below this limit.
    pub max_base: u32,
    /// Maximum physical slot count for a supported base.
    pub max_slot: u32,
}

const CPU_CATALOG: &[CpuCatalogEntry] = &[
    CpuCatalogEntry {
        model: "XGK-CPUH",
        family: CpuFamily::Xgk,
        type_code: 0,
        max_base: 8,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGK-CPUS",
        family: CpuFamily::Xgk,
        type_code: 1,
        max_base: 4,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGK-CPUA",
        family: CpuFamily::Xgk,
        type_code: 3,
        max_base: 4,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGK-CPUE",
        family: CpuFamily::Xgk,
        type_code: 4,
        max_base: 2,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGK-CPUU",
        family: CpuFamily::Xgk,
        type_code: 5,
        max_base: 8,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGK-CPUUN",
        family: CpuFamily::Xgk,
        type_code: 14,
        max_base: 8,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGK-CPUHN",
        family: CpuFamily::Xgk,
        type_code: 16,
        max_base: 8,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGK-CPUSN",
        family: CpuFamily::Xgk,
        type_code: 17,
        max_base: 4,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGB-XBMS",
        family: CpuFamily::Xgb,
        type_code: 2,
        max_base: 1,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGB-DR16C3",
        family: CpuFamily::Xgb,
        type_code: 6,
        max_base: 1,
        max_slot: 8,
    },
    CpuCatalogEntry {
        model: "XGB-XBCH",
        family: CpuFamily::Xgb,
        type_code: 7,
        max_base: 1,
        max_slot: 11,
    },
    CpuCatalogEntry {
        model: "XGB-DR32HL",
        family: CpuFamily::Xgb,
        type_code: 8,
        max_base: 1,
        max_slot: 11,
    },
    CpuCatalogEntry {
        model: "XGB-XBCE",
        family: CpuFamily::Xgb,
        type_code: 9,
        max_base: 1,
        max_slot: 11,
    },
    CpuCatalogEntry {
        model: "XGB-XBCS",
        family: CpuFamily::Xgb,
        type_code: 10,
        max_base: 1,
        max_slot: 11,
    },
    CpuCatalogEntry {
        model: "XGB-XBCEB",
        family: CpuFamily::Xgb,
        type_code: 12,
        max_base: 1,
        max_slot: 11,
    },
    CpuCatalogEntry {
        model: "XGB-XBCEX",
        family: CpuFamily::Xgb,
        type_code: 13,
        max_base: 1,
        max_slot: 11,
    },
    CpuCatalogEntry {
        model: "XGB-XBCU",
        family: CpuFamily::Xgb,
        type_code: 15,
        max_base: 1,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGB-XBMH",
        family: CpuFamily::Xgb,
        type_code: 18,
        max_base: 1,
        max_slot: 9,
    },
    CpuCatalogEntry {
        model: "XGB-XBMHP",
        family: CpuFamily::Xgb,
        type_code: 19,
        max_base: 1,
        max_slot: 9,
    },
    CpuCatalogEntry {
        model: "XGB-XBMH2",
        family: CpuFamily::Xgb,
        type_code: 21,
        max_base: 1,
        max_slot: 11,
    },
    CpuCatalogEntry {
        model: "XGB-XBCXS",
        family: CpuFamily::Xgb,
        type_code: 22,
        max_base: 1,
        max_slot: 11,
    },
    CpuCatalogEntry {
        model: "XGB-XECH",
        family: CpuFamily::Xgb,
        type_code: 103,
        max_base: 1,
        max_slot: 11,
    },
    CpuCatalogEntry {
        model: "XGB-XECS",
        family: CpuFamily::Xgb,
        type_code: 108,
        max_base: 1,
        max_slot: 11,
    },
    CpuCatalogEntry {
        model: "XGB-XECE",
        family: CpuFamily::Xgb,
        type_code: 109,
        max_base: 1,
        max_slot: 11,
    },
    CpuCatalogEntry {
        model: "XGB-XECU",
        family: CpuFamily::Xgb,
        type_code: 112,
        max_base: 1,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGB-KL",
        family: CpuFamily::Xgb,
        type_code: 113,
        max_base: 1,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGB-GIPAM",
        family: CpuFamily::Xgb,
        type_code: 114,
        max_base: 1,
        max_slot: 2,
    },
    CpuCatalogEntry {
        model: "XGB-XEMHP",
        family: CpuFamily::Xgb,
        type_code: 115,
        max_base: 1,
        max_slot: 11,
    },
    CpuCatalogEntry {
        model: "XGB-XEMH2",
        family: CpuFamily::Xgb,
        type_code: 116,
        max_base: 1,
        max_slot: 11,
    },
    // Captured from native XG5000 4.82.1 XGR ST/IL projects.
    CpuCatalogEntry {
        model: "XGR-CPUH",
        family: CpuFamily::Xgr,
        type_code: 101,
        max_base: 8,
        max_slot: 12,
    },
    // The ordered CPU-name table on the installed XG5000 4.82.1 disk, anchored
    // to known XGK/XGB types, yields these XGI Configuration Type mappings.
    // Physical rack limits come from LS ELECTRIC's CPU documentation.
    CpuCatalogEntry {
        model: "XGI-CPUU",
        family: CpuFamily::Xgi,
        type_code: 100,
        max_base: 8,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGI-CPUH",
        family: CpuFamily::Xgi,
        type_code: 102,
        max_base: 8,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGI-CPUS",
        family: CpuFamily::Xgi,
        type_code: 104,
        max_base: 4,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGI-CPUE",
        family: CpuFamily::Xgi,
        type_code: 106,
        max_base: 2,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGI-CPUU/D",
        family: CpuFamily::Xgi,
        type_code: 107,
        max_base: 8,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGI-CPUS/P",
        family: CpuFamily::Xgi,
        type_code: 110,
        max_base: 1,
        max_slot: 12,
    },
    CpuCatalogEntry {
        model: "XGI-CPUUN",
        family: CpuFamily::Xgi,
        type_code: 111,
        max_base: 8,
        max_slot: 12,
    },
];

/// Return CPU models supported by the Workspace Overview selector.
pub fn cpu_catalog() -> &'static [CpuCatalogEntry] {
    CPU_CATALOG
}

#[cfg(feature = "write")]
pub(crate) fn find_cpu(model: &str) -> Result<&'static CpuCatalogEntry, XgwxError> {
    cpu_catalog()
        .iter()
        .find(|entry| entry.model.eq_ignore_ascii_case(model))
        .ok_or_else(|| XgwxError::UnknownCpuModel {
            model: model.to_owned(),
        })
}

pub(crate) fn cpu_for_type(type_code: u32) -> Option<&'static CpuCatalogEntry> {
    cpu_catalog()
        .iter()
        .find(|entry| entry.type_code == type_code)
}

/// Hardware layout captured from a native XG5000 project. This is deliberately
/// variant-specific: a CPU type alone does not identify compact built-in I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct CpuHardwareProfile {
    pub cpu_model: &'static str,
    pub variant: &'static str,
    pub builtin_io_base: u32,
    pub builtin_io_slot: u32,
    pub builtin_io_id: u32,
    /// Source fixture and native Save As encodings, respectively.
    pub builtin_io_sub_types: &'static [u32],
    pub builtin_network_id: u32,
    pub builtin_network_option_type: u32,
    /// Expansion model selection is not yet supported by the XGK catalog.
    pub expansion_catalog_supported: bool,
}

const XBM_DR16S: CpuHardwareProfile = CpuHardwareProfile {
    cpu_model: "XGB-XBMS",
    variant: "XBM-DR16S",
    builtin_io_base: 0,
    builtin_io_slot: 0,
    builtin_io_id: 42249,
    builtin_io_sub_types: &[1, 0],
    builtin_network_id: 23104,
    builtin_network_option_type: 32773,
    expansion_catalog_supported: false,
};

impl crate::XgwxDocument {
    /// Recognize the captured compact variant by CPU type and built-in module
    /// identity. `None` means unverified, not that the CPU has no fixed hardware.
    pub(crate) fn cpu_hardware_profile(&self) -> Option<&'static CpuHardwareProfile> {
        let configurations = self.configurations();
        if configurations.len() != 1 || configurations[0].type_code != Some(2) {
            return None;
        }
        let modules = self.module_summaries();
        let mut builtin = modules
            .iter()
            .filter(|m| m.base == Some(0) && m.slot == Some(0));
        let module = builtin.next()?;
        if builtin.next().is_some()
            || module.id != Some(XBM_DR16S.builtin_io_id)
            || !module
                .sub_type
                .is_some_and(|value| XBM_DR16S.builtin_io_sub_types.contains(&value))
            || !module.name.as_deref()?.contains("XBM-DR16S")
        {
            return None;
        }
        Some(&XBM_DR16S)
    }
}

#[cfg(all(test, feature = "wasm"))]
mod tests {
    #[test]
    fn catalog_serialization_preserves_uppercase_family_names() {
        let catalog = serde_json::to_value(super::cpu_catalog()).unwrap();
        for (model, family) in [
            ("XGK-CPUH", "XGK"),
            ("XGB-XBMS", "XGB"),
            ("XGR-CPUH", "XGR"),
            ("XGI-CPUU", "XGI"),
        ] {
            let entry = catalog
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["model"] == model)
                .unwrap();
            assert_eq!(entry["family"], family);
        }
    }
}
