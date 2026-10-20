#!/usr/bin/env python3
"""Prepare pinned Linux packaging inputs in checkout-owned paths, without running them."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import urllib.request

_spec = importlib.util.spec_from_file_location('dependency_notices', Path(__file__).with_name('dependency-notices.py'))
core = importlib.util.module_from_spec(_spec);_spec.loader.exec_module(core)
require, sha = core.require, core.sha
CATALOG = 'scripts/notices/linux-packager-tools.json'
OUTPUT = 'target/linux-source-tools'
ARCHES = {'amd64': 'x86_64', 'arm64': 'aarch64'}
MAX_ASSET = 64*1024*1024


def download(url):
    require(url.startswith('https://'), 'Packaging asset requires HTTPS')
    request = urllib.request.Request(url, headers={'Accept': 'application/octet-stream', 'User-Agent': 'TagGroove-source-delivery'})
    with urllib.request.urlopen(request, timeout=60) as response:
        require(response.geturl().startswith('https://'), 'Packaging asset redirected outside HTTPS')
        raw = response.read(MAX_ASSET+1)
    require(len(raw) <= MAX_ASSET, 'Packaging asset exceeds bounds')
    return raw


def checked(raw, pin, *, size_field='size', alternatives=()):
    require(type(pin[size_field]) is int and 0 < pin[size_field] <= MAX_ASSET,
            'Invalid packaging asset size pin')
    require(len(raw) == pin[size_field] and sha(raw) in {pin['sha256'], *alternatives},
            'Pinned packaging input bytes differ')
    return raw


def inspect(inspector, arguments, bound):
    with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
        result = subprocess.run([str(inspector), *arguments], stdout=output, stderr=errors, timeout=60,
                                env={**os.environ, 'LC_ALL': 'C'})
        output.seek(0);errors.seek(0);raw = output.read(bound+1);diagnostic = errors.read(65537)
    require(result.returncode == 0 and len(raw) <= bound and len(diagnostic) <= 65536,
            'Unable to inspect pinned packaging tools within bounds')
    return raw


def extract_tools(inspector, asset, pins):
    """Extract only unique regular members, never run the AppImage or its tools."""
    require(set(pins) == {'strip', 'patchelf'}, 'Unexpected packaging tool set')
    with tempfile.TemporaryDirectory(prefix='taggroove-linux-tools-') as folder:
        image = Path(folder)/'linuxdeploy.AppImage';image.write_bytes(asset)
        listing = inspect(inspector, ['l', '-slt', '-sccUTF-8', str(image)], 8*1024*1024).decode('utf-8')
        members = {}
        for block in listing.replace('\r\n', '\n').split('\n\n'):
            entry = dict(line.split(' = ', 1) for line in block.splitlines() if ' = ' in line)
            member = entry.get('Path')
            if member in {pin['member'] for pin in pins.values()}:
                require(member not in members and entry.get('Folder') == '-'
                        and not entry.get('Symbolic Link') and not entry.get('Hard Link')
                        and not entry.get('Mode', '').startswith('l'), 'Ambiguous or linked packaging tool member')
                members[member] = entry
        require(set(members) == {pin['member'] for pin in pins.values()}, 'Missing pinned packaging tool members')
        return {name: checked(inspect(inspector, ['x', '-so', str(image), pin['member']], pin['size']), pin)
                for name, pin in pins.items()}


def prepare(root, architecture, inspector, *, fetch=download):
    require(architecture in ARCHES, 'Unsupported Linux packaging architecture')
    spec = core.unique_json((root/CATALOG).read_bytes())
    require(spec['schema'] == 1 and spec['linuxdeploy_commit'] == '07333c6', 'Unexpected Linux packager pin catalog')
    cli = core.unique_json((root/'package-lock.json').read_bytes())['packages']['node_modules/@tauri-apps/cli']
    require(all(cli.get(field) == spec['tauri_cli'][field] for field in ['version', 'integrity']),
            'Linux packaging pins require the exact locked Tauri CLI')
    arch = ARCHES[architecture];pin = spec['architectures'][architecture]
    require(pin['asset'] == 'linuxdeploy-'+arch+'.AppImage', 'Linux packager architecture differs')
    runtime = core.unique_json((root/'scripts/notices/linux-runtime-material.json').read_bytes())['asset_pins'][arch]
    inspector_spec = core.unique_json((root/'src/licenses/appimage-common/SOURCES.json').read_bytes())['inspector']
    inspectors = [entry for entry in inspector_spec['archives'] if entry['arch'] == arch]
    require(len(inspectors) == 1 and inspector.is_file() and not inspector.is_symlink()
            and sha(inspector.read_bytes()) == inspectors[0]['binary_sha256'], 'Pinned host inspector bytes differ')
    cache = OUTPUT+'/xdg-cache'
    base = OUTPUT+'/'+architecture
    cache_asset = cache+'/tauri/linuxdeploy-'+spec['linuxdeploy_commit']+'-'+arch+'.AppImage'
    guards, outputs = {}, {}

    def current(relative):
        path = root/core.safe(relative)
        for parent in path.parents:
            if parent == root:break
            require(not parent.is_symlink() and (not parent.exists() or parent.is_dir()), 'Unsafe packaging stage parent')
        require(not path.is_symlink() and (not path.exists() or path.is_file()), 'Unsafe packaging stage file')
        raw = None
        if path.exists():
            require(path.stat().st_size <= MAX_ASSET, 'Existing packaging input exceeds bounds')
            with path.open('rb') as handle:raw = handle.read(MAX_ASSET+1)
            require(len(raw) <= MAX_ASSET, 'Existing packaging input grew beyond bounds')
        guards[relative] = raw
        return raw

    def asset(relative, record, size_field='size', alternatives=()):
        raw = current(relative)
        if raw is None:raw = fetch(record.get('asset_api_url', record['url']))
        checked(raw, record, size_field=size_field, alternatives=alternatives)
        outputs[relative] = raw
        return raw

    deployed = asset(cache_asset, pin, alternatives=(pin['cli_modified_sha256'],))
    asset(base+'/runtime', runtime['runtime'], 'byte_length')
    asset(cache+'/tauri/AppRun-'+arch, runtime['launcher'], 'byte_length')
    require(set(pin['tools']) == {'strip', 'patchelf'}, 'Unexpected packaging tool set')
    missing = []
    for name, record in pin['tools'].items():
        relative = base+'/'+name;raw = current(relative)
        if raw is None:missing.append(name)
        else:outputs[relative] = checked(raw, record)
    if missing:
        extracted = extract_tools(inspector, deployed, pin['tools'])
        for name in missing:outputs[base+'/'+name] = checked(extracted[name], pin['tools'][name])
    core.publish_outputs(root, outputs, guards)
    for relative in outputs:os.chmod(root/relative, 0o755)
    return {'cache': root/cache, 'runtime': root/base/'runtime',
            'tools': {name: root/base/name for name in pin['tools']}, 'tool_pins': pin['tools']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--architecture', choices=ARCHES, required=True)
    parser.add_argument('--inspector', type=Path, required=True)
    args = parser.parse_args()
    try:
        result = prepare(args.root.resolve(), args.architecture, args.inspector.resolve())
        print(json.dumps({'cache': str(result['cache']), 'runtime': str(result['runtime']),
                          'tools': {name: str(path) for name, path in result['tools'].items()}}))
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        parser.exit(1, 'Linux packaging tools: '+str(error)+'\n')


if __name__ == '__main__':main()
