# Development tools

This unpublished Cargo package holds acceptance generators, diagnostic probes,
record comparisons and benchmarks. These are development utilities, not user
examples. Some require repository fixtures, specific native layouts, or private
input files. They may generate or overwrite output files. Consult each tool's
source and usage message before running it.

Run from the repository root so relative fixture paths resolve:

```sh
cargo run --manifest-path dev-tools/Cargo.toml --bin xg5000-acceptance -- generate /tmp/xgwx-run
cargo run --release --manifest-path dev-tools/Cargo.toml --bin bench -- --warmup 50 --iterations 500 fixtures
```

The default features enable the library's `write` and `il` APIs. Binary names are
listed in `Cargo.toml` and preserve the former Cargo example target names.
Generated candidates still require native XG5000 acceptance when documented by
the tool; a local parse or byte comparison does not establish native acceptance.

This directory and all fixtures are excluded from the crates.io package.
