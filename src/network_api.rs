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
    doc: &'a XgwxDocument,
}

impl XgwxDocument {
    /// Borrow the network namespace. See [`NetworkView`] for configuration getters
    /// and examples.
    pub fn network(&self) -> NetworkView<'_> {
        NetworkView { doc: self }
    }
}

impl NetworkView<'_> {
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
