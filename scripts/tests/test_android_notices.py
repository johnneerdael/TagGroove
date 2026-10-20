"""Focused Maven notice/provenance tests; no Gradle, network or app build."""
import hashlib
import importlib.util
import io
import json
import re
from pathlib import Path
import tempfile
import unittest
import zipfile
import warnings
import subprocess
import sys
import shutil
import tarfile

SCRIPT = Path(__file__).resolve().parents[1] / 'android-notices.py'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def archive(entries):
    stream = io.BytesIO()
    with zipfile.ZipFile(stream, 'w') as z:
        for name, raw in entries:
            z.writestr(name, raw)
    return stream.getvalue()


class AndroidNoticesTests(unittest.TestCase):
    def module(self):
        self.assertTrue(SCRIPT.is_file(), 'Missing source-bound Maven notice collector')
        spec = importlib.util.spec_from_file_location('android_notices', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_aar_and_nested_jar_material_is_exact_and_checksum_bound(self):
        m = self.module()
        nested = archive([('META-INF/NOTICE', b'Upstream notice\r\n'),
                          ('META-INF/FastDoubleParser-LICENSE', b'Parser license\n')])
        raw = archive([('classes.jar', nested), ('LICENSE.txt', 'Copyright ©\n'.encode()),
                       ('assets/data', b'not a notice')])
        result = m.zip_materials(raw, sha(raw))
        self.assertEqual({x['path'] for x in result}, {'LICENSE.txt', 'classes.jar!/META-INF/NOTICE',
                         'classes.jar!/META-INF/FastDoubleParser-LICENSE'})
        self.assertEqual(next(x['text'] for x in result if x['path'].endswith('/NOTICE')), 'Upstream notice\r\n')
        for item in result:
            self.assertEqual(sha(item['text'].encode()), item['sha256'])
        with self.assertRaisesRegex(ValueError, 'checksum'):
            m.zip_materials(raw, '0' * 64)

    def test_unsafe_duplicate_and_symlink_zip_members_refuse(self):
        m = self.module()
        for kind in ['traversal', 'duplicate', 'symlink']:
            with self.subTest(kind=kind):
                stream = io.BytesIO()
                with zipfile.ZipFile(stream, 'w') as z:
                    if kind == 'traversal':
                        z.writestr('../LICENSE', b'unsafe')
                    elif kind == 'duplicate':
                        z.writestr('LICENSE', b'one')
                        with warnings.catch_warnings():
                            warnings.simplefilter('ignore', UserWarning)
                            z.writestr('LICENSE', b'two')
                    else:
                        info = zipfile.ZipInfo('LICENSE')
                        info.create_system = 3
                        info.external_attr = 0o120777 << 16
                        z.writestr(info, b'/outside')
                raw = stream.getvalue()
                with self.assertRaises(ValueError):
                    m.zip_materials(raw, sha(raw))

    def test_pom_identity_and_inherited_license_parent_are_explicit(self):
        m = self.module()
        raw = b'''<project xmlns="http://maven.apache.org/POM/4.0.0"><modelVersion>4.0.0</modelVersion>
<parent><groupId>org.example</groupId><artifactId>parent</artifactId><version>2</version></parent>
<artifactId>library</artifactId><version>1</version></project>'''
        data = m.pom_record(raw, 'org.example:library:1')
        self.assertEqual(data['parent'], 'org.example:parent:2')
        self.assertEqual(data['licenses'], [])
        self.assertEqual(data['sha256'], sha(raw))
        with self.assertRaisesRegex(ValueError, 'identity'):
            m.pom_record(raw, 'org.example:other:1')
        with self.assertRaises(ValueError):
            m.pom_record(b'<!DOCTYPE foo [<!ENTITY x "text">]>' + raw, 'org.example:library:1')

    def test_resolution_refuses_stale_inputs_missing_edges_and_unknown_artifacts(self):
        m = self.module()
        inputs = {'build.gradle.kts': 'a' * 64}
        graph = {'schema': 1, 'configuration': 'arm64ReleaseRuntimeClasspath', 'inputs': inputs,
                 'components': [{'id': 'project :app', 'module': 'project:app:unspecified',
                                 'dependencies': [{'requested': 'org.example:library:1', 'selected': 'org.example:library:1', 'failure': None}]},
                                {'id': 'org.example:library:1', 'module': 'org.example:library:1', 'dependencies': []}],
                 'artifacts': [{'component': 'org.example:library:1', 'file': '/cache/library-1.jar', 'sha256': 'b' * 64}]}
        selected = m.validate_snapshot(graph, inputs)
        self.assertEqual(set(selected), {'org.example:library:1'})
        for change in ['stale', 'missing', 'unknown', 'failure']:
            with self.subTest(change=change):
                altered = json.loads(json.dumps(graph))
                if change == 'stale': altered['inputs'] = {}
                if change == 'missing': altered['components'].pop()
                if change == 'unknown': altered['artifacts'][0]['component'] = 'org.example:other:1'
                if change == 'failure': altered['components'][0]['dependencies'][0]['failure'] = 'unresolved'
                with self.assertRaises(ValueError):
                    m.validate_snapshot(altered, inputs)

    def fixture(self, root, m):
        files = {'Cargo.toml': '[workspace]\nmembers=["src-tauri"]\n',
                 'Cargo.lock': 'version=4\n[[package]]\nname="opentag"\nversion="0.2.0"\n',
                 'src-tauri/Cargo.toml': '[package]\nname="opentag"\nversion="0.2.0"\n',
                 'src-tauri/src/lib.rs': '.plugin(tauri_plugin_dialog::init())\n.plugin(tauri_plugin_document_access::init())\n',
                 'package.json': '{"name":"taggroove","version":"0.2.0"}',
                 'package-lock.json': '{"version":"0.2.0","packages":{"":{"version":"0.2.0"}}}',
                 'scripts/android-notices.py': '# collector fixture\n',
                 m.REPORT: '// report fixture\n',
                 'crates/document-access/android/build.gradle.kts': '// module fixture\n'}
        for name in ['app/build.gradle.kts','build.gradle.kts','settings.gradle','gradle.properties',
                     'gradle/wrapper/gradle-wrapper.properties']:
            files[m.ANDROID + '/' + name] = '// fixture\n'
        for name, text in files.items():
            p = root / name
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(text)
        canonical = b'Apache 2.0 full text fixture\r\n'
        source = 'src/licenses/android/apache-2.0.txt'
        (root / source).parent.mkdir(parents=True)
        (root / source).write_bytes(canonical)
        raw = archive([('META-INF/LICENSE', canonical), ('META-INF/NOTICE', b'Exact supplier notice\n')])
        cache = root / 'cache'
        for filename, content in [('library-1.jar', raw), ('library-1.pom', b'''<project xmlns="http://maven.apache.org/POM/4.0.0"><groupId>org.example</groupId><artifactId>library</artifactId><version>1</version><licenses><license><name>Apache 2.0</name></license></licenses></project>''')]:
            p = cache / 'org.example/library/1' / hashlib.sha1(content).hexdigest().lstrip('0') / filename
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_bytes(content)
        material = {'apache_license_names': ['Apache 2.0'], 'canonical_license': {'source_file': source,
                    'sha256': sha(canonical), 'reference_component': 'org.example:library:1',
                    'archive_sha256': sha(raw), 'archive_member': 'META-INF/LICENSE'}}
        (root / m.MATERIAL).write_bytes(m.encoded(material))
        graph = {'schema': 1, 'configuration': m.CONFIGURATION, 'inputs': m.graph_inputs(root),
                 'components': [{'id': 'project :app', 'module': 'project:app:unspecified',
                                 'dependencies': [{'requested': 'org.example:library:1', 'selected': 'org.example:library:1', 'failure': None}]},
                                {'id': 'org.example:library:1', 'module': 'org.example:library:1', 'dependencies': []}],
                 'artifacts': [{'component': 'org.example:library:1', 'file': '/cache/library-1.jar', 'sha256': sha(raw)}]}
        return graph, cache

    def test_collection_check_and_replay_bind_components_poms_notices_and_current_inputs(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            graph, cache = self.fixture(root, m)
            data = m.collect(root, graph, cache)
            m.validate_inventory(root, data)
            self.assertEqual(len(data['components']), 1)
            self.assertEqual(len(data['texts']), 2)
            self.assertEqual(m.collect(root, data['resolution'], cache), data)
            for change in ['missing', 'text', 'source', 'pom']:
                altered = json.loads(json.dumps(data))
                if change == 'missing': altered['components'] = []
                if change == 'text': altered['texts'][next(iter(altered['texts']))] += 'forged'
                if change == 'source': altered['inputs'] = {}
                if change == 'pom': altered['components'][0]['pom_chain'][0]['coordinate'] = 'org.example:other:1'
                with self.subTest(change=change), self.assertRaises(ValueError):
                    m.validate_inventory(root, altered)
            p = next(cache.glob('org.example/library/1/*/*.jar'))
            p.write_bytes(b'changed')
            with self.assertRaises(ValueError):
                m.collect(root, graph, cache)

    def test_committed_inventory_survives_release_stamp_but_refuses_changed_sources(self):
        m = self.module()
        repository = SCRIPT.parent.parent
        data = json.loads((repository / m.INVENTORY).read_text())
        canonical = json.loads((repository / m.MATERIAL).read_text())['canonical_license']['source_file']
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            for name in {*m.source_inputs(repository), 'src-tauri/tauri.conf.json', canonical}:
                if name == 'android-plugin-set':
                    continue
                target = root / name
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(repository / name, target)
            # Plugin registration is checked separately from the build scripts.
            target = root / 'src-tauri/src/lib.rs'
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(repository / 'src-tauri/src/lib.rs', target)
            before = m.source_inputs(root)
            command = ('import {stampVersion} from ' + json.dumps((repository / 'scripts/release.mjs').as_uri()) +
                       '; await stampVersion(process.argv[1], "0.2.60");')
            subprocess.run(['node', '--input-type=module', '-e', command, str(root)], check=True,
                           capture_output=True, text=True)
            self.assertEqual(m.source_inputs(root), before)
            m.validate_inventory(root, data)
            def changed_recipe(raw):
                recipe = json.loads(raw)
                recipe['components']['zlib']['url'] = 'https://example.invalid/changed-zlib.tar.gz'
                return m.encoded(recipe)

            changes = {
                'scripts/audio-deps/sources.json': changed_recipe,
                'Cargo.lock': lambda raw: re.sub(rb'(checksum = ")([a-f0-9])',
                                                lambda match: match[1] + (b'0' if match[2] != b'0' else b'1'), raw, count=1),
                'crates/document-access/android/build.gradle.kts': lambda raw: raw + b'\nimplementation("unexpected:dependency:1")\n',
                'scripts/android-notices.py': lambda raw: raw + b'\n# changed collector\n',
            }
            for name, change in changes.items():
                with self.subTest(source=name):
                    path = root / name
                    original = path.read_bytes()
                    altered = change(original)
                    self.assertNotEqual(altered, original)
                    path.write_bytes(altered)
                    with self.assertRaisesRegex(ValueError, 'Stale Android notice source inputs'):
                        m.validate_inventory(root, data)
                    path.write_bytes(original)

    def test_cli_attaches_current_payload_and_refuses_detached_outputs(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            graph, cache = self.fixture(root, m)
            metadata = root / 'metadata.json'
            metadata.write_bytes(m.encoded(graph))
            (root / 'THIRD_PARTY_NOTICES.md').write_text('# Existing notices\n')
            command = [sys.executable, str(SCRIPT), '--root', str(root)]
            result = subprocess.run([*command, '--generate', '--metadata', str(metadata), '--maven-cache', str(cache)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            result = subprocess.run([*command, '--check', '--verify-material', '--maven-cache', str(cache)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn('Exact supplier notice', (root / 'THIRD_PARTY_NOTICES.md').read_text())
            self.assertIn('Exact supplier notice', (root / 'THIRD_PARTY_NOTICES.rtf').read_text())
            (root / m.PAYLOAD).write_text('detached')
            result = subprocess.run([*command, '--check'], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('detached Android notice output', result.stderr)

    def test_parent_pom_licenses_are_replayed_and_cycles_or_unknown_licenses_refuse(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            graph, cache = self.fixture(root, m)
            old = next(cache.glob('org.example/library/1/*/*.pom'))
            old.unlink()
            child = b'<project xmlns="http://maven.apache.org/POM/4.0.0"><parent><groupId>org.example</groupId><artifactId>parent</artifactId><version>2</version></parent><artifactId>library</artifactId><version>1</version></project>'
            parent = b'<project xmlns="http://maven.apache.org/POM/4.0.0"><groupId>org.example</groupId><artifactId>parent</artifactId><version>2</version><licenses><license><name>Apache 2.0</name></license></licenses></project>'
            def put(name, filename, content):
                group, artifact, version = name.split(':')
                path = cache / group / artifact / version / hashlib.sha1(content).hexdigest().lstrip('0') / filename
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content)
                return path
            put('org.example:library:1', 'library-1.pom', child)
            parent_file = put('org.example:parent:2', 'parent-2.pom', parent)
            data = m.collect(root, graph, cache)
            m.validate_inventory(root, data)
            self.assertEqual([x['coordinate'] for x in data['components'][0]['pom_chain']],
                             ['org.example:library:1', 'org.example:parent:2'])
            parent_file.unlink()
            unknown = parent.replace(b'Apache 2.0', b'Unmapped license')
            unknown_file = put('org.example:parent:2', 'parent-2.pom', unknown)
            with self.assertRaisesRegex(ValueError, 'Unmapped'):
                m.collect(root, graph, cache)
            unknown_file.unlink()
            cycle = parent.replace(b'<licenses><license><name>Apache 2.0</name></license></licenses>',
                                   b'<parent><groupId>org.example</groupId><artifactId>library</artifactId><version>1</version></parent>')
            put('org.example:parent:2', 'parent-2.pom', cycle)
            with self.assertRaisesRegex(ValueError, 'Cyclic'):
                m.license_chain(cache, 'org.example:library:1')

    def test_capture_requires_current_app_pinned_modules_and_exact_generated_dependencies(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.fixture(root, m)
            wrapper = root / m.ANDROID / 'gradle/wrapper/gradle-wrapper.properties'
            wrapper.write_text('distributionUrl=https://example.invalid/gradle-9.6.1-bin.zip\n')
            cache = root / 'cargo-cache'
            cache.mkdir()
            lock = (root / 'Cargo.lock').read_text()
            projects = [{'name': 'app', 'directory': str(root / 'project/app')}]
            for name, (package, member) in m.MODULES.items():
                directory = root / 'modules' / name
                directory.mkdir(parents=True)
                content = b'// exact upstream Gradle configuration\n' if package else (root / member).read_bytes()
                (directory / 'build.gradle.kts').write_bytes(content)
                projects.append({'name': name, 'directory': str(directory)})
                if package:
                    archive = cache / (package + '-1.crate')
                    with tarfile.open(archive, 'w:gz') as handle:
                        entry = tarfile.TarInfo(package + '-1/' + member)
                        entry.size = len(content)
                        handle.addfile(entry, io.BytesIO(content))
                    lock += '\n[[package]]\nname="' + package + '"\nversion="1"\nsource="registry+https://github.com/rust-lang/crates.io-index"\nchecksum="' + sha(archive.read_bytes()) + '"\n'
            (root / 'Cargo.lock').write_text(lock)
            project = root / 'project'
            shutil.copytree(root / m.ANDROID, project)
            settings = []
            for item in projects[1:]:
                settings += ["include ':" + item['name'] + "'", "project(':" + item['name'] + "').projectDir = new File(" + json.dumps(item['directory']) + ")"]
            (project / 'tauri.settings.gradle').write_text('\n'.join(settings))
            script = 'val implementation = configurations.getByName("implementation")\ndependencies {\nimplementation("androidx.lifecycle:lifecycle-process:2.10.0")\n' + '\n'.join('implementation(project(":' + item['name'] + '"))' for item in projects[1:]) + '\n}'
            (project / 'app/tauri.build.gradle.kts').write_text(script)
            report = {'projects': projects, 'gradleVersion': '9.6.1'}
            m.validate_project(root, project, report, cache)
            for change in ['module', 'generated', 'version', 'app']:
                with self.subTest(change=change):
                    path = root / 'modules/tauri-android/build.gradle.kts' if change == 'module' else project / ('app/tauri.build.gradle.kts' if change == 'generated' else 'app/build.gradle.kts')
                    before = path.read_bytes()
                    if change == 'version': report['gradleVersion'] = '8.0'
                    else: path.write_bytes(before + b'\nimplementation("unexpected:dependency:1")\n')
                    with self.assertRaises(ValueError):
                        m.validate_project(root, project, report, cache)
                    path.write_bytes(before)
                    report['gradleVersion'] = '9.6.1'


if __name__ == '__main__':
    unittest.main()
