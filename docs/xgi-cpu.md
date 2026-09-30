# XGI CPU identification

The installed XG5000 4.82.1 CPU name table is ordered by numeric
`<Configuration Type>`. Anchoring its entries to the existing XGK/XGB mappings
gives the XGI model mapping below. The base limits for the six standard models
come from the [LS ELECTRIC XGI CPU manual](https://www.ls-electric.com/upload/customer/download/a51ea074-ffef-434c-b369-ccbf446a03cc/MANUAL_XGI_CPU_ENG.pdf), chapter 2. The [CPUS/P OS notes](https://ssq.ls-electric.com/uploads/document/17114964489140/OS%EB%B2%84%EC%A0%84%EA%B4%80%EB%A6%AC%ED%91%9C_XGI-CPUH%EC%99%B85%EC%A2%85_V4.40.pdf) describe the same CPU hardware as CPUS on an XGR-E08P or XGR-E12P base without expansion. A base count includes base 0.

| Type | Model | Maximum bases |
| ---: | --- | ---: |
| 100 | XGI-CPUU | 8 |
| 102 | XGI-CPUH | 8 |
| 104 | XGI-CPUS | 4 |
| 106 | XGI-CPUE | 2 |
| 107 | XGI-CPUU/D | 8 |
| 110 | XGI-CPUS/P | 1 |
| 111 | XGI-CPUUN | 8 |

The supplied XGWX project stores type `106`, and its basic parameter has
`CPUType="43009"`. All seven `ProgramData` elements report
`Version="LD VER 1.1"` and `ProjectType="2"`. Each payload decodes as a ladder
program. The current partial decoder finds 43 rungs in total but also reports
unrecognized records; IL conversion stops at the first one. Structural editing
uses guarded operations for the captured IEC layouts in all seven programs.
This decode does not yet establish complete ladder semantics for this project.
The VS Code extension shows both Unicode text fragments in byte order and a
positioned IEC layout instead of using the legacy 10-column ladder canvas.
Captured contact and coil records also expose their stored x/y coordinates;
the first program's contact at x=1, y=12 corresponds to XG5000's L3 row.
Y extends past 255 in this workspace and is decoded from both stored bytes.
The IEC row envelope decoder locates 86, 13, 40, 77, 41, 46, and 80 stored
rows in the seven programs, respectively (383 total). Row indices have gaps;
the high-water row counts in the payload headers are larger. All 1,065
captured UTF-16 strings map to exactly one stored row, and every identified
contact, coil, and fixed block position has a Y value equal to four times its
row index. The extension shows each text fragment's stored row and group.
The geometry decoder identifies 168 `FF 02` long horizontal wire records, 105
`FF 01` three-grid-unit wire records, and
192 reciprocal vertical branch pairs across the seven programs. Its branch
endpoints match by group, coordinate, and target row. Short pass-through wires,
function pin links, and other IEC records have validated boundaries, while their
circuit semantics still need to be established.
The next parser stage frames all 1,507 row-local records in this file:
289, 61, 157, 320, 129, 160, and 391 per program. It recognizes 81
function-block records and validates simple contacts, coils, comments, wires,
function operands, branches, and link references. Each row has exactly one
segmentation matching its stored record count. The frame API returns `None`
if a row is ambiguous or fails to consume its entire native byte span.
All 81 function records expose an opcode family, opcode, name, position, body
rows, control ports, and data ports. Eleven stateful blocks also expose an
instance name. The body decoder validates 192 data ports and 162 control ports,
including their visual rows, direction, native reference ordinal, type mask,
and array type expression. The 46 MOVE ports use the captured
`ARRAY[0..-1] OF ANY` type expression.
All 196 captured `0x68`/`0x69` link references resolve to exactly one function
block at the stored group and x/y coordinate (50, 4, 23, 24, 23, 28, and 44
per program). Each block header's pin count matches its references, with one
reference per ordinal. In this sample, `0x68` marks input pins and `0x69`
marks the output pin. The parser validates each reference against its decoded
port direction, visual row, and IEC type mask.
All 176 `FF 46` expression records also map uniquely to those blocks: 115 at
input-link rows and 61 at output positions. The extension displays the owning
block and pin beside each editable expression and places expressions at their
stored x/y coordinates in the layout preview. The writer validates classified
local symbols, instance members, device addresses, and literals against the
decoded pin type, and rejects non-writable values on output pins. Numeric
arithmetic using `+`, `-`, `*`, `/`, unary signs, and parentheses is classified
from its operands and checked against the pin type. Unsupported arithmetic
forms are rejected; other unclassified expressions still rely on XG5000 Check
Program.
Direct device operands now parse the address area, width (`X/B/W/D/L`), and
numeric components before type checking. Word addresses are compatible with
WORD, INT, and UINT pins; `%IW0.0.0` is readable, while input addresses are
rejected as output destinations. The [XG5000 IEC user manual](https://www.ls-electric.com/upload/customer/download/555ffd42-c484-498e-bb4b-283f3375c492/manual_XG5000IEC_V2.7.pdf)
shows three-component physical addresses such as `%IW0.1.0` and typed direct
device searches. This check rejects malformed bare device tokens; other
compound forms and CPU-specific address ranges still need separate validation.
The generated program 1 `MOVE.IN=%IW0.0.0` opened in XG5000 4.82.1. It rendered the
address, passed all-program Check Program with 0 errors, 1 warning category,
and 42 messages, and native Save As preserved all seven ProgramData and PB50
local-symbol payloads. The generated and resaved pair is recorded as `GDA` and
`GDAR` in the IEC group-copy capture directory.
The supplied project's program 1 `MOVE.IN` at byte 1422 also accepted `0+1`.
XG5000 rendered the expression, checked all programs with 0 errors, 1 warning
category, and 42 messages, and preserved all seven decoded ProgramData
payloads on Save As. The VS Code source editor emitted bytes identical to the
generated project. The parser rejects this numeric expression on the captured
`TON.PT` TIME pin and as a non-writable MOVE output, as well as `0+TRUE` and
malformed direct-address arithmetic. Evidence is `probe_expression_0_plus_1.xgwx`,
`native_resaved_expression_0_plus_1.xgwx`, `expr-render.png`, `expr-check.png`,
and `expr-editor-after.png` under the IEC group-copy capture directory.
Sixty-one opcode/name-verified fixed function blocks (ADD, SUB, MUL, DIV,
MOVE, EQ, GT, GE, LT, LE) expose their stored positions and typed pins.
The captured `FF 3F 00 00 00 01` records identify 46 IEC LD comments across
the seven programs. The extension can edit those comments, including changes
up to 255 UTF-16 units, while leaving unclassified text read only.

The [LS ELECTRIC manual](https://www.ls-electric.com/upload/customer/download/a51ea074-ffef-434c-b369-ccbf446a03cc/MANUAL_XGI_CPU_ENG.pdf) lists Ladder Diagram, SFC, and Structured Text as XGI programming languages. CPU family alone does not determine the language of a program.

`select_cpu` recognizes the current XGI model as a no-op. XGI model changes
remain guarded because this catalog maps identities and physical limits, but
does not migrate CPU-specific parameters or hardware. On 2026-09-17, the
installed XG5000 4.82.1 Windows VM opened a guest-local copy of the supplied
project. Its project tree identified `LSPLC(XGI-CPUE)`, and the `조명` source
program opened as IEC LD. XG5000 rendered contacts, coils, branches, comments,
and IEC function blocks. Opening directly from the read-only transfer drive
failed, while opening the local copy succeeded. A same-length comment edited by
libxgwx displayed correctly in XG5000, passed Check Program with the same
diagnostics as the original (0 errors, 1 warning category, 42 messages), and survived
native Save As with all seven program payloads unchanged. A native edit from
`조명제어` to `LIGHT_CONTROL_LONGER` changed only the comment marker's length byte
and UTF-16 text in the decoded payload. The variable-length libxgwx writer
produced a byte-identical decoded program payload. This validates IEC comment
editing. The later captures below cover selected element and function edits;
IEC wiring and general function topology remain undecoded.

For the next parser slice, a native edit of the first rising-edge contact from
variable `스위치_1` to existing BOOL variable `ON` changed only that operand's
UTF-16 marker length and text; no other decoded program bytes changed. The
contact's `FF 08` record prefix is 15 bytes before the text marker. Nine such
records appear in this sample. The guarded
`update_iec_ld_rising_contact_operand` writer produces a byte-identical
decoded program 0 payload for this change. This covers one operand record
family; it does not establish a general IEC element layout or symbol
validation rules.

Three more native Save As captures replaced a normally open `FF 06` contact,
a normally closed `FF 07` contact, and an `FF 0E` output coil with the existing
BOOL variable `ON`. Each step changed only the selected UTF-16 operand and its
length marker; the other six program payloads stayed identical. The guarded
`update_iec_ld_element_operand` writer produced a byte-identical decoded
program 0 payload after all three edits. The sample has 351 recognized contact
and output-coil operand strings across all seven programs, including two
`FF 0A` negated rising-edge contacts identified from the same record family.
`FF 0A` has not had a separate native mutation capture. General element and
function creation, deletion, movement, and row-group rebuilding remain guarded.

A native ADD `IN2` edit from literal `1` to `2` changed its `FF 46` string and
seven `FF 43` header bytes elsewhere in program 0. A generated text-only edit
opened in XG5000, displayed `2`, passed Check Program with baseline diagnostics,
and survived Save As with all seven decoded program payloads byte-identical to
the generated file. A generated variable-length edit from `1` to `123` passed
the same native checks and round trip. The extension now exposes all 176
captured `FF 46` expressions in this sample. Function body geometry and typed
pin topology are decoded; complete expression parsing and execution semantics
remain pending.

On 2026-09-22, a graph-validated typed edit changed the `커튼` program's
`TON.PT` input from `T#5s` to `T#6s`. XG5000 4.82.1 opened the generated
workspace, rendered `T#6s` on the timer input, and reported the baseline
0 errors, 1 warning category, and 42 messages for the full project. Native Save As
preserved all seven decoded `ProgramData` payloads byte-identically. The
generated file, native resave, manifest, and screenshots are under
`captures/smarthome-iec-typed-operand` in the offline VM directory.

A native ADD-to-SUB replacement changed the corresponding function opcode
`0x47` to `0x7F` and the UTF-16 function name. The guarded
`update_iec_ld_arithmetic_function` writer changes those two fields together.
XG5000 opened the generated file, rendered SUB, passed Check Program with the
baseline diagnostics, and preserved all seven decoded program payloads on
Save As. Seven ADD/SUB instances are identified in the supplied project. The
same three-operand record family includes one MUL and one DIV block. Their
opcodes are verified against their stored names (`0x48` and `0x63`). The
arithmetic writer now accepts ADD, SUB, MUL, and DIV. A generated ADD-to-MUL
edit at program 0 string offset `0xB7C` changed its opcode and three UTF-16
name bytes, with all other decoded program bytes unchanged. XG5000 opened it,
rendered MUL with the original operands and wiring, and Check Program reported
the baseline 0 errors, 1 warning category, and 42 messages. Native in-editor MUL/DIV
replacement and a Save As round trip have not been captured.

A native normally open to normally closed edit of the first `시작` contact in
program 0 changed only the `FF 06` record code to `FF 07` at decoded payload
offset `0x154`. The guarded `update_iec_ld_contact_kind` writer produced a
byte-identical program 0 payload; the other six payloads stayed unchanged.
XG5000 reopened the generated project, rendered that contact as normally
closed, and Check Program reported the baseline 0 errors, 1 warning category,
and 42 messages. QEMU's writable vvfat transfer crashed with a `vvfat.c` assertion
during this acceptance pass, so the generated file was transferred through a
read-only ISO image. This capture validates the NO/NC pair, not other contact
type changes.

A native insertion of existing BOOL variable `ON` on `조명` row L2 at x=7
split the x=4..91 long wire into x=4..4 and x=10..91, inserted a 23-byte
`FF 06` contact, changed the row record count from 3 to 5, and set the row
header's x coordinate to 7. The decoded program grew by 42 bytes; all later
bytes in program 0 and all six other programs stayed identical. The guarded
`insert_iec_ld_no_contact` writer produced byte-identical decoded payloads for
all seven programs compared with XG5000 Save As. XG5000 Check Program after the
native edit reported the baseline 0 errors, 1 warning category, and 42 messages.
This validates one contact-wire-coil row topology. Insertion into branched,
function, or other row layouts still needs native captures.

`insert_iec_ld_contact` now accepts the four decoded contact codes: normally
open, normally closed, rising-edge, and negated rising-edge. It uses the native
wire split above, requires the operand to resolve to BOOL, and validates the
circuit graph after insertion. XG5000 4.82.1 rendered, checked, and resaved a
generated normally closed `ON` insertion at x=7 with 0 errors, the baseline 1
warning category and 42 messages; Save As preserved every decoded ProgramData payload
byte for byte. Normally open and normally closed insertion and each contact
code change therefore have native evidence. Combined insertion captures for
rising-edge and negated rising-edge remain to be collected.

The insertion site decoder and writer now also support linear rows containing
more than one contact, with alternating contact and long-wire records followed
by a coil. Program 2 row L40 has two contacts and two wires. Inserting the
existing BOOL `도어열림` at x=19 split its second wire, grew the row from five to
seven records, and advanced the row header's rightmost contact coordinate from
13 to 19. XG5000 4.82.1 opened the generated file, rendered all three contacts
on that row, and Check Program reported 0 errors, 1 warning category, and 42 messages
in the result header. After Save As, all seven decoded ProgramData payloads
were byte-identical to the generated file. The generated and native captures
are under `/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-linear-contact/`.
The other five-record wire position and longer linear rows use the same
guarded record shape but do not yet have separate native mutation captures.

A native Delete of that inserted `ON` contact removed its 23-byte `FF 06`
record, changed the row record count from 5 to 4, and retained both split wire
records. An unchanged Save As control preserved every decoded program byte.
Delete also changed seven `FF 43` row display-geometry bytes outside the edited row in
program 0. The guarded `delete_iec_ld_no_contact` writer applies the local
record removal and count update; its decoded output differs from native Delete
only in those seven geometry bytes. XG5000 opened the generated file and Check
Program reported the baseline 0 errors, 1 warning category, and 42 messages. The
writer does not copy the seven changes without a general rule for them.

The same guarded deletion shape now accepts a normally closed contact already
in the supplied project. XG5000 deleted `현관도어닫힘` from program 2 row L40,
retained its two wire records, and lowered the row count from five to four.
Native deletion also changed four display-geometry header bytes in earlier
rows of that program. The generated deletion omits those four changes.
XG5000 4.82.1 opened the generated file, displayed the empty contact position,
and Save As preserved all seven generated ProgramData payloads byte for byte.
Check Program reported 3 errors, 0 warnings, and 36 messages for both the
native and generated deletions. This particular edit leaves a disconnected
circuit; the writer exposes it as an intermediate edit that needs a subsequent
wiring repair. Captures are under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-delete-original/`.

XG5000's F5 horizontal-wire command repairs that gap by inserting a 15-byte
`FF 01` short-wire record at x=13, raising the row count from four to five,
and resetting the row header coordinate to the first contact at x=1. The
guarded `repair_iec_ld_horizontal_wire` writer makes that exact mutation.
Starting from the native deleted-contact file, all seven generated ProgramData
payloads are byte-identical to the native F5 Save As. The repaired project
renders as one continuous wire and Check Program returns the baseline 0 errors,
1 warning category, and 42 messages. The native edit, generated edit, native resave,
and screenshots are under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-wire-repair/`.

The inverse `delete_iec_ld_horizontal_wire` operation removes a 15-byte
`FF 01` segment between two matching long wires and records a one-cell gap.
Only four sites in the supplied project pass the post-edit circuit graph:
program 0 L52 and L56, program 1 L6, and program 2 L36. Removing and
reinserting each one restores all seven ProgramData payloads byte-for-byte.
On the earlier native repair capture, deleting the inserted L3 wire reproduces
XG5000's native deleted file byte-for-byte. XG5000 4.82.1 also opened the
generated L52 deletion, visibly rendered the gap, reported 3 errors and 36
messages for the temporarily disconnected circuit, and completed Save As.
All seven ProgramData and PB50 payloads in that native resave were byte-
identical to the generated file. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-wire-delete/`.

For deletion inside longer linear rows, XG5000's Cell Delete command is the
safe native operation. It removes the selected contact, shifts later contacts
and connecting wire coordinates left by one cell, keeps the output coil fixed,
and updates the row header. Captures for both the last and middle contacts in a
seven-record row match `delete_iec_ld_no_contact_cell` byte for byte except for
the same four display-only geometry bytes changed by native Save As. XG5000
Check Program reports zero errors after both Cell Delete edits. Zero-length wire
fragments are excluded from the F5 repair-site detector because native F5 did
not make that longer-row shape valid. Native files and screenshots are under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-cell-delete/`.

The deletion-site decoder and linear-wire insertion writer now include all six
addressed contact record codes: normally open (`FF 06`), normally closed
(`FF 07`), rising-edge (`FF 08`), falling-edge (`FF 09`), negated rising-edge
(`FF 0A`), and negated falling-edge (`FF 0B`). The generic
`delete_iec_ld_contact` and `delete_iec_ld_contact_cell` writers require the
expected contact kind as a stale-edit guard. Generated insert-then-delete tests
cover all six kinds and validate the circuit graph after each operation.
XG5000 4.82.1 rendered and checked generated falling-edge and negated
falling-edge insertions in an existing multi-contact row with the project
baseline diagnostics, then Save As preserved every decoded ProgramData payload
byte-for-byte. Those captures are under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-six-contact-insertions/`.
XG5000 4.82.1 rendered and checked a generated rising-edge Cell Delete with 0
errors, the baseline 1 warning category and 42 messages, then preserved every decoded
ProgramData payload during Save As. Native in-editor Delete and Cell Delete
captures currently cover NO and NC; direct falling and negated-falling deletion
captures remain to be collected. The generated source, native resave,
and screenshots are under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-rising-cell-delete/`.

The contact-kind writer accepts all six addressed contact records, including
the four kinds present in the original smart home project, and the coil-kind
writer accepts all six coil records. Each changes only the guarded record code
and preserves the operand and position. The five representative kind changes
produce the same ProgramData bytes as direct generic-rung construction for
normally-closed/inverse, rising/rising, falling/Set, negated-rising/falling,
and negated-falling/Reset. XG5000 4.82.1 rendered, checked, and Save-As
preserved those constructed payloads, so the byte-identical kind-change output
has the same native-accepted encoding. The all-code evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-rung-kinds/`.
The earlier direct rising-edge to negated-rising-edge mutation capture remains
under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-contact-kind/`.

`edit_iec_ld_branch_segment` adds or removes one paired `BranchStart`/`BranchEnd`
segment between adjacent rows in an existing native row group. It rejects
function/comment rows for insertion. Final-segment removal is guarded to the
captured two-row group shape whose lower branch row contains contacts only.
Removing the middle x=6 segment between program 6 rows L3 and L4 reduced its
branch count from 86 to 85; adding it back reproduced the original ProgramData
byte for byte. XG5000 4.82.1 rendered the generated removal and reported one
expected disconnected-logic error, zero warnings, and 36 messages. Native Save
As preserved all seven generated ProgramData payloads byte-identically. Evidence
is under `/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-branch-segment/`.

Removing the final x=6 segment between program 0 rows L3 and L4 in XG5000
deleted the lower contact-only branch row, replaced the upper branch boundary
with a short wire, reduced the row count from 86 to 85 and the record count
from 289 to 286, and shifted all later row and function pin coordinates by one
row. The guarded writer reproduces the captured record stream except for three
opaque row-header cache bytes changed by XG5000's direct editing operation; the
writer retains their source values. The other six ProgramData payloads remain
byte-identical. XG5000 then opened the
writer-generated file, rendered the changed program, and reported the baseline
0 errors, 1 warning category, and 42 messages. Native Save As preserved all seven
generated ProgramData payloads byte for byte. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-final-branch-row/`.

The decoder identifies 29 opcode/name-verified EQ, GT, GE, LT, and LE blocks
in this project. `update_iec_ld_comparison_function` changes the paired opcode
and UTF-16 name in one guarded edit. A generated file exercised EQ-to-GT,
GT-to-GE, GE-to-LT, LT-to-LE, and LE-to-EQ at five distinct captured sites.
XG5000 4.82.1 rendered the program 0 L37 edit as `>`, checked all programs
with 0 errors, 1 warning category, and 42 messages, and saved the file.
All seven decoded ProgramData payloads and all seven PB50 tables in the native
Save As are byte-identical to the generated file. The bundled WASM reproduces
the generated file. Direct native in-editor mutations for each operator remain
to be captured. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-comparison-cycle/`.

## IEC local symbols

The supplied project has an empty global `<Symbols>` table and seven
program-local `PB50` symbol tables containing 101 records (15, 7, 14, 36,
23, 3, and 3). `iec_local_symbols()` decodes their names, mapped addresses,
storage classes, descriptions, and function instance type references. Mapped
`%MX`, `%IX`, and `%QX` addresses can be changed with the guarded
`update_iec_local_symbol_address` writer when the area stays fixed and the new
numeric bit is unique within the program. The writer permits a different
number of address digits and updates both the PB50 text field and its binary
bit number. For dotted addresses it
supports the captured base-zero, 64-bit-per-slot layout. XG5000 visibly
displayed a generated `조명.ON` `%MX8` to `%MX9` edit and Check Program reported
the baseline 0 errors, 1 warning category, and 42 messages. The seven decoded
ProgramData payloads are unchanged by this edit. XG5000 Save As of the
corrected two-field edit retained `%MX9` and numeric bit `9`; all seven decoded
ProgramData, OnlineUploadData, Symbols, and RungTableData payloads were
byte-identical to the generated file. An earlier text-only version also passed
Check Program but still displayed `%MX8`; that result was insufficient to
validate the address change.

Changing the same mapped BOOL from `%MX8` to `%MX100` grew its PB50 address
field by two UTF-16 units and updated the binary bit number to 100. The
rendered VS Code editor produced the generated file byte-for-byte. XG5000
4.82.1 displayed `%MX100`, checked all programs with 0 errors, 1 warning
category, and 42 messages, and retained the address and bit number after
Save As. The native save rewrote program 0's PB50 table, clearing an 11-unit
field in each of its 15 records; all parsed symbol fields remained equal.
It also changed four program 0 display/header bytes, three of which were
seen in the earlier native description Save As. The other six ProgramData
and PB50 payloads were byte-identical. This is native acceptance of the
address edit, with a visible normalization boundary in the saved file.
Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-address-length/`.

XG5000's Delete command on the mapped address of program 0 `ON` cleared its
address and storage class, set the binary allocation number to `FFFFFFFF`,
and cleared its width. The writer now applies that transition to mapped BOOL
symbols and can assign a supported bit address to an unallocated BOOL.
The generated unmap matched XG5000's parsed symbol fields, and mapping the
same symbol back restored all seven original PB50 and ProgramData payloads.
The rendered editor's Clear action emitted the exact generated bytes. XG5000
opened the generated unmap, displayed the blank address, checked all programs
with 0 errors, 1 warning category, and 42 messages, and completed Save As.
The direct native unmap left all seven ProgramData payloads unchanged; its
program 0 PB50 save also cleared an 11-unit field in all 15 records. Evidence
is under `/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-address-unmap/`.
The generated file's native Save As preserved all seven ProgramData payloads
byte-for-byte and all parsed local symbol fields; it made the same 330-byte
program 0 PB50 normalization while leaving the other six PB50 payloads intact.
The fixture-level clear-and-restore check covers all 65 mapped BOOL symbols:
52 `%MX`, 7 `%QX`, and 6 `%IX` across the seven programs. Each symbol's PB50
table and every ProgramData payload return to their original decoded bytes.
The native direct edit and generated-file Save As above cover `%MX8`; the
other address areas still need their own direct XG5000 mutation captures.
For a generated edit, the writer cleared program 0 `조명_1` `%QX10` and
program 1 `커튼_제어` `%IX0.0.7` together. XG5000 displayed the blank output
address, checked all programs with 0 errors, 1 warning category, and 42
messages, and saved the project. The native Save As retained every parsed
local symbol field and all ProgramData bytes except the four previously seen
program 0 display-cache bytes. It normalized program 0 PB50 by clearing the
same 11-unit field in all 15 records; program 1 and the other five PB50
payloads stayed byte-identical. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-address-io-unmap/`.

`rename_iec_local_symbol` updates the local
PB50 name and every classified contact, coil, and function operand reference
in the same program. A generated `ON` to `ON2` rename changed all six contact
references and XG5000 reported the baseline 0 errors, 1 warning category, and
42 messages. A native table-only rename left those operands unchanged and produced
four errors. Instance symbol renaming now also updates matching function-block
instance markers when the block name matches the local type reference. A
generated `INST3` to `INST4` rename reparses with both records synchronized;
XG5000 4.82.1 opened the generated file, displayed `INST4` as an `R_TRIG`
local instance, and completed Save As. The native file reparses with the
renamed function instance, and all seven decoded ProgramData payloads are
byte-identical to the generated file. XG5000's all-program Check Program run
reported zero errors; its result header showed 1 warning category and 42 messages,
while the summary line showed 27 warnings. The header counts match the
earlier baseline capture. Other
reference records are rejected until classified.

`update_iec_local_symbol_description` edits a program-local PB50 description
with an expected-current-value guard. It accepts up to 255 UTF-16 code units
and preserves all seven decoded ProgramData payloads in the generated file.
XG5000 4.82.1 displayed `Living room switch` on `조명.ON`, checked the generated
project with zero errors (1 warning category and 42 messages in the result header),
and retained the description after Save As. That native Save As changed three
bytes in program 0 at offsets 3208, 3871, and 4426; programs 1–6 remained
byte-identical to the generated file. The native capture is checked by the
ignored `xgi_native_description_save_preserves_edit` test.

`insert_iec_local_symbol` adds an unallocated primitive variable to a sorted
program-local PB50 table and increments its XML `Count`. The generated
`TEST_LOCAL` BOOL record exactly matched a variable created in XG5000. XG5000
4.82.1 opened the generated file, displayed the symbol as BOOL without an
address, and Check Program reported 0 errors, 1 warning category, and 42 messages.
After Save As, the complete 16-record local table was byte-identical to the
native-created table, and all seven decoded ProgramData payloads were
byte-identical to the generated file. Other primitive type choices share the
captured type-code layout but have not each received a native insertion check.
The generated, native-created, and native-resaved captures are kept under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-local-symbol/`.

`delete_iec_local_symbol` removes one PB50 record and decrements `Count` when
the owning ProgramData has no marker string matching the symbol name. It
rejects referenced symbols and stale names. XG5000 4.82.1 deleted
`TEST_LOCAL` from the generated-add project, checked all programs with 0 errors,
1 warning category, and 42 messages, and saved the result. Its 15-record table is
exactly the native-created 16-record table with that one 108-byte record
removed. The generated deletion restores the original smart home table and
leaves all seven ProgramData payloads unchanged. XG5000 also opened that
generated deletion, displayed the 15-row table, checked all programs with 0
errors, 1 warning category, and 42 messages in the result header, and saved it back.
The native-resaved local table exactly matches the native-deleted table.
ProgramData 1–6 are byte-identical to the generated file; program 0 differs
only at offsets 3208, 3871, and 4426, the same three native normalization
bytes observed in the description Save As. The captures are
`native_local_delete.xgwx`, `xgi-local-delete-generated.xgwx`, and
`native_generated_delete.xgwx` in the directory above.

PB50 also stores a primitive type ID immediately after each name. The sample
contains BOOL (`1`), INT (`7`), UDINT (`12`), TIME (`16`), and function
instances (`24`). XG5000's primitive selector runs from BOOL (`1`) through
DATE_AND_TIME (`19`); generated REAL (`14`) and DATE (`17`) files displayed
those types in the native variable table. The reader exposes the type ID,
name, and current allocation. `update_iec_local_symbol_type` changes an
automatic primitive variable's type and clears its allocation, matching native
INT-to-WORD and INT-to-DINT saves. The generated WORD file displayed correctly,
reported the same Check Program diagnostics as the native WORD file (0 errors,
2 warnings, 42 messages), and XG5000 Save As produced decoded payloads
byte-identical to the native WORD save. Native DINT reported 2 errors because
the project expressions depend on INT. Mapped BOOL type changes require a
separate address-removal workflow: XG5000 warned when WORD was selected while
`%MX8` remained mapped, so the type writer guards mapped symbols.

## IEC blank rows

XG5000 Ctrl+L on program 0 at L30 inserts an implicit blank IEC row without
adding payload bytes or a stored row record. It increments the row high-water
mark from 90 to 91, shifts every stored row after L30 by one, and moves all
decoded row, wire, branch, function block, pin, link, and expression Y
coordinates that cross that boundary by four native coordinate units. The
selected L30 row's secondary boundary coordinate also moves from L29 to L30.
`insert_iec_ld_blank_row` applies those guarded transformations. Its output
differs from the direct native edit only at three opaque `FF 43` cache bytes
already observed to normalize during other XG5000 edits.

Ctrl+D on the inserted empty L31 row closes the gap, decrements the high-water
mark, and shifts all later decoded coordinates back up. XG5000 retains the L30
secondary boundary coordinate created by Ctrl+L and normalizes four opaque
row-cache bytes. `delete_iec_ld_blank_row` matches every decoded structural
field in that native deletion and rejects occupied or still-referenced rows.

XG5000 4.82.1 opened both generated files. It rendered the added gap and the
closed-gap result, and Check Program reported the baseline 0 errors, 1 warning
category, and 42 messages for each. The detail pane listed 27 warning instances.
Native Save As preserved every decoded ProgramData
payload byte-identically after both insertion and deletion. The direct native
edits, generated files, native resaves, manifest, and screenshots are under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-blank-row/`.

Program 1 contains TON and MOVE function blocks. Its row shifts now walk the
validated function record structure to locate each pin coordinate. Inserting a
blank row after L5 moves MOVE from L6 to L7-L9. A standalone comment can then
be inserted at L6 with `insert_iec_ld_comment`, which copies a captured comment
group and replaces its text. XG5000 4.82.1 rendered that comment and shifted
MOVE, checked all programs with 0 errors, 1 warning category, and 42 messages,
and preserved every decoded ProgramData and PB50 local-symbol payload during
Save As. A separate program 0 L5 comment passed the same checks. Generated
files, native resaves, and screenshots are under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-group-copy/`.

## IEC simple rung creation

Creating a normally open `ON` contact at column 0 of the implicit L31 gap and
an `OFF` output coil at the right rail in XG5000 produces one 112-byte,
single-row group. The group contains a 35-byte row header and three records: a
normally open contact at native X coordinate 1, a long wire spanning 4 through
91, and an output coil at 94. The program group count changes from 35 to 36,
the new group is ordinal 17, and every later group ordinal increases by one.
Existing row coordinates do not move.

`insert_iec_ld_rung` reproduces that structure after confirming the row is an
unoccupied implicit gap, no graph area or edge crosses it, the insertion does
not split an existing group, and both operands resolve to BOOL. It accepts the
six addressed contact records (`FF 06` through `FF 0B`) and six coil records
(`FF 0E` through `FF 13`). `insert_iec_ld_linear_rung` remains a compatibility
wrapper for normally-open contact and output coil. Its output matches the
direct native edit except for four opaque row-cache bytes at
program offsets `0x0c88`, `0x0f1f`, `0x114a`, and `0x13f9` that XG5000 also
normalizes during other structural edits.

`delete_iec_ld_rung` accepts only a single-row group with the exact expected
contact kind, long wire, coil kind, coordinates, flags, and operand text. It
removes the group, decrements later group ordinals, and restores the generated
blank-row source byte-for-byte. All 36 contact/coil combinations pass generated
create/delete round trips. The extension exposes both kind selectors and both
operations in the blank-row inspector.

XG5000 4.82.1 rendered the generated rail-to-rail rung and Check Program
reported the project baseline: 0 errors, 1 warning category, 27 warning
instances, and 42 messages. Native Save As preserved every ProgramData payload
byte-for-byte. The direct edit, generated file, native resave, manifest, and
screenshots are under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-linear-rung/`.

Five additional generated projects pair normally-closed with inverse output,
rising-edge with rising-edge output, falling-edge with Set, negated-rising with
falling-edge output, and negated-falling with Reset. Together they cover all 12
supported record codes. XG5000 rendered every symbol, each all-program check
returned the same baseline diagnostics, and every native Save As preserved all
seven ProgramData payloads byte-for-byte. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-rung-kinds/`.

## IEC parallel contact on a simple rung

XG5000 4.82.1 added `NO OFF` below a captured simple `NO ON` to `OUTPUT OFF`
rung at program 0 L31. The valid branch keeps the top contact, long wire, and
coil in group 17; it inserts a 27-byte BranchStart after the top contact and
adds L32 with a `NO OFF` contact and 9-byte BranchEnd. The group row count
becomes two, its top row record count becomes four, and all later row
coordinates shift down one. The branch joins both paths at native X 3.

`insert_iec_ld_parallel_contact` accepts this captured one-row shape only,
checks the exact top contact and coil operands, and requires a BOOL variable
for the new contact. Its output has the same 14,554-byte program 0 payload as
the direct XG5000 edit. The only four byte differences are display-cache bytes
on unrelated rows that XG5000 normalizes during native Save As. The other six
program payloads match the native capture byte-for-byte. The extension exposes
the action on eligible simple rungs. Captures and the generated project are in
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-group-copy/`.
XG5000 reopened the generated file, rendered both contacts, and Check Program
reported 0 errors, 1 warning category, and 42 messages. Its Save As preserved
all seven generated ProgramData payloads byte-for-byte. The captured screenshot
is `generated_parallel_check.png`; the native output is
`native_resaved_parallel.xgwx` in that directory.
Removing its final branch segment in the library or VS Code webview removes
the lower contact row and restores the pre-insertion XGWX bytes exactly.

Program 0 L42–L43 has another captured final-branch shape: the upper row
contains `NO ON`, the branch start, `NC OFF`, and the output coil, while the
lower row contains `NO 자기유지1` and the branch end. XG5000 Ctrl+D on L43
removed the lower row and branch start, kept both upper contacts and wires,
and shifted later rows up. The writer now accepts this exact serial-contact
continuation and emits the same row and record data. Its direct native
counterpart differs only in 15 row-header display-cache bytes. XG5000 4.82.1
rendered the writer-generated L42 rung, checked all programs with 0 errors,
1 warning category, and 42 messages, and preserved all seven generated
ProgramData payloads byte-for-byte on Save As. The branch removal preflight
accepts this shape in two places. Captures are in
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-branch-wire-delete/`.

Program 3 L17–L18 has a related final branch with no intermediate wire: the
upper row has `NO` and `NC` contacts at x1 and x4, then a long wire and coil.
XG5000 Ctrl+D on L18 removed its lower contact and branch end, removed the
upper branch start, and retained the upper contacts, wire, and coil. The writer
now accepts that exact adjacent-contact continuation. Its generated program
matches the native edit except for ten unrelated row-header display-cache
bytes. XG5000 4.82.1 rendered the generated L17 rung, checked all programs
with 0 errors, 1 warning category, and 42 messages, and preserved all seven
generated ProgramData payloads byte-for-byte on Save As. The branch removal
preflight accepts this shape. Evidence is in
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-direct-contact-branch-delete/`.

Programs 4 (`가스제어`) and 5 (`보일러`) each have a final x6 branch after two
leading contacts. The upper row retains five serial contacts, a long wire,
and its coil; the lower row has two contacts and the branch end. XG5000's
direct deletion of program 4 L8 matched the generated edit byte-for-byte
across all seven decoded ProgramData payloads. The writer accepts this exact
record and coordinate layout for program 4 L7–L8 and program 5 L5–L6. The
combined generated file opened and rendered in XG5000 4.82.1 and passed
all-program Check Program with 0 errors, 1 warning category, and 42 messages.
Save As preserved all seven generated ProgramData payloads byte-for-byte.
Its generated and native files are in
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-x6-branch-delete/`.

Programs 1 (`커튼`) and 2 (`공동현관`) have x6 branches whose lower rows hold
a contact, a short wire, and a branch end. The writer now accepts their two
captured upper-row layouts and preserves the surviving short wires and
contacts. XG5000's direct program 1 deletion produced the same records and
row structure; nine row-header display-cache bytes differed. That direct
Save As also changed program 0, so it is only evidence for the program 1
edit. XG5000 rendered the combined generated project, checked all programs
with 0 errors, 1 warning category, and 42 messages, and preserved all seven
generated ProgramData payloads byte-for-byte on Save As. Captures are in
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-short-wire-branch-delete/`.

Program 3 (`엘리베이터`) has two final x24 branches with a lower output coil and
long wire at L51–L52 and L53–L54. Native Delete Line on L52 removed its lower
row and branch start, leaving the upper output intact. The writer matches that
group byte-for-byte and accepts both captured record layouts. XG5000 4.82.1
rendered and checked the generated project with both branches removed (0
errors, 1 warning category, 42 messages). Save As preserved all seven decoded
ProgramData and local-symbol payloads byte-for-byte. Captures are in
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-output-branch-delete/`.

Program 3 (`엘리베이터`) also has repeated three- and four-row x6 contact
branches. Native Delete Line on group 11 L26 removes the terminal contact,
short wire, branch end, and preceding branch start. It leaves the L24–L25
branch and shifts later IEC rows. The writer's group 11 bytes match the native
capture exactly. The same guarded terminal shape applies to groups 12–15;
XG5000 4.82.1 rendered and checked a generated project with all five terminal
rows removed (0 errors, 1 warning category, 42 messages). Save As preserved
all seven decoded ProgramData and local-symbol payloads byte-for-byte.
Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-three-row-contact-branch/`.

Groups 16 and 17 end with two addressed contacts and a branch end at x6.
Native Delete Line on group 16 L45 removes that lower row and the preceding
branch start. The writer's group 16 bytes match the native capture exactly.
The same guarded shape accepts group 17. XG5000 4.82.1 rendered and checked
the generated project with both terminal rows removed (0 errors, 1 warning
category, 42 messages). Save As preserved all seven decoded ProgramData and
local-symbol payloads byte-for-byte. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-two-contact-terminal-branch/`.

Group 16 also has a two-contact middle row at L44. Native Delete Line removes
the row and reconnects the branch from L43 to the surviving L45 row, which
shifts to L44. The guarded writer reproduces the native group bytes exactly
and also accepts the matching group 17 middle row. XG5000 rendered, checked,
and resaved the generated project with both edits (0 errors, 1 warning
category, 42 messages), preserving all seven decoded ProgramData and PB50
local-symbol payloads byte-for-byte. The capture is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-middle-contact-branch/`.

Program 3 groups 11–17 also have first lower two-contact rows beneath their
top rows. Native Delete Line at group 11 L25 removes that row and reconnects
the top branch to the surviving lower row. The guarded writer matches the
native group 11 bytes exactly. XG5000 4.82.1 opened, checked, and resaved a
generated project with all seven matching first lower rows removed (0 errors,
1 warning, 42 messages). All seven decoded ProgramData and PB50 local-symbol
payloads survived Save As byte-for-byte. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-first-lower-contact-branch/`.

Program 3 group 15 also has a middle row at L39 with one contact and a short
wire. Native Delete Line reconnects L38 to the surviving L40 row, which
shifts to L39. The guarded writer matches XG5000's group bytes exactly.
XG5000 opened, checked (0 errors, 1 warning, 42 messages), and resaved the
generated file; all seven decoded ProgramData and PB50 local-symbol payloads
remained byte-for-byte equal. The capture is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-middle-shortwire-branch/`.

The branch removal preflight now accepts 75 of the project's 192 vertical
segments. The reproducible audit is `cargo run --features write --example iec_branch_removal_audit -- /home/ne0ekspert/Downloads/smarthome_project_0225.xgwx`.
Of the 117 remaining segments, 84 need row-group rebuilds, 32 have
unverified record layouts, and 1 has a connected `FF` and lower output row.
That last row has a separate native-validated Delete Line action below.
Pass `--by-group` to see the remaining edits grouped by program and native
group index.

Program 3 group 20 L57 is a three-row branch with one contact on each
lower row and an x3 vertical feed. Native Delete Line removes L57, its
paired branch, and the branch-start record on L56. The guarded writer
matches the native decoded program payload apart from seven unrelated
row display-cache bytes that XG5000 refreshes on Save. XG5000 also opened,
checked, and resaved the writer output, preserving all seven decoded
ProgramData payloads byte-for-byte; Program Check reported 0 errors and
1 warning. The capture and acceptance example
are under `smarthome-iec-three-row-x3-terminal` and
`examples/iec_x3_terminal_contact_branch_acceptance.rs`.

The same group also supports Delete Line on its middle single-contact row
at L56. XG5000 reconnects the original L57 contact to the L55 x3 branch.
The guarded writer matches the native group bytes and all other decoded
program bytes apart from five unrelated row display-cache bytes refreshed
by Save As. XG5000 opened, checked, and resaved the generated output with
0 errors and 1 warning; all seven decoded ProgramData payloads remained
byte-for-byte equal. The capture is under `smarthome-iec-three-row-x3-middle`.

The remaining vertical-segment count includes connections through multi-row
function blocks. Native Ctrl+D at program 5 group 11 L25 removes the entire
first comparison block and its pins, then removes its output-pin row; it does
not act as an independent L24–L25 branch deletion. Program Check reports one
error for the resulting disconnected ladder. Treat that native result as a
block-deletion and repair reference, not as a valid branch edit. The capture
is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-function-branch-row/`.

Program 6 group 13 is another function-fed branch chain. Native Delete Line on
L47 removes the first `EQ`, its input/output expressions and link references,
the x6 and x12 contact branches, and the original L49 row. It reconnects the
x3 and x15 feeds to the next `EQ`. `delete_iec_ld_heating_chain_head` reproduces
the native decoded program 6 payload apart from 24 unrelated row display-cache
bytes refreshed by XG5000 Save As. The native deletion and the generated file
both passed all-program Check Program with 0 errors, 1 warning category, and
42 messages. XG5000 opened and resaved the generated project with all seven
decoded ProgramData payloads byte-identical. The VS Code Delete block action
exposes this captured site. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-p6-group13/`.

In the same group, native Delete Line on L51 and L55 separately removes the
second or third `EQ`, its input/output pin records, and the last row of that
four-row block. It retains the x3 and x15 branch feeds. The captured writer
operation `delete_iec_ld_heating_chain_middle` supports the corresponding
L50 and L54 blocks in the original project and rejects other layouts. Both
generated decoded program 6 payloads differ from the respective native Save
As only in 25 row display-cache bytes; programs 0–5 are byte-identical.
Native Check Program reported 0 errors, 1 warning category, and 42 messages
for both edits. The captures, screenshots, and generated files are in the
same evidence directory. XG5000 also opened and resaved each generated file;
all seven decoded ProgramData payloads in each generated/resaved pair are
byte-identical.

At L58, native Delete Line on the first `EQ` pin row L59 removes that block
but leaves an orphan x15 branch feed. XG5000 reports one input/output error
for that intermediate file. The guarded
`delete_iec_ld_heating_chain_x3_eq_repaired` operation removes the block and
four dangling x15 feed segments as one edit, preserving the x3 feed. XG5000
opened the generated project, rendered the edited group, reported 0 errors
on Check Program, and saved it. All seven decoded ProgramData payloads in
the generated and native-resaved files are byte-identical. Evidence is in
the same group 13 capture directory.

At L62, native Delete Line on the L63 pin row removes the comparison, both
x6/x12 contact branches, and the original L65 row. It retains the x15 feed
to the following comparison. `delete_iec_ld_heating_chain_contact_eq`
reproduces the structural change under a narrow layout guard. The generated
program 6 differs from the direct native capture in 37 row display-cache
bytes; programs 0–5 match. Both direct native and generated projects opened
in XG5000 and passed all-program Check Program with 0 errors. Evidence is in
the same group 13 capture directory. XG5000 resaved the generated project
with all seven decoded ProgramData payloads byte-identical.

At L66 and L70, native Delete Line on the first `EQ` pin rows (L67 and
L71) removes each comparison and its pins while preserving the x15 feed to
the following comparison. `delete_iec_ld_heating_chain_x15_eq` handles the
two guarded layouts. The generated program 6 differs from each direct native
capture only in 30 or 31 row display-cache bytes; programs 0–5 match.
XG5000 opened both generated projects, reported zero errors on all-program
Check Program, and saved each one. All seven decoded ProgramData payloads
in each generated/resaved pair are byte-identical. The packaged VS Code
webview's Delete block button emitted the validated bytes at both sites.
Evidence is in the same group 13 capture directory.
The x15-fed writer now accepts the same record shape after a prior deletion
shifts the next `EQ` from L70 to L69. Deleting L66 then shifted L69, or L70
then L66, produces identical decoded program bytes. XG5000 opened the
combined generated file, reported 0 errors and the baseline 27 warning
instances, and saved it with all seven decoded ProgramData payloads unchanged.
The rendered VS Code webview also emitted the combined file after two
successive Delete block clicks.

The head, two middle, contact-fed, and two x15-fed `EQ` deletions in program 6
group 13 now compose in either order. Applying the six edits from the first
block forward or from the last block backward produces identical decoded
program bytes (`generated_p6_g13_six.xgwx`). The bundled WASM test and rendered
VS Code webview both emitted that exact project after six successive edits.
XG5000 4.82.1 opened the generated file, displayed the edited program, and
reported 0 errors and the baseline 27 warnings on all-program Check Program
(`check_six.png`). Its Save As result (`native_resaved_p6_g13_six.xgwx`)
contains all seven decoded ProgramData payloads byte-identical to the generated
file. The screenshots and files are in the same group 13 evidence directory.
The x3-fed `EQ` at original L58 also composes with those six edits. Its guarded
repair removes the shortest contiguous x15 feed tail that restores the decoded
circuit graph. Deleting x3 last or between the earlier and later comparisons
produces identical seven-edit program bytes. The rendered webview emitted that
file after seven successive Delete block clicks, and XG5000 Save As preserved
all seven decoded ProgramData payloads byte-for-byte. However, native Check
Program reports one `L0000` input/output connection error at the remaining
contact-only heating network near L46. The local circuit graph accepts its
geometry but does not detect that missing output. The seven-edit file is an
intermediate, not a program-check-clean endpoint. Deleting the now-incomplete
group 13 network as a follow-up edit restores a clean result: XG5000 4.82.1
reported 0 errors and the baseline 27 warnings, and Save As preserved all
seven decoded ProgramData payloads byte-for-byte. The rendered editor's eighth
edit emitted that generated cleanup project exactly.

Program 0's 16-row comparison chain at L67–L82 is a distinct function deletion
case. Native XG5000 Delete Line at L69 removes the first `EQ`, its operand and
pin-link records, and the original L70 row. It preserves the x12 vertical feed
to the next `EQ` and flags the L68 branch start with a native marker value of
4. `delete_iec_ld_eq_chain_head` reproduces that captured edit under a narrow
group and record guard. The generated group's decoded bytes match the native
deletion capture; four earlier row display cache bytes differ because XG5000
refreshes them during Save As. XG5000 also opened and resaved the generated
file with all seven decoded ProgramData payloads byte-identical. This is a
function and row deletion action; it does not count as branch segment removal
in the 54/192 audit. Evidence is in
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-group33-row-delete/`
and `/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-eq-chain-head/`.

The same guarded branch edit now accepts all six addressed BOOL coil kinds,
and its inverse removes the lower row for those kinds. For the supplied
project's program 5 L4, the writer changed `OUTPUT %MX762` to `SET %MX762`
and added `NC %MX760` below `NO %MX761`. Its circuit graph and inverse edit
passed local checks; XG5000 4.82.1 opened the generated project, reported zero
errors in Program Check, and preserved all seven decoded ProgramData payloads
byte-for-byte on Save As. The bundled WASM produced the same project bytes.
Evidence is in `/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-parallel-set/`:
`generated_parallel_set.xgwx`, `native_resaved_parallel_set.xgwx`, and
`check_program.png`. The other five coil kinds share the writer guard but
have not been directly checked as parallel branches in XG5000.

The supplied project's program 5 has an eligible simple rung at L4:
`NO %MX761` to `OUTPUT %MX762`. The writer added existing direct BOOL address
`%MX760` in parallel on L5 and shifted the following network down. XG5000
rendered the new contact and checked all programs with 0 errors, 1 warning
category, and 42 messages. Its Save As preserved all seven generated
ProgramData payloads byte-for-byte. The VS Code form now accepts typed BOOL
operands, including direct device addresses, and suggests captured contacts
and declared BOOL symbols. The rendered webview's edit bytes matched the
generated project exactly. Files are `generated_original_parallel.xgwx`,
`native_resaved_original_parallel.xgwx`, `bpar-render.png`, `bpar-check.png`,
and `bpar-editor-after.png` in the capture directory above.

`insert_iec_ld_parallel_contact_kind` extends the lower branch contact to
`NO`, `NC`, `RISING`, `FALLING`, `NEGATED_RISING`, and `NEGATED_FALLING`.
The old API remains an `NO` wrapper. The extension exposes the same six kinds
in its parallel-contact form. On the original project, the generated
`NC %MX760` branch at program 5 L5 rendered with the slashed contact in XG5000;
Check Program reported 0 errors, 1 warning category, and 42 messages. Native
Save As left all seven decoded ProgramData payloads byte-identical. The rendered
webview edit bytes matched the generated file, and removing the branch restored
the original decoded program and local-symbol data. The six kinds pass local
branch shape and inverse checks; only `NC` has this new native branch acceptance.
Evidence is `generated_original_parallel_nc.xgwx`,
`native_resaved_original_parallel_nc.xgwx`, `bpnc-render.png`, `bpnc-check.png`,
and `bpnc-editor-after.png` in the capture directory above.

The top contact in the guarded simple-rung branch can also be any of the six
addressed kinds. In the supplied project's program 0, L2 is a
`RISING 스위치_1` contact to `OUTPUT 시작` coil. The writer inserted `NO ON` at L3,
retained the rising-edge top contact, and shifted later networks. XG5000
rendered the two-row branch and checked all programs with 0 errors, 1 warning
category, and 42 messages. Native Save As preserved all seven decoded
ProgramData payloads byte-for-byte. The webview emitted the same generated
bytes. The
library's branch removal restored every decoded program and local-symbol
payload. Evidence is `generated_rising_parallel.xgwx`, `rpar-render.png`,
`rpar-check.png`, `rpar-editor-after.png`, and
`native_resaved_rising_parallel.xgwx` in the capture directory above.

The supplied project contains six terminal MOVE groups whose top row is one
addressed contact, one long wire, and the function block, followed only by the
block's operand and reference rows. XG5000 Delete on program 3's first MOVE
retained the rising contact and removed the wire, block, and its L2-L3 pin
rows. The guarded `delete_iec_ld_terminal_function` writer reproduces that
structure and rejects stale block names or any group with unrelated child
records. The generated file rendered the same contact-only L1 and later MOVE
blocks, and Check Program reported the expected unfinished-network result of 1
error, 0 warnings, and 36 messages. XG5000 Save As preserved all seven
generated ProgramData payloads byte-for-byte. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-function-delete/`.

Native XG5000 insertion of `MOVE` after the retained first program 3 contact
restored L2-L3 with input `1` and output `%MW300`. The generated
`insert_iec_ld_terminal_move` group matches the native insertion; four row
display-cache bytes elsewhere in the program differ because XG5000 refreshed
them during its direct edit. XG5000 4.82.1 rendered and checked the generated
project with 0 errors, 27 warnings, and 42 messages. Native Save As preserved
all seven ProgramData payloads and IEC local-symbol summaries byte-for-byte.
The writer also accepts validated integer literals and writable `%MW` outputs
at this captured L1 site. A generated `2` to `%MW301` insertion rendered and
passed all-program Check Program with 0 errors, 28 warnings, and 42 messages;
Save As again preserved every ProgramData payload and local-symbol summary.
Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-terminal-function-insert/`.

Program 2 contains a standalone `WORD_TO_UDINT` group whose top row is a
one-cell wire and the function block and whose two child rows contain only its
operands and references. Native XG5000 Delete removes the entire group while
leaving L1-L3 as an implicit blank gap and keeping every later stored row at
its original row coordinate. The guarded
`delete_iec_ld_standalone_function` writer reproduces that layout, decrements
later group ordinals, and rejects stale names or any unrelated group records.
The generated file rendered the same blank gap, and Check Program reported the
project baseline of 0 errors, 1 warning category, 27 warning instances, and 42
messages. XG5000 Save As preserved all seven generated ProgramData payloads
byte-for-byte. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-standalone-function-delete/`.

The original program 4 has another three-row gap at L26-L28. Native XG5000
inserted `WORD_TO_UDINT` there with `%MW301` input and the existing `div_값`
UDINT output. The captured group differs from the program 2 L1 template only
in its group ordinal, row indices, vertical coordinates, and output text.
The positioned writer group matches the native group byte-for-byte. The full
decoded program differs in nine preexisting row display-cache bytes that
XG5000 refreshed. The library-generated project rendered the block in XG5000
4.82.1; Check Program reported 0 errors, 27 warnings, and 42 messages. Save As
preserved all seven generated ProgramData payloads and IEC local-symbol
summaries byte-for-byte.

Program 0 contains a connected single-output `FF` cell at L14. Its top row
also contains the leading contact, branch, wire, and output coil, while its
second row contains another branch path and coil. Native XG5000 Delete removes
only the function-block record and its output-link record. It keeps both rows
and every surrounding record. The guarded `delete_iec_ld_function_cell`
writer reproduces that shape, rejects stale block names, and requires each
affected row to retain at least one record. XG5000 4.82.1 rendered the generated
deletion with the blank function position at L14. Check Program reported the
project baseline of 0 errors, 1 warning category, 27 warning instances, and 42
messages. Save As preserved all seven generated ProgramData payloads
byte-for-byte. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-function-cell-delete/`.

The deleted L14 shape exposes one guarded connected-function insertion site at
raw x=4. Native XG5000 insertion adds a 108-byte `FF` function record using the
existing local `FF` instance and a 9-byte output-link record in the second row.
`insert_iec_ld_function_cell` reproduces those records, increments both row
record counts, and validates the instance type, exact gap, pin link, operands,
and circuit graph. Its decoded output differs from the direct native edit in
one row-header display-cache byte at program offset `0x636`. XG5000 4.82.1
rendered the generated `FF` at L14, reported the project baseline of 0 errors,
1 warning category, 27 warning instances, and 42 messages, and preserved all
seven generated ProgramData payloads byte-for-byte during Save As. Evidence is
under `/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-function-cell-insert/`.

Native Delete Line on the lower output row of this same two-row `FF` branch
(program 0, group 9, L15) removes the lower row, branch start, connected `FF`
block, and its output-link record. The new
`delete_iec_ld_ff_branch_output_row` operation reproduces the native group
byte-for-byte and rejects any other program, group, row, or stale group bytes.
The generated program differs from the direct native capture only in seven
unrelated display-cache bytes refreshed by XG5000. XG5000 4.82.1 opened and
rendered the generated file, checked all programs with 0 errors, 1 warning
category, and 42 messages, then Save As preserved all seven decoded
ProgramData and local-symbol payloads byte-for-byte. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-function-output-branch/`.

## Remaining editor work

`replace_iec_ld_group` now replaces an occupied network with a copy of another
complete network in the same IEC program. It composes the guarded group delete
and copy operations on a private document, so stale row indices or an invalid
destination leave the original file unchanged. On the supplied project, copying
program 0 L2 over L6 kept the seven-program count and validated circuit graph.
The rendered VS Code control emitted the generated file byte-for-byte. XG5000
4.82.1 rendered L6 with the copied L2 contact and coil, checked all programs
with 0 errors, 1 warning category, and 42 messages, and saved it. All seven
decoded ProgramData payloads and all seven PB50 symbol tables in that Save As
were byte-identical to the generated file. The source remains at L2; review
duplicated output operands or function instances before using a copy as
independent logic. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-network-replace/`.

The guarded `copy_iec_ld_group_to_program` operation copies complete
contact, coil, wire, branch, comment, and function networks between IEC LD
programs. It checks referenced local symbols, typed function operands, and
function instances against the destination program. The rendered webview
copied program 5 L4 (`NO
%MX761` to `OUTPUT %MX762`) into program 6 L8 of the original smart-home
project; its emitted file matched the library-generated file byte-for-byte.
XG5000 4.82.1 rendered the new L8 network and Check Program reported 0 errors,
1 warning category, and 42 messages. Native Save As preserved all seven
generated ProgramData payloads byte-for-byte. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-group-copy/`.

For a function copy, the writer cleared program 3 L1–L3 and copied program 2
L1–L3, a `WORD_TO_UDINT` block whose output uses the existing UDINT local
`변환`. XG5000 4.82.1 rendered the block in program 3, checked all programs
with 0 errors, 1 warning category, and 42 messages, and saved the result.
All seven decoded ProgramData payloads were byte-identical across the native
Save As. The program 2 PB50 payload became 308 bytes shorter, while every
decoded local symbol field in all seven programs remained equal. Program 2's
local-variable table was opened during this capture; the cause of the binary
normalization has not been isolated. The bundled WASM emitted the same bytes
as the library and parsed the native result. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-cross-function-copy/`.

`copy_iec_ld_group_to_program_with_locals` extends cross-program copying when
the source network uses primitive local variables absent from the destination.
It inserts the missing declarations, retains supported mapped BOOL addresses,
and copies the network in one atomic edit. It also clones missing captured
function-instance declarations and reserves a free automatic allocation. A
destination name with a different type or address and a duplicate mapped bit
are rejected. The VS Code cross-program form exposes this as **Copy missing
local variables**. On the supplied project, program 6 L0–L2 received program 0
L48–L50 plus its `OFF` `%MX7` and `ON` `%MX8` locals. XG5000 4.82.1 rendered
the copied branch and `MOVE` block, checked all programs with 0 errors,
1 warning category, and 42 messages, and saved the file. All seven decoded
ProgramData payloads and PB50 tables survived Save As byte-for-byte. The
bundled WASM emitted the same file. This native capture validates the two
mapped BOOL declarations. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-cross-local-copy/`.

Program 0's R_TRIG network at L32–L35 also copied into program 4 L22–L25
with its missing `INST_사본2` declaration. XG5000 checked the result with zero
errors and preserved all seven ProgramData payloads and the new program 4 PB50
table byte-for-byte on Save As. The unrelated program 2 PB50 table was
normalized, while its decoded symbols remained equal. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-cross-instance-copy/`.
`replace_iec_ld_group_from_program` now composes destination deletion and
cross-program copy on a private document, optionally adding missing locals.
Replacing program 4 network 14 with the program 0 R_TRIG network emits the
same file byte-for-byte as that XG5000-accepted capture. Omitting the required
instance fails atomically. The rendered VS Code replacement form also emitted
the same bytes (`webview-replace-after.png` in that capture).

The cross-program scan now includes Unicode operand names in IEC contact,
coil, and function-expression records. Previously it omitted `변환` from a
program 2 `WORD_TO_UDINT` network, allowing an unsafe copy into program 4
without that UDINT local. The guarded copy now rejects the missing local;
**Copy missing local variables** inserts it with a free automatic allocation
at bit 2688 before copying program 2 L1–L3 to program 4 L26–L28. XG5000
displayed both the declaration and block, checked all programs with zero
errors, and Save As preserved all seven ProgramData and PB50 payloads
byte-for-byte. The bundled WASM and rendered webview copy emitted the exact
generated file. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-cross-udint-copy/`.

Program 0 L2 has enough room in its long wire for two serial contacts. The
writer inserts `NO ON` at x25, then `NC 스위치_1` at x49 in the remaining wire.
`스위치_1` is already used as a captured contact operand in that program but is
absent from its decoded local and global symbol tables. The writer therefore
accepts exact reuse of an existing contact or coil operand as BOOL for contact
insertion; an unknown name is rejected. The generated `G2C.XGWX` had a valid
IEC circuit graph. XG5000 4.82.1 rendered both contacts, Check Program reported
0 errors and the baseline warning category, and Save As retained all seven
decoded ProgramData and PB50 local-symbol payloads byte-for-byte. The VS Code
webview produced exactly the generated bytes through two Insert contact edits,
and the scrolled preview showed the second contact. Evidence is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-group-copy/`.

The leading x1 contact on the captured L3 branch is also editable with native
Delete semantics. XG5000 removes its record and leaves the first cell empty.
The writer matches that change except eight unrelated row display-cache bytes;
XG5000 opened and resaved the generated file without changing any decoded
ProgramData payload. The capture is under
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-branched-contact-insert/`.
The inverse `insert_iec_ld_leading_contact` edit fills that empty x1 cell with
an addressed BOOL contact. Inserting `NO 시작` into the native deleted capture
matched all seven native XG5000 insertion payloads byte-for-byte. XG5000
rendered and resaved the library-generated file without changing any decoded
ProgramData payload.

XG5000 Cell Delete on the restored L3 x1 contact moves `NC 조명_1` from x4 to
x1 and inserts an `FF 01` short wire at x4, leaving the branch records in
place. The guarded `delete_iec_ld_contact_cell` writer matched the direct
native capture byte-for-byte across all seven program payloads. XG5000 then
opened the generated file, rendered the shifted contact, and resaved it with
every decoded ProgramData payload unchanged. The VS Code Cell Delete form
exposes this site.

XG5000 F3 on the resulting L3 x4 short wire replaces its `FF 01` record with
an addressed contact, leaving the row header and branch records unchanged.
`insert_iec_ld_short_wire_contact` matches the direct native `NO 시작` capture
byte-for-byte across all seven program payloads. XG5000 rendered and resaved
the generated project without changing any decoded ProgramData payload.

The captured two-row branch at program 0 L3 also supports native Delete Line
on its upper row. XG5000 removes that row and its paired branch-end record,
keeps the contact-only lower row at L3, and shifts later coordinates up.
`delete_iec_ld_branch_top_row` matches the direct native payload except nine
display-cache bytes; XG5000 opened and resaved the generated file without
changing any of the seven decoded ProgramData payloads. This operation can
leave an incomplete contact line, as the native editor does. Evidence is in
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-branched-contact-insert/`.

Program 6 group 3 has nested contact branches at L4 and L6. Native XG5000
Ctrl+D on either row passes Check Program with 0 errors, 1 warning, and 42
messages. `delete_iec_ld_nested_contact_branch_row` removes the selected
contact row, reconnects the outer x3 branch to the next row, removes the two
inner branch starts, and shifts later rows up. The generated files match the
respective direct native Save As payloads apart from row-header display-cache
bytes. XG5000 opened, checked, and resaved the generated L4 project without
changing any of its seven decoded ProgramData payloads. Native deletion of
the final output row L7 fails Check Program, so the UI exposes only L4 and L6.
Evidence is in `/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-p6-output-row/`.

Program 6 group 8 repeats this row shape at L21 and L23. Direct native
Ctrl+D on each row passed Check Program with 0 errors, 1 warning, and 42
messages. The writer matches both native decoded payloads except row-header
display-cache bytes. XG5000 opened, rendered, checked, and resaved the
generated L21 project with all seven decoded ProgramData payloads unchanged.
The editor exposes both additional rows through the same guarded action.

Program 6 group 14 has a chain of x3 contact branches below a function block.
Direct native Ctrl+D on L83 and L84 passed Check Program with 0 errors, 1 warning,
and 42 messages. `delete_iec_ld_chained_contact_branch_row` removes either
contact-only middle row, joins the neighboring x3 segments, and shifts later
rows up. Both generated decoded payloads match their native Save As across all
seven programs except row-header display-cache bytes. XG5000 opened, checked,
and resaved the generated L84 project with all seven decoded ProgramData payloads
unchanged. The editor exposes L83 and L84 through a guarded action.
Evidence is in `/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-p6-output-row/`.

Program 2 groups 7 and 8 each have a middle row containing only a branch end
and branch start, at L21 and L28 respectively. Native XG5000 Delete Line on
either row passed Check Program with 0 errors, 1 warning, and 42 messages.
`delete_iec_ld_empty_branch_row` joins the incoming and outgoing segments,
removes the row, and shifts subsequent coordinates. Both generated decoded
payloads match the respective native Save As across all seven programs except
row-header display-cache bytes. XG5000 opened, checked, and resaved the generated
L21 project with all seven decoded ProgramData payloads byte-identical. The editor exposes only these two captured
sites through a guarded action. Evidence is in the same capture directory.

The `ProjectType=2` payload shares the row/group header shape of the legacy
ladder format, but its y coordinate uses two bytes and its function records
have different layouts. Row and record boundaries and the captured function
body layouts are identified. `iec_circuit_graph()` maps the seven programs to
897 electrical edges, 881 non-overlapping occupied areas, 102 connected
geometry components, and all 196 typed function bindings. It rejects isolated
branch endpoints and mismatched pin-expression coordinates. The extension displays a
positioned preview of decoded comments, contacts, coils, horizontal wires,
vertical branch pairs, function blocks, and their input and output pin
endpoints, labels, types, and body geometry. Supported text labels open their
source editor from the preview. The graph does not yet assign execution
semantics to every component or distinguish every power-flow wire from a
data-flow wire. Further structural edits need native captures for multi-row
group construction and merging, multi-element insertion, additional function
insertion and connected-function deletion shapes, movement, and broader
function replacement, followed by generated-file open, Check Program, and
Save As comparisons. A complete IEC expression parser
is still needed for compound expressions. Function instance type changes and
mapped-variable type transitions remain undecoded.
