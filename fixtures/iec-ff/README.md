# IEC LD FF placement

These small synthetic XGI-CPUE projects contain one native `FF` instance named
`FF`, without the private reference project's declarations or programs.

- `empty-with-instance.xgwx`: empty LD program and a captured native FF instance
  record, used for normal tests and the editor WASM test.
- `fresh-generated.xgwx`: generated fresh FF placements at L0/x4, L6/x16 and
  L12/x91; existing instance allocation is retained.
- `fresh-native-roundtrip.xgwx`: XG5000 4.82.1.0 Check Program and Save As of that
  generated file, on 2026-10-17. All-program checking with strict type checking
  and missing-FB-input checks enabled reported **0 errors, 0 warnings, 16 messages**.
  ProgramData and all decoded instance fields including offsets remain exact.
- `contact-prefix.xgwx`: the native-saved synthetic project with a single NO
  `%MX0` contact added at L20/x1.
- `contact-native.xgwx`: native FF insertion at L20/x10 after that contact.
  The writer reproduces the complete native ProgramData byte-for-byte, including
  every earlier FF network. Native all-program checking of this contact-fed
  project also reported 0 errors and 0 warnings.

Separately captured native fresh insertions at L94/x4 and L100/x16 in a local
reference project also reconstruct exactly after deleting/recreating their
whole groups; that private project is not distributed here. The
`iec_ff_acceptance` dev tool compares those captures and regenerates a clean
acceptance project from its isolated FF declaration.

Fresh placement supports two clear rows or a normal network consisting of one
leading contact at x1. It accepts a uniquely declared FF instance; it does not
automatically create one. Other occupied groups, disabled networks, comments,
branch merges and overlap remain guarded. Existing specialized two-row FF
refill insertion keeps its original independent writer. `Q` is left available
for subsequent output assignment or wiring. Native acceptance covers Open,
Check Program and Save As; no PLC download or execution was performed.

## Q output wiring

FF takes an instance operand only. Q is a BOOL power-flow output and does not
require a destination-variable assignment. The editor's F5/Enter/double-click
wire action extends Q directly, preserving the instance declaration and the
hidden native Q link. Numeric output pins retain their destination rules.

`q-wires-native.xgwx` captures two native F5 wire cells from the contact-fed
FF.Q at L20/x13 and x16. The writer reproduces its entire ProgramData exactly,
with no output-variable assignment. This deliberately open wire endpoint is
an encoding fixture: native Check Program reports one incomplete-rung error,
zero warnings and 12 messages; it is not a complete executable program.
