use crate::*;

/// Borrowed inspection of a workspace's XML and original binary container.
///
/// Header, compressed member and trailer describe the parsed input. After an
/// edit, use `to_bytes()` and reparse to inspect the emitted container metadata.
#[derive(Debug, Clone, Copy)]
pub struct ContainerView<'a> {
    doc: &'a XgwxDocument,
}

impl XgwxDocument {
    /// Inspect XML and preserved container bytes without exposing mutable state.
    ///
    /// Document fields cannot be mutated directly:
    /// ```compile_fail
    /// # use xgwx::XgwxDocument;
    /// fn change(doc: &mut XgwxDocument) { doc.xml.clear(); }
    /// ```
    pub fn container(&self) -> ContainerView<'_> {
        ContainerView { doc: self }
    }

    /// Explicit low-level XML editing, keeping the parsed tree synchronized.
    #[cfg(feature = "write")]
    pub fn container_mut(&mut self) -> ContainerEditor<'_> {
        ContainerEditor { doc: self }
    }
}

#[cfg(all(test, feature = "write"))]
mod tests {
    use super::*;

    #[test]
    fn inconsistent_internal_state_is_rejected_by_domain_editors() {
        let mut doc = create_project("XGK-CPUSN", "LD").unwrap();
        doc.xml = doc.xml.replacen("NewProject", "Changed", 1);
        let before = doc.clone();
        assert!(doc.configuration_mut().get(0).is_err());
        assert!(doc.project_mut().rename("Factory").is_err());
        assert!(doc.variable_mut().get(0).is_err());
        assert!(doc.variable_mut().named("anything").is_err());
        assert_eq!(doc, before);
    }

    #[test]
    fn inconsistent_internal_hardware_and_network_state_is_rejected() {
        let mut doc =
            XgwxDocument::parse(include_bytes!("../fixtures/networks/cnetbase-native.xgwx"))
                .unwrap();
        doc.xml = doc.xml.replacen("<Project", "<OtherProject", 1);
        let before = doc.clone();
        assert!(
            doc.module_mut()
                .get(0)
                .unwrap()
                .update(&ModulePatch {
                    comment: Some("Changed".into()),
                    ..Default::default()
                })
                .is_err()
        );
        assert_eq!(doc, before);

        let mut doc =
            XgwxDocument::parse(include_bytes!("../fixtures/networks/cnetbase-native.xgwx"))
                .unwrap();
        doc.xml = doc.xml.replacen("<Project", "<OtherProject", 1);
        let before = doc.clone();
        assert!(doc.network_mut().get(0).is_err());
        assert_eq!(doc, before);
    }
}

impl<'a> ContainerView<'a> {
    /// Original binary header metadata.
    pub fn header(self) -> &'a XgwxHeader {
        &self.doc.header
    }
    /// Current inflated XML, including edits.
    pub fn xml(self) -> &'a str {
        &self.doc.xml
    }
    /// Parsed tree corresponding to the current XML.
    pub fn root(self) -> &'a XmlElement {
        &self.doc.root
    }
    /// Original compressed main XML member, retained for unchanged emission.
    pub fn main_gzip(self) -> &'a [u8] {
        &self.doc.main_gzip
    }
    /// Original bytes after the main gzip member, including alignment padding.
    pub fn trailer(self) -> &'a [u8] {
        &self.doc.trailer
    }
    /// Valid gzip members discovered in the original trailer.
    pub fn trailer_gzip_members(self) -> &'a [GzipMember] {
        &self.doc.trailer_gzip_members
    }
    /// XML property records, including unknown property types.
    pub fn properties(self) -> Vec<PropertySummary> {
        self.doc.properties()
    }
    /// Decode binary XML payloads, retaining errors for individual records.
    pub fn decoded_payloads(self) -> Vec<Result<DecodedPayloadSummary, XgwxError>> {
        self.doc.decoded_payloads()
    }
}

/// Low-level replacement of project XML. Prefer domain editors for supported
/// PLC operations; this editor validates XML syntax, not native PLC semantics.
#[cfg(feature = "write")]
pub struct ContainerEditor<'a> {
    doc: &'a mut XgwxDocument,
}

#[cfg(feature = "write")]
impl ContainerEditor<'_> {
    /// Inspect the current XML and preserved container data.
    pub fn view(&self) -> ContainerView<'_> {
        self.doc.container()
    }

    /// Replace XML atomically after parsing it successfully.
    ///
    /// Unknown sections are retained exactly as supplied. Header, compressed
    /// input member and trailer remain preserved until serialization. Success
    /// does not imply the XML is accepted by XG5000 or by domain writers.
    pub fn replace_xml(&mut self, xml: impl Into<String>) -> Result<(), XgwxError> {
        let xml = xml.into();
        let root = crate::parse_xml(&xml)?;
        self.doc.xml = xml;
        self.doc.root = root;
        Ok(())
    }
}
