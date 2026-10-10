# User examples

Run these commands from a repository checkout. Supply your own workspace file;
fixtures and examples are excluded from the crates.io package.

| Example | Purpose | Command |
| --- | --- | --- |
| `inspect` | Print project, program and hardware summaries | `cargo run --example inspect -- project.xgwx` |
| `il` | Print IL converted from supported ladder programs | `cargo run --features il --example il -- project.xgwx` |
| `write-module` | Change a supported input module's filter | `cargo run --features write --example write-module -- input.xgwx output.xgwx 0 2 5` |
| `tui` | Browse a workspace in an interactive terminal | `cargo run --example tui -- project.xgwx` |
| `gui` | View decoded ladder programs on a graphical desktop | `cargo run --features gui --example gui -- project.xgwx` |

Among the Rust examples, only `write-module` writes a file. Its output path may
be overwritten; choose a different path from the input when preserving the original.

The [C inspector](c/inspect.c) demonstrates native JSON inspection and optional
guarded renaming into a new file. Build commands and ownership rules are in
the [C API guide](../docs/c-api.md).

The [Python ctypes example](python/lists.py) loads module/network/program/global
lists and optionally applies an atomic JSON edit batch through the same C ABI.

Acceptance generators, diagnostics, probes and benchmarks live in
[`dev-tools`](../dev-tools/README.md).
