#!/usr/bin/env python3
"""Retain pinned runtime sources and notices without executing supplier code.

The caller must authenticate the specification and retained app files separately.
Archive hashes and source bindings do not establish historical producer APK byte
identity, signature validity, a reproducible build, or a license conclusion.
"""
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import re
import tarfile
import urllib.parse
import urllib.request


SPEC_FILE = 'scripts/notices/linux-runtime-material.json'
SOURCE_PREFIX = 'src/licenses/linux-runtime/'
ARCHIVE_PREFIX = 'linux-runtime/archives/'
MAX_ARCHIVE_BYTES = 64 * 1024 * 1024
MAX_RAW_BYTES = 8 * 1024 * 1024
MAX_UNPACKED_BYTES = 512 * 1024 * 1024


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def relative(value):
    require(isinstance(value, str) and re.fullmatch(r'[A-Za-z0-9+_.~@/-]+', value),
            'Invalid runtime material path')
    path = PurePosixPath(value)
    require(not path.is_absolute() and '..' not in path.parts and str(path) == value
            and value not in ('', '.'), 'Unsafe runtime material path')
    return value


def https(url):
    require(isinstance(url, str), 'Invalid runtime source URL')
    parsed = urllib.parse.urlsplit(url)
    require(parsed.scheme == 'https' and parsed.hostname and not parsed.username
            and not parsed.password, 'Runtime source downloads require HTTPS')
    return url


def checksum(value, length=64):
    require(isinstance(value, str) and re.fullmatch('[0-9a-f]{'+str(length)+'}', value),
            'Invalid runtime material checksum')
    return value


def catalog(spec):
    """Validate portable paths and build the complete expected material set."""
    require(spec.get('schema_version') == 1 and spec.get('archives'),
            'Unsupported or empty runtime source specification')
    archives, raw_files, selected = {}, {}, {}
    ids = {}

    def retained(record):
        source = relative(record['source_file'])
        require(source.startswith(SOURCE_PREFIX), 'Runtime raw file is outside its retained directory')
        checksum(record['sha256'])
        if 'byte_length' in record:
            require(type(record['byte_length']) is int and 0 < record['byte_length'] <= MAX_RAW_BYTES,
                    'Runtime raw file exceeds bounds')
        require(source not in raw_files, 'Duplicate retained runtime file: '+source)
        raw_files[source] = record

    for record in spec['archives']:
        name = relative(record['archive_filename'])
        require('/' not in name and name not in archives and record['id'] not in ids,
                'Duplicate or invalid runtime archive selection')
        require(type(record['byte_length']) is int and 0 < record['byte_length'] <= MAX_ARCHIVE_BYTES,
                'Runtime source archive exceeds bounds')
        relative(record['archive_directory']);checksum(record['sha256']);https(record['url'])
        archives[name], ids[record['id']] = record, name
        selected[name] = {}
        require(record.get('notice_members'), 'Runtime archive has no selected notices')
        for notice in record['notice_members']:
            member = relative(notice['member'])
            require(member.startswith(record['archive_directory']+'/') and member not in selected[name],
                    'Duplicate or misplaced runtime notice member')
            retained(notice)
            selected[name][member] = [notice['source_file']]

    for group in spec.get('recipes', {}).values():
        for record in group['files']:
            relative(record['path']);retained(record)
            if 'apkbuild_sha512' in record:checksum(record['apkbuild_sha512'], 128)
            if 'archive_member' in record:
                require(group.get('archive_id') in ids, 'Recipe source archive is missing')
                name = ids[group['archive_id']]
                expected = archives[name]['archive_directory']+'/'+record['path']
                require(relative(record['archive_member']) == expected, 'Recipe archive member differs')
                selected[name].setdefault(expected, []).append(record['source_file'])
    for record in spec.get('evidence_files', []):retained(record)
    return archives, raw_files, selected


def verify_raw(files, records):
    for source, record in records.items():
        path = 'app/'+source
        require(path in files and isinstance(files[path], bytes), 'Missing retained runtime file: '+source)
        raw = files[path]
        require(0 < len(raw) <= MAX_RAW_BYTES and sha(raw) == record['sha256']
                and ('byte_length' not in record or len(raw) == record['byte_length']),
                'Retained runtime file bytes differ: '+source)


def checksum_rows(raw, algorithm):
    length = 128 if algorithm == 'sha512' else 64
    rows = re.findall(r'([0-9a-f]{'+str(length)+r'})  ([A-Za-z0-9+_.~-]+)', raw.decode('utf-8'))
    result = {}
    for digest, name in rows:
        require(name not in result, 'Duplicate upstream runtime checksum filename')
        result[name] = digest
    return result


def verify_upstream(files, spec, archives, records, payloads=None):
    """Check retained dependency recipe checksum bindings, without running them."""
    for name, archive in archives.items():
        binding = archive.get('upstream_integrity', {})
        if 'recipe_source_file' not in binding:continue
        source = relative(binding['recipe_source_file'])
        require(source in records, 'Missing retained runtime checksum recipe')
        algorithm = binding.get('algorithm')
        require(algorithm in {'sha256', 'sha512'} and binding.get('verified') is True,
                'Invalid upstream runtime checksum binding')
        expected = checksum(binding['expected'], 128 if algorithm == 'sha512' else 64)
        require(binding.get('observed') == expected, 'Upstream runtime checksum observation differs')
        filename = relative(binding.get('upstream_download_filename', name))
        require('/' not in filename, 'Invalid upstream runtime checksum filename')
        require(checksum_rows(files['app/'+source], algorithm).get(filename) == expected,
                'Runtime source checksum differs from retained recipe: '+name)
        if algorithm == 'sha256':
            require(expected == archive['sha256'], 'Runtime archive pin differs from upstream checksum')
        if payloads is not None:
            observed = hashlib.new(algorithm, payloads[ARCHIVE_PREFIX+name]).hexdigest()
            require(observed == expected, 'Runtime archive differs from upstream checksum: '+name)
    for group in spec.get('recipes', {}).values():
        patches = [record for record in group['files'] if 'apkbuild_sha512' in record]
        if not patches:continue
        recipes = [record for record in group['files'] if record['path'] == 'APKBUILD']
        require(len(recipes) == 1, 'Missing unique APKBUILD for retained runtime patches')
        rows = checksum_rows(files['app/'+recipes[0]['source_file']], 'sha512')
        for record in patches:
            require(rows.get(record['path']) == record['apkbuild_sha512']
                    and hashlib.sha512(files['app/'+record['source_file']]).hexdigest()
                    == record['apkbuild_sha512'], 'Retained runtime patch differs from APKBUILD')


def verify_members(raw, archive, members, files):
    """Read selected regular members in place; never extract archive paths."""
    seen, expanded = set(), 0
    try:
        with tarfile.open(fileobj=io.BytesIO(raw), mode='r|*') as source:
            for member in source:
                expanded += member.size
                require(expanded <= MAX_UNPACKED_BYTES, 'Runtime source archive expansion exceeds bounds')
                if member.name not in members:continue
                require(member.name not in seen and member.isfile(),
                        'Runtime notice/recipe member is duplicated or not a regular file')
                require(0 < member.size <= MAX_RAW_BYTES, 'Runtime notice/recipe member exceeds bounds')
                stream = source.extractfile(member)
                require(stream is not None, 'Runtime notice/recipe member has no content')
                content = stream.read(MAX_RAW_BYTES+1)
                require(len(content) == member.size and all(
                    content == files['app/'+path] for path in members[member.name]),
                    'Runtime notice/recipe bytes differ from source archive: '+member.name)
                seen.add(member.name)
        require(seen == set(members), 'Runtime source archive is missing selected notice/recipe members')
    except (tarfile.TarError, EOFError, OSError) as error:
        raise ValueError('Invalid runtime source archive: '+archive['archive_filename']) from error


def verify_files(files: dict[str, bytes], spec: dict) -> set[str]:
    """Verify passed material only: no filesystem, network, or code execution."""
    archives, records, members = catalog(spec)
    expected = {ARCHIVE_PREFIX+name for name in archives}
    require({path for path in files if path.startswith(ARCHIVE_PREFIX)} == expected,
            'Missing or unexpected runtime source archive material')
    verify_raw(files, records)
    for name, record in archives.items():
        raw = files[ARCHIVE_PREFIX+name]
        require(isinstance(raw, bytes) and len(raw) == record['byte_length'] and sha(raw) == record['sha256'],
                'Runtime source archive bytes differ: '+name)
    verify_upstream(files, spec, archives, records, files)
    for name, record in archives.items():verify_members(files[ARCHIVE_PREFIX+name], record, members[name], files)
    return expected


def read_local(root, name, limit):
    relative(name)
    root = root.resolve()
    path = root/name
    require(path.resolve().is_relative_to(root) and path.is_file()
            and not any(parent.is_symlink() for parent in [path, *path.parents] if parent != root),
            'Missing or unsafe retained runtime file: '+name)
    with path.open('rb') as source:raw = source.read(limit+1)
    require(len(raw) <= limit, 'Retained runtime file exceeds bounds: '+name)
    return raw


def local_material(root):
    spec = json.loads(read_local(root, SPEC_FILE, 1024 * 1024))
    archives, records, _ = catalog(spec)
    files = {'app/'+name: read_local(root, name, MAX_RAW_BYTES) for name in records}
    verify_raw(files, records)
    verify_upstream(files, spec, archives, records)
    return spec, files


def source_files(root: Path, archive_directory: Path | None = None) -> dict[str, bytes]:
    """Collect only pinned archives; an explicit local directory never downloads."""
    spec, files = local_material(root)
    names = {record['archive_filename'] for record in spec['archives']}
    if archive_directory is not None:
        require(archive_directory.is_dir() and {path.name for path in archive_directory.iterdir()} == names,
                'Explicit runtime archive directory must contain the exact pinned files')
    payloads = {}
    for record in spec['archives']:
        name, size = record['archive_filename'], record['byte_length']
        if archive_directory is not None:
            raw = read_local(archive_directory, name, size)
        else:
            with urllib.request.urlopen(https(record['url']), timeout=30) as response:
                https(response.geturl())
                raw = response.read(size+1)
        require(len(raw) == size and sha(raw) == record['sha256'], 'Runtime source archive bytes differ: '+name)
        payloads[ARCHIVE_PREFIX+name] = raw
    verify_files({**files, **payloads}, spec)
    return payloads


def notice_text(root: Path) -> str:
    """Render the pinned complete notice members, validated from retained bytes."""
    spec, files = local_material(root)
    sections = ['## AppImage runtime and launcher notices\n',
                'The following complete supplier notice members are bound to the retained source archives.\n']
    for archive in spec['archives']:
        sections.append(f"### {archive['id']} — {archive['version']}\n")
        sections.append(f"Source: {archive['url']}\n\nSHA256: `{archive['sha256']}`\n")
        for notice in archive['notice_members']:
            text = files['app/'+notice['source_file']].decode('utf-8')
            fence = '`'*max(3, 1+max((len(match) for match in re.findall(r'`+', text)), default=0))
            sections.append(f"Member: `{notice['member']}`\n\n{fence}text\n{text}"
                            + ('' if text.endswith('\n') else '\n')+f'{fence}\n')
    return '\n'.join(sections)
