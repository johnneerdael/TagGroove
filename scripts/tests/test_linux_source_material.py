"""Source-package integrity controls; no network, app builds or library execution."""
import copy
import hashlib
import importlib.util
from pathlib import Path
import unittest


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
