#!/usr/bin/env python3
"""Resolve native test build configuration and launch paths from the same metadata."""
import argparse
import json
import os
from pathlib import Path
import tomllib


def metadata(root):
    product = json.loads((root / "src-tauri/tauri.conf.json").read_text())["productName"]
    cargo = tomllib.loads((root / "src-tauri/Cargo.toml").read_text())
    binaries = cargo.get("bin", [{"name": cargo["package"]["name"]}])
    if len(binaries) != 1:
        raise ValueError("Native tests require one explicit application binary")
    return product, binaries[0]["name"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--binary-path", choices=["macos", "windows", "linux"])
    parser.add_argument("--runner-os", default=os.environ.get("RUNNER_OS", ""))
    parser.add_argument("--identifier", default="org.opentag.editor.usertest")
    args = parser.parse_args()
    product, binary = metadata(args.root)
    if args.binary_path:
        if args.binary_path == "macos":
            print(f"target/debug/bundle/macos/{product} UserTest.app/Contents/MacOS/{binary}")
        else:
            print(f"target/debug/{binary}" + (".exe" if args.binary_path == "windows" else ""))
        return
    config = {"identifier": args.identifier, "productName": f"{product} UserTest"}
    if args.runner_os != "Linux":
        config["app"] = {"windows": [{"title": product, "width": 1240, "height": 800,
                                     "minWidth": 800, "minHeight": 560,
                                     "backgroundThrottling": "disabled"}]}
    print(json.dumps(config))


if __name__ == "__main__":
    main()
