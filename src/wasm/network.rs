use crate::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmNetworkSummary {
    pub(super) name: Option<String>,
    pub(super) network_type: Option<String>,
    pub(super) type_name: Option<String>,
    pub(super) modules: Vec<WasmNetworkModuleSummary>,
}

impl WasmNetworkSummary {
    pub(super) fn from_network(network: NetworkSummary) -> Self {
        Self {
            name: network.name,
            network_type: network.network_type,
            type_name: network.type_name,
            modules: network
                .modules
                .into_iter()
                .map(WasmNetworkModuleSummary::from_module)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmNetworkModuleSummary {
    pub(super) name: Option<String>,
    pub(super) type_name: Option<String>,
    pub(super) id: Option<u32>,
    pub(super) base: Option<u32>,
    pub(super) slot: Option<u32>,
    pub(super) alias: Option<String>,
    pub(super) description: Option<String>,
}

impl WasmNetworkModuleSummary {
    pub(super) fn from_module(module: NetworkModuleSummary) -> Self {
        Self {
            name: module.name,
            type_name: module.type_name,
            id: module.id,
            base: module.base,
            slot: module.slot,
            alias: module.alias,
            description: module.description,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmCnetSummary {
    pub(super) station_no: Option<u32>,
    pub(super) type_code: Option<u32>,
    pub(super) base: Option<u32>,
    pub(super) slot: Option<u32>,
    pub(super) sub_type: Option<u32>,
    pub(super) ports: Vec<WasmCnetPortSummary>,
}

impl WasmCnetSummary {
    pub(super) fn from_cnet(cnet: &CnetConfigInfoSummary) -> Self {
        Self {
            station_no: cnet.station_no,
            type_code: cnet.type_code,
            base: cnet.base,
            slot: cnet.slot,
            sub_type: cnet.sub_type,
            ports: cnet
                .ports
                .iter()
                .map(WasmCnetPortSummary::from_port)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmCnetPortSummary {
    pub(super) station_no: Option<u32>,
    pub(super) mode: Option<String>,
    pub(super) baud_rate: Option<u32>,
    pub(super) data_bits: Option<String>,
    pub(super) stop_bits: Option<String>,
    pub(super) parity: Option<String>,
    pub(super) di_address: Option<String>,
    pub(super) do_address: Option<String>,
    pub(super) ai_address: Option<String>,
    pub(super) ao_address: Option<String>,
}

impl WasmCnetPortSummary {
    pub(super) fn from_port(port: &CnetPortConfigSummary) -> Self {
        Self {
            station_no: port.station_no,
            mode: port.mode_kind.map(|value| value.label().to_owned()),
            baud_rate: port.baud_rate,
            data_bits: port.data_bits.map(|value| value.label().to_owned()),
            stop_bits: port.stop_bits.map(|value| value.label().to_owned()),
            parity: port.parity_mode.map(|value| value.label().to_owned()),
            di_address: port.di_address.clone(),
            do_address: port.do_address.clone(),
            ai_address: port.ai_address.clone(),
            ao_address: port.ao_address.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmFenetSummary {
    pub(super) station_no: Option<u32>,
    pub(super) type_code: Option<u32>,
    pub(super) base: Option<u32>,
    pub(super) slot: Option<u32>,
    pub(super) sub_type: Option<u32>,
    pub(super) ip_address: Option<String>,
    pub(super) subnet: Option<String>,
    pub(super) gateway: Option<String>,
    pub(super) dns: Option<String>,
}

impl WasmFenetSummary {
    pub(super) fn from_fenet(fenet: &FenetConfigInfoSummary) -> Self {
        Self {
            station_no: fenet.station_no,
            type_code: fenet.type_code,
            base: fenet.base,
            slot: fenet.slot,
            sub_type: fenet.sub_type,
            ip_address: fenet.ip_address.as_ref().map(|value| value.address.clone()),
            subnet: fenet.subnet.as_ref().map(|value| value.address.clone()),
            gateway: fenet.gateway.as_ref().map(|value| value.address.clone()),
            dns: fenet.dns.as_ref().map(|value| value.address.clone()),
        }
    }
}
