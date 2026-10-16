#!/usr/bin/env python3
"""Build and smoke-test release libraries from the exact crates.io package."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile


ROOT = Path(__file__).resolve().parents[1]


def run(*args, **kwargs):
    return subprocess.check_output(args, text=True, **kwargs).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("crate", type=Path)
    parser.add_argument("--target", required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    args = parser.parse_args()
    crate = args.crate.resolve()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    name = f"libxgwx-{version}-{args.target}"
    commit = run("git", "rev-parse", "HEAD", cwd=ROOT)

    with tempfile.TemporaryDirectory() as temporary:
        temporary = Path(temporary)
        with tarfile.open(crate) as archive:
            archive.extractall(temporary / "source", filter="data")
        source = temporary / "source" / f"libxgwx-{version}"
        packaged = tomllib.loads((source / "Cargo.toml").read_text())["package"]
        if packaged["name"] != "libxgwx" or packaged["version"] != version:
            raise SystemExit("Crate name/version does not match this checkout")
        vcs = json.loads((source / ".cargo_vcs_info.json").read_text())
        if vcs["git"]["sha1"] != commit or vcs["git"].get("dirty", False):
            raise SystemExit("Crate must come from this exact clean source commit")

        target_dir = ROOT / "target" / "release-ffi"
        subprocess.run([
            "cargo", "build", "--locked", "--release", "--lib",
            "--no-default-features", "--features", "ffi,write,il",
            "--target", args.target, "--target-dir", str(target_dir),
            "--manifest-path", str(source / "Cargo.toml"),
        ], check=True)
        built = target_dir / args.target / "release"
        bundle = temporary / name
        (bundle / "include").mkdir(parents=True)
        (bundle / "lib").mkdir()
        shutil.copy2(source / "include" / "xgwx.h", bundle / "include")
        shutil.copy2(source / "LICENSE-APACHE", bundle)
        windows = args.target.endswith("windows-msvc")
        macos = args.target.endswith("apple-darwin")
        libraries = (["xgwx.dll", "xgwx.dll.lib", "xgwx.lib"] if windows else
                     ["libxgwx.dylib" if macos else "libxgwx.so", "libxgwx.a"])
        for library in libraries:
            shutil.copy2(built / library, bundle / "lib")

        # Link a real C caller to the staged header and shared library, then
        # exercise ownership, guarded edits, and serialization without fixtures.
        smoke = temporary / ("smoke.exe" if windows else "smoke")
        if windows:
            subprocess.run([
                "cl", "/nologo", "/W4", "/std:c11",
                f"/I{bundle / 'include'}", str(ROOT / "tests" / "ffi_smoke.c"),
                str(bundle / "lib" / "xgwx.dll.lib"), f"/Fe:{smoke}",
            ], check=True, cwd=temporary)
        else:
            subprocess.run([
                os.environ.get("CC", "cc"), "-std=c11", "-Wall", "-Wextra", "-Werror",
                f"-I{bundle / 'include'}", str(ROOT / "tests" / "ffi_smoke.c"),
                f"-L{bundle / 'lib'}", "-lxgwx",
                f"-Wl,-rpath,{bundle / 'lib'}", "-o", str(smoke),
            ], check=True)
        environment = os.environ.copy()
        environment["PATH"] = str(bundle / "lib") + os.pathsep + environment["PATH"]
        subprocess.run([str(smoke)], check=True, env=environment)
        metadata = {
            "version": version, "source_commit": commit, "target": args.target,
            "features": ["ffi", "write", "il"],
            "crate_sha256": hashlib.sha256(crate.read_bytes()).hexdigest(),
            "rustc": run("rustc", "--version", "--verbose"),
            "runner_os": os.environ.get("ImageOS", os.name),
        }
        (bundle / "BUILD.json").write_text(json.dumps(metadata, indent=2) + "\n")
        (bundle / "README.txt").write_text(
            f"libxgwx {version} ({args.target})\n"
            "Features: ffi,write,il. Header: include/xgwx.h. Libraries: lib/.\n"
            "Use the shared library for ctypes/PInvoke. Windows C/C++ callers\n"
            "link xgwx.dll.lib and deploy xgwx.dll alongside the application.\n"
            "The static library also requires the platform's native Rust link dependencies.\n"
            "Build toolchain and source identity: BUILD.json.\n"
            f"C API guide: https://github.com/ne0ekspert/libxgwx/blob/v{version}/docs/c-api.md\n"
        )
        if windows:
            destination = output / f"{name}.zip"
            with zipfile.ZipFile(destination, "w", zipfile.ZIP_DEFLATED) as archive:
                for path in sorted(bundle.rglob("*")):
                    if path.is_file():
                        archive.write(path, path.relative_to(temporary))
        else:
            destination = output / f"{name}.tar.gz"
            with tarfile.open(destination, "w:gz") as archive:
                archive.add(bundle, arcname=name)
        print(destination)


if __name__ == "__main__":
    main()
