# CPU-aware hardware validation

The writer now rejects CPU changes that would reinterpret XGK module IDs as
compact hardware. The first captured compact profile is XGB-XBMS / XBM-DR16S;
other XGB variants remain unverified and are not assigned this layout.

## Supported operations

- Selecting the existing CPU type leaves the document unchanged.
- XGK-to-XGK changes require retained bases and catalog modules to fit the
  target CPU's base and slot limits. Unknown module identities are rejected.
- Cross-family and compact-model changes require migration and are rejected.
- XGK catalog selection, insertion, option reading/writing, and the typed input
  filter writer require an XGK configuration. Numeric module IDs alone cannot
  establish compatibility.
- Compact hardware deletion and identity changes are rejected. Comments remain
  editable. Raw compact `Details` writes are rejected until their variant's
  settings encoding is verified; this does not mean the native settings are
  immutable.
- Raw XGK identity patches must match the catalog and obey module slot spans.
- Hardware operations require exactly one configuration. Comment-only patches
  retain the existing unique base/slot target behavior.

`XgwxDocument::cpu_hardware_profile()` recognizes the captured compact variant
using configuration type and module identity/name. WASM summaries expose it as
`hardware.cpuProfile`; null means unverified, not an absence of fixed hardware.
The existing CPU catalog remains a model/type mapping, not a claim that every
listed CPU supports hardware migration.

## Native observations, 2026-09-09

Offline XG5000 VM, using `fixtures/XGB_Enet01.xgwx`:

| Record | Position | Identity |
| --- | --- | --- |
| Main I/O | Base 0, slot 0 (main) | XBM-DR16S, Id 42249, SubType 1 (source), 0 (native resave) |
| Built-in Cnet | Base 0, slot 0 | Id 23104, OptionType 32773 |
| Expansion Ethernet | Base 0, slot 1 | XBL-EMTA |

The native I/O table labels slot 0 as main and shows XBM-DR16S. Double-clicking
opens filter, pulse-catch and emergency-output settings. That dialog's static
model label says XBC-DR16A; no additional variant equivalence is inferred from
that label. The UI shows expansion positions, but their complete allowed model
catalog and limits are not yet captured. XGK expansion models are not offered
as substitutes.

The unchanged compact baseline passed Check Program with 0 errors, 0 warnings,
and 13 messages. The generated `CompactAcceptance` comment was visible on main
I/O and passed the same check. Native Save As comparisons are recorded below.

## Validation limits

These guards validate the supported hardware operations and position limits;
they do not prove program instruction compatibility, all CPU-specific opaque
parameters, electrical compatibility, or PLC execution. XGK CPU changes still
need native program checks. Compact expansion editing and built-in settings
writers need further native captures before they can be enabled.

## Save As results

| Case | Native check | Save As / reparse |
| --- | --- | --- |
| Unchanged compact control | 0 errors, 0 warnings | Built-in SubType changes 1 → 0 |
| XBM-DR16S built-in comment | 0 errors, 0 warnings | Comment persists; same SubType normalization as control |
| XGK-CPUSN → XGK-CPUHN | 0 errors, 0 warnings | CPU, modules and networks match |

The compact comparator normalizes only the recognized main I/O SubType 1/0
and the previously verified volatile network-name pointers. All remaining
module and network values match. The production writer does not normalize
SubType, and both observed encodings retain built-in protection. Original VM
inputs C00, C01 and M01 remained byte-identical to their host sources.

Generated inputs, native resaves R00/R01/R02, screenshots, the verifier snapshot,
results and SHA-256 checksums are retained locally under
`target/xg5000-cpu-acceptance-20260909/` (ignored build artifacts). The earlier
2026-09-08 acceptance records remain historical evidence. No PLC was connected.

## SFC CPU changes, 2026-10-08

`select_cpu` supports switches among XGI-CPUE, CPUS, CPUH, CPUU, CPUU/D and
CPUUN for supported SFC-only workspaces with captured default parameters and
empty I/O tables. All 30 directional changes and six no-op selections are
covered by Rust and WASM tests, including ST source and declaration preservation.

| Models | Configuration types | M_AREA_SIZE_0..3 | Default latch ends |
| --- | --- | ---: | ---: |
| CPUE | 106 | 32 | 8191 |
| CPUS | 104 | 64 | 16383 |
| CPUH / CPUU / CPUU/D | 102 / 100 / 107 | 256 | 65535 |
| CPUUN | 111 | 512 | 131071 |

Values are the native parameter fields, not independently derived memory limits.
The writer updates the captured CPUUN capability bit (`0x8000`) and clears
`0x80000` after a model change, matching native CPU changes. Native Check Program
and Save As set `0x80000` on all six models; same-model selections preserve it.
Other configuration flags are preserved. The eight empty base
placeholders remain, matching native XG5000. CPUUN also adds the captured default
`FENET PARAMETER`, empty `MotionParamInfo`, and one workspace tree node; switching
away removes those defaults and decrements `WksNodeCount` with checked arithmetic.
No customized Ethernet or motion configuration is discarded.

Native XG5000 4.82.1 warns that CPU changes reset all parameters. The writer
therefore rejects custom or unknown parameters rather than resetting them.
Only captured sets of the five `OUTPUT_PARAMETER_RESERVED_0..4` fields are
accepted; their bytes are preserved and their meaning remains unverified.
Configured I/O/network modules, CPUS/P, cross-family changes and unsupported
SFC layouts remain unavailable.

`fixtures/sfc/cpu-*-native.xgwx` captures native PLC Properties changes and
model defaults. `sfc_cpu_acceptance` generates all six models from the typed
ST fixture, or accepts a second argument naming a native-saved input project.
Generated CPUH, CPUU, CPUU/D and CPUUN projects each passed strict all-program
Check Program with **0 errors, 0 warnings, 23 messages**, joining the previously
validated CPUE/CPUS pair. The native-saved CPUUN → CPUE conversion passed the same
check after removing the default Ethernet and motion sections. Native Save As
results are `cpu-*-roundtrip.xgwx`; tests compare chart rows, ST source, primitive
and FB declarations, CPU identities and continued editability.

Switching a generated CPUE project through any supported model and back restores
its XML except for the cleared validation-state bit, which native checking sets
again. Recreating CPUUN-only default sections in a native-saved project
can normalize parameter whitespace; source code and settings remain intact.
Acceptance is offline in XG5000 4.82.1, with no PLC connected. Configured hardware,
custom parameter migration and PLC execution are outside this validation.
