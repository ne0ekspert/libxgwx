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
text, insertion and guarded application deletion are supported as described
below. The entire program is validated before editing; unknown record
layouts and malformed or truncated records reject the structural operation.

Contacts occupy columns 0–8; coils occupy column 9. `raw_y` is the decoded
physical row coordinate (0, 4, 8, …), not a logical rung index. Existing rows are
preserved, including empty rows. An empty program can receive its first element
at any supported physical row, retaining gaps as sparse rows. Y coordinates
are u32 in the Rust API and use three little-endian bytes in native records.
`insert_ladder_row` inserts a physical row before the selected
coordinate, matching native Ctrl+L, and stretches crossing branch connections.
`edit_ladder_branch` adds or removes a vertical connection between adjacent rows
at a column boundary. Row deletion and horizontal-wire editing are not implemented.

Addressed elements accept uppercase P/M/K/F/L/T/C followed by decimal digits,
or a decimal D register with one hexadecimal bit index (`D0000.0` through
`D0000.F`), within 2–32 ASCII bytes total. INV, PUP and PDN use an empty operand. This is a
bounded serialization syntax, not CPU-specific device-range or program
validation. Symbol names and other addressing forms are not supported by the
structural API.

The private manual coverage audit distinguishes application/comparison catalog
entries from native contact and coil elements. For example, LOAD/AND/OR use a
normally open element with cell, series or parallel placement; they are not
three application blocks with interchangeable native IDs. `editorCovered`
includes these existing element forms, while `cataloged` counts only the
instruction catalogs. Neither count establishes CPU availability or native
acceptance of every instruction. END, NOP and stack/control-flow commands remain
separate from the editable element inventory.

The D-register bit syntax is confirmed by `XGK(B)InstructionHelp_Kr_V3.5.chm`,
sections 2.2.2 (bit data, examples `D0010.1` and `D0011.A`) and 2.3.9
(D registers). The LOAD/AND and OUT instruction tables explicitly permit `D.x`.
This manual check confirms the addressing rule; it does not replace native
XG5000 validation of serialized edits.

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
Inserting a coil adds trailing wire without bridging earlier gaps. After a
branch boundary at coordinate 3, 6, etc., the first trailing wire cell is
that boundary plus 1; contacts and ordinary wires advance by 3. This distinction
also applies when adding an output application on a lower branch row.

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
to 65,535 physical rows (indices 0 through 65534), including the native extended
row-count header. Connections must join adjacent
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
and validates arity, text, and occupied spans. `insert_ladder_comparison` covers
78 input contacts from 13 manual families: word, DWORD, REAL, LREAL, string,
word/DWORD group, word/DWORD range, nibble, byte, unsigned word and unsigned
DWORD. The six relations are `=`, `>`, `<`, `>=`, `<=` and `<>`. Binary forms
occupy three cells; group/range forms occupy four. Native identifiers come
from XGTCodeDB, and record flags remain contact flags when editing.
The same input envelope supports the indexed-bit contacts `B` and `BN` (IL
`LOADB`/`LOADBN`). Their source and bit-index operands are WORD values; only the
low four index bits select the bit, so a WORD index is not restricted to 0–15.
The source must be a permitted word device rather than a constant or bit address.
The VS Code text prompt uses `LOADB`/`LOADBN`, preserving `B` as the ordinary
normally-closed contact alias. Serialized records keep the native `B`/`BN` names.
`FF` is an output application with a BIT destination, matching the manual's
toggle-on-rising-input operation.
`delete_ladder_comparison` checks the expected full source text, native opcode
and operand count before removing the contact and its operand references.
It leaves the occupied gap and preserves other elements and row coordinates.
Native D= to DG= capture verifies that comparison resizing keeps the left
anchor, updates the remembered operand position and consumes adjacent wires;
both complete captured payloads are reproduced byte for byte. Output applications
retain their right anchor. Growth into another element or branch is rejected. Native Check Program still determines
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

The Instruction selector exposes 877 fixed-arity LD application instructions,
including 15 operandless instructions.
For example, `MOV,0,D000000` can become `ADD,1,2,D000000` and then
`TON,T0000,100`. Edit operands before applying; selector defaults are placeholders.
The browser filters documented XGK model incompatibilities for 31 instructions
from reviewed manual tables (module transfers, communication commands and
INLATCH). The document writer rejects incompatible insertion and mnemonic
replacement atomically; operand-only repairs of an existing instruction remain
possible. Other commands and unknown CPU models are not filtered. Firmware and
module requirements are not checked. Use XG5000 Check Program for full
instruction availability and operand validity. Basic/control-flow categories
1 and 22 remain excluded from replacement. Operandless application blocks can
be inserted and replaced; END remains protected.

`scripts/generate-instruction-catalog.py` reproduces the factual mnemonic,
`nIndex` opcode and `bySize` operand count mappings from an XGTCodeDB TSV export
of installed XG5000 4.82.1.0 `l.kor/CMDDB.mdb`. The database is not redistributed.
Export SHA-256: `75eded4ca287e08334d8789e742e3c4305835bf76a586ec858b2539032b2f957`.
The expanded 877-entry catalog uses a fresh full-table export with SHA-256
`84f9637c25ece73a54d5340871459a097bf5362eae561b84a359ec3f05b67c25`.

Native edits captured in `fixtures/ladder-edit/instructions/R70.bin` (MOV to ADD)
and `R71.bin` (ADD to TON) verify growth and shrinkage. The first comparison
normalizes display heights recalculated by native editing on unrelated rows;
all remaining bytes match. ADD to TON matches byte for byte. All 877 catalog
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
895 of the 942 application/comparison entries that take operands have reviewed
operand rules. The 15 operandless entries need no operand metadata.
Arity mismatches, absent pages and unrecognized type tables have no inferred rule.
Each operand retains its manual page for review. Missing permission tables are
represented as unknown rather than prohibited.

Some shared tables are imprecise or contain errors: the I2R/I2L table labels both
operands WORD/DWORD despite its conversion explanation, and the DEC table says
NIBBLE/BYTE despite describing signed word decrement. Reviewed overrides separate
MOV/DMOV, real moves, integer/real conversions, arithmetic widths, increment and
decrement variants, and nibble/byte moves. Other shared tables retain their unions;
these are not a complete per-CPU instruction checker.

The generator expands `NAME(EX)` motion-module variants, inherits vertically
merged type cells, and excludes relative memory descriptions such as `S2+1`
from argument counts. Scaling variants retain their individual INT/DINT/REAL
types. Independent usage tables resolve stale extra rows; reviewed page-specific
repairs cover the PIDINIT loop operand, SRS, XSWR and image-only output comparison
diagrams. Constant-only PID operands retain their usage table's empty device list.

Audit the complete chapter 4 instruction list against a local native database:

```sh
python3 scripts/audit-instruction-help.py /path/to/extracted-help \
  /path/to/XGTCodeDB.tsv --output /path/to/private-coverage.json
```

The report records manual pages, native IDs/arity, catalog membership and operand
rule coverage. It distinguishes catalog availability from native validation.
The v3.5 manual contains eight names that do not match the installed database
(including ADDCP, BETOW and XTURN); these are reported for review, not assigned
invented opcodes. Basic/control categories remain outside the writable application catalog until
their native layouts are verified. All 78 comparison contacts were generated in two programs and passed native
all-program checks with 0 errors and 0 warnings on XGK-CPUUN. After Save As,
every program payload and parsed local record remained exact (12,986 and
13,580 bytes), without normalization. Private evidence: XGC1GEN/XGC2GEN and
XGC1S/XGC2S in iec-full-edit-audit-20261004. This is editor/compiler validation;
PLC execution was not performed.
ADDBP, BTOW and XTRUN rules use the correctly spelled command in their own usage
tables despite the page-title typos. The remaining unmatched titles are retained
in the audit for explicit review.

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

### XGK application deletion and basic bit reset

`delete_ladder_instruction` removes a catalog application, its operand
references and its immediately preceding output feed wire. Expected full text,
opcode, arity, output anchor and a single unbranched row group are checked.
END, unknown instructions, branches and output comments remain protected.
WASM reports `instructionDeletion` per cell; the shared diagram Delete action
uses this capability. Native MOV deletion leaves the input comparison alone;
Check Program reports one incomplete-rung error. Tests reproduce that entire
13,440-byte captured payload exactly.

BRST and BRSTP are explicit exceptions to the excluded basic category: their
shared application envelope has a native BRST capture. Operand rules come from
the manual (BIT destination and WORD bit count). Reinserted BRST reproduces the
complete 13,592-byte native payload and passed native checks with 0 errors and
0 warnings. This does not establish support for other basic/control layouts.

### Native indexed-bit and FF evidence

Native XG5000 insertion of B, BN and FF passed full Check Program with
0 errors and 0 warnings. The 676-byte saved program is reconstructed exactly
by the writer regression fixture, with local-symbol records also matching.
The reconstruction uses the native dialog's padded D addresses; arbitrary
short-address normalization and PLC runtime behavior remain unverified.
In the text prompt, use LOADB/LOADBN for indexed-bit contacts so the existing
B alias continues to mean a normally closed contact.

### Quoted string operands

XGK instruction text supports single-quoted ASCII string constants with embedded
spaces, commas and parentheses, up to 31 characters. The prompt and native
combined/decomposed record parser preserve the complete quoted operand. Literal
permission is required explicitly by the manual rule, so string destinations
and unknown literal permissions remain guarded. Non-ASCII literals and embedded
apostrophe escapes remain unsupported pending native encoding evidence.

The operand generator also reads usage tables headed `문자열`, including source
literal permissions and destination device restrictions for string instructions.

Native $MOV/$MOVP string-literal insertion passed full Check Program with 0 errors
and 0 warnings. The writer reconstructs the complete 683-byte saved payload and
local records exactly, including the observed literal row-prefix variant.

### Reviewed CPU model tables

`scripts/generate-instruction-availability.py` extracts the unambiguous model
tables in manual sections 4.39 and 4.40, plus the explicit INLATCH restriction
in section 4.24.21. Reproduce or check the generated data with:

```sh
python3 scripts/generate-instruction-availability.py /path/to/extracted-help --check
```

`ladder_instruction_cpu_allowed` returns `None` for unreviewed commands or
non-XGK/unknown models, and a model-table result otherwise. `Some(true)` does
not establish firmware, module, runtime or native edit acceptance. INLATCH is
limited to XGK-CPUUN/HN/SN; GETIP and SETIP have the same reviewed model list.
GETCOMM and PUTCOMM are excluded on every reviewed XGK model. Motion pages
with different XGF/XBF command variants are deliberately not interpreted as
a single shared model restriction. Native database CPU bit masks remain
undecoded; no instruction or firmware availability is inferred from them.

### Output entry from interior blank cells

The sibling editor accepts coil and output application commands from any blank
XGK cell, including cells on lower branch rows. OUT/SET/RST/OUTP/OUTN and
application commands use the right output position on the same physical row.
Contact commands still use the cursor column. Interior P/N aliases remain
contacts; pulse coils use OUTP/OUTN. The existing occupied-output checks prevent
an insertion from replacing an output already on that row.

Rendered checks on a private copy of `elements.xgwx` inserted OUT M00030 from
row 12, column 2 and SET M00031 from row 16, column 5. They also inserted
MOV 1 D100 from row 12, column 3. Double-click and Enter opened the built-in
prompt, each output appeared at the right side, and undo restored the baseline.
The WASM regression verifies both coil and application insertion retain all
nine original branch connections and the existing inverse coil on row 8.

Native acceptance of the branch-output writer uses XGK-CPUSN and XG5000
4.82.1.0. The generated OUT/MOV branch additions passed all-program logical,
syntax and duplicate-coil checks with 0 errors, 0 warnings and 13 messages.
Native Save As preserved all 2,531 ProgramData bytes and the one local-symbol
table exactly. `fixtures/ladder-edit/branches/branch_outputs.bin` retains the
complete saved payload; the writer test reconstructs it without normalization.
This proves file-format/compiler acceptance, not PLC runtime execution.


## Wide XGK canvas acceptance — 2026-10-06

Synthetic XGK-CPUSN projects at rows 256 and 65534 opened at their expected
physical positions, passed native Check Program with zero errors and warnings,
and retained byte-identical ProgramData after Save As. The boundary project
also reopened at row 65534. Public fixtures and details are in
[`fixtures/canvas-rows`](../fixtures/canvas-rows/README.md).

The VS Code canvas grows on scroll up to 65,535 rows, rendering blank cells only
around the viewport. Selecting a distant empty cell and inserting a contact,
coil, comparison or application instruction directly extends the sparse native
program. Scrolling does not write bytes. Native boundary validation covers
contact/coil projects; local tests additionally exercise high-row function
operands, branch references, comment shifts, row-count header transitions,
stale edits and overflow rejection. IEC high-coordinate support remains limited
to its separately validated writer range.
