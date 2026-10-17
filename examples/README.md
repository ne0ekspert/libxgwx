# User examples

Run these commands from a repository checkout. Supply your own workspace file;
fixtures and examples are excluded from the crates.io package.

| Example | Purpose | Command |
| --- | --- | --- |
| `inspect` | Print project, program and hardware summaries | `cargo run --example inspect -- project.xgwx` |
| `modules` | List bases and their typed modules, plus all network protocol settings | `cargo run --example modules -- project.xgwx` |
| `namespaces` | Inspect program and variable namespaces | `cargo run --example namespaces -- project.xgwx` |
| `tasks` | Create task-driven XGK/XGI programs and all supported trigger conditions | `cargo run --features write --example tasks -- new-output-directory` |
| `program_handles` | Create and edit selected ST/IEC programs and local declarations | `cargo run --features write --example program_handles -- output-directory` |
| `il` | Print IL converted from supported ladder programs | `cargo run --features il --example il -- project.xgwx` |
| `write-module` | Change a supported input module's filter | `cargo run --features write --example write-module -- input.xgwx output.xgwx 0 2 5` |
| `tui` | Browse a workspace in an interactive terminal | `cargo run --example tui -- project.xgwx` |
| `gui` | View decoded ladder programs on a graphical desktop | `cargo run --features gui --example gui -- project.xgwx` |

`write-module` writes to its output path. `program_handles` optionally saves two
newly generated projects when given an output directory. Output files may be
overwritten; choose separate output paths when preserving existing workspaces.

The [C inspector](c/inspect.c) demonstrates native JSON inspection and optional
guarded renaming into a new file. Build commands and ownership rules are in
the [C API guide](../docs/c-api.md).

The [Python ctypes example](python/lists.py) loads module/network/program/global
lists and optionally applies an atomic JSON edit batch through the same C ABI.

Acceptance generators, diagnostics, probes and benchmarks live in
[`dev-tools`](../dev-tools/README.md).

`hardware_handles` (`--features write`) demonstrates selected base/module edits,
input filters, stale-field patches, consuming deletion, and verified serialization.

`cargo run --example network_handles --features write` lists selected networks and
memberships, updates their metadata, and edits a uniquely associated FEnet IP
address. The example verifies serialization by reparsing the output.

`cargo run --example project_configuration_handles --features write` inspects
project metadata, option entries and scoped configuration bases, then renames the
project, selects a supported CPU, and verifies the serialized result.

`cargo run --example global_variable_handles --features write` generates supported
I/O globals, lists selected records, checks a captured snapshot, updates a name and
description, and reparses the serialized result.

`cargo run --example parameter_handles -- project.xgwx` lists selected parameter
sections and PLC ownership, decodes HSC channels, positioning axes and PID loops,
and inspects parameters within each configuration. Decode failures are reported
per section, so other records remain accessible. This example needs no `write` feature.

`cargo run --example services_handles -- project.xgwx` inspects high-speed link
blocks, safety channels and trend sections, then lists each PLC's scoped records.
It needs no `write` feature.
