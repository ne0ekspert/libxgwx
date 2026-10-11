#![deny(missing_docs)]
//! Document-bound rack/base views and their typed modules.
use crate::{BaseSummary, Module, XgwxDocument, XgwxError, XmlElement};

/// Borrowed collection of hardware bases, in XML document order.
/// Indices refer to records, so repeated base numbers in different PLCs remain distinct.
#[derive(Debug, Clone, Copy)]
pub struct BaseView<'a> {
    doc: &'a XgwxDocument,
}

/// A hardware rack/base belonging to a particular configuration and document.
///
/// This shared handle does not copy the document. Its modules retain the same
/// classification, network associations, and order as [`XgwxDocument::modules`].
/// XML ownership takes precedence over location attributes. Otherwise, a module
/// joins a base only when its configuration and base number resolve uniquely.
/// Ambiguous or unplaced modules remain available in the document-wide inventory.
///
/// ```no_run
/// let doc = xgwx::XgwxDocument::from_path("project.xgwx")?;
/// for base in doc.bases() {
///     println!("Base {:?}, PLC {:?}", base.number(), base.configuration_index());
///     for module in base.modules() { println!("{module:#?}"); }
/// }
/// let first = doc.base().get(0)?;
/// let modules = first.modules();
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Base<'a> {
    doc: &'a XgwxDocument,
    index: usize,
    configuration_index: Option<usize>,
    element: &'a XmlElement,
}

impl XgwxDocument {
    /// Borrow the base collection. See [`Base`] for per-base module access.
    pub fn base(&self) -> BaseView<'_> {
        BaseView { doc: self }
    }

    /// List document-bound base handles in document order.
    /// Use [`Self::base_summaries`] for detached native summaries.
    pub fn bases(&self) -> Vec<Base<'_>> {
        self.base().list()
    }
}

impl<'a> BaseView<'a> {
    /// List all bases as shared handles into this document.
    pub fn list(self) -> Vec<Base<'a>> {
        crate::modules::base_handles(self.doc)
    }

    /// Select a base by zero-based document index, not by its native base number.
    pub fn get(self, index: usize) -> Result<Base<'a>, XgwxError> {
        self.list()
            .into_iter()
            .nth(index)
            .ok_or(XgwxError::BaseIndexNotFound { index })
    }
}

impl<'a> Base<'a> {
    pub(crate) fn document(&self) -> &'a XgwxDocument {
        self.doc
    }

    pub(crate) fn new(
        doc: &'a XgwxDocument,
        index: usize,
        configuration_index: Option<usize>,
        element: &'a XmlElement,
    ) -> Self {
        Self {
            doc,
            index,
            configuration_index,
            element,
        }
    }
    /// Zero-based base record index in document order.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Owning PLC index in [`XgwxDocument::configurations`] order, if present.
    pub fn configuration_index(&self) -> Option<usize> {
        self.configuration_index
    }

    /// Stored native base number, or `None` if it is absent or invalid.
    pub fn number(&self) -> Option<u32> {
        crate::attr_u32(self.element, "Base")
    }

    /// Stored physical slot count, or `None` if it is absent or invalid.
    pub fn slot_count(&self) -> Option<u32> {
        crate::attr_u32(self.element, "SlotCount")
    }

    /// Read a detached native summary, including all raw XML attributes.
    pub fn summary(&self) -> BaseSummary {
        BaseSummary::from_element(self.element)
    }

    /// Enumerate typed modules belonging to this base, retaining unknown hardware.
    /// Network memberships without installed hardware are included only when their
    /// owning base resolves uniquely. This list is not a physical slot count.
    pub fn modules(&self) -> Vec<Module> {
        crate::modules::base_modules(self.doc, self.index)
    }
}
