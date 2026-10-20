# Native FEnet, HSC, PID and positioning edits

`ParameterRef::native_fields()` lists supported existing fields with a record
selector, exact expected value, and integer bounds. With feature `write`,
`ParameterEditor::edit_native()` applies a batch atomically to that selected
parameter. Indices are document-wide parameter indices, with zero-based channel,
loop, axis and step indices inside each parameter.

```rust
# fn edit(doc: &mut xgwx::XgwxDocument) -> Result<(), xgwx::XgwxError> {
use xgwx::{NativeParameterChange, NativeParameterTarget, ParameterKind};
let parameter = doc.parameter().of_kind(ParameterKind::PidCalculation)[0];
let index = parameter.index();
let field = parameter.native_fields().into_iter().find(|field|
    field.target == NativeParameterTarget::PidCalculationLoop { loop_index: 0 }
        && field.name == "TargetValue").unwrap();
doc.parameter_mut().get(index)?.edit_native(&[NativeParameterChange {
    target: field.target,
    name: field.name,
    expected: field.value,
    replacement: 100,
}])?;
let bytes = doc.to_verified_bytes()?;
# Ok(()) }
```

The following XGB layouts come from `fixtures/XGB_Enet01.xgwx` and `XGB_Enet02.xgwx`:

| Family | Writable fields | Layout guard |
| --- | --- | --- |
| HSC | `counter_mode`, `pulse_input_mode`, `unit_time_ms`, `pulses_per_revolution` for each channel | 224 decoded bytes, declared 448 hex digits, ASCII hex/whitespace |
| PID calculation | Decoded settings and loop attributes | Header words 17736/17473, `ParaSize=0`, one `EmbedPidLoopDataList` with 16 direct loop records |
| PID tuning | Decoded settings and loop attributes | Footer words 12358/21552, `CheckSum=0`, one `PidTunLoopDataList` with 16 direct loop records |
| Positioning | Decoded axis attributes and table-step attributes | One two-axis group, matching axis-data/axis-parameter records, matching declared step counts |

For HSC/PID/positioning, an owned record must belong to a recognized XGB configuration. Unowned records
can be edited when their layout matches. Unsupported or ambiguous layouts stay
read-only. No sections, axes, loops or steps are created, removed or resized.

HSC counter modes accept 0/1 and pulse input modes accept 0 through 3, matching
the existing decoder. The timing and revolution fields accept 1 through 60000.
Edits change only the selected header nibble or little-endian word, preserving
all other digits and whitespace. The tentative compare mode, preset bytes,
ring counter and comparison thresholds remain read-only pending native mapping.

PID and positioning changes use native XML attribute names and the existing
readers' signed or unsigned 32-bit integer domains. These are raw storage edits:
enum values, settings masks, split gain components and PWM bit offsets are not
converted to labels, booleans, floating-point gains or formatted device addresses.
The integer bounds are not CPU-specific operating limits. Callers must supply
values and related fields appropriate for their CPU and application.

Every change requires the exact inspected value. Missing fields, invalid current
integers, unknown field names, duplicate targets, stale values and replacements
outside the advertised integer domain fail before mutation. An identical numeric
replacement leaves the original attribute spelling intact. Headers, sizes,
checksums, footer words, reserved bytes/attributes, unknown XML and other parameter
records are preserved. Nonzero tuning checksums are unsupported; no checksum
algorithm is assumed. The workspace container still uses the existing verified
serializer and its header-checksum/alignment handling.

## C and WASM

C inspection section `parameters` includes `native_fields` on each item. The
existing atomic JSON transaction accepts:

```json
{
  "operation": "edit_native_parameters",
  "parameter_index": 3,
  "changes": [{
    "target": {"kind": "pid_calculation_loop", "loop_index": 0},
    "name": "TargetValue",
    "expected": "0",
    "replacement": 100
  }]
}
```

Target kinds are `safety_communication`, `hsc_channel` (`channel`), `pid_calculation`,
`pid_calculation_loop` (`loop_index`), `pid_tuning`, `pid_tuning_loop`
(`loop_index`), `position_axis` (`axis_index`), and `position_step`
(`axis_index`, `step_index`). Unknown JSON fields are rejected.

WASM exports `inspect_xgwx_native_parameter_fields(bytes, parameter_index)` and,
with `write`, `edit_xgwx_native_parameters(bytes, parameter_index, changes)`.
Both use the same snake_case record and field format shown above. The edit
function returns serialized workspace bytes through the existing WASM session.

## Validation boundary

Regression tests edit the supported fields in both native XGB fixtures and
verify serialization/reparsing, exact record selection, unchanged other records,
HSC byte offsets/whitespace preservation, no-op byte identity, rejection and
rollback. C tests compare the transaction result to Rust and verify rollback
when a later operation fails. These checks establish library/container behavior;
representative edited fields are also exercised through native XG5000 open,
Check Program and Save As. See [native acceptance fixtures](../fixtures/native-parameters/README.md) for the
capture and comparison. No live PLC operation is performed.

## Legacy FEnet and I/O parameter sections

`FENET PARAMETER` uses a single direct `Safety_Comm` record, separate from
module `XGPD_CONFIG_INFO_FENET` settings. On captured XGK layouts,
`native_fields()` exposes its decoded timeouts, socket count, driver code,
host-table flag and IPv4 attributes with target `safety_communication`.
`IPAddress`, `Gateway` and `Subnet` store little-endian `u32` addresses;
for example `10.20.30.40` is `673059850`. Subnet replacements require
contiguous leading ones. The native dialog bounds receive wait to 2–255 seconds, retransmission to
1–600 counts of 10 ms, and dedicated connections to 1–4. Driver codes are
2 (XGT server) and 5 (Modbus TCP/IP server). Channel mappings, host entries
and unknown fields are preserved.
VS Code displays dotted IPv4 controls, bounded integer controls and selectors.

`IO PARAMETER` already uses the module/base editors. The parameter handle now
exposes `io_module_indices()` and `io_base_indices()` for records within that
exact section, preserving ownership even when PLC coordinates repeat.
C/WASM/MCP inspection includes these indices. VS Code omits I/O sections from
Parameters; module properties/options and base slot counts are edited in Hardware.
MCP uses the
existing `modules` property IR for module edits and `set_base_slot_count` for
base edits, retaining the existing transaction and backup behavior.

Module FEnet timeout units are also writable through the existing FEnet editor:
`rcvWaitTimeUnit` and `clientWaitTimeUnit` use 0 for seconds and 1 for 10 ms,
as established by the native Modbus/Smart fixtures. VS Code provides selectors;
MCP includes the fields in its `networks` property IR. Unknown units stay
read-only in VS Code; missing attributes are never created.

Native XG5000 4.82.1.0 acceptance on 2026-10-20 is retained under
`fixtures/fenet-io-parameters/`. Save As preserved all eight exposed legacy
FEnet fields, I/O module settings, base records, channel mappings, host table
and ladder payload. A second capture confirms Modbus TCP/IP DriverType 5.
The native program check reported zero errors and zero warnings. The webview
was exercised with real WASM in a browser harness at desktop and narrow widths;
this does not establish native VS Code host interaction or live PLC operation.
