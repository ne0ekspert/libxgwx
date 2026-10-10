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
#[cfg(feature = "write")]
mod cnet_write;
#[cfg(feature = "write")]
pub use cnet_write::{CnetFieldEdit, CnetSettingsPatch};
mod comparison_catalog;
mod cpu;
mod document;
mod sfc;
mod text_program;
pub use text_program::TextProgram;
#[cfg(feature = "write")]
pub use text_program::TextProgramPatch;
pub use sfc::{
    SfcArrayBound, SfcBlock, SfcDeclaration, SfcEntity, SfcPosition, SfcProgram, SfcRow, SfcVariable,
};
#[cfg(feature = "write")]
pub use sfc::{SfcEntityPatch, SfcSequencePatch, SfcVariablePatch};
mod error;
#[cfg(feature = "write")]
mod iec_chain_comparison_write;
#[cfg(feature = "write")]
mod iec_coil_comparison_write;
#[cfg(feature = "write")]
mod iec_coil_write;
#[cfg(feature = "write")]
mod iec_connected_timer_write;
#[cfg(feature = "write")]
mod iec_contact_mesh_move_write;
#[cfg(feature = "write")]
mod iec_contact_write;
#[cfg(feature = "write")]
mod iec_conversion_pair_write;
#[cfg(feature = "write")]
mod iec_function_write;
#[cfg(feature = "write")]
mod iec_output_wire_write;
mod iec_graph;
mod iec_ld;
#[cfg(feature = "write")]
mod io_variables;
#[cfg(feature = "write")]
pub use io_variables::IoVariableGenerationRow;
#[cfg(feature = "write")]
mod iec_long_feed_timer_write;
#[cfg(feature = "write")]
mod iec_open_spine_comparison_write;
#[cfg(feature = "write")]
mod iec_paired_comparison_write;
mod iec_records;
#[cfg(feature = "write")]
mod iec_staggered_move_write;
mod iec_symbols;
#[cfg(feature = "write")]
mod iec_upper_contact_move_write;
#[cfg(feature = "il")]
mod il;
mod instruction_catalog;
mod instruction_cpu;
mod instruction_operands;
mod internal;
mod ladder_records;
#[cfg(feature = "write")]
mod ladder_write;
mod mnemonic;
mod model;
#[cfg(feature = "write")]
mod writer;
#[cfg(feature = "write")]
mod program_create;
mod program_support;
#[cfg(all(feature="write",feature="il"))]
mod vendor_il;
#[cfg(all(feature="write",feature="il"))]
pub use vendor_il::{VendorIlProgram, VendorIlPatch};
#[cfg(feature = "write")]
pub use program_create::NewProgram;

#[cfg(feature = "wasm")]
mod wasm;

#[cfg(feature = "write")]
pub use catalog::*;
pub use comparison_catalog::ladder_comparison_catalog;
pub use instruction_cpu::{LadderInstructionCpuRestriction, ladder_instruction_cpu_allowed, ladder_instruction_cpu_restriction};
pub use cpu::*;
pub use document::XgwxDocument;
pub use error::XgwxError;
pub use iec_graph::{
    IecCircuitArea, IecCircuitAreaKind, IecCircuitEdge, IecCircuitEdgeKind, IecCircuitGraph,
    IecCircuitPoint, IecFunctionBinding, IecPowerComponent,
};
pub use iec_ld::{IecGeometry, IecHorizontalSegment, IecRowFrame, IecVerticalConnection};
pub use iec_records::{
    IecConnectedArithmeticDeletionSite, IecFunctionBlock, IecFunctionCellDeletionSite,
    IecFunctionCellInsertionSite, IecFunctionOperandLink, IecFunctionPin, IecFunctionPinDirection,
    IecFunctionReference, IecHorizontalWireDeletionSite, IecHorizontalWireRepairSite,
    IecLeadingContactInsertionSite, IecNoContactDeletionSite, IecNoContactInsertionSite,
    IecRecordFrame, IecRecordKind, IecShortWireContactInsertionSite,
    IecStandaloneFunctionDeletionSite, IecStandaloneFunctionInsertionSite,
    IecTerminalFunctionDeletionSite, IecTerminalFunctionInsertionSite,
    IecWiredComparisonInsertionSite,
};
pub use iec_symbols::{IEC_SYSTEM_BOOL_VARIABLES, IecLocalSymbol};
#[cfg(feature = "il")]
pub use il::*;
pub use instruction_catalog::{LadderInstructionSpec, ladder_instruction_catalog};
pub use instruction_operands::{
    LadderOperandSpec, ladder_instruction_operand_rules, ladder_operand_type_matches,
};
#[cfg(feature = "write")]
pub use ladder_records::{LadderEditElement, LadderEditKind};
#[cfg(feature = "write")]
pub use ladder_write::{LadderBranchEdit, LadderCellEdit, LadderCommentEdit, LadderCommentKind};
pub use mnemonic::*;
pub use model::*;
#[cfg(feature = "write")]
pub use writer::*;

#[cfg(feature = "wasm")]
pub use wasm::{cpu_catalog_wasm, parse_xgwx};
#[cfg(all(feature = "wasm", feature = "write"))]
pub use wasm::{
    delete_xgwx_iec_ld_function_cell_wasm, delete_xgwx_iec_ld_no_contact_cell_wasm,
    delete_xgwx_iec_ld_no_contact_wasm, delete_xgwx_iec_ld_standalone_function_wasm,
    delete_xgwx_iec_ld_terminal_function_wasm, delete_xgwx_module_wasm,
    edit_xgwx_ladder_branch_wasm, edit_xgwx_ladder_cell_wasm, edit_xgwx_ladder_comment_wasm,
    insert_xgwx_iec_ld_contact_wasm, insert_xgwx_iec_ld_function_cell_wasm,
    insert_xgwx_iec_ld_no_contact_wasm, insert_xgwx_ladder_row_wasm, insert_xgwx_module_wasm,
    repair_xgwx_iec_ld_horizontal_wire_wasm, select_xgwx_cpu_wasm, select_xgwx_module_wasm,
    set_xgwx_module_input_filter_wasm, set_xgwx_module_option_wasm,
    update_xgwx_iec_ld_arithmetic_function_wasm, update_xgwx_iec_ld_comment_wasm,
    update_xgwx_iec_ld_comparison_function_wasm, update_xgwx_iec_ld_contact_kind_wasm,
    update_xgwx_iec_ld_element_operand_wasm, update_xgwx_iec_ld_function_operand_wasm,
    update_xgwx_iec_ld_rising_contact_operand_wasm, update_xgwx_ladder_cell_wasm,
    update_xgwx_module_wasm, update_xgwx_network_module_wasm, update_xgwx_network_wasm,
    update_xgwx_program_wasm, update_xgwx_variable_wasm, xgk_module_catalog_wasm,
    xgwx_module_option_values_wasm,
};

pub(crate) use internal::*;

#[cfg(test)]
mod tests;
