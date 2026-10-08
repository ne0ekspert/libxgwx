# Native SFC acceptance fixtures

These synthetic projects were constructed in offline XG5000 4.82.1 on an
XGI-CPUE on 2026-10-08. They contain no user production program.

- `native-loop.xgwx`: `Start` label → initial step `S0` → `%MX0` transition →
  step `S1` with comment `Ready for cycle` → `%MX1` transition → `Start` jump.
- `edited-native-roundtrip.xgwx`: library/UI changes `%MX0` to `%MX2` and the
  step comment to `Cycle active & ready`, reopened and saved by XG5000.
- `native-action.xgwx`: the edited chart with an `N (Non stored)` variable
  action `%MX10` attached to `S0`.

Strict all-program Check Program had logic, syntax, duplicate-coil and strict
type checks enabled: all three charts reported **0 errors, 0 warnings**.
The generated edit and XG5000 Save As result have identical SFC entity trees
and local variable tables. The UI harness produced exactly the same project
bytes as `cargo run --features write --example sfc_acceptance`.

SFC uses `SFC_ProgramList` XML, not binary ladder `ProgramData`. Transition
properties are mirrored in the adjacent type-9 annotation entity; both copies
must be updated together. The parser retains unknown entity kinds/attributes.

Supported writes are step comments and existing variable transitions
using direct `%MX` BOOL operands. Step names, qualifiers, program-backed
transitions/actions, nested block references, and structural changes are read
only. Their symbol-table and topology effects have not been validated for writes.

SFC local symbols are preserved as native XML/payload data; their binary record
format is not decoded by the IEC ladder local-variable decoder.
