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

`select_cpu` recognizes the current XGI model as a no-op. XGI-CPUE, CPUS, CPUH,
CPUU, CPUU/D and CPUUN changes support validated SFC projects with captured
default parameters and empty I/O tables; see [SFC CPU validation](cpu-hardware-validation.md#sfc-cpu-changes-2026-10-08).
Other XGI models, custom parameters and configured modules remain guarded. On 2026-09-17, the
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
adjacent contacts. Inserting at the first cell of a long wire replaces that
cell with the contact and retains only the wire to its right. The row gains
one record, and the captured contact flags are preserved. A fixture regression
reconstructs 26 original native adjacent-contact layouts byte-for-byte and
checks successive insertions at x4 and x7 after the existing x1 contact.
This regression uses existing native payloads; it is not a new VM Save As run.

The insertion site decoder and writer also support linear rows containing
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
create/delete round trips. The extension's blank-cell picker now inserts an
individual contact or coil using `insert_iec_ld_single_element` below.

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

## IEC standalone contact or coil in an empty cell

`insert_iec_ld_single_element` uses the guarded empty-row group insertion to
create exactly one addressed contact or coil record at a specified contact-grid
coordinate. It creates no wire or other element. The six contact and six coil
kinds share the existing rung opcode mappings. Operands must resolve to BOOL;
coil destinations must also be writable. The row may be an implicit gap, the
first row after the stored program range, or an existing row with an unoccupied
cell. Insertion preserves the
row group and other records, including function-reference ordinals. It rejects
wires, contacts, coils, function bodies, operand cells, comments and branch
connections occupying the requested cell.

The extension accepts contact and coil commands in its native blank-cell text
prompt. Both contacts and coils use the selected cell. The row
can be an incomplete circuit until additional supported edits connect it.

The fixture test `xgi_empty_row_inserts_only_the_requested_element` checks all
12 kinds, atomic rejection, and preservation of other programs. A generated
project adds one standalone element after each of the seven programs. XG5000
4.82.1 opened it, rendered the contact-only and coil-only rows, and saved it as
SINGLEROUND. All seven ProgramData payloads and parsed IEC local symbols were
preserved byte-for-byte. This establishes open/render/save acceptance, not a
successful program check for the intentionally incomplete circuits. Evidence:
`/home/ne0ekspert/VMs/xg5000-win10/captures/smarthome-iec-single-element/`.

Consecutive insertion on an existing row was validated with 31 contacts and one
coil entered from right to left at every grid position. XG5000 4.82.1 opened the
generated project and checked all programs with strict type checking: zero
errors and the original 27 warnings. Native Save As preserved all seven decoded
ProgramData payloads byte for byte. Generated and saved files are in
`/home/ne0ekspert/VMs/xg5000-win10/captures/iec-editor-cells-20261002/`.

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


## Terminal IEC feed deletion and open-tail cleanup

`edit_iec_ld_branch_segment` accepts a terminal row containing a contact and
contiguous decoded wires, ending in one incoming branch. Its preceding row must
contain the incoming and outgoing branch records, optionally preceded by decoded
final function continuation markers (`FF 69`). Those markers are retained verbatim. Earlier function rows
and their reference ordinals remain unchanged. Removing the final segment
reproduces native Delete Line: it removes the feed row, its branch-start record
on the preceding row, and shifts later rows upward.

The native program 2 L29 capture keeps an open endpoint at L28 x18. This is a
structurally decodable intermediate circuit; native Check Program reports one
connection error. `iec_circuit_graph` still rejects open endpoints. The separate
`iec_circuit_layout` reader reports them in `open_branch_endpoints`, while still
rejecting overlaps, unmatched branch records and invalid function bindings.
WASM uses the layout reader so the incomplete native circuit stays visible.

Selecting the last segment of a single, unambiguous open tail removes its
vertical chain back to the first horizontal connection. Function descriptors,
pins and expression cells are retained, and the result must pass the strict
circuit graph validator. Whole-network deletion also accepts a structurally
valid incomplete source, but its result must pass that strict validator.

The extension's branch operation applies feed deletion and tail cleanup as one
undoable edit, labelled "Remove terminal feed and tail". Existing open tails use
"Remove open branch tail". The first repaired project passes XG5000 4.82.1
all-program strict checking with zero errors and the source's 27 warnings.
Native Save As preserves all seven decoded ProgramData payloads byte for byte.
The browser-emitted result matches the generated payloads exactly; its source
row-height caches differ from the native-normalized result at only three bytes.
Evidence and the reproducible acceptance tool are under
`/home/ne0ekspert/VMs/xg5000-win10/captures/iec-terminal-feed-20261002/` and
`examples/iec_terminal_feed_acceptance.rs`.

This does not enable general group splitting, forked/open branches, or deletion
of terminal feeds whose preceding row carries operand or non-final function references.

The combined suite also removes and cleans the long-wire feed at original program
2 L22 x21. XG5000 checked both terminal feed edits together with zero errors and
the baseline 27 warnings. Native Save As preserved all seven decoded program
payloads byte for byte (`TAILSUITE.xgwx` / `SUITEROUND.xgwx`).


### Terminal feeds sharing function continuation rows

The preceding row may retain final `FF 69` function continuation references to
left-hand blocks. The writer removes only the outgoing branch record, updates
the retained record count, and preserves every function binding. Source-project
Rust and bundled WASM tests cover program 3 L73 x18 and L67 x21.

Native Delete Line at program 3 L73 matches the generated edit except four
unrelated row-height cache bytes. Deletion followed by unique-tail cleanup from
the original workspace passes strict all-program Check Program with zero errors
and the original 27 warnings. Native Save As preserves all seven decoded
ProgramData payloads byte for byte. A real browser interaction emits the same
payloads as one undoable edit, using a mock VS Code host bridge. Evidence is in
`VMs/xg5000-win10/captures/iec-function-reference-feed-20261002`.
The combined L73 short-wire and L67 long-wire suite also passes native strict
all-program checking with zero errors and the baseline 27 warnings. All seven
decoded ProgramData payloads survive native Save As unchanged. Other
function-reference row shapes remain guarded.


### Current branch-removal coverage audit

`cargo run --features write --example iec_branch_edit_coverage -- SOURCE`
preflights every decoded vertical segment on a separate document clone without
writing the source. On the supplied smart-home workspace, 86 of 192 segments
pass writer preflight. The remaining 106 rejections comprise 73 middle-row group
rebuilds, 29 unsupported structural layouts, 3 final-row group rebuilds, and
1 final contact-row shape. Accepted preflight is not native acceptance of every
segment. Connected function groups account for most remaining guarded shapes.


### Arithmetic deletion beside an external branch spine

`delete_iec_ld_branched_arithmetic` removes a scalar four-row ADD/SUB/MUL/DIV
body, its three references and three operand expressions, and its feed wire
while preserving the neighboring contact/branch spine. It removes the final
continuation row and shifts later rows up once. Other retained record shapes
are rejected atomically. Stale block names are also rejected.

Native program 5 L18 Delete Line removes the ADD beginning at L15. The writer
matches that capture except two row-height display-cache bytes. A generated
suite deletes that ADD and program 6's ADD at L30 from the original workspace.
XG5000 opens and checks it with zero errors and the baseline 27 warnings;
Save As preserves all seven decoded ProgramData payloads byte for byte.
SUB/MUL/DIV use the same guarded writer shape but have no separate native
mutation captures. The editor's function marker supports Delete/Backspace and
the inspector uses the same writer. Chromium acceptance covers one undoable
Delete-key edit with the bundled WASM and a mock VS Code host bridge.
Evidence: `VMs/xg5000-win10/captures/iec-branched-arithmetic-20261002`.


### Branch selection on simple chained rows

Branch-segment removal reuses the native-validated chained-row deletion writers
when the destination is a branch-only row or a single-contact x3 branch row.
This also works inside groups containing functions. It reconnects the incoming
and outgoing spine and shifts later coordinates, preserving function bindings.
The source sites are program 2 L21/L28 and program 6 L83/L84.

All four outputs match the previously captured generated files byte for byte
across all seven programs. Direct native Delete Line captures differ only in
row-height caches. The program 2 L21 and program 6 L84 outputs also match their
native resaved generated files byte for byte. These checks reuse existing
native captures; no new VM check is claimed for this routing change.
Evidence: `VMs/xg5000-win10/captures/iec-chained-branch-selection-20261002`.


### Connected trigger deletion and horizontal gap repair

`delete_iec_ld_function_cell` also recognizes a scalar R_TRIG with a contact
or horizontal-wire predecessor, a following long wire, and one output
reference. It removes the body/reference while keeping every affected row
nonempty and preserving other functions, operands, wires and local symbols.
`repair_iec_ld_horizontal_wire` reconnects the resulting one-cell gap with the
native short-wire record, including gaps after contacts or short wires.

Native Delete on program 0 L20 removes R_TRIG and leaves three Check Program
errors. Native F5 at x7 repairs the circuit and passes strict all-program
checking with zero errors and the baseline 27 warnings. Generated deletion and
repair match both native structures except three row-height caches. The
generated repair separately opens, renders, checks, and saves in XG5000; all
seven decoded ProgramData payloads survive Save As byte for byte.

Source fixture checks also cover R_TRIG at L26 and L32, but those shapes do not
have separate native mutation captures. Browser Delete followed by wire repair
emits two undoable edits matching the generated file; its VS Code bridge is
mocked. Evidence: `VMs/xg5000-win10/captures/iec-connected-trigger-20261002`.
The maintained `iec_function_edit_coverage SOURCE` example audits every block
without modifying the source; its initial audit accepted 20 of 81 blocks before
this trigger extension. Neither preflight nor a structurally valid graph proves
native executable circuit semantics.


### Connected trigger insertion and restoration

`IecFunctionCellInsertionSite` now identifies its supported `function_name`,
which WASM exposes as `functionName`. `insert_iec_ld_function_cell` supports
R_TRIG in a one-cell gap before a wire feeding another function. Its output
reference is inserted before the retained next-row records. Only the two
stored rows gain records; other functions, rows, wires and symbols stay intact.
The requested instance must be declared with the same function type.

Deleting and restoring each of program 0's L20/L26/L32 triggers reproduces
all seven original ProgramData payloads byte for byte. The generated L20
restoration renders in XG5000 and passes strict all-program Check Program with
zero errors and the baseline 27 warnings. L26/L32 share the guarded writer
shape and have fixture round-trip coverage, without separate native captures.
The FF native insertion/deletion regression checks also pass.

Enter on the blank cell offers `R_TRIG Instance` with compatible instance
suggestions through the existing VS Code instruction prompt. Chromium verifies
Delete then Enter/restoration produces two undoable edits and restores every
program summary. Its VS Code host bridge is mocked.
Evidence: `VMs/xg5000-win10/captures/iec-trigger-restoration-20261002`.

Native Save As of the generated L20 trigger restoration preserves all seven
decoded ProgramData payloads byte for byte.


## Compound operand native boundary

A generated five-MOVE suite tested `%MW0 MOD 7 + 1`, `%MW0 >= 2`,
`%MX0 OR NOT %MX1 AND (%MW2 >= 2)`, `%MW0 XOR (%MW1 AND NOT %MW2)`,
and `TRUE XOR FALSE` as input operand text. XG5000 4.82.1 opens and renders
this file, but strict all-program Check Program reports ten errors: an invalid
input/output type and an undeclared variable for each expression. Native Save
As retains all seven generated ProgramData payloads byte for byte despite the
errors. Saving and preserving bytes therefore do not establish compilability.

`update_iec_ld_function_operand` rejects these comparison, Boolean, bitwise,
and MOD compound forms before changing the document. This is a native-proven
boundary, not implemented expression evaluation. The earlier `0+1` check/save
capture remains valid as a compatibility observation, but does not establish
arithmetic evaluation semantics. The native direct-variable chooser also
rejects `0+1` as an entry. A native-valid expression representation or lowering
into connected function blocks is still needed.

The instruction prompt groups parenthesized operands containing spaces,
keeps autocomplete within an unfinished group, and groups existing spaced
operands on reopening without changing their stored text. Unsupported forms
fail host validation while keeping the prompt open. The arithmetic classifier
also bounds nesting to prevent unbounded recursion.

Evidence: `VMs/xg5000-win10/captures/iec-expression-boundary-20261002`,
including the rejected generated project, native error screenshot, native
Save As, and the generator source used before adding the rejection guard.


## Row-preserving vertical wire edits

Native range Delete at program 0 L70, selecting the cells to the left of x12,
removes the paired branch records connecting L69 and L70. It retains both row
envelopes, all function operands/references, and the later EQ blocks. The native
result has two isolated geometry endpoints at x12, L69 and L70, but XG5000
4.82.1 strict all-program Check Program reports zero errors and the baseline
27 warnings. An open geometry endpoint is therefore not sufficient evidence
of a malformed native record stream or of a compiler error.

`edit_iec_ld_vertical_wire` toggles paired vertical records while preserving
all existing rows and their other records. It checks framing, geometry, typed
function bindings, stale selection, and counts. Deletion that would empty a
stored row remains guarded. Insertion joins existing electrical points in the
same group, allowing a removed gap to be reconnected. General new-row group
construction remains unfinished. Adjacent stored groups can now be joined
through the separately guarded connection writer described below.

The L69-L70 generated deletion matches native editing except row display-cache
bytes. The generated project separately opens, renders and passes native strict
all-program checking with zero errors and 27 warnings. Native Save As preserves all seven generated ProgramData
payloads byte for byte. Fixture verification
removes and restores each of the source project's 192 vertical segments and
recovers all seven original ProgramData payloads exactly. This is writer
coverage, not native Check Program acceptance of all 192 individual deletions.
The older 86/192 audit measures branch-row cleanup/removal, a different edit
operation. Its unsupported row-group rebuilds are not solved by retaining rows.

In the diagram, a vertical wire can be selected and deleted with Delete or
Backspace. Adjacent exposed endpoints show a gap control; double-click or Enter
reconnects them. Focus moves from the removed wire to that gap, then back to the
restored wire. The existing branch operations picker uses row-preserving removal
when its cleanup writer cannot handle the selected shared row, and addition can
reconnect function-row gaps. Nonstructural text/kind/operator writers now preserve the decoded layout and
typed bindings while allowing existing exposed endpoints. Structural writers
retain their separate closed-graph or captured-repair requirements. Browser acceptance uses the bundled WASM with a mock VS Code bridge.

Evidence: `VMs/xg5000-win10/captures/iec-vertical-rebuild-20261002`.


## Nonstructural edits with exposed endpoints

Text, supported contact/coil-kind and arithmetic/comparison-operator writers
now accept a structurally decoded open layout. A private helper verifies equal
record order, electrical geometry, occupied areas, connected components, exposed
endpoints and typed function bindings before committing. Only byte offsets,
labels and the guarded contact/coil kinds are normalized for this comparison.
Structural writers retain their existing validation requirements.

The program 0 group 33 L69-L70 x12 wire deletion followed by EQ IN2 `0` to
`123` and EQ to GE passes native strict all-program Check Program with zero
errors and 27 baseline warnings. Native Save As preserves all seven generated
ProgramData payloads byte for byte. Chromium acceptance exercises wire Delete,
block Enter edit and gap Enter repair with a mocked VS Code host bridge.

Evidence: `VMs/xg5000-win10/captures/iec-open-layout-text-20261002`.


## Connecting and separating adjacent stored networks

`connect_iec_ld_groups` joins consecutive stored groups at adjacent boundary
rows and inserts a vertical connection. It retains existing rows, records and
typed function bindings, removes one group envelope and renumbers later groups.
The selected boundary must have existing electrical points and must not
intersect a function body. Native F6 captures establish the endpoint flags and
row-end metadata for wire-bearing, branch-ending and contact-only upper rows.
A joined network is disabled when either source network is disabled; both
mixed execution orders were checked in XG5000. Existing individual record flags
are retained, and the new wire uses the captured endpoint style.

`split_iec_ld_group` separates adjacent stored rows only after no electrical
edge or function body crosses the boundary. Both resulting groups retain the
merged execution setting. Same-setting joins followed by wire deletion and
splitting recover the original program payloads exactly. A mixed-setting join
cannot recover the two earlier settings automatically.

The diagram's F6 key connects the selected cell's left boundary to the next
stored row, including an adjacent network. The branch picker offers these
connections and a **Separate networks** action for disconnected boundaries.
Each operation is one undoable edit. General new-row construction and joining
without an existing electrical point remain guarded.

The acceptance fixture covers program 0 L2-L3, program 3 L18-L19 and a new
contact on program 0 L66 connected to the EQ network beginning at L67. The
native reverse-mode capture at program 1 L8-L9 establishes execution-setting
behavior; that placement lacks an existing upper electrical point and remains
outside the writer's supported connection sites.

The combined generated candidate passes native strict all-program Check
Program with zero errors and the baseline 27 warnings. Native Save As preserves
all seven generated ProgramData payloads byte for byte. Browser acceptance uses
the bundled WASM with a mock VS Code bridge.

Evidence: `VMs/xg5000-win10/captures/iec-group-connect-20261002`.


## Extending a vertical wire into a blank row

`extend_iec_ld_vertical_wire` materializes the adjacent implicit blank row at
an existing group's end. The start must be an existing electrical point and
clear function bodies. It appends one 35-byte row envelope with a BranchEnd
and inserts the paired BranchStart in the upper row; later rows remain in
place. It checks decoded layout, row/record counts and unchanged typed function
bindings. The existing open-tail cleanup removes the wire and its branch-only
row. An internal branch removal preserves the final anchor cache when that
anchor record is retained.

F6 beside the selected cell now extends into an implicit blank row as well as
connecting stored rows. Wire Delete removes the branch-only endpoint row when
row-preserving removal cannot retain it. Existing shared-row wire deletion
continues to preserve those rows. Chromium acceptance uses bundled WASM and a
mock VS Code bridge, checks focus on the added wire and exact restoration after
Delete.

Program 0 L4-L5 x3 generated extension matches direct native F6 records except
four-byte row display caches. Native strict all-program checking reports zero
errors and 27 baseline warnings. Native Save As preserves all seven generated
ProgramData payloads byte for byte. The generated deletion restores the source
payloads exactly and matches native range Delete except the display caches.
Leading contact placement on the new branch-only row is supported as described
below; broader construction without existing endpoints remains unfinished.

Evidence: `VMs/xg5000-win10/captures/iec-new-branch-row-20261002`.


## Contact placement on a new branch row

The single-element writer accepts a leading contact in a row whose only record
is the branch endpoint at its right edge. It uses the decoded open layout for
that captured shape, inserts the contact before BranchEnd, preserves row-tail
metadata and applies the group execution flag to the newly created contact.
The result must pass the closed circuit-graph validation. Existing contact and
coil insertion paths retain their guards. All six supported contact kinds
retain BOOL operand checks.

Contact deletion sites include the corresponding two-record contact/branch-end
row. Deletion preserves its envelope and BranchEnd and verifies the newly open
endpoint. It recovers the preceding wire-only payload exactly. Diagram contacts
accept Delete and Backspace through supported native gap or cell deletion;
focus returns to the empty cell.

Generated program 0 L5 NO ON insertion and deletion match native records except
row display caches. Generated insertion renders and passes native strict
all-program checking with zero errors and 27 baseline warnings. Save As keeps
all seven generated ProgramData payloads byte for byte. All six contact kinds
are covered by local fixture checks; native insertion acceptance here is NO.
Chromium uses bundled WASM with mocked VS Code input to exercise F6, Enter
insertion, Delete, double-click NC reinsertion and Backspace.

Evidence: `VMs/xg5000-win10/captures/iec-branch-contact-20261002`.


## Coil placement on a new branch row

A row containing only an incoming x3 branch endpoint now accepts a writable
BOOL coil through `insert_iec_ld_single_element` when an empty cell at x4
or farther right is selected. Native XG5000 F9 places the coil at raw x94,
adds a long wire from x4 to x91, and retains the branch-end record first.
The writer matches this record order, execution flags and row anchor metadata.
Wider feeds are covered below. Occupied branch rows remain guarded.

`delete_iec_ld_terminal_coil` also accepts this captured three-record shape.
It removes the feed wire and coil together, retains the incoming endpoint,
and restores the original wire-only payload exactly. Stale operands and
non-writable or non-BOOL coil operands are rejected without changing the file.
The fixture check covers all six coil kinds and selection at x4, x7 and x94.

The extension's blank-cell instruction prompt uses the same writer. After a
coil insertion the cursor advances to x1 on the next row. Delete and Backspace
on supported coils remove the wire and coil and focus the emptied coil cell.
Rendered browser checks cover Enter, double-click, both deletion keys and
cursor placement through a mock VS Code bridge; these are not live extension
host acceptance.

`iec_branch_coil_acceptance` compares insertion/deletion with native captures
and verifies exact restoration of all seven original wire-only payloads.
The generated candidate passes native strict all-program checking with 0
errors and 27 original warnings. Native Save As preserves all seven generated
program payloads byte-for-byte.

Evidence: `VMs/xg5000-win10/captures/iec-branch-coil-20261002`.


## Wider branch-row coil feeds and partially constructed programs

Incoming branch-only rows at a contact-grid boundary now use that boundary to
place the coil feed, rather than assuming x3. The coil remains at raw x94;
the long wire starts at branch x + 1 and ends at x91. The inserted row anchor
is branch x - 2, and native deletion restores branch x - 1. Insertion preserves
the branch-end record before the wire and coil.

Coil placement/deletion can leave other branches open. The writer validates
the decoded layout and requires the open endpoint list to lose or gain only
the selected endpoint. All function-pin bindings must stay unchanged after
record offsets move. This permits constructing several branch outputs in
separate edits without requiring every branch to be filled at once.

The `iec_wide_branch_coil_acceptance` example generates x6 and x24 branches,
fills their L5/L13 outputs, deletes both outputs, and checks exact restoration
of all seven wire-only payloads. Optional native captures compare all four
intermediate states, masking only row display-cache bytes. The fixture checks
both edit orders, occupied-cell/stale-operand/type guards and unchanged other
programs. Browser checks cover both blank-cell Enter insertions and coil
Delete/Backspace with another branch open through a mock VS Code bridge.

The x93 endpoint beside a right-rail output remains guarded for coil placement:
its native insertion geometry is not established. The initial x6/x93 probe
opened, saved and checked with 0 errors and 27 warnings, but selecting visible
empty cells did not reproduce F9 coil placement at x93. This is not accepted
as native insertion evidence. Multi-open-branch leading contact construction
retains its earlier guard. Captured x3 branch function placement is covered below;
other branch boundaries remain guarded.

The completed native coil candidate and both-deleted open-row candidate each
pass strict all-program checking with 0 errors and 27 original warnings. All
four generated intermediate states match native payloads apart from row
caches. Separately opening and saving the generated coil candidate preserves
all seven generated ProgramData payloads byte-for-byte.

Evidence: `VMs/xg5000-win10/captures/iec-wide-branch-coil-20261002`.


## Scalar function placement with branch room creation

A final branch-only row with incoming boundary x3 now accepts a scalar block
at raw x7. The writer creates the minimum number of blank body rows when the
next network intersects the footprint, shifts later rows and pin links, and
extends the incoming branch through the body. MOVE needs two operand rows;
arithmetic and comparisons need three. Existing execution modes, unrelated
open endpoints, locals, and other program payloads are preserved. Disabled
function and expression records now decode with their native flag value 4.

The native experiment first rejected MOVE because its body intersected the
following network. Inserting two native blank rows made placement possible.
The generated MOVE matches that native edit apart from known row display-cache
bytes. Generated MOVE, ADD, and EQ candidates each pass strict all-program
checking with 0 errors, 27 original warnings, and 42 messages. Opening and
saving each generated candidate preserves all seven ProgramData payloads
byte-for-byte, without masking any bytes.

Deletion removes the block, its incoming feed, its owned expressions and
references, while retaining the vertical branch scaffold and row positions.
The result can accept another supported block with the same body height.
Deletion reconstructs the original payload by reinsertion before accepting an
edit, rejecting unsupported structures. Generated MOVE deletion matches native
Delete apart from two known row display-cache bytes. Tests delete and reinsert
all ten supported scalar instructions and restore the generated document
exactly. The generated two-pin and three-pin deletion candidates also pass
strict all-program checking with 0 errors and 27 original warnings. Native
Save As preserves all seven ProgramData payloads exactly for each deletion
candidate. ADD and EQ deletion candidates are byte-identical.

The `iec_function_room_acceptance` example generates placements, exercises
delete/reinsert, and optionally compares native placement captures. Fixture
checks cover later function pins, preserved disabled state, stale targets,
occupied cells, types, and unrelated payloads. Rendered checks through a mock
VS Code bridge cover Enter and double-click insertion, Delete/Backspace,
blank-cell focus, reinsertion, and the one-cell feed wire.

Only boundary x3 / raw x7 branch placement and its captured scaffold are
accepted. Wider boundaries, WORD_TO_UDINT placement, and
other function families remain guarded pending native evidence.

Evidence: `VMs/xg5000-win10/captures/iec-function-room-20261002`.


## Replacing scalar blocks with a different body height

The retained x3 / raw x7 scaffold now permits MOVE after a three-pin scalar
block. Native XG5000 preserves the surplus branch row, so the writer retains it
and its open endpoint. Deletion restores that taller scaffold and supports
reinsertion without changing later networks. Generated placement matches the
direct native edit apart from known row display-cache bytes; deletion differs
only in two previously established display-cache bytes. Independently opening
and checking the generated shorter replacement reports 0 errors, 27 original
warnings, and 42 messages. Native Save As preserves all seven generated
ProgramData payloads byte-for-byte, with no masking.

The reverse change extends a two-pin scaffold by one vertical row and inserts
an implicit blank row only when the following stored row occupies that space.
Later networks and pin links shift together. The generated MOVE-to-ADD result
is byte-identical to the previously native-validated FBADDGEN project, including
the complete document envelope and all seven ProgramData payloads. A manual
Ctrl+L/F6/F10 attempt rejected ADD as overlapping a vertical wire; that attempt
is recorded as a rejected placement, not native insertion evidence. The writer
uses the previously validated generated ADD topology.

Fixture tests exercise cross-height replacement for all ten scalar instructions
and prove that subsequent delete/reinsert restores the complete generated
file exactly. Rendered tests use a mock VS Code bridge and exercise shorter
Enter placement, taller double-click placement, Delete/Backspace, focus return,
and exact-byte reinsertion.

Evidence: `VMs/xg5000-win10/captures/iec-function-scaffold-20261002`.


## Atomic direct branch function replacement

`replace_iec_ld_branch_function` resolves the original block position, applies
supported branch deletion and typed scalar insertion to a document clone, and
publishes the candidate only when both operations succeed. Stale names, invalid
commands, incompatible types, and read-only destinations leave the complete
original document unchanged. The WASM replacement export accepts operands in
pin reference order.

The instruction prompt probes supported branch deletion before offering all
ten scalar commands. Enter and double-click on the block open that prompt;
other layouts retain their existing family/operand editors. Direct replacement
emits one editor change and restores focus to the replacement block.

The `iec_replace_branch_function` example resolves a target by program, row,
and raw X. Its MOVE-to-ADD and ADD-to-MOVE output files are complete-document
byte-identical to FBGROWGEN and FBSHORTGEN respectively. Those candidates have
native strict-check and exact seven-payload Save As evidence in the preceding
batches. This composed operation introduces no new native record topology.

Evidence: `VMs/xg5000-win10/captures/iec-function-direct-20261002`.

## Terminal timer deletion with a short feed

The terminal-function deletion recognizer also accepts the captured short-wire
feed. The supplied gas-control program's TON at L15 / raw x7 can be deleted
through the existing block Delete action. The writer removes its feed, body,
owned operands and references, retaining the leading contact, later row
positions, and the local timer instance declaration. Other program payloads
remain unchanged. Stale block names are rejected before writing.

The generated deletion matches native XG5000 Delete in all seven ProgramData
payloads after masking the established row display-cache bytes. Native Delete
leaves a contact-only unfinished rung and two blank operand rows. Strict
all-program checking reports one input/output error, zero warnings and 36
messages on that intermediate project. Deleting the block therefore does not
produce a completed, compilable rung. The following insertion operation can
complete the captured rung again.

Opening the generated project and running the same strict check produces the
same result. Native Save As preserves all seven generated ProgramData payloads
byte-for-byte without masking. The comparison against the direct native Delete
capture differs only at three row display-cache bytes in the edited program.

Evidence: `VMs/xg5000-win10/captures/iec-terminal-timer-20261002`.

## Typed terminal TON restoration

`insert_iec_ld_terminal_timer` builds the captured TON body and pin rows after
an enabled normal contact with two implicit blank rows before the next group.
It requires an existing, unused local TON instance, a TIME preset, and a
writable TIME elapsed destination. Unknown instances, incompatible types,
constants as outputs, occupied rows, and oversized operand markers are rejected
before writing. The declaration and later row positions remain unchanged.

Using the original instance and operands restores all seven original
ProgramData payloads byte-for-byte. The builder also accepts typed TIME literals
such as `T#2s`. Native insertion of that literal with the retained Timer instance
and local elapsed destination passes strict all-program checking with zero
errors, 27 original warnings and 42 messages. Native fresh insertion emits a
one-cell long feed and clears body flags; the builder preserves the original
short feed and body flags, so this direct capture is not an exact payload match.

The generated `T#2s` candidate independently opens and passes the same strict
check with zero errors, 27 warnings and 42 messages. Native Save As preserves
all seven generated ProgramData payloads exactly, without masking, and all
seven local-symbol tables remain unchanged.

The empty-cell instruction prompt offers `TON Instance Preset Elapsed` at the
supported site. Autocomplete filters the instance and TIME pins, and insertion
returns focus to the new block. Browser checks through a mock VS Code bridge
exercise Delete, Enter insertion, exact restoration bytes, block focus, and
Delete followed by double-click reopening.

Evidence: `VMs/xg5000-win10/captures/iec-terminal-timer-restore-20261002`.

## Scalar chain deletion and typed refill

`delete_iec_ld_scalar_chain_function` removes one scalar block from a pure
horizontal chain with shared operand rows. It removes only that block's body,
input/output expressions, and continuation references. Neighbor blocks, shared
row indices, local declarations, and other programs remain unchanged. Removing
the tail also trims trailing wires and marks the surviving block as terminal.
Unsupported mixed layouts and stale block names are rejected before writing.

The gas-control L18 MOVE and INT_TO_UDINT deletion outputs match direct native
XG5000 Delete captures apart from the established row display-cache bytes.
Deleting the first MOVE leaves an unfinished gap: strict all-program checking
reports three errors, zero warnings and 36 messages. Tail deletion leaves the
preceding MOVE complete; its direct native capture reports zero errors,
27 original warnings and 42 messages.

Typed insertion now supports INT_TO_UDINT with an INT source and writable UDINT
destination. Refilling a shared-row gap preserves record order; appending to a
surviving chain tail restores its nonterminal flag and the short-wire feeds.
Refilling either captured block with its original operands reproduces all seven
original ProgramData payloads exactly. Incompatible pin types fail atomically.
The webview exposes these deletion sites, offers typed conversion placement in
the empty-cell instruction prompt, and focuses the inserted block. Enter opens
its operand editor again. Browser interaction checks use a mock VS Code bridge.

Evidence: `VMs/xg5000-win10/captures/iec-scalar-chain-delete-20261002`.

Generated head and tail deletions and the restored chain independently open in
XG5000 with the expected strict-check reports above. Native Save As preserves
all seven ProgramData payloads for each candidate exactly, without masking.
The corrected tail refill is payload-identical to the head refill; native
restoration validation used the latter. The current source audit reports
28/81 function deletions passing writer preflight; the other 53 remain guarded.
This count does not imply native validation of every eligible site.

## Mixed-height horizontal scalar chains

The chain deletion writer recognizes shared three- and four-row envelopes for
scalar family 0x20 blocks with two or three pins. Owned expressions and body
references are counted against the target's decoded pin count. A trailing body
row that becomes empty is removed from the group's stored rows without moving
later ladder row indices. Interior gaps and mixed structural records remain
rejected. Surviving taller neighbors keep their complete body and pin rows.

Typed placement adds UDINT_TO_TIME (UDINT to TIME), TIME_TO_UDINT (TIME to
UDINT), and UDINT_TO_INT (UDINT to INT), using the captured opcode and pin
metadata. Each output must be writable and have the declared destination type.
The empty-cell instruction prompt filters autocomplete by these pin types.
Refilling the supplied gas-control L22 MUL/conversion, L30 conversion/DIV, and
L34 UDINT_TO_INT with original operands reproduces all original ProgramData
payloads exactly, including the original short-wire chain feeds.

Native XG5000 Delete captures for both blocks at L22 and L30 match the generated
deletions after masking only row display-cache bytes. Removing either leading
block yields three errors, zero warnings and 36 messages from the unfinished
gap; removing either trailing block yields zero errors, 27 original warnings
and 42 messages. The native taller-block deletion removes its now-empty final
stored row, while preserving the later row numbers. Current source audit:
32/81 function deletions pass writer preflight, with 49 still guarded. This is
structural coverage, not per-site native acceptance of every eligible block.

All four generated deletion candidates independently passed native strict
checking with the same reports as the native Delete captures above. Native
Save As preserved all seven ProgramData payloads for each candidate exactly,
without masking: 28 payload comparisons, zero differences. Leading deletion
gaps report three errors until refilled; trailing deletions report zero errors
and the original 27 warnings. The VM was cleanly powered down after capture.

## Deleting the last scalar-chain block

Standalone deletion accepts the captured one-cell ShortWire feed as well as
LongWire. This removes the last MUL (four stored rows) or TIME_TO_UDINT (three
stored rows) after deleting its neighboring tail, and the original gas-control
GE at L41. All subordinate records must belong to that block. Retained group
ordinals are renumbered while their row indices and coordinates stay unchanged.
For a contiguous final group ending at the program's total row boundary,
deletion trims the total row count to the deleted group's first row. Native
GE deletion changes that boundary from 45 to 41; terminal gaps with an
unverified boundary are rejected.

MUL and corrected GE payloads match native Delete exactly in all seven
programs. TIME_TO_UDINT differs only in two retained-row display-cache bytes;
all structural fields match. Original function-deletion preflight coverage is
33/81, with 48 remaining guarded. Additional post-deletion chain states are
covered by the sequence tests and native captures. Evidence:
`VMs/xg5000-win10/captures/iec-last-chain-block-20261003`.

The generated MUL and TIME_TO_UDINT deletions and corrected final GE deletion
independently pass native strict all-program checking with zero errors, 27
original warnings and 42 messages. Their Save As files preserve all seven
ProgramData payloads exactly, without masking.

### Staggered MOVE deletion with neighboring pin rows

The scalar deletion writer also supports a captured staggered MOVE at x4 below
an earlier scalar block at x16. In the supplied project this makes lighting L22,
L28, and L53 removable without deleting the neighboring function. It retains
the existing one-cell LongWire feed, removes only the MOVE body and its two
operand expressions and references, and drops the now-empty final stored row.
The affected retained rows use the native 39-unit height cache. Other record
bytes and all six other program payloads remain unchanged.

Direct native single-cell Delete captures for L22 and L53 match these outputs
except for redraw changes to unrelated row-height caches. Native Go to line
selects the whole row: Escape must clear that selection before selecting the
block. Earlier BTAILNAT/STAILNAT/STAIL2NAT captures removed overlapping neighbors
and are excluded from single-block acceptance evidence.

The generated deletions open and run strict all-program checking. Their retained
feed has no output while the block is absent, so XG5000 reports one invalid
input/output error, zero warnings, and 36 messages. This is an incomplete edit
state. Refilling L22 with MOVE restores zero errors, the baseline 27 warnings,
and 42 messages. Local tests cover all three eligible sites, preserve every
neighbor record byte, and keep deletion of the overlapping upper block guarded.
Native acceptance for the new deletion layouts covers L22 and L53; L28 is
structural coverage. Evidence: `iec-shared-tail-20261003` in the VM capture folder.

Both generated deletion files and the L22 refill survived native Save As with
all seven decoded program payloads byte-for-byte unchanged, without masking.
Browser checks exercised Delete for both layouts and Enter-to-refill for L22,
including exact generated file bytes and no JavaScript errors. These checks
use a mock VS Code bridge, not a live extension host.

### Completed branch-tail scalar editing

Delete, refill, and atomic scalar replacement now accept the captured completed
branch tail at x13 after a boundary at x6: boiler SUB L19, heating SUB L34,
and heating GT L42. Deletion removes the owned body, operand links, references,
and feed, retains the branch endpoint, and trims only empty owned pin rows.
Refill uses the native LongWire feed and creates no extra child branches.
The layout is restricted to the captured normal execution mode. Other modes,
unknown pin ownership, and overlapping records remain guarded.

Direct native SUB refill and GT deletion agree structurally with the writer;
their differences are confined to identified unrelated row-height caches.
Nine generated candidates survived native Save As with all seven ProgramData
payloads exact, without masking: 63 comparisons, zero differences. Deleted
tails report one unfinished-input/output error, zero warnings and 36 messages.
Original-operand SUB and GT refills, MOVE replacement, and EQ replacement pass
strict all-program checking with zero errors, the baseline 27 warnings and
42 messages. A heating SUB variant using a different destination passes with
zero errors and 29 warnings, including additional duplicate writes.

Rust and WASM tests cover all three sites and ten scalar commands, preservation
of neighboring records and other programs, and atomic rejection of invalid
output types. Browser checks cover Delete, Enter refill, Enter replacement,
and double-click replacement through a mock VS Code bridge. Horizontal LongWire
rendering now extends through its final occupied cell and connects to incoming
vertical branches, matching the native feed geometry.

Current original-project deletion preflight coverage is 39/81 function sites;
42 remain guarded. This count is structural coverage, not native acceptance
of every eligible site. Evidence is in
`VMs/xg5000-win10/captures/iec-completed-branch-tail-20261003`.

### Wider completed tails and disabled networks

Completed-tail editing additionally supports boiler GT L36 (boundary x9,
function x13), lighting EQ L79 (boundary x12, function x16), and lighting MOVE
L87 (boundary x9, function x16). Native single-cell Delete and typed refill
captures validate all three shapes. Direct comparisons agree in every
structural field and record; only identified display-height caches differ.
Other boundary, function-position, and execution-mode combinations remain
guarded. Evidence is in `iec-wide-completed-tail-20261003` in the VM capture
folder.

For the captured disabled lighting networks, native refill keeps zero execution
flags inside the function body while marking its feed and operands disabled.
New pin-row headers continue the retained EN row's cached end-Y sequence; real
pin and reference coordinates still use the actual rows. Deleting the final
MOVE trims the program total from 90 to 88 rows, retaining L87's branch endpoint.
Refill expands the boundary to fit either two or three pins. Rust tests cover
all six completed-tail sites and all ten scalar commands, including exact
replacement-then-deletion recovery and preservation of all other programs.
WASM and browser checks cover the three new Delete/refill interactions using
a mock VS Code bridge. Current original-project deletion preflight coverage
is 42/81 function sites; 39 remain guarded.

All seven generated candidates passed native open and strict all-program
checking. Boiler GT deletion reports one unfinished input/output error, zero
warnings and 36 messages; its original-operand refill restores zero errors,
27 warnings and 42 messages. Both disabled lighting deletions, their refills,
and the final MOVE-to-ADD replacement report zero errors, the baseline 27
warnings and 42 messages. Native Save As preserves all seven program payloads
for every candidate exactly, without masking: 49 comparisons, zero differences.


### Contact-fed scalar tails and the final heating comparison

Completed-tail editing also supports heating EQ L74 at boundary x15, function
x19, in normal execution mode. Native Delete retains the final BranchEnd and
removes its one-cell feed and owned pin rows; refill reconnects the endpoint.

Two captured contact-prefix layouts support scalar Delete, typed refill and
replacement: boiler MOVE L43 (BranchEnd x3, NC contact x4, block x13, normal
mode), and curtain MOVE L6 (NO contact x1, block x19, disabled mode). The
original mixed short/long wire feeds are validated before removal. Deletion
preserves the prefix records exactly and removes only the block, its feeds,
owned operands/references and empty pin rows. Native refill uses one LongWire
from the contact to EN. Disabled refill keeps zero internal body flags and
marks the feed and operand expressions disabled, as in the native captures.
Other contact-prefix, block-position and execution-mode combinations stay
read-only.

A three-pin replacement in the curtain network needs one extra row before
the next network. The writer composes the validated blank-row insertion with
scalar placement; the following network moves from L9 to L10. Native XG5000
renders the ADD replacement and the following network correctly. Boiler
replacement expands the final program boundary; deleting the replacement
restores the retained-contact state exactly. Rust tests exercise all ten
scalar commands on both contact-fed sites, preserving every survivor record
on deletion and all six other program payloads. Focused WASM and browser
checks cover deletion, Enter refill and Enter replacement. Browser checks use
a mock VS Code bridge, not a live extension host.

All eight generated candidates passed native open and strict all-program
checking, with logic, syntax, duplicate-coil and strict-type checks enabled.
Heating EQ and boiler MOVE deletion each report one unfinished input/output
error; their refills restore zero errors. Both ADD replacements and all three
disabled curtain edits report zero errors. Successful complete edits retain
the baseline 27 warnings and 42 messages. Native Save As preserves all seven
program payloads exactly for each candidate: 56 comparisons, zero differences,
with no masking. Direct Delete/refill comparisons also agree structurally;
only unrelated display-height caches differ in heating L3/L4/L12 and curtain
L1/L2/L3/L9/L10/L11/L12. Boiler direct comparisons are exact.

Evidence is in `iec-contact-tail-20261003` in the VM capture folder, including
native captures, generated files, strict-check screenshots, rendered ADD,
round-trip logs and local/browser checks. The validation source matches all
seven payloads in the original supplied project exactly. Current deletion
preflight coverage is 45/81 function sites; 36 remain guarded. This count is
structural writer coverage, not native acceptance of every eligible site.

### Scalar bodies on continuing vertical spines

Scalar Delete, typed refill and replacement now support boiler LT L32
(boundary x9, block x13, normal execution) and lighting EQ L71/L75
(boundary x12, block x16, disabled execution). Deleting the body preserves
all four spine rows and every BranchEnd/BranchStart record. Later functions
keep their row indices. A shorter MOVE uses three rows and retains the fourth
spine row; deleting the replacement restores the deleted state exactly.
Other boundaries, modes, contact prefixes and conversions remain guarded.

Native body deletion and refill captures establish the retained scaffold and
pin-cache layout. Disabled continuing bodies, feeds and operand expressions
carry execution flag 4. Direct comparisons agree on every record; only
unrelated display-height caches differ in boiler L36/L37 and lighting
L22/L26/L75/L76. Native shorter-MOVE deletion also preserves the scaffold.

All eight generated deletion, original refill, MOVE and ADD candidates for
boiler L32 and lighting L71 passed native strict all-program checking with
logic, syntax, duplicate-coil and strict-type checks enabled. Each reports
zero errors and 42 messages. Boiler deletion reports 26 warnings; the other
seven retain the baseline 27 warnings. Native Save As preserves all seven
program payloads exactly for every candidate: 56 comparisons, zero
differences, without masking.

Rust tests exercise all ten scalar commands at all three sites, exact
replacement/deletion recovery, surviving records, unrelated programs and
atomic rejection of an invalid BOOL output. Focused WASM tests pass for
continuing, completed and contact-fed tails. Browser checks cover Delete,
Enter refill and MOVE/ADD replacement at boiler L32 and lighting L71 using
a mock VS Code bridge. They are not live extension-host acceptance.

Evidence is in `iec-continuing-scalar-20261003` in the VM capture folder.
Current original-project deletion preflight coverage is 48/81 function sites;
33 remain guarded. This is structural writer coverage, not native acceptance
of every eligible site or complete IEC editor support.

### Heating comparisons on the wider continuing spine

The continuing-spine writer also supports normal-mode heating EQ L66/L70,
at boundary x15 and block x19. Native body Delete keeps all four spine rows
and the following comparison at L70/L74 respectively. Typed refill and
MOVE/arithmetic/comparison replacement reuse those rows. This adds refill
and replacement to two sites that previously had only a dedicated deletion
path that shifted rows. The generic body deletion now takes precedence in
the editor and preserves the native body-selection behavior.

Four native Delete/refill captures match every generated record. Only
unrelated display-height caches differ: heating L3/L4/L12 plus L70/L71
for the L66 edit, or L74/L75 for the L70 edit. No record differences are
masked. Rust tests cover all ten scalar commands at both sites, unchanged
survivor bytes and row indices, exact replacement/deletion recovery, all
other programs, and atomic rejection of an invalid BOOL output. Focused
WASM tests pass for continuing, completed and contact-fed tails. Browser
Delete, Enter refill, MOVE replacement, Delete recovery and ADD refill at
both sites match the generated bytes, with no JavaScript errors. Browser
validation uses a mock VS Code bridge rather than a live extension host.

All eight generated candidates (Delete, original EQ refill, MOVE and ADD
at each site) pass native strict all-program checking with logic, syntax,
duplicate-coil and strict-type checks enabled. They report zero errors and
42 messages. Delete and original refill retain 27 warnings; MOVE and ADD
report 28 warnings. Native Save As preserves all seven program payloads
exactly for each candidate: 56 comparisons, zero differences, without
masking. The validation source matches all seven original supplied-project
payloads exactly.

Evidence is in `iec-heating-continuing-20261003` in the VM capture folder.
Deletion preflight coverage remains 48/81: these two sites were already
counted by the older dedicated deletion path. The editability gain is typed
refill, replacement and preservation of the continuing rows. Other spine
layouts and execution modes remain guarded; full IEC editing is unfinished.

### Upper scalar branches fed by two contacts

Delete, typed refill and scalar replacement now support boiler LE L24 and
MOVE L40 (boundary x3, block x13), and heating LT L38 (boundary x6, block
x13). Both existing contacts and the continuing branch rows remain intact.
Replacing the three-row boiler MOVE with a three-pin function adds one
branch row, moving the lower MOVE from L43 to L44. Subsequent deletion
retains that expanded scaffold, matching native XG5000 behavior.

Nine direct native captures establish deletion, refill and branch growth.
Generated records agree exactly; differences are limited to unrelated
display-height caches in boiler L28/L29, L43/L44/L45 (L44/L45/L46 after
growth), and heating L3/L4/L12/L42/L43. The shifted boiler pin row retains
its native cached endpoint height. Broader contact prefixes, execution
modes and conversions remain guarded.

All thirteen generated candidates were opened, checked with logic, syntax,
duplicate-coil and strict-type checks enabled for all programs, and saved
by XG5000. Complete refills and replacements report zero errors and 42
messages: heating MOVE/ADD have 26 warnings; the others have 27. Heating
deletion reports zero errors, 25 warnings and 42 messages. Boiler deletion,
including deletion after ADD growth, leaves an open contact output and
reports one unfinished input/output error, zero warnings and 36 messages.
Native Save As preserves all seven program payloads exactly for all thirteen
candidates: 91 comparisons, zero differences, without masking.

Rust checks cover all ten scalar commands at these sites, survivor bytes,
row growth, unrelated programs and atomic rejection of invalid BOOL outputs.
The full Rust suite passes (93 tests; 96 ignored), as do four focused WASM
tests. Browser Delete, Enter refill and MOVE/ADD replacement checks match
the generated bytes at all three sites, with no JavaScript errors. These
browser checks use a mock VS Code bridge, not a live extension host.

Evidence is in `iec-upper-contact-20261003` in the VM capture folder.
Original-project function deletion preflight coverage is now 51/81 sites;
30 remain guarded. This measures structural writer coverage, not complete
IEC editing or native acceptance of every eligible site.

### Continuing scalar branch after NO and NC contacts

The writer now supports boiler GE L28 at block x13, after BranchEnd x3,
NO x4, NC x7 and BranchStart x9. Delete preserves those four records and
all four continuing rows. Typed refill and scalar replacement reuse the
scaffold; the following LT stays at L32. Normal execution is supported;
other contact combinations and conversions remain guarded.

Direct native Delete and GE refill agree with every generated record.
Only the unrelated display-height caches in boiler L32/L33 differ
(`0x24` to `0x27`). The validation source matches all seven program
payloads in the original supplied project exactly.

Four generated candidates (Delete, original GE refill, MOVE and ADD)
pass native all-program checks with logic, syntax, duplicate-coil and
strict-type checks enabled. Each reports zero errors and 42 messages.
Delete reports 25 warnings, GE refill 27, and MOVE/ADD 26. Native Save As
preserves all seven program payloads exactly in each candidate: 28
comparisons, zero differences, without masking.

Rust checks exercise all ten scalar commands, exact deletion recovery,
survivor bytes, other programs and invalid BOOL-output rejection. The
full Rust suite passes (93 tests; 96 ignored), along with four focused
WASM tests. Browser Delete, Enter refill, MOVE replacement, Delete recovery
and ADD refill match the generated bytes with no JavaScript errors. Browser
checks use a mock VS Code bridge, not a live extension host.

Evidence is in `iec-mixed-contact-20261003` in the VM capture folder.
Original-project function deletion preflight coverage is now 52/81 sites;
29 remain guarded. This is structural writer coverage, not complete IEC
editing or native acceptance of every eligible site.

### Scalar tail after an R_TRIG instance

Lighting ADD L32 at x19 now supports Delete, typed refill and scalar
replacement while retaining its leading NO contact, R_TRIG x10 instance
and shared control reference at L33. Delete trims the owned trailing L34/L35
frames without shifting later rows. It marks the retained trigger terminal,
sets the native retained row heights, and keeps its cached endpoint values.
Refill reconnects Q to EN with a native long wire and restores the trigger's
connected flag. The captured normal-mode prefix is supported; other trigger
prefixes, execution modes and conversions remain guarded.

The Rust and WASM scalar-chain replacement APIs combine guarded deletion and
typed insertion atomically. The instruction editor uses this path for supported
chain blocks, allowing changes between MOVE, arithmetic and comparisons.
Invalid output types or an unsupported placement leave the source unchanged.

Native Delete and original ADD refill agree on every generated record. Only
unrelated lighting display-height caches differ: L22 (`0x34` to `0x27`), L26
(`0x34` to `0x32`) and L37 (`0x3d` to `0x27`). All four generated candidates
(Delete, original ADD refill, MOVE and EQ) pass native all-program logic,
syntax, duplicate-coil and strict-type checks with zero errors and 42 messages.
Delete and EQ report 26 warnings; ADD refill and MOVE retain 27. Native Save As
preserves all seven program payloads exactly for each candidate: 28 comparisons,
zero differences, without masking. The validation source matches all seven
original supplied-project payloads exactly.

Rust checks cover all ten scalar commands, unchanged trigger instance and
reference, row retention, other programs, exact deletion recovery and atomic
rejection of incompatible or non-writable outputs and conversions. The full
Rust suite passes (93 tests; 97 ignored), as do three focused WASM tests and
14 extension tests. Browser Delete, Enter refill, MOVE replacement, Delete
recovery and EQ refill match generated bytes with no JavaScript errors. Browser
checks use a mock VS Code bridge, not a live extension host.

Evidence is in `iec-trigger-scalar-20261003` in the VM capture folder.
Original-project function deletion preflight coverage is now 53/81 sites;
28 remain guarded. This is structural writer coverage, not complete IEC
editing or native acceptance of every eligible site.

### MOVE consuming a staggered comparison result

Lighting MOVE L38 at x19 now supports Delete and typed MOVE refill after
EQ L37 at x10. Native Delete removes the body, its feed from EQ.OUT, its
two operand expressions and two references. It marks EQ terminal and
retains both comparison inputs, the comparison output reference, all four
shared rows and every later row number. Refill restores the long result
wire and EQ's connected flag. This captured normal-mode EQ scaffold accepts
MOVE only; arithmetic, comparison and conversion placement remain guarded.

Direct native Delete and original MOVE refill agree with every generated
record. Only unrelated lighting display-height caches differ: L22
(`0x34` to `0x27`), L26 (`0x34` to `0x32`) and L42 (`0x2f` to `0x27`).
The validation source matches all seven original supplied-project payloads.
Both generated candidates pass native all-program logic, syntax,
duplicate-coil and strict-type checks with zero errors and 42 messages.
Delete reports 26 warnings; refill reports 27. Native Save As preserves all
seven program payloads exactly in each candidate: 14 comparisons, zero
differences, without masking.

Rust checks cover row retention, shared comparison inputs, connected flags,
other programs, repeated deletion recovery and atomic rejection of unsupported
replacements or non-writable outputs. The full Rust suite passes (93 tests;
98 ignored), along with two focused WASM tests. Extension tests pass (124;
27 fixture-dependent tests skipped), and browser Delete and Enter refill
match the generated bytes with no JavaScript errors. Browser checks use a
mock VS Code bridge, not a live extension host.

Evidence is in `iec-staggered-comparison-20261003` in the VM capture folder.
The folder also contains a native EQ deletion capture for further work;
EQ head deletion/refill was not enabled by that batch; the next section covers it. Original-project
function deletion preflight coverage is now 54/81 sites; 27 remain guarded.
This is structural writer coverage, not complete IEC editing.


### Wired comparison head and retained MOVE consumer

Lighting L37 at x10 now supports deletion, refill and replacement with EQ,
GT, GE, LT and LE while retaining its MOVE consumer at L38 x19. These
comparisons accept two input operands; the BOOL result is connected directly
to MOVE.EN. The captured normal-mode scaffold is required. Other layouts
remain guarded. Deletion removes the empty stored L37 row and its leading
wires. Refill adds the native long prefix wire and inherits the retained row's
cached end coordinate. MOVE deletion and refill also work after this refill.

The instruction editor opens through Enter or double-click and presents two
source operands for this wired layout. Same-command operand changes use an
atomic replacement, allowing both sources to change from WORD to BOOL together.
Incompatible types, unknown symbols and unsupported placements leave the file
unchanged. The insertion counter includes the newly available comparison site.

Direct native deletion and refill agree with the generated records, except
for unrelated lighting display-height caches at L22, L26 and L42. Eight
native all-program checks cover deletion, EQ refill, GT/GE/LT/LE replacements,
MOVE deletion after refill, and EQ TRUE/FALSE inputs. The six complete comparison
candidates report zero errors, 27 warnings and 42 messages. MOVE deletion reports
zero errors, 26 warnings and 42 messages. Removing EQ alone leaves an unfinished
connection: both native Delete and the generated deletion report two errors,
zero warnings and 36 messages. This intermediate state must be refilled before
program use.

All eight native Save As outputs retain all seven decoded program payloads
exactly: 56 comparisons, zero differences, without masking. The source baseline
also matches all seven original supplied-project payloads. Rust validation
passes 93 tests (99 ignored) plus the focused fixture test; three focused WASM
tests and 18 extension tests pass. Browser interaction checks cover 11 edits,
including cancellation followed by Enter, actual blank-cell double-click,
consumer deletion/refill and atomic WORD-to-BOOL changes, with exact generated
bytes and no JavaScript errors. Browser checks use a mock VS Code bridge.

Evidence is in `iec-wired-comparison-20261003` in the VM capture folder.
Original-project function deletion preflight coverage is now 55/81 sites;
26 remain guarded. These counts describe structural deletion coverage, not
complete IEC editing or native acceptance of every eligible site.


### Native evidence for the curtain TON connected to a coil

The next guarded layout is curtain TON L1 at x19, instance INST13, with
preset T#5s, a shared contact branch on L1/L2 and a wired Q output to the
retained coil at x94. Its source network is disabled (group mode 1).
Native captures are saved as CTD1003 and CTR1003 in
`iec-curtain-timer-20261003` in the VM capture folder. Both pass all-program
logic, syntax, duplicate-coil and strict-type checks with zero errors,
27 warnings and 42 messages. This is native file compatibility evidence;
it does not establish live timer behavior or enable the writer yet.

Native Delete removes the body, preset expression and both references. It
retains the contacts, branch, feed wire x16, output wire x22 and coil on L1,
retains the contact and branch closure on L2, and drops the empty stored L3
row. The group has two stored rows, while the next network keeps its original
L4 index. Native row height caches become 50 and 39; the retained L1 cached
maximum x remains 19, and L2 maximum x becomes 5. Deletion therefore cannot
use the existing terminal-contact cleanup, which would discard branch records.

Native refill reuses the existing declaration without creating another
instance, recreates L3, and restores the same contacts, branch and coil. It
clears the three embedded pin-layout flags at body offsets 32, 110 and 164
from 2 to 0 and marks the TIME literal expression at offset 11 with 4.
The top height cache becomes 62; both lower caches become 39. It also updates
unrelated curtain display-height caches at L4, L10, L11 and L12. Comparison
logs retain these differences explicitly. The other six program payloads
remain byte-for-byte identical in both captures. No masking is used.

At the time of these reference captures, writer deletion/refill, insertion-site
metadata, instruction prompting and native acceptance of generated candidates
remained to be implemented; the next section records that implementation. Original-project function deletion preflight coverage remains
55/81; native reference captures alone do not increase it.


### Connected curtain TON editing

The connected curtain TON layout now uses a structural scaffold guard and an
explicit native builder in `iec_connected_timer_write.rs`. Delete removes only
the timer body, its preset and references, retaining the contacts, shared
branch, feed wire, output wire and coil. It drops the now-empty stored third
row without moving later networks. Refill recreates the native body, TIME
literal flags and pin rows. The source disabled mode remains unchanged.

The existing terminal-timer insertion API also accepts this scaffold, with an
empty elapsed destination. It requires an unused existing local TON instance
and a valid TIME preset, and rejects a supplied elapsed destination for this
captured layout. The instruction prompt offers two fields, Instance and Preset,
through Enter or double-click. Function deletion and timer insertion metadata
expose these operations through the existing WASM and editor command paths.

The Rust fixture test compares the entire generated edited network against
both native reference networks exactly, without masking, and checks preserved
row numbers, all other programs, declaration reuse, repeated deletion recovery
and atomic rejection of invalid instances, preset types and output placement.
Three focused WASM tests pass, including existing terminal TON and wired
comparison regressions. The full Rust suite passes 93 tests (100 ignored),
and 18 extension tests pass. Browser Delete, Enter refill, Delete and actual
blank-cell double-click refill produce four byte-exact edits. Existing timer
Enter and cancellation/reopening also pass with no runtime or console errors.
Browser validation uses Playwright with a mock VS Code bridge, not a live
extension host. Evidence is in `iec-curtain-timer-20261003` in the VM captures.

Native Save As preserves all seven generated program payloads exactly for
both deletion and refill (CTDG1003 and CTRG1003): 14 comparisons with zero
differences, without masking. Original-project function deletion preflight
coverage is now 56/81 sites; 25 remain guarded. This is deletion coverage,
not proof of complete IEC editing. The disabled source network has not been
validated against live PLC timer execution.

## Terminal TON with long feed wires (2026-10-03)

Native XG5000 Delete/refill captures cover a terminal TON at x22 after either
one normal contact or a normal contact followed by two NC contacts. Deletion
retains the contacts and their series feed, removes the timer and its two pin
rows, and leaves the next network's row index unchanged. The single-contact
layout also removes the intermediate feed wires. Both native deletion states
report one unfinished-rung error; they are editable intermediate states.

`iec_long_feed_timer_write` verifies the complete retained layout before
advertising x22 insertion. `insert_iec_ld_function(..., "TON", [instance, preset])`
uses an existing unused local TON instance and a TIME preset, with no ET
expression. The existing x7 terminal timer API and x19 connected timer remain
separate placement paths. Unknown instances, non-TIME presets and other
positions are rejected atomically. The VS Code prompt requests Instance and
Preset time, and supports Enter and double-click on the timer's empty cell.

The fixture-dependent `xgi_long_feed_timer_delete_and_refill` regression checks
whole-group equality against both native captures, repeat Delete/refill,
unchanged other programs and rejected edits. Browser checks use the real WASM
with a mock VS Code bridge; they do not exercise the native VS Code input box.

Generated Delete/refill candidates were opened, checked for all programs with
logic/syntax/strict type checks enabled, and saved under new native project
names. Both refills report 0 errors and the existing 27 warnings; both Delete
states reproduce the native reference's 1 error. All seven decoded program
payloads are exact after each Save As: 28/28 comparisons. Original-project
function deletion preflight coverage is now 58/81 sites, with 23 still guarded.
These figures do not establish full IEC editing coverage or PLC execution.

## Paired comparisons with a retained output branch (2026-10-03)

`iec_paired_comparison_write` supports the staggered comparison pairs at
x4/x13 and x7/x16, followed by a three- or four-segment vertical branch,
a result contact, and an output coil. The complete record order, canonical
function descriptors, two input bindings, three references, feed wires,
normal group modes, and branch geometry must match before an edit is offered.
This covers eight original GT/LE positions in the common-entry and elevator
programs. The standard comparison picker uses two typed inputs and a wired
BOOL result; incompatible inputs are rejected without changing the document.

Deleting the first comparison splits the input contact into its own group.
Deleting the second retains the first comparison and the output branch,
removes the inter-block feed, and marks the first output unconnected. Deleting
both removes the empty intervening row, preserves both contacts and the
output branch, and advertises both refill positions. Refill synthesizes the
missing row when necessary and merges the groups when the first comparison
is restored. Both deletion orders and both refill orders are covered.

Ten manual native captures establish the two common-entry layouts, including
both-deleted states. Generated elevator Delete/refill candidates and combined
refills for both programs were opened, checked with the existing all-program
logic/syntax/strict-type settings, and saved under fresh native project names.
All nine generated refills report zero errors and the existing 27 warnings.
The four generated elevator deletions report three errors for a missing head
or two for a missing tail, reflecting their unfinished circuits. All seven
program payloads remain exact after each of thirteen generated Save As cases:
91/91 comparisons, without masking differences.

Evidence is in `iec-paired-comparison-20261003` under the VM capture directory.
The fixture-dependent Rust regression checks exact native common-entry area
bytes, unchanged other programs, repeated editing, both operation orders,
and atomic type rejection. WASM checks cover all eight positions. Rendered
browser checks use the real WASM and a mock VS Code bridge for Delete, Enter,
double-click refill and prompt cancellation/reopening; the native VS Code
input box and PLC execution are not exercised by that browser harness.

Original-project function deletion preflight coverage is now 66/81, with
15 remaining guarded layouts. This is not a claim of complete IEC editing.


## Conversion pairs sharing a TON input branch (2026-10-03)

`iec_conversion_pair_write` supports the captured normal-mode six-row groups
with two NO input contacts, three vertical segments at x6, an
`INT_TO_UDINT` at x10, a `UDINT_TO_TIME` at x19, and a TON at x10 three rows
below. The complete shapes, canonical descriptors, reference ordinals,
expression records, and timer binding flags must match before writing. This
covers four original conversion positions in the boiler and heating programs.
The timer records and instance bindings are preserved byte for byte. Its
captured internal flags are not interpreted as a disabled network mode.

Deleting the first conversion retains the short input feed while the second
conversion remains. Deleting the second removes its long feed and marks the
first ENO unconnected. Deleting both removes the now-unused short feed while
retaining all six stored rows and the timer branch. Both refill orders
reproduce the same file. The x10 site accepts INT_TO_UDINT with an INT source
and writable UDINT destination; x19 accepts UDINT_TO_TIME with a UDINT source
and writable TIME destination. Other functions at these sites stay guarded.

Five manual native boiler captures establish individual Delete/refill and
both-deleted records. Nine generated projects were opened, checked with
all-program logic, syntax and strict type checks, and saved under new native
project names. Six generated refills report zero errors, the existing 27
warnings and 42 messages. The generated heating first-conversion deletion
reports three errors from the unfinished feed; its second-conversion and
both-deleted states retain zero errors and the existing warnings. All seven
decoded program payloads remain exact in every generated Save As case: 63/63
comparisons without masking differences. Evidence is in
`iec-conversion-timer-20261003` under the VM capture directory.

Fixture-dependent Rust and WASM checks cover four positions, repeated Delete,
both operation orders, unchanged timer records and other programs, atomic
operand rejection, and guarded unknown timer bindings/network modes.
Rendered tests use real WASM with a mock VS Code bridge: Delete, Enter and
double-click refill, cancel/reopen, and both-deleted refill. Thirty-two UI
edits and twenty saved-file comparisons pass without console errors at
1440x1000. Native VS Code input-box behavior and PLC execution were not tested
by that harness.

Original-project function deletion preflight coverage is now 70/81, with
11 guarded layouts remaining. Deletion coverage is not full IEC editing
coverage; shared timers, remaining MOVE/EQ layouts, and wider editing workflows
still require native validation.

### Shared conversion-branch TON editing (2026-10-03)

The boiler TON at L11/x10 and heating TON at L15/x10 now use the shared
conversion-group writer for Delete and typed refill. Native captures retain the
three vertical branch segments and four stored rows when the timer is absent;
refill restores the two pin rows and the terminal wire without replacing the
conversion pair. Conversion edits also work while the timer is absent.

The typed input accepts `TON <existing local TON instance> <TIME preset>`.
Invalid instance names, incompatible preset types, stale targets and unexpected
record shapes fail before document mutation. Fresh native timer bindings use
encoding zero; existing bindings with encoding zero or two remain preserved by
conversion edits. These encodings are not assigned an inferred meaning.

Rust fixture checks compare both native Delete/refill groups, a TIME literal,
and timer-first refill after all three blocks are deleted. They exercise all six
deletion orders against all six refill orders in both programs, repeated Delete,
and unchanged payloads in the other six programs. Run them using
`LIBXGWX_SMARTHOME_FIXTURE`, `LIBXGWX_SHARED_TIMER_CAPTURE` and optional
`LIBXGWX_SHARED_TIMER_OUTPUT` with the ignored test
`xgi_shared_branch_timer_delete_and_refill`.

The extension uses the same TON/TIME operand rules for insertion and supported
connected timer editing. Enter and double-click refill, cancellation/reopening,
literal replacement, and combined three-block edits pass browser checks with
real WASM and a mock VS Code prompt bridge. The native VS Code InputBox itself
was not exercised by these browser checks.

The source-file audit now accepts 72/81 original function deletions. Remaining
nine guards are lighting MOVE L48/x19, L52/x16, L56/x16, L59/x16, L84/x16;
common EQ L5/x7, TON L31/x19, MOVE L36/x19; and heating MOVE L80/x19.
Deletion coverage is narrower than full IEC workspace editability.

Native generated-file acceptance covered twelve Open / strict all-program
Check Program / Save As cases: individual timer Delete, variable refill, TIME
literal refill, all three blocks deleted, timer-only refill, and complete
combined refill, for both boiler and heating. All 84 decoded program payload
comparisons were exact without masking cached fields. The eight refill cases
reported zero errors, 27 existing warnings and 42 messages. The two individual
Delete cases reported one error and the two all-deleted cases reported two
errors, with zero warnings and 36 messages, in the unfinished networks.
The captured empty branch remains editable so it can be completed.

Evidence is retained outside the repositories in
`captures/iec-shared-branch-timer-20261003` under the local XG5000 VM directory,
including native saved workspaces, generated candidates, strict-check
screenshots, per-case payload comparisons, browser evidence and test logs.

## Enabled common-entry branch timer (2026-10-03)

The common-entry program's connected TON at L31/x19 now supports guarded
Delete, refill, preset replacement and replacement with an unused declared
local TON instance. The retained two-row input branch and coil are preserved.
Enabled groups retain the captured 50-unit lower row and TIME literal flag 0;
disabled curtain groups keep their separately captured row height and flag.
Cached row coordinates are preserved, including the native refill row's
relative cached coordinate. Unknown modes and binding encodings are rejected.

The ignored `xgi_enabled_connected_timer_delete_and_refill` fixture test uses
`LIBXGWX_SMARTHOME_FIXTURE`, `LIBXGWX_ENABLED_TIMER_CAPTURE` and optional
`LIBXGWX_ENABLED_TIMER_OUTPUT`. It compares native Delete and 5/7-second refill
groups, repeated edits, invalid inputs, unused instance replacement and combined
edits with the common program's other TON. Browser checks exercise Delete,
Enter and double-click refill, cancel/reopen, preset and instance replacement
using real WASM and a mock VS Code prompt bridge.

Function deletion preflight now accepts 73/81 original blocks. Eight guarded
blocks remain: lighting MOVE L48/x19, L52/x16, L56/x16, L59/x16 and L84/x16;
common EQ L5/x7 and MOVE L36/x19; heating MOVE L80/x19. This coverage does
not establish full IEC workspace editability.

Five generated Open / strict all-program Check Program / Save As cases validate
Delete, 5-second refill, 7-second refill, unused instance replacement and combined
refill of both common-entry timers. All 35 decoded program payload comparisons
are exact without masking cached fields. Four restored/edited cases report zero
errors, the existing 27 warnings and 42 messages. The timer-absent case reports
three errors, zero warnings and 36 messages, matching native unfinished-circuit
deletion. Scalar replacement to seven seconds produces the identical artifact
as seven-second refill and shares its native acceptance. Evidence is retained
in `captures/iec-enabled-branch-timer-20261003` under the local XG5000 VM directory.

## Common-entry contact-fed MOVE (2026-10-03)

The enabled common-entry MOVE at L36/x19 now supports Delete and typed refill.
Native Delete removes the fragmented feed, body and owned pin rows, retaining
only the leading contact. Native refill uses one continuous feed and preserves
the 50-unit contact row and its cached row-coordinate sequence. The original
and refilled canonical MOVE bodies share the same deletion path. Unknown
execution modes, body encodings and noncontiguous feeds remain guarded.

The ignored `xgi_common_contact_move_delete_and_refill` test uses
`LIBXGWX_SMARTHOME_FIXTURE`, `LIBXGWX_COMMON_MOVE_CAPTURE` and optional
`LIBXGWX_COMMON_MOVE_OUTPUT`. Native Delete and refill groups match exactly;
repeat Delete recovers the same workspace bytes. Invalid and incompatible
operands are rejected atomically, and the other six program payloads remain
unchanged. Other instruction names on this retained enabled-contact scaffold
remain guarded pending native placement evidence.

Operand editing now intersects the known scalar operand types of MOVE,
arithmetic and comparison blocks. Comparison BOOL outputs do not participate
in the input-type intersection; conversion source/destination types remain
separate. Supported scalar instruction prompts replace changed operands as one
atomic instruction edit so both sides can change type together. Browser checks
with real WASM and a mock VS Code prompt bridge cover Enter/double-click refill,
cancel/reopen, operand replacement, type-error rejection and repeated Delete.
The native VS Code InputBox itself is not exercised by the browser harness.

Original function deletion preflight now accepts 74/81 blocks. Seven remaining
guards are lighting MOVE L48/x19, L52/x16, L56/x16, L59/x16 and L84/x16;
common EQ L5/x7; and heating MOVE L80/x19. This is not full IEC editability.

Three generated Open / strict all-program Check Program / Save As cases cover
Delete, original operands and alternate operands `MOVE 1 %MW302`. All 21
program payloads match exactly without masking cached fields. Both restored
cases report zero errors, 27 existing warnings and 42 messages; deletion reports
one unfinished-circuit error, zero warnings and 36 messages. Atomic replacement
and alternate refill produce the identical artifact and share native acceptance.
Evidence is retained in `captures/iec-common-move-20261003` under the local
XG5000 VM directory, including generated/native workspaces, strict-check
screenshots, byte comparisons, hashes, browser evidence and test logs.

## Enabled lighting contact MOVE tails (2026-10-03)

Lighting MOVE L56/x16 and L59/x16 now support guarded Delete and typed refill.
Delete keeps the leading NO contact and removes the continuous feed, body and
owned operand/reference rows. Refill restores a canonical MOVE at x16 with a
single continuous feed from x4. Other instruction names on this retained
enabled-contact layout remain guarded pending native evidence.

Native Delete retains row field 17 as 39. A refill performed without closing the
native editor can reuse hidden pin-row caches that are absent from the saved
deletion file. Reopening that file discards those caches: newly created pin rows
use their actual row coordinates. The writer follows the reopened-file behavior.
The retained top-row cache remains unchanged. Independent native deletion and
reopened refill groups match the Rust output exactly for both original positions.

The fixture test `xgi_lighting_contact_moves_delete_and_refill` uses
`LIBXGWX_SMARTHOME_FIXTURE`, `LIBXGWX_LIGHTING_MOVE_CAPTURE`, and optional
`LIBXGWX_LIGHTING_MOVE_OUTPUT`. It checks native groups, repeated Delete,
unsupported instruction rejection, and preservation of the other six programs.
WASM tests additionally check atomic replacement and incompatible BOOL/WORD
rejection. Browser checks with real WASM and a mock VS Code prompt bridge cover
Delete, Enter refill, physical double-click refill, and cancel/reopen; all eight
emitted workspace edits match Rust artifacts exactly. The native VS Code
InputBox itself is not exercised by this harness.

Four generated Open / strict all-program Check Program / Save As cases preserve
all 28 decoded program payloads exactly, without masking cached fields. Both
refills report zero errors, 27 existing warnings and 42 messages. Each deletion
leaves one incomplete-circuit error, zero warnings and 36 messages until refilled.
Evidence is retained in `captures/iec-lighting-contact-moves-20261003` under the
local XG5000 VM directory.

Original function deletion preflight now accepts 76/81 blocks. Five guarded
positions remain: lighting MOVE L48/x19, L52/x16 and L84/x16; common EQ L5/x7;
and heating MOVE L80/x19. This count does not establish full IEC editability.

## Numeric MOVE literals for BOOL destinations (2026-10-03)

Lighting MOVE L63/x16 originally assigns numeric `0` to local BOOL `자기유지2`.
Direct operand editing, atomic instruction replacement and Delete/refill now
accept numeric `0` and `1` for MOVE BOOL destinations. This allowance is
contextual to MOVE; it does not broaden arithmetic, comparison or conversion
operand types. Other numeric spellings remain subject to their existing types.
Native strict checking accepts `1` and rejects `2` with type error L0706.

Four generated Open / strict all-program Check Program / Save As cases cover
refill with `0`, refill with `1`, changing the BOOL destination, and changing the
existing input to `1`. All 28 decoded program payloads match exactly without
masking cached fields. Each case reports zero errors, the existing 27 warnings
and 42 messages. The separate manual input edit changes row field 17 at L22,
L26, L63 and L67; that manual file is not used as a whole-payload equality oracle.

The fixture test `xgi_move_bool_numeric_literals_preserve_destination_editability`
uses `LIBXGWX_SMARTHOME_FIXTURE`, optional `LIBXGWX_BOOL_MOVE_OUTPUT` and optional
`LIBXGWX_BOOL_MOVE_CAPTURE` for all seven payloads in each native save. Tests
check atomic rejection of incompatible operands and preserve the other six
programs. Browser checks use real WASM and a mock VS Code prompt bridge for
Delete, Enter refill, physical double-click refill, editing, type rejection and
cancel/reopen. Five emitted edits match Rust artifacts exactly. The native
VS Code InputBox itself is not exercised by that harness. Evidence is retained
in `captures/iec-bool-move-20261003` under the local XG5000 VM directory.

## MOVE after parallel contacts (2026-10-03)

Lighting MOVE L48/x19 now supports guarded Delete, typed refill and atomic
replacement. Its two NO contacts between branch boundaries 3 and 6 remain
connected when the body and its feed are removed. Native Delete discards the
empty final reference row and retains the two contact rows. Refill recreates
that final row using its actual coordinate and preserves the existing row
caches. The supported placement has room before the following stored row;
final-program variants and other function names require additional native
evidence.

Native Delete and fresh refill groups match the writer exactly. The fixture
test `xgi_parallel_contact_move_delete_and_refill` uses
`LIBXGWX_SMARTHOME_FIXTURE` and `LIBXGWX_PARALLEL_MOVE_CAPTURE`, with optional
`LIBXGWX_PARALLEL_MOVE_OUTPUT` and `LIBXGWX_PARALLEL_MOVE_SAVE_CAPTURE`. It
checks repeated Delete, other instruction rejection and preservation of the
other six programs. WASM checks additionally cover atomic replacement. Browser
checks cover Enter, double-click, physical empty-cell double-click and prompt
cancel/reopen; five emitted edits match Rust artifacts exactly. The browser
uses real WASM and a mock VS Code prompt bridge, not the native InputBox.

Two generated Open / strict all-program Check Program / Save As cases preserve
all 14 decoded program payloads exactly without masking cached fields. Refill
reports zero errors, the existing 27 warnings and 42 messages. Deletion leaves
one L0000 input/output error, zero warnings and 36 messages until restored.
Evidence is retained in `captures/iec-lighting-parallel-move-20261003` under the
local XG5000 VM directory.

Original function deletion preflight now accepts 77/81 blocks. Four guarded
positions remain: lighting MOVE L52/x16 and L84/x16; common EQ L5/x7; and
heating MOVE L80/x19. Broader IEC placement, wiring and declaration editing
also remain incomplete.

A native deletion capture for lighting L52/x16 was retained for the following
writer change, described below.

## Upper staggered MOVE split and refill (2026-10-03)

Lighting MOVE L52/x16 supports guarded Delete, typed refill and atomic
replacement without moving the independently powered lower MOVE at L53/x4.
Native Delete splits the former network into a lone contact group and the
lower MOVE's three-row group. Both keep their end-Y caches and reset row field
17 to 39. Refill merges the groups and adds one long feed from the contact to
EN. It preserves the lower block, its operands and its final reference row.
The placement wrapper skips its usual blank-row insertion for this verified
scaffold, allowing the upper pins to use empty cells beside the lower block.

The fixture test `xgi_upper_staggered_move_delete_and_refill` uses
`LIBXGWX_SMARTHOME_FIXTURE` and `LIBXGWX_STAGGERED_MOVE_CAPTURE` for native
Delete and fresh refill comparisons. Optional `LIBXGWX_STAGGERED_MOVE_OUTPUT`
exports generated files; optional `LIBXGWX_STAGGERED_MOVE_SAVE_CAPTURE` checks
all seven payloads in each native Save As file. It checks atomic replacement,
repeated deletion, rejection of an unsupported ADD refill and preservation of
the other six programs. Other upper staggered layouts remain guarded.

Original function deletion preflight now accepts 78/81 blocks. Three guarded
positions remain: lighting MOVE L84/x16, common EQ L5/x7 and heating MOVE
L80/x19. This does not establish full IEC editability.

Two generated Open / strict all-program Check Program / Save As cases preserve
all 14 decoded program payloads exactly, without masking cached fields. Refill
reports zero errors, the existing 27 warnings and 42 messages. Deletion reports
one L0000 input/output error, zero warnings and 36 messages until restored.
WASM checks cover the new layout alongside the parallel-contact and BOOL MOVE
regressions. Browser checks cover Delete, Enter refill, double-click replacement,
repeated Delete, physical empty-cell double-click and prompt cancel/reopen; all
five emitted edits match Rust artifacts. The browser uses real WASM and a mock
VS Code prompt bridge, not the native InputBox. Evidence is retained in
`captures/iec-lighting-staggered-move-20261003` under the local XG5000 VM directory.

## Upper MOVE on continuing contact spines (2026-10-03)

The six-row disabled network containing two MOVE blocks can now delete and
refill its upper x16 MOVE while retaining both left contact spines, the contact
at x10, and the lower MOVE three rows below. Native Delete preserves all six
rows and resets their height field to 39. Fresh native refill adds one long
feed at x13 and uses flag 4 on the upper MOVE and its operands, preserving the
existing contact, branch, lower-block and cached end-Y bytes.

`iec_upper_contact_move_write` validates the entire relative layout before
changing it. The fixture test `xgi_upper_contact_move_delete_and_refill` uses
`LIBXGWX_SMARTHOME_FIXTURE` and `LIBXGWX_UPPER_CONTACT_MOVE_CAPTURE`; it compares
the affected group with native deletion and fresh refill captures, checks
atomic replacement and repeat deletion, and preserves earlier networks and
other programs. Two focused WASM tests pass. Browser interaction with real
WASM and a simulated VS Code prompt bridge emitted five edits matching Rust
output. Native VS Code InputBox interaction is not covered by that harness.

Original function deletion preflight accepts 79/81 blocks. This is not full
IEC editing coverage. Both generated files open and pass strict all-program checking with zero
errors and the existing 27 warnings. Native Save As preserves all 14 decoded
program payloads exactly. Evidence is stored
locally in `captures/iec-lighting-upper-move-20261003` under the XG5000 VM
folder; the VM transfer volume contains the generated delete/refill projects.

## Comparison result wired to a terminal coil (2026-10-03)

A comparison at x7 can now be deleted and refilled while retaining its result
wire at x10 through x91 and terminal coil at x94 on the first data-pin row.
Native Delete removes the power feed, body, input expressions and references,
leaving one stored row. Its cached maximum cell remains x7. Fresh native
refill recreates four rows, normalizes the power feed to one long wire, retains
the coil row's field-17 value 50, and preserves the native end-Y cache sequence.

`iec_coil_comparison_write` validates this relative layout and exposes the
vacant comparison position through wired-comparison insertion sites. The
shared prompt accepts two input operands. The fixture test
`xgi_coil_comparison_delete_and_refill` uses `LIBXGWX_SMARTHOME_FIXTURE` and
`LIBXGWX_COIL_COMPARISON_CAPTURE`; it matches native deletion and fresh refill,
checks atomic replacement and repeat deletion, and preserves other networks
and programs. Optional `LIBXGWX_COIL_COMPARISON_OUTPUT` produces Delete, EQ
refill and GT replacement projects for native acceptance. Optional
`LIBXGWX_COIL_COMPARISON_SAVE_CAPTURE` compares their decoded programs with
native Save As captures.

The native manual refill restores the original local UDINT variable and
numeric comparison value and passes strict all-program checking with zero
errors and the existing 27 warnings. Original function deletion preflight now
accepts 80/81 blocks; heating MOVE L80/x19 remains guarded. Two focused WASM tests pass. Browser interaction emits five edits matching
Rust output, using real WASM and a simulated VS Code prompt bridge.
Generated EQ refill and GT replacement pass strict all-program checking with
zero errors, the existing 27 warnings and 42 messages. Deletion retains an
unconnected coil and reports L0000 and L0401 until refilled. Native Save As
preserves all 21 decoded program payloads across the three generated cases
exactly, including cached fields. The fixture also passes with
`LIBXGWX_COIL_COMPARISON_SAVE_CAPTURE` enabled. Evidence is stored in
`captures/iec-common-coil-comparison-20261003` under the local XG5000 VM folder.
Deletion coverage is not full editability.

## MOVE after a six-row contact mesh (2026-10-03)

Native Delete of heating MOVE L80/x19 retains all six contact rows, including
the continuing left branch and the four shorter parallel branches. All six
height fields become 39. Fresh native refill uses a long feed at x16,
canonical MOVE at x19, its input at x16 and output at x22, retaining the end-Y
cache. The vacant row maximum fields are 15, 14, 3, 3, 3, 2; fresh refill uses
19, 22, 19, 3, 3, 2. The native refill of `0` into `%MW129` passes strict
all-program checking with zero errors and the existing 27 warnings.

`iec_contact_mesh_move_write` recognizes the complete relative layout and
preserves contact operands and branch bytes. The fixture test
`xgi_contact_mesh_move_delete_and_refill` uses `LIBXGWX_SMARTHOME_FIXTURE` and
`LIBXGWX_CONTACT_MESH_MOVE_CAPTURE` to compare the affected network with fresh
native captures and check atomic replacement, repeat deletion and unchanged
surrounding programs. `LIBXGWX_CONTACT_MESH_MOVE_OUTPUT` emits generated Delete
and refill projects. `LIBXGWX_CONTACT_MESH_MOVE_SAVE_CAPTURE` enables comparison
with their native Save As captures when available.

All 81 original function deletions now pass writer preflight. Three focused
WASM tests pass. Browser checks cover Delete, Enter refill, double-click
replacement, repeated Delete, physical blank-cell double-click and prompt
cancel/reopen; five emitted edits match Rust files exactly. These checks use
real WASM with a simulated VS Code prompt bridge, not the native InputBox.
Generated refill passes strict all-program native checking with zero errors
and 27 warnings; deletion reports one L0000 for the incomplete network until
refilled. Native Save As preserves all 14 decoded program payloads exactly.
The fixture passes with its Save As capture gate enabled. Evidence is stored in
`captures/iec-heating-contact-move-20261003` under the XG5000 VM folder.

Full IEC editability remains incomplete. A fresh project audit finds only
20/284 contacts published as deletion sites. All 192 vertical segments support
row-preserving removal preflight; 86 support branch cleanup removal. The UI
already falls back to row-preserving removal. These counts do not establish
native acceptance of every branch edit. Native Delete and fresh refill of
the leading contact at heating L80/x1 are captured in
`captures/iec-contact-mesh-contact-20261003` for the next contact writer change.

### Leading contact in the six-row heating mesh

Heating L80/x1 now supports contact Delete and blank-cell refill through the
leading-contact writer and the general single-element insertion API. The edit
preserves the MOVE at x19, its original short feed at x16, both operands, all
six branch rows and all surrounding networks. Only the selected contact record,
its row record count and the six native row-height caches change. The recognized
mesh remains available to the existing MOVE writer after leading-contact deletion.
Unsupported shapes and stale operands remain rejected without changing the file.

Native manual Delete and refill agree on every edited-network byte. Native manual
editing additionally refreshed unrelated heating heights at L3 (62 to 39), L4
(62 to 39), and L12 (78 to 50). The fixture asserts those three exact differences;
generated edits preserve the earlier networks. Both generated candidates pass
strict all-program Check Program with 0 errors, 27 warnings and 42 messages.
Native Save As preserves all seven decoded program payloads exactly in each
case: 14 comparisons, zero differences, without cache exceptions.

The Rust write suite passes 94 tests (114 ignored). The contact capture test also
checks surrounding bytes, repeat Delete, stale-operand rejection and WORD-as-BOOL
rejection. Two focused WASM tests cover mesh contacts and mesh MOVE editing.
Browser Delete, Enter refill, double-click editing, repeat Delete, physical
blank-cell double click, and cancel/reopen pass; five emitted edits match the
Rust-generated files exactly. Browser validation uses real WASM and a mock VS
Code prompt bridge. Evidence is in `iec-contact-mesh-contact-20261003` under the
VM captures folder. Original-project contact deletion preflight coverage is
21/284; this case does not establish full contact or IEC editing coverage.

### Addressed contacts across the supplied IEC programs

The contact writer now removes one canonical addressed contact record while
retaining its row, branch endpoints, wires, function references and surrounding
payload bytes. It checks the expected kind, position and operand, validates the
resulting circuit layout, and requires every function-pin binding to survive.
The layout validator permits open branch endpoints after deletion so an
unfinished network can be saved and then refilled. Sole-element stored rows,
unknown record flags and unknown execution modes remain guarded.

Blank-cell insertion accepts branches at the cell's left and right boundaries,
preserves native record order among branch endpoints, follows surviving
execution flags, and closes the selected contact's open endpoints. Other open
endpoints and all function bindings must remain unchanged. Native disabled
groups may contain either marked or unmarked records; insertion follows the
surviving records rather than changing their execution flags.

A final full-project Rust preflight covers all 284 original contacts: every
Delete succeeds, and 282 refills recover the original program records exactly.
The resolver still rejects two undeclared names in disabled networks after their
final contact use is removed: `스위치_1` and `스위치_2`.
Fresh WASM publishes 284/284 deletion sites. This is preflight coverage, not
284 independent native acceptance captures.

Native interior NC Delete/refill at heating L80/x7 agrees on every circuit
record. Manual editing also refreshes nine explicitly checked display-height
caches. The generated upper deletion passes strict all-program Check Program
with 0 errors, 27 warnings and 42 messages. The generated lower deletion at
L81/x7 leaves two open endpoints; native checking reports two L0000 input/output
errors, zero warnings and 36 messages for that incomplete network. Refilling
restores the original program payloads. Three focused WASM tests and browser
upper/lower Delete, Enter, double click, repeat Delete and cancel/reopen checks
pass. Browser validation uses real WASM with a mock VS Code prompt bridge.
Evidence is in `iec-mesh-interior-contacts-20261003` under VM captures.

Native Save As preserves all seven decoded program payloads for the upper
interior deletion, lower interior deletion and restored original program:
21 comparisons, zero differences, without cache exceptions. The incomplete
lower network can be saved and reopened; its two open endpoints remain until
refill. The `xgi_interior_contact_native_capture` fixture enforces this gate.


### IEC system BOOL operands

The installed Korean IEC instruction help (`XGI(R)InstructionHelp_kor.chm`,
section 부2.4, `100.htm`) identifies 13 BOOL user flags: `_T20MS`, `_T100MS`,
`_T200MS`, `_T1S`, `_T2S`, `_T10S`, `_T20S`, `_T60S`, `_ON`, `_OFF`, `_1ON`,
`_1OFF` and `_STOG`. Type resolution recognizes their symbolic names without
assigning CPU-dependent device addresses or adding local declarations. They
are read-only operands; coil destinations still require writable BOOL values.
WASM exposes a separate system-variable catalog, and IEC instruction prompts
include its type and description in autocomplete.

The `_ON` contact at heating L30/x4 now restores the original program payload
exactly after its last usage is deleted. Rust and WASM checks reject unknown
flag names and WORD operands. Browser Delete, Enter refill, double click,
repeat Delete and cancel/reopen checks pass; all five emitted edits match Rust
candidates, and autocomplete includes all 13 flags. Browser checks use real
WASM and a mock VS Code prompt bridge. Evidence is in
`iec-system-operands-20261003` under VM captures.

Native XG5000 opens the generated `_ON` deletion and restored file. Strict
all-program checking reports four errors for deletion (two L0000 open-input
errors and two L0401 omitted-input errors), zero warnings and 36 messages.
The restored file passes with zero errors, the original 27 warnings and 42
messages. Both results are captured as `SYONDEL-check.png` and
`SYONREF-check.png` in the system-operand evidence directory.

Before the addressed-coil extension, the original-coil preflight found 67
coils, only seven successful deletions and two exact refills. The newer coil
coverage and native evidence are recorded below.

Native Save As preserves all seven decoded program payloads in both system-flag
cases: 14 comparisons, zero differences and no cache exceptions. The ignored
`xgi_system_flag_native_save_capture` fixture enforces exact restored-source
and native-capture equality. The updated VSIX is installed; all 14 runtime
files match the checkout, and its packaged README and manifest match.


### Addressed IEC coils

The coil writer now accepts canonical addressed coils in retained stored rows,
including rows with branches, contacts, function blocks and function references.
Deleting a right-rail coil removes its preceding long feed as in native XG5000.
It retains surrounding records, execution flags, row caches and function-pin
bindings. The earlier simple-row and branch-only writers remain available.
Blank-cell refill reconstructs the native feed from the last contact, branch
endpoint, function control output or reference cell. It closes only the selected
feed endpoint and preserves all other endpoints and bindings. Simple-row refill
uses the surviving contact's execution flag rather than always disabling the
new wire and coil.

A group-header candidate must carry a canonical execution mode. Previously,
removing coils from four elevator rows left long-wire bytes immediately before
another row that were mistaken for duplicate group headers. This parser fix
keeps those layouts framed without changing stored bytes.

The full original-coil preflight reports 67 coils, 67 successful deletions and
67 refills with exact original program payload recovery. Contact regression
coverage remains 284 deletions and 282 exact refills. This is preflight coverage,
not 67 independent native acceptance captures. Focused WASM tests cover active,
disabled, branched and function-fed rows and reject stale operands, WORD
operands and read-only system flags as coil destinations. Browser Delete, Enter,
double click, repeat Delete, physical double click and cancel/reopen pass; all
five emitted edits match Rust candidate bytes. Browser checks use real WASM
with a mock VS Code prompt bridge.

Native heating L24/x94 Delete and F9 refill agree with every generated circuit
record. Native editing refreshes exactly three display-height bytes: L3 and L4
62 to 39, and L12 78 to 50. The ignored `xgi_addressed_coil_native_capture` fixture
checks these differences explicitly. Evidence is in
`iec-addressed-coils-20261003` under VM captures.


Generated heating L24/x94 deletion opens in native XG5000 and reports one
L0000 error for the missing output, zero warnings and 36 messages under strict
all-program checking. Generated refill passes with zero errors, the original
27 warnings and 42 messages. The final full-coil audit also verifies all seven
program payloads after every refill, including the six unrelated programs.


Native Save As retains all seven decoded program payloads for generated coil
Delete and refill: 14 comparisons, zero differences and no cache exceptions.
The native coil fixture enforces this gate when `LIBXGWX_COIL_SAVE_CAPTURE=1`.
The VSIX is installed, and all 14 installed runtime files match the checkout;
its packaged README and manifest also match. The complete Rust suite reports
95 passed and 117 ignored; three focused WASM cases and 19 prompt/rule tests
pass. Full IEC editing remains unfinished beyond this original-coil coverage.

### Undeclared contacts in disabled networks

The remaining original-contact refill failures were lighting L2/x1
`스위치_1` and L6/x1 `스위치_2`. Both are rising contacts in disabled
networks and neither name has a local or global declaration. Contact insertion
now preserves unresolved identifier references only when the group execution
mode is disabled and the surviving row records carry the captured disabled
flags. It does not allocate or declare variables. Known non-BOOL symbols,
malformed expressions, enabled rows, unmarked rows and unresolved coil
destinations remain rejected.

The full original-project preflight now deletes and exactly refills all 284
contacts, retaining all seven decoded program payloads. This is local coverage,
not 284 separate native acceptance captures. The focused WASM test also checks
that declarations remain unchanged. Browser Delete, Enter refill, double-click
editing, repeated Delete and cancel/reopen pass; five emitted edits match Rust
bytes. These browser checks use real WASM with a simulated VS Code prompt bridge.

Native text entry of the undeclared switch name opens an Add Variable dialog;
canceling does not insert it. Native Delete followed by Undo restores the
imported unresolved contact. These captures agree on every circuit byte after
accounting for two explicit lighting display-height refreshes (L22 52 to 39,
L26 52 to 50). The lighting local-symbol record offsets become 22 bytes shorter
per preceding record; every parsed declaration field, including allocation
number and width, remains identical. The fixture checks the offsets explicitly
rather than discarding declaration differences.

Both generated deletions and the common restored project pass strict
all-program Check Program with 0 errors, 27 warnings and 42 messages. Both
refill candidates are byte-identical, so one restored-project native capture
covers them. Evidence is stored in `iec-disabled-identifiers-20261003` under the
VM captures folder. The ignored `xgi_disabled_identifier_native_capture` test
uses `LIBXGWX_SMARTHOME_FIXTURE` and `LIBXGWX_DISABLED_IDENTIFIER_CAPTURE`;
`LIBXGWX_DISABLED_IDENTIFIER_SAVE_CAPTURE=1` enables its generated Save As gate.

That gate passes: native Save As preserves all seven decoded program payloads
for each generated candidate (21 exact comparisons), with no cache exceptions.
Every parsed local-symbol field, including record offsets, is unchanged in
these generated saves. The Rust suite passes 96 tests with 118 external
fixture tests ignored; the focused native fixture passes with its Save As gate
enabled. Full IEC editing remains unfinished beyond original contact/coil
Delete and refill coverage.

The next function audit is available through
`iec_function_edit_coverage SOURCE --refill`. It reads operands from framed
expression records, orders input operands before outputs, and exercises the
public insertion routes used by the cell editor after each independent Delete.
All 81 deletions pass; 75 refills are accepted and 12 recover all seven original
program payloads exactly. Accepted refills with differing payloads need a
separate circuit/layout comparison; this count is not native acceptance.
Six chained EQ refills remain rejected for overlapping retained cells:
lighting L67/x16 and heating L46, L50, L54, L58 and L62 at x19. These are the
next concrete gaps in original-function restoration. The audit output is
`function-refill-preflight.log` in the disabled-identifier evidence folder.

### Refilling the disabled contact-fed comparison chain

Lighting L67/x16 now restores the row that the captured chain-head Delete
closed. Native Ctrl+L before the following comparison provides room; native
EQ insertion splits the x12 branch into an endpoint and continuation on the
new row. The writer follows that layout, preserves the NO/NC contact feeds,
shifts the later comparison bodies and their pin links, and retains the disabled
network mode. New function, operand and feed records receive the native disabled
flags. It does not create local declarations.

The generated refill agrees with native insertion of `EQ %MW10 0 %MX0` on every
circuit record. Native editing also refreshes exactly three display-height
fields: lighting L22 52 to 39, L26 52 to 50, and L76 56 to 39. The fixture checks
those differences explicitly across all seven program payloads. Removing the
new body retains the restored branch rows and matches native Undo of placement
with the same height differences. Repeating Delete/refill reproduces the full
generated file exactly. This does not claim native Delete Line has the same
row-retention behavior as Undo.

Native manual refill passes strict all-program checking with 0 errors,
27 warnings and 42 messages. The focused WASM test covers declaration
preservation, neighboring programs, function counts and rows, deletion-site
publication, repeat Delete/refill, stale names, WORD destinations and read-only
flags. Browser Delete, Enter refill, double-click replacement, repeated Delete,
physical blank-cell double click and cancel/reopen pass; five edits match Rust
bytes. Browser checks use real WASM with a simulated VS Code prompt bridge.

Evidence is in `iec-chain-comparison-refill-20261003` under the VM captures
folder. `xgi_chain_comparison_refill_native_capture` uses
`LIBXGWX_SMARTHOME_FIXTURE` and `LIBXGWX_CHAIN_REFILL_CAPTURE`.
`LIBXGWX_CHAIN_REFILL_OUTPUT` emits the generated refill and row-preserving
deletion. `LIBXGWX_CHAIN_REFILL_SAVE_CAPTURE=1` enables their native Save As gate.
The five heating-chain comparison refills remain guarded pending their own
native branch reconstruction evidence.

The generated refill and subsequent deletion both pass strict all-program
Check Program with 0 errors, 27 warnings and 42 messages. Their native Save As
gate passes: all fourteen decoded program payloads are byte-identical to the
generated candidates, without display-cache exceptions. Every parsed local
symbol field, including record offsets, also remains identical. These captures
are `CH0GENS` and `CH0GDELS` in the same evidence directory.

The current independent function audit accepts 76 of 81 refills. Twelve recover
all original program payloads exactly; accepted differences still require
circuit/layout classification. The five rejected refills are the heating
comparisons above. The current log is `function-refill-preflight.log` in the
chain-comparison evidence directory.

The first heating head now has native reconstruction evidence, but no new
runtime support yet. Starting with `FD6R46X19`, Ctrl+L before the following
comparison (L49) restores room, and EQ placement at L46/x19 with
`%MW129`, `1`, `%MX729` passes strict all-program checking with 0 errors,
27 warnings and 42 messages. `CH6NREF` captures the refill; `CH6NROOM` captures
Undo of the three operands and placement while retaining the inserted row.
An accidentally selected ECRRS placement was undone before selecting EQ.

Compared with the public blank-row operation on that deletion, the native
scaffold materializes L49 with a 35-byte header and two BranchEnd/BranchStart
pairs at x3 and x15 (107 added bytes). Each preceding start and following end
is redirected to L49. Both continuations have enabled flags. Seven row heights
also refresh: heating L3 and L4 62 to 39, L12 78 to 50, L50 and L54 61 to 39,
and L51 and L55 68 to 39. The row-aware diagnostic `iec_row_diff` reports these
bounded changes without offset drift obscuring later records.

The native refill is not identical to the original source: the chain-head
deletion removed two additional branches from L46 and four contact/feed records
from L47. Native reinsertion retains that post-deletion feed shape. The source
comparison has 118 fewer circuit-record bytes after refill; all six other
program payloads remain exact. This distinction must remain explicit when
implementing and testing heating refill, rather than treating original-file
equality as its native behavior.

The writer now implements this enabled chain-head reconstruction by graph
shape, without program names or project identifiers. It recognizes the retained
NO/NC contact prefix and both consecutive x3/x15 spines, shifts the closed row,
and materializes its native branch endpoint/continuation records. Subsequent
Delete retains the scaffold and follows native Undo of placement. The dedicated
`xgi_heating_head_refill_native_capture` fixture matches all circuit records in
`CH6NREF` and `CH6NROOM`, checks each of the seven height differences, and
verifies repeated Delete/refill byte equality and atomic destination rejection.
It uses the same capture/output environment variables as the lighting fixture;
its optional Save As gate expects `CH6GENS` and `CH6GDELS`.

The Rust suite passes 96 tests with 120 external fixture tests ignored. Focused
WASM checks cover lighting and heating restoration, declaration preservation,
neighboring programs, scalar-deletion-site publication, and repeat operations.
The heating browser test passes Delete, Enter refill, double-click replacement,
repeat Delete, physical blank-cell double click and cancel/reopen; five edits
match Rust bytes. It uses real WASM with a simulated VS Code prompt bridge.
Both generated states pass strict all-program checking with 0 errors,
27 warnings and 42 messages. Native Save As preserves all fourteen program
payloads and every parsed local-symbol field, including offsets, exactly;
the Save As fixture gate passes without display-height exceptions. The current
function audit accepts 77/81 refills, with 12 recovering every original payload
exactly. The other four heating refills remain guarded. Evidence and the audit
log (`heating-function-refill-preflight.log`) are in the same capture directory.

### Refilling the two middle heating comparisons

The enabled dual-feed writer also recognizes a middle comparison by its
retained x3/x15 branch rows and a preceding comparison in the same network,
without a program index, network number or row constant in the new runtime
guard. It restores the closed final pin row, splits both feeds into native
endpoint/continuation records, and retains the next comparison's branch end.
L50 and L54 native placement captures each pass strict all-program checking
with 0 errors, 27 warnings and 42 messages. Their original operands are
`EQ %MW129 2 %MX730` and `EQ %MW129 3 %MX731`.

Both generated refills match every native circuit record. Each native capture
refreshes seven display heights. L50: heating L3/L4 62 to 39, L12 78 to 50,
L54/L58 61 to 39 and L55/L59 68 to 39. L54: heating L3/L4 62 to 39,
L12 78 to 50, L58 61 to 39, L59 68 to 39, L62 73 to 39 and L63 74 to 39.
The fixture checks each field explicitly across all seven program payloads.
Removing the generated body preserves the restored scaffold and matches native
Undo of placement with the same height differences. The circuit records of
both refills also agree with the original source; cached row-header fields can
differ after native deletion and insertion.

`xgi_heating_middle_refill_native_capture` checks both captured cases,
declaration preservation, repeated Delete/refill and atomic destination
rejection. It uses the existing chain-refill fixture/capture/output variables.
The optional Save As gate expects `M50GENS`, `M50GDELS`, `M54GENS`, and
`M54GDELS`; all four generated states pass strict all-program checking with
0 errors, 27 warnings and 42 messages. Native Save As preserves all 28 program
payloads and every parsed local-symbol field, including offsets, exactly. The
Save As fixture gate passes without display-height exceptions. Native captures are `M50NREF`, `M50NROOM`, `M54NREF` and
`M54NROOM` in `iec-chain-comparison-refill-20261003`. L54's output operand was
visually found missing during the first input attempt; it was corrected and
the strict check repeated before the final capture.

The Rust suite passes 96 tests with 121 external fixtures ignored, and the
three focused WASM tests pass without skips. The two remaining rejected
heating refill shapes are the x3-fed comparison and the later contact-fed
comparison. Full IEC editing remains unfinished.

Both middle browser checks pass Delete, Enter refill, double-click replacement,
repeated Delete, physical blank-cell double click and cancel/reopen. Five edits
per case match Rust bytes, using real WASM with a simulated VS Code prompt
bridge; rendered screenshots were reviewed. The current original-function
preflight accepts 79/81 refills, with 12/81 restoring every original payload
exactly. The x3-fed and contact-fed heating comparisons remain rejected. The
lighting, heating-head and heating-middle native Save As fixture gates all
pass (3 tests); the middle gate compares all four generated states. Evidence
is in `iec-chain-comparison-refill-20261003`, including `middle-save-gates.log`,
`middle-function-refill-preflight.log` and both middle browser logs/screenshots.

### Refilling the outer-fed heating comparison

The x3-only retained scaffold is now recognized structurally: an enabled
network, a preceding three-pin comparison in the same network, three retained
branch endpoint/continuation pairs at x3, and a following comparison after the
closed pin row. Native Ctrl+L and EQ placement materialize the fourth branch
row (71 bytes) and connect EN with an x4-to-x16 feed. The original operands are
`EQ %MW129 0 %MX735`. Original deletion repairs the prior comparison's orphan
x15 feed; refill retains that repaired shape rather than restoring the removed
original x15 records.

Native `M58NREF` passes strict all-program checking with 0 errors, 27 warnings
and 42 messages. `M58NROOM` captures Undo of the three operand edits and block
placement, retaining the restored branch row. Generated refill and deletion
match all native circuit records across all seven programs, with seven
explicit display-height refreshes: heating L3/L4 62 to 39, L12 78 to 50,
L62 73 to 39, L63 74 to 39, L66 52 to 39 and L67 56 to 39.
`xgi_heating_outer_refill_native_capture` verifies both states, parsed locals,
repeat Delete/refill and atomic rejection of WORD/read-only destinations. All
four chain-refill manual-capture fixtures pass. Generated files are `M58GEN`
and `M58GDEL`; both pass strict all-program checking with 0 errors, 27 warnings
and 42 messages. Native Save As `M58GENS` and `M58GDELS` preserves all fourteen
program payloads and every parsed local-symbol field, including offsets,
exactly. All four chain-refill Save As fixture gates pass. Evidence is in
`iec-chain-comparison-refill-20261003`. The later contact-fed L62 refill remains
unfinished.

The Rust suite passes 110 tests with 122 external fixtures ignored. The full
Cargo test command also compiles the diagnostic examples; an ambiguous row
parse in `iec_branched_function_acceptance` was fixed with an explicit u16.
Four focused WASM tests pass without skips. The L58 browser check passes five
edits matching Rust bytes and cancel/reopen, with no console/page errors; its
rendered screenshot was reviewed. It uses real WASM with a simulated VS Code
prompt bridge. Original-function preflight accepts 80/81 refills and 81/81
deletions, with 12/81 refills restoring all original payloads exactly. This is
preflight coverage, not proof of arbitrary placement or combined edits. The
full editable-project goal remains unfinished.

### Refilling the contact-fed heating comparison

The contact-fed `EQ` at original L62/x19 retains the three contacts at x7,
x10 and x13 and the inner x15 continuation when it is deleted and refilled.
The writer recognizes that structural prefix and derives the feed boundary
from its retained branch records. It does not use a project name or a fixed
row number to select this insertion layout.

Native `M62NREF` and `M62NROOM` captures agree with the generated circuit
records and all seven decoded program payloads after seven explicit heating
row-height changes: L3/L4 62 to 39, L12 78 to 50, L66/L70 52 to 39,
and L67/L71 56 to 39. `xgi_heating_contact_refill_native_capture` verifies
these captures, repeat Delete/refill, unchanged locals, and atomic rejection
of invalid destinations. All five chain-refill capture fixtures pass together.

Five focused WASM tests pass. The L62 browser check emits five edits matching
the Rust files exactly and checks Delete, Enter, physical double-click,
replacement, and cancel/reopen. It uses real WASM with a simulated VS Code
prompt bridge. Original-function local preflight now accepts 81/81 deletions
and 81/81 refills; only 12 refills reproduce every original program payload
exactly. This coverage does not establish arbitrary placement or combined
editing support. Generated `M62GEN` and `M62GDEL` both pass strict all-program checking with
0 errors, 27 warnings and 42 messages. Their native Save As captures
`M62GENS` and `M62GDELS` preserve all fourteen decoded program payloads
and every parsed local-symbol field, including record offsets, exactly.
All five chain-refill Save As fixture gates pass.

### Combined contact deletion and comparison refill

Deleting a contact at x7, x10 or x13 in the retained L62 prefix used to prevent
comparison refill. The prefix guard now checks the fixed branch and short-wire
records plus the surviving contacts, in their original cell positions, rather
than requiring all three contacts and their original kinds. All retained branch
coordinates, flags, row heights and neighboring function constraints remain
checked. Placement accounts for the EN feed consuming the open x15 endpoint
when its preceding contact is missing. The scalar Delete wrapper accepts an
incomplete circuit only when the comparison scaffold writer has validated the
retained records; other scalar layouts retain their existing graph checks.

Native `combined/C62N7` and `combined/C62N13` permit comparison placement with
the x7 or x13 contact missing. Their circuit records agree with the generated
files, with the same seven explicit cached row-height changes as L62 refill.
Strict all-program checking reports six errors with x7 missing and ten with
x13 missing. These are incomplete editing states, not compilable programs.
Restoring x7 natively (`C62N7F`) returns to zero errors, 27 warnings and 42
messages, and its circuit records match the completed generated refill.

`xgi_combined_contact_comparison_native_capture` covers all seven nonempty
subsets of the three contacts: Delete contacts, refill EQ, Delete/refill EQ
again, then restore the contacts. Each completed result matches all seven
payloads of the previously validated `M62GEN` and preserves parsed locals.
Native capture comparisons cover missing x7, missing x13 and restored x7.
Rebuilt WASM passes all seven combinations plus the two existing comparison
refill tests without skips. The x7 and x13 browser workflows each emit five
edits matching Rust bytes, with no page or console errors; both rendered
results were reviewed. These use real WASM and a simulated VS Code prompt
bridge. Generated `C62R7`, `C62G7`, `C62R13` and `C62G13` complete native
strict all-program checks with 6, 6, 10 and 9 errors respectively, zero
warnings and 36 messages. These contact-gap states remain intentionally
incomplete. Their native Save As captures preserve all 28 decoded program
payloads and every parsed local-symbol field, including offsets, exactly.
The combined capture fixture passes with `LIBXGWX_COMBINED_SAVE_CAPTURE=1`.

### Remaining row-shift editing regression

The diagnostic `iec_shifted_function_probe` inserts a blank row before the
original heating comparison chain, checks that function positions move by one
row, and verifies unchanged local symbols and unrelated program payloads.
Running the function-edit audit against this shifted file rejects Delete for
all five original comparisons at L47, L51, L55, L59 and L63 before the
structural changes described below. The legacy heating deletion writers
required captured program, group and row indices; this was the baseline
regression that motivated deriving positions from decoded records.

### Structural deletion after row and group changes

The five heating comparison deletion routes now derive the program, group and
row positions from decoded records. The head uses its group's first row; the
middle, outer and contact-fed blocks use their selected block row. The orphan
x15 repair finds a unique five-row chain from its branch endpoints. Record
kinds, feed positions, function ownership, operand references and the resulting
circuit remain guarded; unrecognized layouts are rejected atomically. The
scalar comparison scaffold already handles refill at the shifted positions.

A fixture checks all five original comparisons after inserting a blank row
before the chain, both with and without an added comment that changes the
group index. Delete and row insertion commute across all seven decoded program
payloads, and locals stay unchanged. A rebuilt WASM check exercises these ten
cases, including repeat scalar Delete/refill. Browser workflows for all five
shifted comparisons pass Delete, Enter refill, double-click editing and
cancel/reopen; all fifteen edits match Rust output exactly, with no console or
page errors. The screenshots were reviewed. The browser uses a simulated VS
Code prompt bridge. The shifted source and all ten generated deletion/refill
states pass strict native all-program checks with zero errors, 27 warnings and
42 messages. Native Save As preserves all 77 program payloads and every parsed
local-symbol field, including offsets, exactly. The shifted fixture passes
with `LIBXGWX_SHIFTED_SAVE_CAPTURE` set to these eleven captures. The existing
five chain-refill native Save As gates also pass.

The next combined-edit regression is contact-kind sensitivity in the original
contact-fed comparison deletion shape. `iec_contact_comparison_kind_probe`
changes its x10 contact among all six addressed kinds without modifying the
source file. Every kind edit succeeds, but comparison Delete accepts only the
original NC kind and rejects NO and the four edge kinds atomically. Native
validation and a structural fix for this combination remain unfinished.

### Comparison deletion after contact-kind changes

The original contact-fed comparison deletion shape now accepts all six
addressed contact kinds in its four known contact positions. Explicit contact
coordinates (x7/x10/x13 on the top row and x7 on the parallel row) are checked
alongside the existing branch geometry, reference ownership and result graph.
Retained contacts keep their kind; the parallel contact is removed with the
native-owned branch when the comparison is deleted.

The Rust and rebuilt WASM matrices pass 48 combinations: all six kinds at four
contact positions, before and after a preceding row insertion. Kind edits and
comparison deletion commute for the retained contacts, removed-branch contacts
produce the baseline deletion, repeated scalar Delete/refill is exact, locals
stay unchanged, and wrong-name edits reject atomically. The standard Rust suite
passes 110 tests with 127 external fixtures ignored. Six browser workflows pass
contact-kind editing, comparison Delete, Enter refill, double-click replacement
and cancel/reopen, with no page or console errors. Twenty-three edits match
Rust bytes; the NC no-op preserves the original source bytes. All six rendered
results were reviewed. The prompt bridge is simulated.

Native checking of the L62 x10 contact across all six kinds and all three states
(contact-kind edit, comparison deletion and refill) passes: eighteen generated
candidates, strict all-program checking, zero errors, 27 warnings and 42 messages
each. Native Save As preserves all 126 program payloads and every parsed local
symbol field, including record offsets, exactly.

Direct native Delete Line on the comparison fed by the NO contact also reports
zero errors with the same warning/message counts. Its circuit records match the
generated deletion exactly. Native interactive editing refreshes six cached
row heights: L3/L4 62 to 39, L12 78 to 50, L62 39 to 73, L65 52 to 39 and L66 56
to 39. It also rewrites local-symbol framing: program 6 symbol indices 1/2 move
from record offsets 124/244 to 102/200. Every parsed symbol value, type, address,
description and allocation is unchanged. The native Save As fixture asserts
these exact manual differences and compares every remaining program byte and
local-symbol field. No such exceptions apply to the eighteen generated saves.
The contact-kind comparison VSIX is packaged and installed. All fourteen
runtime files match the tested checkout and VSIX, the installed manifest matches
the source manifest, and the installed README matches the packaged README.
Full arbitrary IEC placement and wiring remain unfinished.

### Contact-first comparison deletion preflight

The next combined-edit gap is independently reproduced at four original contact
positions: heating L62 x7/x10/x13 and L63 x7. Deleting each contact alone succeeds,
but none of ten comparison-deletion routes accepts the remaining original L62
x19 comparison shape. Every rejected route leaves the document bytes unchanged.
The diagnostic `iec_contact_before_comparison_probe` takes the source, program,
comparison coordinates and contact sites from CLI arguments and never overwrites
the source. This is local preflight evidence, not native acceptance. Supporting
this contact-first sequence remains unfinished; the installed contact-kind fix
covers kind changes with the contacts retained.

The local contact-first implementation now validates the original non-contact
branch and operand scaffold separately from optional addressed contacts. Top
contacts are allowed only at x7/x10/x13 in their original order and position
between the original branch records; the parallel contact is allowed only at
x7. Other row record sequences, branch geometry and operand/reference ownership
remain checked. Removal and retained row counts follow the surviving contacts.
The source graph may be incomplete after either top or parallel contact deletion;
the result graph must be complete when all three top contacts survive. Missing
top contacts retain an explicitly validated partial editing state.

The Rust lifecycle matrix passes all fifteen nonempty contact-deletion masks
with and without a preceding row insertion (30 cases). Contact-first and
comparison-first deletion commute across all seven payloads, local fields remain
unchanged, wrong-name edits reject atomically, comparison refill can be repeated
exactly, and restoring the retained contacts produces a complete circuit graph.
All 48 previous contact-kind lifecycle cases still pass. Rebuilt WASM passes the
same 30 cases and matches all sixty generated Rust states, including canonical
completion after restoring contacts. Five browser workflows pass the four
single deletions and all-three-top deletion, comparison Delete/refill, retained
contact restoration and prompt cancel/reopen, using real WASM with a simulated
VS Code prompt bridge. Their deletion, refill and completed restoration states
match Rust. All five rendered results were reviewed. Native validation and
installation of this contact-first update remain pending.

The updated standard Rust suite passes 110 tests with 128 external fixtures
ignored. In XG5000, the source with only the L62 x7 contact removed (`A1C`)
opens and reports two errors, zero warnings and 36 messages under strict
all-program checking. Native Delete Line on its first comparison pin row L63
reports six errors, zero warnings and 36 messages, consistent with a remaining
incomplete contact feed. Its Save As completed as `A1NS`. Two further native
Delete Line captures completed: removing only the parallel contact (`A8C`)
reports three errors before comparison deletion and zero errors, baseline 27
warnings and 42 messages afterward (`A8NS`); removing all three top contacts
(`A7C`) reports ten errors before deletion and five errors, zero warnings and
36 messages afterward (`A7NS`). All checks used the reviewed strict all-program
options. These are native intermediate-state observations; generated
contact-first checks and saved payload/local-field comparisons remain pending.

After confirming the VM was off, read-only disk extraction recovered all three
manual captures. `xgi_contact_first_native_delete_capture` passes: all seven
payloads per capture match after the six explicitly captured row-height cache
updates, and every parsed local field matches after the two captured framing
offset changes (124/244 to 102/200). For `A7NS` only, native Delete Line also
removes the exact 15-byte x4 ShortWire in L62 and changes its record count from
three to two. The regression asserts that complete record and all remaining
bytes. The generated writer currently retains this wire; native's depleted
two-record feed is a distinct shape that still needs refill/edit support.
These manual results do not establish generated contact-first Save As acceptance.

The native comparison exposed a functional difference in the all-top-missing
case: the pre-cleanup generated `A7D` reports nine compiler errors, while native
Delete Line reports five. The writer now removes the captured x4 ShortWire
when no top contacts survive. Its native deletion regression passes without
any circuit-record exception; only the six cache heights and two local framing
offsets differ. Refill recognizes the depleted two-record feed and restores its
x4 contact feed while materializing comparison pin rows. The captured native
middle-row height of 73 is accepted only for the validated middle prefix.

All 30 updated Rust lifecycle cases pass. Of sixty generated states, only
`A7D` and `A15D` change from the archived pre-cleanup results; all contact,
refill and completion outputs remain identical. Refill/repeated deletion and
contact restoration also pass directly on `A1NS`, `A8NS` and `A7NS`, preserving
every native local-symbol field. The standard suite passes 110 tests with 131
external tests ignored. Native strict all-program checking of the completed
`A7F` reports zero errors, baseline 27 warnings and 42 messages; Save As
completed as `A7FS`. Updated WASM/browser verification, generated native
Save As comparisons and installation remain pending.

The rebuilt WASM passes the updated thirty-case matrix and all sixty Rust
output comparisons. A separate WASM test passes refill, repeated Delete/refill
and restoration on all three native captures, matching all six Rust `N*R/F`
outputs exactly. The five real-diagram browser workflows also pass with a
simulated VS Code prompt bridge; the updated screenshots were reviewed.
Read-only extraction after VM shutdown verifies `A1DS` and `A7FS` against
their generated sources: all fourteen payloads and every parsed local field,
including offsets, are exact. The old nine-error `A7DS` matches the archived
pre-cleanup `A7D` and is retained only as diagnostic evidence. The corrected
all-top-missing source is staged under `B7D` for its separate native check and
Save As; the native-save gate requires `B7DS` for that state. Remaining native
generated-state captures and installation are still pending.

The corrected `B7D` opens in XG5000 and strict all-program checking reports
five errors, zero warnings and 36 messages, matching manual `A7NS` rather than
the old nine-error state. Save As completed as `B7DS`; extraction and comparison
are pending until the VM is off. The mandatory generated-save regression now
requires all fifteen `A` states (using `B7DS` for corrected `A7D`) plus the six
native-import refill/restoration `N` states. Its test code compiles; the full
capture gate has not run because those remaining saves are still required.

The contact-first native save gate is now complete. All fifteen generated
deletion/refill/restoration states and six native-import refill/restoration
states were opened, checked with reviewed strict all-program options, and
saved under separately reviewed names. After orderly VM shutdown, read-only
extraction recovered all twenty-one saves. The mandatory
`xgi_contact_first_native_save_capture` test passes: all 147 program payloads
and every parsed local-symbol field, including record offsets, match exactly.
All eight completed restorations report zero compiler errors and the baseline
27 warnings. Partial top-contact states retain their observed missing-input
errors; those intermediate states are not claimed to compile. Removing the
parallel contact and comparison, and refilling that comparison, reports zero
errors. This gate includes the corrected `B7DS` capture, not the archived
pre-cleanup `A7DS`. Full arbitrary IEC placement and wiring remain unfinished.

An isolated real VS Code development host now covers the contact-first UI
sequence with actual built-in prompts. Double-click and Enter open contact
and comparison prompts; Escape then Enter reopens them; contact autocomplete
lists BOOL symbols. Real Delete removes a contact and its comparison, Enter
on the selected blank cell refills EQ, and blank-cell double-click restores
one contact. Selection advances to the next contact. Ctrl+S restores the QA
copy byte-for-byte to its starting `A7F` file. This run exposed a mouse-focus
bug hidden by the earlier browser tests' direct cell focus: pointerdown on
the board could leave focus on the page body. The webview now focuses the
blank indicator after a click finishes, and the same failed interaction
passes. The original project was untouched. Browser-plugin support was
unavailable; Playwright controlled the isolated Electron host over local CDP.

A fresh project-wide writer audit covers all original addressed elements:
`iec_contact_edit_coverage` accepts deletion and exact refill for 284/284
contacts, and `iec_coil_edit_coverage` does so for 67/67 coils. The branch audit
accepts row-preserving vertical removal for 192/192 segments; cleanup removal
that rebuilds row groups remains restricted to 86/192 segments. These results
are writer preflight, not native acceptance for every original element or
arbitrary sequences of edits. Broader free function placement remains a gap.

First-open binary-editor fallback was reproduced in isolated installed and
development VS Code hosts. The extension now activates on startup and recovers
clean XGWX text tabs after checking the file signature and editor preferences.
Fresh development and installed profiles on VS Code 1.137.0 open the workspace
editor automatically with one tab, without Reopen or Reload. Dirty text and
explicit editor associations are preserved; recovery examines only startup tabs.
This routing change does not alter project serialization or extend the native
acceptance of IEC edits.

## Fresh MOVE placement on an implicit blank row (2026-10-04)

The native capture in `iec-full-edit-audit-20261004/FM87S.xgwx` places
`MOVE %MW129 %MW130` at program 6, L87, raw X4, after the source's last
stored row. XG5000 adds the EN feed and three row envelopes without moving
earlier rows. Strict all-program checking reports 0 errors, 27 warnings and
42 messages. Save As was reviewed and the capture extracted read-only after
the VM exited.

The scalar writer now permits this fresh MOVE position beyond the stored
row count and sets a new shared operand row's end cell to OUT. The
`iec_fresh_function_acceptance` example reconstructs the new group on the
native-saved baseline and checks all seven program payloads and every local
symbol field, including offsets, exactly. It also emits a separate file
generated directly from the original project for native acceptance.

Manual native insertion changes six preceding row-height cache bytes and
two local-symbol record offsets. These are separate from the fresh group,
which matches exactly. Opening the generated-original file, reviewing strict
all-program checking (0 errors, baseline 27 warnings, 42 messages), and Save
As `FM87GS.xgwx` preserves all seven generated program payloads and every
local field including offsets exactly. The VM exited before read-only
extraction. Rust regression checks: 107 passed, 131 external acceptance tests
ignored. Rebuilt WASM produces byte-identical output to Rust for this edit.
Real VS Code host QA passes fresh L87/x4 insertion through Enter,
double-click editing, Escape/Enter reopening and Ctrl+S. Saved QA bytes match
the Rust output exactly. The UI now exposes trailing selectable blank rows;
editing and saving preserve diagram scroll (verified scrollTop 5711 before
and after). `vscode-xgwx-fresh-move.vsix` is installed, with all 14 runtime
files, the manifest and packaged README verified against the package.
Arbitrary function positions and other fresh scalar families remain unfinished.

## Fresh arithmetic and comparison capture (2026-10-04)

Native insertion after the fresh MOVE capture adds `ADD %MW129 1 %MW130`
at L91/raw X16 and `EQ %MW129 1 %MX735` at L96/raw X10. XG5000 displays
EQ as `=`. Strict all-program checking reports 0 errors, 29 warnings and
42 messages after ADD, then 0 errors, 31 warnings and 42 messages after EQ.
The additional warnings have not yet been attributed. Reviewed Save As
captures `FA91S.xgwx` and `FE96S.xgwx` were extracted read-only after VM exit.

`iec_scalar_placement_capture` removes and reconstructs each new complete
group on its native-saved baseline. Both recreations preserve all seven
program payloads and every local field, including offsets, exactly.
The writer now allows fresh scalar blocks beyond the final stored row;
conversion blocks retain their separate placement guard. Four focused Rust
tests pass. `iec_fresh_scalar_sequence` generates MOVE, ADD and EQ directly
from the original project and checks the old heating prefix, six other
programs and all local fields. The generated-original sequence passes native
strict all-program Check with 0 errors, 31 warnings and 42 messages. Reviewed
Save As `FSQGS.xgwx`, extracted read-only after VM exit, preserves all seven
payloads and every local field including offsets exactly. Rebuilt WASM emits
the same bytes as Rust. Real VS Code blank-cell Enter prompts insert all
three blocks, and Ctrl+S saves byte-identical output while preserving diagram
scrollTop 6289. `vscode-xgwx-fresh-scalars.vsix` is installed; all 14 runtime
files, manifest and packaged README match the checkout/package/installation.
Broader mixed-layout wiring remains unfinished.

## Independent function placement after a wire gap (2026-10-04)

An exposed endpoint in an earlier network previously blocked scalar insertion
through the public writer even when the new function was below all stored rows.
Independent fresh placement now preserves all earlier payload bytes after the
program header, exposed endpoints and existing typed pin bindings exactly.
Placement in the affected stored layout retains its separate validation rules.

`iec_open_layout_fresh_acceptance` places `MOVE 1 %MW100` at program 0,
L92/raw X4 on the native-saved wire-gap/text-edit baseline `NATIVE_ROUND.xgwx`
from `iec-open-layout-text-20261002`. XG5000 opens and renders the generated
file; reviewed strict all-program Check reports 0 errors, 27 baseline warnings
and 42 messages. Reviewed Save As `OPFGS.xgwx`, extracted read-only after VM
exit, preserves all seven payloads and every local field including offsets
exactly. Rebuilt WASM produces identical bytes. Real VS Code blank-cell Enter
inserts the MOVE and Ctrl+S saves the exact Rust output, retaining scrollTop
6017. The original project was untouched.

Rust regression: 107 tests passed, 132 external tests ignored. The separately
run open-layout fixture test passes MOVE/ADD/EQ independent placement and
atomic overlap rejection. The native combined-state gate in this batch covers
MOVE; the other scalar bodies retain their earlier native capture evidence.
`vscode-xgwx-open-fresh.vsix` is installed. All 14 runtime files, the manifest
and packaged README match the checkout/package/installation.

## Function cell edits beside an unrelated wire gap (2026-10-04)

The function-cell writer now permits FF and R_TRIG deletion/refill in a closed
network when another network has exposed wire endpoints. It verifies unchanged
group/row counts, exact untouched group envelopes, retained endpoint positions
and unchanged unrelated pin bindings. Editing the network containing the exposed
endpoints still requires separate support.

`iec_open_cell_cycle` deletes lighting FF L14/raw X4 and R_TRIG L20/raw X7 and
L26/raw X7 from the native-saved `NATIVE_ROUND.xgwx` baseline, then refills their
original instances. XG5000 opens both generated states. Strict all-program Check
on the intermediate deletion reports six errors (four invalid I/O and two
missing-required-input errors), zero warnings and 36 messages. Refilling restores
zero errors, 27 baseline warnings and 42 messages. Deletion leaves an incomplete
circuit; it is not a compilable-program acceptance result.

Reviewed native Save As files `OCDGS.xgwx` and `OCRGS.xgwx`, extracted read-only
after confirmed VM exit, preserve all seven program payloads and every local
field including offsets exactly. No display-cache exceptions are used. Rebuilt
WASM and real VS Code Delete/Enter/Ctrl+S produce the exact Rust files. The prompt
now carries IEC context so instance names such as `FF` are resolved by the IEC
writer rather than classified as XGK addresses; autocomplete still checks types.

Rust regression reports 107 passed and 132 external tests ignored. The full
extension run exposed stale disabled-network unresolved-name rejection assertions;
the corrected case checks malformed-expression rejection and unchanged local
declarations and passes. These checks cover the three selected cells beside the
retained wire gap, not all mixed-layout editing operations.

`vscode-xgwx-open-cells.vsix` is installed; all 14 runtime files, the manifest
and packaged README match the checkout/package/installation. The two native
leading-contact and short-wire-contact fixture regressions also pass with the
updated disabled-network validation assertions.

The refreshed open-layout function audit improves deletion preflight from
63/81 to 67/81 and refill preflight from 60/81 to 64/81; 12/81 refills preserve
all payloads exactly (previously 9/81). This is preflight evidence. Remaining
failures include ADD/SUB/MOVE/comparison transformations that split or shift
groups and still require a completely closed program graph. The connected
trigger repair WASM regression now checks the actual `localVariables` field
with a nonempty declaration assertion, and passes.

## Arithmetic splits and merges beside a wire gap (2026-10-04)

Connected ADD/SUB deletion now preserves unrelated networks while splitting
the edited group. Scalar refill similarly preserves unrelated networks while
merging its affected groups. Validation checks exact untouched envelope bytes
after the group number, remapped exposed endpoints and unrelated typed pin
bindings, including their pin-point group numbers. The edited source groups
must contain no exposed endpoints. Group renumbering and shifted byte offsets
are expected consequences; they do not permit rewriting other network contents.

`iec_open_arithmetic_cycle` deletes lighting ADD L20/raw X16 and SUB L26/raw
X16 on `NATIVE_ROUND.xgwx`, then restores their three original operands
(`%MW700`, `1`, `%MW700`). The combined deletion shifts the earlier wire-gap
group index by two; both refills return its original index and endpoint positions.
Native strict all-program checking reports 0 errors, 25 warnings and 42 messages
for deletion and 0 errors, 27 baseline warnings and 42 messages for restoration.
Reviewed Save As targets are `OADGS.xgwx` and `OARGS.xgwx`.

WASM produces both Rust files byte-for-byte. Real VS Code Delete, blank-cell
Enter refill and Ctrl+S also reproduce both files exactly. Rust regression:
107 passed, 132 external tests ignored. The earlier FF/R_TRIG native exact
comparison still passes after the preservation-check refactor. Three previously
blocked MOVE refills (L56/L59/L63) now pass preflight; their combined-state native
validation remains separate pending work.

The native Save As gate passes for both combined states after confirmed VM
exit and read-only extraction: all seven payloads and every local field including
offsets are exact to the generated input. No height-cache or symbol-offset
exceptions are used.

`vscode-xgwx-open-arithmetic.vsix` is installed. All 14 runtime files, the
manifest and packaged README match the checkout/package/installation.

## MOVE and wired EQ edits beside a wire gap (2026-10-04)

Scalar-chain MOVE deletion and wired comparison deletion/refill now preserve
unrelated open networks using the same exact group-preservation checks. The
edited source group must have no exposed endpoints; this does not yet enable
arbitrary edits within the source open network.

`iec_open_move_cycle` exercises lighting MOVE L22/raw X4, MOVE L28/raw X4,
MOVE L38/raw X19 and wired EQ L37/raw X10. It captures three states: all
three MOVEs deleted, the result MOVE restored with EQ deleted, and all four
blocks restored. Strict native all-program checks report respectively
2 errors/0 warnings/36 messages, 4 errors/0 warnings/36 messages, and
0 errors/27 baseline warnings/42 messages. The deletion states retain incomplete
connections: the first reports two invalid I/O errors; the second reports
three invalid I/O errors and one omitted required input. They are editable
intermediate states, not valid complete programs.

Reviewed native Save As files `OMDGS.xgwx`, `OEDGS.xgwx` and `OMRGS.xgwx`
were extracted read-only after confirmed VM exit. All seven program payloads
and every local-variable field, including offsets, match each generated
checkpoint exactly. No exceptions or normalization are used. WASM and real
VS Code Delete, blank-cell Enter insertion and Ctrl+S reproduce all three
checkpoints byte-for-byte. Rust regression: 107 passed, 132 external tests ignored.

`vscode-xgwx-open-move.vsix` is installed. All 14 runtime files, the manifest
and packaged README match the checkout/package/installation.

The refreshed all-function audit reports 75/81 deletions and 75/81 refills
accepted (previously 67/81 and 64/81), with 12/81 exact payload refills.
These counts are writer preflight, not native acceptance of every case.
Remaining deletion failures are lighting MOVE L48/raw X19, L52/raw X16,
L84/raw X16 and comparisons L67/L71/L75 in the exposed-endpoint group.

## Parallel and upper MOVE layouts beside a wire gap (2026-10-04)

The parallel-contact MOVE writer, staggered upper MOVE deletion, and upper
continuing-contact MOVE deletion/refill now preserve unrelated open networks.
The outer scalar-chain validation derives the replacement group count from
the validated result: staggered upper deletion splits one group into two.
Every untouched group envelope, exposed endpoint and unrelated pin binding
is still checked by `iec_preserves_groups_around_edit`. Edited source groups
with exposed endpoints remain guarded.

`iec_open_move_layout_cycle` deletes lighting MOVE L48/raw X19, L52/raw X16
and L84/raw X16, then reinserts their original operands (`0` to `%MW600`,
`0` to `%MW700`, and `0` to `%MW10`). The combined deletion shifts the
unrelated exposed endpoint group indices by one; refill restores them.
Native strict all-program Check reports 2 invalid I/O errors, 0 warnings and
36 messages for deletion, and 0 errors, 27 baseline warnings and 42 messages
for restoration. The deleted state is an editable incomplete circuit.

Reviewed native Save As targets `OMLDGS.xgwx` and `OMLRGS.xgwx` were extracted
read-only after confirmed VM exit. Both preserve all seven payloads and every
local field, including offsets, exactly to the generated files. WASM and real
VS Code Delete, Enter insertion and Ctrl+S reproduce both checkpoints
byte-for-byte. Final Rust regression: 107 passed, 132 external tests ignored.

`vscode-xgwx-open-move-layout.vsix` is installed. All 14 runtime files, the
manifest and packaged README match the checkout/package/installation.

## Comparison edits within an open network (2026-10-04)

GE and continuing EQ deletion/refill now support the existing open comparison
network. A row preservation check requires every exposed endpoint, group header,
row outside the edited footprint, branch record within that footprint and
unrelated typed pin binding to remain unchanged. The first comparison has a
dedicated guarded scaffold; this does not enable arbitrary wiring replacement.

`iec_open_comparison_cycle` deletes GE L67/raw X16 and EQ L71/L75/raw X16,
then reinserts their original operands. Strict native all-program checks of
both checkpoints complete with 0 errors, 27 baseline warnings and 42 messages.
Reviewed Save As files `OCMDGS.xgwx` and `OCMRGS.xgwx`, extracted read-only
after confirmed VM exit, preserve all seven program payloads and every local
field including offsets exactly. No exceptions or normalization are used.
WASM and real VS Code Delete, blank-cell Enter insertion and Ctrl+S reproduce
both generated files byte-for-byte. Rust regression: 107 passed, 132 external
tests ignored. Broader mixed wiring and variable/settings editing remain open.

The refreshed full function audit accepts 81/81 deletions and 81/81 refills,
with 13/81 byte-identical payload refills. These are independent writer
preflights on the open-layout baseline, not native acceptance of every case.
`vscode-xgwx-open-comparison.vsix` is installed; all 14 runtime files, semantic
manifest and packaged README match the package and installation.

## Terminal coils beside a wire gap (2026-10-04)

Simple terminal coil deletion and refill now permit an unrelated open network.
The edited network retains its exact captured one-row shape; every untouched
group envelope, exposed endpoint and unrelated pin binding is preserved.
This removes the whole-program closed-graph restriction for that shape.

`iec_open_coil_cycle` deletes the lighting output coils at L2/L6/L10, raw X94,
then restores their original BOOL operands. Both strict native all-program
checks complete with 0 errors, 27 baseline warnings and 42 messages. Reviewed
Save As targets `OCLDGS.xgwx` and `OCLRGS.xgwx`, extracted read-only after VM
exit, preserve all seven program payloads and every local field including
offsets exactly. Restoration also matches the source payloads/local fields.
WASM and real VS Code Delete, Enter reinsertion and Ctrl+S reproduce both
generated files byte-for-byte. Rust library checks: 99 passed with `wasm,write`,
132 external tests ignored; the `write` library check separately passes 96.

The refreshed open-layout writer audit accepts 67/67 coil deletions and exact
refills. The contact audit accepts 284/284 deletions and refills (it retains
the previously validated mesh height allowance). These are independent writer
preflights, not native acceptance of every cell or arbitrary mixed wiring.

`vscode-xgwx-open-coil.vsix` is installed. All 14 runtime files, semantic
manifest and packaged README match the package and installation.

## Blank row editing with an existing wire gap (2026-10-04)

Blank row insertion/deletion now permits an open-layout source. In addition to
the existing exact frame checks, `iec_preserves_layout_after_row_shift` compares
the entire coordinate-translated layout: wiring, occupied areas, pin bindings,
power components and exposed endpoints, with all record offsets retained.
The existing operation dialog uses the updated WASM validation gate.

`iec_open_blank_row_cycle` inserts a blank line after lighting L10 and removes
the resulting empty L11. Every later function and both gap endpoints shift
down one row, then recover. Both strict native all-program checks complete
with 0 errors, 27 baseline warnings and 42 messages. Reviewed Save As targets
`OBIGS.xgwx` and `OBRGS.xgwx`, extracted read-only after confirmed VM exit,
preserve all seven program payloads and every local field including offsets
exactly, without normalization or exceptions. Restoration matches the source.
WASM and actual VS Code operation-dialog insertion/deletion and Ctrl+S reproduce
both generated checkpoints byte-for-byte. Rust library regression: 99 passed,
132 external tests ignored with `wasm,write`.

The open-layout wiring audit accepts 191/191 row-preserving vertical removals,
but only 75/191 branch cleanup removals. Cleanup and broader mixed wiring remain
unfinished; these audit counts are writer preflight, not native acceptance of
every wire edit.

`vscode-xgwx-open-blank.vsix` is installed. All 14 runtime files, semantic
manifest and packaged README match the package and installation.

### Parallel wire cleanup beside an unrelated gap

The writer now permits paired-wire removals from closed networks when a separate
network is incomplete. It requires another vertical segment between the selected
rows and preserves every unrelated group envelope, exposed endpoint and typed
binding. The UI determines open-tail status from the selected network.

The private `iec_open_parallel_branch_cycle` audit removes both alternatives in
lighting groups 8, 26 and 34, then reconnects them. All seven program payloads and
every local field restore exactly. Writer cleanup preflight increases from
75/191 to 81/191; row-preserving removal remains 191/191. Regression results are
99 Rust library tests passed (132 ignored), and 127 extension tests passed
(58 skipped). Both WASM cycles match the Rust output exactly.

Native Save As captures preserve all seven payloads and every local field,
including offsets, without normalization. Strict all-program Check reports:

- First deletion set: 2 errors, 0 warnings, 36 messages (L0000 invalid
  input/output and L0403 conversion error).
- Second deletion set: 1 error, 0 warnings, 36 messages (L0000 invalid
  input/output).
- Restored source: 0 errors, 27 warnings, 42 messages.

Isolation identifies L48–L49/x3 as the source of L0403. Deleting that same
wire directly in XG5000 produces the same two diagnostics and completed footer.
Every program-0 row record matches the generated isolated deletion; native
interactive editing changes six display-height fields. The generated isolated
L48–L49/x3 and L84–L85/x3 Save As captures separately preserve all seven
payloads and every local field including offsets exactly, without exceptions.
The latter deletion passes strict all-program Check with 0 errors, 27 warnings
and 42 messages. Reconnecting the wires restores the compiling baseline.

Actual VS Code interaction removes all six wires through the operations picker,
checks the selected-network labels, reconnects through its insertion picker and
uses Ctrl+S at four checkpoints. Each saved workspace equals the corresponding
Rust/WASM output byte for byte. Full-file editability remains unfinished:
row-deleting cleanup, broader mixed wiring, globals, hardware/settings and
first-window editor routing still need work.

Private evidence: `iec-full-edit-audit-20261004/open-parallel-*` and
`isolate-*`.

Delivery: installed `vscode-xgwx-open-parallel.vsix`; all 14 runtime files,
semantic package manifest and packaged README match the installed extension.

### Branch row cleanup beside an unrelated gap

Captured two-row contact/wire branch cleanup now works when another network
has an open wire gap. The writer proves preservation by removing the edited
network from two private validation views, applying the verified blank-row
translator to the source view, and requiring exact remaining payload bytes.
The edited network must stay closed. This covers later function pin coordinates
and the original gap; it does not relax general row-group reconstruction guards.

`iec_open_branch_row_acceptance` checks two independent lighting edits:
group 3 L3–L4/x6 and group 22 L42–L43/x3. Each removes the lower row and shifts
later coordinates by one. Both passed native XG5000 strict Check Program over
all seven programs with **0 errors, 27 warnings, 42 messages**, matching the
source baseline. After native Save As, all seven ProgramData payloads and every
local-symbol field, including offsets, matched exactly without normalization.

The rebuilt WASM produced both generated files exactly. Actual VS Code picker
interaction, Ctrl+S and Ctrl+Z passed four exact file checkpoints, including
restoration of the original source bytes. Rust library regression checks passed
99 tests (132 ignored). Writer preflight increased from 81 to 85 of 191 branch
cleanup segments; all 191 row-preserving wire removals still passed preflight.
These coverage counts are not native acceptance for every segment. Other mixed
branch shapes, row-deleting function feeds and tails, and general reconstruction
remain guarded and need additional native evidence.

This update was packaged and installed after the native and actual-editor gates.
All 14 runtime files matched the source, VSIX and installed extension exactly;
package metadata and the packaged README were also verified. The native VM and
QA development host were closed, and the read-only extraction process stopped.

### Terminal feed and tail cleanup beside other open networks

Captured terminal feed deletion now accepts a closed selected network while a
different network has a gap. A private validation view proves that all other
network bytes equal the verified one-row coordinate translation. The selected
network may expose exactly the captured temporary endpoint. Unique-tail cleanup
scopes its endpoint search to that network and requires all other network bytes
to stay exact, plus unchanged function-binding semantics. Forked tails and
unsupported feed/reference shapes remain rejected.

`iec_open_feed_acceptance` exercises a short-wire feed in program 2 at L29/x18
with an unrelated gap at L19–L20/x21, and a long-wire feed sharing a function
continuation row in program 3 at L67/x21 with a later gap at L71–L72/x18.
All translated function bindings and retained gap points are checked. Reconnecting
each unrelated gap gives exactly the cleanup of the original closed network,
including all program payloads and all local-symbol fields.

Native XG5000 strict all-program checking completed for both gap baselines,
both feed-only intermediates and both cleaned results. Each deliberately gapped
baseline reports **2 errors, 0 warnings, 36 messages**. Feed deletion temporarily
raises that to **3 errors, 0 warnings, 36 messages**; tail cleanup returns it to
the baseline **2 errors, 0 warnings, 36 messages**. These are incomplete circuits,
not zero-error native acceptance claims. All four native Save As checkpoints
match every one of the seven ProgramData payloads and every local-symbol field,
including offsets, exactly, without normalization or allowances.

Rebuilt WASM matches all four generated stages. Real VS Code picker interaction
performs each feed-plus-tail removal as one edit labelled "Remove terminal feed
and tail"; saving and undo pass four exact file checkpoints. Rust library
regression checks pass 99 tests with 132 ignored. General network reconstruction,
feeds with unsupported operand/reference prefixes, and broader editor work remain
incomplete.

### Whole-network edits beside unrelated wiring gaps

Whole-network deletion, copying, relocation, and replacement now accept decoded
IEC layouts with gaps in other networks. Insertions preserve every untouched network envelope
except its group number; moves prove exact equality after removing the relocated
network from both layouts. Cross-program copying retains the existing declaration,
address-collision, and function-instance checks.

`iec_open_network_acceptance` exercises deletion, contact-network copy and move,
replacement, MOVE-network copy, and cross-program MOVE copying with missing local
declarations. All six generated files opened in XG5000 and completed strict
all-program Check Program with zero errors and 42 messages. The first four had
27 warnings; the two MOVE-copy cases had 28 warnings. Native Save As preserved all
seven program payloads and every local-symbol field, including offsets, exactly.
Copy/delete and move/back checks also restore the expected payloads exactly.
Those six captures cover closed selected networks with unrelated gaps.
General mixed wiring reconstruction remains incomplete.

Actual VS Code validation covered row-inspector deletion, contact and MOVE
network Ctrl+C/Ctrl+V, move and replacement pickers, cross-program clipboard
copying with missing locals, Ctrl+S, and Ctrl+Z restoration. Saved QA files matched
the corresponding generated checkpoints exactly. Save acknowledgments now update
status controls without replacing the diagram, preserving keyboard focus and
unfinished inspector input.

Selected incomplete networks can also be copied and moved when their decoded
layout is valid. Endpoint coordinates and group indices must translate exactly;
removing the selected envelope must leave all other network bytes unchanged.
`iec_incomplete_network_acceptance` covers a four-comparison network copied
within a program, moved to vacant rows, and copied with locals across programs.
All three completed native strict all-program Check Program with zero errors.
The first two retained their prepared baseline of 19 warnings; cross-program
copy retained its baseline of 17 warnings. All had 42 messages. Native Save As
preserved all seven program payloads and every local field, including offsets,
exactly. Same-program copy/delete and move/back restore the prepared source.
Actual VS Code clipboard copy, move-picker placement, cross-program clipboard
copy, save and undo were checked against exact generated file checkpoints.

## Numeric IEC local memory mappings

Local address editing now supports captured WORD `%MW` mappings and DINT/REAL
`%MD` mappings, in addition to BOOL bit addresses. The PB50 allocation uses
a bit offset and width: `%MW2200` is 35200/16, `%MD1000` is 32000/32.
Assigning, clearing, and remapping update both address text and allocation
metadata. Overlapping mapped ranges are rejected even across `%MW` and `%MD`;
type mismatches, malformed addresses, and overflow leave the document unchanged.
The initial capture covered these three numeric types; the subsequent capture
below covers all primitive memory mappings. Numeric I/O shapes remain guarded.

`iec_numeric_address_acceptance` recreates the three native mappings exactly,
checks clearing and atomic rejection, then remaps all three. The generated file
completed strict all-program XG5000 Check Program with zero errors, 27 warnings
and 42 messages. Native Save As preserved all seven program payloads and every
local field including offsets exactly. The baseline is a native-saved local
table: opening the native table cleared its existing stored usage-marker strings.
Those differences were observed separately and were not normalized in acceptance.
Actual VS Code remapping, Ctrl+S, and three-step undo matched generated file
checkpoints exactly; WASM output matched the Rust writer exactly.
Actual VS Code clearing and reassigning all three mappings also matched the
Rust-generated file checkpoints. Reassignment restores the native program and
local declaration data exactly; compressed file bytes follow the writer encoding.
The variable editor rejects a WORD mapping that overlaps a DINT mapping.

## All primitive IEC local memory mappings

A native XG5000 capture now covers all 19 primitive local types. PB50 mapped
allocations store bit offsets and widths, with these verified address shapes:

| Prefix | Bit width | Types |
| --- | --- | --- |
| `%MX` | 1 | BOOL |
| `%MB` | 8 | BYTE, SINT, USINT |
| `%MW` | 16 | WORD, INT, UINT, DATE |
| `%MD` | 32 | DWORD, DINT, UDINT, REAL, TIME, TIME_OF_DAY |
| `%ML` | 64 | LWORD, LINT, ULINT, LREAL, DATE_AND_TIME |

The native table rejects an address whose size differs from the declared type.
DATE uses 16 bits; DATE_AND_TIME uses 64 bits. The writer and variable editor
now allow all captured primitive memory shapes for mapped or unallocated locals.
Numeric I/O shapes, arrays and structures still require separate validation.
Function instances remain excluded. The automatic primitive capture below
extends explicit memory assignment to the four automatic types in this project.

`iec_primitive_mapping_acceptance` prepares 19 unallocated declarations while
preserving all seven original program payloads. The native mapping capture
completed strict all-program Check with zero errors, 27 warnings and 42 messages.
The example verifies each captured prefix, bit offset, width and storage class;
clears and reassigns all 19 to recreate every local field including offsets;
remaps all 19; and checks atomic wrong-size, malformed, overflow, equivalent
address and cross-width overlap rejection. The generated project also completed
strict all-program Check with zero errors, 27 warnings and 42 messages. Native
Save As preserves all seven program payloads and every local field including
record offsets exactly, without normalization.

The prior WORD/DINT/REAL native gate remains exact. The Rust write suite passes
96 tests with 132 external tests ignored. WASM produces the same generated file
as Rust. Actual VS Code remapping of all 19 types, Ctrl+S, and 19-step undo/save
match the generated/native byte checkpoints exactly. The full extension suite
passes 128 tests with 58 external tests skipped and zero failures.

### Mapping existing automatically allocated primitive locals

Native captures establish assignment from storage class `A` to `M` for BOOL,
INT, UDINT, and TIME. The writer accepts an existing primitive allocation only
when its captured width matches its type, and changes the address, bit offset,
width and storage class together. Other allocations and all program payloads
are preserved. Automatic BOOL assignment is restricted to `%MX`; numeric types
use their captured memory size. Automatic I/O and instance mappings remain
guarded.

`iec_automatic_mapping_acceptance` reproduces four native assignments, rejects
overlap, malformed numbers, overflow and automatic I/O mapping atomically,
and checks all seven program payloads and every local field including offsets.
The native manual capture and the generated project's Save As both match
exactly without normalization. Strict all-program Check completes with zero
errors, 27 baseline warnings and 42 messages. The native table's final cell must
be committed by selecting another table cell before Check or Save As; an
initial TIME edit visible in the input did not persist and was recaptured.

WASM output is byte-identical to Rust. Actual VS Code assignment of the four
types and Ctrl+S matches the generated file; four undo operations followed by
save restore the baseline exactly. Rust checks pass, the write library suite
passes 96 tests with 132 external tests ignored, and the extension suite passes
128 tests with 58 external tests skipped. This does not implement automatic
allocation of new locals or arbitrary automatic composite declarations.

### New unallocated locals used as ladder operands

`iec_new_local_allocation_probe` declares a new BOOL and INT in program 6,
then places a BOOL output rung and an INT MOVE output using those declarations.
The generated project passes strict all-program native Check with zero errors,
27 baseline warnings and 42 messages. Native Save As preserves all seven
program payloads and every local field including offsets exactly. Both new
locals retain empty storage classes and absent allocation numbers and widths.
Thus native editing and Check do not require an eager automatic allocator for
these two cases. This capture does not establish PLC compilation or download
behavior, new UDINT/TIME operand cases, or composite declaration support.
