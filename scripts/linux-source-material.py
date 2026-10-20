#!/usr/bin/env python3
"""Validate exact distribution source selections; archive contents are never executed."""
import hashlib
import lzma
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import tempfile
import urllib.parse
import urllib.request


UBUNTU_ARCHIVE_SIGNER = 'F6ECB3762474EDA9D21B7022871920D1991BC93C'


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


def iter_paragraphs(raw):
    """Parse Deb822 without silently replacing repeated or malformed fields."""
    current, previous = {}, None
    for line in raw.decode('utf-8').replace('\r\n', '\n').split('\n') + ['']:
        if not line:
            if current:
                yield current
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


def paragraphs(raw):
    return list(iter_paragraphs(raw))


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
    return source_record(matches[0])


def source_record(record):
    name, selected_version = package(record['package']), version(record['version'])
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


def select_binary(index, name, selected_version, architecture):
    package(name);version(selected_version)
    require(architecture in {'amd64', 'arm64'}, 'Unsupported release binary architecture')
    matches = [record for record in paragraphs(index)
               if record.get('package') == name and record.get('version') == selected_version
               and record.get('architecture') in {architecture, 'all'}]
    require(len(matches) == 1, 'Expected one exact distribution binary selection')
    return binary_record(matches[0])


def binary_record(record):
    name, selected_version = package(record['package']), version(record['version'])
    filename = relative(record['filename'])
    require(filename.startswith('pool/') and filename.endswith('.deb'), 'Invalid binary package pool path')
    checksum = checksum_rows(f"{record['sha256']} {record['size']} {filename}")[filename]
    require(0 < checksum['size'] <= 512 * 1024 * 1024, 'Binary package exceeds archive bounds')
    source_name, source_version = binary_source(record)
    return {'package': name, 'version': selected_version, 'architecture': record['architecture'],
            'filename': filename, **checksum, 'source_package': source_name, 'source_version': source_version}


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


def authenticated_release(raw, keyring):
    """Authenticate immutable repository metadata, without trusting supplied new keys.

    Historical source verification uses the signature and exact selected version;
    it does not treat an old Release as a fresh update/security recommendation.
    """
    require(0 < len(raw) <= 4 * 1024 * 1024 and 0 < len(keyring) <= 512 * 1024,
            'Repository signature input exceeds bounds')
    tool = shutil.which('gpgv')
    require(tool is not None, 'Repository signature verification requires gpgv')
    with tempfile.TemporaryDirectory(prefix='taggroove-source-signature-') as folder:
        root = Path(folder)
        (root/'keyring.gpg').write_bytes(keyring)
        (root/'InRelease').write_bytes(raw)
        checked = subprocess.run([tool, '--homedir', folder, '--keyring', str(root/'keyring.gpg'),
                                  '--status-fd', '1', str(root/'InRelease')],
                                 capture_output=True, text=True, timeout=60)
    require(checked.returncode == 0, 'Repository Release signature verification failed')
    signatures = [line.split()[2:] for line in checked.stdout.splitlines()
                  if line.startswith('[GNUPG:] VALIDSIG ')]
    require(len(signatures) == 1 and signatures[0]
            and (signatures[0][0] == UBUNTU_ARCHIVE_SIGNER or signatures[0][-1] == UBUNTU_ARCHIVE_SIGNER),
            'Repository signature is not from the pinned Ubuntu archive signer')
    require(raw.startswith(b'-----BEGIN PGP SIGNED MESSAGE-----'), 'Expected a clear-signed repository Release')
    return signed_payload(raw)


def download(url, limit):
    require(urllib.parse.urlsplit(url).scheme == 'https', 'Repository downloads require HTTPS')
    with urllib.request.urlopen(url, timeout=60) as response:
        final = urllib.parse.urlsplit(response.geturl())
        require(final.scheme == 'https' and final.hostname in {'archive.ubuntu.com', 'ports.ubuntu.com'},
                'Repository download redirected outside the approved archive')
        raw = response.read(limit + 1)
    require(len(raw) <= limit, 'Repository download exceeds bounds')
    return raw


class RepositorySnapshot:
    """Capture selected material through a fixed signature/index/package chain.

    Only source material and repository proofs enter `materials`; downloaded
    binary packages remain capture inputs for later file/transformation checks.
    """
    def __init__(self, architecture, suite, keyring, fetch=download):
        require(architecture in {'amd64', 'arm64'} and suite in {'noble', 'noble-updates', 'noble-security'},
                'Unsupported distribution source repository')
        self.architecture, self.suite, self.fetch = architecture, suite, fetch
        self.origin = ('https://archive.ubuntu.com/ubuntu/' if architecture == 'amd64'
                       else 'https://ports.ubuntu.com/ubuntu-ports/')
        self.materials, self._downloads, self._indexes = {}, {}, {}
        self.prefix = f'repositories/{architecture}/{suite}/'
        signed = self._read(f'dists/{suite}/InRelease', 4 * 1024 * 1024)
        self.release = authenticated_release(signed, keyring)
        records = paragraphs(self.release)
        require(len(records) == 1 and records[0].get('suite') == suite
                and records[0].get('codename') == 'noble'
                and architecture in records[0].get('architectures', '').split(),
                'Signed repository suite/architecture differs from selection')
        self.components = set(records[0].get('components', '').split())
        self.checksums = checksum_rows(records[0]['sha256'])
        self.materials[self.prefix+'InRelease'] = signed
        self.materials['repositories/ubuntu-keyring.gpg'] = keyring

    def _read(self, path, limit):
        relative(path)
        if path not in self._downloads:
            raw = self.fetch(self.origin+path, limit)
            require(isinstance(raw, bytes) and len(raw) <= limit, 'Repository response exceeds bounds')
            self._downloads[path] = raw
        raw = self._downloads[path]
        require(len(raw) <= limit, 'Cached repository response exceeds bounds')
        return raw

    def _verified_pool_file(self, path, record):
        require(relative(path).startswith('pool/'), 'Expected a distribution pool file')
        require(type(record['size']) is int and 0 < record['size'] <= 512 * 1024 * 1024,
                'Distribution pool file exceeds bounds')
        raw = self._read(path, record['size'])
        require(len(raw) == record['size'] and sha(raw) == record['sha256'],
                'Distribution pool file differs from signed metadata: '+path)
        return raw

    def _index(self, component, kind):
        require(component in self.components and kind in {'binary', 'source'}, 'Unsupported repository index')
        key = (component, kind)
        if key not in self._indexes:
            path = (component+'/source/Sources.xz' if kind == 'source'
                    else f'{component}/binary-{self.architecture}/Packages.xz')
            require(path in self.checksums and self.checksums[path]['size'] <= 512 * 1024 * 1024,
                    'Missing or oversized signed repository index')
            address = path.rsplit('/', 1)[0]+'/by-hash/SHA256/'+self.checksums[path]['sha256']
            raw = self._read(f'dists/{self.suite}/'+address, self.checksums[path]['size'])
            verify_index(self.release, path, raw)
            decoder = lzma.LZMADecompressor()
            decoded = decoder.decompress(raw, max_length=512 * 1024 * 1024)
            require(decoder.eof and not decoder.unused_data, 'Repository index decompression exceeds bounds or has trailing data')
            lookup = {}
            fields = ({'package', 'version', 'architecture', 'source', 'filename', 'size', 'sha256'}
                      if kind == 'binary' else {'package', 'version', 'directory', 'checksums-sha256'})
            for record in iter_paragraphs(decoded):
                record_key = (record.get('package'), record.get('version'))
                if kind == 'binary':record_key += (record.get('architecture'),)
                lookup.setdefault(record_key, []).append({name: value for name, value in record.items() if name in fields})
            self._indexes[key] = lookup
            self.materials[self.prefix+path] = raw
        return self._indexes[key]

    def _binary(self, name, selected_version, component):
        package(name);version(selected_version)
        index = self._index(component, 'binary')
        matches = index.get((name, selected_version, self.architecture), []) + index.get((name, selected_version, 'all'), [])
        require(len(matches) == 1, 'Expected one exact repository binary package')
        return binary_record(matches[0])

    def collect(self, name, selected_version, component):
        binary = self._binary(name, selected_version, component)
        matches = [(source_component, record) for source_component in sorted(self.components)
                   for record in self._index(source_component, 'source').get(
                       (binary['source_package'], binary['source_version']), [])]
        require(len(matches) == 1, 'Expected one exact repository source package')
        source_component, source = matches[0]
        selected = source_record(source)
        payloads = {name: self._verified_pool_file(selected['directory']+'/'+name, record)
                    for name, record in selected['files'].items()}
        verify_sources(selected, payloads)
        self.materials.update({'distribution/'+selected['directory']+'/'+name: raw for name, raw in payloads.items()})
        return {'suite': self.suite, 'architecture': self.architecture, 'component': component,
                'source_component': source_component,
                'binary': binary, 'source': selected}

    def binary_bytes(self, selection):
        require(selection.get('suite') == self.suite and selection.get('architecture') == self.architecture,
                'Binary evidence is bound to another repository selection')
        record = self._binary(selection['binary']['package'], selection['binary']['version'], selection['component'])
        require(record == selection['binary'], 'Binary evidence differs from authenticated repository selection')
        return self._verified_pool_file(record['filename'], record)
