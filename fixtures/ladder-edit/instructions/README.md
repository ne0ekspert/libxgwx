# Native instruction replacement captures

Offline XG5000 4.82.1.0 on Windows 10, 2026-09-09.
Start from `fixtures/elements.xgwx` unchanged, then use native instruction editing:

- R70.bin: replace `MOV,0,D000000` with `ADD,1,2,D000000` and Save As.
- R71.bin: from R70, replace ADD with `TON,T0000,100` and Save As.

These files contain decompressed ProgramData only. Native editing recalculates
all row display heights on the first edit; tests normalize only those unrelated
height bytes for the source-to-R70 comparison. R70-to-R71 is byte exact.
Generated replacement files were separately opened, checked (0 errors/warnings),
and resaved in XG5000 without ProgramData changes; see docs/ladder-editing.md.

## Operandless applications

`operandless.bin`: native XG5000 4.82.1.0 Save As, 2026-10-04, a new XGK-CPUUN
project with NO M00000 -> STOP, NO M00001 -> WDT, and END. Full program logical,
syntax and duplicate-coil checks completed with 0 errors and 0 warnings.
The saved project's title was reviewed before clean VM shutdown and read-only
extraction. The fixture contains its complete 419-byte ProgramData payload.
Tests reproduce both application records byte for byte while preserving every
other payload byte, exercise growth/restoration, and keep END protected.
This validates the shared operandless record envelope, not PLC execution or
native checks of every catalog entry. Private evidence: XGKZEROS.xgwx and
xgk-check-clean.png under the local iec-full-edit-audit-20261004 capture directory.

## Comparison width changes

`comparison_dword.bin` and `comparison_dword_group.bin`: complete native
ProgramData payloads (541 and 574 bytes), XG5000 4.82.1.0, 2026-10-04.
The same private XGK-CPUUN program replaces its first contact with
`D= D000100 D000104`, saves, then replaces that comparison with
`DG= D000100 D000104 1`. The latter passed all-program logical, syntax and
duplicate-coil checks with 0 errors and 0 warnings. Save As source paths and
saved titles were reviewed before clean shutdown and read-only extraction.
The comparison stays at its left anchor and grows into the following wire;
its remembered operand position advances. Tests compare every payload byte
in both directions, with no normalization. Private evidence: D2CMPS.xgwx,
DG2CMPS.xgwx and xgc-ternary-check.png in iec-full-edit-audit-20261004.

## Complete comparison catalog and output deletion

`comparisons_first_half.bin` (12,986 bytes) and `comparison_output.bin`:
native Save As of a generated private program containing
39 comparison/MOV pairs, following NO->STOP and NO->WDT, then END. These are the
two halves of the complete 78-comparison suite; both batches passed full native
checks with 0 errors/warnings and retained all payload/local-record bytes exactly.
Native XG5000 4.82.1.0, XGK-CPUUN, 2026-10-04. Full payload: 13,580 bytes.

`comparison_output_deleted.bin`: native Delete on the first MOV application,
including its output feed wire. Full payload: 13,440 bytes. Check Program reports
one incomplete-rung error, 0 warnings, after removing that output.

`basic_brst.bin`: insert BRST M00020 8 in that gap, then Save As. Full payload:
13,592 bytes. Native all-program checks: 0 errors, 0 warnings. The test compares
every byte through deletion and BRST reinsertion without normalization.

Every Save As source path and saved title was reviewed; capture extraction was
read-only after both the actual VM PID and QMP socket disappeared. Private
sources: XGC2S.xgwx, XGCDS.xgwx, XGKBRSTS.xgwx; screenshots xgc2-check-result,
xgc-delete-check-result, xgk-brst-check-result in iec-full-edit-audit-20261004.

## Indexed-bit inputs and FF

`indexed_bits_ff.bin`: complete 676-byte native ProgramData payload, XG5000
4.82.1.0, XGK-CPUUN, 2026-10-04. Native insertion replaced STOP with FF M00030
and the two initial NO contacts with B D000100 4 and BN D000104 D000108.
All-program logical, syntax and duplicate-coil checks completed with 0 errors
and 0 warnings (13 messages). Save As source path, saved title and guest file
existence were reviewed before shutdown and read-only extraction.

The test reconstructs the entire payload from `operandless.bin` and compares
every byte without normalization. Native insertion pads word addresses; the
reconstruction supplies those exact spellings. This does not establish address
normalization for arbitrary generated operands. B/BN remember their final
operand position in the row prefix. Private evidence: XGI2S.xgwx and
indexed-repeat-check-result.png in iec-full-edit-audit-20261004.

## Quoted string literals

`string_literals.bin`: complete 683-byte native ProgramData payload, XG5000
4.82.1.0, XGK-CPUUN, 2026-10-04. A private baseline replaces STOP with
$MOVP 'Room B, off' D000200 and WDT with $MOV 'Room A, on' D000100.
All-program logical, syntax and duplicate-coil checks completed with 0 errors
and 0 warnings (13 messages). The first trial used D104 and produced overlapping
string destination warnings; the accepted capture uses D200.

Source filename/path, saved title XGSTRS and guest file existence were reviewed.
Read-only extraction occurred after the actual VM PID and QMP socket disappeared.
The writer reconstructs all 683 bytes and local records exactly, without
normalization. Both literal rows use prefix byte 17 value 40 instead of the
ordinary 39 variant. Private evidence: XGSTRS.xgwx, strings-final-check-options,
strings-final-check-result, strings-save-source, strings-save-destination,
strings-saved-title and strings-file-verified in iec-full-edit-audit-20261004.

This proves the captured ASCII literal encodings and compiler checks, not PLC
runtime behavior, non-ASCII encodings or apostrophe escape syntax.
