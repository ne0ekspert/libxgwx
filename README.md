# libxgwx

Rust parser for LS XG5000 `.xgwx` workspace files.

The parser handles the container layout observed in LS XG5000 workspace files:

- `XG` binary workspace header
- gzip-compressed UTF-8 XML project payload
- optional trailing binary metadata

Unknown binary sections are preserved so callers can inspect them later.

## Usage

```rust
use xgwx::XgwxDocument;

let doc = XgwxDocument::from_path("project.xgwx")?;
let project = doc.project_info();

println!("header label: {:?}", doc.header.label);
println!("project name: {:?}", project.name);
println!("file version: {:?}", project.file_version);

for module in doc.modules() {
    println!(
        "module base={:?} slot={:?} id={:?} name={:?}",
        module.base,
        module.slot,
        module.id,
        module.name
    );
}

for program in doc.programs() {
    println!("program: {:?}", program.name);
}

# Ok::<(), Box<dyn std::error::Error>>(())
```

## API

- `XgwxDocument::parse(&[u8])` parses from bytes.
- `XgwxDocument::from_path(...)` parses from a file.
- `XgwxDocument::project_info()` returns root project metadata.
- `XgwxDocument::configurations()`, `networks()`, `network_modules()`, `bases()`,
  `modules()`, `tasks()`, `programs()`, `high_speed_links()`, and
  `high_speed_link_blocks()` return owned high-level summaries.
- XGI-D24A/B hardware module summaries decode the first `Details` byte as the
  digital input filter: default (`0`) or `1`, `3`, `5`, `10`, `20`, `70`, or
  `100` milliseconds. Unknown raw values are preserved.
- `XgwxDocument::variables()` decodes the compressed global symbol table into
  variable summaries with name, formatted address, memory area, address number,
  data type, description, source reference, and range.
- `XgwxDocument::ladder_programs()` partially decodes base64/bzip2 ladder
  `ProgramData`, preserving raw bytes and extracting embedded strings and likely
  ladder elements such as instruction calls, comparisons, timers, logic
  operators, device references, constants, comments, and internal references.
- With the opt-in `il` feature, `LadderProgramData::to_il()` converts positioned
  LD structure to typed, one-step-per-line instruction list output. The related
  `to_il_with_variable_names(...)` method resolves unique exact address matches
  through the decoded symbol table.
- With the opt-in `write` feature, `XgwxDocument::update_module(...)` applies
  token-preserving in-memory changes to one module selected by base and slot.
  The first module patching scope is limited to existing `Id`, `SubType`,
  `Name`, `Comment`, and `Details` attributes;
  `set_module_input_filter(...)` provides a typed XGI-D24A/B helper.
- The same `write` feature exposes `update_program(...)` for existing program
  metadata and `update_ladder_cell_text(...)` for decoded ladder strings. A
  ladder replacement must keep the same UTF-16 length so proprietary record
  offsets and undecoded topology bytes remain unchanged.
- `update_variable(...)` edits one decoded global symbol record by document
  order. Name, address area, data type, and description replacements must keep
  their UTF-16 length; the numeric address is updated in place.
- `ladder_mnemonic_info(...)` and `known_ladder_mnemonics()` expose category
  and description metadata for known ladder instruction mnemonics.
- `XgwxDocument::project_options()`, `parameters()`, `hsc_parameters()`,
  `safety_comm()`, `trend_monitoring()`, `xgpd_config_infos()`,
  `cnet_config_infos()`, `fenet_config_infos()`, and `properties()` expose
  other high-level project sections found in the XML.
- `XgwxDocument::position_parameters()` exposes X/Y position-control axis
  parameters and step tables from `POSITION PARAMETER` sections.
- `XgwxDocument::pid_cal_parameters()` and
  `XgwxDocument::pid_tune_parameters()` expose embedded PID loop calculation and
  tuning settings.
- `XgwxDocument::cnet_config_infos()` exposes Cnet module and serial port
  settings. It preserves raw values and decodes known enums:
  `Mode` (`0` = RS232C, `1` = RS422, `2` = RS485), `DataBit`
  (`0` = 7 bits, `1` = 8 bits), `StopBit` (`0` = 1, `1` = 2), and
  `Parity` (`0` = NONE, `1` = EVEN, `2` = ODD). `Bps` is also exposed as a
  decoded baud rate using `Bps * 1200`.
- Cnet DI/DO/AI/AO device areas are decoded from ASCII numeric device codes.
  DI/DO addresses are formatted as bit addresses with LSD hex notation, while
  AI/AO addresses are formatted as decimal word addresses.
- `XgwxDocument::fenet_config_infos()` exposes FEnet module IPv4 settings such
  as IP address, subnet, gateway, and DNS.
- `XgwxDocument::hsc_parameters()` decodes XGB `HSC PARAMETER` `PAYLOAD`
  attributes, preserving raw bytes and exposing the known per-channel counter
  mode (`0` = Linear Counter, `1` = Ring Counter) and pulse input mode
  (`0` = 1-Phase 1-Input 1x, `1` = 1-Phase 2-Input 1x, `2` = CW/CCW,
  `3` = 2-Phase 4x), plus compare output mode, internal/external preset byte
  fields, ring counter maximum, compare output minimum, compare output maximum,
  unit time in milliseconds, and pulses per revolution values. Compare output
  modes map `0..=6` to Less Than, Less Or Equal, Equal, Greater Or Equal,
  Greater Than, Includes, and Excludes.
- `XgwxDocument::decoded_payloads()` returns an inventory of base64 binary
  payloads with XML path, compression flag, encoded length, raw length, decoded
  length, attributes, and decoded bytes.
- `XgwxDocument::xml` contains the inflated XML payload.
- `XgwxDocument::root` contains a lightweight owned XML tree.
- `XgwxDocument::trailer` contains raw bytes after the main XML payload.
- `XgwxDocument::trailer_gzip_members` contains any valid gzip members found in the trailer.

## XGK Module Catalog

The library contains one native Rust catalog representing the latest stable
XG5000 module definitions. [`src/catalog_data.rs`](src/catalog_data.rs) stores
the 90 selectable XGK modules, including their model, category, `Id`,
`SubType`, full display name, default `Details` payload, and verified option
encodings. Definitions also record physical slot span; for example, XGF-TC4UD
occupies two consecutive base slots. It also retains every captured visible option, including
nested per-file and per-file-data controls, so consumers can show unmapped fields as read only.
The library does not load module JSON files at build time or
runtime, and catalog entries do not carry an XG5000 version identifier.

Placement-specific base and slot values are intentionally omitted. Catalog
updates replace the Rust definitions as the supported latest-stable snapshot
rather than adding a parallel versioned catalog.

With the `write` feature, `xgk_module_catalog()` exposes this snapshot and
`XgwxDocument::select_module(base, slot, model)` atomically selects a catalog
model. Selection preserves `Base`, `Slot`, and `Comment`, while replacing
`Id`, `SubType`, `Name`, and `Details` with the captured XG5000 defaults:

```rust,no_run
use xgwx::XgwxDocument;

let mut document = XgwxDocument::from_path("project.xgwx")?;
document.select_module(0, 2, "XGF-RD8A")?;
document.write_to("project-with-rd8a.xgwx")?;
# Ok::<(), xgwx::XgwxError>(())
```

Selection validates physical placement. Multi-slot modules are rejected if
they extend past the base or overlap a module in a following slot.
`XgwxDocument::delete_module(base, slot)` removes one uniquely identified
module while retaining the surrounding base and unrelated workspace data.
`XgwxDocument::insert_module(base, slot, model)` adds a catalog-default module
to an empty physical slot and applies the same base-capacity and overlap checks.
Insertion, replacement, and deletion keep the companion network entries in
sync for captured XGK communication modules, including Cnet, FEnet,
EtherNet/IP, BACnet, FDEnet, Dnet, and Rnet. Captured XGPD defaults are also
written for `XGL-EDMT`, `XGL-EDMF`, `XGL-DMEA/B`, and `XGL-RMEA/B`.

Catalog entries expose every captured dialog row through `visible_options`.
The writable `options` subset contains only fields whose `Details` byte mapping
and numeric choices have been verified. Use
`module_option_values` to read the current selections and `set_module_option`
to update one module-wide, channel, or group value. Unknown keys, indices, and
values fail without mutating the document.

The verified high-speed-counter subset includes the dropdown settings for
`XGF-HD2A`, `XGF-HO2A`, and `XGF-HO8A`; their numeric counter, comparison, and
frequency fields remain read only until their range and encoding rules are
mapped separately.

For example:

```rust,no_run
use xgwx::XgwxDocument;

let mut document = XgwxDocument::from_path("project.xgwx")?;
document.select_module(0, 2, "XGF-AD8A")?;
document.set_module_option(0, 2, "inputRange", 5, 6)?;
document.write_to("project-with-options.xgwx")?;
# Ok::<(), xgwx::XgwxError>(())
```

## LD to IL Conversion

Enable the optional `il` feature to convert decoded ladder programs. Literal
device addresses are retained by default:

```rust,no_run
use xgwx::XgwxDocument;

let doc = XgwxDocument::from_path("fixtures/elements.xgwx")?;
for program in doc.ladder_programs() {
    let il = program?.to_il()?;
    println!("{il}");
}

# Ok::<(), Box<dyn std::error::Error>>(())
```

Run that code with `--features il`. Each `IlStep` is either a rung comment or
an instruction, and `IlProgram` renders the steps separated by newlines;
embedded comment newlines are escaped. For symbolic operands, decode
`doc.variables()` and pass them to
`to_il_with_variable_names(...)`; addresses without one unique exact match are
left unchanged, as are symbols whose address or name is duplicated. Conversion
fails when the partial ladder decoder encounters an unknown record or
disconnected/unsupported topology rather than guessing.

Print the IL generated for a workspace with the included example:

```bash
cargo run --quiet --features il --example il -- fixtures/elements.xgwx
```

## Module Writing

For controlled XG5000 open/inspect/Save As checks, see the
[acceptance procedure and results](docs/xg5000-acceptance.md). The included
`xg5000-acceptance` example generates independent writer cases and verifies
their edited sections after an external save.

Enable the opt-in `write` feature to prepare attribute changes on an existing
module. The module must be uniquely identified by its current base and slot:

```rust,no_run
use xgwx::{ModuleInputFilter, ModulePatch, XgwxDocument};

let mut doc = XgwxDocument::from_path("project.xgwx")?;
doc.update_module(
    0,
    2,
    &ModulePatch {
        comment: Some("Updated from an external editor".to_owned()),
        ..ModulePatch::default()
    },
)?;
doc.set_module_input_filter(0, 2, ModuleInputFilter::Ms5)?;

# Ok::<(), Box<dyn std::error::Error>>(())
```

The patcher replaces only requested XML attribute values and preserves unknown
XML in memory. `to_bytes()` emits byte-identical data for an unchanged document.
After an edit it recompresses the main XML, applies XG5000's four-byte alignment,
updates the additive container checksum, validates the nested Security CRC64
frames, and preserves the existing Security metadata byte-for-byte. Layouts
outside the validated 138-byte XG5000 header format still fail closed with
`AuthenticatedRewriteUnsupported`.

When both `wasm` and `write` are enabled, the generated package also exposes
`update_xgwx_module(bytes, base, slot, patch)` and
`set_xgwx_module_input_filter(bytes, base, slot, rawFilter)`, plus
`update_xgwx_program(bytes, programIndex, patch)` and
`update_xgwx_ladder_cell(bytes, programIndex, offset, expected, replacement)`,
and `update_xgwx_variable(bytes, variableIndex, patch)`.
Network metadata is available through `update_xgwx_network(bytes, networkIndex,
patch)` and `update_xgwx_network_module(bytes, base, slot, patch)`. The latter
only changes the user-facing `ConfigName`, `Alias`, and `Description` fields;
hardware and protocol identity fields stay immutable.

The included example can be used as a serialization and XG5000 validation
harness:

```bash
cargo run --features write --example write-module -- \
  input.xgwx output.xgwx 0 2 5
```

## TUI PoC

Run the included example against a local workspace file:

```sh
cargo run --example tui -- path/to/project.xgwx
```

Use `j`/`k` or arrow keys to move through programs, and `q` or `Esc` to quit.
Use `Tab` to switch between program structure, network, variable, parameter,
and data views. Use `PageUp`/`PageDown` to scroll the right detail panel.

The TUI detail panels intentionally render complete details and rely on
scrolling instead of hiding entries behind `... more` counters. In the
Parameters view, `BASIC PARAMETER` attributes with numeric suffixes such as
`KEY_0`, `KEY_1`, and `KEY_2` are grouped under `KEY` with indented index rows.

The Networks view joins parsed Cnet and FEnet configuration records back to
network modules by stable module type only: `NetworkModule Id` equals
`XGPD_CONFIG_INFO_* Type`. It does not use base or slot for that association,
because those fields can be changed by users in XG5000 projects.

## Inspect Example

Run the non-interactive inspector for a concise text summary:

```sh
cargo run --example inspect -- fixtures/XGB_Enet01.xgwx
```

This prints project counts plus decoded Cnet, FEnet, HSC, position, and PID
summaries. It is useful for comparing parser output across fixture files without
opening the TUI.

## Benchmark Example

Run the fixture-backed benchmark in release mode:

```sh
cargo run --release --example bench -- --warmup 50 --iterations 500 fixtures
```

The benchmark reads `.xgwx` fixture bytes once, then measures parser-only work
and parser plus higher-level decoding paths such as variables, ladder programs,
parameters, network summaries, and decoded payloads.

## GUI Ladder PoC

Run the graphical ladder viewer:

```sh
cargo run --features gui --example gui -- path/to/project.xgwx
```

The GUI lists decoded ladder programs on the left and renders the selected
program as a scrollable, zoomable ladder canvas using parsed rungs, cells, and
wire segments.

## WebAssembly Demo

The parser can be compiled for browser use through the optional `wasm` feature.
The browser-facing API accepts bytes, so users can parse files selected from a
local file picker without uploading them anywhere.

```sh
cargo check --target wasm32-unknown-unknown --features wasm
wasm-pack build --target web --out-dir web/dist/pkg --out-name libxgwx --features wasm --no-default-features
cp web/index.html web/styles.css web/app.js web/dist/
```

The GitHub Pages workflow builds the same upload-only demo from `web/`. It does
not publish `.xgwx` fixtures into the Pages artifact. The demo shows project
summaries, a program sidebar, decoded variables, hardware modules, decoded
network summaries, and a parameter selector with the same section attributes
and decoded HSC, position, PID, safety, Cnet, and FEnet details as the TUI. It
also includes a best-effort SVG ladder viewer for decoded ladder programs.

## Fixtures

The repository includes small `.xgwx` files in `fixtures/` for the normal test
suite. They are included with the original author's permission for parser
development and testing; see `fixtures/README.md` for attribution and
permission details.

To run an additional smoke test against another real workspace, keep that file
outside the commit and pass it with an environment variable:

```sh
LIBXGWX_FIXTURE=path/to/project.xgwx cargo test parses_real_fixture_from_env -- --ignored
```

CPU and hardware writes are CPU-aware: XGK model changes check retained base
and slot limits; cross-family and compact-model conversions are rejected.
The XGK module catalog cannot edit compact hardware. The captured XBM-DR16S
profile protects built-in I/O identity while allowing comments, and is exposed
through `cpu_hardware_profile()` and WASM `hardware.cpuProfile`.
See [CPU hardware validation](docs/cpu-hardware-validation.md) for the verified
scope and native acceptance evidence.

Structural contact/coil editing, row insertion and vertical branch connection editing are available for verified LD layouts; see [LD editing and native acceptance](docs/ladder-editing.md) for the API, supported operations and limits.
