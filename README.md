# libxgwx

Rust parser for LS XG5000 `.xgwx` workspace files.

The parser handles the container layout observed in LS XG5000 workspace files:

- `XG` binary workspace header
- gzip-compressed UTF-8 XML project payload
- optional trailing binary metadata

Unknown binary sections are preserved so callers can inspect them later.

## Usage

```rust
use libxgwx::XgwxDocument;

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
- `XgwxDocument::variables()` decodes the compressed global symbol table into
  variable summaries with name, formatted address, memory area, address number,
  data type, description, source reference, and range.
- `XgwxDocument::ladder_programs()` partially decodes base64/bzip2 ladder
  `ProgramData`, preserving raw bytes and extracting embedded strings and likely
  ladder elements such as instruction calls, comparisons, timers, logic
  operators, device references, constants, comments, and internal references.
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

## GUI Ladder PoC

Run the graphical ladder viewer:

```sh
cargo run --features gui --example gui -- path/to/project.xgwx
```

The GUI lists decoded ladder programs on the left and renders the selected
program as a scrollable, zoomable ladder canvas using parsed rungs, cells, and
wire segments.

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
