#!/usr/bin/env python3
"""Verify that a crates.io version has the exact candidate package checksum."""

import argparse
import hashlib
import json
from pathlib import Path
import time
import tomllib
import urllib.error
import urllib.request


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("crate", type=Path)
    parser.add_argument("--wait", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    version = tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
    expected = hashlib.sha256(args.crate.read_bytes()).hexdigest()
    request = urllib.request.Request(
        f"https://crates.io/api/v1/crates/libxgwx/{version}",
        headers={"User-Agent": "libxgwx-release (https://github.com/ne0ekspert/libxgwx)"},
    )
    for attempt in range(24 if args.wait else 1):
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                actual = json.load(response)["version"]["checksum"]
        except urllib.error.HTTPError as error:
            if error.code != 404:
                raise
            if not args.wait:
                print("missing")
                return
            if attempt < 23:
                time.sleep(5)
                continue
            raise SystemExit("Published version has not appeared on crates.io")
        if actual != expected:
            raise SystemExit(f"crates.io already has different contents for {version}; choose a new version")
        print("published")
        return


if __name__ == "__main__":
    main()
