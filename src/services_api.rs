#![deny(missing_docs)]
//! Borrowed high-speed link, safety communication, and trend monitoring records.
//! These readers preserve native data; no writers for these families are validated.
use crate::*;
#[derive(Debug, Clone, Copy)]
struct Record<'a> {
    doc: &'a XgwxDocument,
    element: &'a XmlElement,
    index: usize,
    configuration_index: Option<usize>,
    link_index: Option<usize>,
}
fn records<'a>(doc: &'a XgwxDocument, tag: &str, scope: Option<usize>) -> Vec<Record<'a>> {
    fn walk<'a>(
        doc: &'a XgwxDocument,
        node: &'a XmlElement,
        tag: &str,
        owner: Option<usize>,
        link: Option<usize>,
        counters: &mut (usize, usize),
        out: &mut Vec<Record<'a>>,
    ) {
        let (owner, link) = if node.name == "Configuration" {
            let index = counters.0;
            counters.0 += 1;
            (Some(index), None)
        } else {
            (owner, link)
        };
        let link = if node.name == "XGPD_HS_LINK" {
            let index = counters.1;
            counters.1 += 1;
            Some(index)
        } else {
            link
        };
        if node.name == tag {
            out.push(Record {
                doc,
                element: node,
                index: out.len(),
                configuration_index: owner,
                link_index: link,
            });
        }
        for child in &node.children {
            walk(doc, child, tag, owner, link, counters, out);
        }
    }
    let mut out = Vec::new();
    walk(doc, &doc.root, tag, None, None, &mut (0, 0), &mut out);
    if let Some(scope) = scope {
        out.retain(|record| record.configuration_index == Some(scope));
    }
    out
}

/// Read-only high-speed link collection. Global indices are retained in PLC-scoped views.
#[derive(Debug, Clone, Copy)]
pub struct HighSpeedLinkView<'a> {
    doc: &'a XgwxDocument,
    scope: Option<usize>,
}
/// Selected high-speed link record borrowed from its document.
#[derive(Debug, Clone, Copy)]
pub struct HighSpeedLinkRef<'a> {
    record: Record<'a>,
}
impl XgwxDocument {
    /// Borrow all high-speed link records, including records outside a PLC configuration.
    pub fn high_speed_link(&self) -> HighSpeedLinkView<'_> {
        HighSpeedLinkView {
            doc: self,
            scope: None,
        }
    }
}
impl<'a> ConfigurationRef<'a> {
    /// Borrow high-speed link records owned by this PLC.
    pub fn high_speed_link(self) -> HighSpeedLinkView<'a> {
        HighSpeedLinkView {
            doc: self.document(),
            scope: Some(self.index()),
        }
    }
}
impl<'a> HighSpeedLinkView<'a> {
    /// List every record in this scope in native document order.
    pub fn list(self) -> Vec<HighSpeedLinkRef<'a>> {
        records(self.doc, "XGPD_HS_LINK", self.scope)
            .into_iter()
            .map(|record| HighSpeedLinkRef { record })
            .collect()
    }
    /// Select by document-wide record index. Records outside this scope are absent.
    pub fn get(self, index: usize) -> Result<HighSpeedLinkRef<'a>, XgwxError> {
        self.list()
            .into_iter()
            .find(|record| record.index() == index)
            .ok_or(XgwxError::RecordIndexNotFound {
                kind: "high-speed link",
                index,
            })
    }
    /// First record in this scope, preserving the legacy reader's optional-first semantics.
    pub fn first(self) -> Option<HighSpeedLinkRef<'a>> {
        self.list().into_iter().next()
    }
    /// Detached native summaries for all records in this scope.
    pub fn summaries(self) -> Vec<HighSpeedLinkSummary> {
        self.list()
            .into_iter()
            .map(|record| record.summary())
            .collect()
    }
}
impl<'a> HighSpeedLinkRef<'a> {
    /// Document-wide record index, distinct from native selector values.
    pub fn index(self) -> usize {
        self.record.index
    }
    /// Owning PLC record index, or `None` for a record outside configurations.
    pub fn configuration_index(self) -> Option<usize> {
        self.record.configuration_index
    }
    /// Owning configuration, when present.
    pub fn configuration(self) -> Option<ConfigurationRef<'a>> {
        self.configuration_index()
            .and_then(|index| self.record.doc.configuration().get(index).ok())
    }
    /// Existing native decoding, with unknown attributes retained.
    pub fn summary(self) -> HighSpeedLinkSummary {
        HighSpeedLinkSummary::from_element(self.record.element)
    }
}

/// Read-only safety communication collection. Global indices are retained in PLC-scoped views.
#[derive(Debug, Clone, Copy)]
pub struct SafetyView<'a> {
    doc: &'a XgwxDocument,
    scope: Option<usize>,
}
/// Selected safety communication record borrowed from its document.
#[derive(Debug, Clone, Copy)]
pub struct SafetyRef<'a> {
    record: Record<'a>,
}
impl XgwxDocument {
    /// Borrow all safety communication records, including records outside a PLC configuration.
    pub fn safety(&self) -> SafetyView<'_> {
        SafetyView {
            doc: self,
            scope: None,
        }
    }
}
impl<'a> ConfigurationRef<'a> {
    /// Borrow safety communication records owned by this PLC.
    pub fn safety(self) -> SafetyView<'a> {
        SafetyView {
            doc: self.document(),
            scope: Some(self.index()),
        }
    }
}
impl<'a> SafetyView<'a> {
    /// List every record in this scope in native document order.
    pub fn list(self) -> Vec<SafetyRef<'a>> {
        records(self.doc, "Safety_Comm", self.scope)
            .into_iter()
            .map(|record| SafetyRef { record })
            .collect()
    }
    /// Select by document-wide record index. Records outside this scope are absent.
    pub fn get(self, index: usize) -> Result<SafetyRef<'a>, XgwxError> {
        self.list()
            .into_iter()
            .find(|record| record.index() == index)
            .ok_or(XgwxError::RecordIndexNotFound {
                kind: "safety communication",
                index,
            })
    }
    /// First record in this scope, preserving the legacy reader's optional-first semantics.
    pub fn first(self) -> Option<SafetyRef<'a>> {
        self.list().into_iter().next()
    }
    /// Detached native summaries for all records in this scope.
    pub fn summaries(self) -> Vec<SafetyCommSummary> {
        self.list()
            .into_iter()
            .map(|record| record.summary())
            .collect()
    }
}
impl<'a> SafetyRef<'a> {
    /// Document-wide record index, distinct from native selector values.
    pub fn index(self) -> usize {
        self.record.index
    }
    /// Owning PLC record index, or `None` for a record outside configurations.
    pub fn configuration_index(self) -> Option<usize> {
        self.record.configuration_index
    }
    /// Owning configuration, when present.
    pub fn configuration(self) -> Option<ConfigurationRef<'a>> {
        self.configuration_index()
            .and_then(|index| self.record.doc.configuration().get(index).ok())
    }
    /// Existing native decoding, with unknown attributes retained.
    pub fn summary(self) -> SafetyCommSummary {
        SafetyCommSummary::from_element(self.record.element)
    }
}

/// Read-only trend monitoring collection. Global indices are retained in PLC-scoped views.
#[derive(Debug, Clone, Copy)]
pub struct TrendView<'a> {
    doc: &'a XgwxDocument,
    scope: Option<usize>,
}
/// Selected trend monitoring record borrowed from its document.
#[derive(Debug, Clone, Copy)]
pub struct TrendRef<'a> {
    record: Record<'a>,
}
impl XgwxDocument {
    /// Borrow all trend monitoring records, including records outside a PLC configuration.
    pub fn trend(&self) -> TrendView<'_> {
        TrendView {
            doc: self,
            scope: None,
        }
    }
}
impl<'a> ConfigurationRef<'a> {
    /// Borrow trend monitoring records owned by this PLC.
    pub fn trend(self) -> TrendView<'a> {
        TrendView {
            doc: self.document(),
            scope: Some(self.index()),
        }
    }
}
impl<'a> TrendView<'a> {
    /// List every record in this scope in native document order.
    pub fn list(self) -> Vec<TrendRef<'a>> {
        records(self.doc, "TrendMonitoring", self.scope)
            .into_iter()
            .map(|record| TrendRef { record })
            .collect()
    }
    /// Select by document-wide record index. Records outside this scope are absent.
    pub fn get(self, index: usize) -> Result<TrendRef<'a>, XgwxError> {
        self.list()
            .into_iter()
            .find(|record| record.index() == index)
            .ok_or(XgwxError::RecordIndexNotFound {
                kind: "trend monitoring",
                index,
            })
    }
    /// First record in this scope, preserving the legacy reader's optional-first semantics.
    pub fn first(self) -> Option<TrendRef<'a>> {
        self.list().into_iter().next()
    }
    /// Detached native summaries for all records in this scope.
    pub fn summaries(self) -> Vec<TrendMonitoringSummary> {
        self.list()
            .into_iter()
            .map(|record| record.summary())
            .collect()
    }
}
impl<'a> TrendRef<'a> {
    /// Document-wide record index, distinct from native selector values.
    pub fn index(self) -> usize {
        self.record.index
    }
    /// Owning PLC record index, or `None` for a record outside configurations.
    pub fn configuration_index(self) -> Option<usize> {
        self.record.configuration_index
    }
    /// Owning configuration, when present.
    pub fn configuration(self) -> Option<ConfigurationRef<'a>> {
        self.configuration_index()
            .and_then(|index| self.record.doc.configuration().get(index).ok())
    }
    /// Existing native decoding, with unknown attributes retained.
    pub fn summary(self) -> TrendMonitoringSummary {
        TrendMonitoringSummary::from_element(self.record.element)
    }
}

/// Selected high-speed block with document index and explicit link ownership.
#[derive(Debug, Clone, Copy)]
pub struct HighSpeedLinkBlockRef<'a> {
    record: Record<'a>,
}
impl<'a> HighSpeedLinkView<'a> {
    /// Every block in this scope, including blocks without a link parent.
    pub fn blocks(self) -> Vec<HighSpeedLinkBlockRef<'a>> {
        records(self.doc, "XGPD_HS_LINK_BLK", self.scope)
            .into_iter()
            .map(|record| HighSpeedLinkBlockRef { record })
            .collect()
    }
    /// Select a block by its document-wide block index, independently of link indices.
    pub fn block(self, index: usize) -> Result<HighSpeedLinkBlockRef<'a>, XgwxError> {
        self.blocks()
            .into_iter()
            .find(|block| block.index() == index)
            .ok_or(XgwxError::RecordIndexNotFound {
                kind: "high-speed link block",
                index,
            })
    }
}
impl<'a> HighSpeedLinkRef<'a> {
    /// Blocks whose nearest link ancestor is this record. Nested links and other
    /// configurations keep separate ownership; native block selector values may repeat.
    pub fn blocks(self) -> Vec<HighSpeedLinkBlockRef<'a>> {
        self.record
            .doc
            .high_speed_link()
            .blocks()
            .into_iter()
            .filter(|block| block.record.link_index == Some(self.index()))
            .collect()
    }
}
impl<'a> HighSpeedLinkBlockRef<'a> {
    /// Document-wide block index, distinct from the native `BlockIndex` selector.
    pub fn index(self) -> usize {
        self.record.index
    }
    /// Owning PLC record index, if present.
    pub fn configuration_index(self) -> Option<usize> {
        self.record.configuration_index
    }
    /// Owning configuration, when present.
    pub fn configuration(self) -> Option<ConfigurationRef<'a>> {
        self.configuration_index()
            .and_then(|index| self.record.doc.configuration().get(index).ok())
    }
    /// Nearest owning high-speed link, or `None` for unowned blocks.
    pub fn link(self) -> Option<HighSpeedLinkRef<'a>> {
        self.record
            .link_index
            .and_then(|index| self.record.doc.high_speed_link().get(index).ok())
    }
    /// Preserved native block attributes and decoded addresses.
    pub fn summary(self) -> HighSpeedLinkBlockSummary {
        HighSpeedLinkBlockSummary::from_element(self.record.element)
    }
}
impl SafetyRef<'_> {
    /// Captured safety channel records in stored child order.
    pub fn channels(self) -> Vec<SafetyCommChannelSummary> {
        self.summary().channels
    }
}
impl TrendRef<'_> {
    /// All direct child sections, including unknown native sections and repeated ones.
    /// Recognized trace, graph and window metadata are also available in `summary()`.
    pub fn sections(self) -> Vec<XmlSectionSummary> {
        self.record
            .element
            .children
            .iter()
            .map(XmlSectionSummary::from_element)
            .collect()
    }
}

#[cfg(test)]
mod legacy_decode_parity {
    use crate::*;
    use flate2::{Compression, write::GzEncoder};
    use std::io::Write;
    fn document(xml: &str) -> XgwxDocument {
        let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
        gzip.write_all(xml.as_bytes()).unwrap();
        let mut bytes = b"XG".to_vec();
        bytes.extend(gzip.finish().unwrap());
        XgwxDocument::parse(&bytes).unwrap()
    }
    #[test]
    fn native_safety_and_local_xgb_records_keep_existing_decodes() {
        let doc = document(include_str!("../src/cpu_defaults/xgi-cpuun.xml"));
        assert_eq!(doc.safety().first().map(|r| r.summary()), doc.safety_comm());
        assert!(doc.safety().first().is_some());
        for file in ["XGB_Enet01.xgwx", "XGB_Enet02.xgwx"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures")
                .join(file);
            if !path.exists() {
                continue;
            }
            let bytes = std::fs::read(path).unwrap();
            let doc = XgwxDocument::parse(&bytes).unwrap();
            let before = doc.clone();
            assert_eq!(doc.high_speed_link().summaries(), doc.high_speed_links());
            assert_eq!(
                doc.high_speed_link()
                    .blocks()
                    .iter()
                    .map(|r| r.summary())
                    .collect::<Vec<_>>(),
                doc.high_speed_link_blocks()
            );
            assert_eq!(doc.safety().first().map(|r| r.summary()), doc.safety_comm());
            assert_eq!(
                doc.trend().first().map(|r| r.summary()),
                doc.trend_monitoring()
            );
            assert_eq!(doc, before);
            #[cfg(feature = "write")]
            assert_eq!(doc.to_verified_bytes().unwrap(), bytes);
        }
    }
}
