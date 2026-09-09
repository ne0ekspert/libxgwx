# XG5000 writer acceptance

Local parser round trips establish serialization consistency. External
acceptance additionally requires XG5000 to open the output, display the intended
values, check the program, and preserve the edit through Save As.

## Reproduce

Run from the repository root with a new output directory:

```sh
cargo run --features write --example xg5000-acceptance -- generate /tmp/xgwx-run
sha256sum fixtures/elements*.xgwx /tmp/xgwx-run/*.xgwx > /tmp/xgwx-run/SHA256SUMS
```

The generator refuses an existing directory. Each case includes its source,
edited workspace, expected semantic values, and an entry in `matrix.tsv`.
Generated local passes leave every external stage `PENDING`.

1. Record the library revision, dirty patch, fixture hashes, and XG5000 About
   version. Keep generated files and resaved outputs separate.
2. Open each source in XG5000 and establish its check/build diagnostics before
   attributing any errors in the edited version to the library.
3. Open the edited file, inspect the target, and capture the relevant dialog.
4. Use View > Check Program (Korean: 보기 > 프로그램 검사), select all programs,
   and compare diagnostics with the source. Opening a
   workspace is insufficient evidence of a valid CPU/hardware/program combination.
5. Use Project > Save As, turn off the option to rename the project, and choose
   a distinct output name. This XG5000 version creates a subdirectory containing
   the resaved workspace.
6. Transfer the result to the host and run:

```sh
cargo run --features write --example xg5000-acceptance -- \
  verify filter /tmp/xgwx-run/filter.xgwx /tmp/xgwx-run/resaved/R01.XGWX
```

The verifier compares the relevant decoded section. It does not certify the
entire project, all opaque payloads, UI acceptance, compilation, or PLC behavior.
Network comparisons ignore only observed volatile `@0x<hex>` suffixes in
internal module names, attribute order, and module/configuration order keyed
by base and slot. The unchanged baseline proved that XG5000 regenerates these
suffixes. Model names, aliases, descriptions, identities, and parameter values
remain part of the comparison.

`module-insert` uses a baseline with base 0 / slot 2 removed. Validate that
derived baseline (also the `module-delete` output) before assessing insertion.
The historical `cpu-cross-family` case below captured the unsafe type-only
conversion. The writer now rejects it; the current generator replaces it with
`compact-comment`. CPU verification now also compares retained hardware and
network records. See [CPU hardware validation](cpu-hardware-validation.md).

## Run: 2026-09-08

- Library baseline: `c6414f441e577e728011e66aae75e2fa2242da5b` plus this
  acceptance harness; no production writer changes at the start of the run.
- Application: XG5000 **4.82.1.0**, About dialog build date **2026-07-20**.
- Environment: existing offline Windows 10 QEMU VM; separate writable transfer
  media, original tools media retained read-only.
- Cases, resaves, verification logs, screenshots, and hashes:
  `target/xg5000-acceptance-20260908/` (ignored local artifacts; archive this
  directory before `cargo clean`).
- Local checks: all 14 generated cases reparse with matching target values;
  unchanged output is byte-identical. Rust suite: 55 passed, 1 ignored.
  The typed input-filter helper and `set_module_option(..., "inputFilter", 0, 5)`
  produce identical bytes for the filter case.

Source SHA-256 values:

```text
0b93e67844a9ec8e9baac33f625440c4808c2f23e54d980ce180cc885c1653ab  fixtures/elements.xgwx
535b5f46766b78d3051550466d436911252e4ced40645f0b948ae25efe064f18  fixtures/elements-io.xgwx
```

### Results

**12 of 14 representative cases pass the tested stages**, including the
unchanged control. Every file opened and saved successfully. Thirteen retained
their target values; thirteen passed Check Program. The two failures are
different cases, described below.

| Case | UI target | Check Program | Resaved target | Result |
| --- | --- | --- | --- | --- |
| Unchanged control | Matches | 0 errors | Matches | PASS |
| Input filter, 5 ms | Matches | 0 errors | Matches | PASS |
| Module comment | Matches | 0 errors | Matches | PASS |
| Replace with XGF-RD8A | Matches | 0 errors | Matches | PASS |
| Delete base 0 / slot 2 | Matches | 0 errors | Matches | PASS |
| Insert XGF-RD8A into empty slot | Matches | 0 errors | Matches | PASS |
| Replace with XGL-EDMF and companion configuration | Matches | 0 errors | Matches | PASS |
| Rename default network | Ignored | 0 errors | Reverted | **FAIL** |
| Network module alias | Matches | 0 errors | Matches | PASS |
| XGK-CPUSN → XGK-CPUHN | Matches | 0 errors | Matches | PASS |
| XGK-CPUSN → XGB-XBMS | Matches | **5 errors** | CPU type matches | **FAIL** |
| Program rename | Matches | 0 errors | Matches | PASS |
| Ladder operand M00000 → M00042 | Matches | 0 errors | Matches | PASS |
| Variable rename | Matches | 0 errors | Matches | PASS |

All checks reported zero warnings. Both unchanged source fixtures passed
Check Program with zero errors. The small `elements.xgwx` source was also
saved separately as `resaved/R14.XGWX`; the hardware fixture's unchanged save
is `resaved/R00.XGWX`.

The [machine-readable result ledger](xg5000-acceptance-20260908.tsv) records
input and resaved SHA-256 values for each case. The generator reproduced all
28 source/edit files byte-for-byte, and all transfer-media inputs still matched
before the final program-check pass. After final checking and VM shutdown,
XG5000 had also rewritten the active `A09.XGWX` in place; its selected CPU type
still matches. The other 27 inputs remained unchanged. Original generated
files, the earlier independent `R09.XGWX` save, and before/after transfer copies
are preserved separately. All 15 independent resaves retained their recorded
hashes after shutdown. The verifier accepts
the unchanged network normalization but rejects both the lost network name
and an unchanged file substituted for the edited filter case.

This is representative coverage of existing writer paths. It does not establish
support for every patch field, catalog module/option, CPU combination, file
layout, or XG5000 version. The CPU cases use the small program fixture, not a
fully populated hardware configuration. PLC download and execution were not
tested. No production writer behavior was changed during this acceptance pass.

## Confirmed gaps

### Default-network rename is discarded

`update_network(0, NetworkPatch { name: Some("AcceptanceNetwork"), ... })`
changes the XML locally, but XG5000 displays and resaves `기본 네트워크` for the
fixture's `Network Type="NETWORK ITEM:UNKNOWN"`. The host verifier reports
the changed name as a mismatch. The same baseline's module alias edit survives.

The writer currently presents this name as editable despite its lack of
external persistence. Before claiming support, determine which network kinds
have editable names and reject changes to names managed by XG5000. This result
does not establish that every network name is immutable.

### Cross-family CPU selection can produce invalid programs

Selecting `XGB-XBMS` on the XGK fixture opens successfully and preserves CPU
type `2` on Save As, but Check Program reports **5 errors and 0 warnings**.
The unchanged XGK source reports **0 errors and 0 warnings**. The error list
includes unsupported `XDST` (`E4200`) and unsupported ladder instructions
(`G0005`). XG5000 also adds built-in XGB configuration when loading this case.

CPU type persistence therefore cannot establish compatibility. CPU changes
need instruction, address, hardware, and parameter compatibility validation
or an explicit conversion path before they can be treated as supported.

### Compact PLCs require model-specific built-in hardware rules

Some CPU models have fixed built-in I/O and communication hardware. The CPU
catalog currently records only model, family, type code, and base/slot limits;
it does not describe built-in modules, reserved positions, or allowed expansion
modules. Existing module placement checks cannot enforce these distinctions.

The existing `fixtures/XGB_Enet01.xgwx` provides a concrete example for CPU type
`2` (`XGB-XBMS`): base 0 / slot 0 contains `XBM-DR16S` I/O, the network section
includes built-in Cnet at that position, and slot 1 contains an `XBL-EMTA`
expansion module. This is evidence for that fixture, not a complete mapping of
every compact CPU variant.

The cross-family case also exposes hardware reinterpretation beyond its five
program errors. Comparing the generated input with `resaved/R10.XGWX` shows:

| Position | Generated XGK module | XG5000 XGB resave |
| --- | --- | --- |
| Base 0 / slot 0 | XGI-D24A/B, Id 42242 | XBE-DC32A, same Id |
| Base 0 / slot 1 | XGQ-TR4A/B, Id 42286 | XBE-TN/TP32A, same Id |
| Base 0 / slot 2 | XGF-PN4B, Id 23264 | Removed |
| Built-in communication | Absent | Cnet added at base 0 / slot 0 |

The CPU verifier intentionally checks only the selected type. Its target-value
pass must not be interpreted as hardware preservation. Module IDs alone are
not sufficient to identify hardware across CPU families.

The next hardware model must distinguish built-in hardware from expansion
positions for each supported CPU/variant. It should prevent deleting,
replacing, or overlapping fixed hardware while permitting its verified
configurable parameters. CPU changes must validate or explicitly migrate the
complete hardware configuration and companion network records. Capture native
XG5000 baselines for each variant before defining these rules; do not infer
that every XGB CPU shares one fixed slot layout.
