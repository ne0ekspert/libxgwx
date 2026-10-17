#![deny(missing_docs)]
//! Guarded editing of native basic-operation parameters.
use crate::ParameterRef;
#[cfg(feature = "write")]
use crate::{XgwxDocument, XgwxError};

/// One editable timing value from the basic-operation dialog (manual pp. 353-354).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct BasicTimingField {
    /// Native attribute name used to address this field.
    pub name: String,
    /// Human-readable label including units.
    pub label: String,
    /// Exact stored value, also used for stale-edit detection.
    pub value: String,
    /// Inclusive manual minimum.
    pub minimum: u32,
    /// Inclusive manual maximum.
    pub maximum: u32,
}

/// Native choice displayed by the basic-operation dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
pub struct BasicParameterChoice {
    /// Serialized native value.
    pub value: u32,
    /// Display label.
    pub label: String,
}
/// Validated control and its captured native encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(tag = "kind", rename_all = "camelCase"))]
pub enum BasicParameterControl {
    /// Integer duration in milliseconds.
    Number {
        /// Inclusive lower bound.
        minimum: u32,
        /// Inclusive upper bound.
        maximum: u32,
    },
    /// Checkbox with explicit native values; zero is not universally false.
    Checkbox {
        /// Native checked value.
        checked: u32,
        /// Native unchecked value.
        unchecked: u32,
    },
    /// Dropdown restricted to captured native choices.
    Select {
        /// Supported values and labels.
        choices: Vec<BasicParameterChoice>,
    },
}
impl BasicParameterControl {
    fn accepts(&self, value: u32) -> bool {
        match self {
            Self::Number { minimum, maximum } => (*minimum..=*maximum).contains(&value),
            Self::Checkbox { checked, unchecked } => value == *checked || value == *unchecked,
            Self::Select { choices } => choices.iter().any(|c| c.value == value),
        }
    }
}
/// One editable basic-operation field with its exact stored value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
pub struct BasicParameterField {
    /// Native field identity.
    pub name: String,
    /// Human-readable label.
    pub label: String,
    /// Exact stored value, required by the edit's stale-value guard.
    pub value: String,
    /// Native control constraints.
    pub control: BasicParameterControl,
}

fn operation_schema(iec: bool) -> Vec<(&'static str, &'static str, BasicParameterControl)> {
    use BasicParameterControl::*;
    let flag = |checked, unchecked| Checkbox { checked, unchecked };
    let mut fields = vec![
        (
            "STATIC_PERIOD_MODE",
            "Fixed-cycle operation",
            flag(43520, 0),
        ),
        (
            "STATIC_PERIOD_TIME",
            "Fixed cycle (ms)",
            Number {
                minimum: 1,
                maximum: 999,
            },
        ),
        (
            "IO_POINT_METHOD",
            "Fixed 64-point I/O allocation",
            flag(43521, 0),
        ),
        (
            "SCAN_WD_TIME",
            "Watchdog timer (ms)",
            Number {
                minimum: 10,
                maximum: 1000,
            },
        ),
        (
            "STANDARD_INPUT_FILTER_TIME",
            "Standard input filter (ms)",
            Select {
                choices: [1, 3, 5, 10, 20, 70, 100]
                    .into_iter()
                    .map(|value| BasicParameterChoice {
                        value,
                        label: value.to_string(),
                    })
                    .collect(),
            },
        ),
        (
            "DEBUG_OUTPUT_PARAMETER",
            "Output during debugging",
            flag(0, 43536),
        ),
        (
            "ERROR_MODE_OUTPUT_PARAMETER",
            "Hold output on error",
            flag(43537, 0),
        ),
        (
            "RUN2STOP_OUTPUT_PARAMETER",
            "Hold output: RUN → STOP",
            flag(43538, 0),
        ),
        (
            "STOP2RUN_OUTPUT_PARAMETER",
            "Hold output: STOP → RUN",
            flag(43539, 0),
        ),
        (
            "ERROR_MODE_LATCH_PARAMETER",
            "Clear non-latched memory on error",
            flag(43540, 0),
        ),
        (
            "SOE_RETAIN_HISTORY",
            "Event history",
            Select {
                choices: vec![
                    BasicParameterChoice {
                        value: 43544,
                        label: "Overwrite with latest events".into(),
                    },
                    BasicParameterChoice {
                        value: 0,
                        label: "Preserve earliest events".into(),
                    },
                ],
            },
        ),
        ("RESET_DISABLE", "Block Reset switch", flag(1, 0)),
        ("O_RESET_DISABLE", "Block Overall Reset", flag(1, 0)),
        ("DCLEAR_DISABLE", "Block D.CLR switch", flag(1, 0)),
        ("O_DCLEAR_DISABLE", "Block Overall D.CLR", flag(1, 0)),
    ];
    if iec {
        fields.retain(|(name, _, _)| {
            !["IO_POINT_METHOD", "ERROR_MODE_LATCH_PARAMETER"].contains(name)
        });
        if let Some(watchdog) = fields
            .iter_mut()
            .find(|(name, _, _)| *name == "SCAN_WD_TIME")
        {
            watchdog.0 = "SCAN_WD_TIME_0";
        }
        fields.push((
            "RESTART_METHOD",
            "Restart mode",
            Select {
                choices: vec![
                    BasicParameterChoice {
                        value: 0,
                        label: "Cold restart".into(),
                    },
                    BasicParameterChoice {
                        value: 43521,
                        label: "Warm restart".into(),
                    },
                ],
            },
        ));
    }
    fields
}

impl ParameterRef<'_> {
    /// Read native basic-operation controls verified against XG5000 4.82.1.
    /// Unknown layouts and unknown checkbox/choice encodings are omitted.
    /// All raw attributes remain available through the parameter summary.
    pub fn basic_operation_fields(self) -> Vec<BasicParameterField> {
        let summary = self.summary();
        if self.type_name() != Some("BASIC PARAMETER") || summary.sections.len() != 1 {
            return Vec::new();
        }
        let section = &summary.sections[0];
        if section.child_count != 0 {
            return Vec::new();
        }
        let attr = |name: &str| {
            section
                .attributes
                .iter()
                .find(|a| a.name == name)
                .map(|a| a.value.as_str())
        };
        let iec = section.name == "XGIBasicParam"
            && self
                .configuration()
                .and_then(|c| c.cpu())
                .is_some_and(|cpu| cpu.family == "XGI");
        if !(section.name == "XGTBasicParam" || iec)
            || attr("bp_ver") != Some("100")
            || attr("size") != Some("400")
        {
            return Vec::new();
        }
        operation_schema(iec)
            .into_iter()
            .filter_map(|(name, label, control)| {
                let value = attr(name)?;
                // Unknown enum values stay visible only as raw read-only attributes.
                if !matches!(control, BasicParameterControl::Number { .. })
                    && !control.accepts(value.parse().ok()?)
                {
                    return None;
                }
                Some(BasicParameterField {
                    name: name.into(),
                    label: label.into(),
                    value: value.into(),
                    control,
                })
            })
            .collect()
    }
}

fn bounds(name: &str) -> Option<(&'static str, u32, u32)> {
    match name {
        "STATIC_PERIOD_TIME" => Some(("Fixed cycle (ms)", 1, 999)),
        "SCAN_WD_TIME" => Some(("Watchdog timer (ms)", 10, 1000)),
        _ => None,
    }
}

impl ParameterRef<'_> {
    /// Editable timing values for the captured XGK version-100 basic layout.
    /// This compatibility view is limited to XGK timings. Use
    /// [`Self::basic_operation_fields`] for native switches and XGI controls.
    /// Updating duration does not enable fixed-cycle operation.
    pub fn basic_timing_fields(self) -> Vec<BasicTimingField> {
        let summary = self.summary();
        if self.type_name() != Some("BASIC PARAMETER") || summary.sections.len() != 1 {
            return Vec::new();
        }
        let section = &summary.sections[0];
        if section.child_count != 0 {
            return Vec::new();
        }
        let attr = |name: &str| {
            section
                .attributes
                .iter()
                .find(|a| a.name == name)
                .map(|a| a.value.as_str())
        };
        if section.name != "XGTBasicParam"
            || attr("bp_ver") != Some("100")
            || attr("size") != Some("400")
        {
            return Vec::new();
        }
        ["STATIC_PERIOD_TIME", "SCAN_WD_TIME"]
            .into_iter()
            .filter_map(|name| {
                let (label, minimum, maximum) = bounds(name)?;
                Some(BasicTimingField {
                    name: name.into(),
                    label: label.into(),
                    value: attr(name)?.into(),
                    minimum,
                    maximum,
                })
            })
            .collect()
    }
}

/// Exclusively borrowed parameter collection for guarded basic-operation edits.
#[cfg(feature = "write")]
pub struct ParameterCollectionEditor<'a> {
    doc: &'a mut XgwxDocument,
}
/// Editor bound to one document-wide parameter index.
#[cfg(feature = "write")]
pub struct ParameterEditor<'a> {
    doc: &'a mut XgwxDocument,
    index: usize,
}
#[cfg(feature = "write")]
impl XgwxDocument {
    /// Select native parameter records for guarded edits.
    pub fn parameter_mut(&mut self) -> ParameterCollectionEditor<'_> {
        ParameterCollectionEditor { doc: self }
    }
}
#[cfg(feature = "write")]
impl<'a> ParameterCollectionEditor<'a> {
    /// Select by document-wide parameter index, including across multiple PLCs.
    pub fn get(self, index: usize) -> Result<ParameterEditor<'a>, XgwxError> {
        self.doc.parameter().get(index)?;
        Ok(ParameterEditor {
            doc: self.doc,
            index,
        })
    }
}
#[cfg(feature = "write")]
impl ParameterEditor<'_> {
    /// Atomically change existing basic timing fields. Each tuple contains the
    /// native field name, expected stored value, and replacement integer.
    /// Unknown layouts, stale values, duplicate fields and values outside the
    /// manual's bounds are rejected before any document mutation.
    pub fn edit_basic_timing(&mut self, changes: &[(&str, &str, u32)]) -> Result<(), XgwxError> {
        if self
            .doc
            .parameter()
            .get(self.index)?
            .basic_timing_fields()
            .is_empty()
        {
            return Err(XgwxError::BasicParameterEdit(
                "unsupported basic timing layout".into(),
            ));
        }
        if changes.iter().any(|(name, _, _)| bounds(name).is_none()) {
            return Err(XgwxError::BasicParameterEdit(
                "unsupported basic timing field".into(),
            ));
        }
        self.edit_basic_operation(changes)
    }
    /// Atomically edit verified native basic-operation controls with stale-value
    /// checks. A blocked Reset/D.CLR switch also requires its Overall block.
    /// Include both fields in a batch when toggling a full-switch block.
    pub fn edit_basic_operation(&mut self, changes: &[(&str, &str, u32)]) -> Result<(), XgwxError> {
        let fail = |message: &str| XgwxError::BasicParameterEdit(message.into());
        let fields = self
            .doc
            .parameter()
            .get(self.index)?
            .basic_operation_fields();
        if fields.is_empty() {
            return Err(fail("unsupported basic parameter layout"));
        }
        let xml = roxmltree::Document::parse(&self.doc.xml).map_err(XgwxError::Xml)?;
        let parameter = xml
            .descendants()
            .filter(|n| n.has_tag_name("Parameter"))
            .nth(self.index)
            .ok_or(XgwxError::ParameterIndexNotFound { index: self.index })?;
        let sections = parameter
            .children()
            .filter(|n| n.is_element())
            .collect::<Vec<_>>();
        let [section] = sections.as_slice() else {
            return Err(fail("ambiguous basic parameter section"));
        };
        let mut seen = std::collections::BTreeSet::new();
        let mut replacements = Vec::new();
        for &(name, expected, replacement) in changes {
            if !seen.insert(name) {
                return Err(fail("duplicate basic parameter field"));
            }
            let field = fields
                .iter()
                .find(|f| f.name == name)
                .ok_or_else(|| fail("unsupported basic parameter field"))?;
            if field.value != expected {
                return Err(fail("stale basic parameter value"));
            }
            if !field.control.accepts(replacement) {
                return Err(fail(
                    "basic parameter value is outside supported bounds or choices",
                ));
            }
            let attribute = section
                .attributes()
                .find(|a| a.name() == name)
                .ok_or_else(|| fail("missing basic parameter field"))?;
            if field.value.parse::<u32>().ok() != Some(replacement) {
                replacements.push((attribute.range_value(), replacement.to_string()));
            }
        }
        for (parent, overall) in [
            ("RESET_DISABLE", "O_RESET_DISABLE"),
            ("DCLEAR_DISABLE", "O_DCLEAR_DISABLE"),
        ] {
            if !changes
                .iter()
                .any(|(name, _, _)| *name == parent || *name == overall)
            {
                continue;
            }
            let resolved = |key| {
                changes
                    .iter()
                    .find(|(name, _, _)| *name == key)
                    .map(|(_, _, v)| *v)
                    .or_else(|| section.attribute(key).and_then(|v| v.parse::<u32>().ok()))
            };
            if resolved(parent) == Some(1) && resolved(overall) != Some(1) {
                return Err(fail(
                    "blocking a switch requires blocking its Overall operation",
                ));
            }
        }
        self.doc.apply_xml_replacements(replacements)
    }
}
