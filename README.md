# libxgwx

Rust parser for LS XG5000 `.xgwx` workspace files.

The parser handles the container layout observed in LS XG5000 workspace files:

- `XG` binary workspace header
- gzip-compressed UTF-8 XML project payload
- optional trailing binary metadata

Unknown binary sections are preserved so callers can inspect them later.

## Usage

```rust
use xgwx::XgwxDocument;

let doc = XgwxDocument::from_path("project.xgwx")?;
let project = doc.project_info();

println!("header label: {:?}", doc.header.label);
println!("project name: {:?}", project.name);
println!("file version: {:?}", project.file_version);

for module in doc.modules() {
    println!(
        "module base={:?} slot={:?} id={:?} name={:?}",
        module.base,
        module.slot,
        module.id,
        module.name
    );
}

for program in doc.programs() {
    println!("program: {:?}", program.name);
}

# Ok::<(), Box<dyn std::error::Error>>(())
```

## API

[API docs](https://ne0ekspert.github.io/libxgwx/api/xgwx/index.html) are generated
from the Rust library with `cargo doc`. The reference includes the parser and
optional `write`, `il`, and `wasm` APIs. The crate name is `xgwx`.

Generate the same reference locally:

```sh
RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links" cargo doc --lib --no-deps --no-default-features --features write,il,wasm
```

Open `target/doc/xgwx/index.html`. Pages publishes the complete generated tree
under `/libxgwx/api/` so its relative assets and source links resolve correctly.
The API overview below complements the generated reference.

- `XgwxDocument::parse(&[u8])` parses from bytes.
- `XgwxDocument::from_path(...)` parses from a file.
- `XgwxDocument::project_info()` returns root project metadata.
- `XgwxDocument::configurations()`, `networks()`, `network_modules()`, `bases()`,
  `modules()`, `tasks()`, `programs()`, `high_speed_links()`, and
  `high_speed_link_blocks()` return owned high-level summaries.
- XGI-D24A/B hardware module summaries decode the first `Details` byte as the
  digital input filter: default (`0`) or `1`, `3`, `5`, `10`, `20`, `70`, or
  `100` milliseconds. Unknown raw values are preserved.
- `XgwxDocument::variables()` decodes the compressed global symbol table into
  variable summaries with name, formatted address, memory area, address number,
  data type, description, source reference, and range.
- `XgwxDocument::ladder_programs()` partially decodes base64/bzip2 ladder
  `ProgramData`, preserving raw bytes and extracting embedded strings and likely
  ladder elements such as instruction calls, comparisons, timers, logic
  operators, device references, constants, comments, and internal references.
- `LadderProgramData::iec_row_frames()` identifies stored IEC LD group and row
  envelopes, including two-byte Y coordinates. It leaves the records inside
  each row intact for the next decoding stage.
- `LadderProgramData::iec_geometry()` identifies captured long and short horizontal wire
  records and reciprocal vertical branch records inside those IEC rows.
- `LadderProgramData::iec_record_frames()` identifies each row-local native
  record boundary when the entire row has one unambiguous segmentation.
- `LadderProgramData::iec_function_blocks()` identifies IEC function names,
  optional instance references, stored positions, opcodes, body rows, control
  ports, data ports, reference ordinals, and IEC type masks within each framed
  function record.
- `iec_function_references()` validates block pin references against their
  native storage coordinates, visible port coordinates, directions, and pin
  counts. `iec_function_operand_links()` maps captured `FF 46` expressions to
  those typed ports.
- `iec_circuit_graph()` combines captured contacts, coils, horizontal wires,
  paired branches, function-enable cells, occupied areas, and typed function
  bindings on the native IEC grid. It rejects overlapping records, isolated
  branch endpoints, and expression cells that disagree with their decoded pin.
- `iec_no_contact_insertion_sites()` identifies long wires with room for a
  contact, including branched rows.
  `insert_iec_ld_contact(...)` splits a captured long wire around a new
  normally open, normally closed, rising-edge, falling-edge, negated
  rising-edge, or negated falling-edge contact
  whose operand resolves to BOOL or already appears in a captured contact or
  coil in that program. The latter rule permits reuse of names that XG5000
  accepts in the source file even when no declaration is decoded; new unknown
  names remain guarded. `insert_iec_ld_no_contact(...)` remains as a
  normally open compatibility wrapper. XG5000 4.82.1 opened, checked, and
  byte-preserved all decoded program payloads after normally open and normally
  closed insertions into the supplied project. A normally open insertion into
  the upper branch of program 0 L3 also passed Check Program and native Save As
  with all seven decoded program payloads preserved. Two serial insertions on
  program 0 L2, including reuse of captured `스위치_1`, passed the same native
  checks and retained all program payloads on Save As.
  `iec_no_contact_deletion_sites()`
  and `iec_no_contact_cell_deletion_sites()` now identify normally open,
  normally closed, rising-edge, and negated rising-edge contacts in their
  captured linear-row shapes. `delete_iec_ld_contact(...)` retains both wire
  fragments; `delete_iec_ld_contact_cell(...)` implements native XG5000 Cell
  Delete by closing the removed cell and shifting later contacts and wire
  coordinates left. The L3 branched contact is now an eligible Cell Delete:
  XG5000's native edit and the library output have the same row records, and
  XG5000's Save As preserved every library-generated ProgramData payload.
  The original `delete_iec_ld_no_contact*` APIs remain as
  compatibility wrappers. Contact Delete now accepts matching long-wire
  fragments in branched rows. XG5000's Delete of the inserted L3 `ON` contact
  matched the generated row records; its native Save As differed only in five
  unrelated display-cache bytes. XG5000 opened and resaved the library-generated
  deletion with all seven decoded ProgramData payloads preserved byte-for-byte.
  XG5000 4.82.1 checked and byte-preserved a generated
  rising-edge Cell Delete through Save As. `iec_horizontal_wire_repair_sites()` and
  `repair_iec_ld_horizontal_wire(...)` insert the native short-wire record that
  reconnects a one-cell gap between matching long wires, including the
  zero-span left wire on a branched row. The generated L3 branch repair matches
  XG5000 F5 byte-for-byte across all seven decoded ProgramData payloads.
  `edit_iec_ld_branch_segment(...)`
  adds or removes one vertical segment between adjacent rows in an existing IEC
  row group. It also removes the final segment from the captured two-row shape
  when the lower branch row contains contacts only, deleting that row and
  shifting all validated later row, record, function block, and pin coordinates.
  `insert_iec_ld_blank_row(...)` and `delete_iec_ld_blank_row(...)` reproduce
  XG5000 Ctrl+L and Ctrl+D at decoded IEC row boundaries: they update the row
  high-water mark and shift all later row, branch, function block, pin, and
  expression coordinates together. XG5000 opened and checked generated
  insertion and deletion files with 0 errors, 1 warning category, and 42 messages,
  then preserved all seven decoded program payloads during each Save As.
  Function pin coordinates are shifted by walking validated function record
  descriptors, including the program 1 TON and MOVE blocks. `insert_iec_ld_comment(...)`
  fills an empty IEC row with a captured standalone comment group and replaces
  its text. XG5000 opened, checked, and resaved both a program 0 comment and a
  program 1 comment placed after shifting MOVE; every decoded program and local
  symbol payload was preserved in both Save As files.
  `insert_iec_ld_single_element(...)` inserts only the requested contact or coil
  into a validated implicit blank row, including the first row after the stored
  program range. It accepts the same six contact and six coil kinds, validates
  BOOL operands and writable coil destinations, and creates no connecting wire
  or second element. Such a row can represent an incomplete circuit. XG5000
  4.82.1 rendered standalone contact and coil rows; native Save As preserved
  all seven program payloads and parsed local symbols in an all-program fixture.
  `insert_iec_ld_rung(...)` fills a validated implicit blank row with a native
  single-row group containing one of six addressed BOOL contact kinds, the
  rail-spanning horizontal wire, and one of six BOOL coil kinds. It updates the
  group count and later group ordinals while leaving later row coordinates
  fixed. It also appends a new network at the first row after the program's
  stored row range and extends that range by one. XG5000 4.82.1 checked a
  project with an appended rung in every smart home program with zero errors
  and the baseline 27 warnings; Save As preserved all seven ProgramData
  payloads and all parsed IEC local symbol fields. The reproducible generator
  is `iec_rung_acceptance append-all SOURCE OUTPUT`, followed by its `verify`
  command against the native Save As file. `delete_iec_ld_rung(...)` removes only the exact expected guarded
  shape, retaining the row range. For insertion into an existing gap, deletion
  restores the original blank-row file byte-for-byte. Removing an appended
  rung leaves its empty row available. The older `*_linear_rung` methods remain
  normally-open/output compatibility wrappers.
  `insert_iec_ld_parallel_contact(...)` adds a BOOL normally open contact below
  a guarded one-row `NO` contact, long wire, and `OUTPUT` coil group. It shifts
  later rows, creates the captured paired branch records, and validates the
  resulting IEC circuit graph. The generated branch matches XG5000's native
  record shape; four unrelated row display-cache bytes are normalized by
  XG5000 during direct editing. XG5000 opened the generated two-row branch,
  checked it with 0 errors, and preserved all seven ProgramData payloads
  byte-for-byte during Save As. Removing its final branch segment restores
  the original one-row project bytes exactly.
  `insert_iec_ld_parallel_contact_kind(...)` accepts any of the six captured
  addressed BOOL contact kinds for the lower path. Its `NC %MX760` edit on the
  supplied project's program 5 L4/L5 rung rendered in XG5000, checked with
  0 errors, and survived Save As with all seven ProgramData payloads identical.
  The other five kinds passed local writer, graph, and inverse checks on that
  rung; their branch shape has not been checked in XG5000.
  The guarded top contact now accepts all six addressed kinds as well. On the
  original project's program 0 L2 rising-edge `스위치_1` rung, the writer added
  a lower `NO ON` contact, and XG5000 rendered it and checked all programs with
  0 errors. Native Save As preserved all seven decoded ProgramData payloads.
  The branch removal restored every decoded program and local-symbol payload
  in the library.
  The same operation on the supplied project's existing program 5 L4 rung
  adds `%MX760` below `%MX761`; XG5000 rendered it, checked all programs with
  0 errors, and preserved all seven generated ProgramData payloads during Save
  As. Removing that branch restores the original decoded program and local
  symbol payloads.
  `delete_iec_ld_simple_row(...)` combines guarded deletion of a captured
  single-row contact/wire/coil group with closing its row, matching XG5000
  Ctrl+D on the supplied project's L2. The generated payload differs from the
  native Save As at eight unrelated row display-cache bytes. XG5000 opened and
  resaved the generated file without changing any decoded program payload.
  `delete_iec_ld_branch_top_row(...)` handles captured two-row groups with a
  terminal coil on top and a contact-only lower branch. It removes the upper
  line, keeps the lower contacts, and shifts later rows up, matching native
  XG5000 Ctrl+D on program 0 L3. XG5000 opened and resaved the generated file
  without changing any decoded program payload.
  The contact-deletion site catalog also recognizes a leading x1 contact
  followed by another contact at x4 in the captured two-row branch shape.
  XG5000 Delete and `delete_iec_ld_contact(...)` both remove that record and
  leave the first cell empty. XG5000 resaved the generated file without
  changing any decoded program payload.
  XG5000 Cell Delete on that x1 contact moves the next contact from x4 to x1
  and places a short wire at x4. `delete_iec_ld_contact_cell(...)` reproduces
  all seven native program payloads byte-for-byte; XG5000 opened and resaved
  the generated file without changing any decoded program payload.
  The short wire left at x4 can be replaced by an addressed BOOL contact.
  `insert_iec_ld_short_wire_contact(...)` reproduced the native F3 insertion
  byte-for-byte across all seven program payloads and survived XG5000 open
  and Save As.
  XG5000 4.82.1 rendered and checked generated representatives covering all
  twelve contact and coil codes with the project baseline diagnostics, then
  preserved all seven decoded program payloads byte-for-byte during every
  Save As.
  `iec_terminal_function_deletion_sites()` identifies captured groups whose
  top row is exactly one addressed contact, one long wire, and one terminal
  function block, with otherwise empty subordinate pin rows.
  `delete_iec_ld_terminal_function(...)` removes the wire, block, and pin rows
  while preserving the leading contact. XG5000 4.82.1 rendered and checked a
  generated first-MOVE deletion in program 3, then preserved all seven decoded
  program payloads byte-for-byte during Save As.
  `iec_terminal_function_insertion_sites()` identifies the retained first
  program 3 contact and its L2-L3 pin-row gap. `insert_iec_ld_terminal_move(...)`
  restores the native `MOVE` with an integer literal input and writable `%MW`
  output. The generated default insertion matches the native XG5000 insertion
  except for four preexisting row display-cache bytes refreshed by XG5000.
  XG5000 rendered and checked the generated insertion with 0 errors, 27
  warnings, and 42 messages; Save As preserved all seven ProgramData payloads
  and local symbol summaries. A generated `2` to `%MW301` binding also rendered
  and passed XG5000 Check Program with 0 errors, 28 warnings, and 42 messages;
  its native Save As preserved the same payloads and symbol summaries.
  `iec_standalone_function_deletion_sites()` identifies complete function-only
  row groups whose top row is a one-cell wire and function block and whose
  child rows contain only that block's operands and references.
  `delete_iec_ld_standalone_function(...)` removes the complete group, leaves
  its row range as an implicit blank gap, and decrements later group ordinals.
  XG5000 4.82.1 rendered and checked the generated `WORD_TO_UDINT` deletion in
  program 2 with the project baseline diagnostics, then preserved all seven
  decoded program payloads byte-for-byte during Save As.
  `iec_standalone_function_insertion_sites()` identifies three-row gaps at
  native group boundaries, including the deleted L1-L3 gap in program 2 and
  the original L26-L28 gap in program 4.
  `insert_iec_ld_standalone_function(...)` places the captured
  `WORD_TO_UDINT` shape with a WORD device address or local WORD input
  and a writable UDINT local output. XG5000 4.82.1 rendered both the original
  `%MW301`/`변환` binding and a generated `%MW302`/new `변환_2` binding. Check
  Program reported 0 errors, 27 warnings, and 42 messages. Native Save As
  preserved all seven decoded program payloads and the IEC local symbol
  summaries. A native program 4 L26 insertion matched the positioned group
  byte-for-byte. XG5000 rendered and checked the library-generated L26 block
  with 0 errors, 27 warnings, and 42 messages, and Save As preserved all seven
  ProgramData payloads and local symbol summaries.
  `iec_function_cell_deletion_sites()` identifies the captured connected
  single-output function-cell shape whose two rows also contain retained rung
  records. `delete_iec_ld_function_cell(...)` removes only the function block
  and its output-link record while preserving both rows and every surrounding
  contact, wire, branch, and coil record. XG5000 4.82.1 rendered the generated
  program 0 `FF` deletion at L14, reported the project baseline of 0 errors,
  27 warnings, and 42 messages, and preserved all seven decoded program
  payloads byte-for-byte during Save As.
  `iec_function_cell_insertion_sites()` identifies the exact two-row gap left
  by that deletion. `insert_iec_ld_function_cell(...)` restores the captured
  connected `FF` record and output-link record using an existing local `FF`
  instance. XG5000 4.82.1 rendered the generated insertion at L14, reported
  the same baseline diagnostics, and preserved all seven decoded program
  payloads byte-for-byte during Save As.
  `iec_connected_arithmetic_deletion_sites()` recognizes two captured five-row
  R_TRIG/arithmetic/MOVE groups: ADD at program 0 L20 and SUB at L26.
  `delete_iec_ld_connected_arithmetic(...)` removes the arithmetic block, its
  incoming wire, and six linked pin records, then splits the retained R_TRIG
  and MOVE into separate groups. Each generated payload has the same length
  and record layout as its native Delete save; six or seven display cache or
  R_TRIG fields differ. XG5000 4.82.1 opened both generated projects, rendered
  the retained circuits, and Save As preserved all seven decoded program
  payloads and local symbol summaries byte-for-byte in each case.
  `delete_iec_ld_terminal_coil(...)` removes the terminal wire and coil from
  a guarded one-row contact-wire-coil rung. XG5000's native Delete save at
  program 0 L2 has the exact generated payload. The inverse
  `insert_iec_ld_terminal_coil(...)` completes the retained contact with a
  BOOL coil; `OUTPUT 시작` restores every decoded program payload exactly. A
  clean XG5000 reopen and Save As of the generated deletion preserved all
  seven ProgramData payloads byte-for-byte. Generated `SET 시작` on the same
  retained contact also passed Check Program with 0 errors and retained all
  seven ProgramData payloads after Save As.
  `delete_iec_ld_group(...)` clears one complete decoded IEC network, leaving
  its row range as an implicit blank gap and renumbering later network ordinals.
  The real smart-home fixture passes local framing and circuit-graph validation
  after removal of every captured group. XG5000 4.82.1 rendered and checked a
  branched L3-L4 network deletion and a five-row R_TRIG/ADD/MOVE deletion at
  L20-L24 with 0 errors and baseline warnings; Save As preserved all seven
  ProgramData payloads byte-for-byte for both generated files.
  `move_iec_ld_group(...)` relocates a complete network into an empty row
  range, translating row, branch, function-pin, and link coordinates together
  and rebuilding network ordinals. The source rows become blank. On the smart
  home project, XG5000 4.82.1 accepted a one-row L6-to-L5 move and a five-row
  R_TRIG/ADD/MOVE L20-L24-to-L67-L71 move after clearing the destination
  network. Both passed Check Program with 0 errors and the baseline 27 warnings;
  native Save As preserved all seven ProgramData payloads byte-for-byte.
  `copy_iec_ld_group(...)` inserts a copy of a complete network into an empty
  row range while retaining the source. On the same project, XG5000 4.82.1
  accepted a one-row L6-to-L5 copy and a five-row R_TRIG/ADD/MOVE L20-L24-to-L67-L71
  copy after clearing the destination network. Check Program reported 0 errors
  for both. The one-row copy had the baseline 27 warning instances; the
  function-network copy had 29. XG5000 classified the extra warnings as
  duplicate output writes (`R0000`): the copied ADD writes `%MW700` and the
  copied MOVE writes `%MW200` again. Native Save As preserved all seven
  ProgramData payloads byte-for-byte. Reassign copied outputs and the reused
  `R_TRIG` instance before treating the copy as independent logic.
  `copy_iec_ld_group_to_program(...)` copies complete contact, coil, wire,
  branch, and comment networks between IEC LD programs after checking each
  operand in the destination program. On the original smart-home project,
  copying program 5 L4 to program 6 L8 passed XG5000 Check Program with 0
  errors; Save As preserved all seven generated ProgramData payloads
  byte-for-byte. Typed function networks also copy when their destination
  locals and function instances resolve with compatible types.
  `duplicate_iec_ld_function_instance(...)` clones a captured eight-byte
  automatic PB50 declaration into the next free allocation slot and rebinds
  only the selected function block. On the copied R_TRIG network, `INST4` at
  allocation 1920 passed XG5000 Check Program with 0 errors. Native Save As
  preserved all seven ProgramData and PB50 local-symbol payloads byte-for-byte.
  `update_iec_ld_function_operand(...)` then reassigned the copied ADD output
  and MOVE input to `%MW701`, and the copied MOVE output to `%MW201` while
  retaining the original network. XG5000 4.82.1 rendered those operands and
  reported 0 errors with the baseline 27 warning instances. Native Save As
  preserved all seven ProgramData and PB50 payloads byte-for-byte.
  Other IEC structural layouts remain guarded.
- `iec_local_symbols()` decodes each XGI program's `PB50` local symbol table;
  mapped BOOL addresses can be changed with
  `update_iec_local_symbol_address(...)` under area and uniqueness
  guards. The writer updates both the displayed address and its PB50 binary
  bit number, including when the address has a different digit count. XG5000
  4.82.1 displayed `%MX100` after changing `%MX8`, checked the project with
  0 errors, and retained the mapped numeric bit after Save As. That native
  save normalized program 0's PB50 table and four program display bytes.
  The same writer can clear a mapped BOOL address or assign a supported bit
  address to an unallocated BOOL. Clearing `조명.ON` from `%MX8` to empty matched
  XG5000's parsed symbol fields, including its automatic storage class and
  cleared allocation. XG5000 opened and checked the generated project with
  0 errors, 1 warning category, and 42 messages.
  `rename_iec_local_symbol(...)` updates the local name and all
  classified LD references in its program, including matching function-block
  instance markers. Primitive type IDs and allocations
  are decoded; `update_iec_local_symbol_type(...)` changes automatic primitive
  types and clears their old allocation, as XG5000 does.
- With the opt-in `il` feature, `LadderProgramData::to_il()` converts positioned
  LD structure to typed, one-step-per-line instruction list output. The related
  `to_il_with_variable_names(...)` method resolves unique exact address matches
  through the decoded symbol table.
- With the opt-in `write` feature, `XgwxDocument::update_module(...)` applies
  token-preserving in-memory changes to one module selected by base and slot.
  The first module patching scope is limited to existing `Id`, `SubType`,
  `Name`, `Comment`, and `Details` attributes;
  `set_module_input_filter(...)` provides a typed XGI-D24A/B helper.
- The same `write` feature exposes `update_program(...)` for existing program
  metadata and `update_ladder_cell_text(...)` for decoded ladder strings. A
  ladder replacement must keep the same UTF-16 length so proprietary record
  offsets and undecoded topology bytes remain unchanged.
- `update_iec_ld_comment(...)` edits captured `ProjectType=2` IEC LD comment
  records, including length changes up to 255 UTF-16 units. It rejects symbol
  and operand strings that have not been identified as comments.
- `update_iec_ld_rising_contact_operand(...)` edits the variable reference in
  captured IEC LD `FF 08` rising-edge contact records.
- `update_iec_ld_element_operand(...)` edits the variable reference in all six
  addressed contact (`FF 06` through `FF 0B`) and all six coil (`FF 0E` through
  `FF 13`) records. Known local symbols and literals must resolve to BOOL;
  direct device addresses must use a BOOL `X` area, and coils reject input
  addresses. Contact and coil insertion applies the same address checks.
  Unclassified expressions in existing elements still require XG5000 Check Program.
- `update_iec_ld_contact_kind(...)` changes among all six addressed contact
  kinds while preserving the operand and position.
  Their marker codes reuse `LadderEditKind` from XGK LD (`0x06` through
  `0x0B`); IEC row framing and BOOL operand checks remain IEC-specific.
  XGK's operandless INV/PUP/PDN operations are separate from these contacts.
  `update_iec_ld_coil_kind(...)` does the same for all six coil kinds.
- `update_iec_ld_function_operand(...)` edits captured `FF 46` function input
  and output expressions, including length changes. Classified local symbols,
  instance members, device addresses, and literals are checked against the
  decoded pin type; output pins also require a writable destination. Numeric
  expressions with `+`, `-`, `*`, `/`, unary signs, and parentheses are parsed
  from classified operands and literals, then checked against the pin type.
  Unsupported arithmetic forms are rejected; other unclassified expressions
  still require XG5000 Check Program.
  XG5000 4.82.1 opened, checked, and byte-preserved all seven decoded program
  payloads after a typed `TON.PT` edit from `T#5s` to `T#6s`.
  It also rendered and checked `MOVE.IN=0+1` on the supplied project's program
  1 with 0 errors, and its Save As preserved all seven ProgramData payloads.
- `update_iec_ld_arithmetic_function(...)` changes captured three-operand IEC
  ADD, SUB, MUL, and DIV blocks, updating both the native opcode and name.
- `update_iec_ld_comparison_function(...)` changes captured three-operand IEC
  EQ, GT, GE, LT, and LE blocks, updating the paired opcode and name.
- `update_variable(...)` edits one decoded global symbol record by document
  order. Names and descriptions may grow or shrink up to 255 UTF-16 units;
  names remain nonempty and unique without regard to case. Address area and
  data type retain their UTF-16 length; the numeric address is updated in place.
- `ladder_mnemonic_info(...)` and `known_ladder_mnemonics()` expose category
  and description metadata for known ladder instruction mnemonics.
- `edit_cnet_settings(&CnetSettingsPatch)` changes captured Cnet serial-port
  fields in one transaction at an exact Base/Slot. Each `CnetFieldEdit` identifies
  the zero-based port index, field, expected value, and replacement. Final
  station/framing/repeater constraints are checked together; opaque modem,
  Modbus mapping, and protocol payloads are preserved. The `bps` field is the
  native selector index (0–14), while `CnetPortConfigSummary::baud_rate` reports
  the actual rate. XGL-C22A/B, CH2A/B, and C42A/B insertions include captured
  two-port defaults. Other hardware layouts remain guarded.
- `XgwxDocument::project_options()`, `parameters()`, `hsc_parameters()`,
  `safety_comm()`, `trend_monitoring()`, `xgpd_config_infos()`,
  `cnet_config_infos()`, `fenet_config_infos()`, and `properties()` expose
  other high-level project sections found in the XML.
- `XgwxDocument::position_parameters()` exposes X/Y position-control axis
  parameters and step tables from `POSITION PARAMETER` sections.
- `XgwxDocument::pid_cal_parameters()` and
  `XgwxDocument::pid_tune_parameters()` expose embedded PID loop calculation and
  tuning settings.
- `XgwxDocument::cnet_config_infos()` exposes Cnet module and serial port
  settings. It preserves raw values and decodes known enums:
  `Mode` (`0` = RS232C, `1` = RS422, `2` = RS485), `DataBit`
  (`0` = 7 bits, `1` = 8 bits), `StopBit` (`0` = 1, `1` = 2), and
  `Parity` (`0` = NONE, `1` = EVEN, `2` = ODD). `Bps` is also exposed as a
  decoded baud rate using `Bps * 1200`.
- Cnet DI/DO/AI/AO device areas are decoded from ASCII numeric device codes.
  DI/DO addresses are formatted as bit addresses with LSD hex notation, while
  AI/AO addresses are formatted as decimal word addresses.
- `XgwxDocument::fenet_config_infos()` exposes FEnet module IPv4 settings such
  as IP address, subnet, gateway, and DNS.
- `XgwxDocument::hsc_parameters()` decodes XGB `HSC PARAMETER` `PAYLOAD`
  attributes, preserving raw bytes and exposing the known per-channel counter
  mode (`0` = Linear Counter, `1` = Ring Counter) and pulse input mode
  (`0` = 1-Phase 1-Input 1x, `1` = 1-Phase 2-Input 1x, `2` = CW/CCW,
  `3` = 2-Phase 4x), plus compare output mode, internal/external preset byte
  fields, ring counter maximum, compare output minimum, compare output maximum,
  unit time in milliseconds, and pulses per revolution values. Compare output
  modes map `0..=6` to Less Than, Less Or Equal, Equal, Greater Or Equal,
  Greater Than, Includes, and Excludes.
- `XgwxDocument::decoded_payloads()` returns an inventory of base64 binary
  payloads with XML path, compression flag, encoded length, raw length, decoded
  length, attributes, and decoded bytes.
- `XgwxDocument::xml` contains the inflated XML payload.
- `XgwxDocument::root` contains a lightweight owned XML tree.
- `XgwxDocument::trailer` contains raw bytes after the main XML payload.
- `XgwxDocument::trailer_gzip_members` contains any valid gzip members found in the trailer.

## XGK Module Catalog

The library contains one native Rust catalog representing the latest stable
XG5000 module definitions. [`src/catalog_data.rs`](src/catalog_data.rs) stores
the 90 selectable XGK modules, including their model, category, `Id`,
`SubType`, full display name, default `Details` payload, and verified option
encodings. Definitions also record physical slot span; for example, XGF-TC4UD
occupies two consecutive base slots. It also retains every captured visible option, including
nested per-file and per-file-data controls, so consumers can show unmapped fields as read only.
The library does not load module JSON files at build time or
runtime, and catalog entries do not carry an XG5000 version identifier.

Placement-specific base and slot values are intentionally omitted. Catalog
updates replace the Rust definitions as the supported latest-stable snapshot
rather than adding a parallel versioned catalog.

With the `write` feature, `xgk_module_catalog()` exposes this snapshot and
`XgwxDocument::select_module(base, slot, model)` atomically selects a catalog
model. Selection preserves `Base`, `Slot`, and `Comment`, while replacing
`Id`, `SubType`, `Name`, and `Details` with the captured XG5000 defaults:

```rust,no_run
use xgwx::XgwxDocument;

let mut document = XgwxDocument::from_path("project.xgwx")?;
document.select_module(0, 2, "XGF-RD8A")?;
document.write_to("project-with-rd8a.xgwx")?;
# Ok::<(), xgwx::XgwxError>(())
```

Selection validates physical placement. Multi-slot modules are rejected if
they extend past the base or overlap a module in a following slot.
`XgwxDocument::delete_module(base, slot)` removes one uniquely identified
module while retaining the surrounding base and unrelated workspace data.
`XgwxDocument::insert_module(base, slot, model)` adds a catalog-default module
to an empty physical slot and applies the same base-capacity and overlap checks.
Insertion, replacement, and deletion keep the companion network entries in
sync for captured XGK communication modules, including Cnet, FEnet,
EtherNet/IP, BACnet, FDEnet, Dnet, and Rnet. Captured XGPD defaults are also
written for `XGL-EDMT`, `XGL-EDMF`, `XGL-DMEA/B`, and `XGL-RMEA/B`.

Catalog entries expose every captured dialog row through `visible_options`.
The writable `options` subset contains only fields whose `Details` byte mapping
and numeric choices have been verified. Use
`module_option_values` to read the current selections and `set_module_option`
to update one module-wide, channel, or group value. Unknown keys, indices, and
values fail without mutating the document.

The verified high-speed-counter subset includes the dropdown settings for
`XGF-HD2A`, `XGF-HO2A`, and `XGF-HO8A`; their numeric counter, comparison, and
frequency fields remain read only until their range and encoding rules are
mapped separately.

For example:

```rust,no_run
use xgwx::XgwxDocument;

let mut document = XgwxDocument::from_path("project.xgwx")?;
document.select_module(0, 2, "XGF-AD8A")?;
document.set_module_option(0, 2, "inputRange", 5, 6)?;
document.write_to("project-with-options.xgwx")?;
# Ok::<(), xgwx::XgwxError>(())
```

## LD to IL Conversion

Enable the optional `il` feature to convert decoded ladder programs. Literal
device addresses are retained by default:

```rust,no_run
use xgwx::XgwxDocument;

let doc = XgwxDocument::from_path("fixtures/elements.xgwx")?;
for program in doc.ladder_programs() {
    let il = program?.to_il()?;
    println!("{il}");
}

# Ok::<(), Box<dyn std::error::Error>>(())
```

Run that code with `--features il`. Each `IlStep` is either a rung comment or
an instruction, and `IlProgram` renders the steps separated by newlines;
embedded comment newlines are escaped. For symbolic operands, decode
`doc.variables()` and pass them to
`to_il_with_variable_names(...)`; addresses without one unique exact match are
left unchanged, as are symbols whose address or name is duplicated. Conversion
fails when the partial ladder decoder encounters an unknown record or
disconnected/unsupported topology rather than guessing.

Print the IL generated for a workspace with the included example:

```bash
cargo run --quiet --features il --example il -- fixtures/elements.xgwx
```

## Module Writing

For controlled XG5000 open/inspect/Save As checks, see the
[acceptance procedure and results](docs/xg5000-acceptance.md). The included
`xg5000-acceptance` example generates independent writer cases and verifies
their edited sections after an external save.

Enable the opt-in `write` feature to prepare attribute changes on an existing
module. The module must be uniquely identified by its current base and slot:

```rust,no_run
use xgwx::{ModuleInputFilter, ModulePatch, XgwxDocument};

let mut doc = XgwxDocument::from_path("project.xgwx")?;
doc.update_module(
    0,
    2,
    &ModulePatch {
        comment: Some("Updated from an external editor".to_owned()),
        ..ModulePatch::default()
    },
)?;
doc.set_module_input_filter(0, 2, ModuleInputFilter::Ms5)?;

# Ok::<(), Box<dyn std::error::Error>>(())
```

The patcher replaces only requested XML attribute values and preserves unknown
XML in memory. `to_bytes()` emits byte-identical data for an unchanged document.
After an edit it recompresses the main XML, applies XG5000's four-byte alignment,
updates the additive container checksum, validates the nested Security CRC64
frames, and preserves the existing Security metadata byte-for-byte. Layouts
outside the validated 138-byte XG5000 header format still fail closed with
`AuthenticatedRewriteUnsupported`.

When both `wasm` and `write` are enabled, the generated package also exposes
`update_xgwx_module(bytes, base, slot, patch)` and
`set_xgwx_module_input_filter(bytes, base, slot, rawFilter)`, plus
`update_xgwx_program(bytes, programIndex, patch)` and
`update_xgwx_ladder_cell(bytes, programIndex, offset, expected, replacement)`,
`update_xgwx_iec_ld_comment(bytes, programIndex, offset, expected, replacement)`,
`update_xgwx_iec_ld_rising_contact_operand(bytes, programIndex, offset, expected, replacement)`,
`update_xgwx_iec_ld_element_operand(bytes, programIndex, offset, expected, replacement)`,
`update_xgwx_iec_ld_contact_kind(bytes, programIndex, offset, expectedKind, replacementKind)`,
`update_xgwx_iec_ld_coil_kind(bytes, programIndex, offset, expectedKind, replacementKind)`,
`insert_xgwx_iec_ld_blank_row(bytes, programIndex, afterRowIndex)`,
`delete_xgwx_iec_ld_blank_row(bytes, programIndex, blankRowIndex)`,
`insert_xgwx_iec_ld_rung(bytes, programIndex, row, contactKind, contact, coilKind, coil)`,
`delete_xgwx_iec_ld_rung(bytes, programIndex, row, expectedContactKind, expectedContact, expectedCoilKind, expectedCoil)`,
`delete_xgwx_iec_ld_terminal_function(bytes, programIndex, blockOffset, expectedName)`,
`delete_xgwx_iec_ld_standalone_function(bytes, programIndex, blockOffset, expectedName)`,
`insert_xgwx_iec_ld_standalone_function(bytes, programIndex, insertionOffset, functionName, inputOperand, outputOperand)`,
`delete_xgwx_iec_ld_function_cell(bytes, programIndex, blockOffset, expectedName)`,
`delete_xgwx_iec_ld_connected_arithmetic(bytes, programIndex, blockOffset, expectedName)`,
`insert_xgwx_iec_ld_function_cell(bytes, programIndex, insertionOffset, functionName, instanceName)`,
`update_xgwx_iec_ld_function_operand(bytes, programIndex, offset, expected, replacement)`,
`update_xgwx_iec_ld_arithmetic_function(bytes, programIndex, offset, expected, replacement)`,
`update_xgwx_iec_ld_comparison_function(bytes, programIndex, offset, expected, replacement)`,
and `update_xgwx_variable(bytes, variableIndex, patch)`.
Network metadata is available through `update_xgwx_network(bytes, networkIndex,
patch)` and `update_xgwx_network_module(bytes, base, slot, patch)`. The latter
only changes the user-facing `ConfigName`, `Alias`, and `Description` fields;
hardware and protocol identity fields stay immutable.

The included example can be used as a serialization and XG5000 validation
harness:

```bash
cargo run --features write --example write-module -- \
  input.xgwx output.xgwx 0 2 5
```

## TUI PoC

Run the included example against a local workspace file:

```sh
cargo run --example tui -- path/to/project.xgwx
```

Use `j`/`k` or arrow keys to move through programs, and `q` or `Esc` to quit.
Use `Tab` to switch between program structure, network, variable, parameter,
and data views. Use `PageUp`/`PageDown` to scroll the right detail panel.

The TUI detail panels intentionally render complete details and rely on
scrolling instead of hiding entries behind `... more` counters. In the
Parameters view, `BASIC PARAMETER` attributes with numeric suffixes such as
`KEY_0`, `KEY_1`, and `KEY_2` are grouped under `KEY` with indented index rows.

The Networks view joins parsed Cnet and FEnet configuration records back to
network modules by stable module type only: `NetworkModule Id` equals
`XGPD_CONFIG_INFO_* Type`. It does not use base or slot for that association,
because those fields can be changed by users in XG5000 projects.

## Inspect Example

Run the non-interactive inspector for a concise text summary:

```sh
cargo run --example inspect -- fixtures/XGB_Enet01.xgwx
```

This prints the CPU model and program payload versions alongside project counts
and decoded Cnet, FEnet, HSC, position, and PID summaries. It is useful for
comparing parser output across fixture files without opening the TUI.

## Benchmark Example

Run the fixture-backed benchmark in release mode:

```sh
cargo run --release --example bench -- --warmup 50 --iterations 500 fixtures
```

The benchmark reads `.xgwx` fixture bytes once, then measures parser-only work
and parser plus higher-level decoding paths such as variables, ladder programs,
parameters, network summaries, and decoded payloads.

## GUI Ladder PoC

Run the graphical ladder viewer:

```sh
cargo run --features gui --example gui -- path/to/project.xgwx
```

The GUI lists decoded ladder programs on the left and renders the selected
program as a scrollable, zoomable ladder canvas using parsed rungs, cells, and
wire segments.

## WebAssembly Demo

The parser can be compiled for browser use through the optional `wasm` feature.
The browser-facing API accepts bytes, so users can parse files selected from a
local file picker without uploading them anywhere.

```sh
cargo check --target wasm32-unknown-unknown --features wasm
wasm-pack build --target web --out-dir web/dist/pkg --out-name libxgwx --features wasm --no-default-features
cp web/index.html web/styles.css web/app.js web/dist/
```

The GitHub Pages workflow builds the same upload-only demo from `web/`. It does
not publish `.xgwx` fixtures into the Pages artifact. The demo shows project
summaries, a program sidebar, decoded variables, hardware modules, decoded
network summaries, and a parameter selector with the same section attributes
and decoded HSC, position, PID, safety, Cnet, and FEnet details as the TUI. It
also includes a best-effort SVG ladder viewer for decoded ladder programs.

## Fixtures

The repository includes small `.xgwx` files in `fixtures/` for the normal test
suite. They are included with the original author's permission for parser
development and testing; see `fixtures/README.md` for attribution and
permission details.

To run an additional smoke test against another real workspace, keep that file
outside the commit and pass it with an environment variable:

```sh
LIBXGWX_FIXTURE=path/to/project.xgwx cargo test parses_real_fixture_from_env -- --ignored
```

CPU and hardware writes are CPU-aware: XGK model changes check retained base
and slot limits; cross-family and compact-model conversions are rejected.
The CPU catalog also recognizes XGI models, including
`XGI-CPUE` (configuration type `106`). An XGI configuration can select its
existing model without changing the file; conversion to another XGI model is
still rejected because its parameters and hardware have not been migrated.
XGI supports LD, SFC, and ST programs; the `ProgramData` version identifies
the language of an individual program. See [XGI CPU support](docs/xgi-cpu.md).
The XGK module catalog cannot edit compact hardware. The captured XBM-DR16S
profile protects built-in I/O identity while allowing comments, and is exposed
through `cpu_hardware_profile()` and WASM `hardware.cpuProfile`.
See [CPU hardware validation](docs/cpu-hardware-validation.md) for the verified
scope and native acceptance evidence.

Structural contact/coil editing, row insertion and vertical branch connection editing are available for verified LD layouts; see [LD editing and native acceptance](docs/ladder-editing.md) for the API, supported operations and limits.

IEC scalar BOOL outputs can drive a horizontal ladder wire: select ENO or a
comparison OUT and use F5, Enter, or double-click. F5 on the following blank
cell extends the wire. Arithmetic and conversion OUT fields keep their numeric
destination. Comparisons accept two sources and an optional BOOL destination;
wiring OUT replaces that assignment. Unsupported block layouts remain guarded.

### SFC charts

`XgwxDocument::sfc_programs()` reads native SFC XML blocks and their positioned
entities independently of binary ladder bodies. It retains unknown property
attributes. The WASM summary exposes this model as `sfc`; the VS Code editor
renders steps, transitions, labels, jumps, and variable actions in a dedicated
SFC view.

With `write`, `edit_sfc_entity(&SfcEntityPatch)` supports step comments
and existing direct `%MX` BOOL transition conditions, with stale-value and
position checks. Transition annotation cells are synchronized. Step names,
qualifiers, program references, and structural changes remain read only.
Native XG5000 checks and Save As comparisons are documented in
[`fixtures/sfc/README.md`](fixtures/sfc/README.md).
