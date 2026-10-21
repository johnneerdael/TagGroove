#!/usr/bin/env python3
"""Build/verify the version-bound source asset; never execute files from an archive."""
import argparse
import base64
import difflib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import urllib.request

_policy_spec = importlib.util.spec_from_file_location('ffmpeg_policy', Path(__file__).resolve().parent / 'audio-deps/ffmpeg_policy.py')
policy = importlib.util.module_from_spec(_policy_spec)
_policy_spec.loader.exec_module(policy)

TARGETS = {
    "windows-x64": "x86_64-pc-windows-msvc",
    "macos-arm64": "aarch64-apple-darwin",
    "macos-x64": "x86_64-apple-darwin",
    "linux-x64": "x86_64-unknown-linux-gnu",
    "linux-arm64": "aarch64-unknown-linux-gnu",
    "android-arm64": "aarch64-linux-android",
    "ios-arm64-simulator-preview": "aarch64-apple-ios-sim",
}
COMPONENTS = {"chromaprint", "ffmpeg", "zlib", "bzip2"}
OFFLINE_CARGO_CONFIG = "app/.cargo/opentag-offline.toml"
OFFLINE_CARGO_BYTES = b'[source.crates-io]\nreplace-with = "vendored-sources"\n[source.vendored-sources]\ndirectory = "vendor"\n'
VERSION_FILES = {"package.json", "package-lock.json", "Cargo.lock", "src-tauri/Cargo.toml",
                 "src-tauri/tauri.conf.json", "src-tauri/gen/apple/project.yml",
                 "src-tauri/gen/apple/opentag_iOS/Info.plist",
                 "src-tauri/gen/apple/opentag.xcodeproj/project.pbxproj"}
MAX_ARCHIVE_BYTES = 4 * 1024**3
MAX_ARCHIVE_MEMBER_BYTES = 512 * 1024 * 1024
MAX_ARCHIVE_FILES = 100000


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def archive_budget(sizes, *, count=0, total=0):
    """Apply identical expanded-size/member/count limits before writing and reading."""
    for size in sizes:
        require(type(size) is int and 0 <= size <= MAX_ARCHIVE_MEMBER_BYTES,
                f"Source archive member bytes {size} exceed bounds 0..{MAX_ARCHIVE_MEMBER_BYTES}")
        count += 1;total += size
        require(total <= MAX_ARCHIVE_BYTES,
                f"Source archive expanded bytes {total} exceed {MAX_ARCHIVE_BYTES}")
        require(count <= MAX_ARCHIVE_FILES,
                f"Source archive member count {count} exceeds {MAX_ARCHIVE_FILES}")
    return count, total


def load(data):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, f"Duplicate JSON key: {key}")
            result[key] = value
        return result
    return json.loads(data, object_pairs_hook=unique)


def encode(value):
    return (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode()


def safe(name):
    path = PurePosixPath(name)
    require(isinstance(name, str) and bool(name) and not path.is_absolute()
            and path.parts and ".." not in path.parts and str(path) == name and "\\" not in name,
            f"Unsafe source path: {name}")
    return name


def identity(commit, version):
    require(re.fullmatch(r"[a-f0-9]{40}", commit or ""), "A full source SHA is required")
    match = re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?", version or "")
    require(match and not any(re.fullmatch(r"0\d+", part) for part in (match[4] or "").split(".")),
            "Invalid application version")


def compiler_closure(wrapper, native):
    entry = wrapper.get("files", {}).get("ffmpeg-source-closure.json")
    require(entry is not None, "Missing FFmpeg compiler source closure")
    closure = load(base64.b64decode(entry["base64"], validate=True))
    require(isinstance(closure, dict) and closure, "Empty FFmpeg compiler source closure")
    sources = {}
    for depfile, files in closure.items():
        safe(depfile)
        require(depfile.endswith(".d") and isinstance(files, dict) and files, "Empty or invalid compiler source closure record")
        for name, digest in files.items():
            safe(name)
            require(re.fullmatch(r"[a-f0-9]{64}", digest or ""), "Invalid compiler source closure hash")
            require(name not in sources or sources[name] == digest, "Conflicting compiler source closure hashes")
            sources[name] = digest
    decoders = set(native.get("ffmpeg_decoders", []))
    required = set()
    if "ahx" in decoders:
        required.update({"libavcodec/bsf/ahx_to_mp2.c", "libavcodec/mpegaudiodec_float.c"})
    config = base64.b64decode(wrapper["files"]["ffmpeg-components.h"]["base64"], validate=True).decode()
    if re.search(r"^#define CONFIG_AHX_PARSER 1\s*$", config, re.M):
        required.add("libavcodec/ahx_parser.c")
    if "mp3float" in decoders:
        required.add("libavcodec/mpegaudiodec_float.c")
    gated_adpcm = {name for patch in native.get("patch_metadata", []) for name in patch.get("audio_components", []) if name.startswith("adpcm_")}
    if decoders & gated_adpcm:
        required.add("libavcodec/adpcm.c")
    require(required.issubset(sources), "Compiler source closure omits enabled LGPL audio implementation")
    policy.verify_no_gpl_sources(sources)
    return sources


def native_manifest(wrapper, commit, version, inventory=None):
    identity(commit, version)
    platform = wrapper.get("platform")
    require(wrapper.get("schema") == 1 and platform in TARGETS
            and wrapper.get("target") == TARGETS[platform]
            and wrapper.get("commit") == commit and wrapper.get("version") == version
            and wrapper.get("tag") == "v" + version, "Native evidence source/version/target mismatch")
    raw = base64.b64decode(wrapper["manifest_base64"], validate=True)
    require(sha(raw) == wrapper.get("manifest_sha256"), "Native manifest hash mismatch")
    native = load(raw)
    require(native.get("format") == 1 and native.get("target") == TARGETS[platform]
            and re.fullmatch(r"[a-f0-9]{64}", native.get("recipe_sha256", "")), "Invalid native build manifest")
    require(set(native.get("components", {})) == COMPONENTS, "Incomplete native source components")
    require(native.get("runtime_programs") == [] and native.get("decoder_protocols") == [],
            "Native backend must not depend on runtime programs or network protocols")
    require(native.get("build_inputs") and native.get("static_libraries") and native.get("prefix_files"),
            "Native build lacks input/library/prefix provenance")
    require(sha(json.dumps(native["build_inputs"], sort_keys=True).encode() + native["target"].encode())
            == native["recipe_sha256"], "Native recipe digest differs from its actual inputs")
    if inventory is not None:
        require(native["components"] == inventory["components"], "Native component source pins differ")
        require(native.get("patch_metadata") == inventory.get("patches", []), "Native patch metadata differs")
        require(native.get("ffmpeg_decoders") == inventory["ffmpeg"]["decoders"]
                and native.get("ffmpeg_demuxers") == inventory["ffmpeg"]["demuxers"], "Native decoder profile differs")
    libs = native["static_libraries"]
    require(all(name.endswith((".a", ".lib")) and native["prefix_files"].get(name) == digest
                for name, digest in libs.items()), "Static library inventory differs")
    expected = set(native["prefix_files"]) - set(libs)
    require(set(wrapper.get("files", {})) == expected, "Missing or unexpected native source evidence")
    for name, entry in wrapper["files"].items():
        safe(name)
        data = base64.b64decode(entry["base64"], validate=True)
        require(sha(data) == entry.get("sha256") == native["prefix_files"][name]
                and len(data) == entry.get("size"), f"Native evidence hash mismatch: {name}")
    config = base64.b64decode(wrapper["files"]["ffmpeg-config.h"]["base64"], validate=True).decode()
    for name, digest in native["prefix_files"].items():
        safe(name)
        require(re.fullmatch(r"[a-f0-9]{64}", digest or ""), "Invalid native prefix hash")
    components = base64.b64decode(wrapper["files"]["ffmpeg-components.h"]["base64"], validate=True).decode()
    for group, declared in (("DECODER", native.get("ffmpeg_decoders")), ("DEMUXER", native.get("ffmpeg_demuxers")),
                            ("PROTOCOL", native.get("decoder_protocols"))):
        enabled = {name.lower() for name in re.findall(rf"^#define CONFIG_([A-Z0-9_]+)_{group} 1\s*$", components, re.M)}
        require(isinstance(declared, list) and len(declared) == len(set(declared)) and enabled == set(declared),
                f"Native configured {group.lower()} inventory differs")
    compiler_closure(wrapper, native)
    generated_entry = wrapper["files"].get("ffmpeg-generated-closure.json")
    require(generated_entry is not None, "Missing generated compiler input closure")
    generated = load(base64.b64decode(generated_entry["base64"], validate=True))
    require(isinstance(generated, dict) and generated, "Empty generated compiler input closure")
    require({name for name in wrapper["files"] if name.startswith("ffmpeg-generated/")} ==
            {"ffmpeg-generated/" + safe(name) for name in generated}, "Generated compiler input inventory differs")
    for name, digest in generated.items():
        require(native["prefix_files"].get("ffmpeg-generated/" + name) == digest,
                "Generated compiler input hash differs")
    for flag in ("GPL", "NONFREE", "VERSION3", "NETWORK"):
        require(re.search(rf"^#define CONFIG_{flag} 0\s*$", config, re.M), f"Forbidden FFmpeg CONFIG_{flag}")
    return native


def collect_native(build_root, target, commit, version, output):
    identity(commit, version)
    require(target in TARGETS.values(), "Unexpected native target")
    build_root = Path(build_root)
    exact = build_root / "opentag-audio-manifest.json"
    matches = [exact] if exact.is_file() else list((build_root / target).glob("*/prefix/opentag-audio-manifest.json"))
    require(len(matches) == 1, "Expected exactly one native build prefix for target")
    manifest = matches[0]
    prefix = manifest.parent
    raw = manifest.read_bytes()
    native = load(raw)
    expected = native.get("prefix_files", {})
    actual = {path.relative_to(prefix).as_posix() for path in prefix.rglob("*")
              if path.is_file() and path != manifest}
    require(actual == set(expected), "Native prefix file set differs from its manifest")
    files = {}
    for name, digest in expected.items():
        safe(name)
        path = prefix / name
        require(path.resolve().is_relative_to(prefix.resolve()) and not path.is_symlink(), "Unsafe native prefix entry")
        data = path.read_bytes()
        require(sha(data) == digest, f"Native installed file hash mismatch: {name}")
        if name not in native.get("static_libraries", {}):
            files[name] = {"sha256": digest, "size": len(data), "base64": base64.b64encode(data).decode()}
    platform = next(key for key, value in TARGETS.items() if value == target)
    wrapper = {"schema": 1, "tag": "v" + version, "version": version, "commit": commit,
               "platform": platform, "target": target, "manifest_sha256": sha(raw),
               "manifest_base64": base64.b64encode(raw).decode(), "files": files}
    native_manifest(wrapper, commit, version)
    output = Path(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(encode(wrapper))
    return wrapper


def git_object(kind, data):
    return hashlib.sha1(f"{kind} {len(data)}\0".encode() + data).hexdigest()


def tree_hash(entries):
    tree = {}
    for name, item in entries.items():
        parts = safe(name).split("/")
        node = tree
        for part in parts[:-1]:
            node = node.setdefault(part, {})
        node[parts[-1]] = (item["mode"], item["oid"])
    def visit(node):
        rows = []
        for name, value in node.items():
            directory = isinstance(value, dict)
            mode, oid = ("40000", visit(value)) if directory else value
            rows.append((name.encode() + (b"/" if directory else b""), mode.encode() + b" " + name.encode() + b"\0" + bytes.fromhex(oid)))
        return git_object("tree", b"".join(row for _, row in sorted(rows)))
    return visit(tree)


def stamped(name, data, version):
    if name not in VERSION_FILES:
        return data
    text = data.decode()
    if name.endswith(".json"):
        obj = load(text)
        obj["version"] = version
        if name == "package-lock.json":
            require("" in obj.get("packages", {}), "Lockfile lacks root package")
            obj["packages"][""]["version"] = version
        return encode(obj)
    if name == "src-tauri/Cargo.toml":
        section = re.search(r"^\[package\][\s\S]*?(?=^\[|\Z)", text, re.M)
        require(section is not None, "Missing application package")
        changed, count = re.subn(r'^(version\s*=\s*["\x27])[^"\x27]+(["\x27])',
                                lambda m: m[1] + version + m[2], section[0], flags=re.M)
        require(count == 1, "Ambiguous application version")
        return (text[:section.start()] + changed + text[section.end():]).encode()
    if name == "Cargo.lock":
        text, count = re.subn(r'(\[\[package\]\]\r?\nname = "opentag"\r?\nversion = ")[^"]+("),?',
                              lambda m: m[1] + version + m[2], text)
        require(count == 1, "Missing locked application version")
        return text.encode()
    apple = version.split("-")[0].split("+")[0]
    if name.endswith("project.yml"):
        for key in ("CFBundleShortVersionString", "CFBundleVersion"):
            text, count = re.subn(rf"^(\s*{key}:\s*).*$", lambda m: m[1] + f'"{apple}"', text, flags=re.M)
            require(count == 1, f"Missing Apple {key}")
    elif name.endswith("Info.plist"):
        for key in ("CFBundleShortVersionString", "CFBundleVersion"):
            text, count = re.subn(rf"(<key>{key}</key>\s*<string>)[^<]*(</string>)", lambda m: m[1] + apple + m[2], text)
            require(count == 1, f"Missing Apple {key}")
    return text.encode()


def stamp_patch(originals, files):
    return "".join("".join(difflib.unified_diff(data.decode().splitlines(keepends=True),
                  files["app/" + name].decode().splitlines(keepends=True), fromfile="a/" + name, tofile="b/" + name))
                   for name, data in sorted(originals.items())).encode()


def source_files(root, commit, version):
    run = lambda *args: subprocess.check_output(["git", "-c", "core.hooksPath=/dev/null", *args], cwd=root)
    require(run("rev-parse", "HEAD").decode().strip() == commit, "Checkout differs from full source SHA")
    commit_data = run("cat-file", "commit", commit)
    entries, files, originals = {}, {}, {}
    for row in run("ls-tree", "-rz", "--full-tree", commit).split(b"\0"):
        if not row:
            continue
        head, name = row.split(b"\t", 1)
        mode, kind, oid = head.decode().split()
        name = name.decode()
        safe(name)
        require(kind == "blob" and mode in {"100644", "100755"}, "Source bundle requires ordinary tracked files")
        data = run("cat-file", "blob", oid)
        expected = stamped(name, data, version)
        current = (root / name).read_bytes()
        require(current in (data, expected), f"Unexpected tracked source modification: {name}")
        entries[name] = {"mode": mode, "oid": oid}
        files["app/" + name] = expected
        if expected != data:
            originals[name] = data
            files["provenance/originals/" + name] = data
    files["provenance/commit.txt"] = commit_data
    files["provenance/tree.json"] = encode(entries)
    files["provenance/version-stamp.patch"] = stamp_patch(originals, files)
    return files, entries


def check_vendor(lock_data, vendor_files):
    lock = tomllib.loads(lock_data.decode())
    inventories = {}
    for name in vendor_files:
        directory, separator, relative = name.partition("/")
        require(separator and relative, f"Unexpected Cargo vendor root file: {name}")
        safe(name)
        inventories.setdefault(directory, set()).add(relative)
    expected_directories = set()
    for package in lock.get("package", []):
        source = package.get("source", "")
        if not source:
            continue
        require(source.startswith("registry+"), "Unsupported non-registry Cargo source; include an explicit vendor configuration")
        directory = f'{package["name"]}-{package["version"]}'
        expected_directories.add(directory)
        checksum_path = directory + "/.cargo-checksum.json"
        require(checksum_path in vendor_files, f"Missing vendored Cargo package: {directory}")
        checksums = load(vendor_files[checksum_path])
        require(checksums.get("package") == package.get("checksum"), f"Cargo package checksum differs: {directory}")
        files = checksums.get("files")
        require(isinstance(files, dict) and files and "Cargo.toml" in files,
                f"Missing or empty Cargo checksum file map/manifest: {directory}")
        require(inventories.get(directory) == set(files) | {".cargo-checksum.json"},
                f"Vendored Cargo file inventory differs: {directory}")
        for name, digest in files.items():
            safe(name)
            require(sha(vendor_files[directory + "/" + name]) == digest,
                    f"Vendored Cargo file differs: {directory}/{name}")
        metadata = tomllib.loads(vendor_files[directory + "/Cargo.toml"].decode()).get("package")
        require(isinstance(metadata, dict) and metadata.get("name") == package["name"]
                and metadata.get("version") == package["version"], f"Vendored Cargo package identity differs: {directory}")
    require(set(inventories) == expected_directories, "Unexpected or missing vendored Cargo package directory")


def add_directory(files, source, prefix, modes):
    source = Path(source)
    require(source.is_dir(), f"Missing source directory: {source}")
    for path in sorted(source.rglob("*")):
        if path.is_file():
            require(not path.is_symlink(), "Symlink in generated source directory")
            name = prefix + "/" + safe(path.relative_to(source).as_posix())
            files[name] = path.read_bytes()
            modes[name] = 0o755 if path.stat().st_mode & 0o111 else 0o644


def verify_upstream_closure(archive, directory, expected, patched_sources=None):
    # A streaming pass avoids repeatedly seeking/re-decompressing a large xz archive.
    found = set()
    prefix = safe(directory) + "/"
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r|*") as handle:
        for member in handle:
            if not member.name.startswith(prefix):
                continue
            name = member.name[len(prefix):]
            if name in expected:
                require(name not in found and member.isfile(), "Invalid or duplicate compiler source archive entry")
                original = handle.extractfile(member).read()
                patched = (patched_sources or {}).get(name)
                if patched is not None:
                    entry, diff = patched
                    original = policy.apply_patch(entry, original, diff)
                require(sha(original) == expected[name],
                        "Compiler source closure differs from pinned upstream archive")
                found.add(name)
    require(found == set(expected), "Compiler source closure file missing from pinned upstream archive")


def verify_contents(files, manifest, commit, version, native_evidence=None, linux_evidence=None, macos_evidence=None):
    identity(commit, version)
    require(manifest.get("schema") == 1 and manifest.get("version") == version
            and manifest.get("source_commit") == commit, "Source package source/version mismatch")
    require(set(manifest.get("targets", {})) == set(TARGETS), "Source package requires all seven native targets")
    entries = load(files["provenance/tree.json"])
    commit_data = files["provenance/commit.txt"]
    require(git_object("commit", commit_data) == commit, "Original Git commit proof differs")
    require(commit_data.splitlines()[0] == ("tree " + tree_hash(entries)).encode(), "Original Git tree proof differs")
    expected_files = {"app/" + name for name in entries} | {
        "provenance/tree.json", "provenance/commit.txt", "provenance/version-stamp.patch",
        OFFLINE_CARGO_CONFIG}
    originals = {}
    for name, entry in entries.items():
        safe(name)
        original = files.get("provenance/originals/" + name, files["app/" + name])
        require(git_object("blob", original) == entry["oid"], f"Original source blob differs: {name}")
        require(stamped(name, original, version) == files["app/" + name], f"Stamped source differs: {name}")
        require(manifest["files"]["app/" + name]["mode"] == (0o755 if entry["mode"] == "100755" else 0o644),
                f"Tracked source executable mode differs: {name}")
        if "provenance/originals/" + name in files:
            require(original != files["app/" + name], "Unexpected unchanged source original")
            expected_files.add("provenance/originals/" + name)
            originals[name] = original
    require(stamp_patch(originals, files) == files["provenance/version-stamp.patch"], "Version stamp diff differs")
    inventory = load(files["app/scripts/audio-deps/sources.json"])
    require(inventory.get("format") == 1 and set(inventory["components"]) == COMPONENTS, "Invalid native source inventory")
    for name, component in inventory["components"].items():
        archive_name = safe(component["archive"])
        expected_files.add("native/archives/" + archive_name)
        expected_files.update(f"native/licenses/{name}/{license_name}" for license_name in component["license_files"])
        archive = files["native/archives/" + archive_name]
        require(sha(archive) == component["sha256"], f"Original upstream archive differs: {name}")
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:*") as handle:
            for license_name in component["license_files"]:
                safe(license_name)
                member = handle.getmember(component["directory"] + "/" + license_name)
                require(member.isfile(), "License must be a regular upstream file")
                require(handle.extractfile(member).read() == files[f"native/licenses/{name}/{license_name}"], "License bytes differ")
    policy.validate_patches(inventory.get("patches", []))
    patched_sources, patched_outputs = {}, {}
    for patch in inventory.get("patches", []):
        component = inventory["components"][patch["component"]]
        with tarfile.open(fileobj=io.BytesIO(files["native/archives/" + component["archive"]]), mode="r:*") as handle:
            member = handle.getmember(component["directory"] + "/" + safe(patch["target"]))
            require(member.isfile(), "Patch original must be a regular upstream file")
            original = handle.extractfile(member).read()
            diff = files["app/scripts/audio-deps/" + safe(patch["file"])]
            patched_outputs[patch["target"]] = policy.apply_patch(patch, original, diff)
            patched_sources[patch["target"]] = (patch, diff)
    closure_sources = {}
    for platform, target in TARGETS.items():
        path = "native/targets/" + platform + ".json"
        expected_files.add(path)
        raw = files[path]
        require(manifest["targets"][platform] == {"target": target, "manifest_sha256": sha(raw)}, "Target source manifest binding differs")
        wrapper = load(raw)
        native = native_manifest(wrapper, commit, version, inventory)
        for name, digest in compiler_closure(wrapper, native).items():
            require(name not in closure_sources or closure_sources[name] == digest, "Cross-target compiler source closure differs")
            closure_sources[name] = digest
        inputs = native["build_inputs"]
        required_inputs = {"build.py", "sources.json", "ffmpeg_policy.py", "bzip2/CMakeLists.txt"} | {patch["file"] for patch in inventory.get("patches", [])}
        require(set(inputs) == required_inputs, "Native recipe inputs are incomplete or unexpected")
        for name, digest in inputs.items():
            safe(name)
            require(sha(files["app/scripts/audio-deps/" + name]) == digest, "Native recipe/source input differs")
        for patch in native.get("patch_metadata", []):
            require(inputs.get(patch["file"]) == patch["sha256"], "Native patch source hash differs")
            patched = base64.b64decode(wrapper["files"][policy.patched_path(patch["target"])]["base64"], validate=True)
            require(patched == patched_outputs[patch["target"]], "Applied native patch output differs from exact replay")
        if native_evidence is not None:
            require((Path(native_evidence) / f"native-{platform}.json").read_bytes() == raw, "Native target evidence differs from built packages")
    ffmpeg = inventory["components"]["ffmpeg"]
    verify_upstream_closure(files["native/archives/" + ffmpeg["archive"]], ffmpeg["directory"], closure_sources, patched_sources)
    notice_path = "app/scripts/notices/component-material.json"
    if notice_path in files:
        notice_spec = importlib.util.spec_from_file_location("component_notices", Path(__file__).with_name("component-notices.py"))
        notices = importlib.util.module_from_spec(notice_spec)
        notice_spec.loader.exec_module(notices)
        notice_data = load(files[notice_path])
        require(notice_data["bindings"]["ffmpeg"] == ffmpeg, "Native notice source pins differ")
        notices.verify_ffmpeg_coverage(notice_data, files["native/archives/" + ffmpeg["archive"]],
                                       ffmpeg["directory"], closure_sources, lambda name: files["app/" + safe(name)])
    runtime_path = 'app/scripts/notices/linux-runtime-material.json'
    if runtime_path in files:
        expected_files.update(runtime_material_helper().verify_files(files, load(files[runtime_path])))
    linux_files, linux_targets = verify_linux_distribution(files, commit, version, linux_evidence)
    expected_files.update(linux_files)
    require(manifest.get('linux_distribution', {}) == linux_targets, 'Linux distribution summary differs')
    macos_files, macos_targets = verify_macos_signing(files, commit, version, macos_evidence)
    expected_files.update(macos_files)
    require(manifest.get('macos_signing', {}) == macos_targets, 'macOS signing summary differs')
    vendor = {name.removeprefix("app/vendor/"): data for name, data in files.items() if name.startswith("app/vendor/")}
    expected_files.update(name for name in files if name.startswith(("app/vendor/", "app/dist/")))
    require(set(files) == expected_files, "Untracked or unexpected source archive file")
    check_vendor(files["app/Cargo.lock"], vendor)
    require(files.get("app/dist/index.html"), "Offline Rust relink requires the built frontend")
    require(files.get(OFFLINE_CARGO_CONFIG) == OFFLINE_CARGO_BYTES, "Offline Cargo source configuration differs")
    return manifest


def runtime_material_helper():
    # Use checked-out trusted verifier code, never Python from a source asset.
    spec = importlib.util.spec_from_file_location('linux_runtime_material', Path(__file__).with_name('linux-runtime-material.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def distribution_helper():
    spec = importlib.util.spec_from_file_location('linux_distribution_capture', Path(__file__).with_name('linux-distribution-capture.py'))
    module = importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    return module


def macos_helper():
    spec = importlib.util.spec_from_file_location('macos_release', Path(__file__).with_name('macos-release.py'))
    module = importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    return module


def verify_macos_signing(files, commit, version, evidence=None):
    policy_path = 'app/src-tauri/macos/signing-policy.json'
    if policy_path not in files:
        require(not any(name.startswith('macos-signing/') for name in files), 'macOS signing input has no Git-bound policy')
        if evidence is not None:
            require(not list(Path(evidence).glob('macos-signing-*.json')), 'Unbound external macOS signing evidence')
        return set(), {}
    identifier = load(files['app/src-tauri/tauri.conf.json'])['identifier']
    helper = macos_helper();required, targets = set(), {}
    for platform in ['macos-arm64', 'macos-x64']:
        path = 'macos-signing/targets/'+platform+'.json'
        require(path in files, 'Missing required macOS signing evidence: '+platform)
        raw = files[path];receipt = load(raw)
        helper.validate_receipt(receipt, files[policy_path], commit, version, platform, identifier)
        if evidence is not None:
            require((Path(evidence)/('macos-signing-'+platform+'.json')).read_bytes() == raw,
                    'macOS signing evidence differs from built packages')
        required.add(path)
        targets[platform] = {key: receipt[key] for key in ['target', 'identity', 'team_id', 'policy_sha256', 'artifacts']}
        targets[platform]['manifest_sha256'] = sha(raw)
    return required, targets


def verify_linux_git_inputs(files, capture, objects):
    """Bind already-verified builder observations to the archive's proven Git tree."""
    def equal(source, record):
        safe(source)
        raw = objects[record['sha256']]
        require(files.get('app/'+source) == raw, 'Linux builder input differs from Git-bound source: '+source)
    for source, record in capture['builder_inputs'].items():equal(source, record)
    recipes = load(files['app/scripts/notices/linux-packager-recipes.json'])['sources']
    require(len({record['id'] for record in recipes}) == len(recipes)
            and {record['id'] for record in recipes} == set(capture['builder_recipes']),
            'Linux builder recipe source set differs')
    for record in recipes:equal(record['source_file'], capture['builder_recipes'][record['id']])
    for record in capture['files'].values():
        if record['kind'] == 'repository-bytes':equal(record['source_file'], record)


def verify_linux_distribution(files, commit, version, evidence=None):
    if 'app/scripts/notices/linux-packager-recipes.json' not in files:
        require(not any(name.startswith('linux-distribution/') for name in files),
                'Linux distribution input has no Git-bound recipe')
        if evidence is not None:
            require(not list(Path(evidence).glob('linux-source-*.json')), 'Unbound external Linux distribution evidence')
        return set(), {}
    require('app/scripts/notices/linux-runtime-material.json' in files, 'Linux distribution requires a runtime descriptor')
    helper = distribution_helper()
    objects = {name.removeprefix('linux-distribution/objects/'): raw for name, raw in files.items()
               if name.startswith('linux-distribution/objects/')}
    required, targets = set(), {}
    runtime = load(files['app/scripts/notices/linux-runtime-material.json'])
    for platform in ('linux-x64', 'linux-arm64'):
        path = 'linux-distribution/targets/'+platform+'.json'
        require(path in files, 'Missing Linux distribution evidence: '+platform)
        raw = files[path];capture = load(raw)
        selected = helper.verify_materials(capture, objects, runtime, commit, version, platform)
        verify_linux_git_inputs(files, capture, objects)
        required.add(path);required.update('linux-distribution/objects/'+digest for digest in selected)
        if evidence is not None:
            require((Path(evidence)/('linux-source-'+platform+'.json')).read_bytes() == raw,
                    'Linux distribution evidence differs from built packages')
        targets[platform] = {'target': capture['target'], 'manifest_sha256': sha(raw),
                             'appimage_sha256': capture['appimage_sha256'], 'appimage_size': capture['appimage_size']}
    return required, targets


def build_bundle(root, commit, version, output, native_evidence=None, source_dir=None, vendor_dir=None, dist_dir=None,
                 linux_runtime_sources=None, linux_evidence=None, macos_evidence=None):
    root, output = Path(root), Path(output)
    identity(commit, version)
    native_evidence = Path(native_evidence or output.parent)
    files, entries = source_files(root, commit, version)
    modes = {"app/" + name: 0o755 if entry["mode"] == "100755" else 0o644 for name, entry in entries.items()}
    inventory = load(files["app/scripts/audio-deps/sources.json"])
    require(set(inventory["components"]) == COMPONENTS, "Native inventory requires four components")
    source_dir = Path(source_dir or os.environ["OPENTAG_AUDIO_SOURCE_DIR"]) if source_dir or "OPENTAG_AUDIO_SOURCE_DIR" in os.environ else None
    for name, component in inventory["components"].items():
        filename = safe(component["archive"])
        if source_dir is not None:
            archive = (source_dir / filename).read_bytes()
        else:
            require(component["url"].startswith("https://"), "Native archive URL must use HTTPS")
            with urllib.request.urlopen(component["url"], timeout=120) as response:
                archive = response.read()
        require(sha(archive) == component["sha256"], f"Upstream archive hash mismatch: {filename}")
        files["native/archives/" + filename] = archive
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:*") as handle:
            for license_name in component["license_files"]:
                safe(license_name)
                item = handle.getmember(component["directory"] + "/" + license_name)
                require(item.isfile(), "License must be an original regular file")
                files[f"native/licenses/{name}/{license_name}"] = handle.extractfile(item).read()
    targets = {}
    for platform, target in TARGETS.items():
        path = native_evidence / f"native-{platform}.json"
        require(path.is_file(), f"Missing required native evidence: {platform}")
        raw = path.read_bytes()
        native_manifest(load(raw), commit, version, inventory)
        files["native/targets/" + platform + ".json"] = raw
        targets[platform] = {"target": target, "manifest_sha256": sha(raw)}
    with tempfile.TemporaryDirectory(prefix="opentag-cargo-source-") as temp:
        if vendor_dir is None:
            vendor_dir = Path(temp) / "vendor"
            subprocess.run(["cargo", "vendor", "--locked", "--versioned-dirs", str(vendor_dir)],
                           cwd=root, check=True, stdout=subprocess.PIPE)
        add_directory(files, vendor_dir, "app/vendor", modes)
    require(OFFLINE_CARGO_CONFIG.removeprefix("app/") not in entries, "Reserved offline Cargo configuration already exists")
    files[OFFLINE_CARGO_CONFIG] = OFFLINE_CARGO_BYTES
    add_directory(files, dist_dir or root / "dist", "app/dist", modes)
    if 'app/scripts/notices/linux-runtime-material.json' in files:
        files.update(runtime_material_helper().source_files(root, linux_runtime_sources))
    else:
        require(linux_runtime_sources is None, 'Runtime source input supplied without a tracked runtime descriptor')
    if 'app/scripts/notices/linux-packager-recipes.json' in files:
        require(linux_evidence is not None, 'Missing Linux distribution evidence directory')
        require('app/scripts/notices/linux-runtime-material.json' in files, 'Linux distribution requires a runtime descriptor')
        files.update(distribution_helper().source_files(Path(linux_evidence), commit, version,
                     load(files['app/scripts/notices/linux-runtime-material.json'])))
    else:
        require(linux_evidence is None, 'Linux distribution input supplied without a tracked recipe')
    _linux_files, linux_targets = verify_linux_distribution(files, commit, version, linux_evidence)
    if 'app/src-tauri/macos/signing-policy.json' in files:
        require(macos_evidence is not None, 'Missing required macOS signing evidence directory')
        helper = macos_helper()
        for platform in ['macos-arm64', 'macos-x64']:
            files['macos-signing/targets/'+platform+'.json'] = helper.bounded(
                Path(macos_evidence)/('macos-signing-'+platform+'.json'), 1024*1024)
    else:
        require(macos_evidence is None, 'macOS signing input supplied without a tracked policy')
    _macos_files, macos_targets = verify_macos_signing(files, commit, version, macos_evidence)
    manifest = {"schema": 1, "version": version, "source_commit": commit, "targets": targets,
                "files": {name: {"size": len(data), "sha256": sha(data), "mode": modes.get(name, 0o644)} for name, data in sorted(files.items())}}
    if linux_targets:manifest['linux_distribution'] = linux_targets
    if macos_targets:manifest['macos_signing'] = macos_targets
    manifest_raw = encode(manifest)
    count, total = archive_budget(len(data) for data in files.values())
    count, total = archive_budget([len(manifest_raw)], count=count, total=total)
    print(f"Source archive preflight: {count} members, {total} expanded bytes (limit {MAX_ARCHIVE_BYTES})", file=sys.stderr, flush=True)
    print("Source archive: verifying original source and dependency evidence before compression", file=sys.stderr, flush=True)
    verify_contents(files, manifest, commit, version, native_evidence, linux_evidence, macos_evidence)
    print("Source archive: integrity checks passed; starting XZ compression", file=sys.stderr, flush=True)
    prefix = "TagGroove-v" + version + "-sources/"
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_suffix(output.suffix + ".tmp")
    try:
        written, next_progress = 0, 256 * 1024**2
        with tarfile.open(temporary, "w:xz", preset=1) as archive:
            for name, data in sorted({**files, "source-manifest.json": manifest_raw}.items()):
                info = tarfile.TarInfo(prefix + safe(name))
                info.size = len(data)
                info.mode = modes.get(name, 0o644)
                archive.addfile(info, io.BytesIO(data))
                written += len(data)
                if written >= next_progress:
                    print(f"Source archive: compressed input {written}/{total} bytes", file=sys.stderr, flush=True)
                    next_progress = (written // (256 * 1024**2) + 1) * 256 * 1024**2
        print(f"Source archive: compression complete ({temporary.stat().st_size} bytes); independently verifying archive", file=sys.stderr, flush=True)
        # Independent verification reloads the archive; do not retain another
        # complete expanded copy while it checks the same multi-GiB sources.
        files.clear()
        verify_bundle(temporary, commit, version, native_evidence, linux_evidence, macos_evidence)
        temporary.replace(output)
        print("Source archive: independent verification passed; archive ready", file=sys.stderr, flush=True)
    finally:
        temporary.unlink(missing_ok=True)
    return manifest


def verify_bundle(path, commit, version, native_evidence=None, linux_evidence=None, macos_evidence=None):
    identity(commit, version)
    prefix = "TagGroove-v" + version + "-sources/"
    files, modes, total, count = {}, {}, 0, 0
    print(f"Source archive: reading and checking {Path(path).stat().st_size} compressed bytes", file=sys.stderr, flush=True)
    next_progress = 512 * 1024**2
    with tarfile.open(path, "r:xz") as archive:
        for item in archive:
            require(item.name.startswith(prefix) and item.isfile(), "Unsafe source archive member")
            name = safe(item.name[len(prefix):])
            require(name not in files, "Duplicate source archive member")
            count, total = archive_budget([item.size], count=count, total=total)
            require(item.mode in (0o644, 0o755), "Unexpected source archive file mode")
            modes[name] = item.mode
            files[name] = archive.extractfile(item).read()
            if total >= next_progress:
                print(f"Source archive: read {total} expanded bytes in {count} members", file=sys.stderr, flush=True)
                next_progress = (total // (512 * 1024**2) + 1) * 512 * 1024**2
    print(f"Source archive: all {count} members read; verifying inventory, source and dependency evidence", file=sys.stderr, flush=True)
    manifest = load(files.pop("source-manifest.json"))
    require(set(manifest.get("files", {})) == set(files), "Source archive file inventory differs")
    for name, data in files.items():
        require(manifest["files"][name] == {"size": len(data), "sha256": sha(data), "mode": modes[name]}, f"Source archive file hash differs: {name}")
    return verify_contents(files, manifest, commit, version, native_evidence, linux_evidence, macos_evidence)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1],
                        help="Immutable release checkout; may differ from trusted verifier tooling")
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--native-evidence", type=Path)
    parser.add_argument("--collect-native", metavar="TARGET")
    parser.add_argument("--verify", type=Path)
    parser.add_argument("--verify-native", type=Path)
    parser.add_argument("--cargo-vendor", type=Path)
    parser.add_argument("--dist", type=Path)
    parser.add_argument("--linux-runtime-sources", type=Path, help="Exact local pinned AppImage runtime/launcher source archives; otherwise fetched by checksum")
    parser.add_argument('--linux-evidence', type=Path, help='Both Linux builder source manifests and, when building, captured objects')
    parser.add_argument('--macos-evidence', type=Path, help='Both source-bound macOS signing receipts')
    parser.add_argument('--verify-macos', type=Path, help='Validate a macOS builder signing receipt offline')
    args = parser.parse_args()
    if args.verify:
        result = verify_bundle(args.verify, args.source_sha, args.version, args.native_evidence, args.linux_evidence, args.macos_evidence)
        print(json.dumps({**{key: result[key] for key in ("schema", "version", "source_commit", "targets")},
                          'linux_distribution': result.get('linux_distribution', {}),
                          'macos_signing': result.get('macos_signing', {})}))
    elif args.verify_native:
        wrapper = load(args.verify_native.read_bytes())
        native_manifest(wrapper, args.source_sha, args.version)
        print(json.dumps({"target": wrapper["target"], "manifest_sha256": sha(args.verify_native.read_bytes())}))
    elif args.verify_macos:
        helper = macos_helper();root = args.root.resolve()
        receipt = load(helper.bounded(args.verify_macos, 1024*1024))
        helper.validate_receipt(receipt, helper.bounded(root/helper.POLICY, 8192), args.source_sha, args.version,
                                receipt['platform'], load((root/'src-tauri/tauri.conf.json').read_bytes())['identifier'])
        print(json.dumps(receipt))
    elif args.collect_native:
        require(args.output and args.native_evidence, "Native collection requires --output and --native-evidence")
        collect_native(args.native_evidence, args.collect_native, args.source_sha, args.version, args.output)
    else:
        require(args.output is not None, "Building requires --output")
        build_bundle(args.root.resolve(), args.source_sha, args.version, args.output,
                     args.native_evidence, vendor_dir=args.cargo_vendor, dist_dir=args.dist,
                     linux_runtime_sources=args.linux_runtime_sources, linux_evidence=args.linux_evidence,
                     macos_evidence=args.macos_evidence)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, tarfile.TarError, subprocess.CalledProcessError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
