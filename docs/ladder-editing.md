# Structural LD editing

The opt-in `write` feature adds `XgwxDocument::edit_ladder_cell` and the WASM
`edit_xgwx_ladder_cell` export. These insert, replace or remove actual element
records. The older `update_ladder_cell_text` API remains available for bounded,
same-length text changes.

Supported programs use the captured `LD VER 1.1`, `ProjectType=1` layout:
linear and branched rows containing normally open/closed contacts, output/set/reset
coils, horizontal wires and preserved END instructions. Recognized comments,
application instructions and pulse elements are preserved, allowing contact/coil
edits elsewhere in the same program. Their records remain protected. The entire
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
