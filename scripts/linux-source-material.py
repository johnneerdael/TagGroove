#!/usr/bin/env python3
"""Validate exact distribution source selections; archive contents are never executed."""
import hashlib
from pathlib import PurePosixPath
import re


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def relative(value):
    require(isinstance(value, str) and re.fullmatch(r'[A-Za-z0-9+_.~@/-]+', value),
            'Invalid distribution material path')
    path = PurePosixPath(value)
    require(not path.is_absolute() and '..' not in path.parts and str(path) == value
            and value not in ('.', ''), 'Unsafe distribution material path')
    return value


def paragraphs(raw):
    """Parse Deb822 without silently replacing repeated or malformed fields."""
    result, current, previous = [], {}, None
    for line in raw.decode('utf-8').replace('\r\n', '\n').split('\n') + ['']:
        if not line:
            if current:
                result.append(current)
            current, previous = {}, None
        elif line[0] in ' \t':
            require(previous is not None, 'Orphan distribution field continuation')
            current[previous] += '\n' + line[1:]
        else:
            require(':' in line, 'Malformed distribution metadata field')
            key, value = line.split(':', 1)
            key = key.lower()
            require(re.fullmatch(r'[a-z0-9][a-z0-9-]*', key) and key not in current,
                    'Duplicate or invalid distribution metadata field')
            current[key], previous = value.lstrip(' \t'), key
    return result


def package(value):
    require(isinstance(value, str) and re.fullmatch(r'[a-z0-9][a-z0-9+.-]*', value),
            'Invalid distribution package name')
    return value


def version(value):
    require(isinstance(value, str) and re.fullmatch(r'[0-9][A-Za-z0-9.+:~_-]*', value),
            'Invalid distribution package version')
    return value


def binary_source(record):
    fields = {}
    for key, value in record.items():
        require(isinstance(key, str) and key.lower() not in fields, 'Duplicate binary metadata field')
        fields[key.lower()] = value
    binary_name, binary_version = package(fields['package']), version(fields['version'])
    source = fields.get('source', binary_name)
    match = re.fullmatch(r'([a-z0-9][a-z0-9+.-]*)(?: \(([^()]+)\))?', source)
    require(match is not None, 'Invalid binary source-package declaration')
    return package(match[1]), version(match[2] or binary_version)


def checksum_rows(value):
    result = {}
    for line in value.strip().splitlines():
        fields = line.split()
        require(len(fields) == 3, 'Malformed distribution checksum record')
        digest, size, name = fields
        relative(name)
        require(re.fullmatch(r'[0-9a-f]{64}', digest) and size.isdecimal()
                and len(size) < 20 and int(size) < 2**63 and name not in result,
                'Invalid or duplicate distribution checksum record')
        result[name] = {'sha256': digest, 'size': int(size)}
    require(result, 'Empty distribution checksum set')
    return result


def select_source(index, name, selected_version):
    package(name);version(selected_version)
    matches = [record for record in paragraphs(index)
               if record.get('package') == name and record.get('version') == selected_version]
    require(len(matches) == 1, 'Expected one exact distribution source selection')
    record = matches[0]
    directory = relative(record['directory'])
    require(directory.startswith('pool/'), 'Distribution source is outside the package pool')
    files = checksum_rows(record['checksums-sha256'])
    require(all('/' not in name for name in files), 'Source member must be a basename')
    require(all(0 < item['size'] <= 512 * 1024 * 1024 for item in files.values()),
            'Distribution source material exceeds archive member bounds')
    require(sum(name.endswith('.dsc') for name in files) == 1
            and len(files) >= 2 and any('.tar.' in name or name.endswith('.tar') for name in files),
            'Source selection requires its description and complete archive material')
    return {'package': name, 'version': selected_version, 'directory': directory, 'files': files}


def signed_payload(raw):
    """Read .dsc fields whose complete bytes are already bound to the signed index.

    This does not authenticate the .dsc author's detached/cleartext signature.
    The caller must authenticate the repository index before trusting selection.
    """
    if not raw.startswith(b'-----BEGIN PGP SIGNED MESSAGE-----'):
        return raw
    text = raw.decode('utf-8').replace('\r\n', '\n')
    require(text.count('-----BEGIN PGP SIGNATURE-----') == 1 and '\n\n' in text,
            'Malformed clear-signed source description')
    body = text.split('\n\n', 1)[1].split('-----BEGIN PGP SIGNATURE-----', 1)[0]
    return '\n'.join(line[2:] if line.startswith('- ') else line for line in body.split('\n')).encode()


def verify_sources(selection, payloads):
    """Require every indexed file and the complete .dsc-declared archive/patch set."""
    package(selection['package']);version(selection['version']);relative(selection['directory'])
    expected = selection['files']
    require(set(payloads) == set(expected), 'Missing or unexpected distribution source material')
    for name, record in expected.items():
        relative(name)
        require('/' not in name and len(payloads[name]) == record['size']
                and sha(payloads[name]) == record['sha256'], 'Distribution source bytes differ: ' + name)
    descriptions = [name for name in expected if name.endswith('.dsc')]
    require(len(descriptions) == 1, 'Expected one source description')
    records = paragraphs(signed_payload(payloads[descriptions[0]]))
    require(len(records) == 1, 'Expected one source description paragraph')
    record = records[0]
    require(record.get('source') == selection['package'] and record.get('version') == selection['version'],
            'Source description identity differs from repository selection')
    declared = checksum_rows(record['checksums-sha256'])
    require(declared == {name: item for name, item in expected.items() if name != descriptions[0]},
            'Source description archive/patch set differs from repository selection')
    return expected


def verify_index(verified_release, path, raw):
    """Bind an index to a Release payload authenticated separately using GnuPG."""
    relative(path)
    records = paragraphs(verified_release)
    require(len(records) == 1, 'Expected one authenticated Release paragraph')
    expected = checksum_rows(records[0]['sha256'])
    require(path in expected and expected[path] == {'sha256': sha(raw), 'size': len(raw)},
            'Distribution index differs from authenticated Release')
