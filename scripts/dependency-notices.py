#!/usr/bin/env python3
"""Collect/check source-bound dependency notice material without executing it.

The Cargo set is a conservative normal-dependency candidate closure. It is not
an assertion about dead-stripped binary membership or redistributed system SDKs.
"""
import argparse
import base64
import hashlib
import importlib.util
import json
from pathlib import Path, PurePosixPath
import re
import tarfile
import tomllib
import sys
import subprocess
import os
import stat
import tempfile

TARGETS = {
    "windows-x64": "x86_64-pc-windows-msvc",
    "macos-arm64": "aarch64-apple-darwin",
    "macos-x64": "x86_64-apple-darwin",
    "linux-x64": "x86_64-unknown-linux-gnu",
    "linux-arm64": "aarch64-unknown-linux-gnu",
    "android-arm64": "aarch64-linux-android",
    "ios-arm64-simulator-preview": "aarch64-apple-ios-sim",
}
FEATURES = ["tauri/custom-protocol"]
INVENTORY = "src/licenses/dependency-inventory.json"
PAYLOAD = "THIRD_PARTY_DEPENDENCIES.md"
SUPPLEMENTS = "scripts/notices/supplements.json"
BEGIN = "<!-- BEGIN GENERATED DEPENDENCY NOTICES -->"
END = "<!-- END GENERATED DEPENDENCY NOTICES -->"
NOTICE_PREFIXES = ("license", "licence", "unlicense", "copying", "copyright", "notice")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode("utf-8")


def unique_json(raw):
    def pairs(values):
        result = {}
        for key, value in values:
            require(key not in result, f"Duplicate JSON key: {key}")
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs)


def safe(path):
    name = PurePosixPath(path)
    require(isinstance(path, str) and path and not name.is_absolute()
            and ".." not in name.parts and "\\" not in path
            and str(name) == path, f"Unsafe notice path: {path}")
    return path


def runtime_packages(metadata):
    packages = {value["id"]: value for value in metadata["packages"]}
    nodes = {value["id"]: value for value in metadata["resolve"]["nodes"]}
    require(len(packages) == len(metadata["packages"]), "Duplicate Cargo package ID")
    require(len(nodes) == len(metadata["resolve"]["nodes"]), "Duplicate Cargo resolve node")
    roots = [key for key, value in packages.items()
             if value["name"] == "opentag" and value.get("source") is None]
    require(len(roots) == 1, "Missing or ambiguous application Cargo root")
    visited, macros, queue = set(), set(), roots[:]
    while queue:
        key = queue.pop()
        if key in visited or key in macros:
            continue
        require(key in packages and key in nodes, f"Missing Cargo dependency: {key}")
        package = packages[key]
        if any("proc-macro" in target["kind"] for target in package["targets"]):
            macros.add(key)
            continue
        visited.add(key)
        queue.extend(dep["pkg"] for dep in nodes[key]["deps"]
                     if any(kind["kind"] is None for kind in dep["dep_kinds"]))
    return [packages[key] for key in sorted(visited) if packages[key].get("source") is not None]


def source_fingerprints(root):
    workspace = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    names = {"Cargo.toml", "Cargo.lock", "package.json", "package-lock.json"}
    for member in workspace.get("workspace", {}).get("members", []):
        safe(member)
        matches = list(root.glob(member))
        require(matches, f"Missing Cargo workspace member: {member}")
        names.update((path / "Cargo.toml").relative_to(root).as_posix() for path in matches)
    for optional in [".cargo/config.toml", SUPPLEMENTS, "scripts/dependency-notices.py",
                     "scripts/audio-deps/sources.json",
                     "crates/regex-compat/vendor/boost-regex/SOURCE.json",
                     "crates/tag-core/src/actions_lcid.source.json"]:
        if (root / optional).is_file():
            names.add(optional)
    result = {}
    for name in sorted(names):
        raw = (root / name).read_bytes()
        if name == "Cargo.lock":
            value = tomllib.loads(raw.decode("utf-8"))
            for package in value.get("package", []):
                if package["name"] == "opentag" and "source" not in package:
                    package["version"] = "APP_RELEASE_VERSION"
            raw = encoded(value)
        elif name.endswith("Cargo.toml"):
            value = tomllib.loads(raw.decode("utf-8"))
            if value.get("package", {}).get("name") == "opentag":
                value["package"]["version"] = "APP_RELEASE_VERSION"
            raw = encoded(value)
        elif name in {"package.json", "package-lock.json"}:
            value = unique_json(raw)
            value.pop("version", None)
            if name == "package-lock.json" and "" in value.get("packages", {}):
                value["packages"][""].pop("version", None)
            raw = encoded(value)
        elif name.endswith(".json"):
            raw = encoded(unique_json(raw))
        elif name.endswith(".toml"):
            raw = encoded(tomllib.loads(raw.decode("utf-8")))
        elif name.endswith(".py"):
            raw = raw.replace(b"\r\n", b"\n")
        result[name] = sha(raw)
    return result


def archive_materials(archive, checksum, prefix):
    require(sha(archive.read_bytes()) == checksum, f"Archive checksum mismatch: {archive.name}")
    safe(prefix)
    with tarfile.open(archive) as handle:
        files = {}
        for member in handle:
            path = member.name.rstrip("/")
            safe(path)
            require(path == prefix or path.startswith(prefix + "/"), "Archive member outside package root")
            require(path not in files, f"Duplicate archive member: {path}")
            files[path] = member
        def read(relative):
            member = files.get(prefix + "/" + safe(relative))
            require(member is not None and member.isfile(), f"Missing/nonregular archive file: {relative}")
            require(member.size <= 16 * 1024 * 1024, f"Notice input exceeds size limit: {relative}")
            return handle.extractfile(member).read()
        metadata_name = "Cargo.toml" if (prefix + "/Cargo.toml") in files else "package.json"
        metadata = read(metadata_name)
        package = tomllib.loads(metadata.decode("utf-8"))["package"] if metadata_name == "Cargo.toml" else unique_json(metadata)
        explicit = package.get("license-file")
        if explicit:
            safe(explicit)
            require(prefix + "/" + explicit in files, "Declared license file is absent")
        materials = []
        for name, member in sorted(files.items()):
            if name == prefix:
                continue
            relative = name.removeprefix(prefix + "/")
            if not PurePosixPath(relative).name.lower().startswith(NOTICE_PREFIXES) and relative != explicit:
                continue
            require(member.isfile(), f"Nonregular notice member: {relative}")
            raw = read(relative)
            materials.append({"path": relative, "sha256": sha(raw), "text": raw.decode("utf-8")})
        vcs = unique_json(read(".cargo_vcs_info.json")) if prefix + "/.cargo_vcs_info.json" in files else None
        return {"package": package, "vcs": vcs, "materials": materials}


def cargo_pins(root):
    data = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
    result = {}
    for package in data["package"]:
        if not package.get("source", "").startswith("registry+"):
            continue
        key = f"cargo:{package['name']}@{package['version']}"
        require(key not in result, f"Duplicate locked registry package: {key}")
        result[key] = package["checksum"]
    return result


def npm_runtime_entries(root):
    manifest = unique_json((root / "package.json").read_bytes())
    packages = unique_json((root / "package-lock.json").read_bytes())["packages"]
    pending = [("", name, True) for name in manifest.get("dependencies", {})]
    # Vite's module-preload support and esbuild's emitted helpers can reach the
    # frontend even though the tools are development dependencies. Preserve their
    # own notices as conservative helper-provider material, not runtime imports.
    providers = {name for name in ["vite", "esbuild"] if name in manifest.get("devDependencies", {})}
    if ("vite" in providers or "vite" in manifest.get("dependencies", {})) and "esbuild" in packages.get("node_modules/vite", {}).get("dependencies", {}):
        providers.add("esbuild")
    pending += [("", name, False) for name in sorted(providers)]
    found, expanded = {}, set()
    while pending:
        parent, name, traverse = pending.pop()
        require(re.fullmatch(r"(?:@[A-Za-z0-9_.-]+/)?[A-Za-z0-9_.-]+", name) is not None, "Unsafe NPM dependency name")
        candidates = []
        current = parent
        while current:
            candidates.append(current + "/node_modules/" + name)
            current = current.rsplit("/node_modules/", 1)[0] if "/node_modules/" in current else ""
        candidates.append("node_modules/" + name)
        path = next((candidate for candidate in candidates if candidate in packages), None)
        require(path is not None, f"Missing locked NPM runtime dependency: {name}")
        found[path] = (name, packages[path])
        # Helper-provider tools are recorded for their output support, not as a
        # claim that every tool implementation dependency ships in the frontend.
        if traverse and path not in expanded:
            expanded.add(path)
            deps = {**packages[path].get("dependencies", {}), **packages[path].get("optionalDependencies", {})}
            pending.extend((path, child, True) for child in deps)
    return found


def validate_inventory(root, data):
    require(data.get("schema") == 1, "Unknown dependency notice schema")
    require(data.get("inputs") == source_fingerprints(root), "Dependency notice source inputs changed; regenerate the inventory")
    require(data.get("features") == FEATURES, "Dependency inventory feature set differs")
    require(set(data.get("targets", {})) == set(TARGETS), "Dependency inventory requires all release targets")
    pins = cargo_pins(root)
    npm = {f"npm:{name}@{record['version']}": record for name, record in npm_runtime_entries(root).values()}
    components = {}
    referenced = set()
    for component in data["components"]:
        key = component["id"]
        require(key not in components, f"Duplicate dependency component: {key}")
        require(key == f"{component['ecosystem']}:{component['name']}@{component['version']}", "Component identity differs")
        components[key] = component
        if component["ecosystem"] == "cargo":
            require(key in pins and component["archive_sha256"] == pins[key], f"Cargo component does not match lock: {key}")
        else:
            require(component["ecosystem"] == "npm" and key in npm, f"Unexpected component: {key}")
            require(component["integrity"] == npm[key]["integrity"], f"NPM component integrity differs: {key}")
        require(component.get("materials"), f"No license/notice material: {key}")
        require(component.get("targets") and len(component["targets"]) == len(set(component["targets"]))
                and set(component["targets"]) <= set(TARGETS), "Invalid component target membership")
        for material in component["materials"]:
            safe(material["path"])
            digest = material["sha256"]
            text = data["texts"].get(digest)
            require(isinstance(text, str) and sha(text.encode("utf-8")) == digest, f"Notice text hash differs: {key}")
            referenced.add(digest)
            if "source_file" in material:
                source = root / safe(material["source_file"])
                require(source.is_file() and sha(source.read_bytes()) == digest, f"Supplemental notice source differs: {key}")
    require(set(data["texts"]) == referenced, "Unreferenced or missing notice texts")
    require({key for key in components if key.startswith("npm:")} == set(npm), "NPM runtime/helper-provider inventory is incomplete")
    for platform, members in data["targets"].items():
        require(len(members) == len(set(members)), "Duplicate target dependency")
        expected = {key for key, component in components.items()
                    if component["ecosystem"] == "cargo" and platform in component["targets"]}
        require(set(members) == expected, f"Missing or unexpected target dependency: {platform}")


def npm_archive(cache, name, version, record):
    filename = name.replace("/", "-").removeprefix("@") + "-" + version + ".tgz"
    archive = cache / safe(filename)
    raw = archive.read_bytes()
    integrities = dict(value.split("-", 1) for value in record["integrity"].split())
    algorithm = next((value for value in ["sha512", "sha384", "sha256"] if value in integrities), None)
    require(algorithm is not None, f"NPM package has no supported strong integrity: {name}")
    require(hashlib.new(algorithm, raw).digest() == base64.b64decode(integrities[algorithm], validate=True),
            f"NPM archive integrity differs: {name}")
    return archive, sha(raw)


def supplemental_materials(root, key, checksum, source, archive, supplements, cargo_sources):
    supplement = supplements.get(key)
    if supplement is None:
        return []
    require(supplement["archive_sha256"] == checksum, f"Supplement targets a different archive: {key}")
    if key.startswith("cargo:"):
        if source.get("vcs"):
            require(source["vcs"]["git"]["sha1"] == supplement.get("vcs_commit"),
                    f"Supplement VCS commit differs: {key}")
        else:
            # Older Cargo archives can predate .cargo_vcs_info.json. Retain
            # their exact source notice instead of inventing a VCS identity.
            require(supplement.get("archive_bound") is True and not supplement.get("vcs_commit")
                    and any("archive_member" in item for item in supplement["materials"]),
                    f"Legacy archive supplement needs an exact source header: {key}")
            require(all("archive_member" in item or (item.get("license_reference") and item.get("source_url"))
                        for item in supplement["materials"]),
                    f"Legacy supplement needs archive headers or explicit standard license references: {key}")
    result = []
    for item in supplement["materials"]:
        raw = (root / safe(item["source_file"])).read_bytes()
        require(sha(raw) == item["sha256"], f"Supplement file hash differs: {key}")
        if "git_blob_sha1" in item:
            git_blob = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
            require(git_blob == item["git_blob_sha1"], f"Supplement Git blob differs: {key}")
            require(item["upstream_commit"] == supplement["vcs_commit"]
                    and "/" + item["upstream_commit"] + "/" in item["source_url"], "Supplement source URL/commit differs")
        if "reference_package" in item:
            parent, parent_checksum, parent_prefix = cargo_sources[item["reference_package"]]
            require(parent_checksum == item["reference_archive_sha256"], "Shared license source archive differs")
            require(parent.get("vcs") and parent["vcs"]["git"]["sha1"] == supplement["vcs_commit"],
                    "Shared workspace license has a different VCS commit")
            member = item["reference_member"].removeprefix(parent_prefix + "/")
            require(any(value["path"] == member and value["sha256"] == sha(raw)
                        for value in parent["materials"]), "Shared workspace notice is absent from its pinned archive")
        if "archive_member" in item:
            safe(item["archive_member"])
            size = item["header_prefix_bytes"]
            require(isinstance(size, int) and 0 < size <= 65536, "Invalid notice header prefix size")
            with tarfile.open(archive) as handle:
                entry = handle.getmember(item["archive_member"])
                require(entry.isfile(), "Notice header source is not a regular file")
                require(handle.extractfile(entry).read(size) == raw, "Notice header differs from the pinned archive")
        result.append({**item, "text": raw.decode("utf-8")})
    return result


def collect(root, target_members, cargo_cache, npm_cache):
    pins = cargo_pins(root)
    supplements = unique_json((root / SUPPLEMENTS).read_bytes())["components"] if (root / SUPPLEMENTS).is_file() else {}
    components, texts, cargo_sources = [], {}, {}
    ids = sorted({key for members in target_members.values() for key in members})
    for key in ids:
        require(key.startswith("cargo:") and key in pins, f"Unlocked target component: {key}")
        name, version = key.removeprefix("cargo:").rsplit("@", 1)
        prefix = safe(name + "-" + version)
        archive = cargo_cache / (prefix + ".crate")
        source = archive_materials(archive, pins[key], prefix)
        require(source["package"]["name"] == name and source["package"]["version"] == version,
                f"Archive package identity differs: {key}")
        cargo_sources[key] = (source, pins[key], prefix)
    def add(key, ecosystem, name, version, checksum, source, archive, extra):
        provided = source["materials"] + supplemental_materials(
            root, key, checksum, source, archive, supplements, cargo_sources)
        require(provided, f"No notice material; supply an explicit pinned supplement: {key}")
        materials = []
        for material in provided:
            text = material["text"]
            digest = material["sha256"]
            require(sha(text.encode("utf-8")) == digest, "Notice text is not a lossless UTF-8 projection")
            require(digest not in texts or texts[digest] == text, "Conflicting notice digest")
            texts[digest] = text
            materials.append({field: value for field, value in material.items() if field != "text"})
        components.append({"id": key, "ecosystem": ecosystem, "name": name, "version": version,
                           "archive_sha256": checksum, "license": source["package"].get("license"),
                           "license_file": source["package"].get("license-file"),
                           "materials": materials, **extra})
    for key in ids:
        source, checksum, prefix = cargo_sources[key]
        name, version = key.removeprefix("cargo:").rsplit("@", 1)
        add(key, "cargo", name, version, checksum, source, cargo_cache / (prefix + ".crate"),
            {"targets": sorted(platform for platform, members in target_members.items() if key in members),
             "source_url": f"https://crates.io/api/v1/crates/{name}/{version}/download",
             "vcs": source["vcs"], "role": "normal-dependency candidate"})
    manifest = unique_json((root / "package.json").read_bytes())
    for name, record in sorted(set_entries(npm_runtime_entries(root)).values(), key=lambda item: (item[0], item[1]["version"])):
        version = record["version"]
        key = f"npm:{name}@{version}"
        archive, checksum = npm_archive(npm_cache, name, version, record)
        source = archive_materials(archive, checksum, "package")
        require(source["package"]["name"] == name and source["package"]["version"] == version,
                f"NPM archive identity differs: {key}")
        add(key, "npm", name, version, checksum, source, archive,
            {"targets": sorted(TARGETS), "source_url": record["resolved"], "integrity": record["integrity"],
             "role": "frontend helper provider" if name in {"vite", "esbuild"} and name not in manifest.get("dependencies", {}) else "runtime dependency"})
    return {"schema": 1, "scope": "Source-defined Cargo normal-dependency candidates and locked runtime JavaScript/helper providers; not dead-stripped binary or system SDK membership.",
            "inputs": source_fingerprints(root), "features": FEATURES,
            "targets": {name: sorted(members) for name, members in sorted(target_members.items())},
            "components": sorted(components, key=lambda item: item["id"]), "texts": dict(sorted(texts.items()))}


def set_entries(entries):
    result = {}
    for name, record in entries.values():
        key = f"npm:{name}@{record['version']}"
        require(key not in result or result[key][1] == record, f"Conflicting locked NPM records: {key}")
        result[key] = (name, record)
    return result


def graph_inputs(root):
    return {name: digest for name, digest in source_fingerprints(root).items()
            if name == "Cargo.lock" or name.endswith("Cargo.toml") or name == ".cargo/config.toml"}


def collect_metadata(root, directory, allow_network=False):
    inputs = graph_inputs(root)
    directory.mkdir(parents=True, exist_ok=True)
    for platform, target in TARGETS.items():
        command = ["cargo", "metadata", "--locked", "--format-version", "1",
                   "--features", "tauri/custom-protocol", "--filter-platform", target]
        if not allow_network:
            command.append("--offline")
        result = subprocess.run(command, cwd=root, capture_output=True, check=False)
        require(result.returncode == 0, "Cargo metadata collection failed: " + result.stderr.decode("utf-8", errors="replace"))
        metadata = unique_json(result.stdout)
        require(graph_inputs(root) == inputs, "Cargo graph inputs changed during metadata collection")
        wrapper = {"schema": 1, "platform": platform, "target": target, "features": FEATURES,
                   "inputs": inputs, "command": command, "metadata": metadata}
        (directory / (platform + ".json")).write_bytes(encoded(wrapper))


def target_inventory(root, directory):
    targets = {}
    for platform in TARGETS:
        wrapper = unique_json((directory / (platform + ".json")).read_bytes())
        require(wrapper.get("schema") == 1 and wrapper.get("platform") == platform
                and wrapper.get("target") == TARGETS[platform] and wrapper.get("features") == FEATURES,
                f"Unbound or mismatched Cargo metadata snapshot: {platform}")
        require(wrapper.get("inputs") == graph_inputs(root), f"Stale metadata source inputs: {platform}")
        metadata = wrapper["metadata"]
        targets[platform] = sorted(f"cargo:{package['name']}@{package['version']}"
                                   for package in runtime_packages(metadata))
    return targets


def render_notices(data):
    numbers = {digest: f"N{index:04d}" for index, digest in enumerate(sorted(data["texts"]), 1)}
    lines = ["# Dependency licenses and notices", "", "Generated from checksum-bound package material. Source-declared target candidates are conservative; this is not an exact dead-stripped binary inventory or a legal determination.", "", "## Component index", ""]
    for component in data["components"]:
        refs = ", ".join(dict.fromkeys(numbers[item["sha256"]] for item in component["materials"]))
        declared = component.get("license") or "See the declared license file"
        lines += [f"- {component['name']} {component['version']} ({component['ecosystem']}; {component['role']}): {declared}. Material: {refs}."]
    lines += ["", "## Notice material", ""]
    for digest, text in sorted(data["texts"].items()):
        lines += [f"### {numbers[digest]} - SHA-256 {digest}", "", text.replace("\r\n", "\n").replace("\r", "\n").rstrip("\n"), ""]
    return "\n".join(lines).rstrip("\n") + "\n"


def common_notices(common, payload):
    require(common.count(BEGIN) == common.count(END) and common.count(BEGIN) <= 1, "Ambiguous generated common-notice markers")
    section = BEGIN + "\n\n" + payload.rstrip("\n") + "\n\n" + END
    if BEGIN not in common:
        return common.rstrip("\n") + "\n\n" + section + "\n"
    prefix, remaining = common.split(BEGIN, 1)
    old, suffix = remaining.split(END, 1)
    require(END not in prefix and BEGIN not in old, "Invalid notice marker order")
    return prefix + section + suffix


def rtf_bytes(common):
    path = Path(__file__).with_name("installer-license.py")
    spec = importlib.util.spec_from_file_location("installer_license", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.render(common)


def publish_outputs(root, outputs, guards=None):
    """Stage the complete set before replacing files; reverse handled failures.

    This is not an all-files atomic commit or a power-loss transaction. Outside
    changes encountered during rollback are retained along with recovery copies.
    Guards bind a leaf to prior bytes, or to expected absence when set to None.
    """
    entries, temporary = [], set()
    retained = set()
    def state(path):
        try:
            info = path.lstat()
        except FileNotFoundError:
            return None
        require(stat.S_ISREG(info.st_mode), f"Notice output is not a regular file: {path}")
        return (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_mode, sha(path.read_bytes()))
    def stage(path, content, mode):
        descriptor, name = tempfile.mkstemp(prefix=".dependency-notices-", dir=path.parent)
        target = Path(name)
        temporary.add(target)
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        os.chmod(target, mode)
        return target
    try:
        # Inspect every output and ancestor before staging or replacing any leaf.
        for name, content in outputs.items():
            path = root / safe(name)
            for parent in path.parents:
                if parent == root:
                    break
                require(not parent.is_symlink() and (not parent.exists() or parent.is_dir()),
                        f"Unsafe notice output parent: {parent}")
            before = state(path)
            if guards and name in guards:
                if guards[name] is None:
                    require(before is None, f"Notice output appeared during generation: {name}")
                else:
                    require(before is not None and path.read_bytes() == guards[name],
                            f"Notice source changed during generation: {name}")
            info = path.stat() if before is not None else None
            entries.append({"path": path, "content": content, "before": before,
                            "original": path.read_bytes() if before is not None else None,
                            "mode": stat.S_IMODE(info.st_mode) if info else 0o644})
        for entry in entries:
            path = entry["path"]
            path.parent.mkdir(parents=True, exist_ok=True)
            entry["staged"] = stage(path, entry["content"], entry["mode"])
            entry["backup"] = stage(path, entry["original"], entry["mode"]) if entry["original"] is not None else None
        published = []
        try:
            for entry in entries:
                require(state(entry["path"]) == entry["before"], f"Notice output changed before publication: {entry['path']}")
                produced = state(entry["staged"])
                os.replace(entry["staged"], entry["path"])
                # Same-directory atomic replacement preserves the staged leaf's
                # identity. Register it before readback can fail or be replaced.
                entry["produced"] = produced
                published.append(entry)
                require(state(entry["path"]) == produced, f"Published notice changed during readback: {entry['path']}")
        except (OSError, ValueError) as error:
            failures = []
            for entry in reversed(published):
                try:
                    require(state(entry["path"]) == entry["produced"], "Published notice was changed outside the generator")
                    if entry["backup"] is None:
                        entry["path"].unlink()
                    else:
                        os.replace(entry["backup"], entry["path"])
                except (OSError, ValueError) as rollback_error:
                    if entry["backup"] is not None:
                        retained.add(entry["backup"])
                    failures.append(f"{entry['path']}: {rollback_error}; recovery copy: {entry['backup']}")
            if failures:
                raise ValueError(f"{error}; notice rollback incomplete: " + "; ".join(failures)) from error
            raise
    finally:
        for path in temporary - retained:
            try:
                path.unlink(missing_ok=True)
            except OSError:
                # A failed cleanup must not recursively remove unknown contents.
                print(f"Retained notice staging path: {path}", file=sys.stderr)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--generate", action="store_true")
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--collect-metadata", type=Path, metavar="DIRECTORY", help="Capture seven Cargo graphs with their source/target/feature bindings; no application build")
    parser.add_argument("--allow-network", action="store_true", help="Allow Cargo to fetch missing registry metadata when collecting graphs")
    parser.add_argument("--metadata-directory", type=Path, help="Seven bound PLATFORM.json snapshots from --collect-metadata")
    parser.add_argument("--cargo-cache", type=Path)
    parser.add_argument("--npm-cache", type=Path)
    parser.add_argument("--verify-material", action="store_true", help="Replay actual archive checksums, exact notice members and supplemental provenance")
    args = parser.parse_args()
    root = args.root.resolve()
    try:
        if args.collect_metadata:
            collect_metadata(root, args.collect_metadata, args.allow_network)
            print("Captured source-bound metadata for all seven release targets")
            return
        if args.generate or args.verify_material:
            require(args.cargo_cache is not None and args.npm_cache is not None, "Material collection requires --cargo-cache and --npm-cache")
        if args.generate:
            require(args.metadata_directory is not None, "Generation requires --metadata-directory")
            before = source_fingerprints(root)
            data = collect(root, target_inventory(root, args.metadata_directory), args.cargo_cache, args.npm_cache)
            require(data["inputs"] == before, "Notice inputs changed during collection")
        else:
            data = unique_json((root / INVENTORY).read_bytes())
        validate_inventory(root, data)
        if args.metadata_directory:
            require(target_inventory(root, args.metadata_directory) == data["targets"], "Cargo target graph differs from the notice inventory")
        if args.verify_material:
            replay = collect(root, data["targets"], args.cargo_cache, args.npm_cache)
            require(encoded(replay) == encoded(data), "Replayed notice material differs from inventory")
        payload = render_notices(data)
        source_raw = (root / "THIRD_PARTY_NOTICES.md").read_bytes()
        source = source_raw.decode("utf-8").replace("\r\n", "\n").replace("\r", "\n")
        common = common_notices(source, payload)
        outputs = {INVENTORY: encoded(data), PAYLOAD: payload.encode("utf-8"),
                   "THIRD_PARTY_NOTICES.md": common.encode("utf-8"),
                   "THIRD_PARTY_NOTICES.rtf": rtf_bytes(common)}
        require(source_fingerprints(root) == data["inputs"], "Notice source inputs changed during validation")
        if args.check:
            for name, content in outputs.items():
                path = root / name
                require(path.is_file() and path.read_bytes() == content, f"Stale or detached notice output: {name}")
        else:
            publish_outputs(root, outputs, {"THIRD_PARTY_NOTICES.md": source_raw})
        print(f"Verified {len(data['components'])} dependency components and {len(data['texts'])} distinct notice texts")
    except (OSError, ValueError, KeyError, TypeError, tarfile.TarError) as error:
        parser.exit(1, f"Dependency notices: {error}\n")


if __name__ == "__main__":
    main()
