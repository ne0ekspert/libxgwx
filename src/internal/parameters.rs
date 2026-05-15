use super::*;

pub(crate) fn parse_hsc_channels(payload: &str, payload_bytes: &[u8]) -> Vec<HscChannelSummary> {
    const HSC_CHANNEL_COUNT: usize = 4;
    const HSC_CHANNEL_RECORD_LEN: usize = 56;
    const HSC_COUNTER_MODE_OFFSET: usize = 1;
    const HSC_PULSE_INPUT_MODE_OFFSET: usize = 5;
    const HSC_COMPARE_OUTPUT_MODE_NIBBLE_OFFSET: usize = 9;
    const HSC_INTERNAL_PRESET_OFFSET: usize = 8;
    const HSC_EXTERNAL_PRESET_OFFSET: usize = 9;
    const HSC_RING_COUNTER_MAX_OFFSET: usize = 20;
    const HSC_COMPARE_OUTPUT_MIN_OFFSET: usize = 24;
    const HSC_COMPARE_OUTPUT_MAX_OFFSET: usize = 28;
    const HSC_UNIT_TIME_MS_OFFSET: usize = 44;
    const HSC_PULSES_PER_REVOLUTION_OFFSET: usize = 46;

    (0..HSC_CHANNEL_COUNT)
        .map(|channel| {
            let counter_mode_raw = hex_nibble_at(payload, HSC_COUNTER_MODE_OFFSET + channel);
            let counter_mode = counter_mode_raw.map(HscCounterMode::from_raw);
            let pulse_input_mode_raw =
                hex_nibble_at(payload, HSC_PULSE_INPUT_MODE_OFFSET + channel);
            let pulse_input_mode = pulse_input_mode_raw.map(HscPulseInputMode::from_raw);
            let raw_start = channel * HSC_CHANNEL_RECORD_LEN;
            let raw = payload_bytes
                .get(raw_start..raw_start + HSC_CHANNEL_RECORD_LEN)
                .unwrap_or_default()
                .to_vec();
            let compare_output_mode_raw =
                hex_nibble_at(payload, HSC_COMPARE_OUTPUT_MODE_NIBBLE_OFFSET);
            let compare_output_mode = compare_output_mode_raw.map(HscCompareOutputMode::from_raw);
            let internal_preset = raw.get(HSC_INTERNAL_PRESET_OFFSET).copied();
            let external_preset = raw.get(HSC_EXTERNAL_PRESET_OFFSET).copied();
            let ring_counter_max =
                read_i32_le(&raw, HSC_RING_COUNTER_MAX_OFFSET).filter(|value| *value >= 2);
            let compare_output_min = read_i32_le(&raw, HSC_COMPARE_OUTPUT_MIN_OFFSET);
            let compare_output_max = read_i32_le(&raw, HSC_COMPARE_OUTPUT_MAX_OFFSET);
            let unit_time_ms = read_u16_le(&raw, HSC_UNIT_TIME_MS_OFFSET)
                .filter(|value| (1..=60000).contains(value));
            let pulses_per_revolution = read_u16_le(&raw, HSC_PULSES_PER_REVOLUTION_OFFSET)
                .filter(|value| (1..=60000).contains(value));

            HscChannelSummary {
                channel,
                counter_mode_raw,
                counter_mode,
                pulse_input_mode_raw,
                pulse_input_mode,
                compare_output_mode_raw,
                compare_output_mode,
                internal_preset,
                external_preset,
                ring_counter_max,
                compare_output_min,
                compare_output_max,
                unit_time_ms,
                pulses_per_revolution,
                raw,
            }
        })
        .collect()
}

pub(crate) fn hex_nibble_at(text: &str, offset: usize) -> Option<u8> {
    text.split_whitespace()
        .flat_map(str::chars)
        .nth(offset)?
        .to_digit(16)
        .and_then(|value| u8::try_from(value).ok())
}

pub(crate) fn position_axis_name(axis_index: usize) -> &'static str {
    match axis_index {
        0 => "X",
        1 => "Y",
        2 => "Z",
        3 => "U",
        _ => "Axis",
    }
}

pub(crate) fn parse_project_option_entries(raw: &str) -> Vec<ProjectOptionEntry> {
    raw.lines()
        .flat_map(|line| line.split(','))
        .filter_map(|part| {
            let part = part
                .trim()
                .trim_matches('{')
                .trim_matches('}')
                .trim_matches('"')
                .trim();
            let (key, value) = part.split_once('=').or_else(|| part.split_once(':'))?;
            let key = key.trim().trim_matches('"').to_owned();
            let value = value.trim().trim_matches('"').to_owned();
            (!key.is_empty()).then_some(ProjectOptionEntry { key, value })
        })
        .collect()
}

pub(crate) fn format_ipv4_le(value: u32) -> String {
    let bytes = value.to_le_bytes();
    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
}

pub(crate) fn ipv4_attrs(element: &XmlElement, prefix: &str) -> Option<Ipv4Summary> {
    let octets = [
        attr_u8(element, &format!("{prefix}_0"))?,
        attr_u8(element, &format!("{prefix}_1"))?,
        attr_u8(element, &format!("{prefix}_2"))?,
        attr_u8(element, &format!("{prefix}_3"))?,
    ];

    Some(Ipv4Summary {
        octets,
        address: format!("{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3]),
    })
}

pub(crate) fn ascii_char_from_u32(value: u32) -> Option<char> {
    u8::try_from(value)
        .ok()
        .filter(u8::is_ascii_graphic)
        .map(char::from)
}

pub(crate) fn format_comm_channel_address(
    channel_name: &str,
    device: Option<char>,
    addr: Option<u32>,
) -> Option<String> {
    let device = device?.to_string();
    let addr = addr?;
    let data_type = if channel_name.starts_with("Safety_Comm_A") {
        "WORD"
    } else {
        "BIT"
    };
    format_variable_address(Some(&device), Some(addr), Some(data_type), None)
}

pub(crate) fn format_cnet_io_address(
    device: Option<char>,
    addr: Option<u32>,
    kind: CnetIoKind,
) -> Option<String> {
    let device = device?.to_string();
    let data_type = match kind {
        CnetIoKind::Bit => "BIT",
        CnetIoKind::Word => "WORD",
    };
    format_variable_address(Some(&device), addr, Some(data_type), None)
}

pub(crate) fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

pub(crate) fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    let bytes = data.get(offset..offset + 2)?;
    Some(u16::from_le_bytes(bytes.try_into().ok()?))
}

pub(crate) fn read_i32_le(data: &[u8], offset: usize) -> Option<i32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(i32::from_le_bytes(bytes.try_into().ok()?))
}

pub(crate) fn format_variable_address(
    area: Option<&str>,
    number: Option<u32>,
    data_type: Option<&str>,
    source_ref: Option<&str>,
) -> Option<String> {
    let area = area?;
    let number = number?;
    if area == "U" {
        return format_u_address(number, data_type, source_ref);
    }

    let width = match area {
        "D" => 6,
        "T" | "C" | "Z" => 4,
        _ => 5,
    };

    if data_type == Some("BIT") && matches!(area, "M" | "P" | "L" | "K") {
        return Some(format_lsd_hex_address(area, number, width));
    }

    Some(format!("{area}{number:0width$}"))
}

pub(crate) fn format_lsd_hex_address(area: &str, number: u32, width: usize) -> String {
    let high = number / 16;
    let low = number % 16;
    let body_width = width.saturating_sub(1);
    format!("{area}{high:0body_width$}{low:X}")
}

pub(crate) fn format_u_address(
    number: u32,
    data_type: Option<&str>,
    source_ref: Option<&str>,
) -> Option<String> {
    let slot = parse_special_module_slot(source_ref)?;
    match data_type {
        Some("BIT") => {
            let offset = number.checked_sub(0x400).unwrap_or(number);
            let word = offset / 16;
            let bit = offset % 16;
            Some(format!("U{slot:02}.{word:02}.{bit:X}"))
        }
        _ => {
            let word = number.checked_sub(0x40).unwrap_or(number);
            Some(format!("U{slot:02}.{word:02}"))
        }
    }
}

pub(crate) fn parse_special_module_slot(source_ref: Option<&str>) -> Option<u32> {
    let mut parts = source_ref?.split(':');
    match (parts.next(), parts.next(), parts.next()) {
        (Some("SP"), Some(_base), Some(slot)) => slot.parse().ok(),
        _ => None,
    }
}
