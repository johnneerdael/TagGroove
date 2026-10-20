"""Corresponding-source integrity and offline-layout tests, using tiny local archives."""
import hashlib
import difflib
import base64
import importlib.util
import io
import inspect
import json
import shutil
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "audio-source-bundle.py"
PCM_NOTICE = b'/* Copyright fixture supplier. Permission is hereby granted. */\n'
PCM_SOURCE = PCM_NOTICE + b'pcm source fixture\n'
CRC_ORIGINAL = b'/* GNU Lesser General Public License */\n#if ARCH_AARCH64\ncrc_arch();\n#endif\n'
CRC_PATCHED = CRC_ORIGINAL.replace(b'#if ARCH_AARCH64', b'#if ARCH_AARCH64 && CONFIG_GPL')
TARGETS = {
    "windows-x64": "x86_64-pc-windows-msvc",
    "macos-arm64": "aarch64-apple-darwin",
    "macos-x64": "x86_64-apple-darwin",
    "linux-x64": "x86_64-unknown-linux-gnu",
    "linux-arm64": "aarch64-unknown-linux-gnu",
    "android-arm64": "aarch64-linux-android",
    "ios-arm64-simulator-preview": "aarch64-apple-ios-sim",
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def load_builder():
    spec = importlib.util.spec_from_file_location("audio_source_bundle", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def cargo_vendor_fixture():
    cargo = b'[package]\nname="example"\nversion="1.0.0"\n'
    source = b'fn main() {}\n'
    checksum = 'a' * 64
    lock = ('[[package]]\nname="example"\nversion="1.0.0"\nsource="registry+https://github.com/rust-lang/crates.io-index"\nchecksum="' + checksum + '"\n').encode()
    data = {'Cargo.toml': cargo, 'src/lib.rs': source}
    metadata = {'package': checksum, 'files': {name: digest(content) for name, content in data.items()}}
    vendor = {'example-1.0.0/' + name: content for name, content in data.items()}
    vendor['example-1.0.0/.cargo-checksum.json'] = json.dumps(metadata).encode()
    return lock, vendor


def fixture(root, original_sha=None, patched_sha=None, cargo_config=None, notice_material=False, runtime_material=None,
            extra_files=None, macos_signing=False):
    """Make a real Git source binding, four local upstream archives and seven fake builds."""
    root = Path(root)
    app, archives, evidence = [root / name for name in ("app", "archives", "evidence")]
    for path in (app, archives, evidence):
        path.mkdir(parents=True)
    files = {
        "package.json": json.dumps({"name": "opentag", "version": "0.1.0"}) + "\n",
        "package-lock.json": json.dumps({"version": "0.1.0", "packages": {"": {"version": "0.1.0"}}}) + "\n",
        "Cargo.toml": '[workspace]\nmembers=["src-tauri"]\nresolver="2"\n',
        "Cargo.lock": 'version = 4\n\n[[package]]\nname = "opentag"\nversion = "0.1.0"\n',
        "src-tauri/Cargo.toml": '[package]\nname = "opentag"\nversion = "0.1.0"\n',
        "src-tauri/tauri.conf.json": json.dumps({"version": "0.1.0"}) + "\n",
        "src-tauri/src/main.rs": "fn main() {}\n",
        "nested/oid/child.txt": "Git tree component named oid\n",
        "src-tauri/gen/apple/project.yml": 'CFBundleShortVersionString: "0.1.0"\nCFBundleVersion: "0.1.0"\n',
        "src-tauri/gen/apple/opentag_iOS/Info.plist": '<plist><key>CFBundleShortVersionString</key><string>0.1.0</string><key>CFBundleVersion</key><string>0.1.0</string></plist>\n',
        "scripts/audio-deps/build.py": "# tiny source recipe fixture, never executed\n",
        "scripts/audio-deps/bzip2/CMakeLists.txt": "# fixture\n",
        "scripts/audio-deps/ffmpeg_policy.py": "# tiny policy source fixture, never executed\n",
        "LICENSE": "Application license fixture\n",
    }
    if cargo_config is not None:
        files[".cargo/config.toml"] = cargo_config
    files.update(extra_files or {})
    if macos_signing:
        files['src-tauri/macos/signing-policy.json'] = (SCRIPT.parents[1]/'src-tauri/macos/signing-policy.json').read_text()
        config = json.loads(files['src-tauri/tauri.conf.json']);config['identifier'] = 'org.opentag.editor'
        files['src-tauri/tauri.conf.json'] = json.dumps(config)+'\n'
    if runtime_material is not None:
        runtime_spec, runtime_files = runtime_material
        files['scripts/notices/linux-runtime-material.json'] = json.dumps(runtime_spec)+'\n'
        for name, raw in runtime_files.items():
            if name.startswith('app/'):
                files[name.removeprefix('app/')] = raw.decode('utf-8')
        runtime_archives = root/'runtime-archives';runtime_archives.mkdir()
        for name, raw in runtime_files.items():
            if name.startswith('linux-runtime/archives/'):
                (runtime_archives/Path(name).name).write_bytes(raw)
    components = {}
    for name in ("chromaprint", "ffmpeg", "zlib", "bzip2"):
        archive = archives / f"{name}-1.tar.gz"
        with tarfile.open(archive, "w:gz") as handle:
            data = f"{name} license fixture\n".encode()
            info = tarfile.TarInfo(f"{name}-1/LICENSE")
            info.size = len(data)
            handle.addfile(info, io.BytesIO(data))
            if name == "ffmpeg":
                configure = b"original upstream configure\n"
                info = tarfile.TarInfo("ffmpeg-1/configure")
                info.size = len(configure)
                handle.addfile(info, io.BytesIO(configure))
                source = PCM_SOURCE
                info = tarfile.TarInfo("ffmpeg-1/libavcodec/pcm.c")
                info.size = len(source)
                handle.addfile(info, io.BytesIO(source))
                source = CRC_ORIGINAL
                info = tarfile.TarInfo("ffmpeg-1/libavutil/crc.c")
                info.size = len(source)
                handle.addfile(info, io.BytesIO(source))
        components[name] = {"version": "1", "url": f"https://example.invalid/{archive.name}",
                            "archive": archive.name, "sha256": digest(archive.read_bytes()),
                            "directory": f"{name}-1", "license": "test-only", "license_files": ["LICENSE"]}
    patches = []
    for target, old, new, filename in [
        ('configure', 'original upstream configure\n', 'patched upstream configure\n', 'patches/ffmpeg.patch'),
        ('libavutil/crc.c', CRC_ORIGINAL.decode(), CRC_PATCHED.decode(), 'patches/crc.patch')]:
        diff = ''.join(difflib.unified_diff(old.splitlines(True), new.splitlines(True), 'a/' + target, 'b/' + target))
        files['scripts/audio-deps/' + filename] = diff
        patches.append({'file': filename, 'sha256': digest(diff.encode()), 'component': 'ffmpeg', 'target': target,
                        'original_sha256': original_sha if target == 'configure' and original_sha else digest(old.encode()),
                        'patched_sha256': patched_sha if target == 'configure' and patched_sha else digest(new.encode())})
    inventory = {"format": 1, "components": components, "patches": patches,
                 "ffmpeg": {"decoders": ["pcm_s16le"], "demuxers": ["wav"]}}
    files["scripts/audio-deps/sources.json"] = json.dumps(inventory, indent=2) + "\n"
    if notice_material:
        materials = {'pcm': {'source_file': 'src/licenses/pcm.txt', 'sha256': digest(PCM_NOTICE), 'origin': {
            'kind': 'tar_slice', 'member': 'ffmpeg-1/libavcodec/pcm.c', 'source_sha256': digest(PCM_SOURCE),
            'offset': 0, 'length': len(PCM_NOTICE), 'sha256': digest(PCM_NOTICE)}}}
        files['scripts/notices/component-material.json'] = json.dumps({
            'bindings': {'ffmpeg': components['ffmpeg']}, 'materials': {} if notice_material == 'missing' else materials}) + '\n'
        files['src/licenses/pcm.txt'] = PCM_NOTICE.decode()
    for name, data in files.items():
        path = app / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data.encode("utf-8"))
    git = lambda *args: subprocess.check_output(
        ["git", "-c", "core.hooksPath=/dev/null", "-c", "core.autocrlf=false", *args], cwd=app, text=True).strip()
    git("init", "-q")
    git("add", ".")
    git("-c", "user.name=Source test", "-c", "user.email=test@example.invalid",
        "-c", "commit.gpgsign=false", "commit", "-qm", "source fixture")
    commit = git("rev-parse", "HEAD")
    builder = load_builder()
    for platform, target in TARGETS.items():
        build_inputs = {name.removeprefix("scripts/audio-deps/"): digest(data.encode())
                        for name, data in files.items() if name.startswith("scripts/audio-deps/")}
        recipe = digest(json.dumps(build_inputs, sort_keys=True).encode() + target.encode())
        prefix = root / "native" / target / recipe[:20] / "prefix"
        payloads = {"lib/libchromaprint.a": b"test-only static archive", "include/chromaprint.h": b"header\n",
                    "ffmpeg-config.h": b"#define CONFIG_GPL 0\n#define CONFIG_NONFREE 0\n#define CONFIG_VERSION3 0\n#define CONFIG_NETWORK 0\n",
                    "ffmpeg-components.h": b"#define CONFIG_PCM_S16LE_DECODER 1\n#define CONFIG_WAV_DEMUXER 1\n#define CONFIG_AHX_PARSER 0\n",
                    "notices/chromaprint/LICENSE": b"license\n",
                    "ffmpeg-patched-configure": b"patched upstream configure\n",
                    "ffmpeg-patched/libavutil/crc.c": CRC_PATCHED,
                    "ffmpeg-generated/config.h": b"generated config fixture\n",
                    "ffmpeg-generated-closure.json": (json.dumps({"config.h": digest(b"generated config fixture\n")}) + "\n").encode(),
                    "ffmpeg-source-closure.json": (json.dumps({"libavcodec/pcm.d": {"libavcodec/pcm.c": digest(PCM_SOURCE)},
                        "libavutil/crc.d": {"libavutil/crc.c": digest(CRC_PATCHED)}}) + "\n").encode()}
        for name, data in payloads.items():
            path = prefix / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        native = {"format": 1, "target": target, "recipe_sha256": recipe, "components": components,
                  "build_inputs": build_inputs, "patch_metadata": patches,
                  "configure_commands": [["cmake", "fixture"]], "ffmpeg_decoders": ["pcm_s16le"],
                  "ffmpeg_demuxers": ["wav"], "runtime_programs": [], "decoder_protocols": [],
                  "static_libraries": {"lib/libchromaprint.a": digest(payloads["lib/libchromaprint.a"])},
                  "prefix_files": {name: digest(data) for name, data in payloads.items()}}
        (prefix / "opentag-audio-manifest.json").write_text(json.dumps(native) + "\n")
        builder.collect_native(root / "native", target, commit, "0.1.0", evidence / f"native-{platform}.json")
    vendor, dist = root / "vendor", root / "dist"
    vendor.mkdir()
    dist.mkdir()
    (dist / "index.html").write_text("<html>stamped frontend fixture</html>\n")
    result = {"root": root, "app": app, "archives": archives, "evidence": evidence,
              "vendor": vendor, "dist": dist, "commit": commit}
    if runtime_material is not None:result['runtime_archives'] = runtime_archives
    if macos_signing:
        selected = json.loads(files['src-tauri/macos/signing-policy.json'])
        macos = root/'macos-evidence';macos.mkdir()
        for platform in ['macos-arm64', 'macos-x64']:
            receipt = {'schema': 1, 'platform': platform, 'target': TARGETS[platform], 'source_commit': commit,
                'version': '0.1.0', 'identity': selected['identity'], 'team_id': selected['team_id'],
                'identifier': 'org.opentag.editor', 'policy_sha256': digest(files['src-tauri/macos/signing-policy.json'].encode()),
                'app_cdhash': 'a'*40, 'checks': {name: True for name in
                    ['codesign', 'hardened_runtime', 'stapler', 'gatekeeper', 'dmg_app_binding']},
                'dmg_notarization': {'status': 'Accepted', 'id': '11111111-2222-4333-8444-555555555555'},
                'artifacts': {suffix: {'size': len(raw), 'sha256': digest(raw)} for suffix in ['.dmg', '.app.tar.gz']
                              for raw in [f'fixture {platform} {suffix}'.encode()]}}
            (macos/f'macos-signing-{platform}.json').write_bytes((json.dumps(receipt)+'\n').encode())
        result['macos_evidence'] = macos
    return result


def build(f, output=None, version="0.1.0"):
    output = output or f["root"] / f"TagGroove-v{version}-sources.tar.xz"
    load_builder().build_bundle(f["app"], f["commit"], version, output,
                               f["evidence"], f["archives"], f["vendor"], f["dist"],
                               **({'linux_runtime_sources': f['runtime_archives']} if 'runtime_archives' in f else {}),
                               **({'macos_evidence': f['macos_evidence']} if 'macos_evidence' in f else {}))
    return output


def rewrite_bundle(path, change, rehash=False):
    with tarfile.open(path, "r:xz") as archive:
        rows = [(item, archive.extractfile(item).read() if item.isfile() else None) for item in archive]
    rows = [(info, change(info.name, data)) for info, data in rows]
    if rehash:
        manifest_info, manifest_data = next(row for row in rows if row[0].name.endswith('/source-manifest.json'))
        manifest = json.loads(manifest_data)
        prefix = manifest_info.name.removesuffix('source-manifest.json')
        for info, data in rows:
            if info is not manifest_info:
                manifest['files'][info.name.removeprefix(prefix)] = {'size': len(data), 'sha256': digest(data), 'mode': info.mode}
        rows = [(info, (json.dumps(manifest) + '\n').encode() if info is manifest_info else data) for info, data in rows]
    with tarfile.open(path, "w:xz") as archive:
        for info, data in rows:
            if data is not None:
                info.size = len(data)
            archive.addfile(info, io.BytesIO(data) if data is not None else None)


def linux_distribution_fixture(root, builder, macos_signing=False):
    path = Path(__file__).with_name('test_linux_distribution_capture.py')
    spec = importlib.util.spec_from_file_location('distribution_fixture', path)
    helpers = importlib.util.module_from_spec(spec);spec.loader.exec_module(helpers)
    cases = helpers.LinuxDistributionCapture();capture = cases.module()
    x64, xobjects, xspec, signer = cases.fixture(capture)
    arm64, aobjects, aspec, _ = cases.fixture(capture, 'arm64')
    runtime_path = Path(__file__).with_name('test_linux_runtime_material.py')
    spec = importlib.util.spec_from_file_location('runtime_fixture', runtime_path)
    runtime_helpers = importlib.util.module_from_spec(spec);spec.loader.exec_module(runtime_helpers)
    runtime_spec, runtime_files = runtime_helpers.LinuxRuntimeMaterial().fixture()
    runtime_spec['asset_pins'] = {**xspec['asset_pins'], **aspec['asset_pins']}
    source_root = SCRIPT.parents[1]
    config = json.loads((source_root/'src-tauri/tauri.conf.json').read_bytes());config['version'] = '0.1.0'
    extra = {'src-tauri/tauri.conf.json': json.dumps(config)+'\n',
             'scripts/notices/linux-packager-recipes.json': json.dumps(capture.RECIPES)+'\n'}
    extra.update({row['source_file']: (source_root/row['source_file']).read_text() for row in capture.RECIPES['sources']})
    f = fixture(root, runtime_material=(runtime_spec, runtime_files), extra_files=extra, macos_signing=macos_signing)
    evidence = f['root']/'linux-evidence'
    config_raw = builder.stamped('src-tauri/tauri.conf.json', (f['app']/'src-tauri/tauri.conf.json').read_bytes(), '0.1.0')
    with patch.object(capture.material, 'UBUNTU_ARCHIVE_SIGNER', signer), \
         patch.object(builder, 'distribution_helper', return_value=capture):
        for record, objects in [(x64, xobjects), (arm64, aobjects)]:
            record['source_commit'] = f['commit'];record['version'] = '0.1.0'
            image = f"fixture {record['platform']} .AppImage".encode()
            record['appimage_sha256'] = digest(image);record['appimage_size'] = len(image)
            record['files']['usr/bin/taggroove']['source_commit'] = f['commit']
            record['builder_inputs']['src-tauri/tauri.conf.json'] = capture.reference(config_raw, objects)
            recipes = {name: objects[row['sha256']] for name, row in record['builder_recipes'].items()}
            for name, raw in capture.generated_files(config, recipes).items():
                record['files'][name] = {'kind': 'generated-builder', **capture.reference(raw, objects)}
            required = capture.verify_materials(record, objects, runtime_spec, f['commit'], '0.1.0', record['platform'])
            capture.export_capture(evidence, record, {key: objects[key] for key in required})
        output = f['root']/'complete.tar.xz'
        result = builder.build_bundle(f['app'], f['commit'], '0.1.0', output,
            f['evidence'], f['archives'], f['vendor'], f['dist'],
            linux_runtime_sources=f['runtime_archives'], linux_evidence=evidence, macos_evidence=f.get('macos_evidence'))
    return f, capture, signer, output


class SourceBundleTests(unittest.TestCase):
    def test_signed_mac_source_receipts_are_required_and_bound_to_git_policy_and_external_artifacts(self):
        self.assertIn('macos_evidence', inspect.signature(self.builder.build_bundle).parameters,
                      'Source publication has no macOS signing evidence input')
        f = fixture(self.f['root']/'mac-signing', macos_signing=True);output = build(f)
        manifest = self.builder.verify_bundle(output, f['commit'], '0.1.0', macos_evidence=f['macos_evidence'])
        self.assertEqual(set(manifest['macos_signing']), {'macos-arm64', 'macos-x64'})
        sidecar = f['macos_evidence']/'macos-signing-macos-x64.json';original = sidecar.read_bytes()
        sidecar.write_bytes(original+b'\n')
        with self.assertRaisesRegex(ValueError, 'differs from built packages'):
            self.builder.verify_bundle(output, f['commit'], '0.1.0', macos_evidence=f['macos_evidence'])
        sidecar.write_bytes(original)
        def changed(name, raw):
            if name.endswith('/macos-signing/targets/macos-arm64.json'):
                receipt = json.loads(raw);receipt['team_id'] = 'OTHERTEAM1';return json.dumps(receipt).encode()
            return raw
        rewrite_bundle(output, changed, rehash=True)
        with self.assertRaisesRegex(ValueError, 'signing policy'):
            self.builder.verify_bundle(output, f['commit'], '0.1.0')
        sidecar.unlink()
        with self.assertRaises((OSError, ValueError)):build(f, f['root']/'missing-mac.tar.xz')

    @unittest.skipUnless(shutil.which('gpgv'), 'GnuPG signature verifier required')
    def test_two_target_distribution_bundle_binds_sidecars_summary_and_exact_object_set(self):
        f, capture, signer, output = linux_distribution_fixture(self.f['root']/'two-linux-targets', self.builder)
        evidence = f['root']/'linux-evidence'
        with patch.object(capture.material, 'UBUNTU_ARCHIVE_SIGNER', signer), \
             patch.object(self.builder, 'distribution_helper', return_value=capture):
            result = self.builder.verify_bundle(output, f['commit'], '0.1.0', f['evidence'], evidence)
            self.assertEqual(set(result['linux_distribution']), {'linux-x64', 'linux-arm64'})
            self.builder.verify_bundle(output, f['commit'], '0.1.0', f['evidence'], evidence)
            sidecar = evidence/'linux-source-linux-x64.json';original = sidecar.read_bytes();sidecar.write_bytes(original+b'\n')
            with self.assertRaisesRegex(ValueError, 'differs from built packages'):
                self.builder.verify_bundle(output, f['commit'], '0.1.0', linux_evidence=evidence)
            sidecar.write_bytes(original)
            altered = f['root']/'summary.tar.xz';shutil.copyfile(output, altered)
            def change_summary(name, data):
                if not name.endswith('/source-manifest.json'):return data
                manifest = json.loads(data);manifest['linux_distribution']['linux-x64']['appimage_sha256'] = '0'*64
                return json.dumps(manifest).encode()
            rewrite_bundle(altered, change_summary)
            with self.assertRaisesRegex(ValueError, 'summary differs'):
                self.builder.verify_bundle(altered, f['commit'], '0.1.0')
            # Rehash the outer file inventory after adding an unreferenced object;
            # internal source/member requirements must still reject that object.
            extra_path = f['root']/'extra-object.tar.xz'
            with tarfile.open(output, 'r:xz') as source:
                rows = [(item, source.extractfile(item).read()) for item in source]
            manifest_info, manifest_raw = next(row for row in rows if row[0].name.endswith('/source-manifest.json'))
            prefix = manifest_info.name.removesuffix('source-manifest.json');raw = b'unreferenced object'
            name = 'linux-distribution/objects/'+digest(raw);manifest = json.loads(manifest_raw)
            manifest['files'][name] = {'size': len(raw), 'sha256': digest(raw), 'mode': 0o644}
            info = tarfile.TarInfo(prefix+name);info.mode = 0o644;rows.append((info, raw))
            with tarfile.open(extra_path, 'w:xz') as target:
                for info, raw in rows:
                    if info is manifest_info:raw = json.dumps(manifest).encode()
                    info.size = len(raw);target.addfile(info, io.BytesIO(raw))
            with self.assertRaisesRegex(ValueError, 'Untracked or unexpected'):
                self.builder.verify_bundle(extra_path, f['commit'], '0.1.0')

    def test_distribution_sources_are_mandatory_when_the_git_bound_recipe_is_present(self):
        self.assertIn('linux_evidence', inspect.signature(self.builder.build_bundle).parameters,
                      'Source publication has no Linux builder evidence input')
        path = Path(__file__).with_name('test_linux_runtime_material.py')
        spec = importlib.util.spec_from_file_location('runtime_source_fixture', path)
        helpers = importlib.util.module_from_spec(spec);spec.loader.exec_module(helpers)
        complete = fixture(self.f['root']/'distribution-source', runtime_material=helpers.LinuxRuntimeMaterial().fixture(),
                           extra_files={'scripts/notices/linux-packager-recipes.json': '{}\n'})
        with self.assertRaisesRegex(ValueError, 'Linux distribution evidence'):
            build(complete)

    def test_builder_observed_config_recipes_and_assets_must_equal_git_bound_bytes(self):
        self.assertTrue(callable(getattr(self.builder, 'verify_linux_git_inputs', None)),
                        'Linux observations are not bound to the Git-proven source tree')
        paths = {'src-tauri/tauri.conf.json': b'stamped config', 'recipe.sh': b'pinned recipe',
                 'src-tauri/icons/icon.png': b'original icon'}
        objects = {digest(raw): raw for raw in paths.values()}
        ref = lambda path: {'sha256': digest(paths[path]), 'size': len(paths[path])}
        capture = {'builder_inputs': {'src-tauri/tauri.conf.json': ref('src-tauri/tauri.conf.json')},
                   'builder_recipes': {'recipe': ref('recipe.sh')},
                   'files': {'TagGroove.png': {'kind': 'repository-bytes',
                       'source_file': 'src-tauri/icons/icon.png', **ref('src-tauri/icons/icon.png')}}}
        files = {'app/'+name: raw for name, raw in paths.items()}
        files['app/scripts/notices/linux-packager-recipes.json'] = json.dumps({'sources': [
            {'id': 'recipe', 'source_file': 'recipe.sh'}]}).encode()
        self.builder.verify_linux_git_inputs(files, capture, objects)
        for path in paths:
            with self.subTest(path=path),self.assertRaises(ValueError):
                self.builder.verify_linux_git_inputs({**files, 'app/'+path: b'changed source'}, capture, objects)

    def test_runtime_sources_are_required_and_cannot_be_detached_by_rehashing_the_bundle(self):
        self.assertIn('linux_runtime_sources', inspect.signature(self.builder.build_bundle).parameters,
                      'Source publication has no runtime material input')
        path = Path(__file__).with_name('test_linux_runtime_material.py')
        spec = importlib.util.spec_from_file_location('runtime_source_fixture', path)
        helpers = importlib.util.module_from_spec(spec);spec.loader.exec_module(helpers)
        runtime = helpers.LinuxRuntimeMaterial().fixture()
        complete = fixture(self.f['root']/'runtime-source', runtime_material=runtime)
        output = build(complete)
        self.builder.verify_bundle(output, complete['commit'], '0.1.0', complete['evidence'])
        with tarfile.open(output, 'r:xz') as archive:
            self.assertTrue(any(name.endswith('/linux-runtime/archives/dep.tar.gz') for name in archive.getnames()))
        selected = complete['runtime_archives']/'dep.tar.gz'
        selected.unlink()
        with self.assertRaises(ValueError):build(complete, complete['root']/'missing-source.tar.xz')
        rewrite_bundle(output, lambda name, data: data+b'changed' if name.endswith('/linux-runtime/archives/dep.tar.gz') else data,
                       rehash=True)
        with self.assertRaises(ValueError):self.builder.verify_bundle(output, complete['commit'], '0.1.0')

    def test_source_publication_requires_bound_compiler_supplier_notices(self):
        complete = fixture(self.f['root'] / 'notice-complete', notice_material=True)
        path = build(complete)
        self.builder.verify_bundle(path, complete['commit'], '0.1.0')
        incomplete = fixture(self.f['root'] / 'notice-missing', notice_material='missing')
        with self.assertRaisesRegex(ValueError, 'Unmapped native source notice'):
            build(incomplete)

    def setUp(self):
        self.assertTrue(SCRIPT.is_file(), "Corresponding-source builder is required")
        self.temp = tempfile.TemporaryDirectory(prefix="opentag-source-test-")
        self.addCleanup(self.temp.cleanup)
        self.f = fixture(self.temp.name)
        self.builder = load_builder()

    def test_complete_source_binds_git_versions_archives_and_seven_targets(self):
        path = build(self.f)
        manifest = self.builder.verify_bundle(path, self.f["commit"], "0.1.0", self.f["evidence"])
        self.assertEqual(manifest["schema"], 1)
        self.assertEqual(manifest["source_commit"], self.f["commit"])
        self.assertEqual(set(manifest["targets"]), set(TARGETS))
        with tarfile.open(path, "r:xz") as archive:
            names = archive.getnames()
        for suffix in ["app/Cargo.lock", "app/dist/index.html", "app/.cargo/opentag-offline.toml",
                       "provenance/version-stamp.patch", "native/archives/chromaprint-1.tar.gz",
                       "native/licenses/chromaprint/LICENSE"]:
            self.assertTrue(any(name.endswith("/" + suffix) for name in names), suffix)

    def test_fixture_source_hashes_survive_windows_text_newlines(self):
        write_text = Path.write_text

        def windows_write_text(path, data, *args, **kwargs):
            kwargs["newline"] = "\r\n"
            return write_text(path, data, *args, **kwargs)

        # Windows translates text-mode writes even when Git autocrlf is false.
        # Exercise the real Git export, recipe hashes and source verification.
        with patch.object(Path, "write_text", windows_write_text):
            f = fixture(self.f["root"] / "windows-text")
            path = build(f)
            manifest = self.builder.verify_bundle(path, f["commit"], "0.1.0", f["evidence"])
        self.assertEqual(manifest["source_commit"], f["commit"])
        self.assertEqual(set(manifest["targets"]), set(TARGETS))

    def test_offline_overlay_preserves_tracked_platform_linker_configuration(self):
        config = '# Project linker settings\n[target.x86_64-pc-windows-msvc]\nlinker = "rust-lld"\n'
        f = fixture(self.f['root'] / 'configured', cargo_config=config)
        try:
            path = build(f)
        except ValueError as error:
            self.fail(f'The source bundle must retain existing linker configuration: {error}')
        self.builder.verify_bundle(path, f['commit'], '0.1.0', f['evidence'])
        with tarfile.open(path, 'r:xz') as archive:
            files = {item.name.split('/', 1)[1]: archive.extractfile(item).read() for item in archive}
        self.assertEqual(files['app/.cargo/config.toml'], config.encode())
        self.assertIn(b'replace-with = "vendored-sources"', files['app/.cargo/opentag-offline.toml'])
        self.assertEqual((f['app'] / '.cargo/config.toml').read_bytes(), config.encode())
        rewrite_bundle(path, lambda name, data: data.replace(b'rust-lld', b'wrong-linker')
                       if name.endswith('/app/.cargo/config.toml') else data, rehash=True)
        with self.assertRaisesRegex(ValueError, 'Original source blob'):
            self.builder.verify_bundle(path, f['commit'], '0.1.0')

    def test_offline_overlay_cannot_be_changed_by_rehashing_the_outer_inventory(self):
        path = build(self.f)
        rewrite_bundle(path, lambda name, data: data.replace(b'vendored-sources', b'other-sources')
                       if name.endswith('/app/.cargo/opentag-offline.toml') else data, rehash=True)
        with self.assertRaisesRegex(ValueError, 'Offline Cargo source configuration differs'):
            self.builder.verify_bundle(path, self.f['commit'], '0.1.0')

    def test_wrong_source_or_version_and_missing_target_fail(self):
        path = build(self.f)
        for commit, version in [("b" * 40, "0.1.0"), (self.f["commit"], "0.1.1")]:
            with self.assertRaises(ValueError):
                self.builder.verify_bundle(path, commit, version)
        (self.f["evidence"] / "native-linux-arm64.json").unlink()
        with self.assertRaises(ValueError):
            build(self.f)

    def test_modified_app_bytes_fail_even_if_file_inventory_is_rehashed(self):
        path = build(self.f)
        rewrite_bundle(path, lambda name, data: b"fn main() { panic!(); }\n" if name.endswith("app/src-tauri/src/main.rs") else data, rehash=True)
        with self.assertRaises(ValueError):
            self.builder.verify_bundle(path, self.f["commit"], "0.1.0")

    def test_upstream_archive_and_native_recipe_tampering_fail(self):
        archive = self.f["archives"] / "chromaprint-1.tar.gz"
        archive.write_bytes(b"wrong upstream source")
        with self.assertRaises(ValueError):
            build(self.f)

    def test_collector_checks_real_prefix_files_not_only_manifest_claims(self):
        prefix = next((self.f["root"] / "native" / TARGETS["linux-x64"]).glob("*/prefix"))
        (prefix / "lib/libchromaprint.a").write_bytes(b"changed library")
        with self.assertRaises(ValueError):
            self.builder.collect_native(self.f["root"] / "native", TARGETS["linux-x64"],
                                        self.f["commit"], "0.1.0", self.f["root"] / "bad.json")

    def test_patch_metadata_cannot_lie_about_original_or_applied_configure(self):
        for key in ('original_sha', 'patched_sha'):
            with self.subTest(key=key):
                f = fixture(self.f['root'] / key, **{key: '0' * 64})
                with self.assertRaises(ValueError):
                    build(f)

    def test_patch_original_and_patched_bytes_are_independently_bound(self):
        path = build(self.f)
        def tamper(name, data):
            if name.endswith("native/targets/linux-x64.json"):
                wrapper = json.loads(data)
                native = json.loads(base64.b64decode(wrapper["manifest_base64"]))
                native["patch_metadata"][0]["patched_sha256"] = "0" * 64
                raw = (json.dumps(native) + "\n").encode()
                wrapper["manifest_base64"] = base64.b64encode(raw).decode()
                wrapper["manifest_sha256"] = digest(raw)
                return (json.dumps(wrapper) + "\n").encode()
            return data
        # Metadata must remain source-bound even if an outer inventory is rehashed.
        rewrite_bundle(path, tamper, rehash=True)
        with self.assertRaises(ValueError):
            self.builder.verify_bundle(path, self.f["commit"], "0.1.0")

    def test_untracked_source_injection_is_not_allowed_by_rehashing_inventory(self):
        path = build(self.f)
        with tarfile.open(path, "r:xz") as archive:
            rows = [(item, archive.extractfile(item).read()) for item in archive]
        prefix = rows[0][0].name.split('/')[0] + '/'
        info = tarfile.TarInfo(prefix + 'app/src-tauri/src/extra.rs')
        info.mode = 0o644
        rows.append((info, b'fn hidden() {}\n'))
        with tarfile.open(path, 'w:xz') as archive:
            for info, data in rows:
                if info.name.endswith('/source-manifest.json'):
                    manifest = json.loads(data)
                    manifest['files']['app/src-tauri/src/extra.rs'] = {'size': len(rows[-1][1]), 'sha256': digest(rows[-1][1]), 'mode': 0o644}
                    data = (json.dumps(manifest) + '\n').encode()
                info.size = len(data)
                archive.addfile(info, io.BytesIO(data))
        with self.assertRaises(ValueError):
            self.builder.verify_bundle(path, self.f['commit'], '0.1.0')

    def test_exact_prefix_collection_and_git_directory_named_oid(self):
        prefix = next((self.f['root'] / 'native' / TARGETS['linux-x64']).glob('*/prefix'))
        self.builder.collect_native(prefix, TARGETS['linux-x64'], self.f['commit'], '0.1.0', self.f['root'] / 'single.json')
        path = build(self.f)
        self.builder.verify_bundle(path, self.f['commit'], '0.1.0')

    def test_unsafe_tar_links_traversal_and_duplicates_fail_before_extraction(self):
        path = self.f['root'] / 'unsafe.tar.xz'
        prefix = 'TagGroove-v0.1.0-sources/'
        for kind in ('traversal', 'symlink', 'duplicate'):
            with self.subTest(kind=kind):
                with tarfile.open(path, 'w:xz') as archive:
                    info = tarfile.TarInfo(prefix + ('../escape' if kind == 'traversal' else 'app/file'))
                    info.mode = 0o644
                    if kind == 'symlink':
                        info.type, info.linkname = tarfile.SYMTYPE, '/tmp/escape'
                        archive.addfile(info)
                    else:
                        archive.addfile(info, io.BytesIO(b''))
                        if kind == 'duplicate':
                            archive.addfile(info, io.BytesIO(b''))
                with self.assertRaisesRegex(ValueError, 'Unsafe|Duplicate'):
                    self.builder.verify_bundle(path, self.f['commit'], '0.1.0')

    def test_explicit_archive_directory_never_falls_back_to_network(self):
        (self.f['archives'] / 'chromaprint-1.tar.gz').unlink()
        with patch.object(self.builder.urllib.request, 'urlopen', side_effect=AssertionError('network used')) as network:
            with self.assertRaises(FileNotFoundError):
                self.builder.build_bundle(self.f['app'], self.f['commit'], '0.1.0', self.f['root'] / 'bad.tar.xz',
                    self.f['evidence'], self.f['archives'], self.f['vendor'], self.f['dist'])
            network.assert_not_called()

    def test_cargo_vendor_requires_lock_checksum_and_every_file(self):
        lock, vendor = cargo_vendor_fixture()
        self.builder.check_vendor(lock, vendor)
        checksum = vendor['example-1.0.0/.cargo-checksum.json']
        for mutation in ({}, {**vendor, 'example-1.0.0/src/lib.rs': b'changed'},
                         {**vendor, 'example-1.0.0/.cargo-checksum.json': checksum.replace(b'a' * 64, b'b' * 64)}):
            with self.assertRaises(ValueError):
                self.builder.check_vendor(lock, mutation)

    def test_vendor_contract_rejects_empty_incomplete_injected_and_wrong_packages(self):
        lock, valid = cargo_vendor_fixture()
        checksum_path = 'example-1.0.0/.cargo-checksum.json'
        base = json.loads(valid[checksum_path])
        cases = {}
        cases['empty'] = {checksum_path: json.dumps({'package': base['package'], 'files': {}}).encode()}
        cases['missing_manifest'] = {
            checksum_path: json.dumps({'package': base['package'], 'files': {'src/lib.rs': digest(valid['example-1.0.0/src/lib.rs'])}}).encode(),
            'example-1.0.0/src/lib.rs': valid['example-1.0.0/src/lib.rs']}
        cases['injected'] = {**valid, 'example-1.0.0/build.rs': b'fn main() {}\n'}
        cases['unlisted_root_file'] = {**valid, 'unexpected.rs': b'fn hidden() {}\n'}
        duplicate = {'unexpected-1.0.0/' + name[len('example-1.0.0/'):]: data for name, data in valid.items()}
        cases['unlisted_duplicate_package'] = {**valid, **duplicate}
        empty_map = {**base, 'files': {**base['files'], 'src/empty.rs': digest(b'')}}
        cases['missing_empty_file'] = {**valid, checksum_path: json.dumps(empty_map).encode()}
        for field, value in [('name', 'other'), ('version', '2.0.0')]:
            cargo = valid['example-1.0.0/Cargo.toml'].replace((field + '="' + ('example' if field == 'name' else '1.0.0') + '"').encode(), (field + '="' + value + '"').encode())
            metadata = {**base, 'files': {**base['files'], 'Cargo.toml': digest(cargo)}}
            cases['wrong_' + field] = {**valid, 'example-1.0.0/Cargo.toml': cargo, checksum_path: json.dumps(metadata).encode()}
        for label, vendor in cases.items():
            with self.subTest(case=label), self.assertRaises(ValueError):
                self.builder.check_vendor(lock, vendor)

    def test_empty_compiler_source_closure_cannot_be_laundered_with_new_hashes(self):
        wrapper = json.loads((self.f['evidence'] / 'native-linux-x64.json').read_bytes())
        raw = b'{"libavcodec/pcm.d": {}}\n'
        native = json.loads(base64.b64decode(wrapper['manifest_base64']))
        native['prefix_files']['ffmpeg-source-closure.json'] = digest(raw)
        wrapper['files']['ffmpeg-source-closure.json'] = {'base64': base64.b64encode(raw).decode(), 'size': len(raw), 'sha256': digest(raw)}
        manifest = (json.dumps(native) + '\n').encode()
        wrapper.update(manifest_base64=base64.b64encode(manifest).decode(), manifest_sha256=digest(manifest))
        with self.assertRaisesRegex(ValueError, 'closure'):
            self.builder.native_manifest(wrapper, self.f['commit'], '0.1.0')

    def test_native_declared_profile_must_match_compiled_configuration(self):
        wrapper = json.loads((self.f['evidence'] / 'native-linux-x64.json').read_bytes())
        native = json.loads(base64.b64decode(wrapper['manifest_base64']))
        native['ffmpeg_decoders'] = ['different_decoder']
        raw = (json.dumps(native) + '\n').encode()
        wrapper.update(manifest_base64=base64.b64encode(raw).decode(), manifest_sha256=digest(raw))
        with self.assertRaisesRegex(ValueError, 'configured decoder'):
            self.builder.native_manifest(wrapper, self.f['commit'], '0.1.0')

    def test_network_enabled_configuration_is_rejected_even_with_matching_hashes(self):
        wrapper = json.loads((self.f['evidence'] / 'native-linux-x64.json').read_bytes())
        native = json.loads(base64.b64decode(wrapper['manifest_base64']))
        raw = base64.b64decode(wrapper['files']['ffmpeg-config.h']['base64']).replace(b'CONFIG_NETWORK 0', b'CONFIG_NETWORK 1')
        native['prefix_files']['ffmpeg-config.h'] = digest(raw)
        wrapper['files']['ffmpeg-config.h'] = {'base64': base64.b64encode(raw).decode(), 'size': len(raw), 'sha256': digest(raw)}
        manifest = (json.dumps(native) + '\n').encode()
        wrapper.update(manifest_base64=base64.b64encode(manifest).decode(), manifest_sha256=digest(manifest))
        with self.assertRaisesRegex(ValueError, 'CONFIG_NETWORK'):
            self.builder.native_manifest(wrapper, self.f['commit'], '0.1.0')

    def test_windows_crlf_configuration_is_accepted_with_its_exact_byte_hashes(self):
        wrapper = json.loads((self.f['evidence'] / 'native-linux-x64.json').read_bytes())
        native = json.loads(base64.b64decode(wrapper['manifest_base64']))
        for name in ('ffmpeg-config.h', 'ffmpeg-components.h'):
            raw = base64.b64decode(wrapper['files'][name]['base64']).replace(b'\n', b'\r\n')
            native['prefix_files'][name] = digest(raw)
            wrapper['files'][name] = {'base64': base64.b64encode(raw).decode(), 'size': len(raw), 'sha256': digest(raw)}
        manifest = (json.dumps(native) + '\n').encode()
        wrapper.update(manifest_base64=base64.b64encode(manifest).decode(), manifest_sha256=digest(manifest))
        self.builder.native_manifest(wrapper, self.f['commit'], '0.1.0')

    def test_stamping_preserves_original_commit_and_records_actual_diff(self):
        with self.assertRaises(ValueError):
            build(self.f, version="0.1.9")
        for path in self.f["evidence"].glob("native-*.json"):
            native = json.loads(path.read_text())
            native.update(version="0.1.9", tag="v0.1.9")
            path.write_text(json.dumps(native) + "\n")
        path = build(self.f, version="0.1.9")
        self.builder.verify_bundle(path, self.f["commit"], "0.1.9")
        with tarfile.open(path, "r:xz") as archive:
            package = next(item for item in archive if item.name.endswith("/app/package.json"))
            self.assertEqual(json.load(archive.extractfile(package))["version"], "0.1.9")
            patch = next(item for item in archive if item.name.endswith("/provenance/version-stamp.patch"))
            self.assertIn(b'+', archive.extractfile(patch).read())

    def test_stamping_matches_existing_release_implementation(self):
        paths = [p for p in self.builder.VERSION_FILES if (self.f['app'] / p).is_file()]
        original = {p: (self.f['app'] / p).read_bytes() for p in paths}
        script = SCRIPT.parent / 'release.mjs'
        subprocess.run(['node', '--input-type=module', '-e',
                        'const {stampVersion}=await import(process.argv[1]);await stampVersion(process.argv[2],process.argv[3]);',
                        script.as_uri(), str(self.f['app']), '0.1.9'], check=True)
        for path, data in original.items():
            self.assertEqual(self.builder.stamped(path, data, '0.1.9'), (self.f['app'] / path).read_bytes(), path)


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] in ('--release-linux-fixture', '--release-complete-fixture'):
        f, capture, signer, path = linux_distribution_fixture(Path(sys.argv[2]), load_builder(),
                                                             macos_signing=sys.argv[1] == '--release-complete-fixture')
        verifier = f['root']/'fixture-python'
        # Run the production verifier with only the trusted signer substituted
        # for this generated test repository. Never install this into CI or app code.
        verifier.write_text(f'#!{sys.executable}\n'+f'''import importlib.util, sys
spec=importlib.util.spec_from_file_location('source_builder',{str(SCRIPT)!r})
builder=importlib.util.module_from_spec(spec);spec.loader.exec_module(builder)
capture=builder.distribution_helper()
capture.material.UBUNTU_ARCHIVE_SIGNER={signer!r}
builder.distribution_helper=lambda:capture
sys.argv=sys.argv[1:]
try:builder.main()
except (OSError,ValueError,KeyError) as error:sys.exit(str(error))
''')
        verifier.chmod(0o755)
        print(json.dumps({'commit': f['commit'], 'bundle': str(path), 'evidence': str(f['evidence']),
                          'linux_evidence': str(f['root']/'linux-evidence'), 'python': str(verifier),
                          **({'macos_evidence': str(f['macos_evidence'])} if 'macos_evidence' in f else {})}))
    elif len(sys.argv) == 3 and sys.argv[1] == '--release-signed-fixture':
        f = fixture(sys.argv[2], macos_signing=True);path = build(f)
        print(json.dumps({'commit': f['commit'], 'bundle': str(path), 'evidence': str(f['evidence']),
                          'macos_evidence': str(f['macos_evidence'])}))
    elif len(sys.argv) == 3 and sys.argv[1] == "--release-fixture":
        f = fixture(sys.argv[2])
        path = build(f)
        print(json.dumps({"commit": f["commit"], "bundle": str(path), "evidence": str(f["evidence"])}))
    else:
        unittest.main()
