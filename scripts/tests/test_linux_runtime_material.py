"""Offline runtime material integrity tests; no downloaded code is executed."""
import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / 'linux-runtime-material.py'
SPEC = 'scripts/notices/linux-runtime-material.json'
PREFIX = 'src/licenses/linux-runtime/'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def archive(members):
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode='w:gz') as target:
        for name, raw in members:
            member = tarfile.TarInfo(name)
            if raw is None:
                member.type = tarfile.SYMTYPE
                member.linkname = '../outside'
                target.addfile(member)
            else:
                member.size = len(raw)
                target.addfile(member, io.BytesIO(raw))
    return output.getvalue()


class LinuxRuntimeMaterial(unittest.TestCase):
    def module(self):
        self.assertTrue(SCRIPT.is_file(), 'Missing Linux runtime material helper')
        spec = importlib.util.spec_from_file_location('linux_runtime_material', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def fixture(self):
        notice = b'Copyright Supplier\nPermission to copy this complete notice.\n'
        dependency = archive([('dep-src/LICENSE', notice), ('dep-src/other', b'Other text')])
        recipe = (f'echo "{digest(dependency)}  dependency.tar.gz" | sha256sum -c -\n').encode()
        runtime_notice = b'Runtime MIT notice\n'
        runtime = archive([('runtime-src/LICENSE', runtime_notice),
                           ('runtime-src/scripts/dependencies.sh', recipe)])
        raw = {PREFIX+'notices/dep/LICENSE': notice,
               PREFIX+'notices/runtime/LICENSE': runtime_notice,
               PREFIX+'recipes/runtime/scripts/dependencies.sh': recipe,
               PREFIX+'evidence/producer.log': b'Supplier selected dependency 1.0\n'}
        archives = []
        for name, data, directory, selected_notice in [
                ('dep', dependency, 'dep-src', notice),
                ('runtime', runtime, 'runtime-src', runtime_notice)]:
            archives.append({'id': name, 'version': '1.0', 'archive_filename': name+'.tar.gz',
                             'url': 'https://example.org/'+name+'.tar.gz',
                             'sha256': digest(data), 'byte_length': len(data),
                             'archive_directory': directory,
                             'notice_members': [{'member': directory+'/LICENSE',
                                                 'source_file': PREFIX+'notices/'+name+'/LICENSE',
                                                 'sha256': digest(selected_notice),
                                                 'byte_length': len(selected_notice)}]})
        archives[0]['upstream_integrity'] = {
            'algorithm': 'sha256', 'expected': digest(dependency), 'observed': digest(dependency),
            'recipe_source_file': PREFIX+'recipes/runtime/scripts/dependencies.sh',
            'upstream_download_filename': 'dependency.tar.gz', 'verified': True}
        spec = {'schema_version': 1, 'archives': archives,
                'recipes': {'runtime': {'archive_id': 'runtime', 'files': [
                    {'path': 'scripts/dependencies.sh',
                     'source_file': PREFIX+'recipes/runtime/scripts/dependencies.sh',
                     'sha256': digest(recipe),
                     'archive_member': 'runtime-src/scripts/dependencies.sh'}]}},
                'evidence_files': [{'source_file': PREFIX+'evidence/producer.log',
                                    'sha256': digest(raw[PREFIX+'evidence/producer.log'])}]}
        payloads = {'linux-runtime/archives/dep.tar.gz': dependency,
                    'linux-runtime/archives/runtime.tar.gz': runtime,
                    **{'app/'+name: data for name, data in raw.items()}}
        return spec, payloads

    def write_fixture(self, root, spec, files):
        path = root/SPEC
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(spec))
        for name, raw in files.items():
            if name.startswith('app/'):
                path = root/name.removeprefix('app/')
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(raw)

    def test_verification_is_offline_and_checks_complete_supplied_material(self):
        m = self.module();spec, files = self.fixture()
        with patch.object(Path, 'read_bytes', side_effect=AssertionError('Filesystem access')):
            with patch.object(m.urllib.request, 'urlopen', side_effect=AssertionError('Network access')):
                self.assertEqual(m.verify_files(files, spec), {
                    'linux-runtime/archives/dep.tar.gz', 'linux-runtime/archives/runtime.tar.gz'})

    def test_missing_or_changed_archives_notices_recipes_and_evidence_are_refused(self):
        m = self.module();spec, files = self.fixture()
        for name in files:
            for mode in ['missing', 'changed']:
                with self.subTest(name=name, mode=mode):
                    changed = dict(files)
                    if mode == 'missing':changed.pop(name)
                    else:changed[name] += b'changed'
                    with self.assertRaises(ValueError):m.verify_files(changed, spec)
        with self.assertRaises(ValueError):
            m.verify_files({**files, 'linux-runtime/archives/unlisted.tar.gz': b'untracked'}, spec)

    def test_selected_notice_must_be_the_unique_regular_archive_member_with_matching_bytes(self):
        m = self.module();spec, files = self.fixture()
        selected = spec['archives'][0]['notice_members'][0]
        for member in ['dep-src/other', 'dep-src/missing']:
            altered = copy.deepcopy(spec);altered['archives'][0]['notice_members'][0]['member'] = member
            with self.subTest(member=member), self.assertRaises(ValueError):m.verify_files(files, altered)
        notice = files['app/'+selected['source_file']]
        for members in [[('dep-src/LICENSE', notice), ('dep-src/LICENSE', notice)],
                        [('dep-src/LICENSE', None)]]:
            altered = copy.deepcopy(spec);changed = dict(files)
            data = archive(members);changed['linux-runtime/archives/dep.tar.gz'] = data
            altered['archives'][0].update(sha256=digest(data), byte_length=len(data))
            altered['archives'][0].pop('upstream_integrity')
            with self.assertRaises(ValueError):m.verify_files(changed, altered)

    def test_recipe_cannot_be_detached_from_its_source_archive_by_repinning_raw_file(self):
        m = self.module();spec, files = self.fixture();changed = dict(files)
        recipe = spec['recipes']['runtime']['files'][0]
        changed['app/'+recipe['source_file']] = b'Changed recipe, with a freshly calculated raw-file hash'
        recipe['sha256'] = digest(changed['app/'+recipe['source_file']])
        with self.assertRaises(ValueError):m.verify_files(changed, spec)

    def test_upstream_checksum_binding_is_checked_even_when_all_raw_hashes_agree(self):
        m = self.module();spec, files = self.fixture()
        # Replacing the declared filename must fail even though the archive and
        # retained recipe hashes remain unchanged and independently correct.
        spec['archives'][0]['upstream_integrity']['upstream_download_filename'] = 'wrong.tar.gz'
        with self.assertRaises(ValueError):m.verify_files(files, spec)

    def test_apkbuild_sha512_source_and_local_patch_pins_are_verified(self):
        m = self.module();spec, files = self.fixture()
        data = files['linux-runtime/archives/dep.tar.gz'];local_patch = b'Exact downstream patch\n'
        source_hash = hashlib.sha512(data).hexdigest();patch_hash = hashlib.sha512(local_patch).hexdigest()
        recipe = f'sha512sums="\n{source_hash}  dep.tar.gz\n{patch_hash}  fix.patch\n"\n'.encode()
        source_file = PREFIX+'recipes/aports/dep/APKBUILD'
        patch_file = PREFIX+'recipes/aports/dep/fix.patch'
        spec['recipes']['aports-dep'] = {'files': [
            {'path': 'APKBUILD', 'source_file': source_file, 'sha256': digest(recipe)},
            {'path': 'fix.patch', 'source_file': patch_file, 'sha256': digest(local_patch),
             'apkbuild_sha512': patch_hash}]}
        files['app/'+source_file] = recipe;files['app/'+patch_file] = local_patch
        spec['archives'][0]['upstream_integrity'] = {
            'algorithm': 'sha512', 'expected': source_hash, 'observed': source_hash,
            'recipe_source_file': source_file, 'verified': True}
        m.verify_files(files, spec)
        spec['recipes']['aports-dep']['files'][1]['apkbuild_sha512'] = '0'*128
        with self.assertRaises(ValueError):m.verify_files(files, spec)
        spec['recipes']['aports-dep']['files'][1]['apkbuild_sha512'] = patch_hash
        files['app/'+source_file] = recipe.replace(source_hash.encode(), b'0'*128)
        spec['recipes']['aports-dep']['files'][0]['sha256'] = digest(files['app/'+source_file])
        with self.assertRaises(ValueError):m.verify_files(files, spec)

    def test_unsafe_paths_duplicate_selections_and_invalid_archive_bounds_are_refused(self):
        m = self.module();spec, files = self.fixture()
        for field, value in [('archive_filename', '../dep.tar.gz'),
                             ('archive_directory', '/outside'), ('byte_length', 10**12)]:
            altered = copy.deepcopy(spec);altered['archives'][0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):m.verify_files(files, altered)
        altered = copy.deepcopy(spec);altered['archives'].append(copy.deepcopy(altered['archives'][0]))
        with self.assertRaises(ValueError):m.verify_files(files, altered)
        altered = copy.deepcopy(spec)
        altered['archives'][0]['notice_members'][0]['source_file'] = '../outside'
        with self.assertRaises(ValueError):m.verify_files(files, altered)

    def test_explicit_archive_directory_is_exact_and_never_falls_back_to_network(self):
        m = self.module();spec, files = self.fixture()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder);self.write_fixture(root, spec, files);directory = root/'archives';directory.mkdir()
            for name, raw in files.items():
                if name.startswith('linux-runtime/archives/'):(directory/Path(name).name).write_bytes(raw)
            with patch.object(m.urllib.request, 'urlopen', side_effect=AssertionError('Network fallback')):
                selected = m.source_files(root, directory)
                self.assertEqual(set(selected), {'linux-runtime/archives/dep.tar.gz',
                                                 'linux-runtime/archives/runtime.tar.gz'})
                (directory/'dep.tar.gz').unlink()
                with self.assertRaises(ValueError):m.source_files(root, directory)
                (directory/'dep.tar.gz').write_bytes(files['linux-runtime/archives/dep.tar.gz']+b'changed')
                with self.assertRaises(ValueError):m.source_files(root, directory)
                (directory/'dep.tar.gz').write_bytes(files['linux-runtime/archives/dep.tar.gz'])
                (directory/'unexpected').write_bytes(b'extra')
                with self.assertRaises(ValueError):m.source_files(root, directory)

    def test_network_collection_requires_https_bounded_exact_hashes_and_https_redirects(self):
        m = self.module();spec, files = self.fixture()
        class Response(io.BytesIO):
            def geturl(self):return 'https://example.org/asset'
        def fetch(url, timeout):
            return Response(files['linux-runtime/archives/'+url.rsplit('/', 1)[1]])
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder);self.write_fixture(root, spec, files)
            with patch.object(m.urllib.request, 'urlopen', side_effect=fetch):
                self.assertEqual(len(m.source_files(root)), 2)
            with patch.object(m.urllib.request, 'urlopen', return_value=Response(b'wrong bytes')):
                with self.assertRaises(ValueError):m.source_files(root)
            class Oversized(Response):
                def read(self, size=-1):
                    if not 0 < size <= spec['archives'][0]['byte_length']+1:
                        raise AssertionError('Unbounded archive response read')
                    return b'x'*size
            with patch.object(m.urllib.request, 'urlopen', return_value=Oversized()):
                with self.assertRaises(ValueError):m.source_files(root)
            class Downgraded(Response):
                def geturl(self):return 'http://example.org/asset'
            with patch.object(m.urllib.request, 'urlopen', return_value=Downgraded(b'bytes')):
                with self.assertRaises(ValueError):m.source_files(root)
            spec['archives'][0]['url'] = 'http://example.org/dep.tar.gz'
            self.write_fixture(root, spec, files)
            with self.assertRaises(ValueError):m.source_files(root)

    def test_notice_rendering_preserves_complete_text_and_refuses_local_tampering(self):
        m = self.module();spec, files = self.fixture()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder);self.write_fixture(root, spec, files)
            text = m.notice_text(root)
            self.assertIn('Copyright Supplier\nPermission to copy this complete notice.\n', text)
            self.assertIn('Runtime MIT notice\n', text)
            (root/(PREFIX+'evidence/producer.log')).write_bytes(b'altered provenance')
            with self.assertRaises(ValueError):m.notice_text(root)

    def test_repository_notice_material_is_portable_and_renders_mimalloc_copyright(self):
        m = self.module();root = SCRIPT.parents[1]
        text = m.notice_text(root)
        self.assertIn('Copyright (c) 2018-2021 Microsoft Corporation, Daan Leijen', text)
        spec = json.loads((root/SPEC).read_text())
        self.assertNotIn('/tmp/', json.dumps(spec))
        for selection in spec['archives']:
            for notice in selection['notice_members']:
                self.assertIn((root/notice['source_file']).read_text(), text)


if __name__ == '__main__':unittest.main()
