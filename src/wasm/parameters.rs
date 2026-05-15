use crate::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmHscSummary {
    pub(super) payload_bytes: usize,
    pub(super) channels: Vec<WasmHscChannelSummary>,
}

impl WasmHscSummary {
    pub(super) fn from_parameter(parameter: HscParameterSummary) -> Self {
        Self {
            payload_bytes: parameter.payload_bytes.len(),
            channels: parameter
                .channels
                .into_iter()
                .map(WasmHscChannelSummary::from_channel)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmHscChannelSummary {
    pub(super) channel: usize,
    pub(super) counter_mode: Option<String>,
    pub(super) pulse_input_mode: Option<String>,
    pub(super) compare_output_mode: Option<String>,
    pub(super) ring_counter_max: Option<i32>,
    pub(super) compare_output_min: Option<i32>,
    pub(super) compare_output_max: Option<i32>,
    pub(super) unit_time_ms: Option<u16>,
    pub(super) pulses_per_revolution: Option<u16>,
}

impl WasmHscChannelSummary {
    pub(super) fn from_channel(channel: HscChannelSummary) -> Self {
        Self {
            channel: channel.channel,
            counter_mode: channel.counter_mode.map(|value| value.to_string()),
            pulse_input_mode: channel.pulse_input_mode.map(|value| value.to_string()),
            compare_output_mode: channel.compare_output_mode.map(|value| value.to_string()),
            ring_counter_max: channel.ring_counter_max,
            compare_output_min: channel.compare_output_min,
            compare_output_max: channel.compare_output_max,
            unit_time_ms: channel.unit_time_ms,
            pulses_per_revolution: channel.pulses_per_revolution,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmPositionSummary {
    pub(super) axis_count: Option<u32>,
    pub(super) axes: Vec<WasmPositionAxisSummary>,
}

impl WasmPositionSummary {
    pub(super) fn from_position(position: PositionParameterSummary) -> Self {
        Self {
            axis_count: position.axis_count,
            axes: position
                .axes
                .into_iter()
                .map(WasmPositionAxisSummary::from_axis)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmPositionAxisSummary {
    pub(super) axis_name: String,
    pub(super) step_count: Option<u32>,
    pub(super) parsed_steps: usize,
}

impl WasmPositionAxisSummary {
    pub(super) fn from_axis(axis: PositionAxisSummary) -> Self {
        Self {
            axis_name: axis.axis_name,
            step_count: axis.step_count,
            parsed_steps: axis.steps.len(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmPidSummary {
    pub(super) cal_parameters: usize,
    pub(super) tune_parameters: usize,
    pub(super) cal_loops: usize,
    pub(super) tune_loops: usize,
}
