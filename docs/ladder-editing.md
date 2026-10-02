# Structural LD editing

The opt-in `write` feature adds `XgwxDocument::edit_ladder_cell` and the WASM
`edit_xgwx_ladder_cell` export. These insert, replace or remove actual element
records. The `update_ladder_cell_text` API also supports variable-length operands in
recognized application instructions; other text records retain bounded,
same-length editing.

Supported programs use the captured `LD VER 1.1`, `ProjectType=1` layout:
linear and branched rows containing normally open/closed, rising/falling-edge and
negated edge contacts; operandless INV, PUP and PDN operations; output, inverse,
set/reset and rising/falling-edge coils; horizontal wires; and preserved END
instructions. Recognized comments and application instructions are preserved,
allowing structural edits elsewhere in the same program. Instruction operand
text and insertion are supported as described below; instruction deletion remains
protected. The entire program is validated before editing; unknown record
layouts and malformed or truncated records reject the structural operation.

Contacts occupy columns 0–8; coils occupy column 9. `raw_y` is the decoded
physical row coordinate (0, 4, 8, …), not a logical rung index. Existing rows are
preserved, including empty rows. An empty program can receive its first element
at raw row 0. `insert_ladder_row` inserts a physical row before the selected
coordinate, matching native Ctrl+L, and stretches crossing branch connections.
`edit_ladder_branch` adds or removes a vertical connection between adjacent rows
at a column boundary. Row deletion and horizontal-wire editing are not implemented.

Addressed elements currently accept uppercase P/M/K/F/L/T/C followed by decimal
digits (2–32 ASCII bytes total). INV, PUP and PDN use an empty operand. This is a
bounded serialization syntax, not CPU-specific device-range or program
validation. Symbol names and other addressing forms are not supported by the
structural API.

```rust
use xgwx::{LadderCellEdit, LadderEditElement, LadderEditKind};

let edit = LadderCellEdit {
    raw_y: 0,
    column: 2,
    expected: None,
    replacement: Some(LadderEditElement {
        kind: LadderEditKind::NormallyOpen,
        operand: "M00002".into(),
    }),
};
// document.edit_ladder_cell(0, &edit)?;
```

For replacement or deletion, provide the expected kind and operand. A stale
selection fails without mutating the document. Set `replacement: None` to delete.
Deletion leaves a wiring gap, matching native Delete; it does not automatically
reconnect contacts. Inserting a contact into an existing wire splits that wire.
Inserting a coil adds trailing wire without bridging earlier gaps.

The sibling VS Code extension exposes Element and Device address controls,
single-element Delete and rectangular keyboard Delete. A selection containing
an unsupported instruction is not deleted. Each completed edit uses the existing
document edit bridge. Programs without the supported layout retain their
existing bounded text editor, with structural insertion/deletion unavailable.
Ctrl+L and the Rows and branches inspector insert rows. Choose a boundary after
column 1–9 to add or remove its connection to the row below. Native rung and
output comments can be created with `edit_ladder_comment`; edits use an expected
text value so stale UI actions fail without changing the document. Rung comment
creation inserts a dedicated comment row and is rejected inside a branch span.
Dedicated rung-comment rows can be removed with
`delete_ladder_rung_comment`; later ladder rows and their embedded coordinates
shift upward, while stale text and branch-crossing deletions fail closed.

## Native acceptance — 2026-09-09

Checked in offline XG5000 4.82.1.0 on Windows 10, using a newly created
XGK-CPUUN project. The native baseline contains NO M00000, NC M00001, output
M00010 and a separate END row. The baseline passes Check Program with zero
errors and warnings.

| Case | Edit | Check Program | Save As |
| --- | --- | --- | --- |
| E01 | NO → NC; M00000 → M42 | 0 errors, 0 warnings | Entire decoded ProgramData identical |
| E02 | Insert NO M00002 into wire | 0 errors, 0 warnings | Entire decoded ProgramData identical |
| E03 | Delete NC M00001 | 3 errors, 0 warnings, matching native deletion | Entire decoded ProgramData identical |
| E04 | Output → Set coil | 0 errors, 0 warnings | Entire decoded ProgramData identical |

E03's errors are expected disconnected-rung diagnostics, not a successful
executable program. A separate native Delete control produces the same gap and
three errors. XG5000 displays M42 as M00042 but preserves the shorter stored
operand through Save As.

Six unit-test transitions compare complete generated payloads to native
captures: first contact, coil insertion, contact deletion, NC insertion into
wire, contact restoration and deletion preserving the native high-water
coordinate. Other checks cover stale expectations, invalid coordinates/operands,
terminal-instruction protection, unknown records and every truncated prefix.
Reset coils were subsequently checked in the branch acceptance cases below.

Reproduce generated inputs with:

```sh
cargo run --features write --example ladder-acceptance -- /tmp/new-ld-cases
cargo test --features il,wasm,write
cargo clippy --all-targets --features il,wasm,write -- -D warnings
```

Captured test fixtures and provenance are in
[`fixtures/ladder-edit`](../fixtures/ladder-edit/README.md). Full local evidence,
including native projects, generated inputs, resaves, screenshots and hashes,
is retained under `target/xg5000-ld-acceptance-20260909` (ignored by Git).
After VM shutdown, all four generated input hashes were unchanged and all
four saved ladder payloads were byte-identical to the generated payloads.

The extension passed its check/test scripts and Playwright interactions with
the real WASM bundle at 1440×1000 and 900×800. Browser checks covered replacement,
variable-length operands, insertion, delete/reinsert, Set coil, rectangular
deletion, END/branch protection, and editing empty programs and cleared rows.
The VS Code messaging bridge was mocked; native VS Code Save/Undo was not
interactively retested. No PLC download or execution was performed.


## Branch editing and native acceptance — 2026-09-09

```rust
use xgwx::LadderBranchEdit;

// Create space below the first row, then connect it after the first column.
// document.insert_ladder_row(0, 4)?;
let connection = LadderBranchEdit {
    raw_y: 0,
    boundary: 1, // 1 through 9, after this many columns
    expected: false,
    present: true,
};
// document.edit_ladder_branch(0, &connection)?;
```

Set `expected: true, present: false` to remove an existing connection. Stale
expectations fail without mutation. The WASM exports are
`insert_xgwx_ladder_row` and `edit_xgwx_ladder_branch`. Row insertion supports up
to 61 physical rows in this captured layout. Connections must join adjacent
rows; overlapping longer spans and protected comment rows reject the operation.
Adding/removing a connection preserves unrelated records and rebuilds native
row groups. Connection removal does not reconnect or repair dangling elements.

The shared record decoder now renders stored horizontal wires and reciprocal
vertical branch references exactly. A lower branch ending at a junction no
longer receives a phantom wire to the right rail. Unsupported layouts retain
the earlier decoding fallback and cannot use these structural operations.

Native controls R40–R44 capture adding a branch, Ctrl+L across a branch, inserting
a sparse blank row, connecting that blank row, and inserting its parallel
contact. Unit tests compare complete generated ProgramData with these controls.
Additional tests cover edits in the mixed `elements.xgwx` program, preservation
of unrelated records, stale edits and malformed branch references.

The branch cases use offline XG5000 4.82.1.0 on Windows 10. E01–E04 and E08
edit `elements.xgwx`; E05–E07 use the small XGK-CPUUN linear fixture.

| Case | Edit | Check Program | Save As |
| --- | --- | --- | --- |
| E01 | Branch NO P00001 → NC M42 | 0 errors, 0 warnings | Entire decoded ProgramData identical |
| E02 | Delete branch contact P00001 | 1 error, 0 warnings (disconnected branch) | Entire decoded ProgramData identical |
| E03 | Insert NO M00002 inside a coil branch wire | 0 errors, 0 warnings | Entire decoded ProgramData identical |
| E04 | Branch Set → Reset M00020 | 0 errors, 0 warnings | Entire decoded ProgramData identical |
| E05 | Insert row, connect it and add parallel NO M00002 | 0 errors, 0 warnings | Entire decoded ProgramData identical |
| E06 | Remove the new branch connection | 1 error, 0 warnings (dangling contact) | Entire decoded ProgramData identical |
| E07 | Ctrl+L before the lower branch contact | 0 errors, 0 warnings | Entire decoded ProgramData identical |
| E08 | Edit a linear contact in the mixed program | 0 errors, 0 warnings | Entire decoded ProgramData identical |

All eight generated ladder payloads survived native Save As unchanged. E02 and
E06 intentionally create disconnected logic and are not valid executable
programs. Regenerating the inputs from the final source produced identical
ladder payloads. Evidence is retained locally under
`target/xg5000-branch-acceptance-20260909`, including input projects, native
controls, resaves, screenshots, the generation/verification scripts and hashes.

Browser validation exercises the real extension JavaScript and WASM at
1440×1000 and 900×800: edit/delete/reinsert existing branch contacts, preserve
comments, Ctrl+L, add/remove connections, insert a parallel contact, stretch a
branch with Ctrl+L, and remove/restore an individual stretched segment. The
VS Code messaging bridge is mocked; native VS Code Save/Undo and PLC execution
remain outside this validation.

Reproduce branch inputs with:

```sh
cargo run --features write --example branch-acceptance -- /tmp/new-branch-cases
```


## Variable-length function-block operands

`update_ladder_cell_text` and the existing WASM `update_xgwx_ladder_cell` export
accept different-length operands for recognized application instructions in
supported LD layouts. For example, `MOV,0,D000000` can become `MOV,1,D1` or
`MOV,12345,D000042`. The editor's Source text field exposes this without requiring
padding to the original length.

Instruction records contain both a combined string and decomposed mnemonic and
operand strings. The writer updates both representations and their length bytes,
preserving unrelated records. Operand-only edits retain opcode, coordinates,
flags and companion references. Mnemonic changes update the opcode, rebuild
operand companions and adjust preceding wires while keeping the right edge fixed.
Expansion into another element or branch is rejected. This also corrects same-length edits that previously updated
only the combined string. After an edit, use freshly decoded offsets for any
following cell; the old offset may have moved.

The mnemonic may change to an entry in `ladder_instruction_catalog()`; the operand
count must match its definition. Operands must be nonempty printable ASCII tokens,
separated by commas; whitespace around commas is trimmed. The combined instruction
text may contain at most 255 UTF-16 units, the captured record's length limit.
Unsupported layouts retain the older same-length text restriction. CPU-specific
operand validity is checked by XG5000, not by this serialization API.

The WASM cell summary exposes `instructionTextEditing` when this operation is
supported; `instructionChoices` exposes the replacement catalog. Stale selections,
unknown mnemonics, incorrect operand counts, malformed
records, and direct edits to internal operand copies are rejected without mutation.

```sh
cargo run --features write --example instruction-acceptance -- /tmp/new-instruction-cases
```


Native acceptance in offline XG5000 4.82.1.0 on Windows 10:

| Case | Replacement | Check Program | Save As |
| --- | --- | --- | --- |
| F01 | `MOV,1,D1` | 0 errors, 0 warnings | Entire ProgramData identical |
| F02 | `MOV,12345,D000042` | 0 errors, 0 warnings | Entire ProgramData identical |
| F03 | `MOV,12345,D1` | 0 errors, 0 warnings | Entire ProgramData identical |
| F04 | `XDST,1,1,700,100,10,0,0` | 0 errors, 0 warnings | Entire ProgramData identical |

These edit `fixtures/elements.xgwx`. Rust tests additionally cover unchanged
surrounding records, byte-exact restoration, equal-total-length edits with
different operand lengths, stale selections, internal operand-copy protection,
and invalid mnemonic/count/text/length rejection. WASM tests edit a following
instruction using its new offset and reject its stale offset.

Playwright exercised longer and shorter MOV operands, invalid mnemonic/count
controls, and a following XDST edit at 1440×1000 and 900×800 with no console
errors. The real extension JavaScript and WASM were used with a mocked VS Code
messaging bridge. Native VS Code Save/Undo and PLC execution were not tested.
Full local evidence is retained under
`target/xg5000-instruction-acceptance-20260909` (ignored by Git).


### Function insertion

`insert_ladder_instruction` places a fixed-arity application instruction at the
XGK output end of a row. It uses the existing catalog, including MOV and I2R,
and validates arity, text, and occupied spans. `insert_ladder_comparison` places
the captured `=`, `>`, `<`, `>=`, `<=`, and `<>` contact forms in three adjacent contact cells. Their
native opcodes are 1254, 1256, 1258, 1260, 1262, and 1264 respectively; their record flags remain contact flags when
changing the comparison or its operands.
`delete_ladder_comparison` checks the expected full source text and native
comparison opcode before removing the comparison and its two operand references.
It leaves the three-cell gap and preserves other elements and row coordinates. Native Check Program still determines
CPU availability and whether the completed rung has a valid input condition.

IEC `insert_iec_ld_function` places MOVE, ADD/SUB/MUL/DIV, or EQ/GT/GE/LT/LE at
an available grid position. Inputs and the output are supplied in reference
order, with the output last. The block requires room for its body and adjacent
operand fields; the selected block anchor must be `4, 7, ... 91`. Existing
contacts, coils, comments, function bodies, operand fields, and branches are
protected from overlap. Known scalar types must be compatible, and the output
must be writable. Array operands and stateful instances are outside this API.
Existing row groups and unrelated programs are preserved; pin references and
the circuit graph are validated before applying the edit.

The VS Code editor uses its built-in insertion picker on double-click or Enter
in a blank cell. XGK offers comparisons in contact columns and application
instructions in the output column. IEC offers the scalar functions above at
positions with room for operand cells. The native command picker accepts spaces
between the instruction and operands, for example `MOV 1 D100`. Existing inspector
Source text editing retains comma-separated operands.

Native XG5000 4.82.1 captures verify the XGK MOV, I2R, `=`, and `>=` records.
The generated XGK suite with complete input conditions compiled with 0 errors
and 0 warnings; native Save As preserved its 3187-byte ProgramData payload
byte-for-byte.
The six-operator comparison suite also compiled with 0 errors and 0 warnings
on 2026-10-01. Native Save As preserved its 4213-byte ProgramData payload
byte-for-byte; evidence is retained in `target/xgk-comparisons-20261001`.
Small pin-descriptor fixtures under `fixtures/function-bodies` contain native
MOVE/ADD/EQ bodies and XGK comparison records, without project logic. The
generated IEC suite containing all ten functions compiled with 0 errors and
preserved all seven decoded ProgramData payloads byte-for-byte on Save As.
The test suite writes five comparison results to one BOOL address; native
double-coil checking therefore reports warnings. This is format and compiler
acceptance, not PLC execution validation.

`examples/function_placement.rs` generates placements from a caller-supplied
source; `examples/native_function_compare.rs` compares native Save As payloads.
Full local capture evidence is kept outside Git under
`VMs/xg5000-win10/captures/function-placement-20260930`.

### Instruction replacement catalog

The Instruction selector exposes 859 fixed-arity LD application instructions.
For example, `MOV,0,D000000` can become `ADD,1,2,D000000` and then
`TON,T0000,100`. Edit operands before applying; selector defaults are placeholders.
The catalog is not filtered by CPU or OS version. Use XG5000 Check Program for
instruction availability and operand validity. Basic/control-flow categories and
zero-operand instructions are excluded from replacement.

`scripts/generate-instruction-catalog.py` reproduces the factual mnemonic,
`nIndex` opcode and `bySize` operand count mappings from an XGTCodeDB TSV export
of installed XG5000 4.82.1.0 `l.kor/CMDDB.mdb`. The database is not redistributed.
Export SHA-256: `75eded4ca287e08334d8789e742e3c4305835bf76a586ec858b2539032b2f957`.

Native edits captured in `fixtures/ladder-edit/instructions/R70.bin` (MOV to ADD)
and `R71.bin` (ADD to TON) verify growth and shrinkage. The first comparison
normalizes display heights recalculated by native editing on unrelated rows;
all remaining bytes match. ADD to TON matches byte for byte. All 859 catalog
entries pass serialization, opcode, and byte-exact restoration tests. These are
structural checks, not native execution of every instruction.


Generated F05 (`ADD,1,2,D000000`) and F06 (`TON,T0000,100`) also passed native
XG5000 Check Program with 0 errors and 0 warnings. Separate Save As results R75
and R76 retain their entire ProgramData byte for byte (2370 and 2341 bytes).
Playwright verified the selector sequence MOV → ADD → TON → SUB → MOV, operand
edits after resizing, invalid mnemonic/count rejection and a following XDST edit.
The real JS/WASM ran with a mocked VS Code bridge at 1440×1000 and 900×800;
no browser console errors occurred. PLC execution and CPU-specific validation of
the complete catalog were not performed. Evidence:
`target/xg5000-instruction-types-20260909` (ignored by Git).

### XGK operand rules

`ladder_instruction_operand_rules` exposes factual type, constant and device-area
permissions extracted from `XGK(B)InstructionHelp_Kr_V3.5.chm` (SHA-256
`b866d589d800addcb039f9f09b182592464d6367faa55abf6dde9ac34a1580e0`).
Regenerate with `scripts/generate-operand-rules.py` after locally extracting the
CHM with 7-Zip; the manual text and images are not included in the repository.
Use `--check` to verify regeneration without modifying the generated file. Missing
or unrecognized manual input is rejected before writing the catalog.
714 of the 865 application/comparison entries have matching operand tables.
Arity mismatches, absent pages and unrecognized type tables have no inferred rule.
Each operand retains its manual page for review. Missing permission tables are
represented as unknown rather than prohibited.

Some shared tables are imprecise or contain errors: the I2R/I2L table labels both
operands WORD/DWORD despite its conversion explanation, and the DEC table says
NIBBLE/BYTE despite describing signed word decrement. Reviewed overrides separate
MOV/DMOV, real moves, integer/real conversions, arithmetic widths, increment and
decrement variants, and nibble/byte moves. Other shared tables retain their unions;
these are not a complete per-CPU instruction checker.

WASM instruction choices include `operandRules`. The command picker displays
expected types and filters declared variable suggestions. XGK integer symbols use
storage names (WORD/DWORD/LWORD), so INT/UINT, DINT/UDINT and LINT/ULINT match the
corresponding storage width. REAL/LREAL remain distinct. Raw device addresses are
starting addresses, not typed IEC variables: D100 can be used for a multiword REAL
operand without declaring a REAL symbol there. The writer rejects decoded
constant/device-area violations in insertion and instruction text replacement.
Literal values are checked against the decoded operand types with exact integer
parsing. Signed decimal operands use signed bounds; hexadecimal/binary values
may express full-width bit patterns. WORD/DWORD/LWORD storage operands accept
signed values or unsigned bit patterns. REAL/LREAL reject non-finite values and
values outside their representable magnitude. Shared tables accept the union
of their listed types. Alignment, complete spans, indexed-device syntax and PLC
model-specific restrictions still require XG5000 Check Program. The serializers
are unchanged by these checks; the native insertion acceptance described above
continues to cover their record layouts.

The operand boundary suite adds `MOV 65535`, `MOV -32768`, signed-decimal ADD,
hexadecimal ADD and `RADD 3.4E38`. XG5000 4.82.1 checked the generated XGK project
with zero errors and zero warnings. Native Save As preserved its decoded program
payload byte for byte. Evidence is in
`/home/ne0ekspert/VMs/xg5000-win10/captures/iec-editor-cells-20261002/`.

### Maintained acceptance tools

- `function_placement`: generates the supported placement suites or one placement
  from a caller-supplied workspace. It rejects overflowing row coordinates.
- `block_insertion_acceptance`: dumps native function records and optional fixture
  bodies from a caller-selected program.
- `native_function_compare`: checks all decoded program payloads after native
  Save As. A changed program count, payload byte, payload length, function shape
  or decoded IEC graph causes a nonzero exit status.

Local `iec_*_probe`, inventory and group-analysis examples are investigative tools;
they are not general editor APIs or evidence that unsupported edits are enabled.
