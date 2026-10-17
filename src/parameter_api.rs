#![deny(missing_docs)]
//! Parameter records with explicit PLC ownership and typed decoding.
use crate::{
    ConfigurationRef, HscParameterSummary, ParameterSummary, PidCalParameterSummary,
    PidTuneParameterSummary, PositionParameterSummary, XgwxDocument, XgwxError, XmlElement,
};

/// Parameter sections in document order, optionally scoped to a selected PLC.
#[derive(Debug, Clone, Copy)]
pub struct ParameterView<'a> {
    doc: &'a XgwxDocument,
    configuration_index: Option<usize>,
}
/// A parameter section borrowed from its document. Native record indices remain stable
/// when the collection is filtered by configuration or parameter kind.
#[derive(Debug, Clone, Copy)]
pub struct ParameterRef<'a> {
    doc: &'a XgwxDocument,
    element: &'a XmlElement,
    index: usize,
    configuration_index: Option<usize>,
}
/// Recognized parameter families. Unknown type strings are retained in summaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterKind {
    /// Captured high-speed counter payload.
    Hsc,
    /// Captured positioning axis records.
    Position,
    /// Embedded PID calculation records.
    PidCalculation,
    /// Embedded PID tuning records.
    PidTuning,
    /// General, missing, or unrecognized native parameter type.
    Other,
}
/// Detached decoded data for one selected parameter section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterData {
    /// High-speed counter fields, with raw payload bytes preserved.
    Hsc(Box<HscParameterSummary>),
    /// Positioning axes and captured axis parameters.
    Position(Box<PositionParameterSummary>),
    /// PID calculation settings and loops.
    PidCalculation(Box<PidCalParameterSummary>),
    /// PID tuning settings and loops.
    PidTuning(Box<PidTuneParameterSummary>),
    /// Native attributes and child section metadata for other parameter types.
    Other(ParameterSummary),
}
/// PID readers sharing the scope of their parent parameter collection.
#[derive(Debug, Clone, Copy)]
pub struct PidView<'a> {
    parameters: ParameterView<'a>,
}
impl XgwxDocument {
    /// Borrow all parameter sections, including records outside configurations.
    /// Native HSC, PID and positioning writes are not yet supported.
    pub fn parameter(&self) -> ParameterView<'_> {
        ParameterView {
            doc: self,
            configuration_index: None,
        }
    }
}
impl<'a> ConfigurationRef<'a> {
    /// Parameter sections owned by this PLC, preserving document-wide indices.
    pub fn parameters(self) -> ParameterView<'a> {
        ParameterView {
            doc: self.document(),
            configuration_index: Some(self.index()),
        }
    }
}
impl<'a> ParameterView<'a> {
    /// Borrow every section within the current scope in native document order.
    pub fn list(self) -> Vec<ParameterRef<'a>> {
        fn walk<'a>(
            doc: &'a XgwxDocument,
            element: &'a XmlElement,
            owner: Option<usize>,
            next_configuration: &mut usize,
            out: &mut Vec<ParameterRef<'a>>,
        ) {
            let owner = if element.name == "Configuration" {
                let index = *next_configuration;
                *next_configuration += 1;
                Some(index)
            } else {
                owner
            };
            if element.name == "Parameter" {
                out.push(ParameterRef {
                    doc,
                    element,
                    index: out.len(),
                    configuration_index: owner,
                });
            }
            for child in &element.children {
                walk(doc, child, owner, next_configuration, out);
            }
        }
        let mut out = Vec::new();
        walk(self.doc, &self.doc.root, None, &mut 0, &mut out);
        if let Some(index) = self.configuration_index {
            out.retain(|record| record.configuration_index == Some(index));
        }
        out
    }
    /// Select by document-wide parameter index within this collection's scope.
    /// A record owned by another PLC is treated as absent.
    pub fn get(self, index: usize) -> Result<ParameterRef<'a>, XgwxError> {
        self.list()
            .into_iter()
            .find(|record| record.index == index)
            .ok_or(XgwxError::ParameterIndexNotFound { index })
    }
    /// Native metadata for all sections within the current scope.
    pub fn summaries(self) -> Vec<ParameterSummary> {
        self.list()
            .into_iter()
            .map(|record| record.summary())
            .collect()
    }
    /// Borrow all sections of a recognized family, preserving document-wide indices.
    pub fn of_kind(self, kind: ParameterKind) -> Vec<ParameterRef<'a>> {
        self.list()
            .into_iter()
            .filter(|record| record.kind() == kind)
            .collect()
    }
    /// Decode HSC sections independently. One invalid payload does not hide other records.
    pub fn hsc(self) -> Vec<Result<HscParameterSummary, XgwxError>> {
        self.of_kind(ParameterKind::Hsc)
            .into_iter()
            .map(|record| HscParameterSummary::from_element(record.element))
            .collect()
    }
    /// Decode captured positioning axes, retaining reported extra axes and raw attributes.
    pub fn position(self) -> Vec<PositionParameterSummary> {
        self.of_kind(ParameterKind::Position)
            .into_iter()
            .map(|record| PositionParameterSummary::from_element(record.element))
            .collect()
    }
    /// Borrow PID calculation and tuning readers within this same scope.
    pub fn pid(self) -> PidView<'a> {
        PidView { parameters: self }
    }
}
impl<'a> ParameterRef<'a> {
    /// Native document-wide parameter record index.
    pub fn index(self) -> usize {
        self.index
    }
    /// Owning PLC record index, or `None` for an unowned section.
    pub fn configuration_index(self) -> Option<usize> {
        self.configuration_index
    }
    /// Owning configuration handle, if present.
    pub fn configuration(self) -> Option<ConfigurationRef<'a>> {
        self.configuration_index
            .and_then(|index| self.doc.configuration().get(index).ok())
    }
    /// Preserved native type string, including unknown families.
    pub fn type_name(self) -> Option<&'a str> {
        self.element.attribute("Type")
    }
    /// Recognized parameter family without decoding its payload.
    pub fn kind(self) -> ParameterKind {
        match self.type_name() {
            Some("HSC PARAMETER") => ParameterKind::Hsc,
            Some("POSITION PARAMETER") => ParameterKind::Position,
            Some("PID CAL PARAMETER") => ParameterKind::PidCalculation,
            Some("PID TUNE PARAMETER") => ParameterKind::PidTuning,
            _ => ParameterKind::Other,
        }
    }
    /// Native section metadata, including unknown XML attributes.
    pub fn summary(self) -> ParameterSummary {
        ParameterSummary::from_element(self.element)
    }
    /// Decode this selected section using the existing captured-layout parser.
    /// Malformed HSC hex data returns an error. Other records retain absent fields
    /// and unknown attributes rather than inventing defaults.
    pub fn decode(self) -> Result<ParameterData, XgwxError> {
        Ok(match self.kind() {
            ParameterKind::Hsc => {
                ParameterData::Hsc(Box::new(HscParameterSummary::from_element(self.element)?))
            }
            ParameterKind::Position => ParameterData::Position(Box::new(
                PositionParameterSummary::from_element(self.element),
            )),
            ParameterKind::PidCalculation => ParameterData::PidCalculation(Box::new(
                PidCalParameterSummary::from_element(self.element),
            )),
            ParameterKind::PidTuning => ParameterData::PidTuning(Box::new(
                PidTuneParameterSummary::from_element(self.element),
            )),
            ParameterKind::Other => ParameterData::Other(self.summary()),
        })
    }
}
impl PidView<'_> {
    /// Decode PID calculation sections in the parent collection's scope and order.
    pub fn calculation(self) -> Vec<PidCalParameterSummary> {
        self.parameters
            .of_kind(ParameterKind::PidCalculation)
            .into_iter()
            .map(|record| PidCalParameterSummary::from_element(record.element))
            .collect()
    }
    /// Decode PID tuning sections in the parent collection's scope and order.
    /// PWM points remain raw native P-area offsets.
    pub fn tuning(self) -> Vec<PidTuneParameterSummary> {
        self.parameters
            .of_kind(ParameterKind::PidTuning)
            .into_iter()
            .map(|record| PidTuneParameterSummary::from_element(record.element))
            .collect()
    }
}

#[cfg(test)]
mod legacy_decode_parity {
    use crate::*;

    #[test]
    fn local_native_xgb_fixtures_retain_existing_decodes_and_container() {
        let mut checked = 0;
        for file in ["XGB_Enet01.xgwx", "XGB_Enet02.xgwx"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures")
                .join(file);
            // Native fixtures can be omitted from a source-only distribution.
            if !path.exists() {
                continue;
            }
            let bytes = std::fs::read(path).unwrap();
            let doc = XgwxDocument::parse(&bytes).unwrap();
            let before = doc.clone();
            assert_eq!(doc.parameter().summaries(), doc.parameters());
            assert_eq!(doc.parameter().position(), doc.position_parameters());
            assert_eq!(
                doc.parameter().pid().calculation(),
                doc.pid_cal_parameters()
            );
            assert_eq!(doc.parameter().pid().tuning(), doc.pid_tune_parameters());
            let old: Vec<_> = doc
                .hsc_parameters()
                .into_iter()
                .map(Result::unwrap)
                .collect();
            let new: Vec<_> = doc
                .parameter()
                .hsc()
                .into_iter()
                .map(Result::unwrap)
                .collect();
            assert_eq!(old, new);
            assert!(!new.is_empty());
            for record in doc.parameter().list() {
                record.decode().unwrap();
            }
            assert_eq!(doc, before);
            #[cfg(feature = "write")]
            assert_eq!(doc.to_verified_bytes().unwrap(), bytes);
            checked += 1;
        }
        eprintln!("validated {checked} local native XGB fixtures");
    }
}
