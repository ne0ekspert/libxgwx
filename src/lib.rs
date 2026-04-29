//! Parser for LS XG5000 `.xgwx` workspace files.
//!
//! The observed file layout is a small `XG` binary header, one gzip-compressed
//! UTF-8 XML project payload, and optional trailing binary metadata. This crate
//! validates and decodes that container, parses the project XML into a compact
//! tree, and keeps unknown binary sections available for callers that need them.
//!
//! # Example
//!
//! ```text
//! use libxgwx::XgwxDocument;
//!
//! if let Ok(doc) = XgwxDocument::from_path("project.xgwx") {
//!     let project = doc.project_info();
//!
//!     println!("project: {:?}", project.name);
//!     println!("programs: {}", doc.programs().len());
//!     println!("modules: {}", doc.modules().len());
//!
//!     for fenet in doc.fenet_config_infos() {
//!         println!(
//!             "FEnet type={:?} ip={:?}",
//!             fenet.type_code,
//!             fenet.ip_address.as_ref().map(|ip| ip.address.as_str())
//!         );
//!     }
//!
//!     for cnet in doc.cnet_config_infos() {
//!         println!("Cnet type={:?} ports={}", cnet.type_code, cnet.ports.len());
//!     }
//! }
//! ```

use std::fmt;
use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;

use base64::Engine;
use bzip2::read::BzDecoder;
use flate2::{Decompress, FlushDecompress, Status};

const XG_MAGIC: &[u8; 2] = b"XG";
const GZIP_MAGIC: &[u8; 2] = b"\x1f\x8b";
const GZIP_HEADER_LEN: usize = 10;
const GZIP_TRAILER_LEN: usize = 8;
const GZIP_METHOD_DEFLATE: u8 = 8;
const UTF16_MARKER: &[u8; 3] = b"\xff\xfe\xff";

/// Parsed representation of an `.xgwx` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XgwxDocument {
    /// Parsed fixed/header metadata.
    pub header: XgwxHeader,
    /// Inflated XML project payload.
    pub xml: String,
    /// XML payload parsed into a lightweight owned tree.
    pub root: XmlElement,
    /// Raw bytes after the main gzip XML member.
    pub trailer: Vec<u8>,
    /// Additional valid gzip members found inside the trailer, if any.
    pub trailer_gzip_members: Vec<GzipMember>,
}

impl XgwxDocument {
    /// Parse an `.xgwx` document from bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self, XgwxError> {
        if bytes.len() < XG_MAGIC.len() || &bytes[..2] != XG_MAGIC {
            return Err(XgwxError::InvalidMagic);
        }

        let gzip_offset = find_gzip_member(bytes, 0).ok_or(XgwxError::MissingMainPayload)?;
        let header = XgwxHeader::parse(&bytes[..gzip_offset], gzip_offset);
        let main_payload = parse_gzip_member(bytes, gzip_offset)?;
        let trailer = bytes[main_payload.end_offset..].to_vec();
        let xml = String::from_utf8(main_payload.data).map_err(XgwxError::Utf8)?;
        let root = parse_xml(&xml)?;
        let trailer_gzip_members = find_trailer_gzip_members(bytes, main_payload.end_offset);

        Ok(Self {
            header,
            xml,
            root,
            trailer,
            trailer_gzip_members,
        })
    }

    /// Parse an `.xgwx` document from a file path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, XgwxError> {
        let bytes = fs::read(path).map_err(XgwxError::Io)?;
        Self::parse(&bytes)
    }

    /// Return high-level metadata from the root `<Project>` element.
    pub fn project_info(&self) -> ProjectInfo {
        ProjectInfo::from_element(&self.root)
    }

    /// Return high-level summaries for all `<Configuration>` elements.
    pub fn configurations(&self) -> Vec<ConfigurationSummary> {
        self.root
            .descendants_named("Configuration")
            .map(ConfigurationSummary::from_element)
            .collect()
    }

    /// Return high-level summaries for all `<Network>` elements.
    pub fn networks(&self) -> Vec<NetworkSummary> {
        self.root
            .descendants_named("Network")
            .map(NetworkSummary::from_element)
            .collect()
    }

    /// Return high-level summaries for all `<NetworkModule>` elements.
    pub fn network_modules(&self) -> Vec<NetworkModuleSummary> {
        self.root
            .descendants_named("NetworkModule")
            .map(NetworkModuleSummary::from_element)
            .collect()
    }

    /// Return high-level summaries for all hardware `<Base>` elements.
    pub fn bases(&self) -> Vec<BaseSummary> {
        self.root
            .descendants_named("Base")
            .map(BaseSummary::from_element)
            .collect()
    }

    /// Return high-level summaries for all hardware `<Module>` elements.
    pub fn modules(&self) -> Vec<ModuleSummary> {
        self.root
            .descendants_named("Module")
            .map(ModuleSummary::from_element)
            .collect()
    }

    /// Return high-level summaries for all `<Task>` elements.
    pub fn tasks(&self) -> Vec<TaskSummary> {
        self.root
            .descendants_named("Task")
            .map(TaskSummary::from_element)
            .collect()
    }

    /// Return high-level summaries for all `<Program>` elements.
    pub fn programs(&self) -> Vec<ProgramSummary> {
        self.root
            .descendants_named("Program")
            .map(ProgramSummary::from_element)
            .collect()
    }

    /// Decode the global variable symbol table.
    ///
    /// XG5000 stores `<Symbols>` as base64, usually bzip2-compressed, binary
    /// data. The observed symbol table contains fixed seven-string records.
    pub fn variables(&self) -> Result<Vec<VariableSummary>, XgwxError> {
        let Some(symbols) = self.root.descendants_named("Symbols").next() else {
            return Ok(Vec::new());
        };

        VariableSummary::from_symbols_element(symbols)
    }

    /// Decode ladder `<ProgramData>` payloads for all `<Program>` elements.
    ///
    /// XG5000 stores ladder bodies as base64, usually bzip2-compressed, binary
    /// blobs. This returns a partial decode that preserves the bytes and extracts
    /// embedded UTF-16 strings and likely instruction calls.
    pub fn ladder_programs(&self) -> Vec<Result<LadderProgramData, XgwxError>> {
        self.root
            .descendants_named("Program")
            .map(LadderProgramData::from_program_element)
            .collect()
    }

    /// Return high-level summaries for all `<XGPD_HS_LINK>` elements.
    pub fn high_speed_links(&self) -> Vec<HighSpeedLinkSummary> {
        self.root
            .descendants_named("XGPD_HS_LINK")
            .map(HighSpeedLinkSummary::from_element)
            .collect()
    }

    /// Return high-level summaries for all `<XGPD_HS_LINK_BLK>` elements.
    pub fn high_speed_link_blocks(&self) -> Vec<HighSpeedLinkBlockSummary> {
        self.root
            .descendants_named("XGPD_HS_LINK_BLK")
            .map(HighSpeedLinkBlockSummary::from_element)
            .collect()
    }

    /// Return project option entries from the `<Options>` section.
    pub fn project_options(&self) -> Option<ProjectOptionsSummary> {
        self.root
            .descendants_named("Options")
            .next()
            .map(ProjectOptionsSummary::from_element)
    }

    /// Return summaries for all top-level `<Parameter>` sections.
    pub fn parameters(&self) -> Vec<ParameterSummary> {
        self.root
            .descendants_named("Parameter")
            .map(ParameterSummary::from_element)
            .collect()
    }

    /// Decode high-speed counter payloads from `HSC PARAMETER` sections.
    ///
    /// The observed XGB payload is stored as a hex ASCII `PAYLOAD` attribute.
    /// This preserves the raw string and bytes while decoding the known
    /// per-channel fields: counter mode, pulse input mode, compare output mode,
    /// preset bytes, ring counter maximum, compare output min/max, unit time,
    /// and pulses per revolution.
    pub fn hsc_parameters(&self) -> Vec<Result<HscParameterSummary, XgwxError>> {
        self.root
            .descendants_named("Parameter")
            .filter(|element| element.attribute("Type") == Some("HSC PARAMETER"))
            .map(HscParameterSummary::from_element)
            .collect()
    }

    /// Return position-control axis tables from `POSITION PARAMETER` sections.
    ///
    /// The observed XGB samples contain X and Y axis data, but this parser
    /// keeps additional axis records if the file reports more axes.
    pub fn position_parameters(&self) -> Vec<PositionParameterSummary> {
        self.root
            .descendants_named("Parameter")
            .filter(|element| element.attribute("Type") == Some("POSITION PARAMETER"))
            .map(PositionParameterSummary::from_element)
            .collect()
    }

    /// Return embedded PID calculation parameters from `PID CAL PARAMETER`.
    pub fn pid_cal_parameters(&self) -> Vec<PidCalParameterSummary> {
        self.root
            .descendants_named("Parameter")
            .filter(|element| element.attribute("Type") == Some("PID CAL PARAMETER"))
            .map(PidCalParameterSummary::from_element)
            .collect()
    }

    /// Return embedded PID tuning parameters from `PID TUNE PARAMETER`.
    ///
    /// PWM point values are raw P-area offsets in XG5000's LSD-hex bit address
    /// convention; callers can format them using the same rule as other P bit
    /// addresses if desired.
    pub fn pid_tune_parameters(&self) -> Vec<PidTuneParameterSummary> {
        self.root
            .descendants_named("Parameter")
            .filter(|element| element.attribute("Type") == Some("PID TUNE PARAMETER"))
            .map(PidTuneParameterSummary::from_element)
            .collect()
    }

    /// Return DeviceNet/FEnet safety communication parameters, if present.
    pub fn safety_comm(&self) -> Option<SafetyCommSummary> {
        self.root
            .descendants_named("Safety_Comm")
            .next()
            .map(SafetyCommSummary::from_element)
    }

    /// Return trend monitoring configuration, if present.
    pub fn trend_monitoring(&self) -> Option<TrendMonitoringSummary> {
        self.root
            .descendants_named("TrendMonitoring")
            .next()
            .map(TrendMonitoringSummary::from_element)
    }

    /// Return DeviceNet XGPD configuration records.
    pub fn xgpd_config_infos(&self) -> Vec<XgpdConfigInfoSummary> {
        self.root
            .descendants_named("XGPD_CONFIG_INFO_DNET")
            .map(XgpdConfigInfoSummary::from_element)
            .collect()
    }

    /// Return Cnet XGPD configuration records, including serial port settings.
    ///
    /// Cnet records can be associated with `<NetworkModule>` entries by
    /// matching `NetworkModule Id` to [`CnetConfigInfoSummary::type_code`].
    /// Base and slot are exposed but should not be used as stable identity.
    pub fn cnet_config_infos(&self) -> Vec<CnetConfigInfoSummary> {
        self.root
            .descendants_named("XGPD_CONFIG_INFO_CNET")
            .map(CnetConfigInfoSummary::from_element)
            .collect()
    }

    /// Return FEnet XGPD configuration records, including module IPv4 settings.
    ///
    /// FEnet records can be associated with `<NetworkModule>` entries by
    /// matching `NetworkModule Id` to [`FenetConfigInfoSummary::type_code`].
    /// Base and slot are exposed but should not be used as stable identity.
    pub fn fenet_config_infos(&self) -> Vec<FenetConfigInfoSummary> {
        self.root
            .descendants_named("XGPD_CONFIG_INFO_FENET")
            .map(FenetConfigInfoSummary::from_element)
            .collect()
    }

    /// Return project property records.
    pub fn properties(&self) -> Vec<PropertySummary> {
        self.root
            .descendants_named("Properties")
            .map(PropertySummary::from_element)
            .collect()
    }

    /// Decode every XML element that advertises a base64 binary payload.
    ///
    /// This includes known sections such as `Symbols`, `ProgramData`, online
    /// tables, safety signatures, and retained values. The returned summaries
    /// preserve decoded bytes but do not assign proprietary semantics to tables
    /// whose binary layout is not yet identified.
    pub fn decoded_payloads(&self) -> Vec<Result<DecodedPayloadSummary, XgwxError>> {
        let mut payloads = Vec::new();
        collect_decoded_payloads(&self.root, &mut Vec::new(), &mut payloads);
        payloads
    }

    /// Returns the direct text inside the root `<Project>` element, if present.
    pub fn project_name(&self) -> Option<&str> {
        let name = self.root.text.trim();
        (!name.is_empty()).then_some(name)
    }
}

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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_symbols_element(symbols: &XmlElement) -> Result<Vec<Self>, XgwxError> {
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
    fn from_program_element(program: &XmlElement) -> Result<Self, XgwxError> {
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
    pub horizontal_lines: Vec<LadderHorizontalLine>,
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

/// One decoded horizontal LD connection segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderHorizontalLine {
    pub raw_y: u8,
    pub raw_x_start: u8,
    pub raw_x_end: u8,
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
}

/// Coil kind decoded from LD output coil records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LadderCoil {
    Output,
    Set,
    Reset,
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Result<Self, XgwxError> {
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
    fn from_raw(value: u8) -> Self {
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
    fn from_raw(value: u8) -> Self {
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
    fn from_raw(value: u8) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(step_index: usize, element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(loop_index: usize, element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(loop_index: usize, element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
enum CnetIoKind {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn from_element(element: &XmlElement) -> Self {
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
    fn parse(raw: &[u8], gzip_offset: usize) -> Self {
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

fn text_value(element: &XmlElement) -> Option<String> {
    let text = element.text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

fn attr_string(element: &XmlElement, name: &str) -> Option<String> {
    element.attribute(name).map(str::to_owned)
}

fn attr_u32(element: &XmlElement, name: &str) -> Option<u32> {
    element.attribute(name)?.trim().parse().ok()
}

fn attr_u8(element: &XmlElement, name: &str) -> Option<u8> {
    element.attribute(name)?.trim().parse().ok()
}

fn attr_i32(element: &XmlElement, name: &str) -> Option<i32> {
    element.attribute(name)?.trim().parse().ok()
}

fn attr_bool(element: &XmlElement, name: &str) -> Option<bool> {
    match element.attribute(name)?.trim() {
        "1" | "true" | "TRUE" | "True" => Some(true),
        "0" | "false" | "FALSE" | "False" => Some(false),
        _ => None,
    }
}

fn decode_base64_payload(text: &str, compressed: bool) -> Result<Vec<u8>, XgwxError> {
    let (_, decoded) = decode_base64_payload_with_raw(text, compressed)?;
    Ok(decoded)
}

fn decode_base64_payload_with_raw(
    text: &str,
    compressed: bool,
) -> Result<(Vec<u8>, Vec<u8>), XgwxError> {
    let encoded = text.split_whitespace().collect::<String>();
    if encoded.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.as_bytes())
        .map_err(XgwxError::Base64)?;

    if !compressed {
        return Ok((decoded.clone(), decoded));
    }

    let raw = decoded;
    let mut decoder = BzDecoder::new(raw.as_slice());
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(XgwxError::Bzip2)?;
    Ok((raw, decompressed))
}

fn decode_hex_ascii_payload(
    text: &str,
    element: &str,
    attribute: &str,
) -> Result<Vec<u8>, XgwxError> {
    let encoded = text.split_whitespace().collect::<String>();
    if encoded.len() % 2 != 0 {
        return Err(XgwxError::InvalidHexPayload {
            element: element.to_owned(),
            attribute: attribute.to_owned(),
        });
    }

    let mut bytes = Vec::with_capacity(encoded.len() / 2);
    let mut chars = encoded.chars();
    while let (Some(high), Some(low)) = (chars.next(), chars.next()) {
        let Some(high) = high.to_digit(16) else {
            return Err(XgwxError::InvalidHexPayload {
                element: element.to_owned(),
                attribute: attribute.to_owned(),
            });
        };
        let Some(low) = low.to_digit(16) else {
            return Err(XgwxError::InvalidHexPayload {
                element: element.to_owned(),
                attribute: attribute.to_owned(),
            });
        };
        bytes.push(((high << 4) | low) as u8);
    }

    Ok(bytes)
}

fn parse_hsc_channels(payload: &str, payload_bytes: &[u8]) -> Vec<HscChannelSummary> {
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

fn hex_nibble_at(text: &str, offset: usize) -> Option<u8> {
    text.split_whitespace()
        .flat_map(str::chars)
        .nth(offset)?
        .to_digit(16)
        .and_then(|value| u8::try_from(value).ok())
}

fn position_axis_name(axis_index: usize) -> &'static str {
    match axis_index {
        0 => "X",
        1 => "Y",
        2 => "Z",
        3 => "U",
        _ => "Axis",
    }
}

fn parse_project_option_entries(raw: &str) -> Vec<ProjectOptionEntry> {
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

fn collect_decoded_payloads(
    element: &XmlElement,
    path: &mut Vec<String>,
    payloads: &mut Vec<Result<DecodedPayloadSummary, XgwxError>>,
) {
    path.push(element.name.clone());

    if is_base64_payload_element(element) {
        payloads.push(decoded_payload_summary(element, path));
    }

    for child in &element.children {
        collect_decoded_payloads(child, path, payloads);
    }

    path.pop();
}

fn decoded_payload_summary(
    element: &XmlElement,
    path: &[String],
) -> Result<DecodedPayloadSummary, XgwxError> {
    let compressed = attr_bool(element, "Compressed").unwrap_or(false);
    let encoded = element.text.split_whitespace().collect::<String>();
    let (raw, data) = decode_base64_payload_with_raw(&element.text, compressed)?;

    Ok(DecodedPayloadSummary {
        path: path.join("/"),
        tag: element.name.clone(),
        compressed,
        encoded_len: encoded.len(),
        raw_len: raw.len(),
        decoded_len: data.len(),
        data,
        attributes: element.attributes.clone(),
    })
}

fn is_base64_payload_element(element: &XmlElement) -> bool {
    if element.attribute("Compressed").is_some() {
        return true;
    }

    if element
        .attribute("dt")
        .is_some_and(|value| value.to_ascii_lowercase().contains("base64"))
    {
        return true;
    }

    is_known_payload_tag(&element.name) && looks_like_base64_payload_text(&element.text)
}

fn is_known_payload_tag(name: &str) -> bool {
    matches!(
        name,
        "Symbols"
            | "HMIFlags"
            | "ProgramData"
            | "OnlineUploadData"
            | "RungTableData"
            | "TableData"
            | "ArrayData"
            | "StringTableData"
            | "PulseTableData"
            | "SafetySignature"
            | "RWItem_StateMemento"
            | "RetainValue"
            | "DataTrace"
    )
}

fn looks_like_base64_payload_text(text: &str) -> bool {
    let encoded = text.split_whitespace().collect::<String>();
    !encoded.is_empty()
        && encoded.len() % 4 == 0
        && encoded
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
}

fn format_ipv4_le(value: u32) -> String {
    let bytes = value.to_le_bytes();
    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
}

fn ipv4_attrs(element: &XmlElement, prefix: &str) -> Option<Ipv4Summary> {
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

fn ascii_char_from_u32(value: u32) -> Option<char> {
    u8::try_from(value)
        .ok()
        .filter(u8::is_ascii_graphic)
        .map(char::from)
}

fn format_comm_channel_address(
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

fn format_cnet_io_address(
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

fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    let bytes = data.get(offset..offset + 2)?;
    Some(u16::from_le_bytes(bytes.try_into().ok()?))
}

fn read_i32_le(data: &[u8], offset: usize) -> Option<i32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(i32::from_le_bytes(bytes.try_into().ok()?))
}

fn format_variable_address(
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

fn format_lsd_hex_address(area: &str, number: u32, width: usize) -> String {
    let high = number / 16;
    let low = number % 16;
    let body_width = width.saturating_sub(1);
    format!("{area}{high:0body_width$}{low:X}")
}

fn format_u_address(
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

fn parse_special_module_slot(source_ref: Option<&str>) -> Option<u32> {
    let mut parts = source_ref?.split(':');
    match (parts.next(), parts.next(), parts.next()) {
        (Some("SP"), Some(_base), Some(slot)) => slot.parse().ok(),
        _ => None,
    }
}

fn extract_ladder_strings(data: &[u8]) -> Vec<LadderString> {
    extract_utf16_marker_strings(data, true, false)
}

fn extract_utf16_marker_strings(
    data: &[u8],
    ascii_graphic_only: bool,
    include_empty: bool,
) -> Vec<LadderString> {
    let mut strings = Vec::new();
    let mut offset = 0;

    while offset + UTF16_MARKER.len() < data.len() {
        if data[offset..].starts_with(UTF16_MARKER) {
            let len_offset = offset + UTF16_MARKER.len();
            let char_len = data[len_offset] as usize;
            let text_start = len_offset + 1;
            let text_end = text_start + char_len.saturating_mul(2);

            if let Some(encoded) = data.get(text_start..text_end) {
                let units = encoded
                    .chunks_exact(2)
                    .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect::<Vec<_>>();

                if let Ok(value) = String::from_utf16(&units)
                    && (include_empty || !value.is_empty())
                    && (!ascii_graphic_only
                        || value.chars().all(|ch| ch.is_ascii_graphic() || ch == ' '))
                {
                    strings.push(LadderString {
                        offset,
                        end_offset: text_end,
                        value,
                    });
                }

                offset = text_end;
                continue;
            }
        }

        offset += 1;
    }

    strings
}

fn extract_ladder_instructions(strings: &[LadderString]) -> Vec<LadderInstruction> {
    strings
        .iter()
        .filter_map(parse_ladder_instruction)
        .collect()
}

fn extract_ladder_elements(data: &[u8], strings: &[LadderString]) -> Vec<LadderElement> {
    let mut elements = Vec::new();
    let mut index = 0;

    while index < strings.len() {
        let Some(element) = parse_ladder_element(data, &strings[index]) else {
            index += 1;
            continue;
        };

        let skip = duplicate_decomposed_string_count(strings, index, &element);
        elements.push(element);
        index += skip + 1;
    }

    elements
}

fn extract_ladder_structure(data: &[u8], elements: &[LadderElement]) -> LadderStructure {
    let vertical_lines = extract_ladder_vertical_lines(data);
    let horizontal_lines = extract_ladder_horizontal_lines(data);
    let mut cells = elements
        .iter()
        .filter_map(|element| {
            let (raw_x, raw_y) = ladder_element_coordinate(data, element)?;
            Some(LadderCell {
                offset: element.offset,
                raw_x,
                raw_y,
                kind: element.kind,
                value: element.value.clone(),
                operands: element.operands.clone(),
                contact: element.contact,
                coil: element.coil,
            })
        })
        .collect::<Vec<_>>();

    cells.sort_by_key(|cell| (cell.raw_y, cell.raw_x, cell.offset));

    let mut rungs = Vec::new();
    for cell in cells {
        if rungs
            .last()
            .is_none_or(|rung: &LadderRung| rung.raw_y != cell.raw_y)
        {
            rungs.push(LadderRung {
                raw_y: cell.raw_y,
                cells: Vec::new(),
            });
        }

        if let Some(rung) = rungs.last_mut() {
            rung.cells.push(cell);
        }
    }

    LadderStructure {
        rungs,
        vertical_lines,
        horizontal_lines,
    }
}

fn extract_ladder_vertical_lines(data: &[u8]) -> Vec<LadderVerticalLine> {
    let mut lines = Vec::new();
    let mut offset = 0;

    while let Some(relative) = data.get(offset..).and_then(|remaining| {
        remaining
            .windows(2)
            .position(|window| window == [0xff, 0x43])
    }) {
        let marker = offset + relative;
        match data.get(marker + 13).copied() {
            Some(0x24 | 0x32) => {
                if let (Some((_, raw_y_end)), Some((raw_x, raw_y_start))) = (
                    read_ladder_coordinate(data, marker + 17),
                    read_ladder_coordinate(data, marker + 36),
                ) && raw_x > 1
                    && raw_y_start.checked_add(4) == Some(raw_y_end)
                {
                    lines.push(LadderVerticalLine {
                        raw_x,
                        raw_y_start,
                        raw_y_end,
                    });
                }
            }
            Some(0x28) => {
                if let (Some((raw_x, raw_y_end)), Some((_, raw_y_start))) = (
                    read_ladder_coordinate(data, marker + 17),
                    read_ladder_coordinate(data, marker + 36),
                ) && raw_y_start.checked_add(4) == Some(raw_y_end)
                {
                    lines.push(LadderVerticalLine {
                        raw_x,
                        raw_y_start,
                        raw_y_end,
                    });
                }
            }
            Some(0x27) => {
                if let (Some((target_x, raw_y_end)), Some((source_x, raw_y_start))) = (
                    read_ladder_coordinate(data, marker + 17),
                    read_ladder_coordinate(data, marker + 36),
                ) && source_x == 0x03
                    && raw_y_start.checked_add(4) == Some(raw_y_end)
                    && let Some(raw_x) = target_x.checked_sub(3)
                {
                    lines.push(LadderVerticalLine {
                        raw_x,
                        raw_y_start,
                        raw_y_end,
                    });
                }
            }
            _ => {}
        }

        offset = marker + 2;
    }

    lines.sort_by_key(|line| (line.raw_y_start, line.raw_y_end, line.raw_x));
    lines.dedup();
    lines
}

fn extract_ladder_horizontal_lines(data: &[u8]) -> Vec<LadderHorizontalLine> {
    let mut lines = Vec::new();
    let mut offset = 0;

    while let Some(relative) = data.get(offset..).and_then(|remaining| {
        remaining
            .windows(2)
            .position(|window| window == [0xff, 0x43])
    }) {
        let marker = offset + relative;
        match data.get(marker + 13).copied() {
            Some(0x24 | 0x32) => {
                if let (Some((target_x, raw_y)), Some((source_x, _))) = (
                    read_ladder_coordinate(data, marker + 17),
                    read_ladder_coordinate(data, marker + 36),
                ) {
                    push_ladder_horizontal_line(&mut lines, raw_y, source_x, target_x);
                }
            }
            Some(0x27) => {
                if let (Some((target_x, raw_y)), Some((source_x, _))) = (
                    read_ladder_coordinate(data, marker + 17),
                    read_ladder_coordinate(data, marker + 36),
                ) {
                    let raw_x_start = if source_x == 0x03 {
                        target_x.saturating_sub(3)
                    } else {
                        source_x
                    };
                    push_ladder_horizontal_line(&mut lines, raw_y, raw_x_start, target_x);
                }
            }
            _ => {}
        }

        offset = marker + 2;
    }

    lines.sort_by_key(|line| (line.raw_y, line.raw_x_start, line.raw_x_end));
    lines.dedup();
    lines
}

fn push_ladder_horizontal_line(
    lines: &mut Vec<LadderHorizontalLine>,
    raw_y: u8,
    raw_x_start: u8,
    raw_x_end: u8,
) {
    let (raw_x_start, raw_x_end) = (raw_x_start.min(raw_x_end), raw_x_start.max(raw_x_end));
    if raw_x_start < raw_x_end {
        lines.push(LadderHorizontalLine {
            raw_y,
            raw_x_start,
            raw_x_end,
        });
    }
}

fn ladder_element_coordinate(data: &[u8], element: &LadderElement) -> Option<(u8, u8)> {
    let preferred_offset = if !element.operands.is_empty() || is_ladder_operation(&element.value) {
        element.offset.checked_sub(14)
    } else {
        element.offset.checked_sub(10)
    };

    preferred_offset
        .and_then(|offset| read_ladder_coordinate(data, offset))
        .or_else(|| {
            [14, 10, 12, 8]
                .iter()
                .filter_map(|back| element.offset.checked_sub(*back))
                .find_map(|offset| read_ladder_coordinate(data, offset))
        })
}

fn read_ladder_coordinate(data: &[u8], offset: usize) -> Option<(u8, u8)> {
    let bytes = data.get(offset..offset + 2)?;
    let raw_x = bytes[0];
    let raw_y = bytes[1];

    (raw_x > 0 && raw_x <= 0x80 && raw_y <= 0xf0 && raw_y % 4 == 0).then_some((raw_x, raw_y))
}

fn parse_ladder_element(data: &[u8], ladder_string: &LadderString) -> Option<LadderElement> {
    let value = ladder_string.value.trim();
    if value.is_empty() {
        return None;
    }

    let parts = value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    if parts.len() > 1 && is_ladder_operation(parts[0]) {
        return Some(LadderElement {
            offset: ladder_string.offset,
            kind: ladder_operation_kind(parts[0]),
            value: parts[0].to_owned(),
            operands: parts[1..].iter().map(|part| (*part).to_owned()).collect(),
            contact: None,
            coil: None,
        });
    }

    if is_ladder_operation(value) {
        return Some(LadderElement {
            offset: ladder_string.offset,
            kind: ladder_operation_kind(value),
            value: value.to_owned(),
            operands: Vec::new(),
            contact: None,
            coil: None,
        });
    }

    if looks_like_device_ref(value) {
        let coil = ladder_coil(data, ladder_string.offset);
        return Some(LadderElement {
            offset: ladder_string.offset,
            kind: LadderElementKind::DeviceRef,
            value: value.to_owned(),
            operands: Vec::new(),
            contact: coil
                .is_none()
                .then(|| ladder_contact(data, ladder_string.offset))
                .flatten(),
            coil,
        });
    }

    if looks_like_internal_ref(value) {
        let coil = ladder_coil(data, ladder_string.offset);
        return Some(LadderElement {
            offset: ladder_string.offset,
            kind: LadderElementKind::InternalRef,
            value: value.to_owned(),
            operands: Vec::new(),
            contact: coil
                .is_none()
                .then(|| ladder_contact(data, ladder_string.offset))
                .flatten(),
            coil,
        });
    }

    if looks_like_constant(value) {
        return Some(LadderElement {
            offset: ladder_string.offset,
            kind: LadderElementKind::Constant,
            value: value.to_owned(),
            operands: Vec::new(),
            contact: None,
            coil: None,
        });
    }

    Some(LadderElement {
        offset: ladder_string.offset,
        kind: LadderElementKind::Comment,
        value: value.to_owned(),
        operands: Vec::new(),
        contact: None,
        coil: None,
    })
}

fn ladder_contact(data: &[u8], string_offset: usize) -> Option<LadderContact> {
    [15, 14, 16].iter().find_map(|back| {
        let marker = string_offset.checked_sub(*back)?;
        match (data.get(marker).copied(), data.get(marker + 1).copied()) {
            (Some(0xff), Some(0x06)) => Some(LadderContact::NormallyOpen),
            (Some(0xff), Some(0x07)) => Some(LadderContact::NormallyClosed),
            _ => None,
        }
    })
}

fn ladder_coil(data: &[u8], string_offset: usize) -> Option<LadderCoil> {
    [15, 16, 14].iter().find_map(|back| {
        let marker = string_offset.checked_sub(*back)?;
        match (data.get(marker).copied(), data.get(marker + 1).copied()) {
            (Some(0xff), Some(0x0e)) => Some(LadderCoil::Output),
            (Some(0xff), Some(0x10)) => Some(LadderCoil::Set),
            (Some(0xff), Some(0x11)) => Some(LadderCoil::Reset),
            _ => None,
        }
    })
}

fn parse_ladder_instruction(ladder_string: &LadderString) -> Option<LadderInstruction> {
    let parts = ladder_string
        .value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    if parts.len() < 2 || !looks_like_mnemonic(parts[0]) {
        return None;
    }

    Some(LadderInstruction {
        offset: ladder_string.offset,
        mnemonic: parts[0].to_owned(),
        operands: parts[1..].iter().map(|part| (*part).to_owned()).collect(),
        raw: ladder_string.value.clone(),
    })
}

fn duplicate_decomposed_string_count(
    strings: &[LadderString],
    index: usize,
    element: &LadderElement,
) -> usize {
    if element.operands.is_empty()
        && is_ladder_operation(&element.value)
        && strings
            .get(index + 1)
            .is_some_and(|string| string.value == element.value)
    {
        return 1;
    }

    if element.operands.is_empty() || !is_ladder_operation(&element.value) {
        return 0;
    }

    let expected = std::iter::once(element.value.as_str())
        .chain(element.operands.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let Some(following) = strings.get(index + 1..index + 1 + expected.len()) else {
        return 0;
    };

    if following
        .iter()
        .zip(expected)
        .all(|(string, expected)| string.value == expected)
    {
        following.len()
    } else {
        0
    }
}

fn is_ladder_operation(value: &str) -> bool {
    matches!(
        value,
        "=" | "<>"
            | ">"
            | "<"
            | ">="
            | "<="
            | "AND"
            | "OR"
            | "XOR"
            | "NOT"
            | "SET"
            | "RST"
            | "RESET"
            | "OUT"
            | "OUTP"
            | "FF"
            | "TON"
            | "TOFF"
            | "TMR"
            | "CTU"
            | "CTD"
            | "MOV"
            | "MOVP"
            | "FMOV"
            | "FMOVP"
            | "DMOV"
            | "RMOV"
            | "I2R"
            | "R2I"
            | "RADD"
            | "RSUB"
            | "RMUL"
            | "RDIV"
            | "ADD"
            | "SUB"
            | "MUL"
            | "DIV"
            | "DADD"
            | "DSUB"
            | "DMUL"
            | "DDIV"
            | "GETM"
            | "FOR"
            | "NEXT"
            | "DNEGP"
            | "END"
    )
}

fn ladder_operation_kind(value: &str) -> LadderElementKind {
    match value {
        "=" | "<>" | ">" | "<" | ">=" | "<=" => LadderElementKind::Comparison,
        "TON" | "TOFF" | "TMR" | "CTU" | "CTD" => LadderElementKind::Timer,
        "AND" | "OR" | "XOR" | "NOT" => LadderElementKind::Logic,
        "SET" | "RST" | "RESET" | "OUT" | "OUTP" | "FF" => LadderElementKind::Operation,
        _ => LadderElementKind::InstructionCall,
    }
}

fn looks_like_device_ref(value: &str) -> bool {
    let Some(first) = value.chars().next() else {
        return false;
    };

    matches!(
        first,
        'M' | 'P' | 'T' | 'C' | 'D' | 'R' | 'U' | 'Z' | 'X' | 'Y' | 'K' | 'L' | 'N'
    ) && value
        .chars()
        .skip(1)
        .all(|ch| ch.is_ascii_hexdigit() || ch == '.' || ch == '[' || ch == ']')
        && value.chars().any(|ch| ch.is_ascii_digit())
}

fn looks_like_internal_ref(value: &str) -> bool {
    value.len() == 6
        && value.starts_with('F')
        && value.chars().skip(1).all(|ch| ch.is_ascii_hexdigit())
}

fn looks_like_constant(value: &str) -> bool {
    if value.starts_with('h') || value.starts_with('H') {
        return value.len() > 1 && value.chars().skip(1).all(|ch| ch.is_ascii_hexdigit());
    }

    value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok()
}

fn looks_like_mnemonic(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    first.is_ascii_alphabetic()
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && value.chars().any(|ch| ch.is_ascii_alphabetic())
}

/// Errors returned while parsing `.xgwx` files.
#[derive(Debug)]
pub enum XgwxError {
    Io(io::Error),
    InvalidMagic,
    MissingMainPayload,
    InvalidGzipHeader {
        offset: usize,
    },
    UnsupportedGzipCompression {
        offset: usize,
        method: u8,
    },
    ReservedGzipFlags {
        offset: usize,
        flags: u8,
    },
    TruncatedGzipMember {
        offset: usize,
    },
    Inflate(flate2::DecompressError),
    GzipCrcMismatch {
        offset: usize,
        expected: u32,
        actual: u32,
    },
    GzipSizeMismatch {
        offset: usize,
        expected: u32,
        actual: u32,
    },
    MissingProgramData,
    Base64(base64::DecodeError),
    InvalidHexPayload {
        element: String,
        attribute: String,
    },
    Bzip2(io::Error),
    Utf8(std::string::FromUtf8Error),
    Xml(roxmltree::Error),
}

impl fmt::Display for XgwxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "failed to read xgwx file: {error}"),
            Self::InvalidMagic => write!(f, "file does not start with XG magic"),
            Self::MissingMainPayload => write!(f, "missing gzip-compressed XML payload"),
            Self::InvalidGzipHeader { offset } => write!(f, "invalid gzip header at byte {offset}"),
            Self::UnsupportedGzipCompression { offset, method } => {
                write!(
                    f,
                    "unsupported gzip compression method {method} at byte {offset}"
                )
            }
            Self::ReservedGzipFlags { offset, flags } => {
                write!(f, "reserved gzip flags 0x{flags:02x} at byte {offset}")
            }
            Self::TruncatedGzipMember { offset } => {
                write!(f, "truncated gzip member at byte {offset}")
            }
            Self::Inflate(error) => write!(f, "failed to inflate gzip payload: {error}"),
            Self::GzipCrcMismatch {
                offset,
                expected,
                actual,
            } => write!(
                f,
                "gzip CRC mismatch at byte {offset}: expected 0x{expected:08x}, got 0x{actual:08x}"
            ),
            Self::GzipSizeMismatch {
                offset,
                expected,
                actual,
            } => write!(
                f,
                "gzip size mismatch at byte {offset}: expected {expected}, got {actual}"
            ),
            Self::MissingProgramData => write!(f, "program is missing a ProgramData element"),
            Self::Base64(error) => write!(f, "failed to base64-decode ProgramData: {error}"),
            Self::InvalidHexPayload { element, attribute } => {
                write!(f, "{element} {attribute} is not valid hex ASCII payload")
            }
            Self::Bzip2(error) => write!(f, "failed to bzip2-decompress ProgramData: {error}"),
            Self::Utf8(error) => write!(f, "XML payload is not valid UTF-8: {error}"),
            Self::Xml(error) => write!(f, "XML payload is not well-formed: {error}"),
        }
    }
}

impl std::error::Error for XgwxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Inflate(error) => Some(error),
            Self::Base64(error) => Some(error),
            Self::Bzip2(error) => Some(error),
            Self::Utf8(error) => Some(error),
            Self::Xml(error) => Some(error),
            _ => None,
        }
    }
}

fn parse_header_label(raw: &[u8]) -> (Option<String>, Option<u32>) {
    let Some(marker_offset) = raw
        .windows(3)
        .position(|window| window == [0xff, 0xfe, 0xff])
    else {
        return (None, None);
    };

    let Some(unit_count) = raw.get(marker_offset + 3).copied().map(usize::from) else {
        return (None, None);
    };

    let start = marker_offset + 4;
    let end = start + unit_count.saturating_mul(2);
    let Some(encoded) = raw.get(start..end) else {
        return (None, None);
    };

    let mut utf16_units = Vec::with_capacity(unit_count);
    for unit in encoded.chunks_exact(2) {
        utf16_units.push(u16::from_le_bytes([unit[0], unit[1]]));
    }

    let label = String::from_utf16(&utf16_units).ok();
    let label_following_u32 = raw
        .get(end..end + 4)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes);

    (label, label_following_u32)
}

fn find_gzip_member(bytes: &[u8], start: usize) -> Option<usize> {
    bytes
        .get(start..)?
        .windows(GZIP_MAGIC.len())
        .position(|window| window == GZIP_MAGIC.as_slice())
        .map(|relative| start + relative)
}

fn find_trailer_gzip_members(bytes: &[u8], trailer_start: usize) -> Vec<GzipMember> {
    let mut members = Vec::new();
    let mut search_from = trailer_start;

    while let Some(offset) = find_gzip_member(bytes, search_from) {
        match parse_gzip_member(bytes, offset) {
            Ok(member) => {
                search_from = member.end_offset;
                members.push(member);
            }
            Err(_) => {
                search_from = offset + 1;
            }
        }
    }

    members
}

fn parse_gzip_member(bytes: &[u8], offset: usize) -> Result<GzipMember, XgwxError> {
    let deflate_start = parse_gzip_header(bytes, offset)?;
    let mut decompressor = Decompress::new(false);
    let mut data = Vec::new();
    let mut input = &bytes[deflate_start..];
    let mut output = [0; 8192];

    loop {
        let total_in_before = decompressor.total_in();
        let total_out_before = decompressor.total_out();
        let status = decompressor
            .decompress(input, &mut output, FlushDecompress::None)
            .map_err(XgwxError::Inflate)?;
        let consumed = usize::try_from(decompressor.total_in() - total_in_before)
            .map_err(|_| XgwxError::TruncatedGzipMember { offset })?;
        let produced = usize::try_from(decompressor.total_out() - total_out_before)
            .map_err(|_| XgwxError::TruncatedGzipMember { offset })?;

        data.extend_from_slice(&output[..produced]);
        input = &input[consumed..];

        if status == Status::StreamEnd {
            break;
        }

        if consumed == 0 && produced == 0 {
            return Err(XgwxError::TruncatedGzipMember { offset });
        }
    }

    let deflate_len = usize::try_from(decompressor.total_in())
        .map_err(|_| XgwxError::TruncatedGzipMember { offset })?;
    let trailer_start = deflate_start + deflate_len;
    let end_offset = trailer_start + GZIP_TRAILER_LEN;
    let trailer = bytes
        .get(trailer_start..end_offset)
        .ok_or(XgwxError::TruncatedGzipMember { offset })?;

    validate_gzip_trailer(offset, trailer, &data)?;

    Ok(GzipMember {
        offset,
        end_offset,
        data,
    })
}

fn parse_gzip_header(bytes: &[u8], offset: usize) -> Result<usize, XgwxError> {
    let header = bytes
        .get(offset..offset + GZIP_HEADER_LEN)
        .ok_or(XgwxError::TruncatedGzipMember { offset })?;

    if &header[0..2] != GZIP_MAGIC {
        return Err(XgwxError::InvalidGzipHeader { offset });
    }

    let method = header[2];
    if method != GZIP_METHOD_DEFLATE {
        return Err(XgwxError::UnsupportedGzipCompression { offset, method });
    }

    let flags = header[3];
    if flags & 0b1110_0000 != 0 {
        return Err(XgwxError::ReservedGzipFlags { offset, flags });
    }

    let mut cursor = offset + GZIP_HEADER_LEN;

    if flags & 0x04 != 0 {
        let xlen_bytes = bytes
            .get(cursor..cursor + 2)
            .ok_or(XgwxError::TruncatedGzipMember { offset })?;
        let xlen = u16::from_le_bytes([xlen_bytes[0], xlen_bytes[1]]) as usize;
        cursor += 2 + xlen;
        if cursor > bytes.len() {
            return Err(XgwxError::TruncatedGzipMember { offset });
        }
    }

    if flags & 0x08 != 0 {
        cursor = skip_zero_terminated(bytes, cursor, offset)?;
    }

    if flags & 0x10 != 0 {
        cursor = skip_zero_terminated(bytes, cursor, offset)?;
    }

    if flags & 0x02 != 0 {
        cursor += 2;
        if cursor > bytes.len() {
            return Err(XgwxError::TruncatedGzipMember { offset });
        }
    }

    Ok(cursor)
}

fn skip_zero_terminated(
    bytes: &[u8],
    cursor: usize,
    gzip_offset: usize,
) -> Result<usize, XgwxError> {
    let Some(relative_end) = bytes
        .get(cursor..)
        .and_then(|remaining| remaining.iter().position(|byte| *byte == 0))
    else {
        return Err(XgwxError::TruncatedGzipMember {
            offset: gzip_offset,
        });
    };

    Ok(cursor + relative_end + 1)
}

fn validate_gzip_trailer(offset: usize, trailer: &[u8], data: &[u8]) -> Result<(), XgwxError> {
    let expected_crc = u32::from_le_bytes([trailer[0], trailer[1], trailer[2], trailer[3]]);
    let actual_crc = crc32fast::hash(data);
    if expected_crc != actual_crc {
        return Err(XgwxError::GzipCrcMismatch {
            offset,
            expected: expected_crc,
            actual: actual_crc,
        });
    }

    let expected_size = u32::from_le_bytes([trailer[4], trailer[5], trailer[6], trailer[7]]);
    let actual_size = data.len() as u32;
    if expected_size != actual_size {
        return Err(XgwxError::GzipSizeMismatch {
            offset,
            expected: expected_size,
            actual: actual_size,
        });
    }

    Ok(())
}

fn parse_xml(xml: &str) -> Result<XmlElement, XgwxError> {
    let document = roxmltree::Document::parse(xml).map_err(XgwxError::Xml)?;
    let root = document.root_element();
    Ok(xml_element_from_node(root))
}

fn xml_element_from_node(node: roxmltree::Node<'_, '_>) -> XmlElement {
    let name = node.tag_name().name().to_owned();
    let attributes = node
        .attributes()
        .map(|attribute| XmlAttribute {
            name: attribute.name().to_owned(),
            value: attribute.value().to_owned(),
        })
        .collect();
    let text = node
        .children()
        .filter(|child| child.is_text())
        .filter_map(|child| child.text())
        .collect();
    let children = node
        .children()
        .filter(|child| child.is_element())
        .map(xml_element_from_node)
        .collect();

    XmlElement {
        name,
        attributes,
        text,
        children,
    }
}

struct XmlDescendantsNamed<'a> {
    name: &'a str,
    stack: Vec<&'a XmlElement>,
}

impl<'a> XmlDescendantsNamed<'a> {
    fn new(root: &'a XmlElement, name: &'a str) -> Self {
        Self {
            name,
            stack: vec![root],
        }
    }
}

impl<'a> Iterator for XmlDescendantsNamed<'a> {
    type Item = &'a XmlElement;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(element) = self.stack.pop() {
            self.stack.extend(element.children.iter().rev());
            if element.name == self.name {
                return Some(element);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::env;
    use std::io::Write;

    #[test]
    fn parses_synthetic_workspace() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Project FileVer="1.2.3.4" GUID="synthetic-guid" Version="513" Attribute="7" WksNodeCount="1">Synthetic
  <Configuration GUID="config-guid">PLC</Configuration>
  <Network Name="Network" Type="Ethernet" NetworkType="FEnet">
    <NetworkModule ConfigName="PLC" Base="0" Slot="1" ID="42" />
  </Network>
  <Program Task="scan" ObjectID="1" Version="2" Kind="0">Main</Program>
</Project>"#;
        let doc = XgwxDocument::parse(&synthetic_xgwx_bytes(xml)).expect("synthetic parses");

        assert!(doc.header.gzip_offset > 0);
        assert_eq!(doc.header.label.as_deref(), Some("XG5000 WORKSPACE FILE"));
        assert_eq!(doc.header.label_following_u32, Some(1));
        assert_eq!(doc.root.name, "Project");
        assert_eq!(doc.project_name(), Some("Synthetic"));
        assert_eq!(doc.project_info().file_version.as_deref(), Some("1.2.3.4"));
        assert_eq!(doc.configurations().len(), 1);
        assert_eq!(doc.networks()[0].modules.len(), 1);
        assert_eq!(doc.programs()[0].name.as_deref(), Some("Main"));
    }

    #[test]
    fn decodes_synthetic_ladder_records() {
        let data = synthetic_ladder_data();
        let strings = extract_ladder_strings(&data);
        let elements = extract_ladder_elements(&data, &strings);
        let structure = extract_ladder_structure(&data, &elements);

        assert!(elements.iter().any(|element| {
            element.kind == LadderElementKind::DeviceRef
                && element.value == "M00001"
                && element.contact == Some(LadderContact::NormallyOpen)
        }));
        assert!(elements.iter().any(|element| {
            element.kind == LadderElementKind::DeviceRef
                && element.value == "M00002"
                && element.contact == Some(LadderContact::NormallyClosed)
        }));
        assert!(elements.iter().any(|element| {
            element.kind == LadderElementKind::DeviceRef
                && element.value == "M00003"
                && element.coil == Some(LadderCoil::Output)
        }));
        assert!(elements.iter().any(|element| {
            element.kind == LadderElementKind::DeviceRef
                && element.value == "M00004"
                && element.coil == Some(LadderCoil::Reset)
        }));
        assert!(elements.iter().any(|element| {
            element.kind == LadderElementKind::InstructionCall
                && element.value == "MOV"
                && element.operands == ["D000001", "D000002"]
        }));
        assert!(structure.vertical_lines.iter().any(|line| (
            line.raw_x,
            line.raw_y_start,
            line.raw_y_end
        ) == (0x03, 0x00, 0x04)));
        assert!(structure.horizontal_lines.iter().any(|line| (
            line.raw_y,
            line.raw_x_start,
            line.raw_x_end
        ) == (0x00, 0x01, 0x58)));
    }

    #[test]
    fn decodes_synthetic_hsc_parameter_counter_modes() {
        let mut payload = vec!['0'; 448];
        payload[1] = '1';
        payload[6] = '1';
        payload[7] = '2';
        payload[8] = '3';
        payload[9] = '4';
        payload[16] = '1';
        payload[17] = '8';
        payload[18] = '2';
        payload[19] = 'A';
        payload[40] = 'C';
        payload[41] = '8';
        payload[48] = '1';
        payload[49] = '4';
        payload[56] = '6';
        payload[57] = '4';
        payload[88] = '8';
        payload[89] = '8';
        payload[90] = '1';
        payload[91] = '3';
        payload[92] = '0';
        payload[93] = '4';
        let payload = payload.into_iter().collect::<String>();
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Project>
  <Parameters>
    <Parameter Type="HSC PARAMETER" PAYLOAD_ASC_LENGTH="448" PAYLOAD="{payload}" />
  </Parameters>
</Project>"#
        );
        let doc = XgwxDocument::parse(&synthetic_xgwx_bytes(&xml)).expect("synthetic parses");
        let hsc = doc
            .hsc_parameters()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .expect("hsc payload decodes");

        assert_eq!(hsc.len(), 1);
        assert_eq!(hsc[0].payload_asc_length, Some(448));
        assert_eq!(hsc[0].payload_bytes.len(), 224);
        assert_eq!(hsc[0].initial_unknown_nibble, Some(0));
        assert_eq!(
            hsc[0]
                .channels
                .iter()
                .map(|channel| channel.counter_mode_raw)
                .collect::<Vec<_>>(),
            [Some(1), Some(0), Some(0), Some(0)]
        );
        assert_eq!(
            hsc[0]
                .channels
                .iter()
                .map(|channel| channel.pulse_input_mode_raw)
                .collect::<Vec<_>>(),
            [Some(0), Some(1), Some(2), Some(3)]
        );
        assert_eq!(
            hsc[0].channels[0].counter_mode,
            Some(HscCounterMode::RingCounter)
        );
        assert_eq!(
            hsc[0].channels[1].counter_mode,
            Some(HscCounterMode::LinearCounter)
        );
        assert_eq!(
            hsc[0].channels[0].pulse_input_mode,
            Some(HscPulseInputMode::OnePhaseOneInputOneX)
        );
        assert_eq!(
            hsc[0].channels[1].pulse_input_mode,
            Some(HscPulseInputMode::OnePhaseTwoInputOneX)
        );
        assert_eq!(
            hsc[0].channels[2].pulse_input_mode,
            Some(HscPulseInputMode::CwCcw)
        );
        assert_eq!(
            hsc[0].channels[3].pulse_input_mode,
            Some(HscPulseInputMode::TwoPhaseFourX)
        );
        assert_eq!(hsc[0].channels[0].compare_output_mode_raw, Some(4));
        assert_eq!(
            hsc[0].channels[0].compare_output_mode,
            Some(HscCompareOutputMode::GreaterThan)
        );
        assert_eq!(hsc[0].channels[0].internal_preset, Some(0x18));
        assert_eq!(hsc[0].channels[0].external_preset, Some(0x2a));
        assert_eq!(hsc[0].channels[0].ring_counter_max, Some(200));
        assert_eq!(hsc[0].channels[0].compare_output_min, Some(20));
        assert_eq!(hsc[0].channels[0].compare_output_max, Some(100));
        assert_eq!(hsc[0].channels[0].unit_time_ms, Some(5000));
        assert_eq!(hsc[0].channels[0].pulses_per_revolution, Some(4));
        assert!(
            hsc[0]
                .channels
                .iter()
                .all(|channel| channel.raw.len() == 56)
        );
    }

    #[test]
    fn decodes_xgb_enet01_hsc_parameter_counter_modes() {
        let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").expect("fixture parses");
        let hsc = doc
            .hsc_parameters()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .expect("hsc payload decodes");

        assert_eq!(hsc.len(), 1);
        assert_eq!(hsc[0].payload_asc_length, Some(448));
        assert_eq!(hsc[0].payload_bytes.len(), 224);
        assert_eq!(hsc[0].initial_unknown_nibble, Some(0));
        assert_eq!(
            hsc[0]
                .channels
                .iter()
                .map(|channel| channel.counter_mode_raw)
                .collect::<Vec<_>>(),
            [Some(1), Some(0), Some(0), Some(0)]
        );
        assert_eq!(
            hsc[0]
                .channels
                .iter()
                .map(|channel| channel.pulse_input_mode_raw)
                .collect::<Vec<_>>(),
            [Some(2), Some(0), Some(0), Some(0)]
        );
        assert_eq!(
            hsc[0].channels[0].counter_mode,
            Some(HscCounterMode::RingCounter)
        );
        assert_eq!(
            hsc[0].channels[0].pulse_input_mode,
            Some(HscPulseInputMode::CwCcw)
        );
        assert_eq!(hsc[0].channels[0].compare_output_mode_raw, Some(4));
        assert_eq!(
            hsc[0].channels[0].compare_output_mode,
            Some(HscCompareOutputMode::GreaterThan)
        );
        assert_eq!(hsc[0].channels[0].internal_preset, Some(0x18));
        assert_eq!(hsc[0].channels[0].external_preset, Some(0));
        assert_eq!(hsc[0].channels[0].ring_counter_max, Some(200));
        assert_eq!(hsc[0].channels[0].compare_output_min, Some(20));
        assert_eq!(hsc[0].channels[0].compare_output_max, Some(100));
        assert_eq!(hsc[0].channels[0].unit_time_ms, Some(5000));
        assert_eq!(hsc[0].channels[0].pulses_per_revolution, Some(4));
        assert!(
            hsc[0].channels[1..]
                .iter()
                .all(|channel| channel.counter_mode == Some(HscCounterMode::LinearCounter))
        );
        assert!(hsc[0].channels[1..].iter().all(|channel| {
            channel.pulse_input_mode == Some(HscPulseInputMode::OnePhaseOneInputOneX)
        }));
    }

    #[test]
    fn decodes_xgb_enet01_position_parameter_axes() {
        let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").expect("fixture parses");
        let position = doc.position_parameters();

        assert_eq!(position.len(), 1);
        assert_eq!(position[0].axis_count, Some(2));
        assert_eq!(position[0].axes.len(), 2);
        assert_eq!(position[0].axes[0].axis_name, "X");
        assert_eq!(position[0].axes[1].axis_name, "Y");

        for axis in &position[0].axes {
            assert_eq!(axis.step_count, Some(30));
            assert_eq!(axis.steps.len(), 30);
            assert_eq!(axis.steps[0].target_position, Some(0));
            assert_eq!(axis.steps[0].operation_velocity, Some(0));

            let parameter = axis.parameter.as_ref().expect("axis parameter");
            assert_eq!(parameter.bias_velocity, Some(1));
            assert_eq!(parameter.velocity_limit, Some(100000));
            assert_eq!(
                parameter.accel_times,
                [Some(500), Some(1000), Some(1500), Some(2000)]
            );
            assert_eq!(
                parameter.decel_times,
                [Some(500), Some(1000), Some(1500), Some(2000)]
            );
            assert_eq!(parameter.soft_upper_limit, Some(i32::MAX));
            assert_eq!(parameter.soft_lower_limit, Some(i32::MIN));
            assert_eq!(parameter.s_curve_ratio, Some(50));
            assert_eq!(parameter.use_limit, Some(1));
            assert_eq!(parameter.return_velocity_high, Some(5000));
            assert_eq!(parameter.return_velocity_low, Some(500));
            assert_eq!(parameter.jog_velocity_high, Some(5000));
            assert_eq!(parameter.jog_velocity_low, Some(1000));
        }
    }

    #[test]
    fn decodes_xgb_enet01_pid_parameters() {
        let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").expect("fixture parses");
        let cal = doc.pid_cal_parameters();
        let tune = doc.pid_tune_parameters();

        assert_eq!(cal.len(), 1);
        assert_eq!(cal[0].header, [Some(17736), Some(17473)]);
        assert_eq!(cal[0].set_pid_out, Some(0));
        assert_eq!(cal[0].prevent_anti_windup, Some(0));
        assert_eq!(cal[0].differential_control_method, Some(65535));
        assert_eq!(cal[0].loops.len(), 16);
        assert_eq!(cal[0].loops[0].target_value, Some(0));
        assert_eq!(cal[0].loops[0].scan_time, Some(100));
        assert_eq!(cal[0].loops[0].proportional_gain_left, Some(1));
        assert_eq!(cal[0].loops[0].mv_max, Some(4000));
        assert_eq!(cal[0].loops[0].mv_min, Some(0));
        assert_eq!(cal[0].loops[0].forward_pwm, Some(32));
        assert_eq!(cal[0].loops[0].pwm_out_period, Some(100));
        assert_eq!(cal[0].loops[0].pv_max, Some(4000));

        assert_eq!(tune.len(), 1);
        assert_eq!(tune[0].set_direction, Some(0));
        assert_eq!(tune[0].permit_pwm, Some(0));
        assert_eq!(tune[0].checksum, Some(0));
        assert_eq!(tune[0].footer, [Some(12358), Some(21552)]);
        assert_eq!(tune[0].loops.len(), 16);
        assert_eq!(tune[0].loops[0].target_value, Some(0));
        assert_eq!(tune[0].loops[0].scan_time, Some(100));
        assert_eq!(tune[0].loops[0].mv_max, Some(4000));
        assert_eq!(tune[0].loops[0].mv_min, Some(0));
        assert_eq!(tune[0].loops[0].set_pwm_at_point, Some(32));
        assert_eq!(tune[0].loops[0].out_period, Some(100));
        assert_eq!(tune[0].loops[0].hysteresis, Some(10));
    }

    #[test]
    fn decodes_xgb_enet01_fenet_ipv4_parameters() {
        let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").expect("fixture parses");
        let fenet = doc.fenet_config_infos();

        assert_eq!(fenet.len(), 1);
        assert_eq!(fenet[0].type_code, Some(23041));
        assert_eq!(fenet[0].base, Some(0));
        assert_eq!(fenet[0].slot, Some(1));
        assert_eq!(fenet[0].sub_type, Some(32771));
        assert_eq!(
            fenet[0].ip_address.as_ref().map(|ip| ip.address.as_str()),
            Some("192.168.0.100")
        );
        assert_eq!(
            fenet[0].subnet.as_ref().map(|ip| ip.address.as_str()),
            Some("255.255.255.0")
        );
        assert_eq!(
            fenet[0].gateway.as_ref().map(|ip| ip.address.as_str()),
            Some("192.168.0.1")
        );
        assert_eq!(
            fenet[0].dns.as_ref().map(|ip| ip.address.as_str()),
            Some("0.0.0.0")
        );
        assert_eq!(
            fenet[0].ip_address2.as_ref().map(|ip| ip.address.as_str()),
            Some("0.0.0.0")
        );
    }

    #[test]
    fn decodes_xgb_enet01_cnet_port_parameters() {
        let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").expect("fixture parses");
        let cnet = doc.cnet_config_infos();

        assert_eq!(cnet.len(), 1);
        assert_eq!(cnet[0].type_code, Some(23104));
        assert_eq!(cnet[0].base, Some(0));
        assert_eq!(cnet[0].slot, Some(0));
        assert_eq!(cnet[0].sub_type, Some(32773));
        assert_eq!(cnet[0].ports.len(), 2);

        assert_eq!(cnet[0].ports[0].station_no, Some(5));
        assert_eq!(cnet[0].ports[0].mode, Some(0));
        assert_eq!(cnet[0].ports[0].mode_kind, Some(CnetMode::Rs232C));
        assert_eq!(cnet[0].ports[0].bps, Some(8));
        assert_eq!(cnet[0].ports[0].baud_rate, Some(9600));
        assert_eq!(cnet[0].ports[0].data_bit, Some(1));
        assert_eq!(cnet[0].ports[0].data_bits, Some(CnetDataBits::Eight));
        assert_eq!(cnet[0].ports[0].stop_bit, Some(0));
        assert_eq!(cnet[0].ports[0].stop_bits, Some(CnetStopBits::One));
        assert_eq!(cnet[0].ports[0].parity, Some(0));
        assert_eq!(cnet[0].ports[0].parity_mode, Some(CnetParity::None));
        assert_eq!(cnet[0].ports[0].driver_type, Some(2));
        assert_eq!(cnet[0].ports[0].do_addr, Some(20));
        assert_eq!(cnet[0].ports[0].do_device, Some('P'));
        assert_eq!(cnet[0].ports[0].do_address.as_deref(), Some("P00014"));
        assert_eq!(cnet[0].ports[0].ai_addr, Some(20));
        assert_eq!(cnet[0].ports[0].ai_device, Some('P'));
        assert_eq!(cnet[0].ports[0].ai_address.as_deref(), Some("P00020"));
        assert_eq!(cnet[0].ports[0].ao_addr, Some(30));
        assert_eq!(cnet[0].ports[0].ao_address.as_deref(), Some("P00030"));

        assert_eq!(cnet[0].ports[1].station_no, Some(15));
        assert_eq!(cnet[0].ports[1].mode, Some(2));
        assert_eq!(cnet[0].ports[1].mode_kind, Some(CnetMode::Rs485));
        assert_eq!(cnet[0].ports[1].do_addr, Some(400));
        assert_eq!(cnet[0].ports[1].do_device, Some('M'));
        assert_eq!(cnet[0].ports[1].do_address.as_deref(), Some("M00250"));
    }

    #[test]
    fn decodes_xgb_enet02_hsc_position_and_pid_parameters() {
        let doc = XgwxDocument::from_path("fixtures/XGB_Enet02.xgwx").expect("fixture parses");

        let hsc = doc
            .hsc_parameters()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .expect("hsc payload decodes");
        assert_eq!(hsc.len(), 1);
        assert_eq!(hsc[0].payload_asc_length, Some(448));
        assert_eq!(
            hsc[0].channels[0].counter_mode,
            Some(HscCounterMode::RingCounter)
        );
        assert_eq!(
            hsc[0].channels[0].pulse_input_mode,
            Some(HscPulseInputMode::CwCcw)
        );
        assert_eq!(
            hsc[0].channels[0].compare_output_mode,
            Some(HscCompareOutputMode::GreaterThan)
        );
        assert_eq!(hsc[0].channels[0].ring_counter_max, Some(200));
        assert_eq!(hsc[0].channels[0].unit_time_ms, Some(5000));
        assert_eq!(hsc[0].channels[0].pulses_per_revolution, Some(4));

        let position = doc.position_parameters();
        assert_eq!(position.len(), 1);
        assert_eq!(position[0].axis_count, Some(2));
        assert_eq!(position[0].axes.len(), 2);
        assert_eq!(position[0].axes[0].axis_name, "X");
        assert_eq!(position[0].axes[1].axis_name, "Y");

        let cal = doc.pid_cal_parameters();
        let tune = doc.pid_tune_parameters();
        assert_eq!(cal.len(), 1);
        assert_eq!(tune.len(), 1);
        assert_eq!(cal[0].loops.len(), 16);
        assert_eq!(tune[0].loops.len(), 16);
        assert_eq!(cal[0].loops[0].forward_pwm, Some(32));
        assert_eq!(tune[0].loops[0].set_pwm_at_point, Some(32));
    }

    #[test]
    fn decodes_xgb_enet02_fenet_ipv4_parameters() {
        let doc = XgwxDocument::from_path("fixtures/XGB_Enet02.xgwx").expect("fixture parses");
        let fenet = doc.fenet_config_infos();

        assert_eq!(fenet.len(), 1);
        assert_eq!(fenet[0].station_no, Some(1));
        assert_eq!(fenet[0].type_code, Some(23041));
        assert_eq!(fenet[0].base, Some(0));
        assert_eq!(fenet[0].slot, Some(1));
        assert_eq!(fenet[0].sub_type, Some(32771));
        assert_eq!(fenet[0].driver_type, Some(5));
        assert_eq!(fenet[0].rcv_wait_time, Some(20));
        assert_eq!(
            fenet[0].ip_address.as_ref().map(|ip| ip.address.as_str()),
            Some("192.168.0.100")
        );
        assert_eq!(
            fenet[0].subnet.as_ref().map(|ip| ip.address.as_str()),
            Some("255.255.255.0")
        );
        assert_eq!(
            fenet[0].gateway.as_ref().map(|ip| ip.address.as_str()),
            Some("192.168.0.1")
        );
    }

    #[test]
    fn decodes_xgb_enet02_cnet_port_parameters() {
        let doc = XgwxDocument::from_path("fixtures/XGB_Enet02.xgwx").expect("fixture parses");
        let cnet = doc.cnet_config_infos();

        assert_eq!(cnet.len(), 1);
        assert_eq!(cnet[0].type_code, Some(23104));
        assert_eq!(cnet[0].sub_type, Some(32773));
        assert_eq!(cnet[0].ports.len(), 2);
        assert_eq!(cnet[0].ports[0].station_no, Some(5));
        assert_eq!(cnet[0].ports[0].mode_kind, Some(CnetMode::Rs232C));
        assert_eq!(cnet[0].ports[0].baud_rate, Some(9600));
        assert_eq!(cnet[0].ports[0].data_bits, Some(CnetDataBits::Eight));
        assert_eq!(cnet[0].ports[0].stop_bits, Some(CnetStopBits::One));
        assert_eq!(cnet[0].ports[0].parity_mode, Some(CnetParity::None));
        assert_eq!(cnet[0].ports[0].do_address.as_deref(), Some("P00014"));
        assert_eq!(cnet[0].ports[0].ai_address.as_deref(), Some("P00020"));
        assert_eq!(cnet[0].ports[0].ao_address.as_deref(), Some("P00030"));
        assert_eq!(cnet[0].ports[1].station_no, Some(15));
        assert_eq!(cnet[0].ports[1].mode_kind, Some(CnetMode::Rs485));
        assert_eq!(cnet[0].ports[1].do_address.as_deref(), Some("M00250"));
    }

    #[test]
    #[ignore = "set LIBXGWX_FIXTURE=/path/to/file.xgwx to run against a real workspace"]
    fn parses_real_fixture_from_env() {
        let Ok(path) = env::var("LIBXGWX_FIXTURE") else {
            return;
        };
        let doc = XgwxDocument::from_path(path).expect("fixture parses");

        assert_eq!(doc.root.name, "Project");
        assert!(!doc.xml.is_empty());
        let _ = doc.project_info();
        let _ = doc.configurations();
        let _ = doc.networks();
        let _ = doc.network_modules();
        let _ = doc.bases();
        let _ = doc.modules();
        let _ = doc.tasks();
        let _ = doc.programs();
        let _ = doc.variables();
        let _ = doc.ladder_programs();
        let _ = doc.hsc_parameters();
        let _ = doc.position_parameters();
        let _ = doc.pid_cal_parameters();
        let _ = doc.pid_tune_parameters();
        let _ = doc.cnet_config_infos();
        let _ = doc.fenet_config_infos();
        let _ = doc.decoded_payloads();
    }

    #[test]
    fn formats_xgwx_variable_addresses() {
        assert_eq!(format_ipv4_le(352364736), "192.168.0.21");
        assert_eq!(format_ipv4_le(16820416), "192.168.0.1");
        assert_eq!(format_ipv4_le(16777215), "255.255.255.0");
        assert_eq!(
            format_variable_address(Some("M"), Some(14), Some("BIT"), None).as_deref(),
            Some("M0000E")
        );
        assert_eq!(
            format_variable_address(Some("P"), Some(14), Some("BIT"), None).as_deref(),
            Some("P0000E")
        );
        assert_eq!(
            format_variable_address(Some("N"), Some(14), Some("WORD"), None).as_deref(),
            Some("N00014")
        );
        assert_eq!(
            format_variable_address(Some("X"), Some(14), Some("BIT"), None).as_deref(),
            Some("X00014")
        );
        assert_eq!(
            format_variable_address(Some("Y"), Some(14), Some("BIT"), None).as_deref(),
            Some("Y00014")
        );
    }

    #[test]
    fn rejects_non_xgwx_data() {
        let error = XgwxDocument::parse(b"not an xgwx").expect_err("invalid magic");
        assert!(matches!(error, XgwxError::InvalidMagic));
    }

    fn synthetic_xgwx_bytes(xml: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"XG");
        bytes.extend_from_slice(UTF16_MARKER);
        let label = "XG5000 WORKSPACE FILE".encode_utf16().collect::<Vec<_>>();
        bytes.push(u8::try_from(label.len()).expect("label fits in u8"));
        for unit in label {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        bytes.extend_from_slice(&1u32.to_le_bytes());

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(xml.as_bytes()).expect("gzip write");
        bytes.extend_from_slice(&encoder.finish().expect("gzip finish"));
        bytes
    }

    fn synthetic_ladder_data() -> Vec<u8> {
        let mut data = Vec::new();
        append_ff43_record(&mut data, 0x32, (0x58, 0x00), (0x01, 0x00));
        append_ff43_record(&mut data, 0x32, (0x58, 0x04), (0x03, 0x00));
        append_device_cell(&mut data, 0xff06, (0x01, 0x00), "M00001");
        append_device_cell(&mut data, 0xff07, (0x04, 0x00), "M00002");
        append_device_cell(&mut data, 0xff0e, (0x5e, 0x00), "M00003");
        append_device_cell(&mut data, 0xff11, (0x5e, 0x04), "M00004");
        append_instruction_cell(&mut data, (0x58, 0x08), "MOV,D000001,D000002");
        data
    }

    fn append_ff43_record(data: &mut Vec<u8>, code: u8, target: (u8, u8), branch: (u8, u8)) {
        let start = data.len();
        data.resize(start + 48, 0);
        data[start] = 0xff;
        data[start + 1] = 0x43;
        data[start + 13] = code;
        data[start + 17] = target.0;
        data[start + 18] = target.1;
        data[start + 36] = branch.0;
        data[start + 37] = branch.1;
    }

    fn append_device_cell(data: &mut Vec<u8>, marker: u16, coord: (u8, u8), value: &str) {
        let [marker_hi, marker_lo] = marker.to_be_bytes();
        data.extend_from_slice(&[
            marker_hi, marker_lo, 0, 0, 0, coord.0, coord.1, 0, 0, 1, 0, 0, 0, 0, 0,
        ]);
        append_ladder_string(data, value);
    }

    fn append_instruction_cell(data: &mut Vec<u8>, coord: (u8, u8), value: &str) {
        data.extend_from_slice(&[coord.0, coord.1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        append_ladder_string(data, value);
    }

    fn append_ladder_string(data: &mut Vec<u8>, value: &str) {
        let units = value.encode_utf16().collect::<Vec<_>>();
        data.extend_from_slice(UTF16_MARKER);
        data.push(u8::try_from(units.len()).expect("string fits in u8"));
        for unit in units {
            data.extend_from_slice(&unit.to_le_bytes());
        }
    }
}
