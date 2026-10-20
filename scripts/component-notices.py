#!/usr/bin/env python3
"""Verify curated native/installer material and its package bindings; no target execution."""
import argparse
import hashlib
import importlib.util
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import zipfile
import io

_spec = importlib.util.spec_from_file_location('dependency_notices', Path(__file__).with_name('dependency-notices.py'))
core = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(core)
require, sha, safe, encoded = core.require, core.sha, core.safe, core.encoded
MATERIAL = 'scripts/notices/component-material.json'
PAYLOAD = 'THIRD_PARTY_NATIVE.md'


def notice_slice(original, entry):
    require(sha(original) == entry['source_sha256'], 'Native notice original source hash differs')
    offset, length = entry['offset'], entry['length']
    require(type(offset) is int and type(length) is int and offset >= 0 and length > 0
            and offset + length <= len(original), 'Invalid native notice slice bounds')
    raw = original[offset:offset + length]
    require(sha(raw) == entry['sha256'], 'Native notice slice hash differs')
    raw.decode('utf-8')
    return raw


def leading_notices(raw):
    match = re.match(rb'(?:\s*/\*.*?\*/)+\s*', raw, re.S)
    if match:
        return match[0]
    lines = []
    for line in raw.splitlines(keepends=True):
        if line.lstrip().startswith(b';') or not line.strip():
            lines.append(line)
        else:
            break
    result = b''.join(lines)
    require(result.strip(), 'Missing leading native notice comments')
    return result


def supplier_blocks(raw):
    """Locate explicit supplier grants, including notices inside implementation files."""
    comments = list(re.finditer(rb'/\*.*?\*/', raw, re.S))
    comments += list(re.finditer(rb'(?m)(?:^[ \t]*//[^\n]*(?:\n|$))+', raw))
    comments += list(re.finditer(rb'(?m)(?:^[ \t]*;[^\n]*(?:\n|$))+', raw))
    result = []
    for comment in comments:
        language = re.sub(rb'(?m)^[ \t]*(?:\*+|;[ *]*)[ \t]?', b'', comment[0])
        if re.search(rb'Permission\s+(?:is\s+hereby|is\s+granted|to\s+use)|Redistribution\s+and\s+use|public\s+domain|Boost\s+Software\s+License', language, re.I):
            result.append((comment.start(), comment[0]))
    return result


def verify_ffmpeg_coverage(spec, archive, directory, expected, read_notice):
    require(sha(archive) == spec['bindings']['ffmpeg']['sha256'], 'Native notice source archive differs')
    prefix = safe(directory) + '/'
    mapping = {}
    for item in spec['materials'].values():
        origin = item['origin']
        if origin['kind'] == 'tar_slice':
            name = origin['member'].removeprefix(prefix)
            mapping.setdefault(name, []).append(item)
    found = set()
    with tarfile.open(fileobj=io.BytesIO(archive), mode='r|*') as handle:
        for member in handle:
            if not member.name.startswith(prefix):
                continue
            name = member.name.removeprefix(prefix)
            if name not in expected:
                continue
            require(name not in found and member.isfile(), 'Duplicate/nonregular native compiler source')
            found.add(name)
            original = handle.extractfile(member).read()
            if name in {'libavcodec/file_open.c', 'libavformat/file_open.c'}:
                require(original.strip() == b'#include "libavutil/file_open.c"'
                        and 'libavutil/file_open.c' in expected, 'Unmapped native source alias')
                continue
            notice = leading_notices(original)
            language = re.sub(rb'(?m)^[ \t]*(?:\*+|;[ *]*)[ \t]?', b'', notice)
            lgpl = re.search(rb'GNU\s+(?:Lesser|Library)\s+General\s+Public\s+License', language, re.I)
            permission = re.search(rb'\bPermission\b|Redistribution\s+and\s+use|public\s+domain', language, re.I)
            require(lgpl or permission, 'Unmapped native source license header: ' + name)
            if permission:
                require(mapping.get(name), 'Unmapped native source notice: ' + name)
            for item in mapping.get(name, []):
                retained = read_notice(safe(item['source_file']))
                require(sha(retained) == item['sha256'] and notice_slice(original, item['origin']) == retained,
                        'Native source notice differs: ' + name)
            for offset, block in supplier_blocks(original):
                matching = [item for item in mapping.get(name, []) if item['origin']['offset'] <= offset
                            and offset + len(block) <= item['origin']['offset'] + item['origin']['length']]
                require(matching, 'Unmapped native supplier notice block: ' + name)
    require(found == set(expected), 'Native compiler source missing from notice coverage pass')


def verify_android(root, spec, package, ndk):
    android = spec['android']
    revision = (ndk / 'source.properties').read_text(encoding='utf-8')
    require(re.search(r'^Pkg\.Revision\s*=\s*' + re.escape(android['ndk_revision']) + r'\s*$', revision, re.M),
            'Android native notice NDK revision differs')
    with zipfile.ZipFile(package) as apk:
        names = apk.namelist()
        require(len(names) == len(set(names)), 'Duplicate APK member')
        libraries = {name for name in names if name.startswith('lib/') and name.endswith('.so')}
        require(libraries == {'lib/arm64-v8a/libc++_shared.so', 'lib/arm64-v8a/libopentag_lib.so'},
                'Unexpected or missing bundled Android native runtime')
        raw = apk.read('lib/arm64-v8a/libc++_shared.so')
        require(sha(raw) == android['runtime_sha256'], 'Bundled Android C++ runtime hash differs')
    candidates = list((ndk / 'toolchains/llvm/prebuilt').glob('*/sysroot/usr/lib/aarch64-linux-android/libc++_shared.so'))
    require(len(candidates) == 1 and candidates[0].read_bytes() == raw, 'Bundled C++ runtime differs from selected NDK')
    notice = (candidates[0].parents[4] / 'NOTICE').read_bytes()
    for name in android['notice_sources']:
        require((root / safe(name)).read_bytes() in notice, 'Android runtime notice is absent from selected NDK')


def verify_windows_helpers(spec, members):
    expected = spec['windows']['helpers']
    require(set(members) == set(expected), 'Unexpected or missing Windows installer helper')
    for name, raw in members.items():
        require(sha(raw) == expected[name]['sha256'], 'Windows installer helper hash differs: ' + name)


def windows_member_names(raw):
    result = {}
    for name in re.findall(r'^Path = (.*\.dll)\r?$', raw.decode('utf-8', errors='strict'), re.M | re.I):
        normalized = name.replace('\\', '/')
        require(normalized not in result, 'Duplicate Windows helper path alias')
        result[normalized] = name
    return result


def verify_windows(spec, package, extractor=None):
    extractor = extractor or shutil.which('7zz') or shutil.which('7z')
    require(extractor is not None, 'Windows installer inspection requires 7zz or 7z')
    listing = subprocess.run([str(extractor), 'l', '-slt', '-sccUTF-8', str(package)], capture_output=True, check=False)
    require(listing.returncode == 0, 'Unable to inspect Windows installer archive')
    names = windows_member_names(listing.stdout)
    require(set(names) == set(spec['windows']['helpers']),
            'Unexpected or missing Windows installer helper')
    members = {}
    for name, source_name in names.items():
        result = subprocess.run([str(extractor), 'x', '-so', str(package), source_name], capture_output=True, check=False)
        require(result.returncode == 0, 'Unable to read Windows installer helper')
        members[name] = result.stdout
    verify_windows_helpers(spec, members)


def built_package(root, platform):
    require(platform in {'android', 'windows'}, 'Unsupported component package platform')
    pattern = ('src-tauri/gen/android/app/build/outputs/apk/**/release/*.apk' if platform == 'android'
               else 'target/x86_64-pc-windows-msvc/release/bundle/nsis/*.exe')
    candidates = [path for path in root.glob(pattern) if path.is_file()]
    require(len(candidates) == 1, 'Expected one unambiguous built ' + platform + ' package')
    return candidates[0]


def materials(root, spec, upstream=None):
    """Validate retained bytes; optional archive replay never extracts or executes."""
    require(spec.get('schema') == 1, 'Unexpected component notice schema')
    result, archives = {}, {}
    if upstream is not None:
        groups = {}
        for item in spec['materials'].values():
            origin = item['origin']
            if origin['kind'] not in {'tar_slice', 'zip_slice'}:
                continue
            filename, member = safe(origin['archive']), safe(origin['member'])
            group = groups.setdefault(filename, {'sha256': origin['archive_sha256'], 'kind': origin['kind'], 'members': set()})
            require(group['sha256'] == origin['archive_sha256'] and group['kind'] == origin['kind'],
                    'Conflicting component archive provenance')
            group['members'].add(member)
        for filename, group in groups.items():
            archive = upstream / filename
            require(sha(archive.read_bytes()) == group['sha256'], 'Component upstream archive hash differs')
            found = {}
            if group['kind'] == 'tar_slice':
                # One streaming pass, rather than re-decompressing the archive per notice.
                with tarfile.open(archive, 'r|*') as handle:
                    for member in handle:
                        if member.name not in group['members']:
                            continue
                        require(member.name not in found and member.isfile(), 'Duplicate/nonregular component source')
                        found[member.name] = handle.extractfile(member).read()
            else:
                with zipfile.ZipFile(archive) as handle:
                    for member in handle.infolist():
                        if member.filename not in group['members']:
                            continue
                        require(member.filename not in found and not member.is_dir(), 'Duplicate/nonregular component license')
                        found[member.filename] = handle.read(member)
            require(set(found) == group['members'], 'Missing upstream component material')
            archives[filename] = found
    for key, item in spec['materials'].items():
        safe(key)
        raw = (root / safe(item['source_file'])).read_bytes()
        require(sha(raw) == item['sha256'], 'Component notice source hash differs: ' + key)
        raw.decode('utf-8')
        origin = item['origin']
        kind = origin['kind']
        if kind == 'git_blob':
            require(re.fullmatch('[a-f0-9]{40}', origin['commit'])
                    and '/' + origin['commit'] + '/' in origin['source_url'], 'Unpinned component Git source')
            require(hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest() == origin['git_blob_sha1'],
                    'Component license Git blob differs')
        elif kind == 'tracked_slice':
            original = (root / safe(origin['source_file'])).read_bytes()
            require(notice_slice(original, origin) == raw, 'Tracked component notice slice differs')
        else:
            require(kind in {'tar_slice', 'zip_slice'}, 'Unknown component notice origin')
            safe(origin['archive'])
            safe(origin['member'])
            require(re.fullmatch('[a-f0-9]{64}', origin['archive_sha256']), 'Invalid component archive hash')
            if upstream is not None:
                original = archives[origin['archive']][origin['member']]
                if kind == 'tar_slice':
                    if origin.get('selection', 'leading') == 'leading':
                        require(leading_notices(original) == raw, 'Native notice is not its complete leading comments')
                    else:
                        require(origin['selection'] == 'supplier-comment'
                                and (origin['offset'], raw) in supplier_blocks(original), 'Native supplier comment selection differs')
                require(notice_slice(original, origin) == raw, 'Component upstream notice differs')
        result[key] = raw
    return result


def validate_bindings(root, spec):
    ffmpeg = core.unique_json((root / 'scripts/audio-deps/sources.json').read_bytes())['components']['ffmpeg']
    require(spec['bindings']['ffmpeg'] == ffmpeg, 'Native notice FFmpeg source pin differs')
    cli = core.unique_json((root / 'package-lock.json').read_bytes())['packages']['node_modules/@tauri-apps/cli']
    require(spec['bindings']['tauri_cli'] == {key: cli[key] for key in ['version', 'integrity']},
            'Installer notice CLI pin differs')
    installer = core.unique_json((root / 'src-tauri/windows/UPSTREAM.json').read_bytes())
    require(installer['cliVersion'] == cli['version']
            and installer['testToolset']['version'] == spec['windows']['nsis_version']
            and installer['testToolset']['sha256'] == spec['windows']['nsis_archive_sha256'],
            'Installer notice toolchain binding differs')
    workflow = (root / '.github/workflows/reviewed-release.yml').read_text(encoding='utf-8')
    require('ndk;' + spec['android']['ndk_revision'] in workflow, 'Android runtime notice SDK pin differs')
    require(set(spec['windows']['helpers']) == {'$PLUGINSDIR/' + name for name in [
        'System.dll', 'nsDialogs.dll', 'StartMenu.dll', 'NSISdl.dll', 'nsis_tauri_utils.dll']},
        'Unexpected installer helper inventory')
    for entry in spec['windows']['helpers'].values():
        require(re.fullmatch('[a-f0-9]{64}', entry['sha256']), 'Invalid installer helper hash')
    require(re.fullmatch('[a-f0-9]{64}', spec['android']['runtime_sha256']), 'Invalid Android runtime hash')
    for item in spec['materials'].values():
        origin = item['origin']
        if origin['kind'] == 'tar_slice':
            require(origin['archive'] == ffmpeg['archive'] and origin['archive_sha256'] == ffmpeg['sha256']
                    and origin['member'].startswith(ffmpeg['directory'] + '/'), 'Native notice archive pin differs')
        if origin['kind'] == 'zip_slice':
            require(origin['archive_sha256'] == spec['windows']['nsis_archive_sha256'], 'Installer notice archive pin differs')


def rendered(root, spec, upstream=None):
    validate_bindings(root, spec)
    raw = materials(root, spec, upstream)
    texts, components, referenced, identifiers = {}, [], set(), set()
    for component in spec['components']:
        require(component['id'] not in identifiers and component['materials'], 'Duplicate or empty component notice')
        identifiers.add(component['id'])
        refs = []
        for key in component['materials']:
            require(key in raw, 'Missing component notice member')
            referenced.add(key)
            item = spec['materials'][key]
            texts[item['sha256']] = raw[key].decode('utf-8')
            refs.append({'path': item['path'], 'sha256': item['sha256']})
        components.append({**component, 'materials': refs})
    require(referenced == set(raw), 'Unreferenced component notice member')
    payload = core.render_notices({'components': components, 'texts': texts})
    payload = payload.replace('# Dependency licenses and notices', '# Native and installer component notices', 1)
    acknowledgements = spec.get('acknowledgements', [])
    require(all(isinstance(text, str) and text.strip() for text in acknowledgements), 'Invalid component acknowledgement')
    if acknowledgements:
        payload += '\n## Acknowledgements\n\n' + '\n\n'.join(acknowledgements) + '\n'
    return payload


def attached_common(source, payload):
    begin, end = '<!-- BEGIN GENERATED NATIVE NOTICES -->', '<!-- END GENERATED NATIVE NOTICES -->'
    require(source.count(begin) == source.count(end) and source.count(begin) <= 1, 'Ambiguous native notice markers')
    section = begin + '\n\n' + payload.rstrip('\n') + '\n\n' + end
    if begin not in source:
        return source.rstrip('\n') + '\n\n' + section + '\n'
    prefix, remaining = source.split(begin, 1)
    old, suffix = remaining.split(end, 1)
    require(end not in prefix and begin not in old, 'Invalid native notice marker order')
    return prefix + section + suffix


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--generate', action='store_true')
    mode.add_argument('--check', action='store_true')
    parser.add_argument('--upstream', type=Path, help='Replay pinned FFmpeg and NSIS source archives in this directory')
    parser.add_argument('--android-package', type=Path)
    parser.add_argument('--ndk', type=Path)
    parser.add_argument('--windows-package', type=Path)
    parser.add_argument('--built-platform', choices=['android', 'windows'])
    args = parser.parse_args()
    root = args.root.resolve()
    try:
        spec_raw = (root / MATERIAL).read_bytes()
        spec = core.unique_json(spec_raw)
        payload = rendered(root, spec, args.upstream)
        if args.built_platform:
            require(not args.android_package and not args.windows_package, 'Choose built-platform or explicit package paths')
            if args.built_platform == 'android':
                args.android_package = built_package(root, 'android')
            else:
                args.windows_package = built_package(root, 'windows')
        if args.android_package:
            require(args.ndk is not None, 'Android native notice verification requires --ndk')
            verify_android(root, spec, args.android_package, args.ndk)
        if args.windows_package:
            verify_windows(spec, args.windows_package.resolve())
        source_raw = (root / 'THIRD_PARTY_NOTICES.md').read_bytes()
        source = source_raw.decode('utf-8').replace('\r\n', '\n').replace('\r', '\n')
        common = attached_common(source, payload)
        outputs = {PAYLOAD: payload.encode('utf-8'), 'THIRD_PARTY_NOTICES.md': common.encode('utf-8'),
                   'THIRD_PARTY_NOTICES.rtf': core.rtf_bytes(common)}
        require((root / MATERIAL).read_bytes() == spec_raw, 'Component notice metadata changed during validation')
        if args.check:
            for name, content in outputs.items():
                require((root / name).read_bytes() == content, 'Stale or detached component notice output: ' + name)
        else:
            core.publish_outputs(root, outputs, {'THIRD_PARTY_NOTICES.md': source_raw})
        print(f"Verified {len(spec['components'])} native/installer components and {len(spec['materials'])} material members")
    except (OSError, ValueError, KeyError, TypeError, tarfile.TarError, zipfile.BadZipFile) as error:
        parser.exit(1, f'Component notices: {error}\n')


if __name__ == '__main__':
    main()
