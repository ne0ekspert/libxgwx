# IEC function output wires

These projects contain synthetic direct addresses and a public blank XGI-CPUE
project. They contain no user project data.

- `base.xgwx`: generated ADD, SUB, DIV, EQ and INT_TO_UDINT, with numeric or
  BOOL destination assignments.
- `add-eno-native.xgwx`: XG5000 F5 on ADD.ENO at L0/x10, followed by Save As.
- `all-native.xgwx`: additional F5 on SUB.ENO at L5/x10, DIV.ENO at L10/x10,
  INT_TO_UDINT.ENO at L19/x10, and native Delete on EQ.OUT's assignment then F5
  at L16/x10. Each numeric result assignment is retained.

The writer reproduces both native ProgramData payloads byte for byte.
These one-cell captures have dangling output wires; opening and Save As do
not establish that an incomplete circuit passes Check Program.

Native screenshots and original captures:
`~/VMs/xg5000-win10/captures/function-output-wires-20261007/`.

- `full-native.xgwx`: generated continuous short-wire feeds from all five
  output pins to output coils, opened and checked in XG5000 (logic, syntax,
  strict types; all programs), then saved as OUTFULLS. Zero errors, with a
  reported duplicate-coil warning. The entire 4,570-byte ProgramData is exact.
  This is native file/compiler acceptance, not PLC execution validation.

## Clear and restore OUT assignments

- `cleared-native.xgwx`: native Delete on all five OUT assignments in `base`,
  retaining each body, reference and row. Check Program reports zero errors and
  zero warnings with logic, syntax and strict type checks enabled.
- `conversion-restored-native.xgwx`: native Enter on the empty INT_TO_UDINT.OUT
  cell and reassignment of `%MD10`, then Save As. The writer matches the full
  ProgramData of both captures exactly, including the row header cursor cache.

Evidence: `~/VMs/xg5000-win10/captures/function-output-clear-20261007/`.
