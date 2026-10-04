# Native function record fixtures

Captured with XG5000 4.82.1 during function placement validation on 2026-09-30.
These contain individual function records rather than project files.

- `iec_move.bin`, `iec_add.bin`, and `iec_eq.bin`: standard native control and
  data pin descriptors. They contain no instance or project variable names.
- `xgk_eq.bin`, `xgk_ge.bin`: native two-operand comparison contact records,
  with generic D-device operands. The test translates their stored coordinates
  before comparing insertion output.

Complete local open/Check Program/Save As evidence is retained outside Git in
`VMs/xg5000-win10/captures/function-placement-20260930`.

- `xgk_gt.bin`, `xgk_lt.bin`, `xgk_le.bin`, and `xgk_ne.bin`: word comparison
  records extracted from a generated project after native XG5000 4.82.1
  Check Program and Save As on 2026-10-01. All six comparison rungs compiled
  with 0 errors and 0 warnings; the 4213-byte ProgramData payload survived
  native Save As unchanged. These records contain only D100/D102 operands.
  Evidence is retained in `target/xgk-comparisons-20261001` (ignored by Git).

- `iec_word_to_udint.bin`: WORD source and UDINT destination pin descriptors
  extracted from the supplied native IEC project's existing conversion block.
  The serializer reproduces its opcode, flags, pin names and reference ordinals
  exactly before translating coordinates for general placement.

- `iec_int_to_udint.bin`, `iec_udint_to_time.bin`, `iec_time_to_udint.bin`,
  and `iec_udint_to_int.bin`: unmodified scalar conversion bodies extracted on
  2026-10-03 from the supplied native IEC project. They contain only function
  and pin names, type masks, flags, and coordinates; no project variable names.
  The test preserves the captured connected/terminal state while comparing the
  canonical prototype. Complete-document restoration outputs reproduce the
  previously native-validated project byte-for-byte. Native deletion and saved
  candidate evidence: `VMs/xg5000-win10/captures/iec-mixed-scalar-chain-20261003`.
