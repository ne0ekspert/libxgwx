# Structural LD editing

The opt-in `write` feature adds `XgwxDocument::edit_ladder_cell` and the WASM
`edit_xgwx_ladder_cell` export. These insert, replace or remove actual element
records. The `update_ladder_cell_text` API also supports variable-length operands in
recognized application instructions; other text records retain bounded,
same-length editing.

Supported programs use the captured `LD VER 1.1`, `ProjectType=1` layout:
linear and branched rows containing normally open/closed contacts, output/set/reset
coils, horizontal wires and preserved END instructions. Recognized comments,
application instructions and pulse elements are preserved, allowing contact/coil
edits elsewhere in the same program. Instruction operand text is editable as
described below; instruction insertion and deletion remain protected. The entire
program is validated before editing; unknown record layouts and malformed or
truncated records reject the structural operation.

Contacts occupy columns 0–8; coils occupy column 9. `raw_y` is the decoded
physical row coordinate (0, 4, 8, …), not a logical rung index. Existing rows are
preserved, including empty rows. An empty program can receive its first element
at raw row 0. `insert_ladder_row` inserts a physical row before the selected
coordinate, matching native Ctrl+L, and stretches crossing branch connections.
`edit_ladder_branch` adds or removes a vertical connection between adjacent rows
at a column boundary. Row deletion and horizontal-wire editing are not implemented.

Operands currently accept uppercase P/M/K/F/L/T/C followed by decimal digits
(2–32 ASCII bytes total). This is a bounded serialization syntax, not CPU-specific
device-range or program validation. Symbol names and other addressing forms are
not supported by the structural API.

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
column 1–9 to add or remove its connection to the row below.

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
