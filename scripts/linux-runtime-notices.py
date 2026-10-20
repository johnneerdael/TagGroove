#!/usr/bin/env python3
"""Generate/check retained Linux runtime notices offline for About and installers."""
import argparse
import importlib.util
from pathlib import Path


def helper(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


core = helper('dependency_notices', 'dependency-notices.py')
runtime = helper('linux_runtime_material', 'linux-runtime-material.py')
BEGIN = b'<!-- BEGIN GENERATED LINUX RUNTIME NOTICES -->'
END = b'<!-- END GENERATED LINUX RUNTIME NOTICES -->'
PAYLOAD = 'THIRD_PARTY_LINUX.md'
COMMON = 'THIRD_PARTY_NOTICES.md'
RTF = 'THIRD_PARTY_NOTICES.rtf'


def attached_common(source, payload):
    """Replace only this block, preserving every other observed source byte."""
    core.require(source.count(BEGIN) == source.count(END) and source.count(BEGIN) <= 1,
                 'Ambiguous Linux runtime notice markers')
    core.require(BEGIN not in payload and END not in payload, 'Linux runtime payload contains notice markers')
    section = BEGIN+b'\n\n'+payload+b'\n'+END
    if BEGIN not in source:
        separator = b'\n' if source.endswith(b'\n') else b'\n\n'
        return source+separator+section+b'\n'
    core.require(source.index(BEGIN) < source.index(END), 'Invalid Linux runtime notice marker order')
    prefix, remaining = source.split(BEGIN, 1)
    old, suffix = remaining.split(END, 1)
    core.require(END not in prefix and BEGIN not in old, 'Invalid Linux runtime notice marker order')
    return prefix+section+suffix


def update(root, check=False):
    source_raw = (root/COMMON).read_bytes()
    payload = runtime.notice_text(root).encode('utf-8')
    common = attached_common(source_raw, payload)
    outputs = {PAYLOAD: payload, COMMON: common, RTF: core.rtf_bytes(common.decode('utf-8'))}
    if check:
        for name, content in outputs.items():
            path = root/name
            core.require(path.is_file() and path.read_bytes() == content, 'Stale Linux runtime notice output: '+name)
    else:
        core.publish_outputs(root, outputs, {COMMON: source_raw})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--generate', action='store_true')
    mode.add_argument('--check', action='store_true')
    args = parser.parse_args()
    try:
        update(args.root.resolve(), check=args.check)
        print('Verified source-bound Linux runtime notice delivery')
    except (OSError, ValueError, KeyError, TypeError) as error:
        parser.exit(1, f'Linux runtime notices: {error}\n')


if __name__ == '__main__':main()
