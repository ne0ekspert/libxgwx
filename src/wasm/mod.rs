use wasm_bindgen::prelude::*;

use crate::*;
use serde::Serialize;

mod ladder;
mod network;
mod parameters;
mod project;

use ladder::*;
use network::*;
use parameters::*;
use project::*;

/// Bounded hardware form edit with stale-field and full-container validation.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = edit_xgwx_browser_hardware)]
pub fn edit_xgwx_browser_hardware_wasm(bytes: &[u8], patch: JsValue) -> Result<Vec<u8>, JsValue> {
    let json = js_sys::JSON::stringify(&patch)?.as_string()
        .ok_or_else(|| JsValue::from_str("hardware patch is not JSON-serializable"))?;
    let patch: BrowserHardwarePatch = serde_json::from_str(&json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut doc = XgwxDocument::parse(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.edit_browser_hardware(&patch).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.to_verified_bytes().map_err(|e| JsValue::from_str(&e.to_string()))
}

const MAX_WASM_VARIABLES: usize = 65536;
const MAX_WASM_LADDER_PROGRAMS: usize = 64;
const MAX_WASM_LADDER_DECODED_BYTES: usize = 32 * 1024 * 1024;
const MAX_WASM_LADDER_ITEMS: usize = 100_000;

/// Place one verified scalar IEC function with operands in reference order.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_function)]
pub fn insert_xgwx_iec_ld_function_wasm(
    bytes: &[u8],
    program_index: usize,
    row_index: u16,
    raw_x: u8,
    name: &str,
    operands_json: &str,
) -> Result<Vec<u8>, JsValue> {
    let operands: Vec<String> =
        serde_json::from_str(operands_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut doc = XgwxDocument::parse(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.insert_iec_ld_function(program_index, row_index, raw_x, name, &operands)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.to_bytes()
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Parse `.xgwx` bytes and return a browser-friendly JavaScript summary.
#[wasm_bindgen]
pub fn parse_xgwx(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let doc = XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    let summary = WasmDocumentSummary::from_document(&doc);
    let json = serde_json::to_string(&summary).map_err(|error| {
        JsValue::from_str(&format!("failed to serialize xgwx summary: {error}"))
    })?;
    js_sys::JSON::parse(&json)
}

/// Apply one strictly bounded IEC demo edit with payload preservation checks.
#[cfg(feature = "write")]
#[wasm_bindgen]
pub fn edit_xgwx_browser_iec(bytes: &[u8], index: usize, patch: JsValue) -> Result<Vec<u8>, JsValue> {
    let json = js_sys::JSON::stringify(&patch)?.as_string().ok_or_else(|| JsValue::from_str("invalid patch"))?;
    let patch: BrowserIecPatch = serde_json::from_str(&json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut doc = XgwxDocument::parse(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.edit_browser_iec(index, &patch).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.to_verified_bytes().map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Probe whether this container can be rewritten without returning probe bytes.
#[cfg(feature = "write")]
#[wasm_bindgen]
pub fn check_xgwx_edit_support(bytes: &[u8]) -> Result<bool, JsValue> {
    let mut doc = XgwxDocument::parse(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.xml.push('\n');
    doc.to_verified_bytes().map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(true)
}

/// Reparse the exact download bytes and verify a lossless round trip.
#[cfg(feature = "write")]
#[wasm_bindgen]
pub fn verify_xgwx_bytes(bytes: &[u8]) -> Result<bool, JsValue> {
    let doc = XgwxDocument::parse(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let roundtrip = doc.to_verified_bytes().map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(roundtrip == bytes)
}

/// Return category and description metadata for known ladder mnemonics.
#[wasm_bindgen(js_name = known_ladder_mnemonics)]
pub fn known_ladder_mnemonics_wasm() -> Result<JsValue, JsValue> {
    let mnemonics = crate::known_ladder_mnemonics()
        .iter()
        .copied()
        .map(WasmLadderMnemonicSummary::from_info)
        .collect::<Vec<_>>();
    let json = serde_json::to_string(&mnemonics).map_err(|error| {
        JsValue::from_str(&format!(
            "failed to serialize ladder mnemonic metadata: {error}"
        ))
    })?;
    js_sys::JSON::parse(&json)
}

/// Return the cataloged XGK, XGB, and XGI CPU models.
#[wasm_bindgen(js_name = cpu_catalog)]
pub fn cpu_catalog_wasm() -> Result<JsValue, JsValue> {
    let json = serde_json::to_string(crate::cpu_catalog())
        .map_err(|error| JsValue::from_str(&format!("failed to serialize CPU catalog: {error}")))?;
    js_sys::JSON::parse(&json)
}

/// Select the primary project configuration's CPU and return rewritten bytes.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = select_xgwx_cpu)]
pub fn select_xgwx_cpu_wasm(bytes: &[u8], model: &str) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.select_cpu(model)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Set the physical slot count for an existing XGK base.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = set_xgwx_base_slot_count)]
pub fn set_xgwx_base_slot_count_wasm(
    bytes: &[u8],
    base: u32,
    slot_count: u32,
) -> Result<Vec<u8>, JsValue> {
    let mut doc = XgwxDocument::parse(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.set_base_slot_count(base, slot_count)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.to_bytes()
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Structurally edit a supported LD contact or coil at a physical cell.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = edit_xgwx_ladder_cell)]
pub fn edit_xgwx_ladder_cell_wasm(
    bytes: &[u8],
    program_index: usize,
    edit: JsValue,
) -> Result<Vec<u8>, JsValue> {
    let edit: LadderCellEdit = serde_json::from_str(
        &js_sys::JSON::stringify(&edit)?
            .as_string()
            .ok_or_else(|| JsValue::from_str("invalid ladder edit"))?,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.edit_ladder_cell(program_index, &edit)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Insert a catalog application instruction at an XGK output position.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_ladder_instruction)]
pub fn insert_xgwx_ladder_instruction_wasm(
    bytes: &[u8],
    program_index: usize,
    raw_y: u32,
    mnemonic: &str,
    operands_json: &str,
) -> Result<Vec<u8>, JsValue> {
    let operands: Vec<String> = serde_json::from_str(operands_json)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_ladder_instruction(program_index, raw_y, mnemonic, &operands)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Insert a native comparison contact at an XGK contact position.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_ladder_comparison)]
pub fn insert_xgwx_ladder_comparison_wasm(
    bytes: &[u8],
    program_index: usize,
    raw_y: u32,
    column: u8,
    mnemonic: &str,
    operands_json: &str,
) -> Result<Vec<u8>, JsValue> {
    let operands: Vec<String> = serde_json::from_str(operands_json)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_ladder_comparison(program_index, raw_y, column, mnemonic, &operands)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete a verified XGK comparison contact and its operand references.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_ladder_comparison)]
pub fn delete_xgwx_ladder_comparison_wasm(
    bytes: &[u8],
    program_index: usize,
    offset: usize,
    expected: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_ladder_comparison(program_index, offset, expected)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete a verified XGK application from an unbranched output row.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_ladder_instruction)]
pub fn delete_xgwx_ladder_instruction_wasm(
    bytes: &[u8],
    program_index: usize,
    offset: usize,
    expected: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_ladder_instruction(program_index, offset, expected)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Create or edit a supported native rung/output comment.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = edit_xgwx_ladder_comment)]
pub fn edit_xgwx_ladder_comment_wasm(
    bytes: &[u8],
    program_index: usize,
    edit: JsValue,
) -> Result<Vec<u8>, JsValue> {
    let edit: LadderCommentEdit = serde_json::from_str(
        &js_sys::JSON::stringify(&edit)?
            .as_string()
            .ok_or_else(|| JsValue::from_str("invalid comment edit"))?,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.edit_ladder_comment(program_index, &edit)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete a supported native rung comment.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_ladder_rung_comment)]
pub fn delete_xgwx_ladder_rung_comment_wasm(
    bytes: &[u8],
    program_index: usize,
    raw_y: u32,
    expected: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_ladder_rung_comment(program_index, raw_y, expected)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Add or remove a supported vertical branch connection.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = edit_xgwx_ladder_branch)]
pub fn edit_xgwx_ladder_branch_wasm(
    bytes: &[u8],
    program_index: usize,
    edit: JsValue,
) -> Result<Vec<u8>, JsValue> {
    let edit: LadderBranchEdit = serde_json::from_str(
        &js_sys::JSON::stringify(&edit)?
            .as_string()
            .ok_or_else(|| JsValue::from_str("invalid branch edit"))?,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.edit_ladder_branch(program_index, &edit)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Insert a physical blank row before the selected row.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_ladder_row)]
pub fn insert_xgwx_ladder_row_wasm(
    bytes: &[u8],
    program_index: usize,
    raw_y: u32,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_ladder_row(program_index, raw_y)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Return the embedded latest-stable XGK module selection catalog.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = xgk_module_catalog)]
pub fn xgk_module_catalog_wasm() -> Result<JsValue, JsValue> {
    let catalog = crate::xgk_module_catalog();
    let json = serde_json::to_string(catalog).map_err(|error| {
        JsValue::from_str(&format!("failed to serialize XGK module catalog: {error}"))
    })?;
    js_sys::JSON::parse(&json)
}

/// Select a catalog module and return rewritten `.xgwx` bytes.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = select_xgwx_module)]
pub fn select_xgwx_module_wasm(
    bytes: &[u8],
    base: u32,
    slot: u32,
    model: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.select_module(base, slot, model)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Insert a catalog module into an empty slot and return rewritten `.xgwx` bytes.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_module)]
pub fn insert_xgwx_module_wasm(
    bytes: &[u8],
    base: u32,
    slot: u32,
    model: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_module(base, slot, model)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete one module and return rewritten `.xgwx` bytes.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_module)]
pub fn delete_xgwx_module_wasm(bytes: &[u8], base: u32, slot: u32) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_module(base, slot)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Return the current values of all verified options for one module.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = xgwx_module_option_values)]
pub fn xgwx_module_option_values_wasm(
    bytes: &[u8],
    base: u32,
    slot: u32,
) -> Result<JsValue, JsValue> {
    let doc = XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    let values = doc
        .module_option_values(base, slot)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let json = serde_json::to_string(&values).map_err(|error| {
        JsValue::from_str(&format!(
            "failed to serialize module option values: {error}"
        ))
    })?;
    js_sys::JSON::parse(&json)
}

/// Set one verified module option and return rewritten `.xgwx` bytes.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = set_xgwx_module_option)]
pub fn set_xgwx_module_option_wasm(
    bytes: &[u8],
    base: u32,
    slot: u32,
    key: &str,
    index: u32,
    value: u32,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.set_module_option(base, slot, key, index, value)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Apply supported module attribute changes and return rewritten `.xgwx` bytes.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_module)]
pub fn update_xgwx_module_wasm(
    bytes: &[u8],
    base: u32,
    slot: u32,
    patch: JsValue,
) -> Result<Vec<u8>, JsValue> {
    let json = js_sys::JSON::stringify(&patch)?
        .as_string()
        .ok_or_else(|| JsValue::from_str("module patch is not JSON-serializable"))?;
    let patch = serde_json::from_str::<ModulePatch>(&json)
        .map_err(|error| JsValue::from_str(&format!("invalid module patch: {error}")))?;
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_module(base, slot, &patch)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Update an XGI-D24A/B input filter and return rewritten `.xgwx` bytes.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = set_xgwx_module_input_filter)]
pub fn set_xgwx_module_input_filter_wasm(
    bytes: &[u8],
    base: u32,
    slot: u32,
    raw_filter: u8,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.set_module_input_filter(base, slot, ModuleInputFilter::from_raw(raw_filter))
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Apply supported program metadata changes and return rewritten `.xgwx` bytes.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_program)]
pub fn update_xgwx_program_wasm(
    bytes: &[u8],
    program_index: usize,
    patch: JsValue,
) -> Result<Vec<u8>, JsValue> {
    let json = js_sys::JSON::stringify(&patch)?
        .as_string()
        .ok_or_else(|| JsValue::from_str("program patch is not JSON-serializable"))?;
    let patch = serde_json::from_str::<ProgramPatch>(&json)
        .map_err(|error| JsValue::from_str(&format!("invalid program patch: {error}")))?;
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_program(program_index, &patch)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_verified_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Apply supported changes to one global variable and return rewritten bytes.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_variable)]
pub fn update_xgwx_variable_wasm(
    bytes: &[u8],
    variable_index: usize,
    patch: JsValue,
) -> Result<Vec<u8>, JsValue> {
    let json = js_sys::JSON::stringify(&patch)?
        .as_string()
        .ok_or_else(|| JsValue::from_str("variable patch is not JSON-serializable"))?;
    let patch = serde_json::from_str::<VariablePatch>(&json)
        .map_err(|error| JsValue::from_str(&format!("invalid variable patch: {error}")))?;
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_variable(variable_index, &patch)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_verified_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Change a captured IEC program-local BOOL symbol's mapped address.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_iec_local_symbol_address)]
pub fn update_xgwx_iec_local_symbol_address_wasm(
    bytes: &[u8],
    program_index: usize,
    symbol_index: usize,
    expected_name: &str,
    expected_address: &str,
    replacement: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_iec_local_symbol_address(
        program_index,
        symbol_index,
        expected_name,
        expected_address,
        replacement,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Rename a captured IEC local symbol and its classified LD references.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = rename_xgwx_iec_local_symbol)]
pub fn rename_xgwx_iec_local_symbol_wasm(
    bytes: &[u8],
    program_index: usize,
    symbol_index: usize,
    expected_name: &str,
    replacement: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.rename_iec_local_symbol(program_index, symbol_index, expected_name, replacement)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Insert an unallocated primitive IEC program-local symbol.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_local_symbol)]
pub fn insert_xgwx_iec_local_symbol_wasm(
    bytes: &[u8],
    program_index: usize,
    name: &str,
    data_type: &str,
    description: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_local_symbol(program_index, name, data_type, description)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Remove an unreferenced IEC program-local symbol.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_local_symbol)]
pub fn delete_xgwx_iec_local_symbol_wasm(
    bytes: &[u8],
    program_index: usize,
    symbol_index: usize,
    expected_name: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_local_symbol(program_index, symbol_index, expected_name)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Change a captured IEC program-local symbol description.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_iec_local_symbol_description)]
pub fn update_xgwx_iec_local_symbol_description_wasm(
    bytes: &[u8],
    program_index: usize,
    symbol_index: usize,
    expected_name: &str,
    expected_description: &str,
    replacement: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_iec_local_symbol_description(
        program_index,
        symbol_index,
        expected_name,
        expected_description,
        replacement,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Change a captured automatic IEC local variable's primitive type.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_iec_local_symbol_type)]
pub fn update_xgwx_iec_local_symbol_type_wasm(
    bytes: &[u8],
    program_index: usize,
    symbol_index: usize,
    expected_name: &str,
    expected_type: &str,
    replacement: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_iec_local_symbol_type(
        program_index,
        symbol_index,
        expected_name,
        expected_type,
        replacement,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Apply one bounded existing network metadata field.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = edit_xgwx_browser_network)]
pub fn edit_xgwx_browser_network_wasm(bytes: &[u8], patch: JsValue) -> Result<Vec<u8>, JsValue> {
    let json=js_sys::JSON::stringify(&patch)?.as_string().ok_or_else(|| JsValue::from_str("invalid network patch"))?;
    let patch=serde_json::from_str::<BrowserNetworkPatch>(&json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut doc=XgwxDocument::parse(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.edit_browser_network(&patch).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.to_verified_bytes().map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Apply a batch of native Cnet serial-port changes atomically.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = edit_xgwx_cnet_settings)]
pub fn edit_xgwx_cnet_settings_wasm(bytes: &[u8], patch: JsValue) -> Result<Vec<u8>, JsValue> {
    let json = js_sys::JSON::stringify(&patch)?.as_string()
        .ok_or_else(|| JsValue::from_str("invalid Cnet patch"))?;
    let patch = serde_json::from_str::<CnetSettingsPatch>(&json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut doc = XgwxDocument::parse(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.edit_cnet_settings(&patch).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.to_verified_bytes().map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Apply supported changes to one network and return rewritten bytes.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = edit_xgwx_fenet_field)]
pub fn edit_xgwx_fenet_field_wasm(bytes: &[u8], patch: JsValue) -> Result<Vec<u8>, JsValue> {
    let json = js_sys::JSON::stringify(&patch)?.as_string()
        .ok_or_else(|| JsValue::from_str("invalid FEnet patch"))?;
    let patch = serde_json::from_str::<FenetFieldPatch>(&json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut doc = XgwxDocument::parse(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.edit_fenet_field(&patch).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.to_verified_bytes().map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Apply supported changes to one network and return rewritten bytes.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_network)]
pub fn update_xgwx_network_wasm(
    bytes: &[u8],
    network_index: usize,
    patch: JsValue,
) -> Result<Vec<u8>, JsValue> {
    let json = js_sys::JSON::stringify(&patch)?
        .as_string()
        .ok_or_else(|| JsValue::from_str("network patch is not JSON-serializable"))?;
    let patch = serde_json::from_str::<NetworkPatch>(&json)
        .map_err(|error| JsValue::from_str(&format!("invalid network patch: {error}")))?;
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_network(network_index, &patch)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Apply supported changes to one network-module metadata record.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_network_module)]
pub fn update_xgwx_network_module_wasm(
    bytes: &[u8],
    base: u32,
    slot: u32,
    patch: JsValue,
) -> Result<Vec<u8>, JsValue> {
    let json = js_sys::JSON::stringify(&patch)?
        .as_string()
        .ok_or_else(|| JsValue::from_str("network-module patch is not JSON-serializable"))?;
    let patch = serde_json::from_str::<NetworkModulePatch>(&json)
        .map_err(|error| JsValue::from_str(&format!("invalid network-module patch: {error}")))?;
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_network_module(base, slot, &patch)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Replace one same-length ladder cell string and return rewritten bytes.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_ladder_cell)]
pub fn update_xgwx_ladder_cell_wasm(
    bytes: &[u8],
    program_index: usize,
    offset: usize,
    expected: &str,
    replacement: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_ladder_cell_text(program_index, offset, expected, replacement)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Replace one verified IEC LD comment with up to 255 UTF-16 units.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_iec_ld_comment)]
pub fn update_xgwx_iec_ld_comment_wasm(
    bytes: &[u8],
    program_index: usize,
    offset: usize,
    expected: &str,
    replacement: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_iec_ld_comment(program_index, offset, expected, replacement)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Replace a verified IEC LD rising-edge contact variable reference.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_iec_ld_rising_contact_operand)]
pub fn update_xgwx_iec_ld_rising_contact_operand_wasm(
    bytes: &[u8],
    program_index: usize,
    offset: usize,
    expected: &str,
    replacement: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_iec_ld_rising_contact_operand(program_index, offset, expected, replacement)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Replace a verified IEC LD contact or output coil variable reference.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_iec_ld_element_operand)]
pub fn update_xgwx_iec_ld_element_operand_wasm(
    bytes: &[u8],
    program_index: usize,
    offset: usize,
    expected: &str,
    replacement: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_iec_ld_element_operand(program_index, offset, expected, replacement)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Change a captured IEC contact among the six addressed IEC contact kinds.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_iec_ld_contact_kind)]
pub fn update_xgwx_iec_ld_contact_kind_wasm(
    bytes: &[u8],
    program_index: usize,
    offset: usize,
    expected: &str,
    replacement: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_iec_ld_contact_kind(program_index, offset, expected, replacement)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Change a captured IEC coil among the six decoded coil kinds.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_iec_ld_coil_kind)]
pub fn update_xgwx_iec_ld_coil_kind_wasm(
    bytes: &[u8],
    program_index: usize,
    offset: usize,
    expected: &str,
    replacement: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_iec_ld_coil_kind(program_index, offset, expected, replacement)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Add or remove one guarded IEC LD vertical branch segment between adjacent
/// rows, including the captured contact-only final-branch row shape.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = edit_xgwx_iec_ld_branch_segment)]
#[allow(clippy::too_many_arguments)]
pub fn edit_xgwx_iec_ld_branch_segment_wasm(
    bytes: &[u8],
    program_index: usize,
    group_index: usize,
    start_row_index: u16,
    end_row_index: u16,
    x: u8,
    expected: bool,
    present: bool,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.edit_iec_ld_branch_segment(
        program_index,
        group_index,
        start_row_index,
        end_row_index,
        x,
        expected,
        present,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Toggle a vertical wire without deleting shared row or function records.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = edit_xgwx_iec_ld_vertical_wire)]
#[allow(clippy::too_many_arguments)]
pub fn edit_xgwx_iec_ld_vertical_wire_wasm(
    bytes: &[u8],
    program_index: usize,
    group_index: usize,
    start_row_index: u16,
    end_row_index: u16,
    x: u8,
    expected: bool,
    present: bool,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.edit_iec_ld_vertical_wire(
        program_index,
        group_index,
        start_row_index,
        end_row_index,
        x,
        expected,
        present,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Connect adjacent IEC groups, rebuilding group envelopes and adding wiring.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = connect_xgwx_iec_ld_groups)]
#[allow(clippy::too_many_arguments)]
pub fn connect_xgwx_iec_ld_groups_wasm(
    bytes: &[u8],
    program_index: usize,
    upper_group_index: usize,
    expected_upper_row: u16,
    lower_group_index: usize,
    expected_lower_row: u16,
    x: u8,
) -> Result<Vec<u8>, JsValue> {
    let mut doc = XgwxDocument::parse(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.connect_iec_ld_groups(
        program_index,
        upper_group_index,
        expected_upper_row,
        lower_group_index,
        expected_lower_row,
        x,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.to_bytes()
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Extend an IEC group into the adjacent implicit blank row.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = extend_xgwx_iec_ld_vertical_wire)]
pub fn extend_xgwx_iec_ld_vertical_wire_wasm(
    bytes: &[u8],
    program_index: usize,
    group_index: usize,
    start_row_index: u16,
    x: u8,
) -> Result<Vec<u8>, JsValue> {
    let mut doc = XgwxDocument::parse(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.extend_iec_ld_vertical_wire(program_index, group_index, start_row_index, x)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.to_bytes()
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Split disconnected IEC row ranges without changing coordinates or elements.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = split_xgwx_iec_ld_group)]
pub fn split_xgwx_iec_ld_group_wasm(
    bytes: &[u8],
    program_index: usize,
    group_index: usize,
    expected_upper_row: u16,
    expected_lower_row: u16,
) -> Result<Vec<u8>, JsValue> {
    let mut doc = XgwxDocument::parse(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.split_iec_ld_group(
        program_index,
        group_index,
        expected_upper_row,
        expected_lower_row,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.to_bytes()
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Insert one implicit IEC LD blank row after a decoded stored row, matching
/// XG5000 Ctrl+L coordinate shifting.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_blank_row)]
pub fn insert_xgwx_iec_ld_blank_row_wasm(
    bytes: &[u8],
    program_index: usize,
    after_row_index: u16,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_ld_blank_row(program_index, after_row_index)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete one implicit IEC LD blank row, matching XG5000 Ctrl+D coordinate
/// shifting while rejecting rows referenced by decoded records.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_blank_row)]
pub fn delete_xgwx_iec_ld_blank_row_wasm(
    bytes: &[u8],
    program_index: usize,
    blank_row_index: u16,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_blank_row(program_index, blank_row_index)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Create one native-shaped normally-open-contact to output-coil rung in an
/// unoccupied implicit IEC LD row.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_linear_rung)]
pub fn insert_xgwx_iec_ld_linear_rung_wasm(
    bytes: &[u8],
    program_index: usize,
    blank_row_index: u16,
    contact_variable: &str,
    coil_variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_ld_linear_rung(
        program_index,
        blank_row_index,
        contact_variable,
        coil_variable,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Insert one contact or coil into an empty IEC cell, without adding other elements.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_single_element)]
pub fn insert_xgwx_iec_ld_single_element_wasm(
    bytes: &[u8],
    program_index: usize,
    row_index: u16,
    raw_x: u8,
    category: &str,
    kind: &str,
    operand: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_ld_single_element(program_index, row_index, raw_x, category, kind, operand)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Create one addressed-contact to coil rung using decoded IEC element kinds.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_rung)]
#[allow(clippy::too_many_arguments)]
pub fn insert_xgwx_iec_ld_rung_wasm(
    bytes: &[u8],
    program_index: usize,
    blank_row_index: u16,
    contact_kind: &str,
    contact_variable: &str,
    coil_kind: &str,
    coil_variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_ld_rung(
        program_index,
        blank_row_index,
        contact_kind,
        contact_variable,
        coil_kind,
        coil_variable,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Add a normally open contact in parallel with a captured simple IEC rung.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_parallel_contact)]
pub fn insert_xgwx_iec_ld_parallel_contact_wasm(
    bytes: &[u8],
    program_index: usize,
    row_index: u16,
    expected_contact_variable: &str,
    expected_coil_variable: &str,
    parallel_variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_ld_parallel_contact(
        program_index,
        row_index,
        expected_contact_variable,
        expected_coil_variable,
        parallel_variable,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Add one of the addressed IEC contact kinds on a lower parallel branch.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_parallel_contact_kind)]
#[allow(clippy::too_many_arguments)]
pub fn insert_xgwx_iec_ld_parallel_contact_kind_wasm(
    bytes: &[u8],
    program_index: usize,
    row_index: u16,
    expected_contact_variable: &str,
    expected_coil_variable: &str,
    parallel_kind: &str,
    parallel_variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_ld_parallel_contact_kind(
        program_index,
        row_index,
        expected_contact_variable,
        expected_coil_variable,
        parallel_kind,
        parallel_variable,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete one captured normally-open-contact to output-coil rung and restore
/// its row to an implicit blank gap.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_linear_rung)]
pub fn delete_xgwx_iec_ld_linear_rung_wasm(
    bytes: &[u8],
    program_index: usize,
    row_index: u16,
    expected_contact_variable: &str,
    expected_coil_variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_linear_rung(
        program_index,
        row_index,
        expected_contact_variable,
        expected_coil_variable,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete one exact addressed-contact to coil rung, restoring an implicit gap.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_rung)]
#[allow(clippy::too_many_arguments)]
pub fn delete_xgwx_iec_ld_rung_wasm(
    bytes: &[u8],
    program_index: usize,
    row_index: u16,
    expected_contact_kind: &str,
    expected_contact_variable: &str,
    expected_coil_kind: &str,
    expected_coil_variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_rung(
        program_index,
        row_index,
        expected_contact_kind,
        expected_contact_variable,
        expected_coil_kind,
        expected_coil_variable,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Remove the terminal wire and coil from a one-row IEC rung.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_terminal_coil)]
pub fn delete_xgwx_iec_ld_terminal_coil_wasm(
    bytes: &[u8],
    program_index: usize,
    coil_record_offset: usize,
    expected_variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_terminal_coil(program_index, coil_record_offset, expected_variable)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Complete a contact-only one-row IEC rung with a wire and BOOL coil.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_terminal_coil)]
pub fn insert_xgwx_iec_ld_terminal_coil_wasm(
    bytes: &[u8],
    program_index: usize,
    contact_record_offset: usize,
    expected_contact_variable: &str,
    coil_kind: &str,
    coil_variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_ld_terminal_coil(
        program_index,
        contact_record_offset,
        expected_contact_variable,
        coil_kind,
        coil_variable,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete an occupied simple IEC LD row and shift later lines up.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_simple_row)]
#[allow(clippy::too_many_arguments)]
pub fn delete_xgwx_iec_ld_simple_row_wasm(
    bytes: &[u8],
    program_index: usize,
    row_index: u16,
    expected_contact_kind: &str,
    expected_contact_variable: &str,
    expected_coil_kind: &str,
    expected_coil_variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_simple_row(
        program_index,
        row_index,
        expected_contact_kind,
        expected_contact_variable,
        expected_coil_kind,
        expected_coil_variable,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete the upper line of a captured two-row IEC branch.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_branch_top_row)]
pub fn delete_xgwx_iec_ld_branch_top_row_wasm(
    bytes: &[u8],
    program_index: usize,
    group_index: usize,
    row_index: u16,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_branch_top_row(program_index, group_index, row_index)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete a native-shaped contact-only middle row in a nested IEC branch.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_nested_contact_branch_row)]
pub fn delete_xgwx_iec_ld_nested_contact_branch_row_wasm(
    bytes: &[u8],
    program_index: usize,
    group_index: usize,
    row_index: u16,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_nested_contact_branch_row(program_index, group_index, row_index)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete a contact-only middle row in a native-shaped x3 branch chain.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_chained_contact_branch_row)]
pub fn delete_xgwx_iec_ld_chained_contact_branch_row_wasm(
    bytes: &[u8],
    program_index: usize,
    group_index: usize,
    row_index: u16,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_chained_contact_branch_row(program_index, group_index, row_index)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete a middle IEC row containing only a vertical branch end and start.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_empty_branch_row)]
pub fn delete_xgwx_iec_ld_empty_branch_row_wasm(
    bytes: &[u8],
    program_index: usize,
    group_index: usize,
    row_index: u16,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_empty_branch_row(program_index, group_index, row_index)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete the captured FF branch output line and its connected FF block.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_ff_branch_output_row)]
pub fn delete_xgwx_iec_ld_ff_branch_output_row_wasm(
    bytes: &[u8],
    program_index: usize,
    group_index: usize,
    row_index: u16,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_ff_branch_output_row(program_index, group_index, row_index)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Insert one normally open contact into a captured linear IEC LD wire.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_no_contact)]
pub fn insert_xgwx_iec_ld_no_contact_wasm(
    bytes: &[u8],
    program_index: usize,
    wire_offset: usize,
    raw_x: u8,
    expected_wire_start_x: u8,
    expected_wire_end_x: u8,
    variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_ld_no_contact(
        program_index,
        wire_offset,
        raw_x,
        expected_wire_start_x,
        expected_wire_end_x,
        variable,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Insert one of the four decoded contact kinds into a captured linear IEC LD wire.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_contact)]
#[allow(clippy::too_many_arguments)]
pub fn insert_xgwx_iec_ld_contact_wasm(
    bytes: &[u8],
    program_index: usize,
    wire_offset: usize,
    raw_x: u8,
    expected_wire_start_x: u8,
    expected_wire_end_x: u8,
    contact_kind: &str,
    variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_ld_contact(
        program_index,
        wire_offset,
        raw_x,
        expected_wire_start_x,
        expected_wire_end_x,
        contact_kind,
        variable,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Replace a captured IEC one-cell wire with an addressed contact.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_short_wire_contact)]
pub fn insert_xgwx_iec_ld_short_wire_contact_wasm(
    bytes: &[u8],
    program_index: usize,
    wire_offset: usize,
    expected_raw_x: u8,
    contact_kind: &str,
    variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_ld_short_wire_contact(
        program_index,
        wire_offset,
        expected_raw_x,
        contact_kind,
        variable,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Insert an addressed contact into a captured upper-branch x1 gap.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_leading_contact)]
pub fn insert_xgwx_iec_ld_leading_contact_wasm(
    bytes: &[u8],
    program_index: usize,
    insertion_offset: usize,
    contact_kind: &str,
    variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_ld_leading_contact(program_index, insertion_offset, contact_kind, variable)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete one normally open contact in the captured linear IEC LD shape.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_no_contact)]
pub fn delete_xgwx_iec_ld_no_contact_wasm(
    bytes: &[u8],
    program_index: usize,
    contact_offset: usize,
    expected_raw_x: u8,
    expected_variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_no_contact(
        program_index,
        contact_offset,
        expected_raw_x,
        expected_variable,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete one decoded contact in the captured linear IEC LD shape.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_contact)]
pub fn delete_xgwx_iec_ld_contact_wasm(
    bytes: &[u8],
    program_index: usize,
    contact_offset: usize,
    expected_raw_x: u8,
    expected_contact_kind: &str,
    expected_variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_contact(
        program_index,
        contact_offset,
        expected_raw_x,
        expected_contact_kind,
        expected_variable,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete one linear-row contact with native XG5000 Cell Delete semantics.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_no_contact_cell)]
pub fn delete_xgwx_iec_ld_no_contact_cell_wasm(
    bytes: &[u8],
    program_index: usize,
    contact_offset: usize,
    expected_raw_x: u8,
    expected_variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_no_contact_cell(
        program_index,
        contact_offset,
        expected_raw_x,
        expected_variable,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete one decoded linear-row contact with native XG5000 Cell Delete semantics.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_contact_cell)]
pub fn delete_xgwx_iec_ld_contact_cell_wasm(
    bytes: &[u8],
    program_index: usize,
    contact_offset: usize,
    expected_raw_x: u8,
    expected_contact_kind: &str,
    expected_variable: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_contact_cell(
        program_index,
        contact_offset,
        expected_raw_x,
        expected_contact_kind,
        expected_variable,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Remove one captured IEC short wire between two long-wire fragments.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_horizontal_wire)]
pub fn delete_xgwx_iec_ld_horizontal_wire_wasm(
    bytes: &[u8],
    program_index: usize,
    wire_offset: usize,
    expected_raw_x: u8,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_horizontal_wire(program_index, wire_offset, expected_raw_x)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Reconnect the one-cell wire gap left by a captured IEC contact deletion.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = repair_xgwx_iec_ld_horizontal_wire)]
pub fn repair_xgwx_iec_ld_horizontal_wire_wasm(
    bytes: &[u8],
    program_index: usize,
    insertion_offset: usize,
    expected_raw_x: u8,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.repair_iec_ld_horizontal_wire(program_index, insertion_offset, expected_raw_x)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete one captured terminal IEC function block and its otherwise empty
/// pin rows while preserving the leading contact.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_terminal_function)]
pub fn delete_xgwx_iec_ld_terminal_function_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_name: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_terminal_function(program_index, block_offset, expected_name)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Remove a scalar branch-mounted block while retaining its branch rows.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_branch_function)]
pub fn delete_xgwx_iec_ld_branch_function_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_name: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_branch_function(program_index, block_offset, expected_name)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Replace a captured scalar branch function atomically, adjusting its footprint.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = replace_xgwx_iec_ld_branch_function)]
pub fn replace_xgwx_iec_ld_branch_function_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_name: &str,
    function_name: &str,
    operands_json: &str,
) -> Result<Vec<u8>, JsValue> {
    let operands: Vec<String> = serde_json::from_str(operands_json)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.replace_iec_ld_branch_function(
        program_index,
        block_offset,
        expected_name,
        function_name,
        &operands,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Replace a supported scalar chain function atomically, adjusting its footprint.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = replace_xgwx_iec_ld_scalar_chain_function)]
pub fn replace_xgwx_iec_ld_scalar_chain_function_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_name: &str,
    function_name: &str,
    operands_json: &str,
) -> Result<Vec<u8>, JsValue> {
    let operands: Vec<String> = serde_json::from_str(operands_json)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.replace_iec_ld_scalar_chain_function(
        program_index,
        block_offset,
        expected_name,
        function_name,
        &operands,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Insert a terminal TON with a declared instance and typed TIME operands.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_terminal_timer)]
pub fn insert_xgwx_iec_ld_terminal_timer_wasm(
    bytes: &[u8],
    program_index: usize,
    contact_offset: usize,
    instance: &str,
    preset: &str,
    elapsed: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc = XgwxDocument::parse(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.insert_iec_ld_terminal_timer(program_index, contact_offset, instance, preset, elapsed)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.to_bytes()
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Delete one scalar body from a shared-row horizontal chain.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_scalar_chain_function)]
pub fn delete_xgwx_iec_ld_scalar_chain_function_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_name: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc = XgwxDocument::parse(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.delete_iec_ld_scalar_chain_function(program_index, block_offset, expected_name)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.to_bytes()
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Restore the captured terminal MOVE after its retained contact.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_terminal_move)]
pub fn insert_xgwx_iec_ld_terminal_move_wasm(
    bytes: &[u8],
    program_index: usize,
    contact_offset: usize,
    input_operand: &str,
    output_operand: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_ld_terminal_move(program_index, contact_offset, input_operand, output_operand)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete one captured standalone IEC function group, leaving its stored rows
/// as an implicit blank gap.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_standalone_function)]
pub fn delete_xgwx_iec_ld_standalone_function_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_name: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_standalone_function(program_index, block_offset, expected_name)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Remove one complete decoded IEC LD network, leaving its rows blank.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_group)]
pub fn delete_xgwx_iec_ld_group_wasm(
    bytes: &[u8],
    program_index: usize,
    group_index: usize,
    expected_first_row_index: u16,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_group(program_index, group_index, expected_first_row_index)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Replace an occupied IEC LD network with a copy of another in the program.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = replace_xgwx_iec_ld_group)]
pub fn replace_xgwx_iec_ld_group_wasm(
    bytes: &[u8],
    program_index: usize,
    source_group_index: usize,
    expected_source_first_row_index: u16,
    destination_group_index: usize,
    expected_destination_first_row_index: u16,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.replace_iec_ld_group(
        program_index,
        source_group_index,
        expected_source_first_row_index,
        destination_group_index,
        expected_destination_first_row_index,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Replace a network from another IEC program, optionally copying its missing locals.
#[cfg(feature = "write")]
// Keep the established positional JavaScript API compatible with existing callers.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen(js_name = replace_xgwx_iec_ld_group_from_program)]
pub fn replace_xgwx_iec_ld_group_from_program_wasm(
    bytes: &[u8],
    source_program_index: usize,
    source_group_index: usize,
    expected_source_first_row_index: u16,
    destination_program_index: usize,
    destination_group_index: usize,
    expected_destination_first_row_index: u16,
    copy_missing_locals: bool,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.replace_iec_ld_group_from_program(
        source_program_index,
        source_group_index,
        expected_source_first_row_index,
        destination_program_index,
        destination_group_index,
        expected_destination_first_row_index,
        copy_missing_locals,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Move one complete decoded IEC LD network into an empty row range.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = move_xgwx_iec_ld_group)]
pub fn move_xgwx_iec_ld_group_wasm(
    bytes: &[u8],
    program_index: usize,
    group_index: usize,
    expected_first_row_index: u16,
    destination_first_row_index: u16,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.move_iec_ld_group(
        program_index,
        group_index,
        expected_first_row_index,
        destination_first_row_index,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Copy one complete decoded IEC LD network into an empty row range.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = copy_xgwx_iec_ld_group)]
pub fn copy_xgwx_iec_ld_group_wasm(
    bytes: &[u8],
    program_index: usize,
    group_index: usize,
    expected_first_row_index: u16,
    destination_first_row_index: u16,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.copy_iec_ld_group(
        program_index,
        group_index,
        expected_first_row_index,
        destination_first_row_index,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Copy a contact/coil IEC network from another program into an empty row range.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = copy_xgwx_iec_ld_group_to_program)]
pub fn copy_xgwx_iec_ld_group_to_program_wasm(
    bytes: &[u8],
    source_program_index: usize,
    group_index: usize,
    expected_first_row_index: u16,
    destination_program_index: usize,
    destination_first_row_index: u16,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.copy_iec_ld_group_to_program(
        source_program_index,
        group_index,
        expected_first_row_index,
        destination_program_index,
        destination_first_row_index,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Copy an IEC network and its missing supported local declarations.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = copy_xgwx_iec_ld_group_to_program_with_locals)]
pub fn copy_xgwx_iec_ld_group_to_program_with_locals_wasm(
    bytes: &[u8],
    source_program_index: usize,
    group_index: usize,
    expected_first_row_index: u16,
    destination_program_index: usize,
    destination_first_row_index: u16,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.copy_iec_ld_group_to_program_with_locals(
        source_program_index,
        group_index,
        expected_first_row_index,
        destination_program_index,
        destination_first_row_index,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Add a standalone comment to one empty IEC LD row.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_comment)]
pub fn insert_xgwx_iec_ld_comment_wasm(
    bytes: &[u8],
    program_index: usize,
    destination_row_index: u16,
    text: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_ld_comment(program_index, destination_row_index, text)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Give one captured IEC function block a separate local instance.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = duplicate_xgwx_iec_ld_function_instance)]
pub fn duplicate_xgwx_iec_ld_function_instance_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_instance: &str,
    new_instance: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.duplicate_iec_ld_function_instance(
        program_index,
        block_offset,
        expected_instance,
        new_instance,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Restore the captured standalone WORD_TO_UDINT group and operand binding.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_standalone_function)]
pub fn insert_xgwx_iec_ld_standalone_function_wasm(
    bytes: &[u8],
    program_index: usize,
    insertion_offset: usize,
    function_name: &str,
    input_operand: &str,
    output_operand: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_ld_standalone_function(
        program_index,
        insertion_offset,
        function_name,
        input_operand,
        output_operand,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete one captured connected IEC function cell while retaining the other
/// records in its stored rows.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_function_cell)]
pub fn delete_xgwx_iec_ld_function_cell_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_name: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_function_cell(program_index, block_offset, expected_name)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete the captured head EQ and close its pin-row gap in the comparison chain.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_eq_chain_head)]
pub fn delete_xgwx_iec_ld_eq_chain_head_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_name: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_eq_chain_head(program_index, block_offset, expected_name)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete the native-validated first comparison block in the heating chain.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_heating_chain_head)]
pub fn delete_xgwx_iec_ld_heating_chain_head_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_name: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_heating_chain_head(program_index, block_offset, expected_name)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete either native-validated middle EQ block in the heating chain.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_heating_chain_middle)]
pub fn delete_xgwx_iec_ld_heating_chain_middle_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_name: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_heating_chain_middle(program_index, block_offset, expected_name)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete the L58 EQ and its dangling feed, yielding a valid circuit.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_heating_chain_x3_eq_repaired)]
pub fn delete_xgwx_iec_ld_heating_chain_x3_eq_repaired_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_name: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_heating_chain_x3_eq_repaired(program_index, block_offset, expected_name)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete the captured contact-fed L62 comparison and its x6/x12 branches.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_heating_chain_contact_eq)]
pub fn delete_xgwx_iec_ld_heating_chain_contact_eq_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_name: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_heating_chain_contact_eq(program_index, block_offset, expected_name)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete either captured x15-fed comparison at L66 or L70.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_heating_chain_x15_eq)]
pub fn delete_xgwx_iec_ld_heating_chain_x15_eq_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_name: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_heating_chain_x15_eq(program_index, block_offset, expected_name)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete the captured connected ADD while preserving its neighboring circuits.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_connected_arithmetic)]
pub fn delete_xgwx_iec_ld_connected_arithmetic_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_name: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_connected_arithmetic(program_index, block_offset, expected_name)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Delete a scalar arithmetic block while retaining its external branch spine.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = delete_xgwx_iec_ld_branched_arithmetic)]
pub fn delete_xgwx_iec_ld_branched_arithmetic_wasm(
    bytes: &[u8],
    program_index: usize,
    block_offset: usize,
    expected_name: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.delete_iec_ld_branched_arithmetic(program_index, block_offset, expected_name)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Insert the captured connected single-output FF function cell.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = insert_xgwx_iec_ld_function_cell)]
pub fn insert_xgwx_iec_ld_function_cell_wasm(
    bytes: &[u8],
    program_index: usize,
    insertion_offset: usize,
    function_name: &str,
    instance_name: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.insert_iec_ld_function_cell(
        program_index,
        insertion_offset,
        function_name,
        instance_name,
    )
    .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Replace a captured IEC LD function input/output expression.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_iec_ld_function_operand)]
pub fn update_xgwx_iec_ld_function_operand_wasm(
    bytes: &[u8],
    program_index: usize,
    offset: usize,
    expected: &str,
    replacement: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_iec_ld_function_operand(program_index, offset, expected, replacement)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Change a captured IEC ADD/SUB/MUL/DIV block kind.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_iec_ld_arithmetic_function)]
pub fn update_xgwx_iec_ld_arithmetic_function_wasm(
    bytes: &[u8],
    program_index: usize,
    offset: usize,
    expected: &str,
    replacement: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_iec_ld_arithmetic_function(program_index, offset, expected, replacement)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Change a captured IEC comparison block among EQ, GT, GE, LT, and LE.
#[cfg(feature = "write")]
#[wasm_bindgen(js_name = update_xgwx_iec_ld_comparison_function)]
pub fn update_xgwx_iec_ld_comparison_function_wasm(
    bytes: &[u8],
    program_index: usize,
    offset: usize,
    expected: &str,
    replacement: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut doc =
        XgwxDocument::parse(bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.update_iec_ld_comparison_function(program_index, offset, expected, replacement)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    doc.to_bytes()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmLadderMnemonicSummary {
    mnemonic: &'static str,
    category: &'static str,
    description: &'static str,
}

impl WasmLadderMnemonicSummary {
    fn from_info(info: LadderMnemonicInfo) -> Self {
        Self {
            mnemonic: info.mnemonic,
            category: info.category.label(),
            description: info.description,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmIecSystemVariableSummary {
    name: &'static str,
    data_type: &'static str,
    writable: bool,
    description: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmDocumentSummary {
    header: WasmHeaderSummary,
    project: WasmProjectSummary,
    cpu: Option<WasmCpuSummary>,
    counts: WasmCounts,
    programs: Vec<WasmProgramSummary>,
    variables: Vec<WasmVariableSummary>,
    local_variables: Vec<Vec<WasmIecLocalSymbolSummary>>,
    iec_system_variables: Vec<WasmIecSystemVariableSummary>,
    hardware: WasmHardwareSummary,
    ladder: Vec<WasmLadderProgramSummary>,
    networks: Vec<WasmNetworkSummary>,
    xgpd: Vec<WasmXgpdSummary>,
    cnet: Vec<WasmCnetSummary>,
    fenet: Vec<WasmFenetSummary>,
    parameters: Vec<WasmParameterSummary>,
    safety_comm: Option<WasmSafetyCommSummary>,
    hsc: Vec<WasmHscSummary>,
    position: Vec<WasmPositionSummary>,
    pid: WasmPidSummary,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmCpuSummary {
    configuration_name: Option<String>,
    type_code: Option<u32>,
    model: Option<&'static str>,
    family: Option<&'static str>,
}

impl WasmDocumentSummary {
    fn from_document(doc: &XgwxDocument) -> Self {
        let mut warnings = Vec::new();
        let project = doc.project_info();
        let configurations = doc.configurations();
        let cpu = configurations.first().map(|configuration| {
            let entry = configuration.type_code.and_then(crate::cpu::cpu_for_type);
            WasmCpuSummary {
                configuration_name: configuration.name.clone(),
                type_code: configuration.type_code,
                model: entry.map(|entry| entry.model),
                family: entry.map(|entry| entry.family),
            }
        });
        let networks = doc.networks();
        let bases = doc.bases();
        let modules = doc.modules();
        let programs = doc.programs();
        let parameters = doc.parameters();
        let safety_comm = doc.safety_comm();
        let position_parameters = doc.position_parameters();
        let variable_summaries = match doc.variables() {
            Ok(variables) => Some(variables),
            Err(error) => {
                warnings.push(format!("variables: {error}"));
                None
            }
        };
        let variable_count = variable_summaries.as_ref().map(Vec::len);
        let local_variables = doc
            .iec_local_symbols()
            .into_iter()
            .enumerate()
            .map(|(program_index, result)| match result {
                Ok(symbols) => symbols
                    .into_iter()
                    .map(WasmIecLocalSymbolSummary::from_symbol)
                    .collect(),
                Err(error) => {
                    warnings.push(format!("program {program_index} local symbols: {error}"));
                    Vec::new()
                }
            })
            .collect::<Vec<Vec<WasmIecLocalSymbolSummary>>>();
        let local_variable_count = local_variables.iter().map(Vec::len).sum::<usize>();
        let variables_for_output = variable_summaries
            .map(|variables| {
                variables
                    .into_iter()
                    .take(MAX_WASM_VARIABLES)
                    .map(WasmVariableSummary::from_variable)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if let Some(total) = variable_count
            && total > MAX_WASM_VARIABLES
        {
            warnings.push(format!(
                "variables: truncated to {MAX_WASM_VARIABLES} entries from {total}"
            ));
        }

        // The web UI does not display the general payload inventory. Ladder
        // programs are decoded separately below under browser-specific budgets.
        let decoded_payload_count = 0;
        let decoded_payload_errors = 0;
        warnings.push(
            "payload decode skipped in browser summary; zero counts here do not imply absence"
                .to_owned(),
        );
        let (ladder, ladder_errors) = decode_browser_ladder(doc, programs.len(), &mut warnings);
        let ladder_program_count = ladder.len();
        let hsc = doc
            .hsc_parameters()
            .into_iter()
            .filter_map(|result| match result {
                Ok(parameter) => Some(WasmHscSummary::from_parameter(parameter)),
                Err(error) => {
                    warnings.push(format!("hsc: {error}"));
                    None
                }
            })
            .collect::<Vec<_>>();
        let pid_cal = doc.pid_cal_parameters();
        let pid_tune = doc.pid_tune_parameters();
        let cnet_configs = doc.cnet_config_infos();
        let fenet_configs = doc.fenet_config_infos();
        let xgpd_configs = doc.xgpd_config_infos();

        Self {
            header: WasmHeaderSummary::from_header(&doc.header, doc.trailer.len()),
            project: WasmProjectSummary {
                name: project.name,
                file_version: project.file_version,
                comment: project.comment,
                guid: project.guid,
                file_last_write_time: project.file_last_write_time,
            },
            cpu,
            counts: WasmCounts {
                configurations: configurations.len(),
                networks: networks.len(),
                modules: modules.len(),
                programs: programs.len(),
                variables: variable_count.map(|count| count + local_variable_count),
                decoded_payloads: decoded_payload_count,
                decoded_payload_errors,
                ladder_programs: ladder_program_count,
                ladder_errors,
                cnet_modules: cnet_configs.len(),
                fenet_modules: fenet_configs.len(),
                hsc_parameters: hsc.len(),
                position_parameters: position_parameters.len(),
                pid_cal_parameters: pid_cal.len(),
                pid_tune_parameters: pid_tune.len(),
            },
            programs: programs
                .into_iter()
                .map(WasmProgramSummary::from_program)
                .collect(),
            variables: variables_for_output,
            local_variables,
            iec_system_variables: IEC_SYSTEM_BOOL_VARIABLES
                .iter()
                .map(|&(name, description)| WasmIecSystemVariableSummary {
                    name,
                    data_type: "BOOL",
                    writable: false,
                    description,
                })
                .collect(),
            hardware: WasmHardwareSummary {
                cpu_profile: doc.cpu_hardware_profile(),
                bases: bases.into_iter().map(WasmBaseSummary::from_base).collect(),
                modules: modules
                    .into_iter()
                    .map(WasmModuleSummary::from_module)
                    .collect(),
            },
            ladder,
            networks: networks
                .into_iter()
                .map(WasmNetworkSummary::from_network)
                .collect(),
            xgpd: xgpd_configs
                .into_iter()
                .map(WasmXgpdSummary::from_xgpd)
                .collect(),
            cnet: cnet_configs
                .iter()
                .map(WasmCnetSummary::from_cnet)
                .collect(),
            fenet: fenet_configs
                .iter()
                .map(WasmFenetSummary::from_fenet)
                .collect(),
            parameters: parameters
                .into_iter()
                .map(WasmParameterSummary::from_parameter)
                .collect(),
            safety_comm: safety_comm.map(WasmSafetyCommSummary::from_safety),
            hsc,
            position: position_parameters
                .into_iter()
                .map(WasmPositionSummary::from_position)
                .collect(),
            pid: WasmPidSummary::from_parameters(pid_cal, pid_tune),
            warnings,
        }
    }
}

fn decode_browser_ladder(
    doc: &XgwxDocument,
    program_count: usize,
    warnings: &mut Vec<String>,
) -> (Vec<WasmLadderProgramSummary>, usize) {
    let mut ladder = Vec::new();
    let mut errors = 0;
    let mut decoded_bytes = 0usize;
    let mut item_count = 0usize;
    let cpu_model = doc
        .configurations()
        .first()
        .and_then(|configuration| configuration.type_code)
        .and_then(crate::cpu::cpu_for_type)
        .map(|cpu| cpu.model);

    for (program_index, element) in doc
        .root
        .descendants_named("Program")
        .take(MAX_WASM_LADDER_PROGRAMS)
        .enumerate()
    {
        let program = match LadderProgramData::from_program_element(element) {
            Ok(program) => program,
            Err(error) => {
                errors += 1;
                warnings.push(format!("ladder program {program_index}: {error}"));
                continue;
            }
        };
        let program_items = WasmLadderProgramSummary::source_item_count(&program);

        if decoded_bytes.saturating_add(program.decoded_len) > MAX_WASM_LADDER_DECODED_BYTES {
            warnings.push(format!(
                "ladder program {program_index}: omitted after reaching the {MAX_WASM_LADDER_DECODED_BYTES}-byte browser decode budget"
            ));
            break;
        }
        if item_count.saturating_add(program_items) > MAX_WASM_LADDER_ITEMS {
            warnings.push(format!(
                "ladder program {program_index}: omitted after reaching the {MAX_WASM_LADDER_ITEMS}-item browser summary budget"
            ));
            break;
        }

        decoded_bytes += program.decoded_len;
        item_count += program_items;
        ladder.push(WasmLadderProgramSummary::from_program(
            program_index,
            &program,
            cpu_model,
        ));
    }

    if program_count > MAX_WASM_LADDER_PROGRAMS {
        warnings.push(format!(
            "ladder: limited to the first {MAX_WASM_LADDER_PROGRAMS} of {program_count} programs"
        ));
    }

    (ladder, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_summary_preserves_parameter_details() {
        let doc =
            XgwxDocument::from_path("fixtures/XGB_Enet02.xgwx").expect("fixture should parse");
        let parameter_count = doc.parameters().len();
        let summary = WasmDocumentSummary::from_document(&doc);

        assert_eq!(summary.parameters.len(), parameter_count);
        let basic = summary
            .parameters
            .iter()
            .find(|parameter| parameter.parameter_type.as_deref() == Some("BASIC PARAMETER"))
            .expect("basic parameter should be present");
        assert!(
            basic
                .sections
                .iter()
                .any(|section| !section.attributes.is_empty())
        );

        assert!(
            summary
                .hsc
                .iter()
                .any(|parameter| !parameter.channels.is_empty())
        );
        assert!(
            summary
                .position
                .iter()
                .any(|parameter| { parameter.axes.iter().any(|axis| axis.parameter.is_some()) })
        );
        assert!(!summary.pid.calculation.is_empty());
        assert!(!summary.pid.tuning.is_empty());

        let json = serde_json::to_value(&summary).expect("summary should serialize");
        assert!(json.pointer("/parameters/0/sections").is_some());
        assert!(json.pointer("/position/0/axes/0/parameter").is_some());
        assert!(json.pointer("/pid/calculation/0/loops").is_some());
        assert!(json.pointer("/pid/tuning/0/loops").is_some());
        assert!(json.pointer("/cnet/0/ports/0/rxTimeout").is_some());
        assert!(json.pointer("/fenet/0/ipAddress2").is_some());

        let safety_doc =
            XgwxDocument::from_path("fixtures/elements.xgwx").expect("fixture should parse");
        let safety_summary = WasmDocumentSummary::from_document(&safety_doc);
        assert!(safety_summary.safety_comm.is_some());
        let safety_json = serde_json::to_value(&safety_summary).expect("summary should serialize");
        assert!(safety_json.pointer("/safetyComm/channels").is_some());
    }

    #[test]
    fn browser_summary_includes_drawable_ladder_data() {
        let doc =
            XgwxDocument::from_path("fixtures/elements.xgwx").expect("ladder fixture should parse");
        let summary = WasmDocumentSummary::from_document(&doc);

        assert_eq!(summary.counts.ladder_programs, 1);
        assert_eq!(summary.counts.ladder_errors, 0);
        assert_eq!(summary.ladder.len(), 1);
        assert!(!summary.ladder[0].rungs.is_empty());
        assert!(!summary.ladder[0].cells.is_empty());

        let json = serde_json::to_value(&summary).expect("summary should serialize");
        assert!(json.pointer("/ladder/0/cells/0/rawX").is_some());
        assert!(json.pointer("/ladder/0/cells/0/rawY").is_some());
        assert!(json.pointer("/ladder/0/projectType").is_some());
        assert!(json.pointer("/ladder/0/sourceStrings/0/value").is_some());
    }
    #[test]
    fn browser_summary_exposes_only_recognized_compact_profile() {
        let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").unwrap();
        let json = serde_json::to_value(WasmDocumentSummary::from_document(&doc)).unwrap();
        assert_eq!(
            json.pointer("/hardware/cpuProfile/variant").unwrap(),
            "XBM-DR16S"
        );
        let doc = XgwxDocument::from_path("fixtures/elements.xgwx").unwrap();
        let json = serde_json::to_value(WasmDocumentSummary::from_document(&doc)).unwrap();
        assert!(json.pointer("/hardware/cpuProfile").unwrap().is_null());
    }
}
