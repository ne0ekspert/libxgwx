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
        /// Multiple configurations, unsupported migrations and hardware limits are rejected
        /// without changing the document. Selecting the current model is a no-op.
        pub fn select_cpu(&mut self, model: &str) -> Result<(), XgwxError> {
            self.doc.ensure_consistent_api_tree()?;
            // The backend is deliberately single-configuration. An index must not
            // make it appear to support converting a different PLC in a workspace.
            if self.index != 0 || self.doc.configurations().len() != 1 {
                return Err(XgwxError::AmbiguousConfiguration);
            }
            self.doc.select_cpu(model)
        }
    }
}
#[cfg(feature = "write")]
pub use editors::{ConfigurationCollectionEditor, ConfigurationEditor};
