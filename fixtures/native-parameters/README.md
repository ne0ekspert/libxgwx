# Native XGB parameter acceptance

These fixtures are test derivatives of `../XGB_Enet01.xgwx`; the parent fixture
attribution and parser-development permission apply. They are not commissioning
projects or generally reusable PLC assets.

`generated.xgwx` was produced by `native_parameter_acceptance generate` using
`ParameterEditor::edit_native`. XG5000 **4.82.1.0**, build **2026-07-20**, opened
it in an offline Windows 10 copy-on-write VM on **2026-10-20**. The untouched
baseline and edited project both passed Check Program with all programs selected:
**0 errors, 0 warnings**. Native dialogs displayed the intended HSC, PID and
X-axis positioning values. Save As preserved the project name and produced
`resaved.xgwx`. The native file was recovered read-only from the isolated VM's
NTFS disk after the application had held an exclusive file lock.

Representative changes (indices below are zero-based):

| Section | Target | Field | Value |
| --- | --- | --- | --- |
| HSC | Channel 0 | Unit time | 2345 ms |
| HSC | Channel 0 | Pulses per revolution | 100 |
| PID calculation | Loop 0 | Target value | 123 |
| PID calculation | Loop 0 | Scan time | 200 |
| PID tuning | Loop 1 | Target value | 234 |
| Positioning | Axis X, step 0 | Target position | 12345 |
| Positioning | Axis Y, step 29 | Dwell time | 100 |

Host parsing compared every exposed native field, including unchanged fields:
16 HSC, 342 PID calculation, 114 PID tuning and 792 positioning fields all matched
exactly after Save As. The files differ in native formatting/container data;
whole-file byte identity is not the acceptance criterion.

Reproduce the generated input and compare a fresh native resave:

```sh
cargo run --manifest-path dev-tools/Cargo.toml --bin native_parameter_acceptance -- generate /tmp/native-parameters
cargo run --manifest-path dev-tools/Cargo.toml --bin native_parameter_acceptance -- check /tmp/native-parameters/edited.xgwx /path/to/native-resaved.xgwx
```

Hashes, field counts and diagnostics are recorded in `acceptance.json`. This is
representative persistence evidence, not validation of every raw integer domain,
every CPU, tentative HSC fields, or live PLC execution.
