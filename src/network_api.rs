#![deny(missing_docs)]
//! Borrowed access to document-wide network and protocol configuration records.
//!
//! Regression checks for the removed document getters:
//! ```compile_fail
//! fn old_api(doc: &xgwx::XgwxDocument) { let _ = doc.cnet_config_infos(); }
//! ```
//! ```compile_fail
//! fn old_api(doc: &xgwx::XgwxDocument) { let _ = doc.fenet_config_infos(); }
//! ```
use crate::{
    CnetConfigInfoSummary, FenetConfigInfoSummary, NetworkModule, NetworkSummary, XgwxDocument,
};

/// Network memberships and protocol settings in their original document order.
/// Configuration lists also retain settings without a matching network membership.
///
/// ```no_run
/// let doc = xgwx::XgwxDocument::from_path("project.xgwx")?;
/// let cnet = doc.network().cnet_config_infos();
/// let fenet = doc.network().fenet_config_infos();
/// for module in doc.network().modules() {
///     if let Some(config) = module.fenet_config_info() {
///         println!("{:?}", config.ip_address);
///     }
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
#[derive(Debug, Clone, Copy)]
pub struct NetworkView<'a> {
    pub(crate) doc: &'a XgwxDocument,
}

impl XgwxDocument {
    /// Borrow the network namespace. See [`NetworkView`] for configuration getters
    /// and examples.
    pub fn network(&self) -> NetworkView<'_> {
        NetworkView { doc: self }
    }
}

impl NetworkView<'_> {
    /// Detached native network membership summaries in document order.
    /// These describe network records rather than installed hardware slots.
    pub fn module_summaries(&self) -> Vec<crate::NetworkModuleSummary> {
        self.doc.network_modules()
    }
    /// Generic Dnet, FDEnet and Rnet configuration metadata. Records retain
    /// unknown attributes and the original tag-family ordering. Use
    /// [`Self::cnet_config_infos`] and [`Self::fenet_config_infos`] for the
    /// separately decoded Cnet and FEnet protocol settings.
    pub fn config_infos(&self) -> Vec<crate::XgpdConfigInfoSummary> {
        self.doc.xgpd_config_infos()
    }
    /// Raw network summaries, including their membership records.
    pub fn list(&self) -> Vec<NetworkSummary> {
        self.doc.networks()
    }

    /// Network memberships with configuration-scoped settings associations.
    pub fn modules(&self) -> Vec<NetworkModule> {
        self.doc.resolved_network_modules()
    }

    /// All Cnet configuration records, including serial port settings.
    /// Records retain their original document order for adapter indices.
    pub fn cnet_config_infos(&self) -> Vec<CnetConfigInfoSummary> {
        self.doc.cnet_config_infos()
    }

    /// All FEnet configuration records, including module IPv4 settings.
    /// Records retain their original document order for adapter indices.
    pub fn fenet_config_infos(&self) -> Vec<FenetConfigInfoSummary> {
        self.doc.fenet_config_infos()
    }
}

/// Borrowed network selected by its document record index.
#[derive(Debug, Clone, Copy)]
pub struct NetworkRef<'a> {
    doc: &'a XgwxDocument,
    index: usize,
}
/// Borrowed membership selected by its document-wide membership index.
#[derive(Debug, Clone, Copy)]
pub struct NetworkModuleRef<'a> {
    doc: &'a XgwxDocument,
    index: usize,
}
impl<'a> NetworkView<'a> {
    /// Select a network by document order.
    pub fn get(self, index: usize) -> Result<NetworkRef<'a>, crate::XgwxError> {
        self.doc
            .networks()
            .get(index)
            .ok_or(crate::XgwxError::NetworkNotFound { index })?;
        Ok(NetworkRef {
            doc: self.doc,
            index,
        })
    }
    /// Select a membership by document order, independently of hardware indices.
    pub fn module(self, index: usize) -> Result<NetworkModuleRef<'a>, crate::XgwxError> {
        self.doc
            .network_modules()
            .get(index)
            .ok_or(crate::XgwxError::NetworkModuleIndexNotFound { index })?;
        Ok(NetworkModuleRef {
            doc: self.doc,
            index,
        })
    }
    /// Select the unique membership at native coordinates.
    pub fn module_at(self, base: u32, slot: u32) -> Result<NetworkModuleRef<'a>, crate::XgwxError> {
        let indices: Vec<_> = self
            .doc
            .network_modules()
            .iter()
            .enumerate()
            .filter(|(_, m)| m.base == Some(base) && m.slot == Some(slot))
            .map(|(i, _)| i)
            .collect();
        match indices.as_slice() {
            [index] => self.module(*index),
            [] => Err(crate::XgwxError::NetworkModuleNotFound { base, slot }),
            _ => Err(crate::XgwxError::AmbiguousNetworkModule { base, slot }),
        }
    }
}
impl<'a> NetworkRef<'a> {
    /// Document-wide network index.
    pub fn index(self) -> usize {
        self.index
    }
    /// Current metadata and direct memberships.
    pub fn summary(self) -> NetworkSummary {
        self.doc.networks()[self.index].clone()
    }
    /// Memberships owned by this network, retaining document-wide indices.
    pub fn modules(self) -> Vec<NetworkModuleRef<'a>> {
        membership_owners(self.doc)
            .into_iter()
            .enumerate()
            .filter(|(_, owner)| *owner == Some(self.index))
            .map(|(index, _)| NetworkModuleRef {
                doc: self.doc,
                index,
            })
            .collect()
    }
    /// Select a membership by its position within this network.
    pub fn module(self, index: usize) -> Result<NetworkModuleRef<'a>, crate::XgwxError> {
        self.modules()
            .get(index)
            .copied()
            .ok_or(crate::XgwxError::NetworkModuleIndexNotFound { index })
    }
}
impl<'a> NetworkModuleRef<'a> {
    /// Document-wide membership index, distinct from installed hardware indices.
    pub fn index(self) -> usize {
        self.index
    }
    /// Current membership metadata.
    pub fn summary(self) -> crate::NetworkModuleSummary {
        self.doc.network_modules()[self.index].clone()
    }
    /// Resolved hardware and protocol configuration, retaining ambiguity.
    pub fn typed(self) -> NetworkModule {
        self.doc.resolved_network_modules()[self.index].clone()
    }
    /// Owning network, when present.
    pub fn network(self) -> Option<NetworkRef<'a>> {
        membership_owners(self.doc)[self.index].map(|index| NetworkRef {
            doc: self.doc,
            index,
        })
    }
}
fn membership_owners(doc: &XgwxDocument) -> Vec<Option<usize>> {
    fn walk(
        node: &crate::XmlElement,
        owner: Option<usize>,
        next: &mut usize,
        out: &mut Vec<Option<usize>>,
    ) {
        let owner = if node.name == "Network" {
            let i = *next;
            *next += 1;
            Some(i)
        } else {
            owner
        };
        if node.name == "NetworkModule" {
            out.push(owner);
        }
        for child in &node.children {
            walk(child, owner, next, out);
        }
    }
    let mut out = Vec::new();
    walk(&doc.root, None, &mut 0, &mut out);
    out
}

#[cfg(feature = "write")]
mod editors {
    use super::*;
    use crate::{
        BrowserNetworkPatch, CnetFieldEdit, CnetSettingsPatch, FenetFieldPatch, NetworkModulePatch,
        NetworkPatch, XgwxError,
    };
    /// Exclusive access to selected networks and memberships.
    pub struct NetworkCollectionEditor<'a> {
        doc: &'a mut XgwxDocument,
    }
    /// Editable network with a bound document index.
    pub struct NetworkEditor<'a> {
        doc: &'a mut XgwxDocument,
        index: usize,
    }
    /// Editable membership with a bound document-wide index.
    pub struct NetworkModuleEditor<'a> {
        doc: &'a mut XgwxDocument,
        index: usize,
    }
    /// FEnet settings for a uniquely associated selected membership.
    pub struct FenetEditor<'a> {
        module: NetworkModuleEditor<'a>,
    }
    /// Cnet serial settings for a uniquely associated selected membership.
    pub struct CnetEditor<'a> {
        module: NetworkModuleEditor<'a>,
    }
    /// Native OPC UA options associated with a selected EOPCT membership.
    pub struct OpcUaEditor<'a> {
        module: NetworkModuleEditor<'a>,
    }
    /// One captured field change; the selected handle supplies its location.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct NetworkFieldEdit {
        /// Supported metadata or protocol field name.
        pub field: String,
        /// Value captured before editing, used to reject stale changes.
        pub expected_value: String,
        /// Validated replacement value.
        pub replacement: String,
    }
    fn consistent(doc: &XgwxDocument) -> Result<(), XgwxError> {
        if crate::parse_xml(&doc.xml)? != doc.root {
            return Err(XgwxError::BrowserNetworkEdit(
                "XML and parsed network tree disagree; reparse the document".into(),
            ));
        }
        Ok(())
    }
    impl XgwxDocument {
        /// Exclusively borrow the network namespace for selected edits.
        pub fn network_mut(&mut self) -> NetworkCollectionEditor<'_> {
            NetworkCollectionEditor { doc: self }
        }
    }
    impl<'a> NetworkCollectionEditor<'a> {
        /// Create, update or delete a native XGK exported tag with a complete stale-list guard.
        pub fn edit_opcua_tag(&mut self, patch: &crate::OpcUaTagPatch) -> Result<(), XgwxError> {
            self.doc.edit_opcua_tag(patch)
        }

        /// Select a network by document order.
        pub fn get(self, index: usize) -> Result<NetworkEditor<'a>, XgwxError> {
            consistent(self.doc)?;
            self.doc.network().get(index)?;
            Ok(NetworkEditor {
                doc: self.doc,
                index,
            })
        }
        /// Select a membership by its document-wide index.
        pub fn module(self, index: usize) -> Result<NetworkModuleEditor<'a>, XgwxError> {
            consistent(self.doc)?;
            self.doc.network().module(index)?;
            Ok(NetworkModuleEditor {
                doc: self.doc,
                index,
            })
        }
        /// Select a unique membership by native coordinates.
        pub fn module_at(self, base: u32, slot: u32) -> Result<NetworkModuleEditor<'a>, XgwxError> {
            consistent(self.doc)?;
            let index = self.doc.network().module_at(base, slot)?.index();
            self.module(index)
        }
    }
    impl<'a> NetworkEditor<'a> {
        /// Borrow the selected network for inspection.
        pub fn view(&self) -> NetworkRef<'_> {
            NetworkRef {
                doc: self.doc,
                index: self.index,
            }
        }
        /// Update supported network metadata.
        pub fn update(&mut self, patch: &NetworkPatch) -> Result<(), XgwxError> {
            consistent(self.doc)?;
            self.doc.update_network(self.index, patch)
        }
        /// Change an existing metadata field with a stale-value guard.
        pub fn edit_field(&mut self, edit: &NetworkFieldEdit) -> Result<(), XgwxError> {
            consistent(self.doc)?;
            self.doc.edit_browser_network(&BrowserNetworkPatch {
                network_index: self.index,
                module: false,
                base: None,
                slot: None,
                field: edit.field.clone(),
                expected_value: edit.expected_value.clone(),
                replacement: edit.replacement.clone(),
            })
        }
        /// Transfer the exclusive borrow to a membership within this network.
        pub fn module(self, index: usize) -> Result<NetworkModuleEditor<'a>, XgwxError> {
            let index = self.view().module(index)?.index();
            Ok(NetworkModuleEditor {
                doc: self.doc,
                index,
            })
        }
    }
    impl<'a> NetworkModuleEditor<'a> {
        /// Select native EOPCT OPC UA settings at this membership's coordinates.
        pub fn opcua(self) -> Result<OpcUaEditor<'a>, XgwxError> {
            let (base, slot) = self.coordinates()?;
            self.doc.edit_opcua_settings(base, slot, &[])?;
            Ok(OpcUaEditor { module: self })
        }
        /// Borrow the selected membership for inspection.
        pub fn view(&self) -> NetworkModuleRef<'_> {
            NetworkModuleRef {
                doc: self.doc,
                index: self.index,
            }
        }
        fn coordinates(&self) -> Result<(u32, u32), XgwxError> {
            consistent(self.doc)?;
            let m = self.view().summary();
            let (Some(base), Some(slot)) = (m.base, m.slot) else {
                return Err(XgwxError::BrowserNetworkEdit(
                    "membership coordinates are absent".into(),
                ));
            };
            if self.doc.network().module_at(base, slot)?.index() != self.index {
                return Err(XgwxError::BrowserNetworkEdit(
                    "selected membership changed".into(),
                ));
            }
            Ok((base, slot))
        }
        /// Update supported membership metadata, preserving its identity.
        pub fn update(&mut self, patch: &NetworkModulePatch) -> Result<(), XgwxError> {
            let (base, slot) = self.coordinates()?;
            self.doc.update_network_module(base, slot, patch)
        }
        /// Change an existing membership field with a stale-value guard.
        pub fn edit_field(&mut self, edit: &NetworkFieldEdit) -> Result<(), XgwxError> {
            let (base, slot) = self.coordinates()?;
            let network_index = self
                .view()
                .network()
                .ok_or_else(|| {
                    XgwxError::BrowserNetworkEdit("membership has no owning network".into())
                })?
                .index();
            self.doc.edit_browser_network(&BrowserNetworkPatch {
                network_index,
                module: true,
                base: Some(base),
                slot: Some(slot),
                field: edit.field.clone(),
                expected_value: edit.expected_value.clone(),
                replacement: edit.replacement.clone(),
            })
        }
        /// Select uniquely associated FEnet settings. Missing or ambiguous settings fail.
        pub fn fenet(self) -> Result<FenetEditor<'a>, XgwxError> {
            self.coordinates()?;
            if self.view().typed().fenet_config_info().is_none() {
                return Err(XgwxError::BrowserNetworkEdit(
                    "FEnet association is absent or ambiguous".into(),
                ));
            }
            Ok(FenetEditor { module: self })
        }
        /// Select uniquely associated Cnet settings. Missing or ambiguous settings fail.
        pub fn cnet(self) -> Result<CnetEditor<'a>, XgwxError> {
            self.coordinates()?;
            if self.view().typed().cnet_config_info().is_none() {
                return Err(XgwxError::BrowserNetworkEdit(
                    "Cnet association is absent or ambiguous".into(),
                ));
            }
            Ok(CnetEditor { module: self })
        }
    }
    impl FenetEditor<'_> {
        /// Current uniquely associated FEnet configuration.
        pub fn summary(&self) -> FenetConfigInfoSummary {
            self.module
                .view()
                .typed()
                .fenet_config_info()
                .unwrap()
                .clone()
        }
        /// Edit one supported native setting with a stale-value guard.
        pub fn edit_field(&mut self, edit: &NetworkFieldEdit) -> Result<(), XgwxError> {
            let (base, slot) = self.module.coordinates()?;
            self.module.doc.edit_fenet_field(&FenetFieldPatch {
                base,
                slot,
                field: edit.field.clone(),
                expected_value: edit.expected_value.clone(),
                replacement: edit.replacement.clone(),
            })
        }
    }
    impl CnetEditor<'_> {
        /// Current uniquely associated Cnet configuration.
        pub fn summary(&self) -> CnetConfigInfoSummary {
            self.module
                .view()
                .typed()
                .cnet_config_info()
                .unwrap()
                .clone()
        }
        /// Apply native port changes atomically; all captured values must match.
        pub fn edit_settings(&mut self, changes: &[CnetFieldEdit]) -> Result<(), XgwxError> {
            let (base, slot) = self.module.coordinates()?;
            self.module.doc.edit_cnet_settings(&CnetSettingsPatch {
                base,
                slot,
                changes: changes.to_vec(),
            })
        }
    }
    impl OpcUaEditor<'_> {
        /// Apply related native server changes in one transaction.
        pub fn edit_settings(
            &mut self,
            changes: &[crate::OpcUaFieldEdit],
        ) -> Result<(), XgwxError> {
            let (base, slot) = self.module.coordinates()?;
            self.module.doc.edit_opcua_settings(base, slot, changes)
        }
    }
}
#[cfg(feature = "write")]
pub use editors::*;
