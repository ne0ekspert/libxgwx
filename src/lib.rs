//! Parser for LS XG5000 `.xgwx` workspace files.
//!
//! The observed file layout is a small `XG` binary header, one gzip-compressed
//! UTF-8 XML project payload, and optional trailing binary metadata. This crate
//! validates and decodes that container, parses the project XML into a compact
//! tree, and keeps unknown binary sections available for callers that need them.
//! Enable the optional `il` feature for typed LD-to-IL conversion.
//!
//! # Example
//!
//! ```text
//! use xgwx::XgwxDocument;
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

mod document;
mod error;
#[cfg(feature = "il")]
mod il;
mod internal;
mod mnemonic;
mod model;

#[cfg(feature = "wasm")]
mod wasm;

pub use document::XgwxDocument;
pub use error::XgwxError;
#[cfg(feature = "il")]
pub use il::*;
pub use mnemonic::*;
pub use model::*;

#[cfg(feature = "wasm")]
pub use wasm::parse_xgwx;

pub(crate) use internal::*;

#[cfg(test)]
mod tests;
