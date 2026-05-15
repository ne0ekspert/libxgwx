use crate::*;
use std::fmt;

/// High-level metadata from the root `<Project>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInfo {
    pub name: Option<String>,
    pub attribute: Option<u32>,
    pub version: Option<u32>,
    pub comment: Option<String>,
    pub wks_node_count: Option<u32>,
    pub guid: Option<String>,
    pub file_version: Option<String>,
    pub file_last_write_time: Option<String>,
    pub attributes: Vec<XmlAttribute>,
}

impl ProjectInfo {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            name: text_value(element),
            attribute: attr_u32(element, "Attribute"),
            version: attr_u32(element, "Version"),
            comment: attr_string(element, "Comment"),
            wks_node_count: attr_u32(element, "WksNodeCount"),
            guid: attr_string(element, "GUID"),
            file_version: attr_string(element, "FileVer"),
            file_last_write_time: attr_string(element, "FileLastWriteTime"),
            attributes: element.attributes.clone(),
        }
    }
}

/// High-level summary of a `<Configuration>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationSummary {
    pub name: Option<String>,
    pub version: Option<u32>,
    pub attribute: Option<u32>,
    pub comment: Option<String>,
    pub kind: Option<u32>,
    pub type_code: Option<u32>,
    pub guid: Option<String>,
    pub write_signature: Option<String>,
    pub attributes: Vec<XmlAttribute>,
}

impl ConfigurationSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            name: text_value(element),
            version: attr_u32(element, "Version"),
            attribute: attr_u32(element, "Attribute"),
            comment: attr_string(element, "Comment"),
            kind: attr_u32(element, "Kind"),
            type_code: attr_u32(element, "Type"),
            guid: attr_string(element, "GUID"),
            write_signature: attr_string(element, "WriteSignature"),
            attributes: element.attributes.clone(),
        }
    }
}

/// High-level summary of a `<Network>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkSummary {
    pub name: Option<String>,
    pub type_name: Option<String>,
    pub network_type: Option<String>,
    pub modules: Vec<NetworkModuleSummary>,
    pub attributes: Vec<XmlAttribute>,
}

impl NetworkSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            name: attr_string(element, "Name"),
            type_name: attr_string(element, "Type"),
            network_type: attr_string(element, "NetworkType"),
            modules: element
                .children_named("NetworkModule")
                .map(NetworkModuleSummary::from_element)
                .collect(),
            attributes: element.attributes.clone(),
        }
    }
}

/// High-level summary of a `<NetworkModule>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkModuleSummary {
    pub config_name: Option<String>,
    pub config_type: Option<u32>,
    pub type_name: Option<String>,
    pub name: Option<String>,
    pub base: Option<u32>,
    pub slot: Option<u32>,
    pub id: Option<u32>,
    pub channel_type: Option<u32>,
    pub option_type: Option<u32>,
    pub alias: Option<String>,
    pub description: Option<String>,
    pub attributes: Vec<XmlAttribute>,
}

impl NetworkModuleSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            config_name: attr_string(element, "ConfigName"),
            config_type: attr_u32(element, "ConfigType"),
            type_name: attr_string(element, "Type"),
            name: attr_string(element, "Name"),
            base: attr_u32(element, "Base"),
            slot: attr_u32(element, "Slot"),
            id: attr_u32(element, "Id"),
            channel_type: attr_u32(element, "ChannelType"),
            option_type: attr_u32(element, "OptionType"),
            alias: attr_string(element, "Alias"),
            description: attr_string(element, "Description"),
            attributes: element.attributes.clone(),
        }
    }
}

/// High-level summary of a hardware `<Base>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseSummary {
    pub base: Option<u32>,
    pub slot_count: Option<u32>,
    pub attributes: Vec<XmlAttribute>,
}

impl BaseSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            base: attr_u32(element, "Base"),
            slot_count: attr_u32(element, "SlotCount"),
            attributes: element.attributes.clone(),
        }
    }
}

/// High-level summary of a hardware `<Module>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleSummary {
    pub base: Option<u32>,
    pub slot: Option<u32>,
    pub id: Option<u32>,
    pub sub_type: Option<u32>,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub details: Option<String>,
    pub attributes: Vec<XmlAttribute>,
}

impl ModuleSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            base: attr_u32(element, "Base"),
            slot: attr_u32(element, "Slot"),
            id: attr_u32(element, "Id"),
            sub_type: attr_u32(element, "SubType"),
            name: attr_string(element, "Name"),
            comment: attr_string(element, "Comment"),
            details: attr_string(element, "Details"),
            attributes: element.attributes.clone(),
        }
    }
}

/// High-level summary of a `<Task>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSummary {
    pub name: Option<String>,
    pub version: Option<u32>,
    pub type_code: Option<u32>,
    pub attribute: Option<u32>,
    pub kind: Option<u32>,
    pub priority: Option<u32>,
    pub task_index: Option<u32>,
    pub device: Option<String>,
    pub device_type: Option<u32>,
    pub word_value: Option<u32>,
    pub word_condition: Option<u32>,
    pub bit_condition: Option<u32>,
    pub attributes: Vec<XmlAttribute>,
}

impl TaskSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            name: text_value(element),
            version: attr_u32(element, "Version"),
            type_code: attr_u32(element, "Type"),
            attribute: attr_u32(element, "Attribute"),
            kind: attr_u32(element, "Kind"),
            priority: attr_u32(element, "Priority"),
            task_index: attr_u32(element, "TaskIndex"),
            device: attr_string(element, "Device"),
            device_type: attr_u32(element, "DeviceType"),
            word_value: attr_u32(element, "WordValue"),
            word_condition: attr_u32(element, "WordCondition"),
            bit_condition: attr_u32(element, "BitCondition"),
            attributes: element.attributes.clone(),
        }
    }
}

/// High-level summary of a `<Program>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramSummary {
    pub name: Option<String>,
    pub task: Option<String>,
    pub version: Option<u32>,
    pub local_variable: Option<u32>,
    pub kind: Option<u32>,
    pub instance_name: Option<String>,
    pub comment: Option<String>,
    pub object_id: Option<String>,
    pub find_program: Option<u32>,
    pub find_var: Option<u32>,
    pub encryption: Option<String>,
    pub attributes: Vec<XmlAttribute>,
}

impl ProgramSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            name: text_value(element),
            task: attr_string(element, "Task"),
            version: attr_u32(element, "Version"),
            local_variable: attr_u32(element, "LocalVariable"),
            kind: attr_u32(element, "Kind"),
            instance_name: attr_string(element, "InstanceName"),
            comment: attr_string(element, "Comment"),
            object_id: attr_string(element, "ObjectId"),
            find_program: attr_u32(element, "FindProgram"),
            find_var: attr_u32(element, "FindVar"),
            encryption: attr_string(element, "Encrytption"),
            attributes: element.attributes.clone(),
        }
    }
}

/// High-level summary of one decoded variable symbol record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableSummary {
    pub format_version: Option<String>,
    pub name: Option<String>,
    /// Deprecated compatibility alias. Prefer [`VariableSummary::data_type`].
    pub type_name: Option<String>,
    /// Deprecated compatibility alias. Prefer [`VariableSummary::address`].
    pub scope: Option<String>,
    pub address_area: Option<String>,
    pub address_number: Option<u32>,
    pub data_type: Option<String>,
    pub description: Option<String>,
    /// Deprecated compatibility alias. Prefer [`VariableSummary::description`].
    pub comment: Option<String>,
    pub address: Option<String>,
    pub source_ref: Option<String>,
    pub range: Option<String>,
}

impl VariableSummary {
    pub(crate) fn from_symbols_element(symbols: &XmlElement) -> Result<Vec<Self>, XgwxError> {
        let compressed = attr_bool(symbols, "Compressed").unwrap_or(false);
        let data = decode_base64_payload(&symbols.text, compressed)?;
        let strings = extract_utf16_marker_strings(&data, false, true);
        let starts = strings
            .iter()
            .enumerate()
            .filter_map(|(index, string)| (string.value == "SV5.0").then_some(index))
            .collect::<Vec<_>>();

        let mut variables = Vec::with_capacity(starts.len());
        for (start_index, start) in starts.iter().copied().enumerate() {
            let end = starts
                .get(start_index + 1)
                .copied()
                .unwrap_or(strings.len());
            let record_strings = &strings[start..end];
            let fields = record_strings
                .iter()
                .map(|string| string.value.clone())
                .collect::<Vec<_>>();
            let address_area = fields.get(2).cloned();
            let data_type = fields.get(3).cloned();
            let source_ref = fields.get(5).cloned();
            let address_number = record_strings
                .get(2)
                .and_then(|string| read_u32_le(&data, string.end_offset));

            variables.push(Self {
                format_version: fields.first().cloned(),
                name: fields.get(1).cloned(),
                type_name: data_type.clone(),
                scope: address_area.clone(),
                address_area: address_area.clone(),
                address_number,
                data_type: data_type.clone(),
                description: fields.get(4).cloned(),
                comment: fields.get(4).cloned(),
                address: format_variable_address(
                    address_area.as_deref(),
                    address_number,
                    data_type.as_deref(),
                    source_ref.as_deref(),
                ),
                source_ref,
                range: fields.get(6).cloned(),
            });
        }

        Ok(variables)
    }
}

/// Partial decode of a ladder `<ProgramData>` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderProgramData {
    pub program_name: Option<String>,
    pub version: Option<String>,
    pub project_type: Option<u32>,
    pub eno_control_option: Option<u32>,
    pub compressed: bool,
    pub encoded_len: usize,
    pub decoded_len: usize,
    pub data: Vec<u8>,
    pub strings: Vec<LadderString>,
    pub elements: Vec<LadderElement>,
    pub structure: LadderStructure,
    pub instructions: Vec<LadderInstruction>,
}

impl LadderProgramData {
    pub(crate) fn from_program_element(program: &XmlElement) -> Result<Self, XgwxError> {
        let program_data = program
            .descendants_named("ProgramData")
            .next()
            .ok_or(XgwxError::MissingProgramData)?;
        let compressed = attr_bool(program_data, "Compressed").unwrap_or(false);
        let encoded = program_data.text.split_whitespace().collect::<String>();
        let data = decode_base64_payload(&program_data.text, compressed)?;
        let strings = extract_ladder_strings(&data);
        let elements = extract_ladder_elements(&data, &strings);
        let structure = extract_ladder_structure(&data, &elements);
        let instructions = extract_ladder_instructions(&strings);

        Ok(Self {
            program_name: text_value(program),
            version: attr_string(program_data, "Version"),
            project_type: attr_u32(program_data, "ProjectType"),
            eno_control_option: attr_u32(program_data, "ENOControlOption"),
            compressed,
            encoded_len: encoded.len(),
            decoded_len: data.len(),
            data,
            strings,
            elements,
            structure,
            instructions,
        })
    }
}

/// UTF-16 string extracted from decoded ladder binary data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderString {
    pub offset: usize,
    pub end_offset: usize,
    pub value: String,
}

/// Best-effort ladder element extracted from decoded ladder binary strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderElement {
    pub offset: usize,
    pub kind: LadderElementKind,
    pub value: String,
    pub operands: Vec<String>,
    pub contact: Option<LadderContact>,
    pub coil: Option<LadderCoil>,
}

/// Best-effort ladder structure reconstructed from decoded LD cell coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderStructure {
    pub rungs: Vec<LadderRung>,
    pub vertical_lines: Vec<LadderVerticalLine>,
    pub branch_groups: Vec<LadderBranchGroup>,
    pub horizontal_lines: Vec<LadderHorizontalLine>,
    pub rung_comments: Vec<LadderRungComment>,
    pub output_comments: Vec<LadderOutputComment>,
    pub unknown_records: Vec<LadderUnknownRecord>,
}

/// One reconstructed LD row/rung.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderRung {
    pub raw_y: u8,
    pub cells: Vec<LadderCell>,
}

/// One positioned LD cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderCell {
    pub offset: usize,
    pub raw_x: u8,
    pub raw_y: u8,
    pub kind: LadderElementKind,
    pub value: String,
    pub operands: Vec<String>,
    pub contact: Option<LadderContact>,
    pub coil: Option<LadderCoil>,
}

/// One decoded vertical LD connection segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderVerticalLine {
    pub raw_x: u8,
    pub raw_y_start: u8,
    pub raw_y_end: u8,
}

/// One continuous vertical LD branch group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderBranchGroup {
    pub raw_x: u8,
    pub raw_y_start: u8,
    pub raw_y_end: u8,
}

/// One decoded horizontal LD connection segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderHorizontalLine {
    pub raw_y: u8,
    pub raw_x_start: u8,
    pub raw_x_end: u8,
}

/// One decoded rung comment spanning the LD code area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderRungComment {
    pub offset: usize,
    pub raw_x: u8,
    pub raw_y: u8,
    pub text: String,
}

/// One decoded output comment positioned at the right side of an LD rung.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderOutputComment {
    pub offset: usize,
    pub raw_x: u8,
    pub raw_y: u8,
    pub text: String,
}

/// Positioned raw LD record whose semantics are not decoded yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderUnknownRecord {
    pub offset: usize,
    pub marker: [u8; 2],
    pub raw_x: u8,
    pub raw_y: u8,
    pub bytes: Vec<u8>,
}

/// Best-effort classification for a decoded ladder element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LadderElementKind {
    InstructionCall,
    Operation,
    Comparison,
    Timer,
    Logic,
    DeviceRef,
    InternalRef,
    Constant,
    Comment,
}

/// Contact polarity decoded from LD contact records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LadderContact {
    NormallyOpen,
    NormallyClosed,
    Inverse,
    RisingPulse,
    FallingPulse,
    AddressedRisingPulse,
    AddressedRisingPulseNot,
    AddressedFallingPulse,
    AddressedFallingPulseNot,
}

/// Coil kind decoded from LD output coil records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LadderCoil {
    Output,
    Inverse,
    Set,
    Reset,
    RisingPulse,
    FallingPulse,
}

/// Best-effort instruction call extracted from decoded ladder strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderInstruction {
    pub offset: usize,
    pub mnemonic: String,
    pub operands: Vec<String>,
    pub raw: String,
}

/// High-level summary of an `<XGPD_HS_LINK>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighSpeedLinkSummary {
    pub self_station_no: Option<u32>,
    pub type_code: Option<u32>,
    pub base: Option<u32>,
    pub slot: Option<u32>,
    pub index: Option<u32>,
    pub tx_rx_cycle: Option<u32>,
    pub link_type: Option<String>,
    pub status_flag_area: Option<String>,
    pub block_info_count: Option<u32>,
    pub blocks: Vec<HighSpeedLinkBlockSummary>,
    pub attributes: Vec<XmlAttribute>,
}

impl HighSpeedLinkSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            self_station_no: attr_u32(element, "SelfStationNo"),
            type_code: attr_u32(element, "Type"),
            base: attr_u32(element, "Base"),
            slot: attr_u32(element, "Slot"),
            index: attr_u32(element, "Index"),
            tx_rx_cycle: attr_u32(element, "TxRXCycle"),
            link_type: attr_string(element, "LinkType"),
            status_flag_area: attr_string(element, "StatusFlagArea"),
            block_info_count: attr_u32(element, "BlockInfoCount"),
            blocks: element
                .children_named("XGPD_HS_LINK_BLK")
                .map(HighSpeedLinkBlockSummary::from_element)
                .collect(),
            attributes: element.attributes.clone(),
        }
    }
}

/// High-level summary of an `<XGPD_HS_LINK_BLK>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighSpeedLinkBlockSummary {
    pub block_index: Option<u32>,
    pub block_id: Option<u32>,
    pub master_slave_type: Option<u32>,
    pub station_no: Option<u32>,
    pub tx_rx_type: Option<u32>,
    pub link_type: Option<u32>,
    pub station_type: Option<u32>,
    pub module_select_type_rnet: Option<u32>,
    pub tx_size: Option<u32>,
    pub rx_size: Option<u32>,
    pub master_no: Option<i32>,
    pub self_station_no: Option<i32>,
    pub new_blk_dpnet: Option<u32>,
    pub block_use: Option<u32>,
    pub conn_type: Option<String>,
    pub data_type: Option<String>,
    pub slave_type: Option<String>,
    pub tx_address: Option<String>,
    pub rx_address: Option<String>,
    pub tx_data_type: Option<String>,
    pub rx_data_type: Option<String>,
    pub tx_var_scope: Option<String>,
    pub rx_var_scope: Option<String>,
    pub module_type: Option<String>,
    pub remote_io: Option<u32>,
    pub attributes: Vec<XmlAttribute>,
}

impl HighSpeedLinkBlockSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            block_index: attr_u32(element, "BlockIndex"),
            block_id: attr_u32(element, "BlockId"),
            master_slave_type: attr_u32(element, "MasterSlaveType"),
            station_no: attr_u32(element, "StationNo"),
            tx_rx_type: attr_u32(element, "TxRxType"),
            link_type: attr_u32(element, "LinkType"),
            station_type: attr_u32(element, "StationType"),
            module_select_type_rnet: attr_u32(element, "ModuleSelectTypeRnet"),
            tx_size: attr_u32(element, "TxSize"),
            rx_size: attr_u32(element, "RxSize"),
            master_no: attr_i32(element, "MasterNO"),
            self_station_no: attr_i32(element, "SelfStationNO"),
            new_blk_dpnet: attr_u32(element, "NewBlkDPnet"),
            block_use: attr_u32(element, "BlockUse"),
            conn_type: attr_string(element, "ConnType"),
            data_type: attr_string(element, "DataType"),
            slave_type: attr_string(element, "SlaveType"),
            tx_address: attr_string(element, "TxAddress"),
            rx_address: attr_string(element, "RxAddress"),
            tx_data_type: attr_string(element, "TxDataType"),
            rx_data_type: attr_string(element, "RxDataType"),
            tx_var_scope: attr_string(element, "TxVarScope"),
            rx_var_scope: attr_string(element, "RxVarScope"),
            module_type: attr_string(element, "ModuleType"),
            remote_io: attr_u32(element, "RemoteIO"),
            attributes: element.attributes.clone(),
        }
    }
}

/// Project options parsed from the `<Options Details="...">` text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectOptionsSummary {
    pub raw_details: Option<String>,
    pub entries: Vec<ProjectOptionEntry>,
    pub attributes: Vec<XmlAttribute>,
}

impl ProjectOptionsSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        let raw_details = attr_string(element, "Details");
        let entries = raw_details
            .as_deref()
            .map(parse_project_option_entries)
            .unwrap_or_default();

        Self {
            raw_details,
            entries,
            attributes: element.attributes.clone(),
        }
    }
}

/// One parsed `key=value` option entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectOptionEntry {
    pub key: String,
    pub value: String,
}

/// High-level summary of a `<Parameter>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterSummary {
    pub parameter_type: Option<String>,
    pub sections: Vec<XmlSectionSummary>,
    pub attributes: Vec<XmlAttribute>,
}

impl ParameterSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            parameter_type: attr_string(element, "Type"),
            sections: element
                .children
                .iter()
                .map(XmlSectionSummary::from_element)
                .collect(),
            attributes: element.attributes.clone(),
        }
    }
}

/// Parsed payload from an XGB `HSC PARAMETER` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HscParameterSummary {
    /// Declared hex-ASCII payload length from `PAYLOAD_ASC_LENGTH`.
    pub payload_asc_length: Option<u32>,
    /// Raw hex-ASCII `PAYLOAD` string.
    pub payload: String,
    /// Decoded bytes from [`HscParameterSummary::payload`].
    pub payload_bytes: Vec<u8>,
    /// The first observed hex nibble before the four channel counter modes.
    pub initial_unknown_nibble: Option<u8>,
    /// Decoded per-channel records. Observed XGB HSC payloads contain four.
    pub channels: Vec<HscChannelSummary>,
    /// Raw XML attributes from the source `<Parameter>` element.
    pub attributes: Vec<XmlAttribute>,
}

impl HscParameterSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Result<Self, XgwxError> {
        let payload = attr_string(element, "PAYLOAD").unwrap_or_default();
        let payload_bytes = decode_hex_ascii_payload(&payload, "Parameter", "PAYLOAD")?;
        let initial_unknown_nibble = hex_nibble_at(&payload, 0);
        let channels = parse_hsc_channels(&payload, &payload_bytes);

        Ok(Self {
            payload_asc_length: attr_u32(element, "PAYLOAD_ASC_LENGTH"),
            payload,
            payload_bytes,
            initial_unknown_nibble,
            channels,
            attributes: element.attributes.clone(),
        })
    }
}

/// One high-speed counter channel record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HscChannelSummary {
    /// Zero-based channel index.
    pub channel: usize,
    /// Raw counter mode nibble.
    pub counter_mode_raw: Option<u8>,
    /// Decoded counter mode.
    pub counter_mode: Option<HscCounterMode>,
    /// Raw pulse input mode nibble.
    pub pulse_input_mode_raw: Option<u8>,
    /// Decoded pulse input mode.
    pub pulse_input_mode: Option<HscPulseInputMode>,
    /// Raw compare output mode nibble.
    pub compare_output_mode_raw: Option<u8>,
    /// Decoded compare output mode.
    pub compare_output_mode: Option<HscCompareOutputMode>,
    /// Tentative byte field observed after the pulse input mode group.
    pub internal_preset: Option<u8>,
    /// Tentative byte field observed after [`HscChannelSummary::internal_preset`].
    pub external_preset: Option<u8>,
    /// Tentative 32-bit value observed as `0xC8` (`200`) for channel 0.
    ///
    /// Observed UI range is `2..=i32::MAX`.
    pub ring_counter_max: Option<i32>,
    /// Tentative `Compare Output Minimum Set Value` field.
    pub compare_output_min: Option<i32>,
    /// Tentative `Compare Output Maximum Set Value` field.
    pub compare_output_max: Option<i32>,
    /// Unit time in milliseconds. Observed UI range is `1..=60000`.
    pub unit_time_ms: Option<u16>,
    /// Pulses per revolution. Observed UI range is `1..=60000`.
    pub pulses_per_revolution: Option<u16>,
    /// The currently uninterpreted 56-byte channel record, if present.
    pub raw: Vec<u8>,
}

/// High-speed counter mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HscCounterMode {
    /// Linear counter mode (`0`).
    LinearCounter,
    /// Ring counter mode (`1`).
    RingCounter,
    /// Unrecognized raw mode value.
    Unknown(u8),
}

impl HscCounterMode {
    pub(crate) fn from_raw(value: u8) -> Self {
        match value {
            0 => Self::LinearCounter,
            1 => Self::RingCounter,
            value => Self::Unknown(value),
        }
    }
}

impl fmt::Display for HscCounterMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LinearCounter => write!(f, "Linear Counter"),
            Self::RingCounter => write!(f, "Ring Counter"),
            Self::Unknown(value) => write!(f, "Unknown ({value})"),
        }
    }
}

/// High-speed counter pulse input mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HscPulseInputMode {
    /// 1-phase, 1-input, 1x mode (`0`).
    OnePhaseOneInputOneX,
    /// 1-phase, 2-input, 1x mode (`1`).
    OnePhaseTwoInputOneX,
    /// CW/CCW pulse input mode (`2`).
    CwCcw,
    /// 2-phase, 4x mode (`3`).
    TwoPhaseFourX,
    /// Unrecognized raw mode value.
    Unknown(u8),
}

impl HscPulseInputMode {
    pub(crate) fn from_raw(value: u8) -> Self {
        match value {
            0 => Self::OnePhaseOneInputOneX,
            1 => Self::OnePhaseTwoInputOneX,
            2 => Self::CwCcw,
            3 => Self::TwoPhaseFourX,
            value => Self::Unknown(value),
        }
    }
}

impl fmt::Display for HscPulseInputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OnePhaseOneInputOneX => write!(f, "1-Phase 1-Input 1x"),
            Self::OnePhaseTwoInputOneX => write!(f, "1-Phase 2-Input 1x"),
            Self::CwCcw => write!(f, "CW/CCW"),
            Self::TwoPhaseFourX => write!(f, "2-Phase 4x"),
            Self::Unknown(value) => write!(f, "Unknown ({value})"),
        }
    }
}

/// Tentative high-speed counter compare output mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HscCompareOutputMode {
    /// Less than comparison (`0`).
    LessThan,
    /// Less than or equal comparison (`1`).
    LessOrEqual,
    /// Equal comparison (`2`).
    Equal,
    /// Greater than or equal comparison (`3`).
    GreaterOrEqual,
    /// Greater than comparison (`4`).
    GreaterThan,
    /// Inclusive range comparison (`5`).
    Includes,
    /// Exclusive range comparison (`6`).
    Excludes,
    /// Unrecognized raw mode value.
    Unknown(u8),
}

impl HscCompareOutputMode {
    pub(crate) fn from_raw(value: u8) -> Self {
        match value {
            0 => Self::LessThan,
            1 => Self::LessOrEqual,
            2 => Self::Equal,
            3 => Self::GreaterOrEqual,
            4 => Self::GreaterThan,
            5 => Self::Includes,
            6 => Self::Excludes,
            value => Self::Unknown(value),
        }
    }
}

impl fmt::Display for HscCompareOutputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LessThan => write!(f, "Less Than"),
            Self::LessOrEqual => write!(f, "Less Or Equal"),
            Self::Equal => write!(f, "Equal"),
            Self::GreaterOrEqual => write!(f, "Greater Or Equal"),
            Self::GreaterThan => write!(f, "Greater Than"),
            Self::Includes => write!(f, "Includes"),
            Self::Excludes => write!(f, "Excludes"),
            Self::Unknown(value) => write!(f, "Unknown ({value})"),
        }
    }
}

/// Parsed X/Y axis data from a `POSITION PARAMETER` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionParameterSummary {
    /// Declared axis count from `NUM_AXIS_DATA`.
    pub axis_count: Option<u32>,
    /// Parsed axis records, paired from axis data and axis parameter children.
    pub axes: Vec<PositionAxisSummary>,
    /// Raw XML attributes from `HSC_PRM_XGB_AXIS_GROUP`.
    pub attributes: Vec<XmlAttribute>,
}

impl PositionParameterSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        let group = element.children_named("HSC_PRM_XGB_AXIS_GROUP").next();
        let axis_count = group.and_then(|group| attr_u32(group, "NUM_AXIS_DATA"));
        let attributes = group
            .map(|group| group.attributes.clone())
            .unwrap_or_default();
        let Some(group) = group else {
            return Self {
                axis_count,
                axes: Vec::new(),
                attributes,
            };
        };

        let axis_data = group
            .children_named("HSC_PRM_XGB_AXIS_DATA")
            .collect::<Vec<_>>();
        let axis_params = group
            .children_named("HSC_PRM_XGB_AXIS_PARAM")
            .collect::<Vec<_>>();
        let axis_len = axis_data.len().max(axis_params.len());
        let axes = (0..axis_len)
            .map(|index| {
                PositionAxisSummary::from_elements(
                    index,
                    axis_data.get(index).copied(),
                    axis_params.get(index).copied(),
                )
            })
            .collect();

        Self {
            axis_count,
            axes,
            attributes,
        }
    }
}

/// One position-control axis. The observed XGB samples contain X and Y axes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionAxisSummary {
    /// Zero-based axis index.
    pub axis_index: usize,
    /// Conventional axis name for the index (`X`, `Y`, `Z`, `U`, or `Axis`).
    pub axis_name: String,
    /// Declared number of step table rows.
    pub step_count: Option<u32>,
    /// Parsed step table rows for this axis.
    pub steps: Vec<PositionStepSummary>,
    /// Shared axis parameters, if the matching parameter node exists.
    pub parameter: Option<PositionAxisParameterSummary>,
    /// Raw XML attributes from the axis data node.
    pub attributes: Vec<XmlAttribute>,
}

impl PositionAxisSummary {
    fn from_elements(
        axis_index: usize,
        axis_data: Option<&XmlElement>,
        axis_param: Option<&XmlElement>,
    ) -> Self {
        let steps = axis_data
            .map(|axis_data| {
                axis_data
                    .children_named("HSC_PRM_XGB_AXIS_STEP_DATA")
                    .enumerate()
                    .map(|(step_index, step)| PositionStepSummary::from_element(step_index, step))
                    .collect()
            })
            .unwrap_or_default();

        Self {
            axis_index,
            axis_name: position_axis_name(axis_index).to_owned(),
            step_count: axis_data.and_then(|axis_data| attr_u32(axis_data, "NUM_STEP_DATA")),
            steps,
            parameter: axis_param.map(PositionAxisParameterSummary::from_element),
            attributes: axis_data
                .map(|axis_data| axis_data.attributes.clone())
                .unwrap_or_default(),
        }
    }
}

/// One position table step for an axis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionStepSummary {
    /// Zero-based step index within the axis.
    pub step_index: usize,
    /// Target position for this step.
    pub target_position: Option<i32>,
    pub interpolation: Option<i32>,
    /// M-code associated with the step.
    pub m_code: Option<i32>,
    /// Dwell time at the step.
    pub dwell_time: Option<u32>,
    /// Operation velocity for the step.
    pub operation_velocity: Option<u32>,
    pub iteration_step: Option<u32>,
    /// Raw `ENUM_OperationMode` value.
    pub operation_mode: Option<u32>,
    /// Raw `ENUM_ControlMode` value.
    pub control_mode: Option<u32>,
    pub operation_step: Option<u32>,
    pub coordination: Option<u32>,
    pub interpolation_mode: Option<u32>,
    pub accel_preset: Option<u32>,
    /// Raw XML attributes from the step node.
    pub attributes: Vec<XmlAttribute>,
}

impl PositionStepSummary {
    pub(crate) fn from_element(step_index: usize, element: &XmlElement) -> Self {
        Self {
            step_index,
            target_position: attr_i32(element, "TargetPosition"),
            interpolation: attr_i32(element, "Interpolation"),
            m_code: attr_i32(element, "MCode"),
            dwell_time: attr_u32(element, "DwellTime"),
            operation_velocity: attr_u32(element, "OperationVelocity"),
            iteration_step: attr_u32(element, "IterationStep"),
            operation_mode: attr_u32(element, "ENUM_OperationMode"),
            control_mode: attr_u32(element, "ENUM_ControlMode"),
            operation_step: attr_u32(element, "ENUM_OperationStep"),
            coordination: attr_u32(element, "ENUM_Coordination"),
            interpolation_mode: attr_u32(element, "ENUM_Interpolation"),
            accel_preset: attr_u32(element, "ENUM_AccelPreset"),
            attributes: element.attributes.clone(),
        }
    }
}

/// Shared position-control parameters used by each axis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionAxisParameterSummary {
    /// Bias/start velocity.
    pub bias_velocity: Option<u32>,
    /// Maximum allowed velocity.
    pub velocity_limit: Option<u32>,
    /// Four acceleration presets from `AccelTime_0..=3`.
    pub accel_times: [Option<u32>; 4],
    /// Four deceleration presets from `DeAccelTime_0..=3`.
    pub decel_times: [Option<u32>; 4],
    /// Software upper travel limit.
    pub soft_upper_limit: Option<i32>,
    /// Software lower travel limit.
    pub soft_lower_limit: Option<i32>,
    pub backlash_compensation: Option<i32>,
    pub s_curve_ratio: Option<u32>,
    pub shape: Option<u32>,
    pub m_code: Option<u32>,
    pub detect_soft_limit: Option<u32>,
    pub display_position: Option<u32>,
    pub use_position: Option<u32>,
    pub pulse_output_level: Option<u32>,
    pub use_limit: Option<u32>,
    pub pulse_output_mode: Option<u32>,
    pub orientation: Option<i32>,
    pub return_velocity_high: Option<u32>,
    pub return_velocity_low: Option<u32>,
    pub return_accel_time: Option<u32>,
    pub return_decel_time: Option<u32>,
    pub return_dwell_time: Option<u32>,
    pub return_policy: Option<u32>,
    pub return_direction: Option<u32>,
    pub jog_accel_time: Option<u32>,
    pub jog_decel_time: Option<u32>,
    pub inching_time: Option<u32>,
    pub jog_velocity_high: Option<u32>,
    pub jog_velocity_low: Option<u32>,
    pub interpolation_method: Option<u32>,
    pub attributes: Vec<XmlAttribute>,
}

impl PositionAxisParameterSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            bias_velocity: attr_u32(element, "BiasVelocity"),
            velocity_limit: attr_u32(element, "VelocityLimit"),
            accel_times: [
                attr_u32(element, "AccelTime_0"),
                attr_u32(element, "AccelTime_1"),
                attr_u32(element, "AccelTime_2"),
                attr_u32(element, "AccelTime_3"),
            ],
            decel_times: [
                attr_u32(element, "DeAccelTime_0"),
                attr_u32(element, "DeAccelTime_1"),
                attr_u32(element, "DeAccelTime_2"),
                attr_u32(element, "DeAccelTime_3"),
            ],
            soft_upper_limit: attr_i32(element, "SoftUpperLimit"),
            soft_lower_limit: attr_i32(element, "SoftLowerLimit"),
            backlash_compensation: attr_i32(element, "BacklashCompensation"),
            s_curve_ratio: attr_u32(element, "SCurveRatio"),
            shape: attr_u32(element, "ENUM_Shape"),
            m_code: attr_u32(element, "ENUM_MCode"),
            detect_soft_limit: attr_u32(element, "ENUM_DectSoftLimit"),
            display_position: attr_u32(element, "ENUM_DisplayPosition"),
            use_position: attr_u32(element, "ENUM_UsePosition"),
            pulse_output_level: attr_u32(element, "ENUM_PulseOutputLevel"),
            use_limit: attr_u32(element, "ENUM_UseLimit"),
            pulse_output_mode: attr_u32(element, "ENUM_PulseOutputMode"),
            orientation: attr_i32(element, "Orientation"),
            return_velocity_high: attr_u32(element, "ReturnVelocityHigh"),
            return_velocity_low: attr_u32(element, "ReturnVelocityLow"),
            return_accel_time: attr_u32(element, "ReturnAccelTime"),
            return_decel_time: attr_u32(element, "ReturnDeAccelTime"),
            return_dwell_time: attr_u32(element, "ReturnDwellTime"),
            return_policy: attr_u32(element, "ENUM_ReturnPolicy"),
            return_direction: attr_u32(element, "ENUM_ReturnDirection"),
            jog_accel_time: attr_u32(element, "JogAccelTime"),
            jog_decel_time: attr_u32(element, "JogDeAccelTime"),
            inching_time: attr_u32(element, "InchingTime"),
            jog_velocity_high: attr_u32(element, "JogVelocityHigh"),
            jog_velocity_low: attr_u32(element, "JogVelocityLow"),
            interpolation_method: attr_u32(element, "ENUM_InterpolationMethod"),
            attributes: element.attributes.clone(),
        }
    }
}

/// Embedded PID calculation parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PidCalParameterSummary {
    /// Raw header words from `Header_0` and `Header_1`.
    pub header: [Option<u32>; 2],
    pub parameter_size: Option<u32>,
    pub set_pid_out: Option<u32>,
    pub set_direction: Option<u32>,
    pub prevent_anti_windup: Option<u32>,
    pub proportional_control_method: Option<u32>,
    pub differential_control_method: Option<u32>,
    pub permit_pwm: Option<u32>,
    /// Parsed PID calculation loop records.
    pub loops: Vec<PidCalLoopSummary>,
    /// Raw XML attributes from the PID calculation node.
    pub attributes: Vec<XmlAttribute>,
}

impl PidCalParameterSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        let loops = element
            .descendants_named("EmbedPidLoopData")
            .enumerate()
            .map(|(loop_index, loop_data)| PidCalLoopSummary::from_element(loop_index, loop_data))
            .collect();

        Self {
            header: [attr_u32(element, "Header_0"), attr_u32(element, "Header_1")],
            parameter_size: attr_u32(element, "ParaSize"),
            set_pid_out: attr_u32(element, "SetPidOut"),
            set_direction: attr_u32(element, "SetDirection"),
            prevent_anti_windup: attr_u32(element, "PreventAntiWindup"),
            proportional_control_method: attr_u32(element, "ProControlCalMethod"),
            differential_control_method: attr_u32(element, "DiffControlCalMethod"),
            permit_pwm: attr_u32(element, "PermitPWM"),
            loops,
            attributes: element.attributes.clone(),
        }
    }
}

/// One embedded PID calculation loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PidCalLoopSummary {
    /// Zero-based loop index.
    pub loop_index: usize,
    /// Set value/target value.
    pub target_value: Option<i32>,
    /// Scan time for this loop.
    pub scan_time: Option<u32>,
    pub proportional_gain_left: Option<i32>,
    pub proportional_gain_right: Option<i32>,
    pub integral_gain_left: Option<i32>,
    pub integral_gain_right: Option<i32>,
    pub differential_gain_left: Option<i32>,
    pub differential_gain_right: Option<i32>,
    pub pv_limit: Option<u32>,
    pub mv_limit: Option<u32>,
    pub mv_max: Option<i32>,
    pub mv_min: Option<i32>,
    pub mv_manual: Option<i32>,
    pub dead_band: Option<i32>,
    pub differential_delay_filter: Option<u32>,
    /// Raw PWM point value. This is a P-area bit offset in LSD-hex notation.
    pub forward_pwm: Option<u32>,
    /// PWM output period.
    pub pwm_out_period: Option<u32>,
    pub set_sv_average: Option<u32>,
    pub pv_tracking_set_value: Option<i32>,
    pub pv_min: Option<i32>,
    pub pv_max: Option<i32>,
    pub attributes: Vec<XmlAttribute>,
}

impl PidCalLoopSummary {
    pub(crate) fn from_element(loop_index: usize, element: &XmlElement) -> Self {
        Self {
            loop_index,
            target_value: attr_i32(element, "TargetValue"),
            scan_time: attr_u32(element, "ScanTime"),
            proportional_gain_left: attr_i32(element, "ProPortionGainDotLeft"),
            proportional_gain_right: attr_i32(element, "ProPortionGainDotRight"),
            integral_gain_left: attr_i32(element, "IntergralGainDotLeft"),
            integral_gain_right: attr_i32(element, "IntergralGainDotRight"),
            differential_gain_left: attr_i32(element, "DifferentialGainDotLeft"),
            differential_gain_right: attr_i32(element, "DifferentialGainDotRight"),
            pv_limit: attr_u32(element, "PVLimit"),
            mv_limit: attr_u32(element, "MVLimit"),
            mv_max: attr_i32(element, "MVMax"),
            mv_min: attr_i32(element, "MVMin"),
            mv_manual: attr_i32(element, "MVMan"),
            dead_band: attr_i32(element, "DeadBand"),
            differential_delay_filter: attr_u32(element, "DiffDelayFilter"),
            forward_pwm: attr_u32(element, "ForwardPWM"),
            pwm_out_period: attr_u32(element, "PWMOutPeriod"),
            set_sv_average: attr_u32(element, "SetSVSetAverage"),
            pv_tracking_set_value: attr_i32(element, "PVTrackingSetValue"),
            pv_min: attr_i32(element, "PVMin"),
            pv_max: attr_i32(element, "PVMax"),
            attributes: element.attributes.clone(),
        }
    }
}

/// Embedded PID tuning parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PidTuneParameterSummary {
    pub set_direction: Option<u32>,
    pub permit_pwm: Option<u32>,
    pub checksum: Option<u32>,
    /// Raw footer words from `Footer_0` and `Footer_1`.
    pub footer: [Option<u32>; 2],
    /// Parsed PID tuning loop records.
    pub loops: Vec<PidTuneLoopSummary>,
    /// Raw XML attributes from the PID tuning node.
    pub attributes: Vec<XmlAttribute>,
}

impl PidTuneParameterSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        let loops = element
            .descendants_named("PidTunLoopData")
            .enumerate()
            .map(|(loop_index, loop_data)| PidTuneLoopSummary::from_element(loop_index, loop_data))
            .collect();

        Self {
            set_direction: attr_u32(element, "SetDirection"),
            permit_pwm: attr_u32(element, "PermitPWM"),
            checksum: attr_u32(element, "CheckSum"),
            footer: [attr_u32(element, "Footer_0"), attr_u32(element, "Footer_1")],
            loops,
            attributes: element.attributes.clone(),
        }
    }
}

/// One embedded PID tuning loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PidTuneLoopSummary {
    /// Zero-based loop index.
    pub loop_index: usize,
    /// Set value/target value.
    pub target_value: Option<i32>,
    /// Scan time for this loop.
    pub scan_time: Option<u32>,
    pub mv_max: Option<i32>,
    pub mv_min: Option<i32>,
    /// Raw PWM point value. This is a P-area bit offset in LSD-hex notation.
    pub set_pwm_at_point: Option<u32>,
    /// PWM output period.
    pub out_period: Option<u32>,
    pub hysteresis: Option<i32>,
    /// Raw XML attributes from the loop node.
    pub attributes: Vec<XmlAttribute>,
}

impl PidTuneLoopSummary {
    pub(crate) fn from_element(loop_index: usize, element: &XmlElement) -> Self {
        Self {
            loop_index,
            target_value: attr_i32(element, "TargetValue"),
            scan_time: attr_u32(element, "ScanTime"),
            mv_max: attr_i32(element, "MVMax"),
            mv_min: attr_i32(element, "MVMin"),
            set_pwm_at_point: attr_u32(element, "SetPWMAtPoint"),
            out_period: attr_u32(element, "OutPeriod"),
            hysteresis: attr_i32(element, "SetHisterisys"),
            attributes: element.attributes.clone(),
        }
    }
}

/// Compact summary for an XML section whose attributes carry the useful data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlSectionSummary {
    pub name: String,
    pub text: Option<String>,
    pub child_count: usize,
    pub attributes: Vec<XmlAttribute>,
}

impl XmlSectionSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            name: element.name.clone(),
            text: text_value(element),
            child_count: element.children.len(),
            attributes: element.attributes.clone(),
        }
    }
}

/// Safety communication parameters from `<Safety_Comm>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyCommSummary {
    pub rcv_wait_time: Option<u32>,
    pub retrans_time: Option<u32>,
    pub glofa_socket_count: Option<u32>,
    pub driver_type: Option<u32>,
    pub ip_address_raw: Option<u32>,
    pub ip_address: Option<String>,
    pub gateway_raw: Option<u32>,
    pub gateway: Option<String>,
    pub subnet_raw: Option<u32>,
    pub subnet: Option<String>,
    pub enable_host_table: Option<u32>,
    pub channels: Vec<SafetyCommChannelSummary>,
    pub attributes: Vec<XmlAttribute>,
}

impl SafetyCommSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            rcv_wait_time: attr_u32(element, "RcvWaitTime"),
            retrans_time: attr_u32(element, "RetransTime"),
            glofa_socket_count: attr_u32(element, "GlofaSocketCnt"),
            driver_type: attr_u32(element, "DriverType"),
            ip_address_raw: attr_u32(element, "IPAddress"),
            ip_address: attr_u32(element, "IPAddress").map(format_ipv4_le),
            gateway_raw: attr_u32(element, "Gateway"),
            gateway: attr_u32(element, "Gateway").map(format_ipv4_le),
            subnet_raw: attr_u32(element, "Subnet"),
            subnet: attr_u32(element, "Subnet").map(format_ipv4_le),
            enable_host_table: attr_u32(element, "EnableHostTable"),
            channels: element
                .children
                .iter()
                .map(SafetyCommChannelSummary::from_element)
                .collect(),
            attributes: element.attributes.clone(),
        }
    }
}

/// One child channel/config record below `<Safety_Comm>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyCommChannelSummary {
    pub name: String,
    pub address: Option<String>,
    pub device: Option<char>,
    pub device_type: Option<u32>,
    pub data_type: Option<u32>,
    pub size: Option<u32>,
    pub addr: Option<u32>,
    pub attributes: Vec<XmlAttribute>,
}

impl SafetyCommChannelSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        let device = attr_u32(element, "DeviceType").and_then(ascii_char_from_u32);
        let addr = attr_u32(element, "Addr");
        let data_type = attr_u32(element, "DataType");
        Self {
            name: element.name.clone(),
            address: format_comm_channel_address(&element.name, device, addr),
            device,
            device_type: attr_u32(element, "DeviceType"),
            data_type,
            size: attr_u32(element, "Size"),
            addr,
            attributes: element.attributes.clone(),
        }
    }
}

/// Trend monitoring settings from `<TrendMonitoring>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrendMonitoringSummary {
    pub trace_configuration: Option<XmlSectionSummary>,
    pub graph_configuration: Option<XmlSectionSummary>,
    pub window_configuration: Option<XmlSectionSummary>,
    pub attributes: Vec<XmlAttribute>,
}

impl TrendMonitoringSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            trace_configuration: element
                .children_named("TraceConfiguration")
                .next()
                .map(XmlSectionSummary::from_element),
            graph_configuration: element
                .children_named("GraphConfiguration")
                .next()
                .map(XmlSectionSummary::from_element),
            window_configuration: element
                .children_named("WindowConfiguration")
                .next()
                .map(XmlSectionSummary::from_element),
            attributes: element.attributes.clone(),
        }
    }
}

/// DeviceNet XGPD configuration record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XgpdConfigInfoSummary {
    pub station_no: Option<u32>,
    pub type_code: Option<u32>,
    pub base: Option<u32>,
    pub slot: Option<u32>,
    pub sub_type: Option<u32>,
    pub attributes: Vec<XmlAttribute>,
}

impl XgpdConfigInfoSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            station_no: attr_u32(element, "StationNo"),
            type_code: attr_u32(element, "Type"),
            base: attr_u32(element, "Base"),
            slot: attr_u32(element, "Slot"),
            sub_type: attr_u32(element, "SubType"),
            attributes: element.attributes.clone(),
        }
    }
}

/// Cnet XGPD configuration record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CnetConfigInfoSummary {
    /// Station number from the module-level record.
    pub station_no: Option<u32>,
    /// Stable module type code. Match this to `NetworkModule Id`.
    pub type_code: Option<u32>,
    /// Configured base value. This is user-editable and not stable identity.
    pub base: Option<u32>,
    /// Configured slot value. This is user-editable and not stable identity.
    pub slot: Option<u32>,
    /// Module subtype/option type.
    pub sub_type: Option<u32>,
    /// Serial port records below `XGPD_CONFIG_INFO_CNET`.
    pub ports: Vec<CnetPortConfigSummary>,
    /// Raw XML attributes from the module record.
    pub attributes: Vec<XmlAttribute>,
}

impl CnetConfigInfoSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            station_no: attr_u32(element, "StationNo"),
            type_code: attr_u32(element, "Type"),
            base: attr_u32(element, "Base"),
            slot: attr_u32(element, "Slot"),
            sub_type: attr_u32(element, "SubType"),
            ports: element
                .children_named("XGPD_CONFIG_INFO_CNET_PORT")
                .map(CnetPortConfigSummary::from_element)
                .collect(),
            attributes: element.attributes.clone(),
        }
    }
}

/// Cnet serial port configuration record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CnetPortConfigSummary {
    /// Port station number.
    pub station_no: Option<u32>,
    pub station_no_b: Option<u32>,
    /// Raw electrical mode value.
    pub mode: Option<u32>,
    /// Decoded electrical mode.
    pub mode_kind: Option<CnetMode>,
    pub modem: Option<u32>,
    /// Raw XG5000 baud selector.
    pub bps: Option<u32>,
    /// Decoded baud rate, currently `Bps * 1200`.
    pub baud_rate: Option<u32>,
    /// Raw data-bit selector.
    pub data_bit: Option<u32>,
    /// Decoded data-bit selector.
    pub data_bits: Option<CnetDataBits>,
    /// Raw stop-bit selector.
    pub stop_bit: Option<u32>,
    /// Decoded stop-bit selector.
    pub stop_bits: Option<CnetStopBits>,
    /// Raw parity selector.
    pub parity: Option<u32>,
    /// Decoded parity selector.
    pub parity_mode: Option<CnetParity>,
    /// Receive timeout.
    pub rx_timeout: Option<u32>,
    /// Character timeout.
    pub char_timeout: Option<u32>,
    /// Inter-character timeout.
    pub inter_char_timeout: Option<u32>,
    pub driver_type: Option<u32>,
    pub request_delay_time: Option<u32>,
    pub parity_error_ignore: Option<u32>,
    /// Decoded DI device area from `DI_DeviceType`.
    pub di_device: Option<char>,
    /// Formatted DI bit address. DI uses LSD-hex bit addressing.
    pub di_address: Option<String>,
    /// Raw ASCII numeric DI device type.
    pub di_device_type: Option<u32>,
    pub di_data_type: Option<u32>,
    pub di_size: Option<u32>,
    /// Raw DI address number.
    pub di_addr: Option<u32>,
    /// Decoded DO device area from `DO_DeviceType`.
    pub do_device: Option<char>,
    /// Formatted DO bit address. DO uses LSD-hex bit addressing.
    pub do_address: Option<String>,
    /// Raw ASCII numeric DO device type.
    pub do_device_type: Option<u32>,
    pub do_data_type: Option<u32>,
    pub do_size: Option<u32>,
    /// Raw DO address number.
    pub do_addr: Option<u32>,
    /// Decoded AI device area from `AI_DeviceType`.
    pub ai_device: Option<char>,
    /// Formatted AI word address. AI uses decimal word addressing.
    pub ai_address: Option<String>,
    /// Raw ASCII numeric AI device type.
    pub ai_device_type: Option<u32>,
    pub ai_data_type: Option<u32>,
    pub ai_size: Option<u32>,
    /// Raw AI address number.
    pub ai_addr: Option<u32>,
    /// Decoded AO device area from `AO_DeviceType`.
    pub ao_device: Option<char>,
    /// Formatted AO word address. AO uses decimal word addressing.
    pub ao_address: Option<String>,
    /// Raw ASCII numeric AO device type.
    pub ao_device_type: Option<u32>,
    pub ao_data_type: Option<u32>,
    pub ao_size: Option<u32>,
    /// Raw AO address number.
    pub ao_addr: Option<u32>,
    pub terminating_resister: Option<u32>,
    pub repeater: Option<u32>,
    /// Raw XML attributes from the port record.
    pub attributes: Vec<XmlAttribute>,
}

impl CnetPortConfigSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        let di_device_type = attr_u32(element, "DI_DeviceType");
        let di_addr = attr_u32(element, "DI_Addr");
        let do_device_type = attr_u32(element, "DO_DeviceType");
        let do_addr = attr_u32(element, "DO_Addr");
        let ai_device_type = attr_u32(element, "AI_DeviceType");
        let ai_addr = attr_u32(element, "AI_Addr");
        let ao_device_type = attr_u32(element, "AO_DeviceType");
        let ao_addr = attr_u32(element, "AO_Addr");
        let di_device = di_device_type.and_then(ascii_char_from_u32);
        let do_device = do_device_type.and_then(ascii_char_from_u32);
        let ai_device = ai_device_type.and_then(ascii_char_from_u32);
        let ao_device = ao_device_type.and_then(ascii_char_from_u32);

        Self {
            station_no: attr_u32(element, "StationNo"),
            station_no_b: attr_u32(element, "StationNoB"),
            mode: attr_u32(element, "Mode"),
            mode_kind: attr_u32(element, "Mode").and_then(CnetMode::from_raw),
            modem: attr_u32(element, "Modem"),
            bps: attr_u32(element, "Bps"),
            baud_rate: attr_u32(element, "Bps").map(|value| value * 1200),
            data_bit: attr_u32(element, "DataBit"),
            data_bits: attr_u32(element, "DataBit").and_then(CnetDataBits::from_raw),
            stop_bit: attr_u32(element, "StopBit"),
            stop_bits: attr_u32(element, "StopBit").and_then(CnetStopBits::from_raw),
            parity: attr_u32(element, "Parity"),
            parity_mode: attr_u32(element, "Parity").and_then(CnetParity::from_raw),
            rx_timeout: attr_u32(element, "RxTimeOut"),
            char_timeout: attr_u32(element, "CharTimeOut"),
            inter_char_timeout: attr_u32(element, "InterCharTimeOut"),
            driver_type: attr_u32(element, "DriverType"),
            request_delay_time: attr_u32(element, "RequestDelayTime"),
            parity_error_ignore: attr_u32(element, "ParityErrorIgnore"),
            di_device,
            di_address: format_cnet_io_address(di_device, di_addr, CnetIoKind::Bit),
            di_device_type,
            di_data_type: attr_u32(element, "DI_DataType"),
            di_size: attr_u32(element, "DI_Size"),
            di_addr,
            do_device,
            do_address: format_cnet_io_address(do_device, do_addr, CnetIoKind::Bit),
            do_device_type,
            do_data_type: attr_u32(element, "DO_DataType"),
            do_size: attr_u32(element, "DO_Size"),
            do_addr,
            ai_device,
            ai_address: format_cnet_io_address(ai_device, ai_addr, CnetIoKind::Word),
            ai_device_type,
            ai_data_type: attr_u32(element, "AI_DataType"),
            ai_size: attr_u32(element, "AI_Size"),
            ai_addr,
            ao_device,
            ao_address: format_cnet_io_address(ao_device, ao_addr, CnetIoKind::Word),
            ao_device_type,
            ao_data_type: attr_u32(element, "AO_DataType"),
            ao_size: attr_u32(element, "AO_Size"),
            ao_addr,
            terminating_resister: attr_u32(element, "TerminatingResister"),
            repeater: attr_u32(element, "Repeater"),
            attributes: element.attributes.clone(),
        }
    }
}

/// Cnet serial electrical interface mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CnetMode {
    /// RS-232C mode (`0`).
    Rs232C,
    /// RS-422 mode (`1`).
    Rs422,
    /// RS-485 mode (`2`).
    Rs485,
}

impl CnetMode {
    fn from_raw(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Rs232C),
            1 => Some(Self::Rs422),
            2 => Some(Self::Rs485),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Rs232C => "RS232C",
            Self::Rs422 => "RS422",
            Self::Rs485 => "RS485",
        }
    }
}

/// Cnet serial data bit setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CnetDataBits {
    /// Seven data bits (`0`).
    Seven,
    /// Eight data bits (`1`).
    Eight,
}

impl CnetDataBits {
    fn from_raw(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Seven),
            1 => Some(Self::Eight),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Seven => "7 bits",
            Self::Eight => "8 bits",
        }
    }
}

/// Cnet serial stop bit setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CnetStopBits {
    /// One stop bit (`0`).
    One,
    /// Two stop bits (`1`).
    Two,
}

impl CnetStopBits {
    fn from_raw(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::One),
            1 => Some(Self::Two),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::One => "1",
            Self::Two => "2",
        }
    }
}

/// Cnet serial parity setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CnetParity {
    /// No parity (`0`).
    None,
    /// Even parity (`1`).
    Even,
    /// Odd parity (`2`).
    Odd,
}

impl CnetParity {
    fn from_raw(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Even),
            2 => Some(Self::Odd),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Even => "EVEN",
            Self::Odd => "ODD",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CnetIoKind {
    Bit,
    Word,
}

/// FEnet XGPD configuration record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FenetConfigInfoSummary {
    /// Station number from the module-level record.
    pub station_no: Option<u32>,
    /// Stable module type code. Match this to `NetworkModule Id`.
    pub type_code: Option<u32>,
    /// Configured base value. This is user-editable and not stable identity.
    pub base: Option<u32>,
    /// Configured slot value. This is user-editable and not stable identity.
    pub slot: Option<u32>,
    /// Module subtype/option type.
    pub sub_type: Option<u32>,
    pub media: Option<u32>,
    pub media_b: Option<u32>,
    /// Primary IP address.
    pub ip_address: Option<Ipv4Summary>,
    /// Primary subnet mask.
    pub subnet: Option<Ipv4Summary>,
    /// Primary gateway address.
    pub gateway: Option<Ipv4Summary>,
    /// Primary DNS address.
    pub dns: Option<Ipv4Summary>,
    /// Secondary IP address, if present.
    pub ip_address2: Option<Ipv4Summary>,
    /// Secondary subnet mask, if present.
    pub subnet2: Option<Ipv4Summary>,
    /// Secondary gateway address, if present.
    pub gateway2: Option<Ipv4Summary>,
    /// Secondary DNS address, if present.
    pub dns2: Option<Ipv4Summary>,
    pub dhcp: Option<u32>,
    pub driver_type: Option<u32>,
    pub rcv_wait_time: Option<u32>,
    pub client_wait_time: Option<u32>,
    pub glofa_socket_count: Option<u32>,
    /// Raw XML attributes from the FEnet record.
    pub attributes: Vec<XmlAttribute>,
}

impl FenetConfigInfoSummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            station_no: attr_u32(element, "StationNo"),
            type_code: attr_u32(element, "Type"),
            base: attr_u32(element, "Base"),
            slot: attr_u32(element, "Slot"),
            sub_type: attr_u32(element, "SubType"),
            media: attr_u32(element, "Media"),
            media_b: attr_u32(element, "MediaB"),
            ip_address: ipv4_attrs(element, "IpAddr"),
            subnet: ipv4_attrs(element, "Subnet"),
            gateway: ipv4_attrs(element, "Gateway"),
            dns: ipv4_attrs(element, "Dns"),
            ip_address2: ipv4_attrs(element, "IpAddr2"),
            subnet2: ipv4_attrs(element, "Subnet2"),
            gateway2: ipv4_attrs(element, "Gateway2"),
            dns2: ipv4_attrs(element, "Dns2"),
            dhcp: attr_u32(element, "Dhcp"),
            driver_type: attr_u32(element, "DriverType"),
            rcv_wait_time: attr_u32(element, "RcvWaitTime"),
            client_wait_time: attr_u32(element, "ClientWaitTime"),
            glofa_socket_count: attr_u32(element, "GlofaSocketCnt"),
            attributes: element.attributes.clone(),
        }
    }
}

/// Four-octet IPv4 setting from split XML attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv4Summary {
    /// Numeric IPv4 octets in display order.
    pub octets: [u8; 4],
    /// Dotted decimal IPv4 address.
    pub address: String,
}

/// Project property record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertySummary {
    pub value: Option<String>,
    pub attributes: Vec<XmlAttribute>,
}

impl PropertySummary {
    pub(crate) fn from_element(element: &XmlElement) -> Self {
        Self {
            value: attr_string(element, "Value"),
            attributes: element.attributes.clone(),
        }
    }
}

/// Decoded base64 binary payload from an XML element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPayloadSummary {
    pub path: String,
    pub tag: String,
    pub compressed: bool,
    pub encoded_len: usize,
    pub raw_len: usize,
    pub decoded_len: usize,
    pub data: Vec<u8>,
    pub attributes: Vec<XmlAttribute>,
}

/// Metadata parsed from the binary header before the main gzip member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XgwxHeader {
    /// Raw header bytes, ending immediately before the main gzip member.
    pub raw: Vec<u8>,
    /// Main gzip member offset in the file.
    pub gzip_offset: usize,
    /// The six-byte signature/version prefix observed at the start of the file.
    pub signature: [u8; 6],
    /// UTF-16LE header label, normally `XG5000 WORKSPACE FILE`.
    pub label: Option<String>,
    /// Four-byte little-endian field immediately after the label, if present.
    pub label_following_u32: Option<u32>,
    /// Little-endian size hint stored immediately before the gzip member.
    ///
    /// In the sample this is three bytes larger than the compressed gzip member.
    /// The exact semantics are not yet documented, so callers should treat it as
    /// advisory metadata.
    pub compressed_size_hint: Option<u32>,
}

impl XgwxHeader {
    pub(crate) fn parse(raw: &[u8], gzip_offset: usize) -> Self {
        let mut signature = [0; 6];
        let sig_len = raw.len().min(signature.len());
        signature[..sig_len].copy_from_slice(&raw[..sig_len]);

        let (label, label_following_u32) = parse_header_label(raw);
        let compressed_size_hint = raw
            .get(raw.len().saturating_sub(4)..)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_le_bytes);

        Self {
            raw: raw.to_vec(),
            gzip_offset,
            signature,
            label,
            label_following_u32,
            compressed_size_hint,
        }
    }
}

/// A valid gzip member discovered in an `.xgwx` container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GzipMember {
    /// Absolute offset of the gzip header in the source file.
    pub offset: usize,
    /// Absolute offset immediately after the gzip trailer.
    pub end_offset: usize,
    /// Inflated member contents.
    pub data: Vec<u8>,
}

/// Owned XML element tree for the project payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlElement {
    pub name: String,
    pub attributes: Vec<XmlAttribute>,
    /// Concatenated direct text children. Descendant text is kept on descendants.
    pub text: String,
    pub children: Vec<XmlElement>,
}

impl XmlElement {
    /// Return the first attribute value with the provided name.
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name == name)
            .map(|attribute| attribute.value.as_str())
    }

    /// Return direct child elements with the provided tag name.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a XmlElement> {
        self.children
            .iter()
            .filter(move |element| element.name == name)
    }

    /// Return this element and descendant elements with the provided tag name.
    pub fn descendants_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a XmlElement> {
        XmlDescendantsNamed::new(self, name)
    }
}

/// XML attribute stored on an [`XmlElement`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlAttribute {
    pub name: String,
    pub value: String,
}
