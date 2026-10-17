# Extended C edit operations

These operations extend `xgwx_document_apply_edits_json`; the existing operations in [c-api.md](c-api.md) retain their signatures. Build with `ffi,write` and additionally `il` for `edit_vendor_il`.

Every edit object includes `operation` plus the fields below. JSON names are strict snake_case. Unknown fields, including nested fields, are rejected. `Option<T>` accepts null or omission; other fields are required unless noted. `usize`, `u32`, `u16` and `u8` are unsigned integers within their native ranges. `Vec<T>` is an array; named DTOs are objects described below. Enums use the exact case shown in the source definitions.

Read `programs` with `xgwx_document_inspect_section_json` to capture `object_id`. Program edits require nonempty `expected_object_id`; copies require both source and destination identities. The extra outer identity on vendor IL and text-source patches must match their inner snapshot. Existing native offset, type, range, ownership, stale-value and unsupported-layout checks still apply. The complete transaction rolls back on any failed edit or verification. After structural edits, inspect again before using offsets.

| Operation | Fields |
| --- | --- |
| `edit_ld_label` | `program_index: usize`, `expected_object_id: String`, `raw_y: u32`, `expected: Option<String>`, `replacement: Option<String>` |
| `edit_opcua_settings` | `base: u32`, `slot: u32`, `changes: Vec<OpcUaFieldEdit>` |
| `edit_opcua_tag` | `configuration_index: usize`, `expected_tags: Vec<OpcUaTag>`, `index: Option<usize>`, `tag: Option<OpcUaTag>` |
| `edit_vendor_il` | `patch: VendorIlPatch`, `expected_object_id: String` |
| `edit_text_variable` | `expected_object_id: String`, `patch: SfcVariablePatch` |
| `edit_browser_hardware` | `patch: BrowserHardwarePatch` |
| `edit_browser_network` | `patch: BrowserNetworkPatch` |
| `edit_fenet_field` | `patch: FenetFieldPatch` |
| `select_cpu` | `model: String` |
| `set_base_slot_count` | `base: u32`, `slot_count: u32` |
| `edit_browser_iec` | `index: usize`, `patch: BrowserIecPatch`, `expected_object_id: String` |
| `rename_project` | `expected_name: String`, `name: String` |
| `generate_io_variables` | `base: Option<u32>`, `slot: Option<u32>` |
| `update_iec_local_symbol_address` | `program_index: usize`, `symbol_index: usize`, `expected_name: String`, `expected_address: String`, `replacement: String`, `expected_object_id: String` |
| `rename_iec_local_symbol` | `program_index: usize`, `symbol_index: usize`, `expected_name: String`, `replacement: String`, `expected_object_id: String` |
| `duplicate_iec_ld_function_instance` | `program_index: usize`, `block_offset: usize`, `expected_instance: String`, `new_instance: String`, `expected_object_id: String` |
| `copy_iec_instance_declaration_to_program` | `source_program_index: usize`, `source_instance: String`, `destination_program_index: usize`, `new_instance: String`, `expected_source_object_id: String`, `expected_destination_object_id: String` |
| `update_iec_local_symbol_description` | `program_index: usize`, `symbol_index: usize`, `expected_name: String`, `expected_description: String`, `replacement: String`, `expected_object_id: String` |
| `insert_iec_local_symbol` | `program_index: usize`, `name: String`, `data_type: String`, `description: String`, `expected_object_id: String` |
| `delete_iec_local_symbol` | `program_index: usize`, `symbol_index: usize`, `expected_name: String`, `expected_object_id: String` |
| `update_iec_local_symbol_type` | `program_index: usize`, `symbol_index: usize`, `expected_name: String`, `expected_type: String`, `replacement: String`, `expected_object_id: String` |
| `update_ladder_cell_text` | `program_index: usize`, `offset: usize`, `expected: String`, `replacement: String`, `expected_object_id: String` |
| `update_iec_ld_comment` | `program_index: usize`, `offset: usize`, `expected: String`, `replacement: String`, `expected_object_id: String` |
| `update_iec_ld_rising_contact_operand` | `program_index: usize`, `offset: usize`, `expected: String`, `replacement: String`, `expected_object_id: String` |
| `update_iec_ld_element_operand` | `program_index: usize`, `offset: usize`, `expected: String`, `replacement: String`, `expected_object_id: String` |
| `update_iec_ld_contact_kind` | `program_index: usize`, `offset: usize`, `expected: String`, `replacement: String`, `expected_object_id: String` |
| `update_iec_ld_coil_kind` | `program_index: usize`, `offset: usize`, `expected: String`, `replacement: String`, `expected_object_id: String` |
| `edit_iec_ld_branch_segment` | `program_index: usize`, `group_index: usize`, `start_row_index: u16`, `end_row_index: u16`, `x: u8`, `expected: bool`, `present: bool`, `expected_object_id: String` |
| `edit_iec_ld_vertical_wire` | `program_index: usize`, `group_index: usize`, `start_row_index: u16`, `end_row_index: u16`, `x: u8`, `expected: bool`, `present: bool`, `expected_object_id: String` |
| `connect_iec_ld_groups` | `program_index: usize`, `upper_group_index: usize`, `expected_upper_row: u16`, `lower_group_index: usize`, `expected_lower_row: u16`, `x: u8`, `expected_object_id: String` |
| `extend_iec_ld_vertical_wire` | `program_index: usize`, `group_index: usize`, `start_row_index: u16`, `x: u8`, `expected_object_id: String` |
| `split_iec_ld_group` | `program_index: usize`, `group_index: usize`, `expected_upper_row: u16`, `expected_lower_row: u16`, `expected_object_id: String` |
| `insert_iec_ld_blank_row` | `program_index: usize`, `after_row_index: u16`, `expected_object_id: String` |
| `insert_iec_ld_parallel_contact` | `program_index: usize`, `row_index: u16`, `expected_contact_variable: String`, `expected_coil_variable: String`, `parallel_variable: String`, `expected_object_id: String` |
| `insert_iec_ld_parallel_contact_kind` | `program_index: usize`, `row_index: u16`, `expected_contact_variable: String`, `expected_coil_variable: String`, `parallel_kind: String`, `parallel_variable: String`, `expected_object_id: String` |
| `delete_iec_ld_blank_row` | `program_index: usize`, `blank_row_index: u16`, `expected_object_id: String` |
| `insert_iec_ld_linear_rung` | `program_index: usize`, `blank_row_index: u16`, `contact_variable: String`, `coil_variable: String`, `expected_object_id: String` |
| `insert_iec_ld_rung` | `program_index: usize`, `blank_row_index: u16`, `contact_kind: String`, `contact_variable: String`, `coil_kind: String`, `coil_variable: String`, `expected_object_id: String` |
| `insert_iec_ld_single_element` | `program_index: usize`, `row_index: u16`, `raw_x: u8`, `category: String`, `kind: String`, `operand: String`, `expected_object_id: String` |
| `delete_iec_ld_linear_rung` | `program_index: usize`, `row_index: u16`, `expected_contact_variable: String`, `expected_coil_variable: String`, `expected_object_id: String` |
| `delete_iec_ld_rung` | `program_index: usize`, `row_index: u16`, `expected_contact_kind: String`, `expected_contact_variable: String`, `expected_coil_kind: String`, `expected_coil_variable: String`, `expected_object_id: String` |
| `delete_iec_ld_terminal_coil` | `program_index: usize`, `coil_record_offset: usize`, `expected_variable: String`, `expected_object_id: String` |
| `insert_iec_ld_terminal_coil` | `program_index: usize`, `contact_record_offset: usize`, `expected_contact_variable: String`, `coil_kind: String`, `coil_variable: String`, `expected_object_id: String` |
| `delete_iec_ld_simple_row` | `program_index: usize`, `row_index: u16`, `expected_contact_kind: String`, `expected_contact_variable: String`, `expected_coil_kind: String`, `expected_coil_variable: String`, `expected_object_id: String` |
| `delete_iec_ld_branch_top_row` | `program_index: usize`, `group_index: usize`, `row_index: u16`, `expected_object_id: String` |
| `delete_iec_ld_nested_contact_branch_row` | `program_index: usize`, `group_index: usize`, `row_index: u16`, `expected_object_id: String` |
| `delete_iec_ld_chained_contact_branch_row` | `program_index: usize`, `group_index: usize`, `row_index: u16`, `expected_object_id: String` |
| `delete_iec_ld_empty_branch_row` | `program_index: usize`, `group_index: usize`, `row_index: u16`, `expected_object_id: String` |
| `delete_iec_ld_ff_branch_output_row` | `program_index: usize`, `group_index: usize`, `row_index: u16`, `expected_object_id: String` |
| `insert_iec_ld_no_contact` | `program_index: usize`, `wire_offset: usize`, `raw_x: u8`, `expected_wire_start_x: u8`, `expected_wire_end_x: u8`, `variable: String`, `expected_object_id: String` |
| `insert_iec_ld_leading_contact` | `program_index: usize`, `insertion_offset: usize`, `contact_kind: String`, `variable: String`, `expected_object_id: String` |
| `insert_iec_ld_contact` | `program_index: usize`, `wire_offset: usize`, `raw_x: u8`, `expected_wire_start_x: u8`, `expected_wire_end_x: u8`, `contact_kind: String`, `variable: String`, `expected_object_id: String` |
| `insert_iec_ld_short_wire_contact` | `program_index: usize`, `wire_offset: usize`, `expected_raw_x: u8`, `contact_kind: String`, `variable: String`, `expected_object_id: String` |
| `delete_iec_ld_contact` | `program_index: usize`, `contact_offset: usize`, `expected_raw_x: u8`, `expected_contact_kind: String`, `expected_variable: String`, `expected_object_id: String` |
| `delete_iec_ld_no_contact` | `program_index: usize`, `contact_offset: usize`, `expected_raw_x: u8`, `expected_variable: String`, `expected_object_id: String` |
| `delete_iec_ld_contact_cell` | `program_index: usize`, `contact_offset: usize`, `expected_raw_x: u8`, `expected_contact_kind: String`, `expected_variable: String`, `expected_object_id: String` |
| `delete_iec_ld_no_contact_cell` | `program_index: usize`, `contact_offset: usize`, `expected_raw_x: u8`, `expected_variable: String`, `expected_object_id: String` |
| `delete_iec_ld_horizontal_wire` | `program_index: usize`, `wire_offset: usize`, `expected_raw_x: u8`, `expected_object_id: String` |
| `delete_ladder_horizontal_wire` | `program_index: usize`, `raw_y: u32`, `start_x: u8`, `end_x: u8`, `expected_object_id: String` |
| `delete_ladder_vertical_wire` | `program_index: usize`, `raw_x: u8`, `start_y: u32`, `end_y: u32`, `expected_object_id: String` |
| `delete_iec_ld_horizontal_wire_record` | `program_index: usize`, `offset: usize`, `expected_start_x: u8`, `expected_end_x: u8`, `expected_object_id: String` |
| `delete_iec_ld_isolated_element` | `program_index: usize`, `offset: usize`, `expected: String`, `expected_object_id: String` |
| `repair_iec_ld_horizontal_wire` | `program_index: usize`, `insertion_offset: usize`, `expected_raw_x: u8`, `expected_object_id: String` |
| `delete_iec_ld_function_output_operand` | `program_index: usize`, `offset: usize`, `expected: String`, `expected_object_id: String` |
| `assign_iec_ld_function_output_operand` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `ordinal: u8`, `value: String`, `expected_object_id: String` |
| `update_iec_ld_function_operand` | `program_index: usize`, `offset: usize`, `expected: String`, `replacement: String`, `expected_object_id: String` |
| `delete_iec_ld_scalar_chain_function` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `expected_object_id: String` |
| `replace_iec_ld_scalar_chain_function` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `function_name: String`, `operands: Vec<String>`, `expected_object_id: String` |
| `delete_iec_ld_terminal_function` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `expected_object_id: String` |
| `insert_iec_ld_terminal_timer` | `program_index: usize`, `contact_offset: usize`, `instance: String`, `preset: String`, `elapsed: String`, `expected_object_id: String` |
| `insert_iec_ld_terminal_move` | `program_index: usize`, `contact_offset: usize`, `input_operand: String`, `output_operand: String`, `expected_object_id: String` |
| `delete_iec_ld_standalone_function` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `expected_object_id: String` |
| `delete_iec_ld_group` | `program_index: usize`, `group_index: usize`, `expected_first_row_index: u16`, `expected_object_id: String` |
| `replace_iec_ld_group` | `program_index: usize`, `source_group_index: usize`, `expected_source_first_row_index: u16`, `destination_group_index: usize`, `expected_destination_first_row_index: u16`, `expected_object_id: String` |
| `replace_iec_ld_group_from_program` | `source_program_index: usize`, `source_group_index: usize`, `expected_source_first_row_index: u16`, `destination_program_index: usize`, `destination_group_index: usize`, `expected_destination_first_row_index: u16`, `copy_missing_locals: bool`, `expected_source_object_id: String`, `expected_destination_object_id: String` |
| `move_iec_ld_group` | `program_index: usize`, `group_index: usize`, `expected_first_row_index: u16`, `destination_first_row_index: u16`, `expected_object_id: String` |
| `copy_iec_ld_group` | `program_index: usize`, `group_index: usize`, `expected_first_row_index: u16`, `destination_first_row_index: u16`, `expected_object_id: String` |
| `copy_iec_ld_group_to_program` | `source_program_index: usize`, `group_index: usize`, `expected_first_row_index: u16`, `destination_program_index: usize`, `destination_first_row_index: u16`, `expected_source_object_id: String`, `expected_destination_object_id: String` |
| `copy_iec_ld_group_to_program_with_locals` | `source_program_index: usize`, `group_index: usize`, `expected_first_row_index: u16`, `destination_program_index: usize`, `destination_first_row_index: u16`, `expected_source_object_id: String`, `expected_destination_object_id: String` |
| `insert_iec_ld_comment` | `program_index: usize`, `destination_row_index: u16`, `text: String`, `expected_object_id: String` |
| `delete_iec_ld_branch_function` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `expected_object_id: String` |
| `replace_iec_ld_branch_function` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `function_name: String`, `operands: Vec<String>`, `expected_object_id: String` |
| `insert_iec_ld_standalone_function` | `program_index: usize`, `insertion_offset: usize`, `function_name: String`, `input_operand: String`, `output_operand: String`, `expected_object_id: String` |
| `insert_iec_ld_function_cell` | `program_index: usize`, `insertion_offset: usize`, `function_name: String`, `instance_name: String`, `expected_object_id: String` |
| `delete_iec_ld_function_cell` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `expected_object_id: String` |
| `delete_iec_ld_eq_chain_head` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `expected_object_id: String` |
| `delete_iec_ld_heating_chain_head` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `expected_object_id: String` |
| `delete_iec_ld_heating_chain_middle` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `expected_object_id: String` |
| `delete_iec_ld_heating_chain_x15_eq` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `expected_object_id: String` |
| `delete_iec_ld_heating_chain_contact_eq` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `expected_object_id: String` |
| `delete_iec_ld_heating_chain_x3_eq_repaired` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `expected_object_id: String` |
| `delete_iec_ld_heating_chain_x3_eq` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `expected_object_id: String` |
| `repair_iec_ld_heating_chain_x3_eq_deletion` | `program_index: usize`, `expected_object_id: String` |
| `delete_iec_ld_connected_arithmetic` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `expected_object_id: String` |
| `delete_iec_ld_branched_arithmetic` | `program_index: usize`, `block_offset: usize`, `expected_name: String`, `expected_object_id: String` |
| `update_iec_ld_arithmetic_function` | `program_index: usize`, `offset: usize`, `expected: String`, `replacement: String`, `expected_object_id: String` |
| `update_iec_ld_comparison_function` | `program_index: usize`, `offset: usize`, `expected: String`, `replacement: String`, `expected_object_id: String` |
| `edit_ladder_cell` | `program_index: usize`, `edit: LadderCellEdit`, `expected_object_id: String` |
| `insert_ladder_instruction` | `program_index: usize`, `raw_y: u32`, `mnemonic: String`, `operands: Vec<String>`, `expected_object_id: String` |
| `insert_ladder_comparison` | `program_index: usize`, `raw_y: u32`, `column: u8`, `mnemonic: String`, `operands: Vec<String>`, `expected_object_id: String` |
| `delete_ladder_comparison` | `program_index: usize`, `offset: usize`, `expected: String`, `expected_object_id: String` |
| `delete_ladder_instruction` | `program_index: usize`, `offset: usize`, `expected: String`, `expected_object_id: String` |
| `edit_ladder_comment` | `program_index: usize`, `edit: LadderCommentEdit`, `expected_object_id: String` |
| `delete_ladder_rung_comment` | `program_index: usize`, `raw_y: u32`, `expected: String`, `expected_object_id: String` |
| `edit_ladder_branch` | `program_index: usize`, `edit: LadderBranchEdit`, `expected_object_id: String` |
| `insert_ladder_row` | `program_index: usize`, `raw_y: u32`, `expected_object_id: String` |
| `edit_cnet_settings` | `patch: CnetSettingsPatch` |
| `edit_sfc_entity` | `patch: SfcEntityPatch`, `expected_object_id: String` |
| `replace_sfc_sequence` | `patch: SfcSequencePatch`, `expected_object_id: String` |
| `edit_sfc_variable` | `patch: SfcVariablePatch`, `expected_object_id: String` |

## Transport objects

`ModuleInputFilter` is `Default`, `Ms1`, `Ms3`, `Ms5`, `Ms10`, `Ms20`, `Ms70`, `Ms100`, or `{"Unknown": number}`. `LadderEditKind` and `LadderCommentKind` use the native variant names in `src/ffi/types.rs`. Option fields default to null unless a default below is specified.

| Object | Fields |
| --- | --- |
| `ModulePatch` | `id: Option<u32>`, `sub_type: Option<u32>`, `name: Option<String>`, `comment: Option<String>`, `details: Option<String>` |
| `ProgramPatch` | `name: Option<String>`, `task: Option<String>`, `version: Option<u32>`, `kind: Option<u32>`, `instance_name: Option<String>`, `comment: Option<String>` |
| `VariablePatch` | `name: Option<String>`, `address_area: Option<String>`, `address_number: Option<u32>`, `data_type: Option<String>`, `description: Option<String>` |
| `NetworkPatch` | `name: Option<String>`, `type_name: Option<String>`, `network_type: Option<String>` |
| `NetworkModulePatch` | `config_name: Option<String>`, `alias: Option<String>`, `description: Option<String>` |
| `BrowserNetworkPatch` | `network_index: usize`, `module: bool`, `base: Option<u32>`, `slot: Option<u32>`, `field: String`, `expected_value: String`, `replacement: String` |
| `FenetFieldPatch` | `base: u32`, `slot: u32`, `field: String`, `expected_value: String`, `replacement: String` |
| `BrowserHardwarePatch` | `operation: String`, `base: u32`, `slot: Option<u32>`, `expected_value: String`, `replacement: String`, `key: Option<String>`, `index: Option<u32>` |
| `BrowserIecPatch` | `operation: String`, `offset: usize`, `expected_kind: String`, `expected_value: String`, `replacement: String` |
| `NewProgram` | `name: String`, `language: String`, `object_id: String`, `symbol_id: String` |
| `CnetFieldEdit` | `port_index: usize`, `field: String`, `expected_value: String`, `replacement: String` |
| `CnetSettingsPatch` | `base: u32`, `slot: u32`, `changes: Vec<CnetFieldEdit>` |
| `SfcEntityPatch` | `program_index: usize`, `block_index: usize`, `entity_index: usize`, `expected_type: u32`, `expected_row: u32`, `expected_column: u32`, `field: String`, `expected_value: String`, `replacement: String` |
| `SfcSequencePatch` | `program_index: usize`, `block_index: usize`, `expected_entities: Vec<SfcEntity>`, `rows: Vec<SfcRow>`, `expected_rows: Option<Vec<SfcRow>>` |
| `SfcVariablePatch` | `program_index: usize`, `expected_variables: Vec<SfcVariable>`, `name: String`, `data_type: String`, `description: String`, `remove: bool`, `declaration: Option<SfcDeclaration>`, `update: bool` |
| `LadderCellEdit` | `raw_y: u32`, `column: u8`, `expected: Option<LadderEditElement>`, `replacement: Option<LadderEditElement>` |
| `LadderCommentEdit` | `kind: LadderCommentKind`, `raw_y: u32`, `expected: Option<String>`, `replacement: String` |
| `LadderBranchEdit` | `raw_y: u32`, `boundary: u8`, `expected: bool`, `present: bool` |
| `LadderEditElement` | `kind: LadderEditKind`, `operand: String` |
| `TextProgramPatch` | `program_index: usize`, `expected_object_id: String`, `expected_language: String`, `expected_source: String`, `source: String` |
| `VendorIlPatch` | `program_index: usize`, `expected_object_id: String`, `expected_source: String`, `source: String` |
| `OpcUaTag` | `name: String`, `data_type: String`, `address_area: String`, `address_number: u32`, `comment: String` |
| `OpcUaFieldEdit` | `field: String`, `expected_value: String`, `replacement: String` |
| `SfcEntity` | `entity_index: usize`, `type_code: Option<u32>`, `row: Option<u32>`, `column: Option<u32>`, `properties: BTreeMap<String, BTreeMap<String, String>>` |
| `SfcRow` | `kind: String`, `title: String`, `comment: String`, `initial: bool`, `action: Option<String>`, `action_qualifier: Option<String>`, `action_time: Option<String>`, `action_code: Option<String>`, `transition_code: Option<String>`, `position: Option<SfcPosition>`, `branch_end: Option<u32>` |
| `SfcPosition` | `row: u32`, `column: u32` |
| `SfcVariable` | `name: String`, `data_type: String`, `description: String`, `system: bool`, `declaration: Option<SfcDeclaration>` |
| `SfcArrayBound` | `lower: i32`, `upper: i32` |
| `SfcDeclaration` | `dimensions: Vec<SfcArrayBound>`, `initial_value: String`, `retain: bool` |

SFC `action_qualifier`, `action_time`, `action_code`, `transition_code`, `position`, `branch_end`, and `declaration` default to null; `dimensions` defaults to `[]`, `initial_value` to `""`, `retain` and `update` to false. Transport snapshots from `sfc`, `sfc.variables`, `sfc.block.<index>`, and `text.variables` can be passed directly into the matching expected lists. Collect every page first. Arrays, advanced types, unsupported CPU families, and layouts remain subject to the native writer limits.
