#!/usr/bin/env python3
"""Collect/check source-bound Android runtime notice candidates; never build an app."""
import argparse
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import tarfile
import subprocess
import sys
import tempfile
import tomllib
import xml.etree.ElementTree as ET
import zipfile

_spec = importlib.util.spec_from_file_location('dependency_notices', Path(__file__).with_name('dependency-notices.py'))
core = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(core)
require, sha, encoded, safe = core.require, core.sha, core.encoded, core.safe
INVENTORY = 'src/licenses/android-inventory.json'
PAYLOAD = 'THIRD_PARTY_ANDROID.md'
MATERIAL = 'scripts/notices/android-material.json'
REPORT = 'scripts/notices/android-runtime-report.init.gradle'
CONFIGURATION = 'arm64ReleaseRuntimeClasspath'
ANDROID = 'src-tauri/gen/android'
MODULES = {'tauri-android': ('tauri', 'mobile/android/build.gradle.kts'),
           'tauri-plugin-dialog': ('tauri-plugin-dialog', 'android/build.gradle.kts'),
           'tauri-plugin-document-access': (None, 'crates/document-access/android/build.gradle.kts')}


def coordinate(value):
    require(isinstance(value, str) and re.fullmatch(r'[A-Za-z0-9_.-]+:[A-Za-z0-9_.-]+:[A-Za-z0-9_.+-]+', value),
            'Invalid exact Maven coordinate')
    return value.split(':')


def zip_materials(raw, checksum):
    require(sha(raw) == checksum, 'Maven archive checksum differs')
    result = []
    def inspect(data, prefix='', nested=False):
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            names = set()
            for member in archive.infolist():
                name = member.filename.rstrip('/')
                safe(name)
                require(name not in names, 'Duplicate Maven ZIP member')
                names.add(name)
                mode = member.external_attr >> 16
                require(not stat.S_ISLNK(mode), 'Symlink in Maven ZIP')
                if member.is_dir():
                    continue
                leaf = PurePosixPath(name).name.lower()
                is_notice = leaf.startswith(core.NOTICE_PREFIXES) or leaf in {
                    'fastdoubleparser-license', 'fastdoubleparser-notice', 'bigint-license'}
                is_jar = not nested and (name == 'classes.jar' or name.startswith('libs/') and name.endswith('.jar'))
                if not (is_notice or is_jar):
                    continue
                require(member.file_size <= (64 if is_jar else 16) * 1024 * 1024, 'Maven notice input exceeds size limit')
                content = archive.read(member)
                if is_jar:
                    inspect(content, prefix + name + '!/', True)
                else:
                    result.append({'path': prefix + name, 'sha256': sha(content), 'text': content.decode('utf-8')})
    inspect(raw)
    return sorted(result, key=lambda item: item['path'])


def pom_record(raw, expected):
    coordinate(expected)
    require(len(raw) <= 4 * 1024 * 1024 and b'<!DOCTYPE' not in raw.upper() and b'<!ENTITY' not in raw.upper(),
            'Unsafe Maven POM')
    root = ET.fromstring(raw)
    ns = '{http://maven.apache.org/POM/4.0.0}'
    require(root.tag == ns + 'project', 'Invalid Maven POM root')
    def text(path):
        return root.findtext('/'.join(ns + name for name in path.split('/')))
    group = text('groupId') or text('parent/groupId')
    version = text('version') or text('parent/version')
    artifact = text('artifactId')
    require(':'.join([group or '', artifact or '', version or '']) == expected, 'Maven POM identity differs')
    parent = None
    if root.find(ns + 'parent') is not None:
        parent = ':'.join(text('parent/' + key) or '' for key in ['groupId', 'artifactId', 'version'])
        coordinate(parent)
    licenses = [{child.tag.removeprefix(ns): child.text for child in license}
                for license in root.findall(ns + 'licenses/' + ns + 'license')]
    return {'coordinate': expected, 'sha256': sha(raw), 'parent': parent, 'licenses': licenses}


def validate_snapshot(snapshot, inputs):
    require(snapshot.get('schema') == 1 and snapshot.get('configuration') == CONFIGURATION,
            'Unexpected Android graph schema/configuration')
    require(snapshot.get('inputs') == inputs, 'Stale Android metadata source inputs')
    nodes = {item['id']: item for item in snapshot['components']}
    require(len(nodes) == len(snapshot['components']) and 'project :app' in nodes, 'Missing or duplicate Android graph root')
    reached, pending = set(), ['project :app']
    while pending:
        name = pending.pop()
        if name in reached:
            continue
        require(name in nodes, 'Missing selected Android graph node')
        reached.add(name)
        for dependency in nodes[name]['dependencies']:
            require(not dependency.get('failure') and dependency.get('selected'), 'Unresolved Android dependency')
            pending.append(dependency['selected'])
    require(reached == set(nodes), 'Unreachable Android graph node')
    modules = {item['module'] for item in nodes.values() if item.get('module') and not item['module'].startswith('project:')}
    selected = {}
    for artifact in snapshot['artifacts']:
        name = artifact['component']
        coordinate(name)
        require(name in modules, 'Unknown Android runtime artifact')
        require(re.fullmatch('[a-f0-9]{64}', artifact.get('sha256', '')), 'Invalid Maven artifact hash')
        filename = Path(artifact['file']).name
        safe(filename)
        require(filename.endswith(('.aar', '.jar')), 'Unexpected Maven runtime artifact type')
        record = {'component': name, 'archive': filename, 'sha256': artifact['sha256']}
        require(name not in selected or selected[name] == record, 'Conflicting Maven artifacts for one coordinate')
        selected[name] = record
    require(selected, 'Empty Android runtime artifact set')
    return selected


def graph_inputs(root):
    names = {ANDROID + '/' + name for name in ['app/build.gradle.kts', 'build.gradle.kts',
             'settings.gradle', 'gradle.properties', 'gradle/wrapper/gradle-wrapper.properties']}
    names.update(path.relative_to(root).as_posix() for path in (root / ANDROID / 'buildSrc').rglob('*')
                 if path.is_file() and path.suffix in {'.kt', '.kts', '.gradle'}
                 and not {'build', '.gradle', '.kotlin'}.intersection(path.relative_to(root / ANDROID / 'buildSrc').parts))
    names.add(MODULES['tauri-plugin-document-access'][1])
    names.add(REPORT)
    result = core.graph_inputs(root)
    result.update({name: sha((root / name).read_bytes().replace(b'\r\n', b'\n')) for name in sorted(names)})
    source = (root / 'src-tauri/src/lib.rs').read_text(encoding='utf-8')
    plugins = re.findall(r'\.plugin\(\s*([A-Za-z0-9_]+)::init\(\)\s*\)', source)
    require(sorted(plugins) == ['tauri_plugin_dialog', 'tauri_plugin_document_access']
            and len(re.findall(r'\.plugin\(', source)) == len(plugins),
            'Android plugin set changed; update the scoped notice resolver')
    result['android-plugin-set'] = sha(encoded(sorted(plugins)))
    return dict(sorted(result.items()))


def source_inputs(root):
    result = {**core.source_fingerprints(root), **graph_inputs(root)}
    for name in ['scripts/android-notices.py', MATERIAL]:
        raw = (root / name).read_bytes()
        result[name] = sha(encoded(core.unique_json(raw)) if name.endswith('.json') else raw.replace(b'\r\n', b'\n'))
    return dict(sorted(result.items()))


def validate_project(root, project, report, cargo_cache):
    """Require captured configuration to use current tracked and pinned module scripts."""
    for name in graph_inputs(root):
        if name.startswith(ANDROID + '/'):
            relative = name.removeprefix(ANDROID + '/')
            require((project / relative).read_bytes().replace(b'\r\n', b'\n') ==
                    (root / name).read_bytes().replace(b'\r\n', b'\n'), 'Android capture project differs: ' + relative)
    projects = {item['name']: item for item in report['projects']}
    wrapper = (root / ANDROID / 'gradle/wrapper/gradle-wrapper.properties').read_text(encoding='utf-8')
    version = re.search(r'gradle-([0-9][0-9.]+)-bin\.zip', wrapper)
    require(version and report.get('gradleVersion') == version[1], 'Android capture Gradle version differs')
    require(set(projects) == {'app', *MODULES} and len(projects) == len(report['projects']),
            'Unexpected configured Android project/module set')
    require(Path(projects['app']['directory']).resolve() == (project / 'app').resolve(), 'Android app project directory differs')
    pins = core.cargo_pins(root)
    packages = tomllib.loads((root / 'Cargo.lock').read_text(encoding='utf-8'))['package']
    for name, (package, member) in MODULES.items():
        actual = Path(projects[name]['directory']) / 'build.gradle.kts'
        if package is None:
            expected = (root / member).read_bytes()
        else:
            matches = [item for item in packages if item['name'] == package and item.get('source', '').startswith('registry+')]
            require(len(matches) == 1, 'Ambiguous Android Cargo module')
            version = matches[0]['version']
            filename = package + '-' + version
            archive = cargo_cache / (filename + '.crate')
            require(sha(archive.read_bytes()) == pins['cargo:' + package + '@' + version], 'Android module crate checksum differs')
            with tarfile.open(archive) as handle:
                item = handle.getmember(filename + '/' + member)
                require(item.isfile(), 'Android Gradle source must be regular')
                expected = handle.extractfile(item).read()
        require(actual.read_bytes().replace(b'\r\n', b'\n') == expected.replace(b'\r\n', b'\n'),
                'Configured Android module differs from pinned source: ' + name)
    def normalized(text):
        return re.sub(r'\s+', '', re.sub(r'(?m)^\s*//.*$', '', text))
    settings = []
    for name in MODULES:
        settings += ["include ':" + name + "'", "project(':" + name + "').projectDir = new File(" +
                     json.dumps(str(Path(projects[name]['directory']).resolve())) + ")"]
    settings_source = (project / 'tauri.settings.gradle').read_text(encoding='utf-8')
    settings_source = re.sub(r'new\s+File\(("(?:\\.|[^"\\])*")\)',
                             lambda match: 'new File(' + json.dumps(str(Path(json.loads(match[1])).resolve())) + ')', settings_source)
    require(normalized(settings_source) == normalized('\n'.join(settings)),
            'Unexpected generated Android settings')
    dependencies = ['implementation("androidx.lifecycle:lifecycle-process:2.10.0")']
    dependencies += ['implementation(project(":' + name + '"))' for name in MODULES]
    expected = 'val implementation = configurations.getByName("implementation")\ndependencies {\n' + '\n'.join(dependencies) + '\n}'
    require(normalized((project / 'app/tauri.build.gradle.kts').read_text(encoding='utf-8')) == normalized(expected),
            'Unexpected generated Android dependency script')


def capture(root, project, gradle, cargo_cache, output):
    inputs = graph_inputs(root)
    project = project.resolve()
    with tempfile.TemporaryDirectory(prefix='taggroove-android-notices-') as folder:
        destination = Path(folder) / 'resolution.json'
        command = [str(gradle), '-p', str(project), '-I', str(root / REPORT), ':app:noticeDependencyReport',
                   '-PnoticeReport=' + str(destination), '--offline', '--no-daemon', '--no-configuration-cache',
                   '--console', 'plain', '--max-workers=1']
        # Dependency configuration does not need or use application signing.
        environment = {key: value for key, value in os.environ.items() if key not in {
            'ANDROID_KEYSTORE_PATH', 'ANDROID_KEYSTORE_BASE64', 'ANDROID_KEYSTORE_PASSWORD',
            'ANDROID_KEY_ALIAS', 'ANDROID_KEY_PASSWORD'}}
        result = subprocess.run(command, cwd=root, env=environment, capture_output=True, text=True, check=False)
        require(result.returncode == 0, 'Android metadata capture failed:\n' + result.stdout[-10000:] + result.stderr[-3000:])
        data = core.unique_json(destination.read_bytes())
        validate_project(root, project, data, cargo_cache)
        require(graph_inputs(root) == inputs, 'Android graph inputs changed during capture')
        data.update({'schema': 1, 'inputs': inputs})
        validate_snapshot(data, inputs)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(encoded(data))


def find_cargo_cache(root):
    packages = tomllib.loads((root / 'Cargo.lock').read_text(encoding='utf-8'))['package']
    names = {package for package, _ in MODULES.values() if package is not None}
    filenames = [item['name'] + '-' + item['version'] + '.crate' for item in packages
                 if item['name'] in names and item.get('source', '').startswith('registry+')]
    require(len(filenames) == len(names), 'Ambiguous Android Cargo module versions')
    home = Path(os.environ.get('CARGO_HOME', Path.home() / '.cargo'))
    matches = [path for path in sorted((home / 'registry/cache').glob('*'))
               if all((path / name).is_file() for name in filenames)]
    require(matches, 'Missing Android Cargo archives; provide --cargo-cache')
    return matches[0]


def cached_file(cache, name, filename, expected=None):
    group, artifact, version = coordinate(name)
    safe(filename)
    matches = list((cache / group / artifact / version).glob('*/' + filename))
    require(matches, 'Missing Maven cache file: ' + name + '/' + filename)
    by_hash = {}
    for path in matches:
        raw = path.read_bytes()
        # Gradle's numeric SHA1 cache key drops leading zero digits.
        require(hashlib.sha1(raw).hexdigest().lstrip('0') == path.parent.name, 'Maven cache identity differs')
        by_hash[sha(raw)] = raw
    require(len(by_hash) == 1, 'Ambiguous Maven cache material: ' + name)
    digest, raw = next(iter(by_hash.items()))
    require(expected is None or digest == expected, 'Maven archive checksum differs: ' + name)
    return raw


def license_chain(cache, name):
    chain, visited = [], set()
    while name:
        require(name not in visited and len(chain) < 8, 'Cyclic or excessive Maven POM inheritance')
        visited.add(name)
        group, artifact, version = coordinate(name)
        record = pom_record(cached_file(cache, name, artifact + '-' + version + '.pom'), name)
        chain.append(record)
        if record['licenses']:
            return chain
        name = record['parent']
    raise ValueError('Missing declared Maven license')


def collect(root, snapshot, cache):
    selected = validate_snapshot(snapshot, graph_inputs(root))
    material = core.unique_json((root / MATERIAL).read_bytes())
    canonical = material['canonical_license']
    raw = (root / safe(canonical['source_file'])).read_bytes()
    require(sha(raw) == canonical['sha256'], 'Canonical Android license source differs')
    reference = selected.get(canonical['reference_component'])
    require(reference and reference['sha256'] == canonical['archive_sha256'], 'Canonical Android license artifact differs')
    source = zip_materials(cached_file(cache, reference['component'], reference['archive'], reference['sha256']), reference['sha256'])
    require(any(item['path'] == canonical['archive_member'] and item['sha256'] == sha(raw) for item in source),
            'Canonical Android license missing from pinned artifact')
    components, texts = [], {sha(raw): raw.decode('utf-8')}
    for name, artifact in sorted(selected.items()):
        archive = cached_file(cache, name, artifact['archive'], artifact['sha256'])
        chain = license_chain(cache, name)
        declared = chain[-1]['licenses']
        require(declared and all(item.get('name') in material['apache_license_names'] for item in declared),
                'Unmapped Maven license declaration: ' + name)
        provided = zip_materials(archive, artifact['sha256'])
        materials = []
        for item in provided:
            texts[item['sha256']] = item['text']
            materials.append({key: value for key, value in item.items() if key != 'text'})
        materials.append({'path': canonical['archive_member'], 'sha256': canonical['sha256'],
                          'source_file': canonical['source_file'], 'reference_component': canonical['reference_component'],
                          'reference_archive_sha256': canonical['archive_sha256']})
        group, library, version = coordinate(name)
        components.append({'id': 'maven:' + group + ':' + library + '@' + version, 'name': group + ':' + library,
                           'version': version, 'ecosystem': 'maven', 'role': 'Android runtime candidate',
                           'coordinate': name, 'archive': artifact['archive'], 'archive_sha256': artifact['sha256'],
                           'license': 'Apache-2.0', 'pom_chain': chain, 'materials': materials})
    # Keep selected graph and artifact identities, with no machine-specific paths.
    graph = {'schema': 1, 'configuration': CONFIGURATION, 'inputs': snapshot['inputs'],
             'components': sorted(snapshot['components'], key=lambda item: item['id']),
             'artifacts': [{'component': item['component'], 'file': item['archive'], 'sha256': item['sha256']}
                           for item in sorted(selected.values(), key=lambda item: item['component'])]}
    return {'schema': 1, 'inputs': source_inputs(root), 'resolution': graph,
            'components': components, 'texts': dict(sorted(texts.items()))}


def validate_inventory(root, data):
    require(data.get('schema') == 1 and data.get('inputs') == source_inputs(root), 'Stale Android notice source inputs')
    selected = validate_snapshot(data['resolution'], graph_inputs(root))
    mapping = core.unique_json((root / MATERIAL).read_bytes())
    canonical = mapping['canonical_license']
    canonical_raw = (root / safe(canonical['source_file'])).read_bytes()
    require(sha(canonical_raw) == canonical['sha256'], 'Canonical Android license source differs')
    seen, referenced = set(), set()
    for item in data['components']:
        name = item['coordinate']
        group, artifact, version = coordinate(name)
        require(name not in seen and name in selected, 'Duplicate or unknown Maven notice component')
        seen.add(name)
        require(item['id'] == 'maven:' + group + ':' + artifact + '@' + version
                and item['name'] == group + ':' + artifact and item['version'] == version
                and item['ecosystem'] == 'maven' and item['role'] == 'Android runtime candidate'
                and item['license'] == 'Apache-2.0', 'Maven notice identity differs')
        require(item['archive'] == selected[name]['archive'] and item['archive_sha256'] == selected[name]['sha256'],
                'Maven notice artifact binding differs')
        chain = item['pom_chain']
        require(chain and len(chain) <= 8 and chain[0]['coordinate'] == name, 'Maven POM chain identity differs')
        require(len({part['coordinate'] for part in chain}) == len(chain), 'Cyclic Maven POM chain')
        for index, part in enumerate(chain):
            coordinate(part['coordinate'])
            require(re.fullmatch('[a-f0-9]{64}', part.get('sha256', '')), 'Invalid Maven POM hash')
            if index < len(chain) - 1:
                require(not part['licenses'] and part['parent'] == chain[index + 1]['coordinate'], 'Maven POM inheritance differs')
        require(chain[-1]['licenses'] and all(part.get('name') in mapping['apache_license_names'] for part in chain[-1]['licenses']),
                'Unmapped Maven license declaration')
        require(item.get('materials'), 'Missing Maven notice material')
        canonical_found, material_ids = False, set()
        for material in item['materials']:
            # AAR nested member separators are descriptive, never filesystem paths.
            safe(material['path'])
            identity = (material['path'], material.get('reference_component'))
            require(identity not in material_ids, 'Duplicate Maven material member')
            material_ids.add(identity)
            digest = material['sha256']
            text = data['texts'].get(digest)
            require(isinstance(text, str) and sha(text.encode('utf-8')) == digest, 'Android notice text hash differs')
            referenced.add(digest)
            if 'source_file' in material:
                expected = {'path': canonical['archive_member'], 'sha256': canonical['sha256'],
                            'source_file': canonical['source_file'], 'reference_component': canonical['reference_component'],
                            'reference_archive_sha256': canonical['archive_sha256']}
                require(material == expected, 'Canonical Android notice mapping differs')
                reference = selected.get(canonical['reference_component'])
                require(reference and reference['sha256'] == canonical['archive_sha256'], 'Canonical Maven artifact differs')
                canonical_found = True
        require(canonical_found, 'Missing full declared Android license text')
    require(seen == set(selected), 'Missing Maven notice component')
    require(referenced == set(data['texts']), 'Missing or unreferenced Android notice text')


def render(data):
    return core.render_notices(data).replace('# Dependency licenses and notices', '# Android/JVM licenses and notices', 1)


def attached_common(source, payload):
    begin, end = '<!-- BEGIN GENERATED ANDROID NOTICES -->', '<!-- END GENERATED ANDROID NOTICES -->'
    require(source.count(begin) == source.count(end) and source.count(begin) <= 1, 'Ambiguous Android notice markers')
    section = begin + '\n\n' + payload.rstrip('\n') + '\n\n' + end
    if begin not in source:
        return source.rstrip('\n') + '\n\n' + section + '\n'
    prefix, remaining = source.split(begin, 1)
    old, suffix = remaining.split(end, 1)
    require(end not in prefix and begin not in old, 'Invalid Android notice marker order')
    return prefix + section + suffix


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--capture', type=Path, metavar='SNAPSHOT')
    mode.add_argument('--generate', action='store_true')
    mode.add_argument('--check', action='store_true')
    parser.add_argument('--project', type=Path, help='Prepared Android Gradle project; metadata only')
    parser.add_argument('--gradle', type=Path)
    parser.add_argument('--cargo-cache', type=Path)
    parser.add_argument('--metadata', type=Path)
    parser.add_argument('--maven-cache', type=Path)
    parser.add_argument('--verify-material', action='store_true')
    args = parser.parse_args()
    root = args.root.resolve()
    try:
        if args.capture:
            project = args.project or root / ANDROID
            gradle = args.gradle or project / ('gradlew.bat' if os.name == 'nt' else 'gradlew')
            cargo_cache = args.cargo_cache or find_cargo_cache(root)
            capture(root, project, gradle.resolve(), cargo_cache, args.capture)
            print('Captured source-bound Android runtime metadata without an application build')
            return
        before = source_inputs(root)
        if args.generate:
            require(args.metadata and args.maven_cache, 'Generation requires --metadata and --maven-cache')
            data = collect(root, core.unique_json(args.metadata.read_bytes()), args.maven_cache)
        else:
            data = core.unique_json((root / INVENTORY).read_bytes())
        validate_inventory(root, data)
        if args.metadata:
            current = validate_snapshot(core.unique_json(args.metadata.read_bytes()), graph_inputs(root))
            require(current == validate_snapshot(data['resolution'], graph_inputs(root)), 'Resolved Android artifacts differ from bundled notices')
        if args.verify_material:
            require(args.maven_cache, 'Material replay requires --maven-cache')
            require(collect(root, data['resolution'], args.maven_cache) == data, 'Replayed Maven material differs')
        payload = render(data)
        raw = (root / 'THIRD_PARTY_NOTICES.md').read_bytes()
        source = raw.decode('utf-8').replace('\r\n', '\n').replace('\r', '\n')
        common = attached_common(source, payload)
        outputs = {INVENTORY: encoded(data), PAYLOAD: payload.encode(),
                   'THIRD_PARTY_NOTICES.md': common.encode(), 'THIRD_PARTY_NOTICES.rtf': core.rtf_bytes(common)}
        require(source_inputs(root) == before == data['inputs'], 'Android notice inputs changed during collection')
        if args.check:
            for name, content in outputs.items():
                require((root / name).read_bytes() == content, 'Stale or detached Android notice output: ' + name)
        else:
            core.publish_outputs(root, outputs, {'THIRD_PARTY_NOTICES.md': raw})
        print(f"Verified {len(data['components'])} Android runtime candidates and {len(data['texts'])} notice texts")
    except (OSError, ValueError, KeyError, TypeError, zipfile.BadZipFile, tarfile.TarError, ET.ParseError) as error:
        parser.exit(1, f'Android notices: {error}\n')


if __name__ == '__main__':
    main()
