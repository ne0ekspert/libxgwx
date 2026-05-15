#![allow(dead_code)]

// These DTO converters back the optional full ladder summary path. The browser
// parser currently skips eager ladder decoding, so the constructors are dormant.
use crate::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmLadderProgramSummary {
    pub(super) program_index: usize,
    pub(super) program_name: Option<String>,
    pub(super) version: Option<String>,
    pub(super) decoded_len: usize,
    pub(super) rungs: Vec<WasmLadderRungSummary>,
    pub(super) cells: Vec<WasmLadderCellSummary>,
    pub(super) vertical_lines: Vec<WasmLadderVerticalLineSummary>,
    pub(super) branch_groups: Vec<WasmLadderBranchGroupSummary>,
    pub(super) horizontal_lines: Vec<WasmLadderHorizontalLineSummary>,
    pub(super) rung_comments: Vec<WasmLadderRungCommentSummary>,
    pub(super) output_comments: Vec<WasmLadderOutputCommentSummary>,
    pub(super) unknown_records: Vec<WasmLadderUnknownRecordSummary>,
    pub(super) instructions: Vec<WasmLadderInstructionSummary>,
}

impl WasmLadderProgramSummary {
    pub(super) fn from_program(program_index: usize, program: &LadderProgramData) -> Self {
        Self {
            program_index,
            program_name: program.program_name.clone(),
            version: program.version.clone(),
            decoded_len: program.decoded_len,
            rungs: program
                .structure
                .rungs
                .iter()
                .map(WasmLadderRungSummary::from_rung)
                .collect(),
            cells: program
                .structure
                .rungs
                .iter()
                .flat_map(|rung| rung.cells.iter())
                .map(WasmLadderCellSummary::from_cell)
                .collect(),
            vertical_lines: program
                .structure
                .vertical_lines
                .iter()
                .map(WasmLadderVerticalLineSummary::from_line)
                .collect(),
            branch_groups: program
                .structure
                .branch_groups
                .iter()
                .map(WasmLadderBranchGroupSummary::from_group)
                .collect(),
            horizontal_lines: program
                .structure
                .horizontal_lines
                .iter()
                .map(WasmLadderHorizontalLineSummary::from_line)
                .collect(),
            rung_comments: program
                .structure
                .rung_comments
                .iter()
                .map(WasmLadderRungCommentSummary::from_comment)
                .collect(),
            output_comments: program
                .structure
                .output_comments
                .iter()
                .map(WasmLadderOutputCommentSummary::from_comment)
                .collect(),
            unknown_records: program
                .structure
                .unknown_records
                .iter()
                .map(WasmLadderUnknownRecordSummary::from_record)
                .collect(),
            instructions: program
                .instructions
                .iter()
                .map(WasmLadderInstructionSummary::from_instruction)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmLadderRungSummary {
    pub(super) raw_y: u8,
    pub(super) cell_count: usize,
}

impl WasmLadderRungSummary {
    pub(super) fn from_rung(rung: &LadderRung) -> Self {
        Self {
            raw_y: rung.raw_y,
            cell_count: rung.cells.len(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmLadderCellSummary {
    pub(super) raw_x: u8,
    pub(super) raw_y: u8,
    pub(super) kind: &'static str,
    pub(super) value: String,
    pub(super) operands: Vec<String>,
    pub(super) contact: Option<&'static str>,
    pub(super) coil: Option<&'static str>,
    pub(super) mnemonic_category: Option<&'static str>,
    pub(super) mnemonic_description: Option<&'static str>,
}

impl WasmLadderCellSummary {
    pub(super) fn from_cell(cell: &LadderCell) -> Self {
        let mnemonic = ladder_mnemonic_info(&cell.value);
        Self {
            raw_x: cell.raw_x,
            raw_y: cell.raw_y,
            kind: wasm_ladder_kind_label(cell.kind),
            value: cell.value.clone(),
            operands: cell.operands.clone(),
            contact: cell.contact.map(wasm_ladder_contact_label),
            coil: cell.coil.map(wasm_ladder_coil_label),
            mnemonic_category: mnemonic.map(|info| info.category.label()),
            mnemonic_description: mnemonic.map(|info| info.description),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmLadderVerticalLineSummary {
    pub(super) raw_x: u8,
    pub(super) raw_y_start: u8,
    pub(super) raw_y_end: u8,
}

impl WasmLadderVerticalLineSummary {
    pub(super) fn from_line(line: &LadderVerticalLine) -> Self {
        Self {
            raw_x: line.raw_x,
            raw_y_start: line.raw_y_start,
            raw_y_end: line.raw_y_end,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmLadderBranchGroupSummary {
    pub(super) raw_x: u8,
    pub(super) raw_y_start: u8,
    pub(super) raw_y_end: u8,
}

impl WasmLadderBranchGroupSummary {
    pub(super) fn from_group(group: &LadderBranchGroup) -> Self {
        Self {
            raw_x: group.raw_x,
            raw_y_start: group.raw_y_start,
            raw_y_end: group.raw_y_end,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmLadderHorizontalLineSummary {
    pub(super) raw_y: u8,
    pub(super) raw_x_start: u8,
    pub(super) raw_x_end: u8,
}

impl WasmLadderHorizontalLineSummary {
    pub(super) fn from_line(line: &LadderHorizontalLine) -> Self {
        Self {
            raw_y: line.raw_y,
            raw_x_start: line.raw_x_start,
            raw_x_end: line.raw_x_end,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmLadderRungCommentSummary {
    pub(super) offset: usize,
    pub(super) raw_x: u8,
    pub(super) raw_y: u8,
    pub(super) text: String,
}

impl WasmLadderRungCommentSummary {
    pub(super) fn from_comment(comment: &LadderRungComment) -> Self {
        Self {
            offset: comment.offset,
            raw_x: comment.raw_x,
            raw_y: comment.raw_y,
            text: comment.text.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmLadderOutputCommentSummary {
    pub(super) offset: usize,
    pub(super) raw_x: u8,
    pub(super) raw_y: u8,
    pub(super) text: String,
}

impl WasmLadderOutputCommentSummary {
    pub(super) fn from_comment(comment: &LadderOutputComment) -> Self {
        Self {
            offset: comment.offset,
            raw_x: comment.raw_x,
            raw_y: comment.raw_y,
            text: comment.text.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmLadderUnknownRecordSummary {
    pub(super) offset: usize,
    pub(super) marker: String,
    pub(super) raw_x: u8,
    pub(super) raw_y: u8,
    pub(super) bytes: String,
}

impl WasmLadderUnknownRecordSummary {
    pub(super) fn from_record(record: &LadderUnknownRecord) -> Self {
        Self {
            offset: record.offset,
            marker: format!("{:02x}{:02x}", record.marker[0], record.marker[1]),
            raw_x: record.raw_x,
            raw_y: record.raw_y,
            bytes: hex_bytes(&record.bytes),
        }
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmLadderInstructionSummary {
    pub(super) mnemonic: String,
    pub(super) operands: Vec<String>,
    pub(super) category: Option<&'static str>,
    pub(super) description: Option<&'static str>,
}

impl WasmLadderInstructionSummary {
    pub(super) fn from_instruction(instruction: &LadderInstruction) -> Self {
        let mnemonic = ladder_mnemonic_info(&instruction.mnemonic);
        Self {
            mnemonic: instruction.mnemonic.clone(),
            operands: instruction.operands.clone(),
            category: mnemonic.map(|info| info.category.label()),
            description: mnemonic.map(|info| info.description),
        }
    }
}

fn wasm_ladder_kind_label(kind: LadderElementKind) -> &'static str {
    match kind {
        LadderElementKind::InstructionCall => "Instruction",
        LadderElementKind::Operation => "Operation",
        LadderElementKind::Comparison => "Comparison",
        LadderElementKind::Timer => "Timer",
        LadderElementKind::Logic => "Logic",
        LadderElementKind::DeviceRef => "Device",
        LadderElementKind::InternalRef => "Internal",
        LadderElementKind::Constant => "Constant",
        LadderElementKind::Comment => "Comment",
    }
}

fn wasm_ladder_contact_label(contact: LadderContact) -> &'static str {
    match contact {
        LadderContact::NormallyOpen => "NO",
        LadderContact::NormallyClosed => "NC",
        LadderContact::Inverse => "INV",
        LadderContact::RisingPulse => "PUP",
        LadderContact::FallingPulse => "PDN",
        LadderContact::AddressedRisingPulse => "P_CONTACT",
        LadderContact::AddressedRisingPulseNot => "P_NOT_CONTACT",
        LadderContact::AddressedFallingPulse => "N_CONTACT",
        LadderContact::AddressedFallingPulseNot => "N_NOT_CONTACT",
    }
}

fn wasm_ladder_coil_label(coil: LadderCoil) -> &'static str {
    match coil {
        LadderCoil::Output => "Output",
        LadderCoil::Inverse => "Inverse",
        LadderCoil::Set => "Set",
        LadderCoil::Reset => "Reset",
        LadderCoil::RisingPulse => "P_COIL",
        LadderCoil::FallingPulse => "N_COIL",
    }
}
