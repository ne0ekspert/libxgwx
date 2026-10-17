# Basic-operation parameters: native evidence

Source: XG5000 Manual V3.1, October 2025 Korean edition, PDF pages 353–354
(section 9.1). Scope: the **basic-operation** tab. Device/memory-area,
error-operation, MODBUS and system-option tabs are preserved, not edited.

Captures were made offline in XG5000 4.82.1.0 on 2026-10-17, using an isolated
QCOW2 overlay. They are controlled blank projects, not user production files.

- `xgk-native-controls.xgwx`: XGK-CPUSN; fixed cycle 321 ms, watchdog 777 ms,
  filter 100 ms, all ordinary switches enabled except debugging output;
  earliest SOE retention; full Reset and D.CLR blocking.
- `xgi-native-overall.xgwx`: XGI-CPUUN; fixed cycle 246 ms, watchdog 678 ms,
  filter 100 ms, cold restart, earliest SOE retention, independent Overall-only
  Reset/D.CLR blocking. Hidden watchdog slots 1–3 remain 500 ms.
- `xgi-native-upper.xgwx`: XGI-CPUUN; fixed cycle 999 ms, watchdog 1000 ms,
  filter 1 ms, warm restart, default output controls, full-switch blocks.

## Captured encodings

| Field | Checked / selected | Unchecked / alternative |
| --- | ---: | ---: |
| STATIC_PERIOD_MODE | 43520 | 0 |
| IO_POINT_METHOD (XGK) | 43521 | 0 |
| DEBUG_OUTPUT_PARAMETER | **0** | **43536** |
| ERROR_MODE_OUTPUT_PARAMETER | 43537 | 0 |
| RUN2STOP_OUTPUT_PARAMETER | 43538 | 0 |
| STOP2RUN_OUTPUT_PARAMETER | 43539 | 0 |
| ERROR_MODE_LATCH_PARAMETER (XGK) | 43540 | 0 |
| RESET_DISABLE / O_RESET_DISABLE | 1 | 0 |
| DCLEAR_DISABLE / O_DCLEAR_DISABLE | 1 | 0 |
| SOE_RETAIN_HISTORY | 43544 = overwrite latest | 0 = preserve earliest |
| RESTART_METHOD (XGI) | 43521 = warm | 0 = cold |

Fixed-cycle duration accepts 1–999 ms; the visible watchdog accepts 10–1000 ms.
Input-filter choices are 1, 3, 5, 10, 20, 70 and 100 ms in both native dialogs.
The XGI dialog's single visible watchdog modifies **only** `SCAN_WD_TIME_0`.

Full Reset/D.CLR blocking checks and disables the corresponding Overall block.
Clearing a full-switch block clears its Overall block; the Overall block can
then be enabled independently. The writer rejects inconsistent final pairs.
Unknown switch/choice codes are preserved and omitted from editable controls.

The acceptance generator/checker is `dev-tools/basic_parameter_acceptance.rs`.
It uses nonempty XGK ladder and IEC ST programs and compares every basic XML
attribute and decoded ProgramData payload after native Save As.

## Native acceptance results

All six cases below were generated through `edit_basic_operation`, opened in
XG5000, inspected in the native dialog, checked with all-program logic/syntax
checks (strict type checks for XGI), and saved using **Save As**. Each native
result had **0 errors and 0 warnings**. The checker confirmed equality of all
basic attributes and decoded ProgramData bytes, including hidden watchdog slots.

| Case | CPU | States / boundaries | Result |
| --- | --- | --- | --- |
| xgk-on | XGK-CPUSN | All checkboxes on; 999/1000 ms; filter 100 | Pass |
| xgk-off | XGK-CPUSN | All checkboxes off; 1/10 ms; filter 1 | Pass |
| xgk-overall | XGK-CPUSN | Overall-only blocks; 321/777 ms; filter 5 | Pass |
| xgi-on | XGI-CPUUN | All checkboxes on; warm restart; 999/1000 ms | Pass |
| xgi-off | XGI-CPUUN | All checkboxes off; cold restart; 1/10 ms | Pass |
| xgi-overall | XGI-CPUUN | Overall-only blocks; cold restart; 321/777 ms | Pass |

Each case has paired `*-generated.xgwx` and `*-resaved.xgwx` fixtures.
`acceptance.json` records their hashes and results. Regenerate or check a pair:

```sh
cargo run --manifest-path dev-tools/Cargo.toml --bin basic_parameter_acceptance -- generate XGK-CPUSN on /tmp/KON.xgwx
cargo run --manifest-path dev-tools/Cargo.toml --bin basic_parameter_acceptance -- check fixtures/basic-parameters/xgk-on-generated.xgwx fixtures/basic-parameters/xgk-on-resaved.xgwx
```

This is offline file/editor acceptance, not execution on a connected PLC.
Other CPU families and parameter tabs are outside this implementation's scope.

## Editor and regression acceptance

- Native VS Code 1.139.1 development host: actual XG5000 Explorer navigation,
  Apply, native Undo, native Redo, Save, editor close and reopen all passed.
  Apply left the disk unchanged until Save. `native-vscode-saved.xgwx` is the
  `xgk-on-generated.xgwx` input with watchdog changed from 1000 to 650 ms.
  Inflated XML is identical except for that one attribute.
- Final bundled WASM: complete forms for XGK-CPUSN and XGI-CPUUN passed browser
  interaction checks at 1200×900 and 720×900 with no browser errors. The checks
  exercised numeric rejection, inverse debug-output encoding, filter/history
  choices, Reset/Overall dependencies, Apply and reopen.
- 409 Rust tests and 15 documentation tests passed. The 135 existing tests that
  require separately supplied/private captures remained ignored; all new basic
  parameter tests ran. The long repository suite was completed in target groups.
- `npm run check` and all 51 extension test files passed. Source formatting,
  default-feature compilation and diff whitespace checks passed.
