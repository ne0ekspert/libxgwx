use crate::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmAttribute {
    pub(super) name: String,
    pub(super) value: String,
}

impl From<XmlAttribute> for WasmAttribute {
    fn from(attribute: XmlAttribute) -> Self {
        Self {
            name: attribute.name,
            value: attribute.value,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmParameterSummary {
    pub(super) parameter_type: Option<String>,
    pub(super) attributes: Vec<WasmAttribute>,
    pub(super) sections: Vec<WasmParameterSectionSummary>,
}

impl WasmParameterSummary {
    pub(super) fn from_parameter(parameter: ParameterSummary) -> Self {
        Self {
            parameter_type: parameter.parameter_type,
            attributes: parameter
                .attributes
                .into_iter()
                .map(WasmAttribute::from)
                .collect(),
            sections: parameter
                .sections
                .into_iter()
                .map(WasmParameterSectionSummary::from_section)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmParameterSectionSummary {
    pub(super) name: String,
    pub(super) text: Option<String>,
    pub(super) child_count: usize,
    pub(super) attributes: Vec<WasmAttribute>,
}

impl WasmParameterSectionSummary {
    fn from_section(section: XmlSectionSummary) -> Self {
        Self {
            name: section.name,
            text: section.text,
            child_count: section.child_count,
            attributes: section
                .attributes
                .into_iter()
                .map(WasmAttribute::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmSafetyCommSummary {
    pub(super) rcv_wait_time: Option<u32>,
    pub(super) retrans_time: Option<u32>,
    pub(super) glofa_socket_count: Option<u32>,
    pub(super) driver_type: Option<u32>,
    pub(super) ip_address: Option<String>,
    pub(super) ip_address_raw: Option<u32>,
    pub(super) gateway: Option<String>,
    pub(super) gateway_raw: Option<u32>,
    pub(super) subnet: Option<String>,
    pub(super) subnet_raw: Option<u32>,
    pub(super) enable_host_table: Option<u32>,
    pub(super) channels: Vec<WasmSafetyCommChannelSummary>,
}

impl WasmSafetyCommSummary {
    pub(super) fn from_safety(safety: SafetyCommSummary) -> Self {
        Self {
            rcv_wait_time: safety.rcv_wait_time,
            retrans_time: safety.retrans_time,
            glofa_socket_count: safety.glofa_socket_count,
            driver_type: safety.driver_type,
            ip_address: safety.ip_address,
            ip_address_raw: safety.ip_address_raw,
            gateway: safety.gateway,
            gateway_raw: safety.gateway_raw,
            subnet: safety.subnet,
            subnet_raw: safety.subnet_raw,
            enable_host_table: safety.enable_host_table,
            channels: safety
                .channels
                .into_iter()
                .map(WasmSafetyCommChannelSummary::from_channel)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmSafetyCommChannelSummary {
    pub(super) name: String,
    pub(super) address: Option<String>,
    pub(super) device: Option<char>,
    pub(super) device_type: Option<u32>,
    pub(super) data_type: Option<u32>,
    pub(super) size: Option<u32>,
    pub(super) addr: Option<u32>,
}

impl WasmSafetyCommChannelSummary {
    fn from_channel(channel: SafetyCommChannelSummary) -> Self {
        Self {
            name: channel.name,
            address: channel.address,
            device: channel.device,
            device_type: channel.device_type,
            data_type: channel.data_type,
            size: channel.size,
            addr: channel.addr,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmHscSummary {
    pub(super) payload_asc_length: Option<u32>,
    pub(super) payload_bytes: usize,
    pub(super) initial_unknown_nibble: Option<u8>,
    pub(super) channels: Vec<WasmHscChannelSummary>,
}

impl WasmHscSummary {
    pub(super) fn from_parameter(parameter: HscParameterSummary) -> Self {
        Self {
            payload_asc_length: parameter.payload_asc_length,
            payload_bytes: parameter.payload_bytes.len(),
            initial_unknown_nibble: parameter.initial_unknown_nibble,
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
    pub(super) counter_mode_raw: Option<u8>,
    pub(super) counter_mode: Option<String>,
    pub(super) pulse_input_mode_raw: Option<u8>,
    pub(super) pulse_input_mode: Option<String>,
    pub(super) compare_output_mode_raw: Option<u8>,
    pub(super) compare_output_mode: Option<String>,
    pub(super) internal_preset: Option<u8>,
    pub(super) external_preset: Option<u8>,
    pub(super) ring_counter_max: Option<i32>,
    pub(super) compare_output_min: Option<i32>,
    pub(super) compare_output_max: Option<i32>,
    pub(super) unit_time_ms: Option<u16>,
    pub(super) pulses_per_revolution: Option<u16>,
    pub(super) raw_bytes: usize,
}

impl WasmHscChannelSummary {
    pub(super) fn from_channel(channel: HscChannelSummary) -> Self {
        Self {
            channel: channel.channel,
            counter_mode_raw: channel.counter_mode_raw,
            counter_mode: channel.counter_mode.map(|value| value.to_string()),
            pulse_input_mode_raw: channel.pulse_input_mode_raw,
            pulse_input_mode: channel.pulse_input_mode.map(|value| value.to_string()),
            compare_output_mode_raw: channel.compare_output_mode_raw,
            compare_output_mode: channel.compare_output_mode.map(|value| value.to_string()),
            internal_preset: channel.internal_preset,
            external_preset: channel.external_preset,
            ring_counter_max: channel.ring_counter_max,
            compare_output_min: channel.compare_output_min,
            compare_output_max: channel.compare_output_max,
            unit_time_ms: channel.unit_time_ms,
            pulses_per_revolution: channel.pulses_per_revolution,
            raw_bytes: channel.raw.len(),
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
    pub(super) parameter: Option<WasmPositionAxisParameterSummary>,
    pub(super) first_step: Option<WasmPositionStepSummary>,
}

impl WasmPositionAxisSummary {
    pub(super) fn from_axis(axis: PositionAxisSummary) -> Self {
        let parsed_steps = axis.steps.len();
        Self {
            axis_name: axis.axis_name,
            step_count: axis.step_count,
            parsed_steps,
            parameter: axis
                .parameter
                .map(WasmPositionAxisParameterSummary::from_parameter),
            first_step: axis
                .steps
                .into_iter()
                .next()
                .map(WasmPositionStepSummary::from_step),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmPositionAxisParameterSummary {
    pub(super) bias_velocity: Option<u32>,
    pub(super) velocity_limit: Option<u32>,
    pub(super) accel_times: [Option<u32>; 4],
    pub(super) decel_times: [Option<u32>; 4],
    pub(super) soft_upper_limit: Option<i32>,
    pub(super) soft_lower_limit: Option<i32>,
    pub(super) backlash_compensation: Option<i32>,
    pub(super) s_curve_ratio: Option<u32>,
    pub(super) use_limit: Option<u32>,
    pub(super) pulse_output_mode: Option<u32>,
    pub(super) orientation: Option<i32>,
    pub(super) return_velocity_high: Option<u32>,
    pub(super) return_velocity_low: Option<u32>,
    pub(super) return_accel_time: Option<u32>,
    pub(super) return_decel_time: Option<u32>,
    pub(super) return_dwell_time: Option<u32>,
    pub(super) return_policy: Option<u32>,
    pub(super) return_direction: Option<u32>,
    pub(super) jog_accel_time: Option<u32>,
    pub(super) jog_decel_time: Option<u32>,
    pub(super) inching_time: Option<u32>,
    pub(super) jog_velocity_high: Option<u32>,
    pub(super) jog_velocity_low: Option<u32>,
    pub(super) interpolation_method: Option<u32>,
}

impl WasmPositionAxisParameterSummary {
    fn from_parameter(parameter: PositionAxisParameterSummary) -> Self {
        Self {
            bias_velocity: parameter.bias_velocity,
            velocity_limit: parameter.velocity_limit,
            accel_times: parameter.accel_times,
            decel_times: parameter.decel_times,
            soft_upper_limit: parameter.soft_upper_limit,
            soft_lower_limit: parameter.soft_lower_limit,
            backlash_compensation: parameter.backlash_compensation,
            s_curve_ratio: parameter.s_curve_ratio,
            use_limit: parameter.use_limit,
            pulse_output_mode: parameter.pulse_output_mode,
            orientation: parameter.orientation,
            return_velocity_high: parameter.return_velocity_high,
            return_velocity_low: parameter.return_velocity_low,
            return_accel_time: parameter.return_accel_time,
            return_decel_time: parameter.return_decel_time,
            return_dwell_time: parameter.return_dwell_time,
            return_policy: parameter.return_policy,
            return_direction: parameter.return_direction,
            jog_accel_time: parameter.jog_accel_time,
            jog_decel_time: parameter.jog_decel_time,
            inching_time: parameter.inching_time,
            jog_velocity_high: parameter.jog_velocity_high,
            jog_velocity_low: parameter.jog_velocity_low,
            interpolation_method: parameter.interpolation_method,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmPositionStepSummary {
    pub(super) target_position: Option<i32>,
    pub(super) operation_velocity: Option<u32>,
    pub(super) dwell_time: Option<u32>,
    pub(super) operation_mode: Option<u32>,
}

impl WasmPositionStepSummary {
    fn from_step(step: PositionStepSummary) -> Self {
        Self {
            target_position: step.target_position,
            operation_velocity: step.operation_velocity,
            dwell_time: step.dwell_time,
            operation_mode: step.operation_mode,
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
    pub(super) calculation: Vec<WasmPidCalSummary>,
    pub(super) tuning: Vec<WasmPidTuneSummary>,
}

impl WasmPidSummary {
    pub(super) fn from_parameters(
        calculation: Vec<PidCalParameterSummary>,
        tuning: Vec<PidTuneParameterSummary>,
    ) -> Self {
        Self {
            cal_parameters: calculation.len(),
            tune_parameters: tuning.len(),
            cal_loops: calculation
                .iter()
                .map(|parameter| parameter.loops.len())
                .sum(),
            tune_loops: tuning.iter().map(|parameter| parameter.loops.len()).sum(),
            calculation: calculation
                .into_iter()
                .map(WasmPidCalSummary::from_parameter)
                .collect(),
            tuning: tuning
                .into_iter()
                .map(WasmPidTuneSummary::from_parameter)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmPidCalSummary {
    pub(super) header: [Option<u32>; 2],
    pub(super) parameter_size: Option<u32>,
    pub(super) set_pid_out: Option<u32>,
    pub(super) set_direction: Option<u32>,
    pub(super) prevent_anti_windup: Option<u32>,
    pub(super) proportional_control_method: Option<u32>,
    pub(super) differential_control_method: Option<u32>,
    pub(super) permit_pwm: Option<u32>,
    pub(super) loops: Vec<WasmPidCalLoopSummary>,
}

impl WasmPidCalSummary {
    fn from_parameter(parameter: PidCalParameterSummary) -> Self {
        Self {
            header: parameter.header,
            parameter_size: parameter.parameter_size,
            set_pid_out: parameter.set_pid_out,
            set_direction: parameter.set_direction,
            prevent_anti_windup: parameter.prevent_anti_windup,
            proportional_control_method: parameter.proportional_control_method,
            differential_control_method: parameter.differential_control_method,
            permit_pwm: parameter.permit_pwm,
            loops: parameter
                .loops
                .into_iter()
                .map(WasmPidCalLoopSummary::from_loop)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmPidCalLoopSummary {
    pub(super) loop_index: usize,
    pub(super) target_value: Option<i32>,
    pub(super) scan_time: Option<u32>,
    pub(super) proportional_gain: [Option<i32>; 2],
    pub(super) integral_gain: [Option<i32>; 2],
    pub(super) differential_gain: [Option<i32>; 2],
    pub(super) mv_min: Option<i32>,
    pub(super) mv_max: Option<i32>,
    pub(super) mv_manual: Option<i32>,
    pub(super) mv_limit: Option<u32>,
    pub(super) pv_min: Option<i32>,
    pub(super) pv_max: Option<i32>,
    pub(super) pv_limit: Option<u32>,
    pub(super) pv_tracking_set_value: Option<i32>,
    pub(super) dead_band: Option<i32>,
    pub(super) forward_pwm: Option<u32>,
    pub(super) forward_pwm_address: Option<String>,
    pub(super) pwm_out_period: Option<u32>,
}

impl WasmPidCalLoopSummary {
    fn from_loop(loop_data: PidCalLoopSummary) -> Self {
        Self {
            loop_index: loop_data.loop_index,
            target_value: loop_data.target_value,
            scan_time: loop_data.scan_time,
            proportional_gain: [
                loop_data.proportional_gain_left,
                loop_data.proportional_gain_right,
            ],
            integral_gain: [loop_data.integral_gain_left, loop_data.integral_gain_right],
            differential_gain: [
                loop_data.differential_gain_left,
                loop_data.differential_gain_right,
            ],
            mv_min: loop_data.mv_min,
            mv_max: loop_data.mv_max,
            mv_manual: loop_data.mv_manual,
            mv_limit: loop_data.mv_limit,
            pv_min: loop_data.pv_min,
            pv_max: loop_data.pv_max,
            pv_limit: loop_data.pv_limit,
            pv_tracking_set_value: loop_data.pv_tracking_set_value,
            dead_band: loop_data.dead_band,
            forward_pwm: loop_data.forward_pwm,
            forward_pwm_address: loop_data.forward_pwm.map(format_pid_p_address),
            pwm_out_period: loop_data.pwm_out_period,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmPidTuneSummary {
    pub(super) set_direction: Option<u32>,
    pub(super) permit_pwm: Option<u32>,
    pub(super) checksum: Option<u32>,
    pub(super) footer: [Option<u32>; 2],
    pub(super) loops: Vec<WasmPidTuneLoopSummary>,
}

impl WasmPidTuneSummary {
    fn from_parameter(parameter: PidTuneParameterSummary) -> Self {
        Self {
            set_direction: parameter.set_direction,
            permit_pwm: parameter.permit_pwm,
            checksum: parameter.checksum,
            footer: parameter.footer,
            loops: parameter
                .loops
                .into_iter()
                .map(WasmPidTuneLoopSummary::from_loop)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmPidTuneLoopSummary {
    pub(super) loop_index: usize,
    pub(super) target_value: Option<i32>,
    pub(super) scan_time: Option<u32>,
    pub(super) mv_min: Option<i32>,
    pub(super) mv_max: Option<i32>,
    pub(super) set_pwm_at_point: Option<u32>,
    pub(super) set_pwm_at_address: Option<String>,
    pub(super) out_period: Option<u32>,
    pub(super) hysteresis: Option<i32>,
}

impl WasmPidTuneLoopSummary {
    fn from_loop(loop_data: PidTuneLoopSummary) -> Self {
        Self {
            loop_index: loop_data.loop_index,
            target_value: loop_data.target_value,
            scan_time: loop_data.scan_time,
            mv_min: loop_data.mv_min,
            mv_max: loop_data.mv_max,
            set_pwm_at_point: loop_data.set_pwm_at_point,
            set_pwm_at_address: loop_data.set_pwm_at_point.map(format_pid_p_address),
            out_period: loop_data.out_period,
            hysteresis: loop_data.hysteresis,
        }
    }
}

fn format_pid_p_address(value: u32) -> String {
    let high = value / 16;
    let low = value % 16;
    format!("P{high:04}{low:X}")
}
