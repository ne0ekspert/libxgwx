#![deny(missing_docs)]
//! Selected configuration records and guarded CPU selection.
use crate::{ConfigurationSummary, CpuCatalogEntry, CpuHardwareProfile, XgwxDocument, XgwxError};

/// Borrowed configuration collection, ordered as stored in the document.
#[derive(Debug, Clone, Copy)]
pub struct ConfigurationView<'a> {
    doc: &'a XgwxDocument,
}
/// A configuration selected by document index, independent of its name.
#[derive(Debug, Clone, Copy)]
pub struct ConfigurationRef<'a> {
    doc: &'a XgwxDocument,
    index: usize,
}
impl XgwxDocument {
    /// Resolve record ownership in one tree traversal, retaining orphan records.
    pub(crate) fn configuration_record_owners(&self, name: &str) -> Vec<Option<usize>> {
        fn visit(
            node: &crate::XmlElement,
            name: &str,
            owner: Option<usize>,
            next: &mut usize,
            out: &mut Vec<Option<usize>>,
        ) {
            let owner = if node.name == "Configuration" {
                let index = *next;
                *next += 1;
                Some(index)
            } else {
                owner
            };
            if node.name == name {
                out.push(owner);
            }
            for child in &node.children {
                visit(child, name, owner, next, out);
            }
        }
        let mut out = Vec::new();
        visit(&self.root, name, None, &mut 0, &mut out);
        out
    }
    /// Borrow the configuration namespace for metadata and CPU inspection.
    pub fn configuration(&self) -> ConfigurationView<'_> {
        ConfigurationView { doc: self }
    }
}
impl<'a> ConfigurationView<'a> {
    /// All configuration metadata in document order.
    pub fn list(self) -> Vec<ConfigurationSummary> {
        self.doc.configurations()
    }
    /// Select a configuration by document order. Names may repeat.
    pub fn get(self, index: usize) -> Result<ConfigurationRef<'a>, XgwxError> {
        self.doc
            .configurations()
            .get(index)
            .ok_or(XgwxError::ConfigurationIndexNotFound { index })?;
        Ok(ConfigurationRef {
            doc: self.doc,
            index,
        })
    }
}
impl<'a> ConfigurationRef<'a> {
    pub(crate) fn document(self) -> &'a XgwxDocument {
        self.doc
    }
    /// Document-wide configuration index.
    pub fn index(self) -> usize {
        self.index
    }
    /// Current native configuration metadata.
    pub fn summary(self) -> ConfigurationSummary {
        self.doc.configurations()[self.index].clone()
    }
    /// Cataloged CPU model, when the stored type code is recognized.
    /// Unknown and absent type codes remain available in [`Self::summary`].
    pub fn cpu(self) -> Option<&'static CpuCatalogEntry> {
        self.summary().type_code.and_then(crate::cpu::cpu_for_type)
    }
    /// Captured compact hardware variant, when uniquely recognized.
    /// Current variant recognition requires a single-configuration document.
    pub fn hardware_profile(self) -> Option<&'static CpuHardwareProfile> {
        self.doc.cpu_hardware_profile()
    }
    /// Programs owned by this PLC, preserving document indices and native order.
    /// Repeated names in sibling PLCs do not affect membership.
    pub fn programs(self) -> Vec<crate::ProgramRef<'a>> {
        self.doc
            .configuration_record_owners("Program")
            .into_iter()
            .enumerate()
            .filter(|(_, owner)| *owner == Some(self.index))
            .map(|(index, _)| crate::ProgramRef::from_index(self.doc, index))
            .collect()
    }
    /// Tasks owned by this PLC, preserving document indices and native order.
    pub fn tasks(self) -> Vec<crate::TaskRef<'a>> {
        self.doc
            .configuration_record_owners("Task")
            .into_iter()
            .enumerate()
            .filter(|(_, owner)| *owner == Some(self.index))
            .map(|(index, _)| crate::TaskRef::from_index(self.doc, index))
            .collect()
    }
    /// Bases owned by this configuration, preserving their document indices.
    pub fn bases(self) -> Vec<crate::Base<'a>> {
        self.doc
            .bases()
            .into_iter()
            .filter(|base| base.configuration_index() == Some(self.index))
            .collect()
    }
}
#[cfg(feature = "write")]
mod editors {
    use super::*;
    /// Exclusive selection of configuration records for supported edits.
    pub struct ConfigurationCollectionEditor<'a> {
        doc: &'a mut XgwxDocument,
    }
    /// Selected configuration editor. CPU changes retain existing migration guards.
    ///
    /// ```no_run
    /// # fn change(doc: &mut xgwx::XgwxDocument) -> Result<(), xgwx::XgwxError> {
    /// doc.configuration_mut().get(0)?.select_cpu("XGK-CPUUN")?;
    /// # Ok(())
    /// # }
    /// ```
    pub struct ConfigurationEditor<'a> {
        doc: &'a mut XgwxDocument,
        index: usize,
    }
    impl XgwxDocument {
        /// Exclusively borrow the configuration namespace.
        pub fn configuration_mut(&mut self) -> ConfigurationCollectionEditor<'_> {
            ConfigurationCollectionEditor { doc: self }
        }
    }
    impl<'a> ConfigurationCollectionEditor<'a> {
        /// Select by document index. A read-valid selection can still reject unsupported edits.
        pub fn get(self, index: usize) -> Result<ConfigurationEditor<'a>, XgwxError> {
            self.doc.ensure_consistent_api_tree()?;
            self.doc.configuration().get(index)?;
            Ok(ConfigurationEditor {
                doc: self.doc,
                index,
            })
        }
    }
    impl ConfigurationEditor<'_> {
        /// Inspect the selected configuration and current CPU.
        pub fn view(&self) -> ConfigurationRef<'_> {
            ConfigurationRef {
                doc: self.doc,
                index: self.index,
            }
        }
        /// Select a supported CPU model while preserving validated program and hardware data.
        /// XGK changes validate only this PLC's hardware and preserve sibling PLCs.
        /// XGI migrations currently require a single PLC. Unsupported migrations and
        /// hardware limits leave the document unchanged. The current model is a no-op.
        pub fn select_cpu(&mut self, model: &str) -> Result<(), XgwxError> {
            self.doc.ensure_consistent_api_tree()?;
            self.doc.select_configuration_cpu(self.index, model)
        }
    }
}
#[cfg(feature = "write")]
pub use editors::{ConfigurationCollectionEditor, ConfigurationEditor};
