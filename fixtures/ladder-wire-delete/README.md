# Native wire-deletion captures

Capture directory names below identify local validation archives; they are not
paths bundled with this repository.

Synthetic XGK-CPUSN and XGI-CPUE projects, checked in the offline XG5000 VM
on 2026-10-07. They contain only direct-memory contacts and coils.

| Fixture | Change | Native evidence |
| --- | --- | --- |
| xgk-horizontal-native.xgwx | Removed the first row's horizontal wire; kept both rows and their contacts/coils | Opened and rendered the gap; Save As retained the generated ProgramData byte-for-byte |
| xgk-vertical-native.xgwx | Removed the x3 branch; kept x6 and both rows | Rendered remaining x6 branch; Check Program zero errors with the intentional duplicate-coil warning; Save As retained ProgramData byte-for-byte |
| iec-horizontal-native.xgwx | Removed the long wire between %MX0 and %MX1 | Rendered the gap; Check Program reported three expected input/output errors (L0000/L0401); Save As retained ProgramData byte-for-byte |

These captures establish editor/serialization compatibility. A disconnected
circuit is not a runnable program. No PLC connection or download was performed.
`tests/ladder_wire_delete.rs` reproduces all three payloads exactly and verifies
preserved elements/rows, unaffected branches and atomic stale-selection rejection.
Isolated IEC contact/coil deletion is covered by local structural tests.

Native projects, generated inputs, payloads and screenshots are retained under
`ladder-delete-20261007/`.
