# libxgwx

Rust parser for LS XG5000 `.xgwx` workspace files.

The parser handles the container layout observed in LS XG5000 workspace files:

- `XG` binary workspace header
- gzip-compressed UTF-8 XML project payload
- optional trailing binary metadata

Unknown binary sections are preserved so callers can inspect them later.

## Usage

Add the package with `cargo add libxgwx`; import it as `xgwx`. Optional features
are `write` for supported edits and project creation, `il` for LD-to-IL
conversion, `wasm` for browser bindings, and `gui` for the repository's GUI example.

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

Read the [generated API documentation](https://ne0ekspert.github.io/libxgwx/api/xgwx/index.html)
for the parser and optional `write`, `il`, and `wasm` APIs. See the
[user examples](examples/README.md) for runnable usage demonstrations.
Hardware catalog details are in the [XGK module guide](docs/xgk-module-catalog.md).

Generate and open the reference locally:

```sh
cargo doc --open --lib --no-deps --features write,il,wasm
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
`xg5000-acceptance` development tool generates independent writer cases and verifies
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

The crates.io package contains the library source, runtime defaults, README and
license. It excludes all fixtures, repository integration tests, examples,
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

`XgwxDocument::sfc_programs()` reads native SFC XML blocks and their positioned
entities independently of binary ladder bodies. It retains unknown property
attributes. The WASM summary exposes this model as `sfc`; the VS Code editor
renders steps, transitions, labels, jumps, and variable actions in a dedicated
SFC view.

With `write`, `edit_sfc_entity(&SfcEntityPatch)` supports step comments
and existing direct `%MX` BOOL transition conditions, with stale-value and
position checks. Transition annotation cells are synchronized.
`replace_sfc_sequence(&SfcSequencePatch)` creates and replaces captured linear
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

`create_program(&NewProgram)` appends a blank LD or SFC program using captured
native templates, with unique program/symbol GUIDs and a unique identifier name.
It binds to the existing scan task, updates the workspace node count, and
preserves existing program records. XGK supports LD creation; XGI supports LD,
and the captured SFC CPU models support SFC. Unsupported CPU/language combinations,
duplicate names/identities and ambiguous configurations are rejected atomically.
The WASM entry point is `create_xgwx_program(bytes, patch)`. Native Save As
coverage and expected blank-program diagnostics are documented in
[creation fixtures](fixtures/program-create/README.md).

`delete_program(program_index, expected_object_id)` removes a top-level program
and its local declarations, checks the selected identity, and updates the workspace
node count. Other program records, shared globals, hardware and scan tasks remain
intact. The WASM export is `delete_xgwx_program`; the extension uses its normal
document edit history so deletion supports Undo.

`move_program(from, to, expected_object_id, expected_target_id)` moves a program
into its final list index. It checks source and destination identities, preserves
complete program XML records and task assignments, and leaves workspace counts
unchanged. The WASM export is `move_xgwx_program`. The extension provides sidebar
dragging with insertion feedback and the normal Undo/Save lifecycle. XG5000 executes
programs in list order within their scan/task; task scheduling remains independent.

### Standalone ST and IEC IL source

`XgwxDocument::text_programs()` reads validated Structured Text (`Kind=4`) and IEC
Instruction List (`Kind=9`) programs from their native `Body/ST_Program/CodeList`
records. Both languages store bzip2-compressed UTF-16 source with a `CodeCount`
measured in UTF-16 code units. Blank programs use the captured XG5000 templates.

With `write`, `edit_text_program(&TextProgramPatch)` checks program identity,
language and expected source before replacing only CodeList. It accepts up to
65536 UTF-16 units, rejects NUL characters, and preserves program metadata,
declarations and neighboring programs. Unknown source layouts, encrypted
programs and bookmark/breakpoint metadata remain read only.
`edit_text_variable(object_id, &SfcVariablePatch)` uses the existing IEC
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

With `write,il`, `vendor_il_programs()` exposes classic XGK ladder programs as vendor IL. `edit_vendor_il(&VendorIlPatch)` checks identity and stale source and builds native ladder records using existing cell/catalog writers and CPU operand guards. It supports series LOAD/AND contacts, negated and edge variants, OUT/SET/RST outputs and fixed-arity catalog instructions. Branches, comments and Auto-allocation ladder text replacement remain guarded. IEC CodeList is never used to store XGK vendor IL. Read eligibility validates each instruction without rebuilding the full ladder; applying an edit builds and verifies a candidate atomically.

Generate CPU-family acceptance candidates with `cargo run --features write,il --manifest-path dev-tools/Cargo.toml --bin text-cpu-expansion-acceptance`. Native captures and semantic Save As comparisons are in `fixtures/text-cpus/`.
