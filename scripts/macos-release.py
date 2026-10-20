#!/usr/bin/env python3
"""Prepare private CI credentials and gate signed macOS release artifacts."""
import argparse
import base64
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import stat
import subprocess
import tempfile
import uuid

_spec = importlib.util.spec_from_file_location('dependency_notices', Path(__file__).with_name('dependency-notices.py'))
core = importlib.util.module_from_spec(_spec);_spec.loader.exec_module(core)
require, sha = core.require, core.sha
POLICY = 'src-tauri/macos/signing-policy.json'
TARGETS = {'aarch64-apple-darwin': 'macos-arm64', 'x86_64-apple-darwin': 'macos-x64'}
OWNER = 'taggroove-macos-notary'
NAMES = ('APPLE_CERTIFICATE', 'APPLE_CERTIFICATE_PASSWORD', 'APPLE_NOTARY_ISSUER_ID',
         'APPLE_NOTARY_KEY_ID', 'APPLE_NOTARY_PRIVATE_KEY')


def bounded(path, limit):
    info = path.lstat()
    require(stat.S_ISREG(info.st_mode) and 0 < info.st_size <= limit, 'Unsafe macOS release input')
    with path.open('rb') as source:raw = source.read(info.st_size+1)
    require(len(raw) == info.st_size, 'macOS release input size changed')
    return raw


def policy(root):
    return policy_data(bounded(root/POLICY, 8192))


def policy_data(raw):
    data = core.unique_json(raw)
    require(data.get('schema') == 1 and re.fullmatch('[A-Z0-9]{10}', data.get('team_id', ''))
            and data.get('identity', '').startswith('Developer ID Application: ')
            and data['identity'].endswith(' ('+data['team_id']+')'), 'Invalid macOS signing policy')
    return data


def credentials(environment, certificate=False):
    names = NAMES if certificate else ('APPLE_NOTARY_ISSUER_ID', 'APPLE_NOTARY_KEY_ID')
    require(all(isinstance(environment.get(name), str) and environment[name] for name in names),
            'Required macOS signing credentials are missing')
    require(re.fullmatch('[A-Z0-9]{10}', environment['APPLE_NOTARY_KEY_ID']), 'Invalid notary key identifier')
    try:uuid.UUID(environment['APPLE_NOTARY_ISSUER_ID'])
    except (ValueError, TypeError):raise ValueError('Invalid notary issuer identifier') from None
    if certificate:
        try:raw = base64.b64decode(''.join(environment['APPLE_CERTIFICATE'].split()), validate=True)
        except ValueError:raise ValueError('Invalid signing certificate encoding') from None
        require(raw, 'Empty signing certificate')


def prepare(root, directory, environment):
    credentials(environment, certificate=True);selected = policy(root)
    require(not directory.resolve().is_relative_to(root.resolve()), 'Private notary material must stay outside the repository')
    require(not directory.exists() and not directory.is_symlink(), 'Notary staging path already exists')
    key = environment['APPLE_NOTARY_PRIVATE_KEY'].encode()
    if not key.startswith(b'-----BEGIN PRIVATE KEY-----'):
        try:key = base64.b64decode(b''.join(key.split()), validate=True)
        except ValueError:raise ValueError('Invalid notary private-key encoding') from None
    require(0 < len(key) <= 48*1024 and re.fullmatch(
        rb'-----BEGIN PRIVATE KEY-----\r?\n[A-Za-z0-9+/=\r\n]+\r?\n-----END PRIVATE KEY-----\r?\n?', key),
        'Invalid notary private-key envelope')
    config = core.encoded({'bundle': {'macOS': {'signingIdentity': selected['identity'],
                                              'hardenedRuntime': True, 'skipStapling': False}}})
    outputs = {'AuthKey.p8': key, 'tauri-release.json': config}
    outputs['owner.json'] = core.encoded({'owner': OWNER, 'files': {
        name: {'size': len(raw), 'sha256': sha(raw)} for name, raw in outputs.items()}})
    directory.mkdir(mode=0o700, parents=False)
    written = []
    try:
        for name, raw in outputs.items():
            path = directory/name
            with path.open('xb') as target:
                info = path.lstat();written.append((path, (info.st_dev, info.st_ino)))
                os.chmod(path, 0o600);target.write(raw)
    except (OSError, ValueError):
        for path, identity in reversed(written):
            try:info = path.lstat()
            except FileNotFoundError:continue
            if stat.S_ISREG(info.st_mode) and (info.st_dev, info.st_ino) == identity:path.unlink()
        if not any(directory.iterdir()):directory.rmdir()
        raise
    return selected


def cleanup(directory):
    if not directory.exists() and not directory.is_symlink():return
    require(directory.is_dir() and not directory.is_symlink(), 'Unsafe notary cleanup directory')
    manifest = core.unique_json(bounded(directory/'owner.json', 8192))
    require(manifest.get('owner') == OWNER and set(manifest.get('files', {})) == {'AuthKey.p8', 'tauri-release.json'}
            and {path.name for path in directory.iterdir()} == {'owner.json', *manifest['files']},
            'Unknown notary staging material; refusing cleanup')
    for name, record in manifest['files'].items():
        raw = bounded(directory/name, 48*1024)
        require(len(raw) == record['size'] and sha(raw) == record['sha256'], 'Notary staging material changed outside this helper')
    for name in manifest['files']:(directory/name).unlink()
    (directory/'owner.json').unlink();directory.rmdir()


def command(arguments):
    # Apple tools can echo credential identifiers on failure. Keep their output
    # private and expose only the operation name; never print arguments or keys.
    with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
        timeout = 2100 if list(arguments[:3]) == ['xcrun', 'notarytool', 'submit'] else 120
        result = subprocess.run(list(map(str, arguments)), stdout=output, stderr=errors, timeout=timeout)
        output.seek(0);errors.seek(0);stdout = output.read(1024*1024+1);stderr = errors.read(1024*1024+1)
    require(result.returncode == 0 and len(stdout) <= 1024*1024 and len(stderr) <= 1024*1024,
            'macOS release command failed: '+str(arguments[0]))
    return (stdout+stderr if Path(arguments[0]).name == 'codesign' and arguments[1:2] == ['--display']
            else stdout).decode('utf-8')


def signature_metadata(text, selected, identifier, hardened):
    def unique(name):
        values = re.findall(r'^'+name+r'=(.*)$', text, re.M)
        require(len(values) == 1, 'Missing or ambiguous signing metadata: '+name)
        return values[0]
    require(unique('TeamIdentifier') == selected['team_id'] and
            re.findall(r'^Authority=(Developer ID Application: .*)$', text, re.M) == [selected['identity']],
            'macOS signer differs from pinned Developer ID policy')
    if identifier is not None:require(unique('Identifier') == identifier, 'Signed bundle identifier differs')
    require(unique('Timestamp').strip() not in ('', 'none', 'None'), 'Missing secure signing timestamp')
    if hardened:
        flags = re.findall(r'^CodeDirectory .*?\bflags=0x([0-9a-fA-F]+)', text, re.M)
        require(len(flags) == 1 and int(flags[0], 16) & 0x10000, 'Missing hardened runtime signature')
        cdhash = unique('CDHash')
        require(re.fullmatch('[a-fA-F0-9]{40}', cdhash), 'Missing application CodeDirectory identity')
        return cdhash.lower()


def describe(path):
    info = path.lstat();require(stat.S_ISREG(info.st_mode) and info.st_size > 0, 'Missing regular macOS release artifact')
    digest = hashlib.sha256()
    with path.open('rb') as source:
        for raw in iter(lambda: source.read(1024*1024), b''):digest.update(raw)
    require(path.stat().st_size == info.st_size, 'macOS artifact size changed during hashing')
    return {'size': info.st_size, 'sha256': digest.hexdigest()}


def validate_receipt(data, policy_raw, commit, version, release_platform, identifier):
    selected = policy_data(policy_raw)
    require(release_platform in TARGETS.values() and data.get('schema') == 1
            and data.get('platform') == release_platform and TARGETS.get(data.get('target')) == release_platform
            and data.get('source_commit') == commit and data.get('version') == version,
            'macOS signing receipt source/version/target differs')
    require(data.get('identity') == selected['identity'] and data.get('team_id') == selected['team_id']
            and data.get('identifier') == identifier and data.get('policy_sha256') == sha(policy_raw),
            'macOS signing receipt differs from source signing policy')
    expected = {'codesign', 'hardened_runtime', 'stapler', 'gatekeeper', 'dmg_app_binding'}
    require(set(data.get('checks', {})) == expected and all(value is True for value in data['checks'].values()),
            'macOS signing receipt has incomplete verification')
    require(re.fullmatch('[0-9a-f]{40}', data.get('app_cdhash', '')) and data.get('dmg_notarization', {}).get('status') == 'Accepted',
            'macOS signing receipt lacks accepted notarization')
    try:uuid.UUID(data['dmg_notarization']['id'])
    except (ValueError, KeyError, TypeError):raise ValueError('Invalid notarization submission identity') from None
    require(set(data.get('artifacts', {})) == {'.dmg', '.app.tar.gz'}, 'macOS signing receipt artifact formats differ')
    for record in data['artifacts'].values():
        require(type(record.get('size')) is int and record['size'] > 0 and re.fullmatch('[0-9a-f]{64}', record.get('sha256', '')),
                'Invalid signed artifact byte identity')
    return data


def verify_and_package(root, target, directory, commit, version, environment, *, run=command):
    require(target in TARGETS and re.fullmatch('[0-9a-f]{40}', commit), 'Invalid macOS release target/source')
    credentials(environment);selected = policy(root)
    key_path = directory/'AuthKey.p8';bounded(key_path, 48*1024)
    require(key_path.stat().st_mode & 0o777 == 0o600, 'Notary private key must remain private')
    config = core.unique_json(bounded(root/'src-tauri/tauri.conf.json', 1024*1024))
    product = config['productName'];require(re.fullmatch('[A-Za-z][A-Za-z0-9 ._-]*', product), 'Unsafe macOS product name')
    require(config['version'] == version and re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.+-]+)?', version),
            'macOS release version differs from stamped source')
    bundle = root/'target'/target/'release/bundle';app = bundle/'macos'/(product+'.app')
    require(app.is_dir() and not app.is_symlink(), 'Missing regular application bundle')
    dmgs = list((bundle/'dmg').glob('*.dmg'));require(len(dmgs) == 1, 'Expected one unambiguous macOS disk image')
    dmg = dmgs[0];describe(dmg)
    def application_info(path):
        require(path.is_dir() and not path.is_symlink(), 'Missing application bundle')
        info = plistlib.loads(bounded(path/'Contents/Info.plist', 1024*1024))
        bundle_version = config.get('bundle', {}).get('macOS', {}).get('bundleVersion', version)
        require(info.get('CFBundleIdentifier') == config['identifier'] and info.get('CFBundleShortVersionString') == version
                and info.get('CFBundleVersion') == bundle_version, 'Built application identity/version differs from release')
    application_info(app)
    def signed(path, hardened):
        run(['codesign', '--verify', '--deep', '--strict', path])
        return signature_metadata(run(['codesign', '--display', '--verbose=4', path]), selected,
                                  config['identifier'] if hardened else None, hardened)
    app_cdhash = signed(app, True);run(['xcrun', 'stapler', 'validate', app])
    run(['spctl', '--assess', '--type', 'execute', '--verbose=2', app])
    signed(dmg, False)
    # The image's outer signature alone permits an old same-signer app. Mount
    # read-only and bind its fully verified app seal to the collected loose app.
    with tempfile.TemporaryDirectory(prefix='taggroove-dmg-', dir=directory.parent) as temporary:
        mount = Path(temporary)/'image';mount.mkdir();mounted = False
        try:
            mounted_info = run(['hdiutil', 'attach', dmg, '-readonly', '-nobrowse', '-noautoopen', '-plist', '-mountpoint', mount])
            mounted = True
            volumes = plistlib.loads(mounted_info.encode()).get('system-entities', [])
            require([row['mount-point'] for row in volumes if 'mount-point' in row] == [str(mount)],
                    'Unexpected mounted disk image layout')
            contained = mount/(product+'.app');application_info(contained)
            require(signed(contained, True) == app_cdhash, 'DMG contained application differs from verified release bundle')
            run(['xcrun', 'stapler', 'validate', contained])
        finally:
            if mounted:run(['hdiutil', 'detach', mount])
    submitted = core.unique_json(run(['xcrun', 'notarytool', 'submit', dmg, '--key', key_path,
        '--key-id', environment['APPLE_NOTARY_KEY_ID'], '--issuer', environment['APPLE_NOTARY_ISSUER_ID'],
        '--wait', '--timeout', '30m', '--output-format', 'json']))
    require(submitted.get('status') == 'Accepted', 'DMG notarization was not accepted')
    try:submission_id = str(uuid.UUID(submitted['id']))
    except (ValueError, KeyError):raise ValueError('Missing accepted DMG notarization identity') from None
    run(['xcrun', 'stapler', 'staple', dmg]);run(['xcrun', 'stapler', 'validate', dmg]);signed(dmg, False)
    run(['spctl', '--assess', '--type', 'open', '--context', 'context:primary-signature', '--verbose=2', dmg])
    # Ignore any action-created archive. Create the collected archive only after
    # the app's Developer ID signature, hardened runtime and stapled ticket pass.
    artifacts = directory.parent/('taggroove-signed-'+target)
    require(not artifacts.exists() and not artifacts.is_symlink(), 'macOS artifact staging already exists')
    artifacts.mkdir(mode=0o700);archive = artifacts/(product+'.app.tar.gz')
    run(['tar', 'czf', archive, '-C', app.parent, app.name])
    receipt = {'schema': 1, 'platform': TARGETS[target], 'target': target, 'source_commit': commit,
        'version': version, 'identity': selected['identity'], 'team_id': selected['team_id'],
        'identifier': config['identifier'], 'policy_sha256': sha(bounded(root/POLICY, 8192)),
        'checks': {'codesign': True, 'hardened_runtime': True, 'stapler': True, 'gatekeeper': True, 'dmg_app_binding': True},
        'app_cdhash': app_cdhash,
        'dmg_notarization': {'status': 'Accepted', 'id': submission_id},
        'artifacts': {'.app.tar.gz': describe(archive), '.dmg': describe(dmg)}}
    validate_receipt(receipt, bounded(root/POLICY, 8192), commit, version, TARGETS[target], config['identifier'])
    relative = 'release-assets/macos-signing-'+TARGETS[target]+'.json'
    core.publish_outputs(root, {relative: core.encoded(receipt)}, {relative: None})
    return {'artifact_paths': [str(archive), str(dmg)], 'receipt_path': str(root/relative)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=['prepare', 'verify-built', 'cleanup'])
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--target', choices=TARGETS);parser.add_argument('--source-commit');parser.add_argument('--version')
    parser.add_argument('--github-output', type=Path)
    args = parser.parse_args();require(platform.system() == 'Darwin', 'macOS release operations require an Apple builder')
    if args.operation == 'prepare':prepare(args.root.resolve(), args.directory, os.environ)
    elif args.operation == 'cleanup':cleanup(args.directory)
    else:
        require(args.target and args.source_commit and args.version, 'Verification requires target/source/version')
        result = verify_and_package(args.root.resolve(), args.target, args.directory, args.source_commit, args.version, os.environ)
        if args.github_output:
            require(args.github_output.is_file() and not args.github_output.is_symlink(), 'Unsafe workflow output file')
            with args.github_output.open('a', encoding='utf-8') as output:
                output.write('artifact_paths='+json.dumps(result['artifact_paths'])+'\n')
        print(json.dumps(result))


if __name__ == '__main__':
    try:main()
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError):
        # Credential paths and values are never included in error output.
        raise SystemExit('macOS release operation refused; check credentials, signing policy and Apple verification status') from None
