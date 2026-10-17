"""Corresponding-source integrity and offline-layout tests, using tiny local archives."""
import hashlib
import base64
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "audio-source-bundle.py"
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


def fixture(root, original_sha=None, patched_sha=None):
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
        "scripts/audio-deps/patches/ffmpeg.patch": "# source patch fixture\n",
        "LICENSE": "Application license fixture\n",
    }
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
                source = b"pcm source fixture\n"
                info = tarfile.TarInfo("ffmpeg-1/libavcodec/pcm.c")
                info.size = len(source)
                handle.addfile(info, io.BytesIO(source))
        components[name] = {"version": "1", "url": f"https://example.invalid/{archive.name}",
                            "archive": archive.name, "sha256": digest(archive.read_bytes()),
                            "directory": f"{name}-1", "license": "test-only", "license_files": ["LICENSE"]}
    patches = [{"file": "patches/ffmpeg.patch", "sha256": digest(files["scripts/audio-deps/patches/ffmpeg.patch"].encode()),
                "component": "ffmpeg", "target": "configure",
                "original_sha256": original_sha or digest(b"original upstream configure\n"),
                "patched_sha256": patched_sha or digest(b"patched upstream configure\n")}]
    inventory = {"format": 1, "components": components, "patches": patches,
                 "ffmpeg": {"decoders": ["pcm_s16le"], "demuxers": ["wav"]}}
    files["scripts/audio-deps/sources.json"] = json.dumps(inventory, indent=2) + "\n"
    for name, data in files.items():
        path = app / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(data)
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
                    "ffmpeg-generated/config.h": b"generated config fixture\n",
                    "ffmpeg-generated-closure.json": (json.dumps({"config.h": digest(b"generated config fixture\n")}) + "\n").encode(),
                    "ffmpeg-source-closure.json": (json.dumps({"libavcodec/pcm.d": {"libavcodec/pcm.c": digest(b"pcm source fixture\n")}}) + "\n").encode()}
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
    return {"root": root, "app": app, "archives": archives, "evidence": evidence,
            "vendor": vendor, "dist": dist, "commit": commit}


def build(f, output=None, version="0.1.0"):
    output = output or f["root"] / f"OpenTag-v{version}-sources.tar.xz"
    load_builder().build_bundle(f["app"], f["commit"], version, output,
                               f["evidence"], f["archives"], f["vendor"], f["dist"])
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


class SourceBundleTests(unittest.TestCase):
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
        for suffix in ["app/Cargo.lock", "app/dist/index.html", "app/.cargo/config.toml",
                       "provenance/version-stamp.patch", "native/archives/chromaprint-1.tar.gz",
                       "native/licenses/chromaprint/LICENSE"]:
            self.assertTrue(any(name.endswith("/" + suffix) for name in names), suffix)

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
        prefix = 'OpenTag-v0.1.0-sources/'
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
    if len(sys.argv) == 3 and sys.argv[1] == "--release-fixture":
        f = fixture(sys.argv[2])
        path = build(f)
        print(json.dumps({"commit": f["commit"], "bundle": str(path), "evidence": str(f["evidence"])}))
    else:
        unittest.main()
