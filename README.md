# libxgwx

Rust parser for LS XG5000 `.xgwx` workspace files.

The parser handles the container layout observed in LS XG5000 workspace files:

- `XG` binary workspace header
- gzip-compressed UTF-8 XML project payload
- optional trailing binary metadata

Unknown binary sections are preserved so callers can inspect them later.

## Known limitations

### Default-network names are not preserved by XG5000

In native tests with XG5000 **4.82.1.0**, changing the tested default network's
XML name from `기본 네트워크` to `AcceptanceNetwork` did not change its displayed
name. XG5000 restored `기본 네트워크` when saving the workspace. The library's
network editor can update that XML field, but a successful local edit or parser
round trip does not establish that XG5000 will preserve the new name.

This result applies to the tested default network with
`Type="NETWORK ITEM:UNKNOWN"`. It does not establish that all network types have
fixed names or that renaming through XG5000's own UI is supported. See the
[native acceptance report](https://github.com/ne0ekspert/libxgwx/blob/master/docs/xg5000-object-api-20261011.md)
for the observed behavior and test scope.

## Usage

Add the package with `cargo add libxgwx`; import it as `xgwx`. Optional features
are `write` for supported edits and project creation, `il` for LD-to-IL
conversion, `wasm` for browser bindings, `ffi` for the native C API, and `gui`
for the repository's GUI example.

```rust
use xgwx::XgwxDocument;

let doc = XgwxDocument::from_path("project.xgwx")?;
let project = doc.project().info();

println!("header label: {:?}", doc.container().header().label);
println!("project name: {:?}", project.name);
println!("file version: {:?}", project.file_version);

for module in doc.module().summaries() {
    println!(
        "module base={:?} slot={:?} id={:?} name={:?}",
        module.base,
        module.slot,
        module.id,
        module.name
    );
}

for program in doc.program().programs() {
    println!("program: {:?}", program.name);
}

# Ok::<(), Box<dyn std::error::Error>>(())
```

## API

Read the [generated API documentation](https://ne0ekspert.github.io/libxgwx/api/xgwx/index.html)
for the parser and optional `write`, `il`, and `wasm` APIs. See the
[user examples](examples/README.md) for runnable usage demonstrations.
Hardware catalog details are in the [XGK module guide](docs/xgk-module-catalog.md).
For C and other native-language bindings, see the [C API guide](docs/c-api.md)
and the public header in `include/xgwx.h`.

Generate and open the reference locally:

```sh
cargo doc --open --lib --no-deps --features write,il,wasm,ffi
```

## Container inspection and XML replacement

Document state is private. Borrow the XML, tree, and preserved binary data
through `doc.container()`:

```rust,no_run
use xgwx::XgwxDocument;
let doc = XgwxDocument::from_path("project.xgwx")?;
let container = doc.container();
println!("XML bytes: {}", container.xml().len());
println!("Original gzip bytes: {}", container.main_gzip().len());
println!("Header: {:?}", container.header());
# Ok::<(), xgwx::XgwxError>(())
```

With `write`, `doc.container_mut().replace_xml(xml)?` parses replacement XML
before updating the document and its tree together. Invalid XML leaves the
document unchanged. This low-level operation checks syntax; use domain editors
for supported PLC edits and their native format guards. Header, gzip, and
trailer accessors describe the original parsed container until emitted bytes
are reparsed. `to_bytes()` preserves an unchanged input byte for byte.

This is a breaking change: replace `doc.xml` with
`doc.container().xml()`, and use the corresponding container accessors for
`root`, `header`, `main_gzip`, `trailer`, and `trailer_gzip_members`.

## Root API migration

Project, configuration, parameter, service, hardware and network readers now
belong to their object namespaces. The former root readers are private.

| Former call | Current call |
| --- | --- |
| `doc.project_info()` | `doc.project().info()` |
| `doc.project_name()` | `doc.project().name()` |
| `doc.project_options()` | `doc.project().options()` |
| `doc.configurations()` | `doc.configuration().list()` |
| `doc.module_summaries()` | `doc.module().summaries()` |
| `doc.base_summaries()` | `doc.base().summaries()` |
| `doc.networks()` | `doc.network().list()` |
| `doc.network_modules()` | `doc.network().module_summaries()` |
| `doc.xgpd_config_infos()` | `doc.network().config_infos()` |
| `doc.parameters()` | `doc.parameter().summaries()` |
| `doc.hsc_parameters()` | `doc.parameter().hsc()` |
| `doc.position_parameters()` | `doc.parameter().position()` |
| `doc.pid_cal_parameters()` | `doc.parameter().pid().calculation()` |
| `doc.pid_tune_parameters()` | `doc.parameter().pid().tuning()` |
| `doc.high_speed_links()` | `doc.high_speed_link().summaries()` |
| `doc.high_speed_link_blocks()` | `doc.high_speed_link().blocks()` followed by each handle's `summary()` |
| `doc.safety_comm()` | `doc.safety().first().map(|record| record.summary())` |
| `doc.trend_monitoring()` | `doc.trend().first().map(|record| record.summary())` |
| `doc.select_cpu(model)` | `doc.configuration_mut().get(0)?.select_cpu(model)` |
| `doc.rename_project(expected, name)` | `doc.project_mut().rename_if(expected, name)` |

The configuration index is document order. CPU migration remains guarded to a
single configuration; selecting an index does not enable multi-PLC CPU writes.
Hardware profiles are inspected through a selected configuration's
`hardware_profile()`. `doc.modules()` remains the unified typed module list.

Root hardware and network mutation bridges are also private:

| Former call | Current call |
| --- | --- |
| `doc.set_base_slot_count(base, count)` | `doc.base_mut().by_number(base)?.set_slot_count(count)` |
| `doc.insert_module(base, slot, model)` | `doc.base_mut().by_number(base)?.insert_module(slot, model)` |
| `doc.delete_module(base, slot)` | `doc.module_mut().at(base, slot)?.delete()` |
| `doc.select_module(base, slot, model)` | `doc.module_mut().at(base, slot)?.select_model(model)` |
| `doc.update_module(base, slot, patch)` | `doc.module_mut().at(base, slot)?.update(patch)` |
| `doc.set_module_option(base, slot, key, index, value)` | `doc.module_mut().at(base, slot)?.set_option(key, index, value)` |
| `doc.set_module_input_filter(base, slot, filter)` | `doc.module_mut().at(base, slot)?.set_input_filter(filter)` |
| `doc.module_option_values(base, slot)` | `doc.module().at(base, slot)?.option_values()` |
| `doc.update_network(index, patch)` | `doc.network_mut().get(index)?.update(patch)` |
| `doc.update_network_module(base, slot, patch)` | `doc.network_mut().module_at(base, slot)?.update(patch)` |

Browser field patches translate to `ModuleEditor::edit_field`,
`BaseEditor::edit_slot_count`, or the selected network editor's `edit_field`.
FEnet edits use `doc.network_mut().module_at(base, slot)?.fenet()?.edit_field(...)`;
Cnet batches use the same membership's `cnet()?.edit_settings(&changes)`.
Field edits retain expected-value guards. Selectors reject ambiguous coordinates
and unsupported associations before editing; C/WASM and MCP wire arguments retain
their existing format.

## Program and Variable Namespaces

Program and variable methods are grouped behind borrowed namespaces. The old
flat methods are no longer public. Shared views require no optional features;
exclusive editors require `write`.

```rust,no_run
use xgwx::XgwxDocument;
let doc = XgwxDocument::from_path("project.xgwx")?;
let programs = doc.program().list();
let languages = doc.program().languages();
let text = doc.program().text_programs();
let globals = doc.variable().globals()?;
let locals = doc.variable().locals();
# Ok::<(), Box<dyn std::error::Error>>(())
```

| Namespace | Contents |
| --- | --- |
| `program()` | Program/task summaries, languages, LD bodies, SFC charts, ST/IL source |
| `variable()` | Global symbols, IEC locals, source/SFC declarations, I/O generation preview |
| `program_mut()` | Select program handles, create programs, read collection metadata |
| `variable_mut()` | Global symbol edits and I/O variable generation |

Global variable updates use `doc.variable_mut().get(index)?.update(&patch)`.
The former indexed collection mutators have been removed. Program body and local
declaration edits use the selected handles described below. Read methods are also
available on editors. An editor borrows the document exclusively: collect the
expected source/identity/declaration values first, then open the editor.
The views do not clone the document and use the existing decoding and guarded
writer implementations.

Migration examples: `doc.programs()` becomes `doc.program().list()`;
`doc.ladder_program(index)` becomes `doc.program().ladder_program(index)`;
`doc.variables()` becomes `doc.variable().globals()`;
SFC declaration edits use `program.variable_mut().edit_declaration(&patch)`.
Container, hardware, and network
methods remain on the document. WASM exports and native C entry points retain
their existing names and data shapes.

Run `cargo run --example namespaces -- project.xgwx` for a complete read example.

### Selected programs

`ProgramSummary` remains detached metadata. Select a document-bound handle to
inspect or edit a particular program:

```rust,no_run
use xgwx::{create_project, TextSourcePatch};
let mut doc = create_project("XGI-CPUE", "ST")?;
let mut program = doc.program_mut().get(0)?;
program.rename("Controller")?;
let mut text = program.text()?;
text.update(&TextSourcePatch {
    expected_source: text.source()?.unwrap(),
    source: "(* Controller source *)\r\n".into(),
})?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The selected `Program` supplies its index and identity to the existing writers.
Use `update(&ProgramPatch)` for metadata, `move_to(index)` to reorder it, and
`delete()` to remove it. Deletion consumes the handle; `move_to()` updates the
handle's index so subsequent edits still target the same program. Creation stays
on the collection as `doc.program_mut().create(&NewProgram)`.
Use `doc.program_mut().get_if(index, expected_object_id)?` when applying a
previously captured selection. `program.move_to_if(to, expected_target_id)`
also checks the captured destination identity before reordering.

| Selected editor | Operations |
| --- | --- |
| `program.iec_ld()?` | All IEC ladder edits, with `iec_ld_` removed from method names |
| `program.ladder()?` | Native XGK ladder edits, with `ladder_` removed from method names |
| `program.sfc()?` | `edit_entity(&SfcEntityEdit)`, `replace_sequence(&SfcSequenceEdit)` |
| `program.text()?` | Standalone ST/IL `update(&TextSourcePatch)` and source inspection |
| `program.vendor_il()?` | XGK IL projection `update(&VendorIlSourcePatch)`; requires `il` |
| `program.variable_mut()` | Local symbol edits and `edit_declaration(&ProgramVariablePatch)` |

```rust,no_run
use xgwx::create_project;
let mut doc = create_project("XGI-CPUE", "LD")?;
let mut program = doc.program_mut().get(0)?;
let mut ladder = program.iec_ld()?;
ladder.insert_single_element(0, 1, "contact", "NO", "%MX0")?;
ladder.insert_single_element(0, 94, "coil", "OUTPUT", "%MX1")?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Each language editor checks the selected program's body type before use. Its
methods omit the bound program index; source/chart/declaration patches also omit
it. Cross-program copies retain the other program's index. In
`replace_group_from_program()`, the selected program is the destination.
Expected values and layout guards remain unchanged. Shared selection is available
without `write`, for example `doc.program().get(index)?.text()?`.

Indexed collection dispatch methods remain available but are hidden from Rustdoc
for migration of wire adapters. Native C and WASM patch schemas retain their
indices and export names. Global symbols remain under `doc.variable_mut()`.

Run `cargo run --example program_handles --features write` for a complete example
that creates programs, edits ST and local declarations, deletes a program, and
builds an IEC ladder rung. Pass an output directory to save both generated files.

## Module Views

### Bases and their modules

`bases()` returns borrowed `Base` handles. Each handle exposes the modules
belonging to that base, including their I/O classification and network settings:

```rust,no_run
let doc = xgwx::XgwxDocument::from_path("project.xgwx")?;
for base in doc.bases() {
    println!("Base {:?}: {:?} slots", base.number(), base.slot_count());
    for module in base.modules() {
        println!("{module:#?}");
    }
}
let base = doc.base().get(0)?; // document index, not the native base number
let modules = base.modules();
# Ok::<(), Box<dyn std::error::Error>>(())
```

`base.configuration_index()` identifies the owning PLC in configuration order.
Repeated base numbers stay separate. XML ancestry determines ownership first;
otherwise the configuration and base number must resolve to one base record.
Ambiguous or unplaced records remain visible through `doc.modules()`. Empty
bases remain in the list, and module-list length is not a physical slot count.

Use `base.summary()` for a detached native record. The former raw `doc.bases()`
getter is now `doc.base().summaries()`; native C/WASM and MCP base listings keep
their existing fields, indices, and ordering. The views require no optional features.

### Network configuration records

Protocol configuration lists are under the network namespace:

```rust,no_run
let doc = xgwx::XgwxDocument::from_path("project.xgwx")?;
let cnet = doc.network().cnet_config_infos();
let fenet = doc.network().fenet_config_infos();
for module in doc.network().modules() {
    if let Some(settings) = module.fenet_config_info() {
        println!("{:?}", settings.ip_address);
    }
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

The old `doc.cnet_config_infos()` and `doc.fenet_config_infos()` methods are no
longer public. The namespace retains every settings record, including unmatched
ones, in document order. On a `NetworkModule`, `cnet_config_info()` and
`fenet_config_info()` borrow only uniquely resolved settings. Missing, ambiguous,
or other protocol configurations return `None`; `module.configuration` retains
the association details. `doc.network().list()` lists raw network summaries.

The crate version remains `0.1.0` while these APIs are unpublished.

The default-feature API provides `HardwareModule` for common installed hardware
metadata, `IOModule` for confirmed numeric I/O identities, and `NetworkModule`
for network memberships with their protocol settings. `modules()` returns the unified `Vec<Module>` view; `doc.module().summaries()`
returns raw installed hardware records. `doc.network().module_summaries()` still returns
network membership summaries.

```rust,no_run
use xgwx::{ConfigurationMatch, Module, XgwxDocument};

let doc = XgwxDocument::from_path("project.xgwx")?;
for module in doc.modules() {
    match module {
        Module::Io(io) => println!("{:?}: {:?}", io.kind, io.base.hardware.name),
        Module::Network(network) => {
            println!("Network: {:?}", network.metadata.name);
            match network.configuration {
                ConfigurationMatch::Matched(settings) => println!("{settings:#?}"),
                ConfigurationMatch::Missing => println!("No matching configuration"),
                ConfigurationMatch::Ambiguous(candidates) => println!("Ambiguous: {candidates:#?}"),
            }
        }
        Module::Other(hardware) => println!("Other: {:?}", hardware.hardware.name),
    }
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

Use `hardware_modules()`, `io_modules()`, or `resolved_network_modules()` for
individual views. Hardware carries its owning configuration index. Network
settings are associated within their XML owner or the configuration named by
`ConfigName`, using current numeric identity, base, slot and available subtype.
Missing and ambiguous associations remain explicit, including built-in interfaces
with no separate installed hardware record. Location values are file-local,
not persistent module identifiers. The I/O classification covers native catalog
identities; unknown IDs/subtypes remain `Module::Other`, regardless of their names.
Unknown protocol layouts remain accessible through the original summaries/XML.
These owned views do not mutate the document.

Run `cargo run --example modules -- project.xgwx` for complete settings output.

### Breaking API migration

`XgwxDocument::modules()` now returns `Vec<Module>` instead of
`Vec<ModuleSummary>`. Match `Module::Io`, `Module::Network`, and `Module::Other`
when enumerating all modules. Code needing the previous installed hardware list
must call `doc.module().summaries()` instead. The earlier `typed_modules()` getter has
been replaced by `modules()`.

WASM `hardware.modules`, native C JSON module lists, and hardware writer targets
continue to use installed hardware summaries; their counts and coordinates do
not include additional network memberships.

## LD to IL Conversion

Enable the optional `il` feature to convert decoded ladder programs. Literal
device addresses are retained by default:

```rust,no_run
use xgwx::XgwxDocument;

let doc = XgwxDocument::from_path("fixtures/elements.xgwx")?;
for program in doc.program().ladder_programs() {
    let il = program?.to_il()?;
    println!("{il}");
}

# Ok::<(), Box<dyn std::error::Error>>(())
```

Run that code with `--features il`. Each `IlStep` is either a rung comment or
an instruction, and `IlProgram` renders the steps separated by newlines;
embedded comment newlines are escaped. For symbolic operands, decode
`doc.variable().variables()` and pass them to
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
`xg5000-acceptance` development tool generates independent writer cases and verifies
their edited sections after an external save.

Enable the opt-in `write` feature to prepare attribute changes on an existing
module. The module must be uniquely identified by its current base and slot:

```rust,no_run
use xgwx::{ModuleInputFilter, ModulePatch, XgwxDocument};

let mut doc = XgwxDocument::from_path("project.xgwx")?;
let mut module = doc.module_mut().at(0, 2)?;
module.update(
    &ModulePatch {
        comment: Some("Updated from an external editor".to_owned()),
        ..ModulePatch::default()
    },
)?;
module.set_input_filter(ModuleInputFilter::Ms5)?;

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
`update_xgwx_iec_ld_comment(bytes, programIndex, offset, expected, replacement)`,
`update_xgwx_iec_ld_rising_contact_operand(bytes, programIndex, offset, expected, replacement)`,
`update_xgwx_iec_ld_element_operand(bytes, programIndex, offset, expected, replacement)`,
`update_xgwx_iec_ld_contact_kind(bytes, programIndex, offset, expectedKind, replacementKind)`,
`update_xgwx_iec_ld_coil_kind(bytes, programIndex, offset, expectedKind, replacementKind)`,
`insert_xgwx_iec_ld_blank_row(bytes, programIndex, afterRowIndex)`,
`delete_xgwx_iec_ld_blank_row(bytes, programIndex, blankRowIndex)`,
`insert_xgwx_iec_ld_rung(bytes, programIndex, row, contactKind, contact, coilKind, coil)`,
`delete_xgwx_iec_ld_rung(bytes, programIndex, row, expectedContactKind, expectedContact, expectedCoilKind, expectedCoil)`,
`delete_xgwx_iec_ld_terminal_function(bytes, programIndex, blockOffset, expectedName)`,
`delete_xgwx_iec_ld_standalone_function(bytes, programIndex, blockOffset, expectedName)`,
`insert_xgwx_iec_ld_standalone_function(bytes, programIndex, insertionOffset, functionName, inputOperand, outputOperand)`,
`delete_xgwx_iec_ld_function_cell(bytes, programIndex, blockOffset, expectedName)`,
`delete_xgwx_iec_ld_connected_arithmetic(bytes, programIndex, blockOffset, expectedName)`,
`insert_xgwx_iec_ld_function_cell(bytes, programIndex, insertionOffset, functionName, instanceName)`,
`update_xgwx_iec_ld_function_operand(bytes, programIndex, offset, expected, replacement)`,
`update_xgwx_iec_ld_arithmetic_function(bytes, programIndex, offset, expected, replacement)`,
`update_xgwx_iec_ld_comparison_function(bytes, programIndex, offset, expected, replacement)`,
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

This prints the CPU model and program payload versions alongside project counts
and decoded Cnet, FEnet, HSC, position, and PID summaries. It is useful for
comparing parser output across fixture files without opening the TUI.

## Benchmark Example

Run the fixture-backed benchmark in release mode:

```sh
cargo run --release --manifest-path dev-tools/Cargo.toml --bin bench -- --warmup 50 --iterations 500 fixtures
```

The benchmark reads `.xgwx` fixture bytes once, then measures parser-only work
and parser plus higher-level decoding paths such as variables, ladder programs,
parameters, network summaries, and decoded payloads. The `ladder avg` column
isolates ladder decoding using an already parsed document. Native wire geometry
is parsed once per decode and shared by both wire orientations; unknown layouts
retain the legacy read-only decoding path.

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

The crates.io package contains the library source, runtime defaults, C header,
README and license. It excludes all fixtures, repository integration tests, examples,
development tools, scripts, and browser assets. Run the full test suite and
acceptance tools from a repository checkout, where the fixtures are available.

To run an additional smoke test against another real workspace, keep that file
outside the commit and pass it with an environment variable:

```sh
LIBXGWX_FIXTURE=path/to/project.xgwx cargo test parses_real_fixture_from_env -- --ignored
```

CPU and hardware writes are CPU-aware: XGK model changes check retained base
and slot limits; cross-family and compact-model conversions are rejected.
The CPU catalog also recognizes XGI models, including
`XGI-CPUE` (configuration type `106`). An XGI configuration can select its
existing model without changing the file. XGI-CPUE, CPUS, CPUH, CPUU, CPUU/D and CPUUN can switch between models
for validated SFC projects with captured default parameters and empty I/O tables.
Changes update model flags and default M memory ranges while preserving source.
CPUUN additionally creates/removes its default local Ethernet and empty motion
sections. CPUS/P, custom parameters and configured hardware require migration.
XGI supports LD, SFC, and ST programs; the `ProgramData` version identifies
the language of an individual program. See [XGI CPU support](docs/xgi-cpu.md).
The XGK module catalog cannot edit compact hardware. The captured XBM-DR16S
profile protects built-in I/O identity while allowing comments, and is exposed
through `cpu_hardware_profile()` and WASM `hardware.cpuProfile`.
See [CPU hardware validation](docs/cpu-hardware-validation.md) for the verified
scope and native acceptance evidence.

Structural contact/coil editing, row insertion and vertical branch connection editing are available for verified LD layouts; see [LD editing and native acceptance](docs/ladder-editing.md) for the API, supported operations and limits.

IEC scalar BOOL outputs can drive a horizontal ladder wire: select ENO or a
comparison OUT and use F5, Enter, or double-click. F5 on the following blank
cell extends the wire. Arithmetic and conversion OUT fields keep their numeric
destination. Comparisons accept two sources and an optional BOOL destination;
wiring OUT replaces that assignment. Unsupported block layouts remain guarded.

### SFC charts

`doc.program().sfc_programs()` reads native SFC XML blocks and their positioned
entities independently of binary ladder bodies. It retains unknown property
attributes. The WASM summary exposes this model as `sfc`; the VS Code editor
renders steps, transitions, labels, jumps, and variable actions in a dedicated
SFC view.

With `write`, `program.sfc()?.edit_entity(&SfcEntityEdit)` supports step comments
and existing direct `%MX` BOOL transition conditions, with stale-value and
position checks. Transition annotation cells are synchronized.
`program.sfc()?.replace_sequence(&SfcSequenceEdit)` creates and replaces captured linear
main charts using typed `SfcRow` values and a complete expected-entity snapshot.
It supports steps, direct `%MX` transitions, labels/jumps, and direct BOOL
actions with N, R, S, L, D, P, SD, DS, or SL qualifiers. Timed
actions use bounded TIME literals such as `T#2s` or `T#500ms`. It regenerates annotations/placeholders and invalidates
compiled caches while preserving identities and local symbol payloads.
The summary's `editableRows` is `null` for unsupported layouts. Unsupported
branch topologies, nested blocks, unknown qualifiers, non-ST program references,
bookmarks/breakpoints, and unknown data remain guarded. Incomplete sequences can be saved and require
completion before XG5000 program checks pass.
Native XG5000 checks and Save As comparisons are documented in
[`fixtures/sfc/README.md`](fixtures/sfc/README.md).

Balanced SFC branches support alternative and simultaneous split/join pairs
with aligned joins, and positioned step/transition editing while
retaining ST sources, action qualifiers and local declarations. The extension
provides branch creation, adding/removing paths, extending every path by a
step/transition pair, and collapsing a branch back to its first path. Nested or
crossing branches, path gaps without connector rows, custom branch priorities, and unknown
native records remain guarded. See [SFC fixtures](fixtures/sfc/README.md).

SFC steps can contain multiple independent variable or ST actions. Additional
slots use captured native Type=7 continuation rows, with adjacent Type=2 actions.
Balanced branch paths receive empty continuation padding when another path needs
an action slot. Qualifiers, timers, and ST sources remain attached to each action;
shared ST names retain one shared source. Continuations cannot introduce step
names, comments, initial status, or actions after a transition. Native bounds are
512 ordinary steps and 65,535 rows/columns per program; branches have no separate
count cap (see the [LS manual, chapter 16](https://sol.ls-electric.com/uploads/document/16411767742780/XG5000_Manual_V2.5_202012_EN.pdf)).
The editor also limits dense grids to 1,048,576 cells as a resource
guard. Mixed linear and simultaneous-branch
stacks have native strict Check Program and Save As retention coverage in
`fixtures/sfc/multi-action-*`.

SFC declarations also support fixed 32-byte STRING values, Retain, literal
initial values, and up to three zero-based array dimensions. Array values use
counted per-member initializer maps; a comma-separated prefix or repeat literal
such as `4(2)` initializes elements in native index order. Unspecified elements
use their defaults. Primitive BOOL/integer/real/TIME literals and quoted ASCII
strings are validated locally. Existing declarations can change description,
initial value and Retain; changing type or bounds is restricted to unreferenced
declarations. System variables, mapped declarations, custom structures, arrays
of STRING/FB instances, sparse initializers and uncaptured member overrides
remain guarded. Stale declarations and failed validation leave the file intact.

Independent SFC paths may contain different numbers of alternating steps and
transitions. Empty Type=7 continuation rows align their join; action continuations
retain their original step ownership. Each path still requires an odd nonzero
count of alternating nodes and the captured split/join entry/exit types. The
extension can extend one selected path, remove one pair from it, or extend all
paths together.

### Creating blank workspaces

With the `write` feature, `create_project(cpu_model, language)` generates a new
`XgwxDocument` without reading a template workspace:

```rust,ignore
let doc = xgwx::create_project("XGI-CPUE", "ST")?;
let bytes = doc.to_verified_bytes()?;
```

The WASM equivalent is `create_xgwx_project(cpuModel, language)`. LD/SFC/ST/IL
choices use guarded CPU selection and the existing program writer. Native
parameter and security defaults remain library-owned captured records;
container compression, alignment, size and checksum are generated. See
[src/project_defaults](src/project_defaults/README.md) for provenance and limits.

### Creating scan programs

`doc.program_mut().create(&NewProgram)` appends a blank LD or SFC program using captured
native templates, with unique program/symbol GUIDs and a unique identifier name.
It binds to the existing scan task, updates the workspace node count, and
preserves existing program records. XGK supports LD creation; XGI supports LD,
and the captured SFC CPU models support SFC. Unsupported CPU/language combinations,
duplicate names/identities and ambiguous configurations are rejected atomically.
The WASM entry point is `create_xgwx_program(bytes, patch)`. Native Save As
coverage and expected blank-program diagnostics are documented in
[creation fixtures](fixtures/program-create/README.md).

`doc.program_mut().get_if(index, expected_object_id)?.delete()` removes a top-level program
and its local declarations, checks the selected identity, and updates the workspace
node count. Other program records, shared globals, hardware and scan tasks remain
intact. The WASM export is `delete_xgwx_program`; the extension uses its normal
document edit history so deletion supports Undo.

`program.move_to_if(to, expected_target_id)` moves a selected program
into its final list index. It checks source and destination identities, preserves
complete program XML records and task assignments, and leaves workspace counts
unchanged. The WASM export is `move_xgwx_program`. The extension provides sidebar
dragging with insertion feedback and the normal Undo/Save lifecycle. XG5000 executes
programs in list order within their scan/task; task scheduling remains independent.

### Standalone ST and IEC IL source

`doc.program().text_programs()` reads validated Structured Text (`Kind=4`) and IEC
Instruction List (`Kind=9`) programs from their native `Body/ST_Program/CodeList`
records. Both languages store bzip2-compressed UTF-16 source with a `CodeCount`
measured in UTF-16 code units. Blank programs use the captured XG5000 templates.

With `write`, `program.text()?.update(&TextSourcePatch)` keeps the selected
identity and language bound and checks expected source before replacing only CodeList. It accepts up to
65536 UTF-16 units, rejects NUL characters, and preserves program metadata,
declarations and neighboring programs. Unknown source layouts, encrypted
programs and bookmark/breakpoint metadata remain read only.
`program.variable_mut().edit_declaration(&ProgramVariablePatch)` uses the existing IEC
PB50 declaration encoder, including arrays, STRING, initial values, Retain and
captured function blocks. Referenced declarations cannot be removed or have
their type/bounds changed. The WASM exports are `edit_xgwx_text_program` and
`edit_xgwx_text_variable`; browser summaries include `textPrograms` with source,
language, editability and declaration metadata.

Generate the native acceptance project with:

```sh
cargo run --features write --manifest-path dev-tools/Cargo.toml --bin text-program-acceptance
```

The paired native capture and generated fixtures are in `fixtures/text-programs`.

`select_cpu()` also supports ST/IEC IL and mixed SFC/ST/IL workspaces when they
use captured default parameters, one configuration and empty I/O tables. It
preserves source, language, identities and declarations while migrating default
memory ranges and CPUUN Ethernet/motion sections. All pairs among XGI-CPUU,
CPUH, CPUS, CPUE, CPUU/D and CPUUN are tested. All six generated text conversions
passed native XG5000 strict syntax/type checks and Save As with zero errors and
warnings; native saves can be converted back to CPUE. See the fixture README
for the XGK-CPUSN mnemonic audit and the distinction from XGI IEC IL.

### Additional text-program CPU families

`program_languages()` supplies the CPU/project-mode capabilities used by both the writer and VS Code. Native blank captures enable ST/IEC IL on XEC-E/H/S/U, XEM-H2/HP, GIPAM, KL and XGR-CPUH. XGK Auto-allocation enables ST and native LD creation. XGK scalar declarations use a separately captured type-ID mapping; native D allocations are normalized only in the reading copy. Advanced XGK declaration shapes remain guarded. XGI-CPUS/P text creation stays disabled until a native profile is captured.

With `write,il`, `doc.program().vendor_il_programs()` exposes classic XGK ladder programs as vendor IL. `program.vendor_il()?.update(&VendorIlSourcePatch)` checks identity and stale source and builds native ladder records using existing cell/catalog writers and CPU operand guards. It supports series LOAD/AND contacts, negated and edge variants, OUT/SET/RST outputs and fixed-arity catalog instructions. Branches, comments and Auto-allocation ladder text replacement remain guarded. IEC CodeList is never used to store XGK vendor IL. Read eligibility validates each instruction without rebuilding the full ladder; applying an edit builds and verifies a candidate atomically.

Generate CPU-family acceptance candidates with `cargo run --features write,il --manifest-path dev-tools/Cargo.toml --bin text-cpu-expansion-acceptance`. Native captures and semantic Save As comparisons are in `fixtures/text-cpus/`.

### Selected hardware editors

Hardware writes can bind the document once and omit repeated base/slot arguments:

```rust,no_run
let mut base = doc.base_mut().get(0)?; // base record index, not native number
base.set_slot_count(4)?;
base.insert_module(0, "XGI-D24A/B")?;
let mut module = base.module(0)?; // starting physical slot; consumes the base editor
module.set_input_filter(xgwx::ModuleInputFilter::Ms5)?;
module.update(&xgwx::ModulePatch {
    comment: Some("Inputs".into()),
    ..Default::default()
})?;
module.delete()?; // consumes the module editor
```

Use `doc.module().get(index)` for a borrowed installed-record view and
`doc.module_mut().get(index)` for its editor. These indices exclude additional
network membership records. `base.installed_modules()` lists each installed
record once; `base.modules()` retains the full typed membership inventory.
Native coordinate selection via `module_mut().at(base, slot)` and
`base_mut().by_number(base)` rejects ambiguous targets. Edits retain the existing
CPU, catalog, slot-span, stale-field, and transaction guards. Configuration-scoped
selection does not enable previously unsupported multi-PLC writes.

Run `cargo run --example hardware_handles --features write` for a complete
create/edit/delete/verified-serialization example.

Selected networks and memberships support bound edits:

```rust,no_run
# fn example(doc: &mut xgwx::XgwxDocument) -> Result<(), xgwx::XgwxError> {
let mut network = doc.network_mut().get(0)?;
network.update(&xgwx::NetworkPatch {
    name: Some("Factory".into()),
    ..Default::default()
})?;
let mut module = network.module(0)?; // Membership position within this network.
module.update(&xgwx::NetworkModulePatch {
    alias: Some("Uplink".into()),
    ..Default::default()
})?;
module.fenet()?.edit_field(&xgwx::NetworkFieldEdit {
    field: "ipAddress".into(),
    expected_value: "192.168.0.100".into(),
    replacement: "10.20.30.40".into(),
})?;
# Ok(())
# }
```

Read handles are available through `doc.network().get(index)` and
`network.modules()`. Their membership indices differ from installed hardware
indices. `doc.network_mut().module_at(base, slot)` selects a globally unique
membership. Protocol selection with `.fenet()` or `.cnet()` rejects missing,
ambiguous, or mismatched configuration associations. Cnet's
`edit_settings(&[CnetFieldEdit])` applies a batch atomically with captured-value
checks. See `examples/network_handles.rs` for a complete runnable example.

Project and configuration APIs bind inspection and editing to their objects:

```rust,no_run
# fn example(doc: &mut xgwx::XgwxDocument) -> Result<(), xgwx::XgwxError> {
let project = doc.project();
println!("Name: {:?}, options: {:?}", project.name(), project.options());
let configuration = doc.configuration().get(0)?;
println!("CPU: {:?}, bases: {}", configuration.cpu(), configuration.bases().len());
doc.project_mut().rename("Factory")?;
doc.configuration_mut().get(0)?.select_cpu("XGK-CPUUN")?;
# Ok(())
# }
```

`ProjectEditor::rename_if(expected_name, replacement)` retains a previously
captured name check for editor transactions. Project options and other metadata
remain read-only. Configuration selection uses document indices, so repeated
PLC names remain distinct. CPU selection still requires one configuration and
uses the existing native migration and hardware-limit guards. See
`examples/project_configuration_handles.rs` for a runnable example.

Global variables also have selected record handles:

```rust,no_run
# fn example(doc: &mut xgwx::XgwxDocument) -> Result<(), xgwx::XgwxError> {
let snapshot = doc.variable().get(0)?.summary().clone();
let mut variable = doc.variable_mut().get_if(0, &snapshot)?;
variable.rename("StartInput")?;
variable.update(&xgwx::VariablePatch {
    description: Some("Factory start button".into()),
    ..Default::default()
})?;
# Ok(())
# }
```

`doc.variable().list()` returns selected read handles; `get(index)` uses native
global-record indices. `named(name)` selects a unique name without case distinctions.
`get_if(index, snapshot)` rejects stale decoded values, including records changed
by I/O generation. Updates preserve other global records and retain native text
length restrictions. Multiple symbol tables are rejected rather than guessing
which PLC owns a record. Global insertion and deletion remain unsupported.

Parameters are read through document-bound sections or a selected PLC:

```rust,no_run
# fn example(doc: &xgwx::XgwxDocument) -> Result<(), xgwx::XgwxError> {
let parameters = doc.configuration().get(0)?.parameters();
let counters = parameters.hsc();
let axes = parameters.position();
let calculation = parameters.pid().calculation();
let tuning = parameters.pid().tuning();
for record in parameters.list() {
    println!("{}: {:?}", record.index(), record.decode()?);
}
# Ok(())
# }
```

`doc.parameter()` includes all sections, including those outside a configuration.
Scoped `get(index)` retains document-wide parameter indices and rejects records
owned by another PLC. `of_kind(ParameterKind)` filters selected handles;
`decode()` returns typed HSC, positioning, PID calculation/tuning data or preserved
metadata for other native types. Invalid HSC payloads remain per-record errors.
These parameter families are read-only; their writer layouts are not validated.

High-speed links, safety communication, and trend monitoring have read-only
collections, both document-wide and scoped to a selected PLC:

```rust,no_run
# fn example(doc: &xgwx::XgwxDocument) -> Result<(), xgwx::XgwxError> {
let plc = doc.configuration().get(0)?;
for link in plc.high_speed_link().list() {
    for block in link.blocks() { println!("{:?}", block.summary()); }
}
let safety = plc.safety().list();
let trends = plc.trend().list();
# Ok(())
# }
```

Document-wide access uses `doc.high_speed_link()`, `doc.safety()`, and `doc.trend()`.
Each collection supports `list()`, `get(document_index)`, and detached `summaries()`.
High-speed blocks retain their nearest link ancestor, configuration, and independent
block indices; native block selectors can repeat. Unowned blocks remain in the
collection's `blocks()` list. Safety and trend lists retain all records, while
`first()` preserves the old optional-first reader behavior. Trend `sections()`
retains unknown and repeated children. No new native writer layouts are assumed.
