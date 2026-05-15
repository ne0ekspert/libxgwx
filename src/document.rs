use crate::*;

#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

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
    #[cfg(not(target_arch = "wasm32"))]
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
