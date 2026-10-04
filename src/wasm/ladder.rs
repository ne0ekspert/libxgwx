use crate::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmLadderProgramSummary {
    pub(super) program_index: usize,
    pub(super) program_name: Option<String>,
    pub(super) version: Option<String>,
    pub(super) project_type: Option<u32>,
    pub(super) decoded_len: usize,
    pub(super) iec_rows: Vec<WasmIecRowSummary>,
    pub(super) iec_records: Vec<WasmIecRecordSummary>,
    pub(super) iec_functions: Vec<WasmIecFunctionSummary>,
    pub(super) iec_function_references: Vec<WasmIecFunctionReferenceSummary>,
    pub(super) iec_function_operand_links: Vec<WasmIecFunctionOperandLinkSummary>,
    pub(super) iec_terminal_function_deletion_sites:
        Vec<WasmIecTerminalFunctionDeletionSiteSummary>,
    pub(super) iec_terminal_timer_insertion_sites: Vec<WasmIecTerminalFunctionInsertionSiteSummary>,
    pub(super) iec_terminal_function_insertion_sites:
        Vec<WasmIecTerminalFunctionInsertionSiteSummary>,
    pub(super) iec_standalone_function_deletion_sites:
        Vec<WasmIecStandaloneFunctionDeletionSiteSummary>,
    pub(super) iec_standalone_function_insertion_sites:
        Vec<WasmIecStandaloneFunctionInsertionSiteSummary>,
    pub(super) iec_scalar_chain_deletion_sites: Vec<WasmIecFunctionCellDeletionSiteSummary>,
    pub(super) iec_wired_comparison_insertion_sites:
        Vec<WasmIecWiredComparisonInsertionSiteSummary>,
    pub(super) iec_function_cell_deletion_sites: Vec<WasmIecFunctionCellDeletionSiteSummary>,
    pub(super) iec_connected_arithmetic_deletion_sites:
        Vec<WasmIecConnectedArithmeticDeletionSiteSummary>,
    pub(super) iec_function_cell_insertion_sites: Vec<WasmIecFunctionCellInsertionSiteSummary>,
    pub(super) iec_no_contact_insertion_sites: Vec<WasmIecNoContactInsertionSiteSummary>,
    pub(super) iec_short_wire_contact_insertion_sites:
        Vec<WasmIecShortWireContactInsertionSiteSummary>,
    pub(super) iec_leading_contact_insertion_sites: Vec<WasmIecLeadingContactInsertionSiteSummary>,
    pub(super) iec_no_contact_deletion_sites: Vec<WasmIecNoContactDeletionSiteSummary>,
    pub(super) iec_no_contact_cell_deletion_sites: Vec<WasmIecNoContactDeletionSiteSummary>,
    pub(super) iec_horizontal_wire_repair_sites: Vec<WasmIecHorizontalWireRepairSiteSummary>,
    pub(super) iec_horizontal_wire_deletion_sites: Vec<WasmIecHorizontalWireDeletionSiteSummary>,
    pub(super) iec_geometry: Option<WasmIecGeometrySummary>,
    pub(super) iec_circuit_graph: Option<WasmIecCircuitGraphSummary>,
    pub(super) source_strings: Vec<WasmLadderStringSummary>,
    pub(super) structural_editing: bool,
    pub(super) instruction_choices: Vec<WasmLadderInstructionChoice>,
    pub(super) retained_instruction_choices: Vec<WasmLadderInstructionChoice>,
    pub(super) comparison_choices: Vec<WasmLadderInstructionChoice>,
    pub(super) branch_connections: Vec<WasmLadderVerticalLineSummary>,
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
    pub(super) fn source_item_count(program: &LadderProgramData) -> usize {
        source_strings(program).len()
            + program.iec_row_frames().map_or(0, |frames| frames.len())
            + program.iec_record_frames().map_or(0, |frames| frames.len())
            + program
                .iec_function_blocks()
                .map_or(0, |blocks| blocks.len())
            + program
                .iec_function_references()
                .map_or(0, |references| references.len())
            + program
                .iec_function_operand_links()
                .map_or(0, |links| links.len())
            + program
                .iec_terminal_function_deletion_sites()
                .map_or(0, |sites| sites.len())
            + program
                .iec_standalone_function_deletion_sites()
                .map_or(0, |sites| sites.len())
            + program
                .iec_standalone_function_insertion_sites()
                .map_or(0, |sites| sites.len())
            + program
                .iec_function_cell_deletion_sites()
                .map_or(0, |sites| sites.len())
            + program
                .iec_connected_arithmetic_deletion_sites()
                .map_or(0, |sites| sites.len())
            + program
                .iec_function_cell_insertion_sites()
                .map_or(0, |sites| sites.len())
            + program
                .iec_no_contact_insertion_sites()
                .map_or(0, |sites| sites.len())
            + program
                .iec_no_contact_deletion_sites()
                .map_or(0, |sites| sites.len())
            + program
                .iec_no_contact_cell_deletion_sites()
                .map_or(0, |sites| sites.len())
            + program
                .iec_horizontal_wire_repair_sites()
                .map_or(0, |sites| sites.len())
            + program.iec_geometry().map_or(0, |geometry| {
                geometry.horizontal.len() + geometry.vertical.len()
            })
            + program.structure.rungs.len()
            + program
                .structure
                .rungs
                .iter()
                .map(|rung| rung.cells.len())
                .sum::<usize>()
            + program.structure.vertical_lines.len()
            + program.structure.branch_groups.len()
            + program.structure.horizontal_lines.len()
            + program.structure.rung_comments.len()
            + program.structure.output_comments.len()
            + program.structure.unknown_records.len()
            + program.instructions.len()
    }

    pub(super) fn from_program(
        program_index: usize,
        program: &LadderProgramData,
        cpu_model: Option<&str>,
    ) -> Self {
        let iec_rows = program.iec_row_frames().unwrap_or_default();
        let iec_records = program.iec_record_frames().unwrap_or_default();
        let iec_functions = program.iec_function_blocks().unwrap_or_default();
        let iec_function_references = program.iec_function_references().unwrap_or_default();
        let iec_function_operand_links = program.iec_function_operand_links().unwrap_or_default();
        let iec_terminal_function_deletion_sites = program
            .iec_terminal_function_deletion_sites()
            .unwrap_or_default();
        let iec_terminal_function_insertion_sites = program
            .iec_terminal_function_insertion_sites()
            .unwrap_or_default();
        let iec_standalone_function_deletion_sites = program
            .iec_standalone_function_deletion_sites()
            .unwrap_or_default();
        let iec_standalone_function_insertion_sites = program
            .iec_standalone_function_insertion_sites()
            .unwrap_or_default();
        let iec_function_cell_deletion_sites = program
            .iec_function_cell_deletion_sites()
            .unwrap_or_default();
        let iec_connected_arithmetic_deletion_sites = program
            .iec_connected_arithmetic_deletion_sites()
            .unwrap_or_default();
        let iec_function_cell_insertion_sites = program
            .iec_function_cell_insertion_sites()
            .unwrap_or_default();
        let iec_no_contact_insertion_sites =
            program.iec_no_contact_insertion_sites().unwrap_or_default();
        let iec_short_wire_contact_insertion_sites = program
            .iec_short_wire_contact_insertion_sites()
            .unwrap_or_default();
        let iec_leading_contact_insertion_sites = program
            .iec_leading_contact_insertion_sites()
            .unwrap_or_default();
        let iec_no_contact_deletion_sites =
            program.iec_no_contact_deletion_sites().unwrap_or_default();
        let iec_no_contact_cell_deletion_sites = program
            .iec_no_contact_cell_deletion_sites()
            .unwrap_or_default();
        let iec_horizontal_wire_repair_sites = program
            .iec_horizontal_wire_repair_sites()
            .unwrap_or_default();
        let iec_horizontal_wire_deletion_sites = program
            .iec_horizontal_wire_deletion_sites()
            .unwrap_or_default();
        let iec_geometry = program.iec_geometry();
        let iec_circuit_graph = program.iec_circuit_layout();
        let iec_comment_offsets = crate::iec_ld::comments(program)
            .into_iter()
            .map(|item| item.offset)
            .collect::<HashSet<_>>();
        let iec_rising_contact_offsets = crate::iec_ld::rising_contact_operands(program)
            .into_iter()
            .map(|item| item.offset)
            .collect::<HashSet<_>>();
        let iec_element_kinds = crate::iec_ld::element_operands(program)
            .into_iter()
            .map(|item| {
                (
                    item.string.offset,
                    (item.kind, [u16::from(item.raw_x), item.raw_y]),
                )
            })
            .collect::<HashMap<_, _>>();
        let iec_function_operand_offsets = crate::iec_ld::function_operands(program)
            .into_iter()
            .map(|item| item.offset)
            .collect::<HashSet<_>>();
        let iec_function_operand_positions = iec_records
            .iter()
            .filter(|record| record.kind == IecRecordKind::FunctionOperand)
            .map(|record| {
                (
                    record.offset + 15,
                    [
                        u16::from(program.data[record.offset + 5]),
                        record.row_index * 4,
                    ],
                )
            })
            .collect::<HashMap<_, _>>();
        let iec_arithmetic_function_offsets = crate::iec_ld::arithmetic_function_names(program)
            .into_iter()
            .map(|item| item.offset)
            .collect::<HashSet<_>>();
        let iec_comparison_function_offsets = crate::iec_ld::comparison_function_names(program)
            .into_iter()
            .map(|item| item.offset)
            .collect::<HashSet<_>>();
        let iec_fixed_function_positions = crate::iec_ld::fixed_function_blocks(program)
            .into_iter()
            .map(|block| (block.string.offset, [u16::from(block.raw_x), block.raw_y]))
            .collect::<HashMap<_, _>>();
        let iec_function_names = iec_functions
            .iter()
            .map(|block| {
                (
                    block.name.offset,
                    [u16::from(block.raw_x), block.row_index * 4],
                )
            })
            .collect::<HashMap<_, _>>();
        let iec_function_instances = iec_functions
            .iter()
            .filter_map(|block| {
                block.instance.as_ref().map(|instance| {
                    (
                        instance.offset,
                        [u16::from(block.raw_x), block.row_index * 4],
                    )
                })
            })
            .collect::<HashMap<_, _>>();
        Self {
            program_index,
            program_name: program.program_name.clone(),
            version: program.version.clone(),
            project_type: program.project_type,
            decoded_len: program.decoded_len,
            iec_rows: iec_rows.iter().map(WasmIecRowSummary::from_frame).collect(),
            iec_records: iec_records
                .iter()
                .map(WasmIecRecordSummary::from_frame)
                .collect(),
            iec_functions: iec_functions
                .iter()
                .map(WasmIecFunctionSummary::from_block)
                .collect(),
            iec_function_references: iec_function_references
                .iter()
                .map(WasmIecFunctionReferenceSummary::from_reference)
                .collect(),
            iec_function_operand_links: iec_function_operand_links
                .iter()
                .map(WasmIecFunctionOperandLinkSummary::from_link)
                .collect(),
            iec_terminal_function_deletion_sites: iec_terminal_function_deletion_sites
                .iter()
                .map(WasmIecTerminalFunctionDeletionSiteSummary::from_site)
                .collect(),
            iec_terminal_timer_insertion_sites: program
                .iec_terminal_timer_insertion_sites()
                .unwrap_or_default()
                .iter()
                .map(WasmIecTerminalFunctionInsertionSiteSummary::from_site)
                .collect(),
            iec_terminal_function_insertion_sites: iec_terminal_function_insertion_sites
                .iter()
                .map(WasmIecTerminalFunctionInsertionSiteSummary::from_site)
                .collect(),
            iec_standalone_function_deletion_sites: iec_standalone_function_deletion_sites
                .iter()
                .map(WasmIecStandaloneFunctionDeletionSiteSummary::from_site)
                .collect(),
            iec_standalone_function_insertion_sites: iec_standalone_function_insertion_sites
                .iter()
                .map(WasmIecStandaloneFunctionInsertionSiteSummary::from_site)
                .collect(),
            iec_scalar_chain_deletion_sites: {
                #[cfg(feature = "write")]
                {
                    program
                        .iec_scalar_chain_deletion_sites()
                        .unwrap_or_default()
                        .iter()
                        .map(WasmIecFunctionCellDeletionSiteSummary::from_site)
                        .collect()
                }
                #[cfg(not(feature = "write"))]
                {
                    Vec::new()
                }
            },
            iec_wired_comparison_insertion_sites: {
                #[cfg(feature = "write")]
                {
                    program
                        .iec_wired_comparison_insertion_sites()
                        .unwrap_or_default()
                        .iter()
                        .map(|site| WasmIecWiredComparisonInsertionSiteSummary {
                            group_index: site.group_index,
                            row_index: site.row_index,
                            raw_x: site.raw_x,
                        })
                        .collect()
                }
                #[cfg(not(feature = "write"))]
                {
                    Vec::new()
                }
            },
            iec_function_cell_deletion_sites: iec_function_cell_deletion_sites
                .iter()
                .map(WasmIecFunctionCellDeletionSiteSummary::from_site)
                .collect(),
            iec_connected_arithmetic_deletion_sites: iec_connected_arithmetic_deletion_sites
                .iter()
                .map(WasmIecConnectedArithmeticDeletionSiteSummary::from_site)
                .collect(),
            iec_function_cell_insertion_sites: iec_function_cell_insertion_sites
                .iter()
                .map(WasmIecFunctionCellInsertionSiteSummary::from_site)
                .collect(),
            iec_no_contact_insertion_sites: iec_no_contact_insertion_sites
                .iter()
                .map(WasmIecNoContactInsertionSiteSummary::from_site)
                .collect(),
            iec_short_wire_contact_insertion_sites: iec_short_wire_contact_insertion_sites
                .iter()
                .map(WasmIecShortWireContactInsertionSiteSummary::from_site)
                .collect(),
            iec_leading_contact_insertion_sites: iec_leading_contact_insertion_sites
                .iter()
                .map(WasmIecLeadingContactInsertionSiteSummary::from_site)
                .collect(),
            iec_no_contact_deletion_sites: iec_no_contact_deletion_sites
                .iter()
                .map(WasmIecNoContactDeletionSiteSummary::from_site)
                .collect(),
            iec_no_contact_cell_deletion_sites: iec_no_contact_cell_deletion_sites
                .iter()
                .map(WasmIecNoContactDeletionSiteSummary::from_site)
                .collect(),
            iec_horizontal_wire_repair_sites: iec_horizontal_wire_repair_sites
                .iter()
                .map(WasmIecHorizontalWireRepairSiteSummary::from_site)
                .collect(),
            iec_horizontal_wire_deletion_sites: iec_horizontal_wire_deletion_sites
                .iter()
                .map(WasmIecHorizontalWireDeletionSiteSummary::from_site)
                .collect(),
            iec_geometry: iec_geometry
                .as_ref()
                .map(WasmIecGeometrySummary::from_geometry),
            iec_circuit_graph: iec_circuit_graph
                .as_ref()
                .map(WasmIecCircuitGraphSummary::from_graph),
            source_strings: source_strings(program)
                .iter()
                .map(|item| {
                    WasmLadderStringSummary::from_string(
                        item,
                        iec_comment_offsets.contains(&item.offset),
                        iec_rising_contact_offsets.contains(&item.offset),
                        iec_element_kinds.get(&item.offset).copied(),
                        iec_function_operand_offsets.contains(&item.offset),
                        iec_function_operand_positions.get(&item.offset).copied(),
                        iec_arithmetic_function_offsets.contains(&item.offset),
                        iec_comparison_function_offsets.contains(&item.offset),
                        iec_fixed_function_positions.get(&item.offset).copied(),
                        iec_function_names.get(&item.offset).copied(),
                        iec_function_instances.get(&item.offset).copied(),
                        iec_rows
                            .iter()
                            .find(|row| item.offset >= row.records_start && item.offset < row.end),
                        iec_records.iter().find(|record| {
                            item.offset >= record.offset && item.offset < record.end
                        }),
                    )
                })
                .collect(),
            structural_editing: {
                #[cfg(feature = "write")]
                {
                    program.version.as_deref() == Some("LD VER 1.1")
                        && program.project_type == Some(1)
                        && crate::ladder_write::editable_ladder_supported(&program.data)
                }
                #[cfg(not(feature = "write"))]
                {
                    false
                }
            },
            instruction_choices: if program.project_type == Some(1) {
                crate::ladder_instruction_catalog()
                    .iter()
                    .filter(|spec| {
                        cpu_model.is_none_or(|model| {
                            crate::ladder_instruction_cpu_allowed(spec.mnemonic, model) != Some(false)
                        })
                    })
                    .map(WasmLadderInstructionChoice::from)
                    .collect()
            } else {
                Vec::new()
            },
            retained_instruction_choices: if program.project_type == Some(1) {
                crate::ladder_instruction_catalog()
                    .iter()
                    .filter(|spec| {
                        cpu_model.is_some_and(|model| {
                            crate::ladder_instruction_cpu_allowed(spec.mnemonic, model) == Some(false)
                        }) && program.structure.rungs.iter().any(|rung| {
                            rung.cells.iter().any(|cell| cell.value == spec.mnemonic)
                        })
                    })
                    .map(WasmLadderInstructionChoice::from)
                    .collect()
            } else {
                Vec::new()
            },
            comparison_choices: if program.project_type == Some(1) {
                crate::ladder_comparison_catalog()
                    .iter()
                    .filter(|spec| {
                        cpu_model.is_none_or(|model| {
                            crate::ladder_instruction_cpu_allowed(spec.mnemonic, model) != Some(false)
                        })
                    })
                    .map(WasmLadderInstructionChoice::from)
                    .collect()
            } else {
                Vec::new()
            },
            branch_connections: {
                #[cfg(feature = "write")]
                {
                    crate::ladder_write::ladder_connections(&program.data)
                        .unwrap_or_default()
                        .into_iter()
                        .map(
                            |(raw_x, raw_y_start, raw_y_end)| WasmLadderVerticalLineSummary {
                                raw_x,
                                raw_y_start,
                                raw_y_end,
                            },
                        )
                        .collect()
                }
                #[cfg(not(feature = "write"))]
                {
                    Vec::new()
                }
            },
            rungs: {
                let decoded = || {
                    program
                        .structure
                        .rungs
                        .iter()
                        .map(WasmLadderRungSummary::from_rung)
                        .collect()
                };
                #[cfg(feature = "write")]
                {
                    match crate::ladder_write::editable_ladder_rows(&program.data) {
                        Ok(rows) => rows
                            .into_iter()
                            .map(|raw_y| WasmLadderRungSummary {
                                raw_y,
                                cell_count: program
                                    .structure
                                    .rungs
                                    .iter()
                                    .find(|row| row.raw_y == raw_y)
                                    .map_or(0, |row| row.cells.len()),
                            })
                            .collect(),
                        Err(_) => decoded(),
                    }
                }
                #[cfg(not(feature = "write"))]
                {
                    decoded()
                }
            },
            cells: program
                .structure
                .rungs
                .iter()
                .flat_map(|rung| rung.cells.iter())
                .map(|cell| WasmLadderCellSummary::from_cell(cell, program))
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
pub(super) struct WasmIecRowSummary {
    pub(super) group_index: usize,
    pub(super) row_index: u16,
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) record_count: u16,
}

impl WasmIecRowSummary {
    fn from_frame(frame: &IecRowFrame) -> Self {
        Self {
            group_index: frame.group_index,
            row_index: frame.row_index,
            start: frame.start,
            end: frame.end,
            record_count: frame.record_count,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecRecordSummary {
    group_index: usize,
    row_index: u16,
    offset: usize,
    end: usize,
    kind: &'static str,
    code: Option<u8>,
}

impl WasmIecRecordSummary {
    fn from_frame(frame: &IecRecordFrame) -> Self {
        let (kind, code) = match frame.kind {
            IecRecordKind::LongWire => ("Long wire", None),
            IecRecordKind::ShortWire => ("Short wire", None),
            IecRecordKind::Contact(code) => ("Contact", Some(code)),
            IecRecordKind::Coil(code) => ("Coil", Some(code)),
            IecRecordKind::Comment => ("Comment", None),
            IecRecordKind::FunctionOperand => ("Function operand", None),
            IecRecordKind::BranchStart => ("Branch start", None),
            IecRecordKind::BranchEnd => ("Branch end", None),
            IecRecordKind::LinkReference(code) => ("Link reference", Some(code)),
            IecRecordKind::FunctionBlock => ("Function block", None),
        };
        Self {
            group_index: frame.group_index,
            row_index: frame.row_index,
            offset: frame.offset,
            end: frame.end,
            kind,
            code,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecFunctionReferenceSummary {
    group_index: usize,
    row_index: u16,
    record_offset: usize,
    target_record_offset: usize,
    target_row_index: u16,
    raw_x: u8,
    code: u8,
    ordinal: u8,
    is_output: bool,
    pin_row_index: u16,
    pin_raw_x: u8,
    data_type_mask: u32,
    is_array: bool,
}

impl WasmIecFunctionReferenceSummary {
    fn from_reference(reference: &IecFunctionReference) -> Self {
        Self {
            group_index: reference.group_index,
            row_index: reference.row_index,
            record_offset: reference.record_offset,
            target_record_offset: reference.target_record_offset,
            target_row_index: reference.target_row_index,
            raw_x: reference.raw_x,
            code: reference.code,
            ordinal: reference.ordinal,
            is_output: reference.is_output,
            pin_row_index: reference.pin_row_index,
            pin_raw_x: reference.pin_raw_x,
            data_type_mask: reference.data_type_mask,
            is_array: reference.is_array,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecFunctionOperandLinkSummary {
    group_index: usize,
    row_index: u16,
    record_offset: usize,
    target_record_offset: usize,
    ordinal: u8,
    is_output: bool,
    pin_row_index: u16,
    pin_raw_x: u8,
    data_type_mask: u32,
    is_array: bool,
}

impl WasmIecFunctionOperandLinkSummary {
    fn from_link(link: &IecFunctionOperandLink) -> Self {
        Self {
            group_index: link.group_index,
            row_index: link.row_index,
            record_offset: link.record_offset,
            target_record_offset: link.target_record_offset,
            ordinal: link.ordinal,
            is_output: link.is_output,
            pin_row_index: link.pin_row_index,
            pin_raw_x: link.pin_raw_x,
            data_type_mask: link.data_type_mask,
            is_array: link.is_array,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecTerminalFunctionDeletionSiteSummary {
    group_index: usize,
    row_index: u16,
    block_offset: usize,
    wire_offset: usize,
    raw_x: u8,
    pin_count: u8,
}

impl WasmIecTerminalFunctionDeletionSiteSummary {
    fn from_site(site: &IecTerminalFunctionDeletionSite) -> Self {
        Self {
            group_index: site.group_index,
            row_index: site.row_index,
            block_offset: site.block_offset,
            wire_offset: site.wire_offset,
            raw_x: site.raw_x,
            pin_count: site.pin_count,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecTerminalFunctionInsertionSiteSummary {
    group_index: usize,
    row_index: u16,
    contact_offset: usize,
    insertion_offset: usize,
    raw_x: u8,
}

impl WasmIecTerminalFunctionInsertionSiteSummary {
    fn from_site(site: &IecTerminalFunctionInsertionSite) -> Self {
        Self {
            group_index: site.group_index,
            row_index: site.row_index,
            contact_offset: site.contact_offset,
            insertion_offset: site.insertion_offset,
            raw_x: site.raw_x,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecStandaloneFunctionDeletionSiteSummary {
    group_index: usize,
    row_index: u16,
    block_offset: usize,
    wire_offset: usize,
    raw_x: u8,
    pin_count: u8,
}

impl WasmIecStandaloneFunctionDeletionSiteSummary {
    fn from_site(site: &IecStandaloneFunctionDeletionSite) -> Self {
        Self {
            group_index: site.group_index,
            row_index: site.row_index,
            block_offset: site.block_offset,
            wire_offset: site.wire_offset,
            raw_x: site.raw_x,
            pin_count: site.pin_count,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecStandaloneFunctionInsertionSiteSummary {
    group_index: usize,
    row_index: u16,
    insertion_offset: usize,
    raw_x: u8,
}

impl WasmIecStandaloneFunctionInsertionSiteSummary {
    fn from_site(site: &IecStandaloneFunctionInsertionSite) -> Self {
        Self {
            group_index: site.group_index,
            row_index: site.row_index,
            insertion_offset: site.insertion_offset,
            raw_x: site.raw_x,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecFunctionCellDeletionSiteSummary {
    group_index: usize,
    row_index: u16,
    block_offset: usize,
    raw_x: u8,
    pin_count: u8,
}

impl WasmIecFunctionCellDeletionSiteSummary {
    fn from_site(site: &IecFunctionCellDeletionSite) -> Self {
        Self {
            group_index: site.group_index,
            row_index: site.row_index,
            block_offset: site.block_offset,
            raw_x: site.raw_x,
            pin_count: site.pin_count,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecConnectedArithmeticDeletionSiteSummary {
    group_index: usize,
    row_index: u16,
    block_offset: usize,
    wire_offset: usize,
    raw_x: u8,
}

impl WasmIecConnectedArithmeticDeletionSiteSummary {
    fn from_site(site: &IecConnectedArithmeticDeletionSite) -> Self {
        Self {
            group_index: site.group_index,
            row_index: site.row_index,
            block_offset: site.block_offset,
            wire_offset: site.wire_offset,
            raw_x: site.raw_x,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecWiredComparisonInsertionSiteSummary {
    group_index: usize,
    row_index: u16,
    raw_x: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecFunctionCellInsertionSiteSummary {
    function_name: &'static str,
    group_index: usize,
    row_index: u16,
    insertion_offset: usize,
    reference_offset: usize,
    raw_x: u8,
}

impl WasmIecFunctionCellInsertionSiteSummary {
    fn from_site(site: &IecFunctionCellInsertionSite) -> Self {
        Self {
            function_name: site.function_name,
            group_index: site.group_index,
            row_index: site.row_index,
            insertion_offset: site.insertion_offset,
            reference_offset: site.reference_offset,
            raw_x: site.raw_x,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecNoContactInsertionSiteSummary {
    group_index: usize,
    row_index: u16,
    wire_offset: usize,
    start_x: u8,
    end_x: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecShortWireContactInsertionSiteSummary {
    group_index: usize,
    row_index: u16,
    wire_offset: usize,
    raw_x: u8,
}

impl WasmIecShortWireContactInsertionSiteSummary {
    fn from_site(site: &IecShortWireContactInsertionSite) -> Self {
        Self {
            group_index: site.group_index,
            row_index: site.row_index,
            wire_offset: site.wire_offset,
            raw_x: site.raw_x,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecLeadingContactInsertionSiteSummary {
    group_index: usize,
    row_index: u16,
    insertion_offset: usize,
}

impl WasmIecLeadingContactInsertionSiteSummary {
    fn from_site(site: &IecLeadingContactInsertionSite) -> Self {
        Self {
            group_index: site.group_index,
            row_index: site.row_index,
            insertion_offset: site.insertion_offset,
        }
    }
}

impl WasmIecNoContactInsertionSiteSummary {
    fn from_site(site: &IecNoContactInsertionSite) -> Self {
        Self {
            group_index: site.group_index,
            row_index: site.row_index,
            wire_offset: site.wire_offset,
            start_x: site.start_x,
            end_x: site.end_x,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecNoContactDeletionSiteSummary {
    group_index: usize,
    row_index: u16,
    contact_offset: usize,
    raw_x: u8,
    contact_code: u8,
}

impl WasmIecNoContactDeletionSiteSummary {
    fn from_site(site: &IecNoContactDeletionSite) -> Self {
        Self {
            group_index: site.group_index,
            row_index: site.row_index,
            contact_offset: site.contact_offset,
            raw_x: site.raw_x,
            contact_code: site.contact_code,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecHorizontalWireRepairSiteSummary {
    group_index: usize,
    row_index: u16,
    insertion_offset: usize,
    raw_x: u8,
}

impl WasmIecHorizontalWireRepairSiteSummary {
    fn from_site(site: &IecHorizontalWireRepairSite) -> Self {
        Self {
            group_index: site.group_index,
            row_index: site.row_index,
            insertion_offset: site.insertion_offset,
            raw_x: site.raw_x,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecHorizontalWireDeletionSiteSummary {
    group_index: usize,
    row_index: u16,
    wire_offset: usize,
    raw_x: u8,
}

impl WasmIecHorizontalWireDeletionSiteSummary {
    fn from_site(site: &IecHorizontalWireDeletionSite) -> Self {
        Self {
            group_index: site.group_index,
            row_index: site.row_index,
            wire_offset: site.wire_offset,
            raw_x: site.raw_x,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecFunctionSummary {
    group_index: usize,
    row_index: u16,
    record_offset: usize,
    record_end: usize,
    raw_x: u8,
    opcode_family: u8,
    opcode: u16,
    pin_count: u8,
    name: String,
    name_offset: usize,
    instance: Option<String>,
    instance_offset: Option<usize>,
    control_input: WasmIecFunctionPinSummary,
    control_output: WasmIecFunctionPinSummary,
    pins: Vec<WasmIecFunctionPinSummary>,
    field_strings: Vec<WasmIecFunctionFieldSummary>,
}

impl WasmIecFunctionSummary {
    fn from_block(block: &IecFunctionBlock) -> Self {
        Self {
            group_index: block.group_index,
            row_index: block.row_index,
            record_offset: block.record_offset,
            record_end: block.record_end,
            raw_x: block.raw_x,
            opcode_family: block.opcode_family,
            opcode: block.opcode,
            pin_count: block.pin_count,
            name: block.name.value.clone(),
            name_offset: block.name.offset,
            instance: block.instance.as_ref().map(|item| item.value.clone()),
            instance_offset: block.instance.as_ref().map(|item| item.offset),
            control_input: WasmIecFunctionPinSummary::from_pin(&block.control_input),
            control_output: WasmIecFunctionPinSummary::from_pin(&block.control_output),
            pins: block
                .pins
                .iter()
                .map(WasmIecFunctionPinSummary::from_pin)
                .collect(),
            field_strings: block
                .field_strings
                .iter()
                .map(WasmIecFunctionFieldSummary::from_string)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmIecFunctionPinSummary {
    name: String,
    name_offset: usize,
    type_expression: Option<String>,
    raw_type_flags: u32,
    data_type_mask: u32,
    data_type: Option<&'static str>,
    is_array: bool,
    direction: &'static str,
    is_control: bool,
    reference_ordinal: Option<u8>,
    raw_x: u8,
    row_index: u16,
}

impl WasmIecFunctionPinSummary {
    fn from_pin(pin: &IecFunctionPin) -> Self {
        Self {
            name: pin.name.value.clone(),
            name_offset: pin.name.offset,
            type_expression: pin
                .type_expression
                .as_ref()
                .map(|field| field.value.clone()),
            raw_type_flags: pin.raw_type_flags,
            data_type_mask: pin.data_type_mask,
            data_type: pin.data_type,
            is_array: pin.is_array,
            direction: match pin.direction {
                IecFunctionPinDirection::Input => "input",
                IecFunctionPinDirection::Output => "output",
            },
            is_control: pin.is_control,
            reference_ordinal: pin.reference_ordinal,
            raw_x: pin.raw_x,
            row_index: pin.row_index,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmIecFunctionFieldSummary {
    offset: usize,
    value: String,
}

impl WasmIecFunctionFieldSummary {
    fn from_string(item: &LadderString) -> Self {
        Self {
            offset: item.offset,
            value: item.value.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecGeometrySummary {
    horizontal: Vec<WasmIecHorizontalSummary>,
    vertical: Vec<WasmIecVerticalSummary>,
}

impl WasmIecGeometrySummary {
    fn from_geometry(geometry: &IecGeometry) -> Self {
        Self {
            horizontal: geometry
                .horizontal
                .iter()
                .map(WasmIecHorizontalSummary::from_segment)
                .collect(),
            vertical: geometry
                .vertical
                .iter()
                .map(WasmIecVerticalSummary::from_connection)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmIecHorizontalSummary {
    group_index: usize,
    row_index: u16,
    offset: usize,
    start_x: u8,
    end_x: u8,
}

impl WasmIecHorizontalSummary {
    fn from_segment(segment: &IecHorizontalSegment) -> Self {
        Self {
            group_index: segment.group_index,
            row_index: segment.row_index,
            offset: segment.offset,
            start_x: segment.start_x,
            end_x: segment.end_x,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmIecVerticalSummary {
    group_index: usize,
    start_row_index: u16,
    end_row_index: u16,
    start_offset: usize,
    end_offset: usize,
    x: u8,
}

impl WasmIecVerticalSummary {
    fn from_connection(connection: &IecVerticalConnection) -> Self {
        Self {
            group_index: connection.group_index,
            start_row_index: connection.start_row_index,
            end_row_index: connection.end_row_index,
            start_offset: connection.start_offset,
            end_offset: connection.end_offset,
            x: connection.x,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmIecCircuitGraphSummary {
    edges: Vec<WasmIecCircuitEdgeSummary>,
    occupied_areas: Vec<WasmIecCircuitAreaSummary>,
    function_bindings: Vec<WasmIecFunctionBindingSummary>,
    power_components: Vec<WasmIecPowerComponentSummary>,
    open_branch_endpoints: Vec<WasmIecCircuitPointSummary>,
}

impl WasmIecCircuitGraphSummary {
    fn from_graph(graph: &IecCircuitGraph) -> Self {
        Self {
            open_branch_endpoints: graph
                .open_branch_endpoints
                .iter()
                .copied()
                .map(WasmIecCircuitPointSummary::from_point)
                .collect(),
            edges: graph
                .edges
                .iter()
                .map(WasmIecCircuitEdgeSummary::from_edge)
                .collect(),
            occupied_areas: graph
                .occupied_areas
                .iter()
                .map(WasmIecCircuitAreaSummary::from_area)
                .collect(),
            function_bindings: graph
                .function_bindings
                .iter()
                .map(WasmIecFunctionBindingSummary::from_binding)
                .collect(),
            power_components: graph
                .power_components
                .iter()
                .map(WasmIecPowerComponentSummary::from_component)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmIecCircuitEdgeSummary {
    kind: &'static str,
    code: Option<u8>,
    start: WasmIecCircuitPointSummary,
    end: WasmIecCircuitPointSummary,
    record_offset: usize,
    paired_record_offset: Option<usize>,
}

impl WasmIecCircuitEdgeSummary {
    fn from_edge(edge: &IecCircuitEdge) -> Self {
        let (kind, code) = match edge.kind {
            IecCircuitEdgeKind::HorizontalWire => ("horizontalWire", None),
            IecCircuitEdgeKind::Contact(code) => ("contact", Some(code)),
            IecCircuitEdgeKind::Coil(code) => ("coil", Some(code)),
            IecCircuitEdgeKind::FunctionEnable => ("functionEnable", None),
            IecCircuitEdgeKind::VerticalBranch => ("verticalBranch", None),
        };
        Self {
            kind,
            code,
            start: WasmIecCircuitPointSummary::from_point(edge.start),
            end: WasmIecCircuitPointSummary::from_point(edge.end),
            record_offset: edge.record_offset,
            paired_record_offset: edge.paired_record_offset,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmIecCircuitPointSummary {
    group_index: usize,
    row_index: u16,
    x: u8,
}

impl WasmIecCircuitPointSummary {
    fn from_point(point: IecCircuitPoint) -> Self {
        Self {
            group_index: point.group_index,
            row_index: point.row_index,
            x: point.x,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmIecCircuitAreaSummary {
    kind: &'static str,
    code: Option<u8>,
    group_index: usize,
    start_row_index: u16,
    end_row_index: u16,
    start_x: u8,
    end_x: u8,
    record_offset: usize,
}

impl WasmIecCircuitAreaSummary {
    fn from_area(area: &IecCircuitArea) -> Self {
        let (kind, code) = match area.kind {
            IecCircuitAreaKind::HorizontalWire => ("horizontalWire", None),
            IecCircuitAreaKind::Contact(code) => ("contact", Some(code)),
            IecCircuitAreaKind::Coil(code) => ("coil", Some(code)),
            IecCircuitAreaKind::FunctionBlock => ("functionBlock", None),
            IecCircuitAreaKind::FunctionOperand => ("functionOperand", None),
        };
        Self {
            kind,
            code,
            group_index: area.group_index,
            start_row_index: area.start_row_index,
            end_row_index: area.end_row_index,
            start_x: area.start_x,
            end_x: area.end_x,
            record_offset: area.record_offset,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmIecFunctionBindingSummary {
    group_index: usize,
    block_record_offset: usize,
    reference_record_offset: usize,
    expression_record_offset: Option<usize>,
    ordinal: u8,
    direction: &'static str,
    function_name: String,
    pin_name: String,
    pin_point: WasmIecCircuitPointSummary,
    expression_cell_x: Option<u8>,
    data_type_mask: u32,
    is_array: bool,
}

impl WasmIecFunctionBindingSummary {
    fn from_binding(binding: &IecFunctionBinding) -> Self {
        Self {
            group_index: binding.group_index,
            block_record_offset: binding.block_record_offset,
            reference_record_offset: binding.reference_record_offset,
            expression_record_offset: binding.expression_record_offset,
            ordinal: binding.ordinal,
            direction: match binding.direction {
                IecFunctionPinDirection::Input => "input",
                IecFunctionPinDirection::Output => "output",
            },
            function_name: binding.function_name.clone(),
            pin_name: binding.pin_name.clone(),
            pin_point: WasmIecCircuitPointSummary::from_point(binding.pin_point),
            expression_cell_x: binding.expression_cell_x,
            data_type_mask: binding.data_type_mask,
            is_array: binding.is_array,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmIecPowerComponentSummary {
    group_index: usize,
    edge_indices: Vec<usize>,
    touches_left_rail: bool,
    touches_right_rail: bool,
}

impl WasmIecPowerComponentSummary {
    fn from_component(component: &IecPowerComponent) -> Self {
        Self {
            group_index: component.group_index,
            edge_indices: component.edge_indices.clone(),
            touches_left_rail: component.touches_left_rail,
            touches_right_rail: component.touches_right_rail,
        }
    }
}

fn source_strings(program: &LadderProgramData) -> Vec<LadderString> {
    if program.project_type == Some(2) {
        // The IEC payload has Unicode comments and symbol names. Keep the
        // ASCII-only strings used by the legacy ladder parser separate.
        crate::internal::extract_utf16_marker_strings(&program.data, false, false)
            .into_iter()
            .filter(|item| item.value.chars().all(|ch| !ch.is_control()))
            .collect()
    } else {
        program.strings.clone()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmLadderStringSummary {
    pub(super) offset: usize,
    pub(super) value: String,
    pub(super) is_iec_comment: bool,
    pub(super) is_iec_rising_contact_operand: bool,
    pub(super) iec_element_kind: Option<&'static str>,
    pub(super) iec_position: Option<[u16; 2]>,
    pub(super) iec_row_index: Option<u16>,
    pub(super) iec_group_index: Option<usize>,
    pub(super) iec_record_offset: Option<usize>,
    pub(super) iec_record_kind: Option<&'static str>,
    pub(super) is_iec_function_operand: bool,
    pub(super) is_iec_arithmetic_function: bool,
    pub(super) is_iec_comparison_function: bool,
    pub(super) is_iec_fixed_function_block: bool,
    pub(super) is_iec_function_name: bool,
    pub(super) is_iec_function_instance: bool,
}

impl WasmLadderStringSummary {
    #[allow(clippy::too_many_arguments)]
    fn from_string(
        value: &LadderString,
        is_iec_comment: bool,
        is_iec_rising_contact_operand: bool,
        iec_element: Option<(&'static str, [u16; 2])>,
        is_iec_function_operand: bool,
        iec_function_operand_position: Option<[u16; 2]>,
        is_iec_arithmetic_function: bool,
        is_iec_comparison_function: bool,
        iec_fixed_function_position: Option<[u16; 2]>,
        iec_function_name_position: Option<[u16; 2]>,
        iec_function_instance_position: Option<[u16; 2]>,
        iec_row: Option<&IecRowFrame>,
        iec_record: Option<&IecRecordFrame>,
    ) -> Self {
        Self {
            offset: value.offset,
            value: value.value.clone(),
            is_iec_comment,
            is_iec_rising_contact_operand,
            iec_element_kind: iec_element.map(|item| item.0),
            iec_position: iec_element
                .map(|item| item.1)
                .or(iec_function_operand_position)
                .or(iec_fixed_function_position)
                .or(iec_function_name_position)
                .or(iec_function_instance_position),
            iec_row_index: iec_row.map(|row| row.row_index),
            iec_group_index: iec_row.map(|row| row.group_index),
            iec_record_offset: iec_record.map(|record| record.offset),
            iec_record_kind: iec_record.map(|record| WasmIecRecordSummary::from_frame(record).kind),
            is_iec_function_operand,
            is_iec_arithmetic_function,
            is_iec_comparison_function,
            is_iec_fixed_function_block: iec_fixed_function_position.is_some(),
            is_iec_function_name: iec_function_name_position.is_some(),
            is_iec_function_instance: iec_function_instance_position.is_some(),
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
    pub(super) offset: usize,
    pub(super) raw_x: u8,
    pub(super) raw_y: u8,
    pub(super) kind: &'static str,
    pub(super) value: String,
    pub(super) operands: Vec<String>,
    pub(super) contact: Option<&'static str>,
    pub(super) coil: Option<&'static str>,
    pub(super) mnemonic_category: Option<&'static str>,
    pub(super) mnemonic_description: Option<&'static str>,
    pub(super) source_text: Option<String>,
    pub(super) instruction_text_editing: bool,
    pub(super) instruction_deletion: bool,
}

impl WasmLadderCellSummary {
    pub(super) fn from_cell(cell: &LadderCell, program: &LadderProgramData) -> Self {
        let mnemonic = ladder_mnemonic_info(&cell.value);
        Self {
            offset: cell.offset,
            raw_x: cell.raw_x,
            raw_y: cell.raw_y,
            kind: wasm_ladder_kind_label(cell.kind),
            value: cell.value.clone(),
            operands: cell.operands.clone(),
            contact: cell.contact.map(wasm_ladder_contact_label),
            coil: cell.coil.map(wasm_ladder_coil_label),
            mnemonic_category: mnemonic.map(|info| info.category.label()),
            mnemonic_description: mnemonic.map(|info| info.description),
            instruction_text_editing: {
                #[cfg(feature = "write")]
                {
                    program.version.as_deref() == Some("LD VER 1.1")
                        && program.project_type == Some(1)
                        && program
                            .strings
                            .iter()
                            .find(|s| s.offset == cell.offset)
                            .is_some_and(|s| {
                                matches!(
                                    crate::ladder_write::update_instruction_text(
                                        &program.data,
                                        cell.offset,
                                        &s.value,
                                        &s.value,
                                    ),
                                    Ok(Some(_))
                                )
                            })
                }
                #[cfg(not(feature = "write"))]
                {
                    false
                }
            },
            instruction_deletion: {
                #[cfg(feature = "write")]
                {
                    program.version.as_deref() == Some("LD VER 1.1")
                        && program.project_type == Some(1)
                        && program
                            .strings
                            .iter()
                            .find(|s| s.offset == cell.offset)
                            .is_some_and(|s| {
                                crate::ladder_write::delete_ladder_instruction(
                                    &program.data,
                                    cell.offset,
                                    &s.value,
                                )
                                .is_ok()
                            })
                }
                #[cfg(not(feature = "write"))]
                {
                    false
                }
            },
            source_text: program
                .strings
                .iter()
                .find(|string| string.offset == cell.offset)
                .map(|string| string.value.clone()),
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WasmLadderInstructionChoice {
    #[serde(flatten)]
    spec: LadderInstructionSpec,
    operand_rules: &'static [LadderOperandSpec],
}
impl From<&LadderInstructionSpec> for WasmLadderInstructionChoice {
    fn from(spec: &LadderInstructionSpec) -> Self {
        Self {
            spec: *spec,
            operand_rules: crate::ladder_instruction_operand_rules(spec.mnemonic),
        }
    }
}
