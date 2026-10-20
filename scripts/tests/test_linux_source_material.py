"""Source-package integrity controls; no network, app builds or library execution."""
import copy
import hashlib
import importlib.util
import lzma
import json
from pathlib import Path
import shutil
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / 'linux-source-material.py'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


class LinuxSourceMaterial(unittest.TestCase):
    def module(self):
        self.assertTrue(SCRIPT.is_file(), 'Missing Linux source-material verifier')
        spec = importlib.util.spec_from_file_location('linux_source_material', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def fixture(self, version='1.2-3ubuntu1'):
        files = {'example_1.2.orig.tar.xz': b'Original supplier source bytes',
                 'example_1.2-3ubuntu1.debian.tar.xz': b'Distribution patches and packaging'}
        rows = '\n'.join(f' {digest(raw)} {len(raw)} {name}' for name, raw in files.items())
        dsc = f'Format: 3.0 (quilt)\nSource: example\nVersion: {version}\nChecksums-Sha256:\n{rows}\n'.encode()
        files['example_1.2-3ubuntu1.dsc'] = dsc
        rows = '\n'.join(f' {digest(raw)} {len(raw)} {name}' for name, raw in files.items())
        index = f'Package: example\nVersion: {version}\nDirectory: pool/main/e/example\nChecksums-Sha256:\n{rows}\n\n'.encode()
        return index, files

    def test_binary_source_uses_explicit_source_version_and_native_fallback(self):
        m = self.module()
        explicit = {'Package': 'libexample1', 'Version': '1.2-3ubuntu1+b1',
                    'Source': 'example (1.2-3ubuntu1)', 'Architecture': 'amd64'}
        self.assertEqual(m.binary_source(explicit), ('example', '1.2-3ubuntu1'))
        self.assertEqual(m.binary_source({'Package': 'example', 'Version': '1:1.2-3'}),
                         ('example', '1:1.2-3'))
        with self.assertRaises(ValueError):
            m.binary_source({**explicit, 'Source': 'example (bad version) extra'})

    def test_index_decoding_accepts_ubuntu_concatenated_xz_streams_and_legal_padding(self):
        m = self.module()
        self.assertTrue(callable(getattr(m, 'decompress_index', None)), 'Missing bounded multi-stream XZ reader')
        first, second = b'Package: first\n\n', b'Package: second\n\n'
        # Noble's signed indexes append this complete, empty XZ stream.
        empty = bytes.fromhex('fd377a585a000004e6d6b446000000001cdf44211fb6f37d010000000004595a')
        self.assertEqual(m.decompress_index(lzma.compress(first)+empty), first)
        self.assertEqual(m.decompress_index(lzma.compress(first)+b'\0'*4+lzma.compress(second)+b'\0'*8), first+second)

    def test_index_decoding_refuses_trailing_garbage_truncation_and_combined_output_overflow(self):
        m = self.module()
        self.assertTrue(callable(getattr(m, 'decompress_index', None)), 'Missing bounded multi-stream XZ reader')
        first, second = lzma.compress(b'first'), lzma.compress(b'second')
        for raw in [b'', first+b'garbage', first+b'\0', first+second[:-1], first[:-1],
                    lzma.compress(b'')*65]:
            with self.subTest(raw=raw[:20]), self.assertRaises(ValueError):m.decompress_index(raw)
        self.assertEqual(m.decompress_index(first+second, limit=11), b'firstsecond')
        with self.assertRaises(ValueError):m.decompress_index(first+second, limit=10)

    @unittest.skipUnless(shutil.which('gpgv'), 'GnuPG signature verifier required')
    def test_signature_authentication_checks_real_bytes_and_the_pinned_archive_signer(self):
        m = self.module()
        self.assertTrue(callable(getattr(m, 'authenticated_release', None)), 'Missing repository signature authentication')
        root = SCRIPT.parent/'tests/fixtures/linux-source'
        signed = lzma.decompress((root/'ubuntu-noble-updates-InRelease.xz').read_bytes())
        keys = (root/'ubuntu-archive-keyring.gpg').read_bytes()
        payload = m.authenticated_release(signed, keys)
        self.assertEqual(m.paragraphs(payload)[0]['suite'], 'noble-updates')
        with self.assertRaises(ValueError):
            m.authenticated_release(signed.replace(b'Suite: noble-updates', b'Suite: noble-altered'), keys)
        with self.assertRaises(ValueError):
            m.authenticated_release(signed, b'Not an OpenPGP keyring')
        # This fixture has a valid signature under its own supplied keyring,
        # but the signer is outside the fixed Ubuntu archive trust policy.
        with self.assertRaisesRegex(ValueError, 'signer'):
            m.authenticated_release((root/'untrusted-InRelease.asc').read_bytes(),
                                    (root/'untrusted-keyring.gpg').read_bytes())

    def test_exact_selection_and_dsc_require_all_source_archives_and_patches(self):
        m = self.module()
        index, files = self.fixture()
        older, _ = self.fixture('1.2-2')
        selected = m.select_source(older + index, 'example', '1.2-3ubuntu1')
        self.assertEqual(selected['version'], '1.2-3ubuntu1')
        self.assertEqual(len(m.verify_sources(selected, files)), 3)
        for missing in ['example_1.2.orig.tar.xz', 'example_1.2-3ubuntu1.debian.tar.xz']:
            with self.subTest(missing=missing), self.assertRaises(ValueError):
                m.verify_sources(selected, {key: raw for key, raw in files.items() if key != missing})
        changed = dict(files);changed['example_1.2.orig.tar.xz'] = b'changed source'
        with self.assertRaises(ValueError):m.verify_sources(selected, changed)
        with self.assertRaises(ValueError):m.verify_sources(selected, {**files, 'outside': b'untracked'})
        with self.assertRaises(ValueError):m.select_source(index + index, 'example', '1.2-3ubuntu1')
        with self.assertRaises(ValueError):m.select_source(index, 'example', '9.9')

    def test_binary_selection_binds_exact_architecture_bytes_and_source_version(self):
        m = self.module()
        self.assertTrue(callable(getattr(m, 'select_binary', None)), 'Missing exact binary package selection')
        record = (b'Package: libexample1\nVersion: 1.2-3+b1\nArchitecture: amd64\n'
                  b'Source: example (1.2-3)\nFilename: pool/main/e/example/libexample1_1.2-3+b1_amd64.deb\n'
                  b'Size: 3\nSHA256: ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n\n')
        arm = record.replace(b'amd64', b'arm64')
        selected = m.select_binary(arm + record, 'libexample1', '1.2-3+b1', 'amd64')
        self.assertEqual(selected['source_package'], 'example')
        self.assertEqual(selected['source_version'], '1.2-3')
        self.assertEqual(selected['size'], 3)
        self.assertEqual(selected['sha256'], 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad')
        with self.assertRaises(ValueError):m.select_binary(record, 'libexample1', '1.2-3+b1', 'arm64')
        with self.assertRaises(ValueError):m.select_binary(record, 'libexample1', '1.2-2', 'amd64')
        with self.assertRaises(ValueError):m.select_binary(record + record, 'libexample1', '1.2-3+b1', 'amd64')
        with self.assertRaises(ValueError):
            m.select_binary(record.replace(b'pool/main/', b'../../outside/'), 'libexample1', '1.2-3+b1', 'amd64')

    @unittest.skipUnless(shutil.which('gpgv'), 'GnuPG signature verifier required')
    def test_repository_collection_authenticates_selection_and_preserves_complete_source_material(self):
        m = self.module()
        self.assertTrue(callable(getattr(m, 'RepositorySnapshot', None)), 'Missing authenticated repository collector')
        root = SCRIPT.parent/'tests/fixtures/linux-source'
        origin = 'https://archive.ubuntu.com/ubuntu/'
        files = {
            'dists/noble-updates/InRelease': (root/'repository-InRelease.asc').read_bytes(),
            'dists/noble-updates/main/source/Sources.xz': (root/'repository-Sources.xz').read_bytes(),
            'dists/noble-updates/main/binary-amd64/Packages.xz': (root/'repository-Packages-amd64.xz').read_bytes(),
            'dists/noble-updates/universe/binary-amd64/Packages.xz': (root/'repository-Packages-amd64.xz').read_bytes(),
            'dists/noble-updates/universe/source/Sources.xz': (root/'repository-Sources-universe.xz').read_bytes(),
            'pool/main/e/example/libexample1_1.2-3+b1_amd64.deb': (root/'libexample1_1.2-3+b1_amd64.deb.fixture').read_bytes(),
        }
        for name in ['example_1.2.orig.tar.xz', 'example_1.2-3.debian.tar.xz', 'example_1.2-3.dsc']:
            files['pool/main/e/example/'+name] = (root/name).read_bytes()
        def hash_paths(values):
            return {name.rsplit('/', 1)[0]+'/by-hash/SHA256/'+digest(raw): name
                    for name, raw in values.items() if name.startswith('dists/') and name.endswith('.xz')}
        addresses = hash_paths(files)
        def fetch(url, limit):
            self.assertTrue(url.startswith(origin), 'Collector changed trusted archive origin')
            path = url.removeprefix(origin)
            if path.startswith('dists/') and not path.endswith('/InRelease'):
                self.assertIn('/by-hash/SHA256/', path, 'Mutable index URL can race signed Release metadata')
                path = addresses[path]
            return files[path]
        signer = json.loads((root/'repository-fixture.json').read_text())['signer']
        keys = (root/'repository-keyring.gpg').read_bytes()
        # Exercise real GnuPG with a fixture-only trust root; production's fixed
        # Ubuntu policy is separately covered by the real archive-signature test.
        with patch.object(m, 'UBUNTU_ARCHIVE_SIGNER', signer):
            snapshot = m.RepositorySnapshot('amd64', 'noble-updates', keys, fetch=fetch)
            selected = snapshot.collect('libexample1', '1.2-3+b1', 'main')
            self.assertEqual(selected['source']['version'], '1.2-3')
            self.assertEqual(len(selected['source']['files']), 3)
            self.assertEqual(snapshot.binary_bytes(selected), files[selected['binary']['filename']])
            self.assertTrue(any(name.endswith('example_1.2-3.debian.tar.xz') for name in snapshot.materials))
            cross_component = snapshot.collect('libexample1', '1.2-3+b1', 'universe')
            self.assertEqual(cross_component['component'], 'universe')
            self.assertEqual(cross_component['source_component'], 'main')
            ports = 'https://ports.ubuntu.com/ubuntu-ports/'
            arm_files = {**files,
                         'dists/noble-updates/main/binary-arm64/Packages.xz': (root/'repository-Packages-arm64.xz').read_bytes(),
                         'pool/main/e/example/libexample1_1.2-3+b1_arm64.deb': files['pool/main/e/example/libexample1_1.2-3+b1_amd64.deb']}
            arm_addresses = hash_paths(arm_files)
            def arm_fetch(url, limit):
                self.assertTrue(url.startswith(ports), 'Arm64 collector changed Ubuntu ports origin')
                path = url.removeprefix(ports)
                if path.startswith('dists/') and not path.endswith('/InRelease'):
                    self.assertIn('/by-hash/SHA256/', path)
                    path = arm_addresses[path]
                return arm_files[path]
            arm_snapshot = m.RepositorySnapshot('arm64', 'noble-updates', keys, fetch=arm_fetch)
            arm_selected = arm_snapshot.collect('libexample1', '1.2-3+b1', 'main')
            self.assertEqual(arm_selected['binary']['architecture'], 'arm64')
            self.assertEqual(arm_snapshot.binary_bytes(arm_selected), arm_files[arm_selected['binary']['filename']])
            with self.assertRaisesRegex(ValueError, 'suite/architecture'):
                m.RepositorySnapshot('amd64', 'noble', keys,
                                     fetch=lambda _url, _limit: files['dists/noble-updates/InRelease'])
            files['pool/main/e/example/example_1.2-3.debian.tar.xz'] = b'changed patches'
            fresh = m.RepositorySnapshot('amd64', 'noble-updates', keys, fetch=fetch)
            with self.assertRaises(ValueError):fresh.collect('libexample1', '1.2-3+b1', 'main')
            files['dists/noble-updates/main/binary-amd64/Packages.xz'] = b'changed index'
            fresh = m.RepositorySnapshot('amd64', 'noble-updates', keys, fetch=fetch)
            with self.assertRaises(ValueError):fresh.collect('libexample1', '1.2-3+b1', 'main')

    def test_dsc_identity_and_declared_file_set_cannot_be_detached_from_index(self):
        m = self.module()
        index, files = self.fixture();selected = m.select_source(index, 'example', '1.2-3ubuntu1')
        for old, new in [(b'Source: example', b'Source: other'),
                         (b'Version: 1.2-3ubuntu1', b'Version: 1.2-3ubuntu2')]:
            with self.subTest(new=new):
                changed = dict(files);changed['example_1.2-3ubuntu1.dsc'] = changed['example_1.2-3ubuntu1.dsc'].replace(old, new)
                selection = copy.deepcopy(selected)
                selection['files']['example_1.2-3ubuntu1.dsc'] = {'sha256': digest(changed['example_1.2-3ubuntu1.dsc']),
                                                                 'size': len(changed['example_1.2-3ubuntu1.dsc'])}
                with self.assertRaises(ValueError):m.verify_sources(selection, changed)
        # A mutually consistent partial index/payload still cannot drop a patch
        # that the retained .dsc requires.
        partial = copy.deepcopy(selected);partial['files'].pop('example_1.2-3ubuntu1.debian.tar.xz')
        with self.assertRaises(ValueError):
            m.verify_sources(partial, {key: raw for key, raw in files.items() if key in partial['files']})

    def test_authenticated_index_hash_is_checked_before_source_selection(self):
        m = self.module();index, _ = self.fixture()
        release = f'Suite: noble-updates\nSHA256:\n {digest(index)} {len(index)} main/source/Sources\n'.encode()
        m.verify_index(release, 'main/source/Sources', index)
        with self.assertRaises(ValueError):m.verify_index(release, 'main/source/Sources', index + b'changed')
        with self.assertRaises(ValueError):m.verify_index(release, '../Sources', index)
        with self.assertRaises(ValueError):
            m.select_source(index.replace(b'pool/main/e/example', b'../../outside'), 'example', '1.2-3ubuntu1')
        with self.assertRaises(ValueError):
            m.select_source(index.replace(b'Package: example', b'Package: example\nPackage: hidden'), 'example', '1.2-3ubuntu1')
        with self.assertRaises(ValueError):
            m.select_source(index.replace(b'Package: example', b'Package: example\npackage: hidden'), 'example', '1.2-3ubuntu1')

    def test_release_empty_unrelated_indexes_do_not_refuse_nonempty_selected_source(self):
        m = self.module();index, _ = self.fixture()
        release = (f'Suite: noble-updates\nSHA256:\n {digest(index)} {len(index)} main/source/Sources\n'
                   ' e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 0 restricted/source/Sources\n'
                   ' e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 0 main/dep11/icons-128x128@2.tar\n').encode()
        m.verify_index(release, 'main/source/Sources', index)

    def test_repository_checksum_covers_the_complete_clear_signed_source_description(self):
        m = self.module();index, files = self.fixture()
        selected = m.select_source(index, 'example', '1.2-3ubuntu1')
        name = 'example_1.2-3ubuntu1.dsc'
        # The repository checksum authenticates these complete bytes. This
        # fixture exercises cleartext framing, not author-signature validation.
        wrapped = (b'-----BEGIN PGP SIGNED MESSAGE-----\nHash: SHA512\n\n' + files[name]
                   + b'-----BEGIN PGP SIGNATURE-----\nfixture-only\n-----END PGP SIGNATURE-----\n')
        files[name] = wrapped
        selected['files'][name] = {'sha256': digest(wrapped), 'size': len(wrapped)}
        m.verify_sources(selected, files)
        with self.assertRaises(ValueError):
            m.verify_sources(selected, {**files, name: wrapped.replace(b'fixture-only', b'changed-only')})

    def test_mixed_case_deb822_fields_work_across_binary_source_description_and_release(self):
        m = self.module();index, files = self.fixture();dsc_name = 'example_1.2-3ubuntu1.dsc'
        old = files[dsc_name]
        files[dsc_name] = old.replace(b'Source:', b'sOuRcE:').replace(b'Version:', b'veRSIon:').replace(b'Checksums-Sha256:', b'CHECKSUMS-SHA256:')
        index = index.replace(digest(old).encode(), digest(files[dsc_name]).encode())
        index = index.replace(b'Package:', b'pAcKaGe:').replace(b'Version:', b'version:').replace(b'Directory:', b'directory:').replace(b'Checksums-Sha256:', b'checksums-sha256:')
        self.assertEqual(m.binary_source({'PACKAGE': 'libexample1', 'version': '1.2-3ubuntu1+b1',
                                          'source': 'example (1.2-3ubuntu1)'}), ('example', '1.2-3ubuntu1'))
        selected = m.select_source(index, 'example', '1.2-3ubuntu1')
        self.assertEqual(len(m.verify_sources(selected, files)), 3)
        release = f'suite: noble-updates\nsHa256:\n {digest(index)} {len(index)} main/source/Sources\n'.encode()
        m.verify_index(release, 'main/source/Sources', index)


if __name__ == '__main__':unittest.main()
