#![deny(missing_docs)]
//! Guarded updates to captured native parameter records.
#[cfg(feature = "write")]
use crate::{ParameterEditor, XgwxError};
use crate::{ParameterKind, ParameterRef, XmlElement};

/// Location of an existing native parameter field. Indices are zero-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(
    any(feature = "wasm", feature = "ffi"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(
    any(feature = "wasm", feature = "ffi"),
    serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)
)]
pub enum NativeParameterTarget {
    /// Decoded settings in the legacy FENET PARAMETER Safety_Comm record.
    SafetyCommunication {},
    /// Counter/pulse mode nibbles and time/revolution words in a 224-byte HSC payload.
    HscChannel {
        /// Channel index (0 through 3).
        channel: usize,
    },
    // Empty struct variants make serde reject extra keys on settings targets,
    // just as it does on the indexed targets below.
    /// PID calculation settings in their native integer encodings.
    PidCalculation {},
    /// One embedded PID calculation loop.
    PidCalculationLoop {
        /// Loop index.
        loop_index: usize,
    },
    /// PID tuning settings in their native integer encodings.
    PidTuning {},
    /// One embedded PID tuning loop.
    PidTuningLoop {
        /// Loop index.
        loop_index: usize,
    },
    /// Shared positioning parameters for one axis.
    PositionAxis {
        /// Axis index.
        axis_index: usize,
    },
    /// One positioning table row.
    PositionStep {
        /// Axis index.
        axis_index: usize,
        /// Step index within the axis.
        step_index: usize,
    },
}

/// Editable native storage field. Values and enum/bit-mask encodings are raw;
/// PID/position bounds describe storage widths. Captured FEnet settings and
/// HSC fields additionally use their known native limits.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(feature = "wasm", feature = "ffi"), derive(serde::Serialize))]
pub struct NativeParameterField {
    /// Record containing this field.
    pub target: NativeParameterTarget,
    /// Native XML attribute name, or HSC decoded field name.
    pub name: String,
    /// Exact expected value for a guarded edit (decimal for packed HSC fields).
    pub value: String,
    /// Inclusive storage/known encoding lower bound.
    pub minimum: i64,
    /// Inclusive storage/known encoding upper bound.
    pub maximum: i64,
}

/// One change in an atomic native parameter edit.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    any(feature = "wasm", feature = "ffi"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(any(feature = "wasm", feature = "ffi"), serde(deny_unknown_fields))]
pub struct NativeParameterChange {
    /// Record containing the field.
    pub target: NativeParameterTarget,
    /// Name from [`NativeParameterField::name`].
    pub name: String,
    /// Exact value from [`NativeParameterField::value`].
    pub expected: String,
    /// Replacement raw native integer.
    pub replacement: i64,
}

// Only decoded settings are editable. Headers, sizes, checksums, reserved data
// and tentative HSC offsets are deliberately absent from these schemas.
type Schema = &'static [(&'static str, i64, i64)];
const SAFETY_COMM: Schema = &[
    ("RcvWaitTime", 2, 255),
    ("RetransTime", 1, 600),
    ("GlofaSocketCnt", 1, 4),
    ("DriverType", 2, 5),
    ("EnableHostTable", 0, 1),
    ("IPAddress", 0, u32::MAX as i64),
    ("Gateway", 0, u32::MAX as i64),
    ("Subnet", 0, u32::MAX as i64),
];
const PID_CAL: Schema = &[
    ("SetPidOut", 0, u32::MAX as i64),
    ("SetDirection", 0, u32::MAX as i64),
    ("PreventAntiWindup", 0, u32::MAX as i64),
    ("ProControlCalMethod", 0, u32::MAX as i64),
    ("DiffControlCalMethod", 0, u32::MAX as i64),
    ("PermitPWM", 0, u32::MAX as i64),
];
const PID_CAL_LOOP: Schema = &[
    ("TargetValue", i32::MIN as i64, i32::MAX as i64),
    ("ScanTime", 0, u32::MAX as i64),
    ("ProPortionGainDotLeft", i32::MIN as i64, i32::MAX as i64),
    ("ProPortionGainDotRight", i32::MIN as i64, i32::MAX as i64),
    ("IntergralGainDotLeft", i32::MIN as i64, i32::MAX as i64),
    ("IntergralGainDotRight", i32::MIN as i64, i32::MAX as i64),
    ("DifferentialGainDotLeft", i32::MIN as i64, i32::MAX as i64),
    ("DifferentialGainDotRight", i32::MIN as i64, i32::MAX as i64),
    ("PVLimit", 0, u32::MAX as i64),
    ("MVLimit", 0, u32::MAX as i64),
    ("MVMax", i32::MIN as i64, i32::MAX as i64),
    ("MVMin", i32::MIN as i64, i32::MAX as i64),
    ("MVMan", i32::MIN as i64, i32::MAX as i64),
    ("DeadBand", i32::MIN as i64, i32::MAX as i64),
    ("DiffDelayFilter", 0, u32::MAX as i64),
    ("ForwardPWM", 0, u32::MAX as i64),
    ("PWMOutPeriod", 0, u32::MAX as i64),
    ("SetSVSetAverage", 0, u32::MAX as i64),
    ("PVTrackingSetValue", i32::MIN as i64, i32::MAX as i64),
    ("PVMin", i32::MIN as i64, i32::MAX as i64),
    ("PVMax", i32::MIN as i64, i32::MAX as i64),
];
const PID_TUNE: Schema = &[
    ("SetDirection", 0, u32::MAX as i64),
    ("PermitPWM", 0, u32::MAX as i64),
];
const PID_TUNE_LOOP: Schema = &[
    ("TargetValue", i32::MIN as i64, i32::MAX as i64),
    ("ScanTime", 0, u32::MAX as i64),
    ("MVMax", i32::MIN as i64, i32::MAX as i64),
    ("MVMin", i32::MIN as i64, i32::MAX as i64),
    ("SetPWMAtPoint", 0, u32::MAX as i64),
    ("OutPeriod", 0, u32::MAX as i64),
    ("SetHisterisys", i32::MIN as i64, i32::MAX as i64),
];
const POSITION_AXIS: Schema = &[
    ("BiasVelocity", 0, u32::MAX as i64),
    ("VelocityLimit", 0, u32::MAX as i64),
    ("AccelTime_0", 0, u32::MAX as i64),
    ("AccelTime_1", 0, u32::MAX as i64),
    ("AccelTime_2", 0, u32::MAX as i64),
    ("AccelTime_3", 0, u32::MAX as i64),
    ("DeAccelTime_0", 0, u32::MAX as i64),
    ("DeAccelTime_1", 0, u32::MAX as i64),
    ("DeAccelTime_2", 0, u32::MAX as i64),
    ("DeAccelTime_3", 0, u32::MAX as i64),
    ("SoftUpperLimit", i32::MIN as i64, i32::MAX as i64),
    ("SoftLowerLimit", i32::MIN as i64, i32::MAX as i64),
    ("BacklashCompensation", i32::MIN as i64, i32::MAX as i64),
    ("SCurveRatio", 0, u32::MAX as i64),
    ("ENUM_Shape", 0, u32::MAX as i64),
    ("ENUM_MCode", 0, u32::MAX as i64),
    ("ENUM_DectSoftLimit", 0, u32::MAX as i64),
    ("ENUM_DisplayPosition", 0, u32::MAX as i64),
    ("ENUM_UsePosition", 0, u32::MAX as i64),
    ("ENUM_PulseOutputLevel", 0, u32::MAX as i64),
    ("ENUM_UseLimit", 0, u32::MAX as i64),
    ("ENUM_PulseOutputMode", 0, u32::MAX as i64),
    ("Orientation", i32::MIN as i64, i32::MAX as i64),
    ("ReturnVelocityHigh", 0, u32::MAX as i64),
    ("ReturnVelocityLow", 0, u32::MAX as i64),
    ("ReturnAccelTime", 0, u32::MAX as i64),
    ("ReturnDeAccelTime", 0, u32::MAX as i64),
    ("ReturnDwellTime", 0, u32::MAX as i64),
    ("ENUM_ReturnPolicy", 0, u32::MAX as i64),
    ("ENUM_ReturnDirection", 0, u32::MAX as i64),
    ("JogAccelTime", 0, u32::MAX as i64),
    ("JogDeAccelTime", 0, u32::MAX as i64),
    ("InchingTime", 0, u32::MAX as i64),
    ("JogVelocityHigh", 0, u32::MAX as i64),
    ("JogVelocityLow", 0, u32::MAX as i64),
    ("ENUM_InterpolationMethod", 0, u32::MAX as i64),
];
const POSITION_STEP: Schema = &[
    ("TargetPosition", i32::MIN as i64, i32::MAX as i64),
    ("Interpolation", i32::MIN as i64, i32::MAX as i64),
    ("MCode", i32::MIN as i64, i32::MAX as i64),
    ("DwellTime", 0, u32::MAX as i64),
    ("OperationVelocity", 0, u32::MAX as i64),
    ("IterationStep", 0, u32::MAX as i64),
    ("ENUM_OperationMode", 0, u32::MAX as i64),
    ("ENUM_ControlMode", 0, u32::MAX as i64),
    ("ENUM_OperationStep", 0, u32::MAX as i64),
    ("ENUM_Coordination", 0, u32::MAX as i64),
    ("ENUM_Interpolation", 0, u32::MAX as i64),
    ("ENUM_AccelPreset", 0, u32::MAX as i64),
];

fn children<'a>(element: &'a XmlElement, name: &str) -> Vec<&'a XmlElement> {
    element.children.iter().filter(|c| c.name == name).collect()
}

// Return paths as well as targets so the writer addresses exactly the node that
// inspection accepted, without a descendant search crossing another parameter.
fn records(element: &XmlElement) -> Vec<(NativeParameterTarget, Vec<usize>, Schema)> {
    use NativeParameterTarget::*;
    let mut out = Vec::new();
    let mut loops =
        |list: &str, node: &str, settings, make: fn(usize) -> NativeParameterTarget, schema| {
            let signature_matches = if list == "EmbedPidLoopDataList" {
                element.attribute("Header_0") == Some("17736")
                    && element.attribute("Header_1") == Some("17473")
                    && element.attribute("ParaSize") == Some("0")
            } else {
                element.attribute("Footer_0") == Some("12358")
                    && element.attribute("Footer_1") == Some("21552")
                    && element.attribute("CheckSum") == Some("0")
            };
            if !signature_matches {
                return;
            }
            let lists = children(element, list);
            if lists.len() != 1
                || children(lists[0], node).len() != 16
                || element.descendants_named(node).count() != 16
                || element.descendants_named(list).count() != 1
            {
                return;
            }
            out.push((
                settings,
                vec![],
                if list == "EmbedPidLoopDataList" {
                    PID_CAL
                } else {
                    PID_TUNE
                },
            ));
            let list_index = element
                .children
                .iter()
                .position(|c| c.name == list)
                .unwrap();
            for (index, (child_index, _)) in element.children[list_index]
                .children
                .iter()
                .enumerate()
                .filter(|(_, c)| c.name == node)
                .enumerate()
            {
                out.push((make(index), vec![list_index, child_index], schema));
            }
        };
    match element.attribute("Type") {
        Some("FENET PARAMETER") => {
            // Only the single direct legacy record is addressed. Preserve all
            // host tables, channel mappings and unknown communication settings.
            if children(element, "Safety_Comm").len() == 1
                && element.descendants_named("Safety_Comm").count() == 1
            {
                let index = element
                    .children
                    .iter()
                    .position(|c| c.name == "Safety_Comm")
                    .unwrap();
                out.push((SafetyCommunication {}, vec![index], SAFETY_COMM));
            }
        }
        Some("PID CAL PARAMETER") => loops(
            "EmbedPidLoopDataList",
            "EmbedPidLoopData",
            PidCalculation {},
            |loop_index| PidCalculationLoop { loop_index },
            PID_CAL_LOOP,
        ),
        Some("PID TUNE PARAMETER") => loops(
            "PidTunLoopDataList",
            "PidTunLoopData",
            PidTuning {},
            |loop_index| PidTuningLoop { loop_index },
            PID_TUNE_LOOP,
        ),
        Some("POSITION PARAMETER") => {
            let groups = children(element, "HSC_PRM_XGB_AXIS_GROUP");
            let [group] = groups.as_slice() else {
                return out;
            };
            let data = children(group, "HSC_PRM_XGB_AXIS_DATA");
            let params = children(group, "HSC_PRM_XGB_AXIS_PARAM");
            // Only the captured two-axis layout has a native fixture.
            if group.attribute("NUM_AXIS_DATA") != Some("2") || data.len() != 2 || params.len() != 2
            {
                return out;
            }
            if data.iter().any(|axis| {
                axis.attribute("NUM_STEP_DATA")
                    .and_then(|n| n.parse::<usize>().ok())
                    != Some(children(axis, "HSC_PRM_XGB_AXIS_STEP_DATA").len())
            }) {
                return out;
            }
            let group_index = element
                .children
                .iter()
                .position(|c| c.name == "HSC_PRM_XGB_AXIS_GROUP")
                .unwrap();
            for (axis_index, (param_index, _)) in group
                .children
                .iter()
                .enumerate()
                .filter(|(_, c)| c.name == "HSC_PRM_XGB_AXIS_PARAM")
                .enumerate()
            {
                out.push((
                    PositionAxis { axis_index },
                    vec![group_index, param_index],
                    POSITION_AXIS,
                ));
            }
            for (axis_index, (data_index, axis)) in group
                .children
                .iter()
                .enumerate()
                .filter(|(_, c)| c.name == "HSC_PRM_XGB_AXIS_DATA")
                .enumerate()
            {
                for (step_index, (child_index, _)) in axis
                    .children
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.name == "HSC_PRM_XGB_AXIS_STEP_DATA")
                    .enumerate()
                {
                    out.push((
                        PositionStep {
                            axis_index,
                            step_index,
                        },
                        vec![group_index, data_index, child_index],
                        POSITION_STEP,
                    ));
                }
            }
        }
        _ => (),
    }
    out
}

fn at_path<'a>(mut element: &'a XmlElement, path: &[usize]) -> &'a XmlElement {
    for &index in path {
        element = &element.children[index];
    }
    element
}

// HSC fields whose interpretation is not marked tentative by the reader.
// Positions are hex nibbles; words use little-endian byte order.
fn hsc_storage(channel: usize, name: &str) -> Option<(usize, usize, i64, i64)> {
    if channel >= 4 {
        return None;
    }
    Some(match name {
        "counter_mode" => (1 + channel, 1, 0, 1),
        "pulse_input_mode" => (5 + channel, 1, 0, 3),
        "unit_time_ms" => ((channel * 56 + 44) * 2, 4, 1, 60000),
        "pulses_per_revolution" => ((channel * 56 + 46) * 2, 4, 1, 60000),
        _ => return None,
    })
}

impl ParameterRef<'_> {
    /// List writable fields of captured native FEnet, HSC, PID and positioning records.
    /// PID/position values use the readers' signed/unsigned 32-bit storage domains; enum
    /// and bit-mask values remain native integers. This is a storage API, not a
    /// CPU-specific commissioning validator. Missing/malformed fields, unknown
    /// layouts, metadata, and tentative HSC fields remain read-only.
    pub fn native_fields(self) -> Vec<NativeParameterField> {
        let mut out = Vec::new();
        if self.type_name() == Some("FENET PARAMETER")
            && self.configuration_index().is_some()
            && !self
                .configuration_cpu()
                .is_some_and(|cpu| cpu.family == "XGK")
        {
            return out;
        }
        if self.type_name() != Some("FENET PARAMETER")
            && self.configuration_index().is_some()
            && !self
                .configuration_cpu()
                .is_some_and(|cpu| cpu.family == "XGB")
        {
            return out;
        }
        if self.kind() == ParameterKind::Hsc {
            let Ok(crate::ParameterData::Hsc(hsc)) = self.decode() else {
                return out;
            };
            if !hsc
                .payload
                .bytes()
                .all(|b| b.is_ascii_hexdigit() || b.is_ascii_whitespace())
                || hsc.payload_bytes.len() != 224
                || hsc.payload_asc_length != Some(448)
            {
                return out;
            }
            for channel in &hsc.channels {
                for (name, value) in [
                    ("counter_mode", channel.counter_mode_raw.map(i64::from)),
                    (
                        "pulse_input_mode",
                        channel.pulse_input_mode_raw.map(i64::from),
                    ),
                    ("unit_time_ms", channel.unit_time_ms.map(i64::from)),
                    (
                        "pulses_per_revolution",
                        channel.pulses_per_revolution.map(i64::from),
                    ),
                ] {
                    let (_, _, minimum, maximum) = hsc_storage(channel.channel, name).unwrap();
                    if let Some(value) = value.filter(|v| (minimum..=maximum).contains(v)) {
                        out.push(NativeParameterField {
                            target: NativeParameterTarget::HscChannel {
                                channel: channel.channel,
                            },
                            name: name.into(),
                            value: value.to_string(),
                            minimum,
                            maximum,
                        });
                    }
                }
            }
            return out;
        }
        for (target, path, schema) in records(self.element) {
            let node = at_path(self.element, &path);
            for &(name, minimum, maximum) in schema {
                if let Some(value) = node.attribute(name).filter(|v| {
                    v.parse::<i64>().ok().is_some_and(|v| {
                        (minimum..=maximum).contains(&v)
                            && (target != (NativeParameterTarget::SafetyCommunication {})
                                || name != "DriverType"
                                || matches!(v, 2 | 5))
                    })
                }) {
                    out.push(NativeParameterField {
                        target,
                        name: name.into(),
                        value: value.into(),
                        minimum,
                        maximum,
                    });
                }
            }
        }
        out
    }
}

#[cfg(feature = "write")]
impl ParameterEditor<'_> {
    /// Atomically update existing FEnet, HSC, PID or positioning storage fields.
    /// Every change must match a field returned by [`ParameterRef::native_fields`]
    /// and its exact current value. Duplicate, stale, out-of-range and unknown
    /// fields fail without mutation. Unchanged native data is preserved verbatim.
    /// No records are created or resized, and no live PLC writes are performed.
    pub fn edit_native(&mut self, changes: &[NativeParameterChange]) -> Result<(), XgwxError> {
        let fail = |message: &str| XgwxError::NativeParameterEdit(message.into());
        let record = self.doc.parameter().get(self.index)?;
        let fields = record.native_fields();
        if fields.is_empty() {
            return Err(fail("unsupported native parameter layout"));
        }
        let paths = records(record.element);
        let xml = roxmltree::Document::parse(&self.doc.xml).map_err(XgwxError::Xml)?;
        let parameter = xml
            .descendants()
            .filter(|n| n.has_tag_name("Parameter"))
            .nth(self.index)
            .ok_or(XgwxError::ParameterIndexNotFound { index: self.index })?;
        let mut replacements = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        let payload_attribute = parameter.attributes().find(|a| a.name() == "PAYLOAD");
        // Escaped payloads have no verified source-digit mapping. Reject them
        // instead of normalizing entities or accidentally editing an escape.
        if record.kind() == ParameterKind::Hsc
            && payload_attribute.is_some_and(|a| {
                self.doc.xml[a.range_value()]
                    .bytes()
                    .any(|b| !b.is_ascii_hexdigit() && !b.is_ascii_whitespace())
            })
        {
            return Err(fail("unsupported escaped HSC payload"));
        }
        let mut payload =
            payload_attribute.map(|a| self.doc.xml[a.range_value()].as_bytes().to_vec());
        // Preserve hex case and whitespace; only replace the selected digits.
        let digits: Vec<_> = payload
            .as_ref()
            .map(|p| {
                p.iter()
                    .enumerate()
                    .filter(|(_, c)| !c.is_ascii_whitespace())
                    .map(|(i, _)| i)
                    .collect()
            })
            .unwrap_or_default();
        let mut payload_changed = false;
        for change in changes {
            if !seen.insert((change.target, change.name.as_str())) {
                return Err(fail("duplicate native parameter field"));
            }
            let field = fields
                .iter()
                .find(|f| f.target == change.target && f.name == change.name)
                .ok_or_else(|| fail("unsupported native parameter field or record"))?;
            if field.value != change.expected {
                return Err(fail("stale native parameter value"));
            }
            if !(field.minimum..=field.maximum).contains(&change.replacement) {
                return Err(fail("native parameter value outside storage bounds"));
            }
            if change.target == (NativeParameterTarget::SafetyCommunication {})
                && change.name == "DriverType"
                && !matches!(change.replacement, 2 | 5)
            {
                return Err(fail(
                    "legacy FEnet driver must be XGT (2) or Modbus TCP/IP (5)",
                ));
            }
            if change.target == (NativeParameterTarget::SafetyCommunication {})
                && change.name == "Subnet"
            {
                let mask = u32::from_be_bytes((change.replacement as u32).to_le_bytes());
                let inverse = !mask;
                if inverse & inverse.wrapping_add(1) != 0 {
                    return Err(fail("subnet mask must contain contiguous leading ones"));
                }
            }
            if field.value.parse::<i64>().ok() == Some(change.replacement) {
                continue;
            }
            if let NativeParameterTarget::HscChannel { channel } = change.target {
                let (start, len, _, _) = hsc_storage(channel, &change.name).unwrap();
                let replacement = if len == 1 {
                    format!("{:X}", change.replacement)
                } else {
                    let bytes = (change.replacement as u16).to_le_bytes();
                    format!("{:02X}{:02X}", bytes[0], bytes[1])
                };
                let payload = payload
                    .as_mut()
                    .ok_or_else(|| fail("missing HSC payload"))?;
                for (offset, byte) in replacement.bytes().enumerate() {
                    payload[digits[start + offset]] = byte;
                }
                payload_changed = true;
            } else {
                let (_, path, _) = paths
                    .iter()
                    .find(|(target, _, _)| *target == change.target)
                    .ok_or_else(|| fail("unsupported native parameter record"))?;
                let mut node = parameter;
                for &index in path {
                    node = node
                        .children()
                        .filter(|n| n.is_element())
                        .nth(index)
                        .ok_or_else(|| fail("missing native parameter record"))?;
                }
                let attribute = node
                    .attributes()
                    .find(|a| a.name() == change.name)
                    .ok_or_else(|| fail("missing native parameter field"))?;
                replacements.push((attribute.range_value(), change.replacement.to_string()));
            }
        }
        if payload_changed {
            // The original payload is valid hex/ASCII, so changing hex digits
            // cannot invalidate UTF-8 or XML attribute escaping.
            replacements.push((
                payload_attribute.unwrap().range_value(),
                String::from_utf8(payload.unwrap()).unwrap(),
            ));
        }
        self.doc.apply_xml_replacements(replacements)
    }
}
