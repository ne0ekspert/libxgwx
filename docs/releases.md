# Release procedure

Use one version for crates.io and GitHub Releases: `Cargo.toml` version `X.Y.Z`
must match tag `vX.Y.Z`. Do not move a published tag or reuse a crate version.
The current development version is still `0.1.0`; choose a new version before
releasing changes beyond the existing crates.io upload.

## Repository setup

Create a GitHub Actions environment named `crates-io` and add an environment
secret named `CARGO_REGISTRY_TOKEN` with permission to publish `libxgwx`.
The workflow uses the repository's `GITHUB_TOKEN` to manage Releases.

## Prepare and release

1. Update the version in `Cargo.toml` and its package entry in `Cargo.lock`.
   Review and commit the intended source changes.
2. Run **Release crate and native libraries** with **Run workflow** on the
   intended branch. Branch runs build and validate candidates without publishing.
   Changes to the release workflow/helpers also trigger this validation on PRs.
3. After reviewing the checks and downloadable candidates, tag that same commit
   and push the tag. For example, for a prepared `0.1.1` release:

   ```sh
   git tag -a v0.1.1 <reviewed-commit> -m 'libxgwx 0.1.1'
   git push origin v0.1.1
   ```

The tag workflow verifies the version, runs fixture-backed Rust and C ABI tests,
checks the extracted crate's optional APIs, and runs `cargo publish --dry-run`.
It builds the FFI libraries from that exact `.crate` archive with `ffi,write,il`
and default features disabled. Each native runner compiles and runs a real C
caller against the staged shared library and header before archiving them.

After every platform passes, the workflow creates a draft Release with all
assets, publishes the crate, verifies the registry checksum against the attached
`.crate`, then makes the GitHub Release public. The two services cannot publish
atomically; the draft remains available for recovery if a later step fails.

## Release assets

| Target | Archive |
| --- | --- |
| Linux x86-64, glibc (built on Ubuntu 22.04) | `libxgwx-X.Y.Z-x86_64-unknown-linux-gnu.tar.gz` |
| Windows x86-64, MSVC | `libxgwx-X.Y.Z-x86_64-pc-windows-msvc.zip` |
| macOS Intel (built on macOS 15) | `libxgwx-X.Y.Z-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon (built on macOS 15) | `libxgwx-X.Y.Z-aarch64-apple-darwin.tar.gz` |

Each archive contains `include/xgwx.h`, shared and static libraries under `lib/`,
`LICENSE-APACHE`, usage notes, and `BUILD.json` recording the source commit,
target, enabled features, Rust toolchain, and source package SHA-256.
Windows includes both `xgwx.dll.lib` (DLL import library) and `xgwx.lib`
(static library). Static linking also requires Rust's platform system libraries;
the automated archive smoke check exercises shared linking.

The Release also includes `libxgwx-X.Y.Z.crate` and `SHA256SUMS` for all archives
and the crate. Download only the archive for your platform; verify it against
the corresponding checksum. Archives omit fixtures and private workspaces.

## Recover a failed release

Rerun the failed jobs for the same tag. A matching existing crates.io checksum
skips the upload; a differing checksum stops the workflow and requires a new
version. Draft Release assets can be replaced during recovery. The workflow
refuses to overwrite an already public GitHub Release.

To retry through **Run workflow**, select the existing version tag. Selecting a
branch performs candidate validation only. Never delete a published crate or
retag different source as a recovery step.
