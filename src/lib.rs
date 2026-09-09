//! Parser for LS XG5000 `.xgwx` workspace files.
//!
//! The observed file layout is a small `XG` binary header, one gzip-compressed
//! UTF-8 XML project payload, and optional trailing binary metadata. This crate
//! validates and decodes that container, parses the project XML into a compact
//! tree, and keeps unknown binary sections available for callers that need them.
//! Enable the optional `il` feature for typed LD-to-IL conversion.
//! Enable the optional `write` feature for supported module and program edits
//! and loss-preserving container serialization.
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

#[cfg(feature = "write")]
mod catalog;
mod cpu;
mod document;
mod error;
#[cfg(feature = "il")]
mod il;
mod internal;
mod ladder_records;
#[cfg(feature = "write")]
mod ladder_write;
mod mnemonic;
mod model;
#[cfg(feature = "write")]
mod writer;

#[cfg(feature = "wasm")]
mod wasm;

#[cfg(feature = "write")]
pub use catalog::*;
pub use cpu::*;
pub use document::XgwxDocument;
pub use error::XgwxError;
#[cfg(feature = "il")]
pub use il::*;
#[cfg(feature = "write")]
pub use ladder_records::{LadderEditElement, LadderEditKind};
#[cfg(feature = "write")]
pub use ladder_write::{LadderBranchEdit, LadderCellEdit};
pub use mnemonic::*;
pub use model::*;
#[cfg(feature = "write")]
pub use writer::*;

#[cfg(feature = "wasm")]
pub use wasm::{cpu_catalog_wasm, parse_xgwx};
#[cfg(all(feature = "wasm", feature = "write"))]
pub use wasm::{
    delete_xgwx_module_wasm, edit_xgwx_ladder_branch_wasm, edit_xgwx_ladder_cell_wasm,
    insert_xgwx_ladder_row_wasm, insert_xgwx_module_wasm, select_xgwx_cpu_wasm,
    select_xgwx_module_wasm, set_xgwx_module_input_filter_wasm, set_xgwx_module_option_wasm,
    update_xgwx_ladder_cell_wasm, update_xgwx_module_wasm, update_xgwx_network_module_wasm,
    update_xgwx_network_wasm, update_xgwx_program_wasm, update_xgwx_variable_wasm,
    xgk_module_catalog_wasm, xgwx_module_option_values_wasm,
};

pub(crate) use internal::*;

#[cfg(test)]
mod tests;
