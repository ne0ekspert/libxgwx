# Broader IEC LD block insertion

The native-backed fresh insertion path supports R_TRIG, F_TRIG, TON, TOF, TP,
RS, SR, CTU_DINT, CTD_DINT, CTUD_DINT, NE, MOD, AND, OR, XOR, NOT and
WORD_TO_UDINT. FF retains its existing fresh and captured-refill paths.

Fresh placement requires clear rows or one leading contact at x1 in a normal
network. It preserves declarations, unrelated networks and their row numbers.
Columns use native anchors x4 through x91 in steps of three. Occupied groups,
comments, disabled networks and branch merges remain guarded; existing specific
TON/refill APIs retain their previous layouts and checks.

Stateful calls take a declared local instance of exactly the requested type,
followed by data inputs in native order. The control input comes from the rung.
Optional trailing data outputs can be omitted; no output variable is required
for Q/QU/QD wiring. Instances are not automatically declared or allocated.

| Block | Operands after the instance | Optional data destination |
| --- | --- | --- |
| R_TRIG, F_TRIG | none | none |
| TON, TOF, TP | PT (TIME) | ET (TIME) |
| RS | R_1 (BOOL) | none |
| SR | R (BOOL) | none |
| CTU_DINT | R (BOOL), PV (DINT) | CV (DINT) |
| CTD_DINT | LD (BOOL), PV (DINT) | CV (DINT) |
| CTUD_DINT | CD, R, LD (BOOL), PV (DINT) | CV (DINT) |

NE takes two compatible scalar sources and an optional BOOL destination. MOD
requires compatible integer sources and a writable integer destination. AND,
OR and XOR take two compatible BOOL/bit-string sources; NOT takes one. Their
OUT destination may be omitted for BOOL wiring, but numeric results require a
destination. WORD_TO_UDINT takes a WORD device/local source and a writable UDINT
destination; native checking rejects a literal as this conversion's source.
Typed numeric literals use the same range validator as IEC declarations.

The editor offers these commands from the first blank column and supplies typed
instance suggestions. F5 wires Q, QU, QD, ENO and supported BOOL OUT pins directly.
Numeric ET/CV and arithmetic/conversion OUT retain their destination rules.

Rust `IecLdEditor::insert_function` and WASM `insert_xgwx_iec_ld_function` share
validation with MCP `insert_iec_ld_function`. Output wires use the corresponding
`insert_function_output_wire` route. The C JSON transaction API adds
`insert_iec_ld_function` and `insert_iec_ld_function_output_wire`, each requiring
`program_index` and `expected_object_id`; the remaining fields match the Rust
arguments. C batches remain atomic and MCP saves retain revision guards and
backups. The MCP package carries the implementation in its vendored library.

Native capture and compiler evidence, including exact Save As preservation, is
recorded in [the fixture README](../fixtures/iec-blocks/README.md).
