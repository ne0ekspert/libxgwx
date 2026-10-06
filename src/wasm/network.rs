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
    pub(super) config_name: Option<String>,
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
            config_name: module.config_name,
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
pub(super) struct WasmXgpdSummary {
    pub(super) kind: String,
    pub(super) station_no: Option<u32>,
    pub(super) type_code: Option<u32>,
    pub(super) base: Option<u32>,
    pub(super) slot: Option<u32>,
    pub(super) sub_type: Option<u32>,
    pub(super) attributes: Vec<WasmNetworkAttribute>,
}

impl WasmXgpdSummary {
    pub(super) fn from_xgpd(config: XgpdConfigInfoSummary) -> Self {
        Self {
            kind: config.kind,
            station_no: config.station_no,
            type_code: config.type_code,
            base: config.base,
            slot: config.slot,
            sub_type: config.sub_type,
            attributes: config
                .attributes
                .into_iter()
                .map(WasmNetworkAttribute::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmNetworkAttribute {
    pub(super) name: String,
    pub(super) value: String,
}

impl From<XmlAttribute> for WasmNetworkAttribute {
    fn from(attribute: XmlAttribute) -> Self {
        Self {
            name: attribute.name,
            value: attribute.value,
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
    pub(super) mode_raw: Option<u32>,
    pub(super) mode: Option<String>,
    pub(super) bps: Option<u32>,
    pub(super) baud_rate: Option<u32>,
    pub(super) data_bit_raw: Option<u32>,
    pub(super) data_bits: Option<String>,
    pub(super) stop_bit_raw: Option<u32>,
    pub(super) stop_bits: Option<String>,
    pub(super) parity_raw: Option<u32>,
    pub(super) parity: Option<String>,
    pub(super) rx_timeout: Option<u32>,
    pub(super) char_timeout: Option<u32>,
    pub(super) inter_char_timeout: Option<u32>,
    pub(super) driver_type: Option<u32>,
    pub(super) di_device: Option<char>,
    pub(super) di_address: Option<String>,
    pub(super) di_device_type: Option<u32>,
    pub(super) di_data_type: Option<u32>,
    pub(super) di_size: Option<u32>,
    pub(super) di_addr: Option<u32>,
    pub(super) do_device: Option<char>,
    pub(super) do_address: Option<String>,
    pub(super) do_device_type: Option<u32>,
    pub(super) do_data_type: Option<u32>,
    pub(super) do_size: Option<u32>,
    pub(super) do_addr: Option<u32>,
    pub(super) ai_device: Option<char>,
    pub(super) ai_address: Option<String>,
    pub(super) ai_device_type: Option<u32>,
    pub(super) ai_data_type: Option<u32>,
    pub(super) ai_size: Option<u32>,
    pub(super) ai_addr: Option<u32>,
    pub(super) ao_device: Option<char>,
    pub(super) ao_address: Option<String>,
    pub(super) ao_device_type: Option<u32>,
    pub(super) ao_data_type: Option<u32>,
    pub(super) ao_size: Option<u32>,
    pub(super) ao_addr: Option<u32>,
    pub(super) terminating_resister: Option<u32>,
    pub(super) repeater: Option<u32>,
}

impl WasmCnetPortSummary {
    pub(super) fn from_port(port: &CnetPortConfigSummary) -> Self {
        Self {
            station_no: port.station_no,
            mode_raw: port.mode,
            mode: port.mode_kind.map(|value| value.label().to_owned()),
            bps: port.bps,
            baud_rate: port.baud_rate,
            data_bit_raw: port.data_bit,
            data_bits: port.data_bits.map(|value| value.label().to_owned()),
            stop_bit_raw: port.stop_bit,
            stop_bits: port.stop_bits.map(|value| value.label().to_owned()),
            parity_raw: port.parity,
            parity: port.parity_mode.map(|value| value.label().to_owned()),
            rx_timeout: port.rx_timeout,
            char_timeout: port.char_timeout,
            inter_char_timeout: port.inter_char_timeout,
            driver_type: port.driver_type,
            di_device: port.di_device,
            di_address: port.di_address.clone(),
            di_device_type: port.di_device_type,
            di_data_type: port.di_data_type,
            di_size: port.di_size,
            di_addr: port.di_addr,
            do_device: port.do_device,
            do_address: port.do_address.clone(),
            do_device_type: port.do_device_type,
            do_data_type: port.do_data_type,
            do_size: port.do_size,
            do_addr: port.do_addr,
            ai_device: port.ai_device,
            ai_address: port.ai_address.clone(),
            ai_device_type: port.ai_device_type,
            ai_data_type: port.ai_data_type,
            ai_size: port.ai_size,
            ai_addr: port.ai_addr,
            ao_device: port.ao_device,
            ao_address: port.ao_address.clone(),
            ao_device_type: port.ao_device_type,
            ao_data_type: port.ao_data_type,
            ao_size: port.ao_size,
            ao_addr: port.ao_addr,
            terminating_resister: port.terminating_resister,
            repeater: port.repeater,
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
    pub(super) ip_address2: Option<String>,
    pub(super) subnet2: Option<String>,
    pub(super) gateway2: Option<String>,
    pub(super) dns2: Option<String>,
    pub(super) dhcp: Option<u32>,
    pub(super) dhcp2: Option<u32>,
    pub(super) driver_type: Option<u32>,
    pub(super) rapienet_protocol: Option<u32>,
    pub(super) rcv_wait_time: Option<u32>,
    pub(super) rcv_wait_time_unit: Option<u32>,
    pub(super) client_wait_time: Option<u32>,
    pub(super) client_wait_time_unit: Option<u32>,
    pub(super) glofa_socket_count: Option<u32>,
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
            ip_address2: fenet
                .ip_address2
                .as_ref()
                .map(|value| value.address.clone()),
            subnet2: fenet.subnet2.as_ref().map(|value| value.address.clone()),
            gateway2: fenet.gateway2.as_ref().map(|value| value.address.clone()),
            dns2: fenet.dns2.as_ref().map(|value| value.address.clone()),
            dhcp: fenet.dhcp,
            dhcp2: fenet
                .attributes
                .iter()
                .find(|a| a.name == "Dhcp2")
                .and_then(|a| a.value.parse().ok()),
            driver_type: fenet.driver_type,
            rapienet_protocol: fenet
                .attributes
                .iter()
                .find(|a| a.name == "RapienetProtocol")
                .and_then(|a| a.value.parse().ok()),
            rcv_wait_time: fenet.rcv_wait_time,
            rcv_wait_time_unit: fenet
                .attributes
                .iter()
                .find(|a| a.name == "RcvWaitTimeUnit")
                .and_then(|a| a.value.parse().ok()),
            client_wait_time: fenet.client_wait_time,
            client_wait_time_unit: fenet
                .attributes
                .iter()
                .find(|a| a.name == "ClientWaitTimeUnit")
                .and_then(|a| a.value.parse().ok()),
            glofa_socket_count: fenet.glofa_socket_count,
        }
    }
}
