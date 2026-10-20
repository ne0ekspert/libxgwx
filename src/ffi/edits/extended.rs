//! Typed C adapters for the guarded native domain writers.
//! Transport DTOs are owned here; this module has no MCP runtime dependency.
use super::super::types::*;
use crate as xgwx;
use crate::XgwxDocument;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(tag = "operation", deny_unknown_fields)]
pub(super) enum Edit {
    #[serde(rename = "edit_ld_label")]
    EditLdLabel(EditLdLabelArgs),
    #[serde(rename = "edit_opcua_settings")]
    EditOpcuaSettings(EditOpcuaSettingsArgs),
    #[serde(rename = "edit_opcua_tag")]
    EditOpcuaTag(EditOpcuaTagArgs),
    #[cfg(feature = "il")]
    #[serde(rename = "edit_vendor_il")]
    EditVendorIl(EditVendorIlArgs),
    #[serde(rename = "edit_text_variable")]
    EditTextVariable(EditTextVariableArgs),
    #[serde(rename = "edit_browser_hardware")]
    EditBrowserHardware(EditBrowserHardwareArgs),
    #[serde(rename = "edit_browser_network")]
    EditBrowserNetwork(EditBrowserNetworkArgs),
    #[serde(rename = "edit_fenet_field")]
    EditFenetField(EditFenetFieldArgs),
    #[serde(rename = "select_cpu")]
    SelectCpu(SelectCpuArgs),
    #[serde(rename = "set_base_slot_count")]
    SetBaseSlotCount(SetBaseSlotCountArgs),
    #[serde(rename = "edit_browser_iec")]
    EditBrowserIec(EditBrowserIecArgs),
    #[serde(rename = "rename_project")]
    RenameProject(RenameProjectArgs),
    #[serde(rename = "generate_io_variables")]
    GenerateIoVariables(GenerateIoVariablesArgs),
    #[serde(rename = "update_iec_local_symbol_address")]
    UpdateIecLocalSymbolAddress(UpdateIecLocalSymbolAddressArgs),
    #[serde(rename = "rename_iec_local_symbol")]
    RenameIecLocalSymbol(RenameIecLocalSymbolArgs),
    #[serde(rename = "duplicate_iec_ld_function_instance")]
    DuplicateIecLdFunctionInstance(DuplicateIecLdFunctionInstanceArgs),
    #[serde(rename = "copy_iec_instance_declaration_to_program")]
    CopyIecInstanceDeclarationToProgram(CopyIecInstanceDeclarationToProgramArgs),
    #[serde(rename = "update_iec_local_symbol_description")]
    UpdateIecLocalSymbolDescription(UpdateIecLocalSymbolDescriptionArgs),
    #[serde(rename = "insert_iec_local_symbol")]
    InsertIecLocalSymbol(InsertIecLocalSymbolArgs),
    #[serde(rename = "delete_iec_local_symbol")]
    DeleteIecLocalSymbol(DeleteIecLocalSymbolArgs),
    #[serde(rename = "update_iec_local_symbol_type")]
    UpdateIecLocalSymbolType(UpdateIecLocalSymbolTypeArgs),
    #[serde(rename = "update_ladder_cell_text")]
    UpdateLadderCellText(UpdateLadderCellTextArgs),
    #[serde(rename = "update_iec_ld_comment")]
    UpdateIecLdComment(UpdateIecLdCommentArgs),
    #[serde(rename = "update_iec_ld_rising_contact_operand")]
    UpdateIecLdRisingContactOperand(UpdateIecLdRisingContactOperandArgs),
    #[serde(rename = "update_iec_ld_element_operand")]
    UpdateIecLdElementOperand(UpdateIecLdElementOperandArgs),
    #[serde(rename = "update_iec_ld_contact_kind")]
    UpdateIecLdContactKind(UpdateIecLdContactKindArgs),
    #[serde(rename = "update_iec_ld_coil_kind")]
    UpdateIecLdCoilKind(UpdateIecLdCoilKindArgs),
    #[serde(rename = "edit_iec_ld_branch_segment")]
    EditIecLdBranchSegment(EditIecLdBranchSegmentArgs),
    #[serde(rename = "edit_iec_ld_vertical_wire")]
    EditIecLdVerticalWire(EditIecLdVerticalWireArgs),
    #[serde(rename = "connect_iec_ld_groups")]
    ConnectIecLdGroups(ConnectIecLdGroupsArgs),
    #[serde(rename = "extend_iec_ld_vertical_wire")]
    ExtendIecLdVerticalWire(ExtendIecLdVerticalWireArgs),
    #[serde(rename = "split_iec_ld_group")]
    SplitIecLdGroup(SplitIecLdGroupArgs),
    #[serde(rename = "insert_iec_ld_blank_row")]
    InsertIecLdBlankRow(InsertIecLdBlankRowArgs),
    #[serde(rename = "insert_iec_ld_parallel_contact")]
    InsertIecLdParallelContact(InsertIecLdParallelContactArgs),
    #[serde(rename = "insert_iec_ld_parallel_contact_kind")]
    InsertIecLdParallelContactKind(InsertIecLdParallelContactKindArgs),
    #[serde(rename = "delete_iec_ld_blank_row")]
    DeleteIecLdBlankRow(DeleteIecLdBlankRowArgs),
    #[serde(rename = "insert_iec_ld_linear_rung")]
    InsertIecLdLinearRung(InsertIecLdLinearRungArgs),
    #[serde(rename = "insert_iec_ld_rung")]
    InsertIecLdRung(InsertIecLdRungArgs),
    #[serde(rename = "insert_iec_ld_single_element")]
    InsertIecLdSingleElement(InsertIecLdSingleElementArgs),
    #[serde(rename = "delete_iec_ld_linear_rung")]
    DeleteIecLdLinearRung(DeleteIecLdLinearRungArgs),
    #[serde(rename = "delete_iec_ld_rung")]
    DeleteIecLdRung(DeleteIecLdRungArgs),
    #[serde(rename = "delete_iec_ld_terminal_coil")]
    DeleteIecLdTerminalCoil(DeleteIecLdTerminalCoilArgs),
    #[serde(rename = "insert_iec_ld_terminal_coil")]
    InsertIecLdTerminalCoil(InsertIecLdTerminalCoilArgs),
    #[serde(rename = "delete_iec_ld_simple_row")]
    DeleteIecLdSimpleRow(DeleteIecLdSimpleRowArgs),
    #[serde(rename = "delete_iec_ld_branch_top_row")]
    DeleteIecLdBranchTopRow(DeleteIecLdBranchTopRowArgs),
    #[serde(rename = "delete_iec_ld_nested_contact_branch_row")]
    DeleteIecLdNestedContactBranchRow(DeleteIecLdNestedContactBranchRowArgs),
    #[serde(rename = "delete_iec_ld_chained_contact_branch_row")]
    DeleteIecLdChainedContactBranchRow(DeleteIecLdChainedContactBranchRowArgs),
    #[serde(rename = "delete_iec_ld_empty_branch_row")]
    DeleteIecLdEmptyBranchRow(DeleteIecLdEmptyBranchRowArgs),
    #[serde(rename = "delete_iec_ld_ff_branch_output_row")]
    DeleteIecLdFfBranchOutputRow(DeleteIecLdFfBranchOutputRowArgs),
    #[serde(rename = "insert_iec_ld_no_contact")]
    InsertIecLdNoContact(InsertIecLdNoContactArgs),
    #[serde(rename = "insert_iec_ld_leading_contact")]
    InsertIecLdLeadingContact(InsertIecLdLeadingContactArgs),
    #[serde(rename = "insert_iec_ld_contact")]
    InsertIecLdContact(InsertIecLdContactArgs),
    #[serde(rename = "insert_iec_ld_short_wire_contact")]
    InsertIecLdShortWireContact(InsertIecLdShortWireContactArgs),
    #[serde(rename = "delete_iec_ld_contact")]
    DeleteIecLdContact(DeleteIecLdContactArgs),
    #[serde(rename = "delete_iec_ld_no_contact")]
    DeleteIecLdNoContact(DeleteIecLdNoContactArgs),
    #[serde(rename = "delete_iec_ld_contact_cell")]
    DeleteIecLdContactCell(DeleteIecLdContactCellArgs),
    #[serde(rename = "delete_iec_ld_no_contact_cell")]
    DeleteIecLdNoContactCell(DeleteIecLdNoContactCellArgs),
    #[serde(rename = "delete_iec_ld_horizontal_wire")]
    DeleteIecLdHorizontalWire(DeleteIecLdHorizontalWireArgs),
    #[serde(rename = "delete_ladder_horizontal_wire")]
    DeleteLadderHorizontalWire(DeleteLadderHorizontalWireArgs),
    #[serde(rename = "delete_ladder_vertical_wire")]
    DeleteLadderVerticalWire(DeleteLadderVerticalWireArgs),
    #[serde(rename = "delete_iec_ld_horizontal_wire_record")]
    DeleteIecLdHorizontalWireRecord(DeleteIecLdHorizontalWireRecordArgs),
    #[serde(rename = "delete_iec_ld_isolated_element")]
    DeleteIecLdIsolatedElement(DeleteIecLdIsolatedElementArgs),
    #[serde(rename = "repair_iec_ld_horizontal_wire")]
    RepairIecLdHorizontalWire(RepairIecLdHorizontalWireArgs),
    #[serde(rename = "delete_iec_ld_function_output_operand")]
    DeleteIecLdFunctionOutputOperand(DeleteIecLdFunctionOutputOperandArgs),
    #[serde(rename = "assign_iec_ld_function_output_operand")]
    AssignIecLdFunctionOutputOperand(AssignIecLdFunctionOutputOperandArgs),
    #[serde(rename = "update_iec_ld_function_operand")]
    UpdateIecLdFunctionOperand(UpdateIecLdFunctionOperandArgs),
    #[serde(rename = "delete_iec_ld_scalar_chain_function")]
    DeleteIecLdScalarChainFunction(DeleteIecLdScalarChainFunctionArgs),
    #[serde(rename = "replace_iec_ld_scalar_chain_function")]
    ReplaceIecLdScalarChainFunction(ReplaceIecLdScalarChainFunctionArgs),
    #[serde(rename = "delete_iec_ld_terminal_function")]
    DeleteIecLdTerminalFunction(DeleteIecLdTerminalFunctionArgs),
    #[serde(rename = "insert_iec_ld_terminal_timer")]
    InsertIecLdTerminalTimer(InsertIecLdTerminalTimerArgs),
    #[serde(rename = "insert_iec_ld_terminal_move")]
    InsertIecLdTerminalMove(InsertIecLdTerminalMoveArgs),
    #[serde(rename = "delete_iec_ld_standalone_function")]
    DeleteIecLdStandaloneFunction(DeleteIecLdStandaloneFunctionArgs),
    #[serde(rename = "delete_iec_ld_group")]
    DeleteIecLdGroup(DeleteIecLdGroupArgs),
    #[serde(rename = "replace_iec_ld_group")]
    ReplaceIecLdGroup(ReplaceIecLdGroupArgs),
    #[serde(rename = "replace_iec_ld_group_from_program")]
    ReplaceIecLdGroupFromProgram(ReplaceIecLdGroupFromProgramArgs),
    #[serde(rename = "move_iec_ld_group")]
    MoveIecLdGroup(MoveIecLdGroupArgs),
    #[serde(rename = "copy_iec_ld_group")]
    CopyIecLdGroup(CopyIecLdGroupArgs),
    #[serde(rename = "copy_iec_ld_group_to_program")]
    CopyIecLdGroupToProgram(CopyIecLdGroupToProgramArgs),
    #[serde(rename = "copy_iec_ld_group_to_program_with_locals")]
    CopyIecLdGroupToProgramWithLocals(CopyIecLdGroupToProgramWithLocalsArgs),
    #[serde(rename = "insert_iec_ld_comment")]
    InsertIecLdComment(InsertIecLdCommentArgs),
    #[serde(rename = "delete_iec_ld_branch_function")]
    DeleteIecLdBranchFunction(DeleteIecLdBranchFunctionArgs),
    #[serde(rename = "replace_iec_ld_branch_function")]
    ReplaceIecLdBranchFunction(ReplaceIecLdBranchFunctionArgs),
    #[serde(rename = "insert_iec_ld_standalone_function")]
    InsertIecLdStandaloneFunction(InsertIecLdStandaloneFunctionArgs),
    #[serde(rename = "insert_iec_ld_function_cell")]
    InsertIecLdFunctionCell(InsertIecLdFunctionCellArgs),
    #[serde(rename = "delete_iec_ld_function_cell")]
    DeleteIecLdFunctionCell(DeleteIecLdFunctionCellArgs),
    #[serde(rename = "delete_iec_ld_eq_chain_head")]
    DeleteIecLdEqChainHead(DeleteIecLdEqChainHeadArgs),
    #[serde(rename = "delete_iec_ld_heating_chain_head")]
    DeleteIecLdHeatingChainHead(DeleteIecLdHeatingChainHeadArgs),
    #[serde(rename = "delete_iec_ld_heating_chain_middle")]
    DeleteIecLdHeatingChainMiddle(DeleteIecLdHeatingChainMiddleArgs),
    #[serde(rename = "delete_iec_ld_heating_chain_x15_eq")]
    DeleteIecLdHeatingChainX15Eq(DeleteIecLdHeatingChainX15EqArgs),
    #[serde(rename = "delete_iec_ld_heating_chain_contact_eq")]
    DeleteIecLdHeatingChainContactEq(DeleteIecLdHeatingChainContactEqArgs),
    #[serde(rename = "delete_iec_ld_heating_chain_x3_eq_repaired")]
    DeleteIecLdHeatingChainX3EqRepaired(DeleteIecLdHeatingChainX3EqRepairedArgs),
    #[serde(rename = "delete_iec_ld_heating_chain_x3_eq")]
    DeleteIecLdHeatingChainX3Eq(DeleteIecLdHeatingChainX3EqArgs),
    #[serde(rename = "repair_iec_ld_heating_chain_x3_eq_deletion")]
    RepairIecLdHeatingChainX3EqDeletion(RepairIecLdHeatingChainX3EqDeletionArgs),
    #[serde(rename = "delete_iec_ld_connected_arithmetic")]
    DeleteIecLdConnectedArithmetic(DeleteIecLdConnectedArithmeticArgs),
    #[serde(rename = "delete_iec_ld_branched_arithmetic")]
    DeleteIecLdBranchedArithmetic(DeleteIecLdBranchedArithmeticArgs),
    #[serde(rename = "update_iec_ld_arithmetic_function")]
    UpdateIecLdArithmeticFunction(UpdateIecLdArithmeticFunctionArgs),
    #[serde(rename = "update_iec_ld_comparison_function")]
    UpdateIecLdComparisonFunction(UpdateIecLdComparisonFunctionArgs),
    #[serde(rename = "edit_ladder_cell")]
    EditLadderCell(EditLadderCellArgs),
    #[serde(rename = "insert_ladder_instruction")]
    InsertLadderInstruction(InsertLadderInstructionArgs),
    #[serde(rename = "insert_ladder_comparison")]
    InsertLadderComparison(InsertLadderComparisonArgs),
    #[serde(rename = "delete_ladder_comparison")]
    DeleteLadderComparison(DeleteLadderComparisonArgs),
    #[serde(rename = "delete_ladder_instruction")]
    DeleteLadderInstruction(DeleteLadderInstructionArgs),
    #[serde(rename = "edit_ladder_comment")]
    EditLadderComment(EditLadderCommentArgs),
    #[serde(rename = "delete_ladder_rung_comment")]
    DeleteLadderRungComment(DeleteLadderRungCommentArgs),
    #[serde(rename = "edit_ladder_branch")]
    EditLadderBranch(EditLadderBranchArgs),
    #[serde(rename = "insert_ladder_row")]
    InsertLadderRow(InsertLadderRowArgs),
    #[serde(rename = "edit_cnet_settings")]
    EditCnetSettings(EditCnetSettingsArgs),
    #[serde(rename = "edit_sfc_entity")]
    EditSfcEntity(EditSfcEntityArgs),
    #[serde(rename = "replace_sfc_sequence")]
    ReplaceSfcSequence(ReplaceSfcSequenceArgs),
    #[serde(rename = "edit_sfc_variable")]
    EditSfcVariable(EditSfcVariableArgs),
}
impl Edit {
    pub(super) fn apply(self, doc: &mut XgwxDocument) -> super::super::Result<()> {
        match self {
            Self::EditLdLabel(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                doc.program_mut()
                    .get(a.program_index)
                    .and_then(|mut p| {
                        p.edit_ld_label(&crate::LadderLabelEdit {
                            raw_y: a.raw_y,
                            expected: a.expected,
                            replacement: a.replacement,
                        })
                    })
                    .map_err(super::super::operation)
            }
            Self::EditOpcuaSettings(a) => doc
                .network_mut()
                .module_at(a.base, a.slot)
                .and_then(|p| {
                    p.opcua()?
                        .edit_settings(&a.changes.into_iter().map(Into::into).collect::<Vec<_>>())
                })
                .map_err(super::super::operation),
            Self::EditOpcuaTag(a) => doc
                .network_mut()
                .edit_opcua_tag(&crate::OpcUaTagPatch {
                    configuration_index: a.configuration_index,
                    expected_tags: a.expected_tags.into_iter().map(Into::into).collect(),
                    index: a.index,
                    tag: a.tag.map(Into::into),
                })
                .map_err(super::super::operation),
            #[cfg(feature = "il")]
            Self::EditVendorIl(a) => {
                super::check_program(doc, a.patch.program_index, &a.expected_object_id)?;
                apply_edit_vendor_il(doc, a).map_err(super::super::operation)
            }
            Self::EditTextVariable(a) => {
                super::check_program(doc, a.patch.program_index, &a.expected_object_id)?;
                apply_edit_text_variable(doc, a).map_err(super::super::operation)
            }
            Self::EditBrowserHardware(a) => {
                apply_edit_browser_hardware(doc, a).map_err(super::super::operation)
            }
            Self::EditBrowserNetwork(a) => {
                apply_edit_browser_network(doc, a).map_err(super::super::operation)
            }
            Self::EditFenetField(a) => {
                apply_edit_fenet_field(doc, a).map_err(super::super::operation)
            }
            Self::SelectCpu(a) => apply_select_cpu(doc, a).map_err(super::super::operation),
            Self::SetBaseSlotCount(a) => {
                apply_set_base_slot_count(doc, a).map_err(super::super::operation)
            }
            Self::EditBrowserIec(a) => {
                super::check_program(doc, a.index, &a.expected_object_id)?;
                apply_edit_browser_iec(doc, a).map_err(super::super::operation)
            }
            Self::RenameProject(a) => apply_rename_project(doc, a).map_err(super::super::operation),
            Self::GenerateIoVariables(a) => {
                apply_generate_io_variables(doc, a).map_err(super::super::operation)
            }
            Self::UpdateIecLocalSymbolAddress(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_update_iec_local_symbol_address(doc, a).map_err(super::super::operation)
            }
            Self::RenameIecLocalSymbol(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_rename_iec_local_symbol(doc, a).map_err(super::super::operation)
            }
            Self::DuplicateIecLdFunctionInstance(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_duplicate_iec_ld_function_instance(doc, a).map_err(super::super::operation)
            }
            Self::CopyIecInstanceDeclarationToProgram(a) => {
                super::check_program(doc, a.source_program_index, &a.expected_source_object_id)?;
                super::check_program(
                    doc,
                    a.destination_program_index,
                    &a.expected_destination_object_id,
                )?;
                apply_copy_iec_instance_declaration_to_program(doc, a)
                    .map_err(super::super::operation)
            }
            Self::UpdateIecLocalSymbolDescription(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_update_iec_local_symbol_description(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLocalSymbol(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_local_symbol(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLocalSymbol(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_local_symbol(doc, a).map_err(super::super::operation)
            }
            Self::UpdateIecLocalSymbolType(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_update_iec_local_symbol_type(doc, a).map_err(super::super::operation)
            }
            Self::UpdateLadderCellText(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_update_ladder_cell_text(doc, a).map_err(super::super::operation)
            }
            Self::UpdateIecLdComment(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_update_iec_ld_comment(doc, a).map_err(super::super::operation)
            }
            Self::UpdateIecLdRisingContactOperand(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_update_iec_ld_rising_contact_operand(doc, a).map_err(super::super::operation)
            }
            Self::UpdateIecLdElementOperand(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_update_iec_ld_element_operand(doc, a).map_err(super::super::operation)
            }
            Self::UpdateIecLdContactKind(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_update_iec_ld_contact_kind(doc, a).map_err(super::super::operation)
            }
            Self::UpdateIecLdCoilKind(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_update_iec_ld_coil_kind(doc, a).map_err(super::super::operation)
            }
            Self::EditIecLdBranchSegment(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_edit_iec_ld_branch_segment(doc, a).map_err(super::super::operation)
            }
            Self::EditIecLdVerticalWire(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_edit_iec_ld_vertical_wire(doc, a).map_err(super::super::operation)
            }
            Self::ConnectIecLdGroups(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_connect_iec_ld_groups(doc, a).map_err(super::super::operation)
            }
            Self::ExtendIecLdVerticalWire(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_extend_iec_ld_vertical_wire(doc, a).map_err(super::super::operation)
            }
            Self::SplitIecLdGroup(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_split_iec_ld_group(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLdBlankRow(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_blank_row(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLdParallelContact(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_parallel_contact(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLdParallelContactKind(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_parallel_contact_kind(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdBlankRow(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_blank_row(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLdLinearRung(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_linear_rung(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLdRung(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_rung(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLdSingleElement(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_single_element(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdLinearRung(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_linear_rung(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdRung(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_rung(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdTerminalCoil(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_terminal_coil(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLdTerminalCoil(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_terminal_coil(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdSimpleRow(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_simple_row(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdBranchTopRow(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_branch_top_row(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdNestedContactBranchRow(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_nested_contact_branch_row(doc, a)
                    .map_err(super::super::operation)
            }
            Self::DeleteIecLdChainedContactBranchRow(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_chained_contact_branch_row(doc, a)
                    .map_err(super::super::operation)
            }
            Self::DeleteIecLdEmptyBranchRow(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_empty_branch_row(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdFfBranchOutputRow(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_ff_branch_output_row(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLdNoContact(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_no_contact(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLdLeadingContact(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_leading_contact(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLdContact(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_contact(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLdShortWireContact(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_short_wire_contact(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdContact(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_contact(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdNoContact(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_no_contact(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdContactCell(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_contact_cell(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdNoContactCell(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_no_contact_cell(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdHorizontalWire(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_horizontal_wire(doc, a).map_err(super::super::operation)
            }
            Self::DeleteLadderHorizontalWire(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_ladder_horizontal_wire(doc, a).map_err(super::super::operation)
            }
            Self::DeleteLadderVerticalWire(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_ladder_vertical_wire(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdHorizontalWireRecord(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_horizontal_wire_record(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdIsolatedElement(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_isolated_element(doc, a).map_err(super::super::operation)
            }
            Self::RepairIecLdHorizontalWire(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_repair_iec_ld_horizontal_wire(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdFunctionOutputOperand(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_function_output_operand(doc, a).map_err(super::super::operation)
            }
            Self::AssignIecLdFunctionOutputOperand(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_assign_iec_ld_function_output_operand(doc, a).map_err(super::super::operation)
            }
            Self::UpdateIecLdFunctionOperand(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_update_iec_ld_function_operand(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdScalarChainFunction(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_scalar_chain_function(doc, a).map_err(super::super::operation)
            }
            Self::ReplaceIecLdScalarChainFunction(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_replace_iec_ld_scalar_chain_function(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdTerminalFunction(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_terminal_function(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLdTerminalTimer(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_terminal_timer(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLdTerminalMove(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_terminal_move(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdStandaloneFunction(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_standalone_function(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdGroup(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_group(doc, a).map_err(super::super::operation)
            }
            Self::ReplaceIecLdGroup(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_replace_iec_ld_group(doc, a).map_err(super::super::operation)
            }
            Self::ReplaceIecLdGroupFromProgram(a) => {
                super::check_program(doc, a.source_program_index, &a.expected_source_object_id)?;
                super::check_program(
                    doc,
                    a.destination_program_index,
                    &a.expected_destination_object_id,
                )?;
                apply_replace_iec_ld_group_from_program(doc, a).map_err(super::super::operation)
            }
            Self::MoveIecLdGroup(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_move_iec_ld_group(doc, a).map_err(super::super::operation)
            }
            Self::CopyIecLdGroup(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_copy_iec_ld_group(doc, a).map_err(super::super::operation)
            }
            Self::CopyIecLdGroupToProgram(a) => {
                super::check_program(doc, a.source_program_index, &a.expected_source_object_id)?;
                super::check_program(
                    doc,
                    a.destination_program_index,
                    &a.expected_destination_object_id,
                )?;
                apply_copy_iec_ld_group_to_program(doc, a).map_err(super::super::operation)
            }
            Self::CopyIecLdGroupToProgramWithLocals(a) => {
                super::check_program(doc, a.source_program_index, &a.expected_source_object_id)?;
                super::check_program(
                    doc,
                    a.destination_program_index,
                    &a.expected_destination_object_id,
                )?;
                apply_copy_iec_ld_group_to_program_with_locals(doc, a)
                    .map_err(super::super::operation)
            }
            Self::InsertIecLdComment(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_comment(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdBranchFunction(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_branch_function(doc, a).map_err(super::super::operation)
            }
            Self::ReplaceIecLdBranchFunction(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_replace_iec_ld_branch_function(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLdStandaloneFunction(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_standalone_function(doc, a).map_err(super::super::operation)
            }
            Self::InsertIecLdFunctionCell(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_iec_ld_function_cell(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdFunctionCell(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_function_cell(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdEqChainHead(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_eq_chain_head(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdHeatingChainHead(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_heating_chain_head(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdHeatingChainMiddle(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_heating_chain_middle(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdHeatingChainX15Eq(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_heating_chain_x15_eq(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdHeatingChainContactEq(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_heating_chain_contact_eq(doc, a)
                    .map_err(super::super::operation)
            }
            Self::DeleteIecLdHeatingChainX3EqRepaired(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_heating_chain_x3_eq_repaired(doc, a)
                    .map_err(super::super::operation)
            }
            Self::DeleteIecLdHeatingChainX3Eq(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_heating_chain_x3_eq(doc, a).map_err(super::super::operation)
            }
            Self::RepairIecLdHeatingChainX3EqDeletion(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_repair_iec_ld_heating_chain_x3_eq_deletion(doc, a)
                    .map_err(super::super::operation)
            }
            Self::DeleteIecLdConnectedArithmetic(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_connected_arithmetic(doc, a).map_err(super::super::operation)
            }
            Self::DeleteIecLdBranchedArithmetic(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_iec_ld_branched_arithmetic(doc, a).map_err(super::super::operation)
            }
            Self::UpdateIecLdArithmeticFunction(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_update_iec_ld_arithmetic_function(doc, a).map_err(super::super::operation)
            }
            Self::UpdateIecLdComparisonFunction(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_update_iec_ld_comparison_function(doc, a).map_err(super::super::operation)
            }
            Self::EditLadderCell(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_edit_ladder_cell(doc, a).map_err(super::super::operation)
            }
            Self::InsertLadderInstruction(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_ladder_instruction(doc, a).map_err(super::super::operation)
            }
            Self::InsertLadderComparison(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_ladder_comparison(doc, a).map_err(super::super::operation)
            }
            Self::DeleteLadderComparison(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_ladder_comparison(doc, a).map_err(super::super::operation)
            }
            Self::DeleteLadderInstruction(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_ladder_instruction(doc, a).map_err(super::super::operation)
            }
            Self::EditLadderComment(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_edit_ladder_comment(doc, a).map_err(super::super::operation)
            }
            Self::DeleteLadderRungComment(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_delete_ladder_rung_comment(doc, a).map_err(super::super::operation)
            }
            Self::EditLadderBranch(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_edit_ladder_branch(doc, a).map_err(super::super::operation)
            }
            Self::InsertLadderRow(a) => {
                super::check_program(doc, a.program_index, &a.expected_object_id)?;
                apply_insert_ladder_row(doc, a).map_err(super::super::operation)
            }
            Self::EditCnetSettings(a) => {
                apply_edit_cnet_settings(doc, a).map_err(super::super::operation)
            }
            Self::EditSfcEntity(a) => {
                super::check_program(doc, a.patch.program_index, &a.expected_object_id)?;
                apply_edit_sfc_entity(doc, a).map_err(super::super::operation)
            }
            Self::ReplaceSfcSequence(a) => {
                super::check_program(doc, a.patch.program_index, &a.expected_object_id)?;
                apply_replace_sfc_sequence(doc, a).map_err(super::super::operation)
            }
            Self::EditSfcVariable(a) => {
                super::check_program(doc, a.patch.program_index, &a.expected_object_id)?;
                apply_edit_sfc_variable(doc, a).map_err(super::super::operation)
            }
        }
    }
}
#[cfg(feature = "il")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditVendorIlArgs {
    patch: VendorIlPatch,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditTextVariableArgs {
    expected_object_id: String,
    patch: SfcVariablePatch,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditBrowserHardwareArgs {
    patch: BrowserHardwarePatch,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditBrowserNetworkArgs {
    patch: BrowserNetworkPatch,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditFenetFieldArgs {
    patch: FenetFieldPatch,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SelectCpuArgs {
    model: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SetBaseSlotCountArgs {
    base: u32,
    slot_count: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditBrowserIecArgs {
    index: usize,
    patch: BrowserIecPatch,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RenameProjectArgs {
    expected_name: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GenerateIoVariablesArgs {
    base: Option<u32>,
    slot: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateIecLocalSymbolAddressArgs {
    program_index: usize,
    symbol_index: usize,
    expected_name: String,
    expected_address: String,
    replacement: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RenameIecLocalSymbolArgs {
    program_index: usize,
    symbol_index: usize,
    expected_name: String,
    replacement: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DuplicateIecLdFunctionInstanceArgs {
    program_index: usize,
    block_offset: usize,
    expected_instance: String,
    new_instance: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CopyIecInstanceDeclarationToProgramArgs {
    source_program_index: usize,
    source_instance: String,
    destination_program_index: usize,
    new_instance: String,

    expected_source_object_id: String,

    expected_destination_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateIecLocalSymbolDescriptionArgs {
    program_index: usize,
    symbol_index: usize,
    expected_name: String,
    expected_description: String,
    replacement: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLocalSymbolArgs {
    program_index: usize,
    name: String,
    data_type: String,
    description: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLocalSymbolArgs {
    program_index: usize,
    symbol_index: usize,
    expected_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateIecLocalSymbolTypeArgs {
    program_index: usize,
    symbol_index: usize,
    expected_name: String,
    expected_type: String,
    replacement: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateLadderCellTextArgs {
    program_index: usize,
    offset: usize,
    expected: String,
    replacement: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateIecLdCommentArgs {
    program_index: usize,
    offset: usize,
    expected: String,
    replacement: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateIecLdRisingContactOperandArgs {
    program_index: usize,
    offset: usize,
    expected: String,
    replacement: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateIecLdElementOperandArgs {
    program_index: usize,
    offset: usize,
    expected: String,
    replacement: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateIecLdContactKindArgs {
    program_index: usize,
    offset: usize,
    expected: String,
    replacement: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateIecLdCoilKindArgs {
    program_index: usize,
    offset: usize,
    expected: String,
    replacement: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditIecLdBranchSegmentArgs {
    program_index: usize,
    group_index: usize,
    start_row_index: u16,
    end_row_index: u16,
    x: u8,
    expected: bool,
    present: bool,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditIecLdVerticalWireArgs {
    program_index: usize,
    group_index: usize,
    start_row_index: u16,
    end_row_index: u16,
    x: u8,
    expected: bool,
    present: bool,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConnectIecLdGroupsArgs {
    program_index: usize,
    upper_group_index: usize,
    expected_upper_row: u16,
    lower_group_index: usize,
    expected_lower_row: u16,
    x: u8,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExtendIecLdVerticalWireArgs {
    program_index: usize,
    group_index: usize,
    start_row_index: u16,
    x: u8,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SplitIecLdGroupArgs {
    program_index: usize,
    group_index: usize,
    expected_upper_row: u16,
    expected_lower_row: u16,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdBlankRowArgs {
    program_index: usize,
    after_row_index: u16,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdParallelContactArgs {
    program_index: usize,
    row_index: u16,
    expected_contact_variable: String,
    expected_coil_variable: String,
    parallel_variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdParallelContactKindArgs {
    program_index: usize,
    row_index: u16,
    expected_contact_variable: String,
    expected_coil_variable: String,
    parallel_kind: String,
    parallel_variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdBlankRowArgs {
    program_index: usize,
    blank_row_index: u16,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdLinearRungArgs {
    program_index: usize,
    blank_row_index: u16,
    contact_variable: String,
    coil_variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdRungArgs {
    program_index: usize,
    blank_row_index: u16,
    contact_kind: String,
    contact_variable: String,
    coil_kind: String,
    coil_variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdSingleElementArgs {
    program_index: usize,
    row_index: u16,
    raw_x: u8,
    category: String,
    kind: String,
    operand: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdLinearRungArgs {
    program_index: usize,
    row_index: u16,
    expected_contact_variable: String,
    expected_coil_variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdRungArgs {
    program_index: usize,
    row_index: u16,
    expected_contact_kind: String,
    expected_contact_variable: String,
    expected_coil_kind: String,
    expected_coil_variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdTerminalCoilArgs {
    program_index: usize,
    coil_record_offset: usize,
    expected_variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdTerminalCoilArgs {
    program_index: usize,
    contact_record_offset: usize,
    expected_contact_variable: String,
    coil_kind: String,
    coil_variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdSimpleRowArgs {
    program_index: usize,
    row_index: u16,
    expected_contact_kind: String,
    expected_contact_variable: String,
    expected_coil_kind: String,
    expected_coil_variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdBranchTopRowArgs {
    program_index: usize,
    group_index: usize,
    row_index: u16,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdNestedContactBranchRowArgs {
    program_index: usize,
    group_index: usize,
    row_index: u16,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdChainedContactBranchRowArgs {
    program_index: usize,
    group_index: usize,
    row_index: u16,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdEmptyBranchRowArgs {
    program_index: usize,
    group_index: usize,
    row_index: u16,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdFfBranchOutputRowArgs {
    program_index: usize,
    group_index: usize,
    row_index: u16,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdNoContactArgs {
    program_index: usize,
    wire_offset: usize,
    raw_x: u8,
    expected_wire_start_x: u8,
    expected_wire_end_x: u8,
    variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdLeadingContactArgs {
    program_index: usize,
    insertion_offset: usize,
    contact_kind: String,
    variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdContactArgs {
    program_index: usize,
    wire_offset: usize,
    raw_x: u8,
    expected_wire_start_x: u8,
    expected_wire_end_x: u8,
    contact_kind: String,
    variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdShortWireContactArgs {
    program_index: usize,
    wire_offset: usize,
    expected_raw_x: u8,
    contact_kind: String,
    variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdContactArgs {
    program_index: usize,
    contact_offset: usize,
    expected_raw_x: u8,
    expected_contact_kind: String,
    expected_variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdNoContactArgs {
    program_index: usize,
    contact_offset: usize,
    expected_raw_x: u8,
    expected_variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdContactCellArgs {
    program_index: usize,
    contact_offset: usize,
    expected_raw_x: u8,
    expected_contact_kind: String,
    expected_variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdNoContactCellArgs {
    program_index: usize,
    contact_offset: usize,
    expected_raw_x: u8,
    expected_variable: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdHorizontalWireArgs {
    program_index: usize,
    wire_offset: usize,
    expected_raw_x: u8,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteLadderHorizontalWireArgs {
    program_index: usize,
    raw_y: u32,
    start_x: u8,
    end_x: u8,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteLadderVerticalWireArgs {
    program_index: usize,
    raw_x: u8,
    start_y: u32,
    end_y: u32,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdHorizontalWireRecordArgs {
    program_index: usize,
    offset: usize,
    expected_start_x: u8,
    expected_end_x: u8,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdIsolatedElementArgs {
    program_index: usize,
    offset: usize,
    expected: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RepairIecLdHorizontalWireArgs {
    program_index: usize,
    insertion_offset: usize,
    expected_raw_x: u8,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdFunctionOutputOperandArgs {
    program_index: usize,
    offset: usize,
    expected: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AssignIecLdFunctionOutputOperandArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,
    ordinal: u8,
    value: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateIecLdFunctionOperandArgs {
    program_index: usize,
    offset: usize,
    expected: String,
    replacement: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdScalarChainFunctionArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReplaceIecLdScalarChainFunctionArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,
    function_name: String,
    operands: Vec<String>,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdTerminalFunctionArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdTerminalTimerArgs {
    program_index: usize,
    contact_offset: usize,
    instance: String,
    preset: String,
    elapsed: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdTerminalMoveArgs {
    program_index: usize,
    contact_offset: usize,
    input_operand: String,
    output_operand: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdStandaloneFunctionArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdGroupArgs {
    program_index: usize,
    group_index: usize,
    expected_first_row_index: u16,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReplaceIecLdGroupArgs {
    program_index: usize,
    source_group_index: usize,
    expected_source_first_row_index: u16,
    destination_group_index: usize,
    expected_destination_first_row_index: u16,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReplaceIecLdGroupFromProgramArgs {
    source_program_index: usize,
    source_group_index: usize,
    expected_source_first_row_index: u16,
    destination_program_index: usize,
    destination_group_index: usize,
    expected_destination_first_row_index: u16,
    copy_missing_locals: bool,

    expected_source_object_id: String,

    expected_destination_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MoveIecLdGroupArgs {
    program_index: usize,
    group_index: usize,
    expected_first_row_index: u16,
    destination_first_row_index: u16,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CopyIecLdGroupArgs {
    program_index: usize,
    group_index: usize,
    expected_first_row_index: u16,
    destination_first_row_index: u16,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CopyIecLdGroupToProgramArgs {
    source_program_index: usize,
    group_index: usize,
    expected_first_row_index: u16,
    destination_program_index: usize,
    destination_first_row_index: u16,

    expected_source_object_id: String,

    expected_destination_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CopyIecLdGroupToProgramWithLocalsArgs {
    source_program_index: usize,
    group_index: usize,
    expected_first_row_index: u16,
    destination_program_index: usize,
    destination_first_row_index: u16,

    expected_source_object_id: String,

    expected_destination_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdCommentArgs {
    program_index: usize,
    destination_row_index: u16,
    text: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdBranchFunctionArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReplaceIecLdBranchFunctionArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,
    function_name: String,
    operands: Vec<String>,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdStandaloneFunctionArgs {
    program_index: usize,
    insertion_offset: usize,
    function_name: String,
    input_operand: String,
    output_operand: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertIecLdFunctionCellArgs {
    program_index: usize,
    insertion_offset: usize,
    function_name: String,
    instance_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdFunctionCellArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdEqChainHeadArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdHeatingChainHeadArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdHeatingChainMiddleArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdHeatingChainX15EqArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdHeatingChainContactEqArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdHeatingChainX3EqRepairedArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdHeatingChainX3EqArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RepairIecLdHeatingChainX3EqDeletionArgs {
    program_index: usize,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdConnectedArithmeticArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteIecLdBranchedArithmeticArgs {
    program_index: usize,
    block_offset: usize,
    expected_name: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateIecLdArithmeticFunctionArgs {
    program_index: usize,
    offset: usize,
    expected: String,
    replacement: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateIecLdComparisonFunctionArgs {
    program_index: usize,
    offset: usize,
    expected: String,
    replacement: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditLadderCellArgs {
    program_index: usize,
    edit: LadderCellEdit,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertLadderInstructionArgs {
    program_index: usize,
    raw_y: u32,
    mnemonic: String,
    operands: Vec<String>,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertLadderComparisonArgs {
    program_index: usize,
    raw_y: u32,
    column: u8,
    mnemonic: String,
    operands: Vec<String>,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteLadderComparisonArgs {
    program_index: usize,
    offset: usize,
    expected: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteLadderInstructionArgs {
    program_index: usize,
    offset: usize,
    expected: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditLadderCommentArgs {
    program_index: usize,
    edit: LadderCommentEdit,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteLadderRungCommentArgs {
    program_index: usize,
    raw_y: u32,
    expected: String,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditLadderBranchArgs {
    program_index: usize,
    edit: LadderBranchEdit,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InsertLadderRowArgs {
    program_index: usize,
    raw_y: u32,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditCnetSettingsArgs {
    patch: CnetSettingsPatch,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditSfcEntityArgs {
    patch: SfcEntityPatch,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReplaceSfcSequenceArgs {
    patch: SfcSequencePatch,

    expected_object_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditSfcVariableArgs {
    patch: SfcVariablePatch,

    expected_object_id: String,
}

#[cfg(feature = "il")]
fn apply_edit_vendor_il(doc: &mut XgwxDocument, args: EditVendorIlArgs) -> Result<(), String> {
    {
        let patch: xgwx::VendorIlPatch = args.patch.clone().into();
        doc.program_mut()
            .get(patch.program_index)
            .and_then(|mut program| {
                if program.object_id() != patch.expected_object_id
                    || patch.expected_object_id.is_empty()
                {
                    return Err(xgwx::XgwxError::TextProgramEdit(
                        "program identity changed; reload before applying".into(),
                    ));
                }
                program.vendor_il()?.update(&xgwx::VendorIlSourcePatch {
                    expected_source: patch.expected_source,
                    source: patch.source,
                })
            })
    }
    .map_err(|e| e.to_string())
}

fn apply_edit_text_variable(
    doc: &mut XgwxDocument,
    args: EditTextVariableArgs,
) -> Result<(), String> {
    {
        let patch: xgwx::SfcVariablePatch = args.patch.clone().into();
        doc.program_mut()
            .get(patch.program_index)
            .and_then(|mut program| {
                if program.object_id() != args.expected_object_id
                    || args.expected_object_id.is_empty()
                {
                    return Err(xgwx::XgwxError::TextProgramEdit(
                        "program identity changed; reload before applying".into(),
                    ));
                }
                program.view().text()?;
                program
                    .variable_mut()
                    .edit_declaration(&xgwx::ProgramVariablePatch {
                        expected_variables: patch.expected_variables,
                        name: patch.name,
                        data_type: patch.data_type,
                        description: patch.description,
                        remove: patch.remove,
                        declaration: patch.declaration,
                        update: patch.update,
                    })
            })
    }
    .map_err(|e| e.to_string())
}

fn apply_edit_browser_hardware(
    doc: &mut XgwxDocument,
    args: EditBrowserHardwareArgs,
) -> Result<(), String> {
    execute_edit_browser_hardware(doc, &args)
}

fn execute_edit_browser_hardware(
    doc: &mut XgwxDocument,
    args: &EditBrowserHardwareArgs,
) -> Result<(), String> {
    (|| {
        let patch: xgwx::BrowserHardwarePatch = args.patch.clone().into();

        if patch.operation == "slotCount" {
            doc.base_mut()
                .by_number(patch.base)?
                .edit_slot_count(&patch.expected_value, &patch.replacement)
        } else {
            let slot = patch.slot.ok_or_else(|| {
                xgwx::XgwxError::BrowserHardwareEdit("missing module slot".into())
            })?;
            doc.module_mut()
                .at(patch.base, slot)?
                .edit_field(&xgwx::ModuleFieldEdit {
                    operation: patch.operation.clone(),
                    expected_value: patch.expected_value.clone(),
                    replacement: patch.replacement.clone(),
                    key: patch.key.clone(),
                    index: patch.index,
                })
        }
    })()
    .map_err(|error| error.to_string())
}

fn apply_edit_browser_network(
    doc: &mut XgwxDocument,
    args: EditBrowserNetworkArgs,
) -> Result<(), String> {
    execute_edit_browser_network(doc, &args)
}

fn execute_edit_browser_network(
    doc: &mut XgwxDocument,
    args: &EditBrowserNetworkArgs,
) -> Result<(), String> {
    (|| {
        let patch: xgwx::BrowserNetworkPatch = args.patch.clone().into();

        let edit = xgwx::NetworkFieldEdit {
            field: patch.field.clone(),
            expected_value: patch.expected_value.clone(),
            replacement: patch.replacement.clone(),
        };
        if patch.module {
            let (Some(base), Some(slot)) = (patch.base, patch.slot) else {
                return Err(xgwx::XgwxError::BrowserNetworkEdit(
                    "missing membership coordinates".into(),
                ));
            };
            let member = doc.network().module_at(base, slot)?;
            if member.network().map(|network| network.index()) != Some(patch.network_index) {
                return Err(xgwx::XgwxError::BrowserNetworkEdit(
                    "membership belongs to another network".into(),
                ));
            }
            let index = member.index();
            doc.network_mut().module(index)?.edit_field(&edit)
        } else {
            doc.network_mut()
                .get(patch.network_index)?
                .edit_field(&edit)
        }
    })()
    .map_err(|error| error.to_string())
}

fn apply_edit_fenet_field(doc: &mut XgwxDocument, args: EditFenetFieldArgs) -> Result<(), String> {
    execute_edit_fenet_field(doc, &args)
}

fn execute_edit_fenet_field(
    doc: &mut XgwxDocument,
    args: &EditFenetFieldArgs,
) -> Result<(), String> {
    (|| {
        let patch: xgwx::FenetFieldPatch = args.patch.clone().into();

        doc.network_mut()
            .module_at(patch.base, patch.slot)?
            .fenet()?
            .edit_field(&xgwx::NetworkFieldEdit {
                field: patch.field.clone(),
                expected_value: patch.expected_value.clone(),
                replacement: patch.replacement.clone(),
            })
    })()
    .map_err(|error| error.to_string())
}

fn apply_select_cpu(doc: &mut XgwxDocument, args: SelectCpuArgs) -> Result<(), String> {
    execute_select_cpu(doc, &args)
}

fn execute_select_cpu(doc: &mut XgwxDocument, args: &SelectCpuArgs) -> Result<(), String> {
    // This legacy C operation has no explicit PLC selector.
    if doc.configuration().list().len() != 1 {
        return Err(crate::XgwxError::AmbiguousConfiguration.to_string());
    }
    doc.configuration_mut()
        .get(0)
        .and_then(|mut plc| plc.select_cpu(&args.model))
        .map_err(|error| error.to_string())
}

fn apply_set_base_slot_count(
    doc: &mut XgwxDocument,
    args: SetBaseSlotCountArgs,
) -> Result<(), String> {
    execute_set_base_slot_count(doc, &args)
}

fn execute_set_base_slot_count(
    doc: &mut XgwxDocument,
    args: &SetBaseSlotCountArgs,
) -> Result<(), String> {
    doc.base_mut()
        .by_number(args.base)
        .and_then(|mut selected_record| selected_record.set_slot_count(args.slot_count))
        .map_err(|error| error.to_string())
}

fn apply_edit_browser_iec(doc: &mut XgwxDocument, args: EditBrowserIecArgs) -> Result<(), String> {
    execute_edit_browser_iec(doc, &args)
}

fn execute_edit_browser_iec(
    doc: &mut XgwxDocument,
    args: &EditBrowserIecArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.index)
        .and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .edit_browser(&args.patch.clone().into())
        })
        .map_err(|error| error.to_string())
}

fn apply_rename_project(doc: &mut XgwxDocument, args: RenameProjectArgs) -> Result<(), String> {
    doc.project_mut()
        .rename_if(&args.expected_name, &args.name)
        .map_err(|e| e.to_string())
}

fn apply_generate_io_variables(
    doc: &mut XgwxDocument,
    args: GenerateIoVariablesArgs,
) -> Result<(), String> {
    execute_generate_io_variables(doc, &args)
}

fn execute_generate_io_variables(
    doc: &mut XgwxDocument,
    args: &GenerateIoVariablesArgs,
) -> Result<(), String> {
    doc.variable_mut()
        .generate_io_variables_for(args.base, args.slot)
        .map_err(|error| error.to_string())
}

fn apply_update_iec_local_symbol_address(
    doc: &mut XgwxDocument,
    args: UpdateIecLocalSymbolAddressArgs,
) -> Result<(), String> {
    execute_update_iec_local_symbol_address(doc, &args)
}

fn execute_update_iec_local_symbol_address(
    doc: &mut XgwxDocument,
    args: &UpdateIecLocalSymbolAddressArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.variable_mut().update_address(
                args.symbol_index,
                &args.expected_name,
                &args.expected_address,
                &args.replacement,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_rename_iec_local_symbol(
    doc: &mut XgwxDocument,
    args: RenameIecLocalSymbolArgs,
) -> Result<(), String> {
    execute_rename_iec_local_symbol(doc, &args)
}

fn execute_rename_iec_local_symbol(
    doc: &mut XgwxDocument,
    args: &RenameIecLocalSymbolArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .variable_mut()
                .rename(args.symbol_index, &args.expected_name, &args.replacement)
        })
        .map_err(|error| error.to_string())
}

fn apply_duplicate_iec_ld_function_instance(
    doc: &mut XgwxDocument,
    args: DuplicateIecLdFunctionInstanceArgs,
) -> Result<(), String> {
    execute_duplicate_iec_ld_function_instance(doc, &args)
}

fn execute_duplicate_iec_ld_function_instance(
    doc: &mut XgwxDocument,
    args: &DuplicateIecLdFunctionInstanceArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.duplicate_function_instance(
                args.block_offset,
                &args.expected_instance,
                &args.new_instance,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_copy_iec_instance_declaration_to_program(
    doc: &mut XgwxDocument,
    args: CopyIecInstanceDeclarationToProgramArgs,
) -> Result<(), String> {
    execute_copy_iec_instance_declaration_to_program(doc, &args)
}

fn execute_copy_iec_instance_declaration_to_program(
    doc: &mut XgwxDocument,
    args: &CopyIecInstanceDeclarationToProgramArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.source_program_index)
        .and_then(|mut program| {
            program.variable_mut().copy_instance_to_program(
                &args.source_instance,
                args.destination_program_index,
                &args.new_instance,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_update_iec_local_symbol_description(
    doc: &mut XgwxDocument,
    args: UpdateIecLocalSymbolDescriptionArgs,
) -> Result<(), String> {
    execute_update_iec_local_symbol_description(doc, &args)
}

fn execute_update_iec_local_symbol_description(
    doc: &mut XgwxDocument,
    args: &UpdateIecLocalSymbolDescriptionArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.variable_mut().update_description(
                args.symbol_index,
                &args.expected_name,
                &args.expected_description,
                &args.replacement,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_iec_local_symbol(
    doc: &mut XgwxDocument,
    args: InsertIecLocalSymbolArgs,
) -> Result<(), String> {
    execute_insert_iec_local_symbol(doc, &args)
}

fn execute_insert_iec_local_symbol(
    doc: &mut XgwxDocument,
    args: &InsertIecLocalSymbolArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .variable_mut()
                .insert(&args.name, &args.data_type, &args.description)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_local_symbol(
    doc: &mut XgwxDocument,
    args: DeleteIecLocalSymbolArgs,
) -> Result<(), String> {
    execute_delete_iec_local_symbol(doc, &args)
}

fn execute_delete_iec_local_symbol(
    doc: &mut XgwxDocument,
    args: &DeleteIecLocalSymbolArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .variable_mut()
                .delete(args.symbol_index, &args.expected_name)
        })
        .map_err(|error| error.to_string())
}

fn apply_update_iec_local_symbol_type(
    doc: &mut XgwxDocument,
    args: UpdateIecLocalSymbolTypeArgs,
) -> Result<(), String> {
    execute_update_iec_local_symbol_type(doc, &args)
}

fn execute_update_iec_local_symbol_type(
    doc: &mut XgwxDocument,
    args: &UpdateIecLocalSymbolTypeArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.variable_mut().update_type(
                args.symbol_index,
                &args.expected_name,
                &args.expected_type,
                &args.replacement,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_update_ladder_cell_text(
    doc: &mut XgwxDocument,
    args: UpdateLadderCellTextArgs,
) -> Result<(), String> {
    execute_update_ladder_cell_text(doc, &args)
}

fn execute_update_ladder_cell_text(
    doc: &mut XgwxDocument,
    args: &UpdateLadderCellTextArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            if program.view().ladder()?.project_type == Some(1) {
                program
                    .ladder()?
                    .update_cell_text(args.offset, &args.expected, &args.replacement)
            } else {
                program
                    .iec_ld()?
                    .update_cell_text(args.offset, &args.expected, &args.replacement)
            }
        })
        .map_err(|error| error.to_string())
}

fn apply_update_iec_ld_comment(
    doc: &mut XgwxDocument,
    args: UpdateIecLdCommentArgs,
) -> Result<(), String> {
    execute_update_iec_ld_comment(doc, &args)
}

fn execute_update_iec_ld_comment(
    doc: &mut XgwxDocument,
    args: &UpdateIecLdCommentArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .update_comment(args.offset, &args.expected, &args.replacement)
        })
        .map_err(|error| error.to_string())
}

fn apply_update_iec_ld_rising_contact_operand(
    doc: &mut XgwxDocument,
    args: UpdateIecLdRisingContactOperandArgs,
) -> Result<(), String> {
    execute_update_iec_ld_rising_contact_operand(doc, &args)
}

fn execute_update_iec_ld_rising_contact_operand(
    doc: &mut XgwxDocument,
    args: &UpdateIecLdRisingContactOperandArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.update_rising_contact_operand(
                args.offset,
                &args.expected,
                &args.replacement,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_update_iec_ld_element_operand(
    doc: &mut XgwxDocument,
    args: UpdateIecLdElementOperandArgs,
) -> Result<(), String> {
    execute_update_iec_ld_element_operand(doc, &args)
}

fn execute_update_iec_ld_element_operand(
    doc: &mut XgwxDocument,
    args: &UpdateIecLdElementOperandArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .update_element_operand(args.offset, &args.expected, &args.replacement)
        })
        .map_err(|error| error.to_string())
}

fn apply_update_iec_ld_contact_kind(
    doc: &mut XgwxDocument,
    args: UpdateIecLdContactKindArgs,
) -> Result<(), String> {
    execute_update_iec_ld_contact_kind(doc, &args)
}

fn execute_update_iec_ld_contact_kind(
    doc: &mut XgwxDocument,
    args: &UpdateIecLdContactKindArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .update_contact_kind(args.offset, &args.expected, &args.replacement)
        })
        .map_err(|error| error.to_string())
}

fn apply_update_iec_ld_coil_kind(
    doc: &mut XgwxDocument,
    args: UpdateIecLdCoilKindArgs,
) -> Result<(), String> {
    execute_update_iec_ld_coil_kind(doc, &args)
}

fn execute_update_iec_ld_coil_kind(
    doc: &mut XgwxDocument,
    args: &UpdateIecLdCoilKindArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .update_coil_kind(args.offset, &args.expected, &args.replacement)
        })
        .map_err(|error| error.to_string())
}

fn apply_edit_iec_ld_branch_segment(
    doc: &mut XgwxDocument,
    args: EditIecLdBranchSegmentArgs,
) -> Result<(), String> {
    execute_edit_iec_ld_branch_segment(doc, &args)
}

fn execute_edit_iec_ld_branch_segment(
    doc: &mut XgwxDocument,
    args: &EditIecLdBranchSegmentArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.edit_branch_segment(
                args.group_index,
                args.start_row_index,
                args.end_row_index,
                args.x,
                args.expected,
                args.present,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_edit_iec_ld_vertical_wire(
    doc: &mut XgwxDocument,
    args: EditIecLdVerticalWireArgs,
) -> Result<(), String> {
    execute_edit_iec_ld_vertical_wire(doc, &args)
}

fn execute_edit_iec_ld_vertical_wire(
    doc: &mut XgwxDocument,
    args: &EditIecLdVerticalWireArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.edit_vertical_wire(
                args.group_index,
                args.start_row_index,
                args.end_row_index,
                args.x,
                args.expected,
                args.present,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_connect_iec_ld_groups(
    doc: &mut XgwxDocument,
    args: ConnectIecLdGroupsArgs,
) -> Result<(), String> {
    execute_connect_iec_ld_groups(doc, &args)
}

fn execute_connect_iec_ld_groups(
    doc: &mut XgwxDocument,
    args: &ConnectIecLdGroupsArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.connect_groups(
                args.upper_group_index,
                args.expected_upper_row,
                args.lower_group_index,
                args.expected_lower_row,
                args.x,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_extend_iec_ld_vertical_wire(
    doc: &mut XgwxDocument,
    args: ExtendIecLdVerticalWireArgs,
) -> Result<(), String> {
    execute_extend_iec_ld_vertical_wire(doc, &args)
}

fn execute_extend_iec_ld_vertical_wire(
    doc: &mut XgwxDocument,
    args: &ExtendIecLdVerticalWireArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .extend_vertical_wire(args.group_index, args.start_row_index, args.x)
        })
        .map_err(|error| error.to_string())
}

fn apply_split_iec_ld_group(
    doc: &mut XgwxDocument,
    args: SplitIecLdGroupArgs,
) -> Result<(), String> {
    execute_split_iec_ld_group(doc, &args)
}

fn execute_split_iec_ld_group(
    doc: &mut XgwxDocument,
    args: &SplitIecLdGroupArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.split_group(
                args.group_index,
                args.expected_upper_row,
                args.expected_lower_row,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_iec_ld_blank_row(
    doc: &mut XgwxDocument,
    args: InsertIecLdBlankRowArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_blank_row(doc, &args)
}

fn execute_insert_iec_ld_blank_row(
    doc: &mut XgwxDocument,
    args: &InsertIecLdBlankRowArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .insert_blank_row(args.after_row_index)
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_iec_ld_parallel_contact(
    doc: &mut XgwxDocument,
    args: InsertIecLdParallelContactArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_parallel_contact(doc, &args)
}

fn execute_insert_iec_ld_parallel_contact(
    doc: &mut XgwxDocument,
    args: &InsertIecLdParallelContactArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_parallel_contact_kind(
        doc,
        &InsertIecLdParallelContactKindArgs {
            expected_object_id: args.expected_object_id.clone(),
            program_index: args.program_index,
            row_index: args.row_index,
            expected_contact_variable: args.expected_contact_variable.clone(),
            expected_coil_variable: args.expected_coil_variable.clone(),
            parallel_kind: "NO".into(),
            parallel_variable: args.parallel_variable.clone(),
        },
    )
}

fn apply_insert_iec_ld_parallel_contact_kind(
    doc: &mut XgwxDocument,
    args: InsertIecLdParallelContactKindArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_parallel_contact_kind(doc, &args)
}

fn execute_insert_iec_ld_parallel_contact_kind(
    doc: &mut XgwxDocument,
    args: &InsertIecLdParallelContactKindArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.insert_parallel_contact_kind(
                args.row_index,
                &args.expected_contact_variable,
                &args.expected_coil_variable,
                &args.parallel_kind,
                &args.parallel_variable,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_blank_row(
    doc: &mut XgwxDocument,
    args: DeleteIecLdBlankRowArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_blank_row(doc, &args)
}

fn execute_delete_iec_ld_blank_row(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdBlankRowArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .delete_blank_row(args.blank_row_index)
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_iec_ld_linear_rung(
    doc: &mut XgwxDocument,
    args: InsertIecLdLinearRungArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_linear_rung(doc, &args)
}

fn execute_insert_iec_ld_linear_rung(
    doc: &mut XgwxDocument,
    args: &InsertIecLdLinearRungArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_rung(
        doc,
        &InsertIecLdRungArgs {
            expected_object_id: args.expected_object_id.clone(),
            program_index: args.program_index,
            blank_row_index: args.blank_row_index,
            contact_kind: "NO".into(),
            contact_variable: args.contact_variable.clone(),
            coil_kind: "OUTPUT".into(),
            coil_variable: args.coil_variable.clone(),
        },
    )
}

fn apply_insert_iec_ld_rung(
    doc: &mut XgwxDocument,
    args: InsertIecLdRungArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_rung(doc, &args)
}

fn execute_insert_iec_ld_rung(
    doc: &mut XgwxDocument,
    args: &InsertIecLdRungArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.insert_rung(
                args.blank_row_index,
                &args.contact_kind,
                &args.contact_variable,
                &args.coil_kind,
                &args.coil_variable,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_iec_ld_single_element(
    doc: &mut XgwxDocument,
    args: InsertIecLdSingleElementArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_single_element(doc, &args)
}

fn execute_insert_iec_ld_single_element(
    doc: &mut XgwxDocument,
    args: &InsertIecLdSingleElementArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.insert_single_element(
                args.row_index,
                args.raw_x,
                &args.category,
                &args.kind,
                &args.operand,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_linear_rung(
    doc: &mut XgwxDocument,
    args: DeleteIecLdLinearRungArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_linear_rung(doc, &args)
}

fn execute_delete_iec_ld_linear_rung(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdLinearRungArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_rung(
        doc,
        &DeleteIecLdRungArgs {
            expected_object_id: args.expected_object_id.clone(),
            program_index: args.program_index,
            row_index: args.row_index,
            expected_contact_kind: "NO".into(),
            expected_contact_variable: args.expected_contact_variable.clone(),
            expected_coil_kind: "OUTPUT".into(),
            expected_coil_variable: args.expected_coil_variable.clone(),
        },
    )
}

fn apply_delete_iec_ld_rung(
    doc: &mut XgwxDocument,
    args: DeleteIecLdRungArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_rung(doc, &args)
}

fn execute_delete_iec_ld_rung(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdRungArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.delete_rung(
                args.row_index,
                &args.expected_contact_kind,
                &args.expected_contact_variable,
                &args.expected_coil_kind,
                &args.expected_coil_variable,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_terminal_coil(
    doc: &mut XgwxDocument,
    args: DeleteIecLdTerminalCoilArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_terminal_coil(doc, &args)
}

fn execute_delete_iec_ld_terminal_coil(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdTerminalCoilArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_terminal_coil(args.coil_record_offset, &args.expected_variable)
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_iec_ld_terminal_coil(
    doc: &mut XgwxDocument,
    args: InsertIecLdTerminalCoilArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_terminal_coil(doc, &args)
}

fn execute_insert_iec_ld_terminal_coil(
    doc: &mut XgwxDocument,
    args: &InsertIecLdTerminalCoilArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.insert_terminal_coil(
                args.contact_record_offset,
                &args.expected_contact_variable,
                &args.coil_kind,
                &args.coil_variable,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_simple_row(
    doc: &mut XgwxDocument,
    args: DeleteIecLdSimpleRowArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_simple_row(doc, &args)
}

fn execute_delete_iec_ld_simple_row(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdSimpleRowArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.delete_simple_row(
                args.row_index,
                &args.expected_contact_kind,
                &args.expected_contact_variable,
                &args.expected_coil_kind,
                &args.expected_coil_variable,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_branch_top_row(
    doc: &mut XgwxDocument,
    args: DeleteIecLdBranchTopRowArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_branch_top_row(doc, &args)
}

fn execute_delete_iec_ld_branch_top_row(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdBranchTopRowArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_branch_top_row(args.group_index, args.row_index)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_nested_contact_branch_row(
    doc: &mut XgwxDocument,
    args: DeleteIecLdNestedContactBranchRowArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_nested_contact_branch_row(doc, &args)
}

fn execute_delete_iec_ld_nested_contact_branch_row(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdNestedContactBranchRowArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_nested_contact_branch_row(args.group_index, args.row_index)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_chained_contact_branch_row(
    doc: &mut XgwxDocument,
    args: DeleteIecLdChainedContactBranchRowArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_chained_contact_branch_row(doc, &args)
}

fn execute_delete_iec_ld_chained_contact_branch_row(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdChainedContactBranchRowArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_chained_contact_branch_row(args.group_index, args.row_index)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_empty_branch_row(
    doc: &mut XgwxDocument,
    args: DeleteIecLdEmptyBranchRowArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_empty_branch_row(doc, &args)
}

fn execute_delete_iec_ld_empty_branch_row(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdEmptyBranchRowArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_empty_branch_row(args.group_index, args.row_index)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_ff_branch_output_row(
    doc: &mut XgwxDocument,
    args: DeleteIecLdFfBranchOutputRowArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_ff_branch_output_row(doc, &args)
}

fn execute_delete_iec_ld_ff_branch_output_row(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdFfBranchOutputRowArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_ff_branch_output_row(args.group_index, args.row_index)
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_iec_ld_no_contact(
    doc: &mut XgwxDocument,
    args: InsertIecLdNoContactArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_no_contact(doc, &args)
}

fn execute_insert_iec_ld_no_contact(
    doc: &mut XgwxDocument,
    args: &InsertIecLdNoContactArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_contact(
        doc,
        &InsertIecLdContactArgs {
            expected_object_id: args.expected_object_id.clone(),
            program_index: args.program_index,
            wire_offset: args.wire_offset,
            raw_x: args.raw_x,
            expected_wire_start_x: args.expected_wire_start_x,
            expected_wire_end_x: args.expected_wire_end_x,
            contact_kind: "NO".into(),
            variable: args.variable.clone(),
        },
    )
}

fn apply_insert_iec_ld_leading_contact(
    doc: &mut XgwxDocument,
    args: InsertIecLdLeadingContactArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_leading_contact(doc, &args)
}

fn execute_insert_iec_ld_leading_contact(
    doc: &mut XgwxDocument,
    args: &InsertIecLdLeadingContactArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.insert_leading_contact(
                args.insertion_offset,
                &args.contact_kind,
                &args.variable,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_iec_ld_contact(
    doc: &mut XgwxDocument,
    args: InsertIecLdContactArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_contact(doc, &args)
}

fn execute_insert_iec_ld_contact(
    doc: &mut XgwxDocument,
    args: &InsertIecLdContactArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.insert_contact(
                args.wire_offset,
                args.raw_x,
                args.expected_wire_start_x,
                args.expected_wire_end_x,
                &args.contact_kind,
                &args.variable,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_iec_ld_short_wire_contact(
    doc: &mut XgwxDocument,
    args: InsertIecLdShortWireContactArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_short_wire_contact(doc, &args)
}

fn execute_insert_iec_ld_short_wire_contact(
    doc: &mut XgwxDocument,
    args: &InsertIecLdShortWireContactArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.insert_short_wire_contact(
                args.wire_offset,
                args.expected_raw_x,
                &args.contact_kind,
                &args.variable,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_contact(
    doc: &mut XgwxDocument,
    args: DeleteIecLdContactArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_contact(doc, &args)
}

fn execute_delete_iec_ld_contact(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdContactArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.delete_contact(
                args.contact_offset,
                args.expected_raw_x,
                &args.expected_contact_kind,
                &args.expected_variable,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_no_contact(
    doc: &mut XgwxDocument,
    args: DeleteIecLdNoContactArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_no_contact(doc, &args)
}

fn execute_delete_iec_ld_no_contact(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdNoContactArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.delete_no_contact(
                args.contact_offset,
                args.expected_raw_x,
                &args.expected_variable,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_contact_cell(
    doc: &mut XgwxDocument,
    args: DeleteIecLdContactCellArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_contact_cell(doc, &args)
}

fn execute_delete_iec_ld_contact_cell(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdContactCellArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.delete_contact_cell(
                args.contact_offset,
                args.expected_raw_x,
                &args.expected_contact_kind,
                &args.expected_variable,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_no_contact_cell(
    doc: &mut XgwxDocument,
    args: DeleteIecLdNoContactCellArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_no_contact_cell(doc, &args)
}

fn execute_delete_iec_ld_no_contact_cell(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdNoContactCellArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.delete_no_contact_cell(
                args.contact_offset,
                args.expected_raw_x,
                &args.expected_variable,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_horizontal_wire(
    doc: &mut XgwxDocument,
    args: DeleteIecLdHorizontalWireArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_horizontal_wire(doc, &args)
}

fn execute_delete_iec_ld_horizontal_wire(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdHorizontalWireArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_horizontal_wire(args.wire_offset, args.expected_raw_x)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_ladder_horizontal_wire(
    doc: &mut XgwxDocument,
    args: DeleteLadderHorizontalWireArgs,
) -> Result<(), String> {
    execute_delete_ladder_horizontal_wire(doc, &args)
}

fn execute_delete_ladder_horizontal_wire(
    doc: &mut XgwxDocument,
    args: &DeleteLadderHorizontalWireArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .ladder()?
                .delete_horizontal_wire(args.raw_y, args.start_x, args.end_x)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_ladder_vertical_wire(
    doc: &mut XgwxDocument,
    args: DeleteLadderVerticalWireArgs,
) -> Result<(), String> {
    execute_delete_ladder_vertical_wire(doc, &args)
}

fn execute_delete_ladder_vertical_wire(
    doc: &mut XgwxDocument,
    args: &DeleteLadderVerticalWireArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .ladder()?
                .delete_vertical_wire(args.raw_x, args.start_y, args.end_y)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_horizontal_wire_record(
    doc: &mut XgwxDocument,
    args: DeleteIecLdHorizontalWireRecordArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_horizontal_wire_record(doc, &args)
}

fn execute_delete_iec_ld_horizontal_wire_record(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdHorizontalWireRecordArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.delete_horizontal_wire_record(
                args.offset,
                args.expected_start_x,
                args.expected_end_x,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_isolated_element(
    doc: &mut XgwxDocument,
    args: DeleteIecLdIsolatedElementArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_isolated_element(doc, &args)
}

fn execute_delete_iec_ld_isolated_element(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdIsolatedElementArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_isolated_element(args.offset, &args.expected)
        })
        .map_err(|error| error.to_string())
}

fn apply_repair_iec_ld_horizontal_wire(
    doc: &mut XgwxDocument,
    args: RepairIecLdHorizontalWireArgs,
) -> Result<(), String> {
    execute_repair_iec_ld_horizontal_wire(doc, &args)
}

fn execute_repair_iec_ld_horizontal_wire(
    doc: &mut XgwxDocument,
    args: &RepairIecLdHorizontalWireArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .repair_horizontal_wire(args.insertion_offset, args.expected_raw_x)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_function_output_operand(
    doc: &mut XgwxDocument,
    args: DeleteIecLdFunctionOutputOperandArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_function_output_operand(doc, &args)
}

fn execute_delete_iec_ld_function_output_operand(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdFunctionOutputOperandArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_function_output_operand(args.offset, &args.expected)
        })
        .map_err(|error| error.to_string())
}

fn apply_assign_iec_ld_function_output_operand(
    doc: &mut XgwxDocument,
    args: AssignIecLdFunctionOutputOperandArgs,
) -> Result<(), String> {
    execute_assign_iec_ld_function_output_operand(doc, &args)
}

fn execute_assign_iec_ld_function_output_operand(
    doc: &mut XgwxDocument,
    args: &AssignIecLdFunctionOutputOperandArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.assign_function_output_operand(
                args.block_offset,
                &args.expected_name,
                args.ordinal,
                &args.value,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_update_iec_ld_function_operand(
    doc: &mut XgwxDocument,
    args: UpdateIecLdFunctionOperandArgs,
) -> Result<(), String> {
    execute_update_iec_ld_function_operand(doc, &args)
}

fn execute_update_iec_ld_function_operand(
    doc: &mut XgwxDocument,
    args: &UpdateIecLdFunctionOperandArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.update_function_operand(
                args.offset,
                &args.expected,
                &args.replacement,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_scalar_chain_function(
    doc: &mut XgwxDocument,
    args: DeleteIecLdScalarChainFunctionArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_scalar_chain_function(doc, &args)
}

fn execute_delete_iec_ld_scalar_chain_function(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdScalarChainFunctionArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_scalar_chain_function(args.block_offset, &args.expected_name)
        })
        .map_err(|error| error.to_string())
}

fn apply_replace_iec_ld_scalar_chain_function(
    doc: &mut XgwxDocument,
    args: ReplaceIecLdScalarChainFunctionArgs,
) -> Result<(), String> {
    execute_replace_iec_ld_scalar_chain_function(doc, &args)
}

fn execute_replace_iec_ld_scalar_chain_function(
    doc: &mut XgwxDocument,
    args: &ReplaceIecLdScalarChainFunctionArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.replace_scalar_chain_function(
                args.block_offset,
                &args.expected_name,
                &args.function_name,
                &args.operands,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_terminal_function(
    doc: &mut XgwxDocument,
    args: DeleteIecLdTerminalFunctionArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_terminal_function(doc, &args)
}

fn execute_delete_iec_ld_terminal_function(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdTerminalFunctionArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_terminal_function(args.block_offset, &args.expected_name)
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_iec_ld_terminal_timer(
    doc: &mut XgwxDocument,
    args: InsertIecLdTerminalTimerArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_terminal_timer(doc, &args)
}

fn execute_insert_iec_ld_terminal_timer(
    doc: &mut XgwxDocument,
    args: &InsertIecLdTerminalTimerArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.insert_terminal_timer(
                args.contact_offset,
                &args.instance,
                &args.preset,
                &args.elapsed,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_iec_ld_terminal_move(
    doc: &mut XgwxDocument,
    args: InsertIecLdTerminalMoveArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_terminal_move(doc, &args)
}

fn execute_insert_iec_ld_terminal_move(
    doc: &mut XgwxDocument,
    args: &InsertIecLdTerminalMoveArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.insert_terminal_move(
                args.contact_offset,
                &args.input_operand,
                &args.output_operand,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_standalone_function(
    doc: &mut XgwxDocument,
    args: DeleteIecLdStandaloneFunctionArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_standalone_function(doc, &args)
}

fn execute_delete_iec_ld_standalone_function(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdStandaloneFunctionArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_standalone_function(args.block_offset, &args.expected_name)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_group(
    doc: &mut XgwxDocument,
    args: DeleteIecLdGroupArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_group(doc, &args)
}

fn execute_delete_iec_ld_group(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdGroupArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_group(args.group_index, args.expected_first_row_index)
        })
        .map_err(|error| error.to_string())
}

fn apply_replace_iec_ld_group(
    doc: &mut XgwxDocument,
    args: ReplaceIecLdGroupArgs,
) -> Result<(), String> {
    execute_replace_iec_ld_group(doc, &args)
}

fn execute_replace_iec_ld_group(
    doc: &mut XgwxDocument,
    args: &ReplaceIecLdGroupArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.replace_group(
                args.source_group_index,
                args.expected_source_first_row_index,
                args.destination_group_index,
                args.expected_destination_first_row_index,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_replace_iec_ld_group_from_program(
    doc: &mut XgwxDocument,
    args: ReplaceIecLdGroupFromProgramArgs,
) -> Result<(), String> {
    execute_replace_iec_ld_group_from_program(doc, &args)
}

fn execute_replace_iec_ld_group_from_program(
    doc: &mut XgwxDocument,
    args: &ReplaceIecLdGroupFromProgramArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.destination_program_index)
        .and_then(|mut program| {
            program.iec_ld()?.replace_group_from_program(
                args.source_program_index,
                args.source_group_index,
                args.expected_source_first_row_index,
                args.destination_group_index,
                args.expected_destination_first_row_index,
                args.copy_missing_locals,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_move_iec_ld_group(doc: &mut XgwxDocument, args: MoveIecLdGroupArgs) -> Result<(), String> {
    execute_move_iec_ld_group(doc, &args)
}

fn execute_move_iec_ld_group(
    doc: &mut XgwxDocument,
    args: &MoveIecLdGroupArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.move_group(
                args.group_index,
                args.expected_first_row_index,
                args.destination_first_row_index,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_copy_iec_ld_group(doc: &mut XgwxDocument, args: CopyIecLdGroupArgs) -> Result<(), String> {
    execute_copy_iec_ld_group(doc, &args)
}

fn execute_copy_iec_ld_group(
    doc: &mut XgwxDocument,
    args: &CopyIecLdGroupArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.copy_group(
                args.group_index,
                args.expected_first_row_index,
                args.destination_first_row_index,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_copy_iec_ld_group_to_program(
    doc: &mut XgwxDocument,
    args: CopyIecLdGroupToProgramArgs,
) -> Result<(), String> {
    execute_copy_iec_ld_group_to_program(doc, &args)
}

fn execute_copy_iec_ld_group_to_program(
    doc: &mut XgwxDocument,
    args: &CopyIecLdGroupToProgramArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.source_program_index)
        .and_then(|mut program| {
            program.iec_ld()?.copy_group_to_program(
                args.group_index,
                args.expected_first_row_index,
                args.destination_program_index,
                args.destination_first_row_index,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_copy_iec_ld_group_to_program_with_locals(
    doc: &mut XgwxDocument,
    args: CopyIecLdGroupToProgramWithLocalsArgs,
) -> Result<(), String> {
    execute_copy_iec_ld_group_to_program_with_locals(doc, &args)
}

fn execute_copy_iec_ld_group_to_program_with_locals(
    doc: &mut XgwxDocument,
    args: &CopyIecLdGroupToProgramWithLocalsArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.source_program_index)
        .and_then(|mut program| {
            program.iec_ld()?.copy_group_to_program_with_locals(
                args.group_index,
                args.expected_first_row_index,
                args.destination_program_index,
                args.destination_first_row_index,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_iec_ld_comment(
    doc: &mut XgwxDocument,
    args: InsertIecLdCommentArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_comment(doc, &args)
}

fn execute_insert_iec_ld_comment(
    doc: &mut XgwxDocument,
    args: &InsertIecLdCommentArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .insert_comment(args.destination_row_index, &args.text)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_branch_function(
    doc: &mut XgwxDocument,
    args: DeleteIecLdBranchFunctionArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_branch_function(doc, &args)
}

fn execute_delete_iec_ld_branch_function(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdBranchFunctionArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_branch_function(args.block_offset, &args.expected_name)
        })
        .map_err(|error| error.to_string())
}

fn apply_replace_iec_ld_branch_function(
    doc: &mut XgwxDocument,
    args: ReplaceIecLdBranchFunctionArgs,
) -> Result<(), String> {
    execute_replace_iec_ld_branch_function(doc, &args)
}

fn execute_replace_iec_ld_branch_function(
    doc: &mut XgwxDocument,
    args: &ReplaceIecLdBranchFunctionArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.replace_branch_function(
                args.block_offset,
                &args.expected_name,
                &args.function_name,
                &args.operands,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_iec_ld_standalone_function(
    doc: &mut XgwxDocument,
    args: InsertIecLdStandaloneFunctionArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_standalone_function(doc, &args)
}

fn execute_insert_iec_ld_standalone_function(
    doc: &mut XgwxDocument,
    args: &InsertIecLdStandaloneFunctionArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.insert_standalone_function(
                args.insertion_offset,
                &args.function_name,
                &args.input_operand,
                &args.output_operand,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_iec_ld_function_cell(
    doc: &mut XgwxDocument,
    args: InsertIecLdFunctionCellArgs,
) -> Result<(), String> {
    execute_insert_iec_ld_function_cell(doc, &args)
}

fn execute_insert_iec_ld_function_cell(
    doc: &mut XgwxDocument,
    args: &InsertIecLdFunctionCellArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.insert_function_cell(
                args.insertion_offset,
                &args.function_name,
                &args.instance_name,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_function_cell(
    doc: &mut XgwxDocument,
    args: DeleteIecLdFunctionCellArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_function_cell(doc, &args)
}

fn execute_delete_iec_ld_function_cell(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdFunctionCellArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_function_cell(args.block_offset, &args.expected_name)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_eq_chain_head(
    doc: &mut XgwxDocument,
    args: DeleteIecLdEqChainHeadArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_eq_chain_head(doc, &args)
}

fn execute_delete_iec_ld_eq_chain_head(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdEqChainHeadArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_eq_chain_head(args.block_offset, &args.expected_name)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_heating_chain_head(
    doc: &mut XgwxDocument,
    args: DeleteIecLdHeatingChainHeadArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_heating_chain_head(doc, &args)
}

fn execute_delete_iec_ld_heating_chain_head(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdHeatingChainHeadArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_heating_chain_head(args.block_offset, &args.expected_name)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_heating_chain_middle(
    doc: &mut XgwxDocument,
    args: DeleteIecLdHeatingChainMiddleArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_heating_chain_middle(doc, &args)
}

fn execute_delete_iec_ld_heating_chain_middle(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdHeatingChainMiddleArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_heating_chain_middle(args.block_offset, &args.expected_name)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_heating_chain_x15_eq(
    doc: &mut XgwxDocument,
    args: DeleteIecLdHeatingChainX15EqArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_heating_chain_x15_eq(doc, &args)
}

fn execute_delete_iec_ld_heating_chain_x15_eq(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdHeatingChainX15EqArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_heating_chain_x15_eq(args.block_offset, &args.expected_name)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_heating_chain_contact_eq(
    doc: &mut XgwxDocument,
    args: DeleteIecLdHeatingChainContactEqArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_heating_chain_contact_eq(doc, &args)
}

fn execute_delete_iec_ld_heating_chain_contact_eq(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdHeatingChainContactEqArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_heating_chain_contact_eq(args.block_offset, &args.expected_name)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_heating_chain_x3_eq_repaired(
    doc: &mut XgwxDocument,
    args: DeleteIecLdHeatingChainX3EqRepairedArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_heating_chain_x3_eq_repaired(doc, &args)
}

fn execute_delete_iec_ld_heating_chain_x3_eq_repaired(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdHeatingChainX3EqRepairedArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_heating_chain_x3_eq_repaired(args.block_offset, &args.expected_name)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_heating_chain_x3_eq(
    doc: &mut XgwxDocument,
    args: DeleteIecLdHeatingChainX3EqArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_heating_chain_x3_eq(doc, &args)
}

fn execute_delete_iec_ld_heating_chain_x3_eq(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdHeatingChainX3EqArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_heating_chain_x3_eq(args.block_offset, &args.expected_name)
        })
        .map_err(|error| error.to_string())
}

fn apply_repair_iec_ld_heating_chain_x3_eq_deletion(
    doc: &mut XgwxDocument,
    args: RepairIecLdHeatingChainX3EqDeletionArgs,
) -> Result<(), String> {
    execute_repair_iec_ld_heating_chain_x3_eq_deletion(doc, &args)
}

fn execute_repair_iec_ld_heating_chain_x3_eq_deletion(
    doc: &mut XgwxDocument,
    args: &RepairIecLdHeatingChainX3EqDeletionArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .repair_heating_chain_x3_eq_deletion()
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_connected_arithmetic(
    doc: &mut XgwxDocument,
    args: DeleteIecLdConnectedArithmeticArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_connected_arithmetic(doc, &args)
}

fn execute_delete_iec_ld_connected_arithmetic(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdConnectedArithmeticArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_connected_arithmetic(args.block_offset, &args.expected_name)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_iec_ld_branched_arithmetic(
    doc: &mut XgwxDocument,
    args: DeleteIecLdBranchedArithmeticArgs,
) -> Result<(), String> {
    execute_delete_iec_ld_branched_arithmetic(doc, &args)
}

fn execute_delete_iec_ld_branched_arithmetic(
    doc: &mut XgwxDocument,
    args: &DeleteIecLdBranchedArithmeticArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .iec_ld()?
                .delete_branched_arithmetic(args.block_offset, &args.expected_name)
        })
        .map_err(|error| error.to_string())
}

fn apply_update_iec_ld_arithmetic_function(
    doc: &mut XgwxDocument,
    args: UpdateIecLdArithmeticFunctionArgs,
) -> Result<(), String> {
    execute_update_iec_ld_arithmetic_function(doc, &args)
}

fn execute_update_iec_ld_arithmetic_function(
    doc: &mut XgwxDocument,
    args: &UpdateIecLdArithmeticFunctionArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.update_arithmetic_function(
                args.offset,
                &args.expected,
                &args.replacement,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_update_iec_ld_comparison_function(
    doc: &mut XgwxDocument,
    args: UpdateIecLdComparisonFunctionArgs,
) -> Result<(), String> {
    execute_update_iec_ld_comparison_function(doc, &args)
}

fn execute_update_iec_ld_comparison_function(
    doc: &mut XgwxDocument,
    args: &UpdateIecLdComparisonFunctionArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.iec_ld()?.update_comparison_function(
                args.offset,
                &args.expected,
                &args.replacement,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_edit_ladder_cell(doc: &mut XgwxDocument, args: EditLadderCellArgs) -> Result<(), String> {
    execute_edit_ladder_cell(doc, &args)
}

fn execute_edit_ladder_cell(
    doc: &mut XgwxDocument,
    args: &EditLadderCellArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut selected_program| {
            selected_program
                .ladder()?
                .edit_cell(&args.edit.clone().into())
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_ladder_instruction(
    doc: &mut XgwxDocument,
    args: InsertLadderInstructionArgs,
) -> Result<(), String> {
    execute_insert_ladder_instruction(doc, &args)
}

fn execute_insert_ladder_instruction(
    doc: &mut XgwxDocument,
    args: &InsertLadderInstructionArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .ladder()?
                .insert_instruction(args.raw_y, &args.mnemonic, &args.operands)
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_ladder_comparison(
    doc: &mut XgwxDocument,
    args: InsertLadderComparisonArgs,
) -> Result<(), String> {
    execute_insert_ladder_comparison(doc, &args)
}

fn execute_insert_ladder_comparison(
    doc: &mut XgwxDocument,
    args: &InsertLadderComparisonArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program.ladder()?.insert_comparison(
                args.raw_y,
                args.column,
                &args.mnemonic,
                &args.operands,
            )
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_ladder_comparison(
    doc: &mut XgwxDocument,
    args: DeleteLadderComparisonArgs,
) -> Result<(), String> {
    execute_delete_ladder_comparison(doc, &args)
}

fn execute_delete_ladder_comparison(
    doc: &mut XgwxDocument,
    args: &DeleteLadderComparisonArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .ladder()?
                .delete_comparison(args.offset, &args.expected)
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_ladder_instruction(
    doc: &mut XgwxDocument,
    args: DeleteLadderInstructionArgs,
) -> Result<(), String> {
    execute_delete_ladder_instruction(doc, &args)
}

fn execute_delete_ladder_instruction(
    doc: &mut XgwxDocument,
    args: &DeleteLadderInstructionArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .ladder()?
                .delete_instruction(args.offset, &args.expected)
        })
        .map_err(|error| error.to_string())
}

fn apply_edit_ladder_comment(
    doc: &mut XgwxDocument,
    args: EditLadderCommentArgs,
) -> Result<(), String> {
    execute_edit_ladder_comment(doc, &args)
}

fn execute_edit_ladder_comment(
    doc: &mut XgwxDocument,
    args: &EditLadderCommentArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut selected_program| {
            selected_program
                .ladder()?
                .edit_comment(&args.edit.clone().into())
        })
        .map_err(|error| error.to_string())
}

fn apply_delete_ladder_rung_comment(
    doc: &mut XgwxDocument,
    args: DeleteLadderRungCommentArgs,
) -> Result<(), String> {
    execute_delete_ladder_rung_comment(doc, &args)
}

fn execute_delete_ladder_rung_comment(
    doc: &mut XgwxDocument,
    args: &DeleteLadderRungCommentArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut program| {
            program
                .ladder()?
                .delete_rung_comment(args.raw_y, &args.expected)
        })
        .map_err(|error| error.to_string())
}

fn apply_edit_ladder_branch(
    doc: &mut XgwxDocument,
    args: EditLadderBranchArgs,
) -> Result<(), String> {
    execute_edit_ladder_branch(doc, &args)
}

fn execute_edit_ladder_branch(
    doc: &mut XgwxDocument,
    args: &EditLadderBranchArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut selected_program| {
            selected_program
                .ladder()?
                .edit_branch(&args.edit.clone().into())
        })
        .map_err(|error| error.to_string())
}

fn apply_insert_ladder_row(
    doc: &mut XgwxDocument,
    args: InsertLadderRowArgs,
) -> Result<(), String> {
    execute_insert_ladder_row(doc, &args)
}

fn execute_insert_ladder_row(
    doc: &mut XgwxDocument,
    args: &InsertLadderRowArgs,
) -> Result<(), String> {
    doc.program_mut()
        .get(args.program_index)
        .and_then(|mut selected_program| selected_program.ladder()?.insert_row(args.raw_y))
        .map_err(|error| error.to_string())
}

fn apply_edit_cnet_settings(
    doc: &mut XgwxDocument,
    args: EditCnetSettingsArgs,
) -> Result<(), String> {
    execute_edit_cnet_settings(doc, &args)
}

fn execute_edit_cnet_settings(
    doc: &mut XgwxDocument,
    args: &EditCnetSettingsArgs,
) -> Result<(), String> {
    (|| {
        let patch: xgwx::CnetSettingsPatch = args.patch.clone().into();

        doc.network_mut()
            .module_at(patch.base, patch.slot)?
            .cnet()?
            .edit_settings(&patch.changes)
    })()
    .map_err(|error| error.to_string())
}

fn apply_edit_sfc_entity(doc: &mut XgwxDocument, args: EditSfcEntityArgs) -> Result<(), String> {
    execute_edit_sfc_entity(doc, &args)
}

fn execute_edit_sfc_entity(doc: &mut XgwxDocument, args: &EditSfcEntityArgs) -> Result<(), String> {
    {
        let patch: xgwx::SfcEntityPatch = args.patch.clone().into();
        doc.program_mut()
            .get(patch.program_index)
            .and_then(|mut program| {
                program.sfc()?.edit_entity(&xgwx::SfcEntityEdit {
                    block_index: patch.block_index,
                    entity_index: patch.entity_index,
                    expected_type: patch.expected_type,
                    expected_row: patch.expected_row,
                    expected_column: patch.expected_column,
                    field: patch.field,
                    expected_value: patch.expected_value,
                    replacement: patch.replacement,
                })
            })
    }
    .map_err(|error| error.to_string())
}

fn apply_replace_sfc_sequence(
    doc: &mut XgwxDocument,
    args: ReplaceSfcSequenceArgs,
) -> Result<(), String> {
    execute_replace_sfc_sequence(doc, &args)
}

fn execute_replace_sfc_sequence(
    doc: &mut XgwxDocument,
    args: &ReplaceSfcSequenceArgs,
) -> Result<(), String> {
    {
        let patch: xgwx::SfcSequencePatch = args.patch.clone().into();
        doc.program_mut()
            .get(patch.program_index)
            .and_then(|mut program| {
                program.sfc()?.replace_sequence(&xgwx::SfcSequenceEdit {
                    block_index: patch.block_index,
                    expected_entities: patch.expected_entities,
                    rows: patch.rows,
                    expected_rows: patch.expected_rows,
                })
            })
    }
    .map_err(|error| error.to_string())
}

fn apply_edit_sfc_variable(
    doc: &mut XgwxDocument,
    args: EditSfcVariableArgs,
) -> Result<(), String> {
    execute_edit_sfc_variable(doc, &args)
}

fn execute_edit_sfc_variable(
    doc: &mut XgwxDocument,
    args: &EditSfcVariableArgs,
) -> Result<(), String> {
    {
        let patch: xgwx::SfcVariablePatch = args.patch.clone().into();
        doc.program_mut()
            .get(patch.program_index)
            .and_then(|mut program| {
                program
                    .variable_mut()
                    .edit_declaration(&xgwx::ProgramVariablePatch {
                        expected_variables: patch.expected_variables,
                        name: patch.name,
                        data_type: patch.data_type,
                        description: patch.description,
                        remove: patch.remove,
                        declaration: patch.declaration,
                        update: patch.update,
                    })
            })
    }
    .map_err(|error| error.to_string())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditLdLabelArgs {
    program_index: usize,
    expected_object_id: String,
    raw_y: u32,
    expected: Option<String>,
    replacement: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditOpcuaSettingsArgs {
    base: u32,
    slot: u32,
    changes: Vec<OpcUaFieldEdit>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditOpcuaTagArgs {
    configuration_index: usize,
    expected_tags: Vec<OpcUaTag>,
    index: Option<usize>,
    tag: Option<OpcUaTag>,
}
