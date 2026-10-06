//! Guarded edits to native Cnet serial-port records.
use crate::{XgwxDocument, XgwxError};
use std::collections::HashSet;

/// One changed setting, addressed by native port order (zero based).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct CnetFieldEdit {
    pub port_index: usize,
    pub field: String,
    pub expected_value: String,
    pub replacement: String,
}

/// Related Cnet changes applied as one validated transaction.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct CnetSettingsPatch {
    pub base: u32,
    pub slot: u32,
    pub changes: Vec<CnetFieldEdit>,
}

impl XgwxDocument {
    /// Edit existing Cnet fields without rebuilding port records or opaque data.
    pub fn edit_cnet_settings(&mut self, patch: &CnetSettingsPatch) -> Result<(), XgwxError> {
        let fail = |reason: &str| XgwxError::BrowserNetworkEdit(reason.to_owned());
        let xml = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let at_position = |n: &roxmltree::Node<'_, '_>| {
            n.attribute("Base").and_then(|v| v.parse().ok()) == Some(patch.base)
                && n.attribute("Slot").and_then(|v| v.parse().ok()) == Some(patch.slot)
        };
        let modules = xml
            .descendants()
            .filter(|n| n.has_tag_name("NetworkModule") && at_position(n))
            .collect::<Vec<_>>();
        let records = xml
            .descendants()
            .filter(|n| n.has_tag_name("XGPD_CONFIG_INFO_CNET") && at_position(n))
            .collect::<Vec<_>>();
        let ([module], [record]) = (modules.as_slice(), records.as_slice()) else {
            return Err(fail("Cnet module or configuration is absent or ambiguous"));
        };
        if module.attribute("Id") != Some("23104")
            || module.attribute("Id") != record.attribute("Type")
            || module.attribute("OptionType") != record.attribute("SubType")
        {
            return Err(fail("Cnet identity does not match the network module"));
        }
        let subtype = record
            .attribute("SubType")
            .and_then(|v| v.parse::<u32>().ok())
            .ok_or_else(|| fail("Cnet subtype is absent"))?;
        if !matches!(subtype, 32768..=32770) {
            return Err(fail("this Cnet hardware layout has not been validated"));
        }
        let ports = record
            .children()
            .filter(|n| n.has_tag_name("XGPD_CONFIG_INFO_CNET_PORT"))
            .collect::<Vec<_>>();
        if ports.len() != 2 {
            return Err(fail("expected two native Cnet ports"));
        }
        let mut seen = HashSet::new();
        let mut replacements = Vec::new();
        for change in &patch.changes {
            if !seen.insert((change.port_index, change.field.as_str())) {
                return Err(fail("duplicate Cnet field edit"));
            }
            let port = ports
                .get(change.port_index)
                .ok_or_else(|| fail("Cnet port is absent"))?;
            let attribute = match change.field.as_str() {
                "stationNo" => "StationNo",
                "modeRaw" => "Mode",
                "bps" => "Bps",
                "dataBitRaw" => "DataBit",
                "stopBitRaw" => "StopBit",
                "parityRaw" => "Parity",
                "rxTimeout" => "RxTimeOut",
                "charTimeout" => "CharTimeOut",
                "driverType" => "DriverType",
                "requestDelayTime" => "RequestDelayTime",
                "parityErrorIgnore" => "ParityErrorIgnore",
                "terminatingResister" => "TerminatingResister",
                "repeater" => "Repeater",
                _ => return Err(fail("unsupported Cnet setting")),
            };
            let old = port
                .attributes()
                .find(|a| a.name() == attribute)
                .ok_or_else(|| fail("Cnet setting attribute is absent"))?;
            let old_value = old
                .value()
                .parse::<u32>()
                .map_err(|_| fail("existing Cnet setting is invalid"))?;
            if old_value.to_string() != change.expected_value {
                return Err(fail("stale Cnet value; reload the settings"));
            }
            if change.replacement.is_empty()
                || !change.replacement.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(fail("enter an unsigned decimal integer"));
            }
            let value = change
                .replacement
                .parse::<u32>()
                .map_err(|_| fail("Cnet integer is too large"))?;
            replacements.push((old.range_value(), value.to_string()));
        }
        let mut candidate = self.clone();
        candidate.apply_xml_replacements(replacements)?;
        let result = roxmltree::Document::parse(&candidate.xml).map_err(XgwxError::Xml)?;
        let record = result
            .descendants()
            .find(|n| n.has_tag_name("XGPD_CONFIG_INFO_CNET") && at_position(n))
            .unwrap();
        for (index, port) in record
            .children()
            .filter(|n| n.has_tag_name("XGPD_CONFIG_INFO_CNET_PORT"))
            .enumerate()
        {
            validate_port(port, subtype, index)?;
        }
        let ports = record
            .children()
            .filter(|n| n.has_tag_name("XGPD_CONFIG_INFO_CNET_PORT"))
            .collect::<Vec<_>>();
        if ports[0].attribute("Repeater") != ports[1].attribute("Repeater") {
            return Err(fail("repeater mode must match on both ports"));
        }
        if ports[0].attribute("Repeater") == Some("1")
            && ports[0].attribute("Bps") != ports[1].attribute("Bps")
        {
            return Err(fail("repeater mode requires matching baud rates"));
        }
        candidate.to_verified_bytes()?;
        *self = candidate;
        Ok(())
    }
}

fn validate_port(
    port: roxmltree::Node<'_, '_>,
    subtype: u32,
    index: usize,
) -> Result<(), XgwxError> {
    let fail = |reason: &str| XgwxError::BrowserNetworkEdit(reason.to_owned());
    let value = |name: &str| {
        port.attribute(name)
            .and_then(|v| v.parse::<u32>().ok())
            .ok_or_else(|| fail(&format!("invalid Cnet {name}")))
    };
    let mode = value("Mode")?;
    let rs232 = subtype == 32769 || subtype == 32770 && index == 0;
    if (rs232 && mode != 0) || (!rs232 && !matches!(mode, 1 | 2)) {
        return Err(fail("communication mode is not supported by this port"));
    }
    let driver = value("DriverType")?;
    if !matches!(driver, 0 | 2 | 3 | 4 | 7) {
        return Err(fail("unsupported Cnet operation mode"));
    }
    let maximum_station = if matches!(driver, 3 | 4) { 255 } else { 31 };
    if value("StationNo")? > maximum_station {
        return Err(fail(&format!(
            "station must be from 0 to {maximum_station} for this operation mode"
        )));
    }
    if value("Bps")? > 14 {
        return Err(fail("unsupported Cnet baud-rate selector"));
    }
    if driver == 3 && value("DataBit")? != 0 {
        return Err(fail("Modbus ASCII requires 7 data bits"));
    }
    if driver == 4 && value("DataBit")? != 1 {
        return Err(fail("Modbus RTU requires 8 data bits"));
    }
    if subtype == 32769 && value("Repeater")? != 0 {
        return Err(fail("this C22 layout does not support repeater mode"));
    }
    if rs232 && value("TerminatingResister")? != 0 {
        return Err(fail("RS232C ports do not support termination resistance"));
    }
    for (name, maximum) in [
        ("DataBit", 1),
        ("StopBit", 1),
        ("Parity", 2),
        ("RxTimeOut", 50),
        ("CharTimeOut", 255),
        ("RequestDelayTime", 255),
        ("ParityErrorIgnore", 1),
        ("TerminatingResister", 1),
        ("Repeater", 1),
    ] {
        if value(name)? > maximum {
            return Err(fail(&format!("Cnet {name} must be from 0 to {maximum}")));
        }
    }
    Ok(())
}

/// Default records captured from XG5000; only electrical modes vary by hardware.
pub(crate) fn default_cnet_ports(subtype: u32) -> String {
    let mut xml = String::from("<Properties Value=\"\"></Properties>");
    for index in 0..2 {
        let rs232 = subtype == 32769 || subtype == 32770 && index == 0;
        let attributes = r#"StationNo="0" StationNoB="0" Mode="0" Modem="0" Bps="8" DataBit="1" StopBit="0" Parity="0" InitCmnd_0="0" InitCmnd_1="0" InitCmnd_2="0" InitCmnd_3="0" InitCmnd_4="0" InitCmnd_5="0" InitCmnd_6="0" InitCmnd_7="0" InitCmnd_8="0" InitCmnd_9="0" InitCmnd_10="0" InitCmnd_11="0" InitCmnd_12="0" InitCmnd_13="0" InitCmnd_14="0" InitCmnd_15="0" InitCmnd_16="0" InitCmnd_17="0" InitCmnd_18="0" InitCmnd_19="0" InitCmnd_20="0" InitCmnd_21="0" RxTimeOut="1" CharTimeOut="1" InterCharTimeOut="0" DriverType="2" RequestDelayTime="0" ParityErrorIgnore="0" DI_DeviceType="80" DI_DataType="88" DI_Size="0" DI_Addr="0" DO_DeviceType="80" DO_DataType="88" DO_Size="0" DO_Addr="200" AI_DeviceType="80" AI_DataType="87" AI_Size="0" AI_Addr="200" AO_DeviceType="80" AO_DataType="87" AO_Size="0" AO_Addr="300" TerminatingResister="0" Repeater="0""#;
        xml.push_str(&format!(
            "<XGPD_CONFIG_INFO_CNET_PORT {}></XGPD_CONFIG_INFO_CNET_PORT>",
            attributes.replacen(
                "Mode=\"0\"",
                if rs232 { "Mode=\"0\"" } else { "Mode=\"1\"" },
                1
            )
        ));
    }
    xml
}
