//! OPC UA server settings stored in native Ethernet configuration records.
use crate::{NetworkView, XgwxDocument, XgwxError};

/// A native OPC UA server and its existing editable settings.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct OpcUaConfig {
    /// Owning module base.
    pub base: u32,
    /// Owning module slot.
    pub slot: u32,
    /// Native settings; absent attributes are never invented by the editor.
    pub fields: Vec<OpcUaSetting>,
    /// Derived primary server endpoint, if present.
    pub endpoint: Option<String>,
}

/// A decoded native server setting.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
pub struct OpcUaSetting {
    /// Stable library field name.
    pub field: String,
    /// Native attribute value.
    pub value: String,
}

// Field, native element, native attribute, allowed numeric range (text when None).
type SettingSpec = (&'static str, &'static str, &'static str, Option<(u32, u32)>);
const SETTINGS: &[SettingSpec] = &[
    ("applicationName", "OPCUA_Basic", "ApplicationName", None),
    ("port", "OPCUA_Basic", "Port", Some((1, 65535))),
    (
        "publishingInterval",
        "OPCUA_Basic",
        "PublishInterval",
        Some((20, 1000)),
    ),
    (
        "samplingInterval",
        "OPCUA_Basic",
        "SamplingInterval",
        Some((20, 1000)),
    ),
    ("nodeIdType", "OPCUA_Basic", "NodeType", Some((0, 1))),
    (
        "securityPolicy",
        "OPCUA_Security",
        "SecurityPolicy",
        Some((1, 127)),
    ),
    (
        "acceptAllClientCertificates",
        "OPCUA_Security",
        "AcceptAllClient",
        Some((0, 1)),
    ),
    ("allowAnonymous", "OPCUA_User", "EnableGuest", Some((0, 1))),
    (
        "allowUsernamePassword",
        "OPCUA_User",
        "EnableUser",
        Some((0, 1)),
    ),
    ("relay", "XGPD_CONFIG_INFO_FENET", "Relay", Some((0, 1))),
    (
        "snmpEnabled",
        "XGPD_CONFIG_INFO_FENET",
        "SNMPEnable",
        Some((0, 1)),
    ),
    (
        "autoNegotiationSpeedLimit",
        "XGPD_CONFIG_INFO_FENET",
        "AutoNegotiationSpeedLimit",
        Some((0, 1)),
    ),
    (
        "sntpEnabled",
        "XGPD_CONFIG_INFO_FENET_SNTP",
        "SNTPEnable",
        Some((0, 1)),
    ),
    (
        "sntpPowerOnReset",
        "XGPD_CONFIG_INFO_FENET_SNTP",
        "SNTPPowerOnReset",
        Some((0, 1)),
    ),
];

fn fail(reason: &str) -> XgwxError {
    XgwxError::BrowserNetworkEdit(reason.into())
}

fn element<'a, 'input>(
    record: roxmltree::Node<'a, 'input>,
    tag: &str,
) -> Result<roxmltree::Node<'a, 'input>, XgwxError> {
    let nodes: Vec<_> = record
        .descendants()
        .filter(|n| n.has_tag_name(tag))
        .collect();
    match nodes.as_slice() {
        [node] => Ok(*node),
        _ => Err(fail("OPC UA setting record is absent or ambiguous")),
    }
}

impl NetworkView<'_> {
    /// Decode OPC UA records for EOPCT modules. Unknown attributes remain preserved.
    pub fn opcua_config_infos(&self) -> Vec<OpcUaConfig> {
        self.doc.opcua_configs()
    }
}

impl XgwxDocument {
    pub(crate) fn opcua_configs(&self) -> Vec<OpcUaConfig> {
        let Ok(xml) = roxmltree::Document::parse(&self.xml) else {
            return Vec::new();
        };
        xml.descendants()
            .filter(|n| {
                n.has_tag_name("XGPD_CONFIG_INFO_FENET")
                    && n.attribute("Type") == Some("23045")
                    && n.attribute("SubType") == Some("32769")
            })
            .filter_map(|record| {
                let basic = element(record, "OPCUA_Basic").ok()?;
                Some(OpcUaConfig {
                    base: record.attribute("Base")?.parse().ok()?,
                    slot: record.attribute("Slot")?.parse().ok()?,
                    fields: SETTINGS
                        .iter()
                        .filter_map(|(field, tag, attr, _)| {
                            Some(OpcUaSetting {
                                field: (*field).into(),
                                value: element(record, tag).ok()?.attribute(*attr)?.into(),
                            })
                        })
                        .collect(),
                    endpoint: basic.attribute("AccessibilityServer0").map(str::to_owned),
                })
            })
            .collect()
    }
}

#[cfg(feature = "write")]
mod write {
    use super::*;
    use std::collections::HashSet;

    /// One setting update guarded by the previously inspected native value.
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
    #[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
    pub struct OpcUaFieldEdit {
        /// Stable field name from [`OpcUaConfig::fields`].
        pub field: String,
        /// Value captured before editing.
        pub expected_value: String,
        /// Replacement native value.
        pub replacement: String,
    }

    pub(crate) fn escape(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
            .replace('\r', "&#13;")
            .replace('\n', "&#10;")
            .replace('\t', "&#9;")
    }

    impl XgwxDocument {
        pub(crate) fn edit_opcua_settings(
            &mut self,
            base: u32,
            slot: u32,
            changes: &[OpcUaFieldEdit],
        ) -> Result<(), XgwxError> {
            self.ensure_consistent_api_tree()?;
            let xml = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
            let at = |n: &roxmltree::Node<'_, '_>| {
                n.attribute("Base").and_then(|v| v.parse().ok()) == Some(base)
                    && n.attribute("Slot").and_then(|v| v.parse().ok()) == Some(slot)
            };
            let modules: Vec<_> = xml
                .descendants()
                .filter(|n| n.has_tag_name("NetworkModule") && at(n))
                .collect();
            let records: Vec<_> = xml
                .descendants()
                .filter(|n| n.has_tag_name("XGPD_CONFIG_INFO_FENET") && at(n))
                .collect();
            let ([module], [record]) = (modules.as_slice(), records.as_slice()) else {
                return Err(fail("EOPCT module or settings are absent or ambiguous"));
            };
            if module.attribute("Id") != Some("23045")
                || module.attribute("OptionType") != Some("32769")
                || record.attribute("Type") != Some("23045")
                || record.attribute("SubType") != Some("32769")
            {
                return Err(fail("OPC UA settings do not match an EOPCT module"));
            }
            let owner = record.ancestors().find(|n| n.has_tag_name("Configuration"));
            if owner.and_then(|n| n.text()).map(str::trim) != module.attribute("ConfigName") {
                return Err(fail("EOPCT configuration does not belong to this PLC"));
            }
            let hardware: Vec<_> = xml
                .descendants()
                .filter(|n| n.has_tag_name("Module") && at(n))
                .collect();
            let [hardware] = hardware.as_slice() else {
                return Err(fail("EOPCT hardware is absent or ambiguous"));
            };
            if hardware.attribute("Id") != Some("23045")
                || hardware.attribute("SubType") != Some("32769")
                || hardware
                    .ancestors()
                    .find(|n| n.has_tag_name("Configuration"))
                    != owner
            {
                return Err(fail("EOPCT hardware does not match its configuration"));
            }
            if changes.is_empty() {
                return Ok(());
            }
            let mut seen = HashSet::new();
            let mut replacements = Vec::new();
            for change in changes {
                if !seen.insert(&change.field) {
                    return Err(fail("duplicate OPC UA setting"));
                }
                let (_, tag, name, range) = SETTINGS
                    .iter()
                    .find(|s| s.0 == change.field)
                    .ok_or_else(|| fail("unsupported OPC UA setting"))?;
                let node = element(*record, tag)?;
                let attr = node
                    .attributes()
                    .find(|a| a.name() == *name)
                    .ok_or_else(|| fail("OPC UA attribute is absent"))?;
                if attr.value() != change.expected_value {
                    return Err(fail("stale OPC UA setting; reload the editor"));
                }
                if let Some((min, max)) = range {
                    let text = &change.replacement;
                    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(fail("enter an unsigned decimal integer"));
                    }
                    let value: u32 = text
                        .parse()
                        .map_err(|_| fail("OPC UA integer is too large"))?;
                    if value < *min || value > *max {
                        return Err(fail(&format!(
                            "{} must be between {min} and {max}",
                            change.field
                        )));
                    }
                    if change.field == "port"
                        && matches!(value, 502 | 2002 | 2004 | 2005 | 2007 | 2008)
                    {
                        return Err(fail("OPC UA port is reserved for another PLC service"));
                    }
                } else if change.replacement.trim().is_empty()
                    || change.replacement.chars().any(char::is_control)
                {
                    return Err(fail(
                        "application name must be nonempty text without control characters",
                    ));
                }
                if change.field == "sntpEnabled"
                    && change.replacement == "1"
                    && element(*record, "XGPD_CONFIG_INFO_FENET_SNTP")?
                        .attribute("arSntpIp_Count")
                        .and_then(|v| v.parse::<u32>().ok())
                        .unwrap_or(0)
                        == 0
                {
                    return Err(fail(
                        "configure at least one NTP server before enabling time synchronization",
                    ));
                }
                replacements.push((attr.range_value(), escape(&change.replacement)));
            }
            let mut candidate = self.clone();
            candidate.apply_xml_replacements(replacements)?;
            let updated = candidate
                .opcua_configs()
                .into_iter()
                .find(|c| c.base == base && c.slot == slot)
                .ok_or_else(|| fail("updated OPC UA server is absent"))?;
            let interval = |field: &str| {
                updated
                    .fields
                    .iter()
                    .find(|s| s.field == field)
                    .and_then(|s| s.value.parse::<u32>().ok())
                    .ok_or_else(|| fail("invalid OPC UA interval"))
            };
            if interval("publishingInterval")? < interval("samplingInterval")? {
                return Err(fail(
                    "publishing interval must be greater than or equal to sampling interval",
                ));
            }
            candidate.sync_opcua_endpoint(base, slot)?;
            candidate.to_verified_bytes()?;
            *self = candidate;
            Ok(())
        }

        pub(crate) fn sync_opcua_endpoint(
            &mut self,
            base: u32,
            slot: u32,
        ) -> Result<(), XgwxError> {
            let xml = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
            let records: Vec<_> = xml
                .descendants()
                .filter(|n| {
                    n.has_tag_name("XGPD_CONFIG_INFO_FENET")
                        && n.attribute("Type") == Some("23045")
                        && n.attribute("Base").and_then(|v| v.parse().ok()) == Some(base)
                        && n.attribute("Slot").and_then(|v| v.parse().ok()) == Some(slot)
                })
                .collect();
            if records.is_empty() {
                return Ok(());
            }
            let [record] = records.as_slice() else {
                return Err(fail("ambiguous EOPCT endpoint"));
            };
            let basic = element(*record, "OPCUA_Basic")?;
            let ip = (0..4)
                .map(|i| {
                    record
                        .attribute(format!("IpAddr_{i}").as_str())
                        .and_then(|v| v.parse::<u8>().ok())
                        .ok_or_else(|| fail("invalid EOPCT IP address"))
                })
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(".");
            let port = basic
                .attribute("Port")
                .ok_or_else(|| fail("missing OPC UA port"))?;
            let mut replacements = Vec::new();
            for (name, value) in [
                ("IPAddress0", ip.clone()),
                ("AccessibilityServer0", format!("opc.tcp://{ip}:{port}")),
            ] {
                let attr = basic
                    .attributes()
                    .find(|a| a.name() == name)
                    .ok_or_else(|| fail("missing OPC UA endpoint attribute"))?;
                replacements.push((attr.range_value(), escape(&value)));
            }
            self.apply_xml_replacements(replacements)
        }
    }
}
#[cfg(feature = "write")]
pub use write::OpcUaFieldEdit;
