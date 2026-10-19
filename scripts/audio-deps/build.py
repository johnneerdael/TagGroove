#!/usr/bin/env python3
"""Build the embedded decoder from pinned source, never host media libraries.

Python/CMake/GNU Make are build tools only. Installed OpenTag needs none of them.
All downloads and outputs live in the explicitly selected owned build directory.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import shlex
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.request

HERE = Path(__file__).resolve().parent
SOURCES = HERE / "sources.json"
TARGETS = {
    "aarch64-apple-darwin", "x86_64-apple-darwin",
    "x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu", "aarch64-linux-android",
    "aarch64-apple-ios-sim", "aarch64-apple-ios",
}


def digest(path):
    result = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(chunk)
    return result.hexdigest()


class ArchiveChecksumMismatch(ValueError):
    """Source bytes do not match their pinned digest."""


def verify_archive(path, expected, *, name=None):
    actual = digest(path)
    if actual != expected:
        raise ArchiveChecksumMismatch(
            f"SHA-256 mismatch for {name or Path(path).name}: expected {expected}, got {actual}")


def extract_archive(path, output):
    """Validate every entry first; materialize safe header aliases on Windows."""
    output = Path(output).resolve()
    with tarfile.open(path) as archive:
        members = archive.getmembers()
        for member in members:
            name = PurePosixPath(member.name)
            destination = output / member.name
            if (name.is_absolute() or ".." in name.parts or "\\" in member.name
                    or ":" in member.name or not destination.resolve().is_relative_to(output)
                    or not (member.isfile() or member.isdir() or member.issym() or member.islnk())):
                raise ValueError(f"Unsafe archive entry: {member.name}")
            if member.issym() or member.islnk():
                base = destination.parent if member.issym() else output
                target = base / member.linkname
                if (PurePosixPath(member.linkname).is_absolute() or "\\" in member.linkname
                        or ":" in member.linkname or not target.resolve().is_relative_to(output)):
                    raise ValueError(f"Unsafe archive link: {member.name}")
        for member in members:
            destination = output / member.name
            if member.isdir():
                destination.mkdir(parents=True, exist_ok=True)
            elif member.isfile():
                destination.parent.mkdir(parents=True, exist_ok=True)
                with archive.extractfile(member) as source, destination.open("wb") as target:
                    shutil.copyfileobj(source, target)
                destination.chmod(member.mode & 0o755)
        for member in members:
            if member.issym() or member.islnk():
                destination = output / member.name
                target = (destination.parent if member.issym() else output) / member.linkname
                if not target.is_file():
                    raise ValueError(f"Unsafe archive link target: {member.name}")
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(target, destination)


def reserve(path):
    free = shutil.disk_usage(path).free
    if free <= 2.3 * 1024**3:
        raise RuntimeError(f"Native build needs more than 2.3 GiB free; found {free} bytes")


def run(args, cwd, log, env=None):
    reserve(cwd)
    with Path(log).open("ab") as stream:
        stream.write(("\n$ " + repr([str(x) for x in args]) + "\n").encode())
        stream.flush()
        result = subprocess.run([str(x) for x in args], cwd=cwd, env=env,
                                stdout=stream, stderr=subprocess.STDOUT, check=False)
    if result.returncode:
        lines = Path(log).read_text(errors="replace").splitlines()
        diagnostic = Path(cwd) / "ffbuild/config.log"
        if diagnostic.is_file():
            lines += ["\nFFmpeg compiler/configure diagnostic:",
                      *diagnostic.read_text(errors="replace").splitlines()[-80:]]
        raise RuntimeError(f"Native build failed ({result.returncode}): {log}\n" + "\n".join(lines[-120:]))


def acquire(spec, downloads, sources):
    path = downloads / spec["archive"]
    if not path.exists():
        offline = os.environ.get("OPENTAG_AUDIO_SOURCE_DIR")
        if offline:
            candidate = Path(offline) / spec["archive"]
            verify_archive(candidate, spec["sha256"])
            shutil.copyfile(candidate, path)
    if not path.exists():
        request = urllib.request.Request(spec["url"], headers={"User-Agent": "OpenTag-source-builder/1"})
        # Retry only newly downloaded wrong bytes, never a tampered existing
        # cache or an explicit offline source. Every attempt keeps the same pin.
        for attempt in range(3):
            with tempfile.NamedTemporaryFile(dir=downloads, delete=False) as temporary:
                candidate = Path(temporary.name)
                try:
                    with urllib.request.urlopen(request, timeout=60) as response:
                        shutil.copyfileobj(response, temporary)
                    temporary.close()
                    verify_archive(candidate, spec["sha256"], name=spec["archive"])
                    candidate.replace(path)
                    break
                except ArchiveChecksumMismatch:
                    if attempt == 2:
                        raise
                finally:
                    temporary.close()
                    candidate.unlink(missing_ok=True)
            time.sleep(0.5 * (attempt + 1))
    verify_archive(path, spec["sha256"])
    directory = sources / spec["directory"]
    # A completed prefix is separately hash-verified before this path. An
    # incomplete/retried build always starts with fresh verified source bytes,
    # rather than trusting a mutable extracted tree or partially applied patch.
    if directory.exists():
        shutil.rmtree(directory)
    extract_archive(path, sources)
    return directory


def recipe_inputs(spec):
    inputs = {"sources.json": digest(SOURCES), "build.py": digest(Path(__file__)),
              "bzip2/CMakeLists.txt": digest(HERE / "bzip2/CMakeLists.txt")}
    for patch in spec.get("patches", []):
        path = HERE / patch["file"]
        verify_archive(path, patch["sha256"])
        inputs[patch["file"]] = digest(path)
    return inputs


def apply_patches(spec, native):
    for patch in spec.get("patches", []):
        path = native[patch["component"]] / patch["target"]
        verify_archive(path, patch["original_sha256"])
        old = path.read_text(encoding="utf-8")
        # This exact, reviewed policy opt-in leaves every upstream license
        # check intact. Match the complete original line, not a loose regex.
        needle = 'enabled gpl && enable lgpl_gpl # Files that are marked as LGPL but some developers prefer only building them with --enable-gpl'
        if old.count(needle) != 1:
            raise ValueError("Pinned FFmpeg policy patch no longer matches its source")
        new = old.replace(needle, needle + '\n# OpenTag opts in to these explicitly LGPL-marked audio components only.\n# GPL/nonfree/version3 remain disabled; the compiled object closure is audited.\nenable lgpl_gpl')
        path.write_bytes(new.encode("utf-8"))
        verify_archive(path, patch["patched_sha256"])


def validate_prefix(prefix, spec, recipe, inputs, target):
    result = json.loads((prefix / "opentag-audio-manifest.json").read_text())
    if (result.get("format") != 1 or result.get("target") != target
            or result.get("recipe_sha256") != recipe or result.get("build_inputs") != inputs
            or result.get("components") != spec["components"]):
        raise RuntimeError("Native build cache recipe/source provenance mismatch")
    files = result.get("prefix_files", {})
    if not files or not result.get("static_libraries"):
        raise RuntimeError("Native build cache is missing installed file hashes")
    actual = {p.relative_to(prefix).as_posix() for p in prefix.rglob("*")
              if p.is_file() and p.name != "opentag-audio-manifest.json"}
    if actual != set(files):
        raise RuntimeError("Native build cache installed file inventory changed")
    for name, expected in files.items():
        path = prefix / name
        if not path.resolve().is_relative_to(prefix.resolve()) or not path.is_file() or digest(path) != expected:
            raise RuntimeError(f"Native build cache failed installed-file verification: {name}")
    for component in spec["components"].values():
        verify_archive(prefix.parents[2] / "downloads" / component["archive"], component["sha256"])
    return result


def target_tools(target, env):
    """Return explicit compiler/SDK settings for the existing release matrix."""
    cmake = ["-DCMAKE_POSITION_INDEPENDENT_CODE=ON", "-DCMAKE_BUILD_TYPE=Release"]
    ffmpeg = ["--arch=" + ("aarch64" if target.startswith("aarch64") else "x86_64")]
    if target.endswith("windows-msvc"):
        msys = Path(env.get("OPENTAG_MSYS2", "C:/msys64")) / "usr/bin"
        if not (msys / "bash.exe").is_file() or not (msys / "make.exe").is_file():
            raise RuntimeError("Windows native source builds need MSYS2 bash and GNU Make (OPENTAG_MSYS2)")
        cc = env.get("CC", "cl.exe")
        compiler = shutil.which(cc, path=env.get("PATH")) or cc
        env["PATH"] = str(Path(compiler).parent) + os.pathsep + str(msys) + os.pathsep + env.get("PATH", "")
        # configure expands $cc as a shell command, so a Program Files path
        # cannot be passed as the command string. Select its basename through
        # the explicit compiler directory, also keeping MSVC's linker first.
        env["OPENTAG_AUDIO_SELECTED_COMPILER"] = str(compiler)
        ffmpeg += ["--toolchain=msvc", "--cc=" + Path(compiler).name,
                   "--extra-cflags=-MD", "--target-os=win32"]
        cmake += ["-G", "NMake Makefiles", "-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreadedDLL"]
        return cmake, ffmpeg, str(msys / "bash.exe"), str(msys / "make.exe")
    if target.endswith("linux-android"):
        ndk = Path(env.get("NDK_HOME", env.get("ANDROID_NDK_HOME", "")))
        hosts = sorted((ndk / "toolchains/llvm/prebuilt").glob("*"))
        if not hosts:
            raise RuntimeError("Android fingerprint builds require NDK_HOME or ANDROID_NDK_HOME")
        tools = hosts[0] / "bin"
        cc = tools / "aarch64-linux-android24-clang"
        ffmpeg += ["--target-os=android", "--enable-cross-compile", "--cc=" + str(cc),
                   "--cxx=" + str(cc) + "++", "--ar=" + str(tools / "llvm-ar"),
                   "--nm=" + str(tools / "llvm-nm"), "--ranlib=" + str(tools / "llvm-ranlib"),
                   "--strip=" + str(tools / "llvm-strip")]
        cmake += ["-DCMAKE_TOOLCHAIN_FILE=" + str(ndk / "build/cmake/android.toolchain.cmake"),
                  "-DANDROID_ABI=arm64-v8a", "-DANDROID_PLATFORM=android-24"]
    elif "apple" in target:
        sdk = "iphonesimulator" if target.endswith("ios-sim") else ("iphoneos" if target.endswith("apple-ios") else "macosx")
        sdk_path = subprocess.check_output(["xcrun", "--sdk", sdk, "--show-sdk-path"], text=True).strip()
        host_sdk = subprocess.check_output(["xcrun", "--sdk", "macosx", "--show-sdk-path"], text=True).strip()
        compiler = subprocess.check_output(["xcrun", "--sdk", sdk, "--find", "clang"], text=True).strip()
        minimum = "13.0" if sdk in {"iphonesimulator", "iphoneos"} else ("11.0" if target.startswith("aarch64") else "10.13")
        arch = "arm64" if target.startswith("aarch64") else "x86_64"
        flags = f"-arch {arch} -isysroot {sdk_path} "
        if sdk in {"iphonesimulator", "iphoneos"}:
            flags += f"-target arm64-apple-ios{minimum}" + ("-simulator" if sdk == "iphonesimulator" else "")
        else:
            flags += f"-mmacosx-version-min={minimum}"
        ffmpeg += ["--target-os=darwin", "--cc=" + compiler, "--extra-cflags=" + flags,
                   "--extra-ldflags=" + flags, "--host-cc=/usr/bin/clang",
                   "--host-cflags=-isysroot " + host_sdk, "--host-ldflags=-isysroot " + host_sdk,
                   "--ar=/usr/bin/ar", "--ranlib=/usr/bin/ranlib", "--nm=/usr/bin/nm",
                   "--strip=/usr/bin/strip"]
        if sdk in {"iphonesimulator", "iphoneos"}:
            ffmpeg += ["--enable-cross-compile"]
            cmake += ["-DCMAKE_SYSTEM_NAME=iOS"]
        cmake += ["-DCMAKE_OSX_SYSROOT=" + sdk_path, "-DCMAKE_OSX_ARCHITECTURES=" + arch,
                  "-DCMAKE_OSX_DEPLOYMENT_TARGET=" + minimum,
                  "-DCMAKE_AR=/usr/bin/ar", "-DCMAKE_RANLIB=/usr/bin/ranlib"]
    else:
        ffmpeg += ["--target-os=linux"]
    return cmake, ffmpeg, shutil.which("bash") or "bash", shutil.which("make") or "make"


def stage_windows_zlib(prefix):
    # FFmpeg's MSVC filter maps -lz to zlib.lib; Cargo links static=z.
    for name in ["zlib.lib", "z.lib"]:
        shutil.copyfile(prefix / "lib/zs.lib", prefix / "lib" / name)


def ffmpeg_command(bash, source, prefix, spec, target_flags):
    # Bash dirname and FFmpeg makefiles require forward slashes even on MSVC.
    # FFmpeg passes extra linker flags through unchanged during its first probe.
    library_path_flag = "-libpath:" if "--toolchain=msvc" in target_flags else "-L"
    flags = ["--prefix=" + prefix.as_posix(), "--disable-autodetect", "--disable-everything",
             "--disable-programs", "--disable-doc", "--disable-debug", "--disable-shared",
             "--enable-static", "--enable-pic", "--enable-asm", "--disable-network",
             "--disable-protocols", "--disable-avdevice", "--disable-avfilter", "--disable-swscale",
             "--disable-gpl", "--disable-nonfree", "--disable-version3",
             "--enable-avcodec", "--enable-avformat", "--enable-avutil", "--enable-swresample",
             "--enable-zlib", "--enable-bzlib", "--extra-cflags=-I" + (prefix / "include").as_posix(),
             "--extra-ldflags=" + library_path_flag + (prefix / "lib").as_posix(),
             "--enable-decoder=" + ",".join(spec["ffmpeg"]["decoders"]),
             "--enable-demuxer=" + ",".join(spec["ffmpeg"]["demuxers"]),
             "--enable-parser=" + ",".join(spec["ffmpeg"]["parsers"]), *target_flags]
    return [bash, (source / "configure").as_posix(), *flags]


def source_closure(build_root, source_root, msvc=False):
    """Resolve compiler dependency records to the exact pinned source tree.

    FFmpeg emits src/ aliases even with an absolute configure path. Resolve
    that alias explicitly, including MSYS builds where src can be a copy.
    Generated configuration is captured separately in the installed prefix.
    """
    closure = {}
    generated = {}
    source_root = source_root.resolve()
    build_root = build_root.resolve()
    for dep in sorted(build_root.rglob("*.d")):
        text = dep.read_text(errors="strict").replace("\\\n", " ")
        entries = {}
        if msvc:
            # FFmpeg's MSVC -showIncludes rule lists headers, not the .c/.asm
            # input. A unit including only SDK headers has an empty .d file.
            # Bind that record to its real object and unique pinned source;
            # never silently discard it or weaken the GNU dependency contract.
            if not dep.with_suffix(".o").is_file():
                raise RuntimeError(f"Native dependency record has no compiled object: {dep}")
            relative = dep.relative_to(build_root)
            units = [source_root / relative.with_suffix(suffix)
                     for suffix in [".c", ".cpp", ".cc", ".asm", ".S"]
                     if (source_root / relative.with_suffix(suffix)).is_file()]
            if len(units) != 1:
                raise RuntimeError(f"Native dependency record needs one pinned translation unit: {dep}")
            unit = units[0]
            unit_hash = digest(unit)
            alias = build_root / "src" / unit.relative_to(source_root)
            if alias.exists() and (not alias.is_file() or digest(alias) != unit_hash):
                raise RuntimeError(f"Native compiled source alias differs from pinned source: {alias}")
            entries[unit.relative_to(source_root).as_posix()] = unit_hash
        _, separator, dependencies = text.partition(": ")
        if not separator and (text.strip() or not msvc):
            raise RuntimeError(f"Invalid native dependency record: {dep}")
        for name in shlex.split(dependencies):
            if name.startswith("src/"):
                # NASM also writes src//header.asm; normalize separators before
                # joining so its extra slash cannot discard the pinned root.
                relative = Path(*PurePosixPath(name).parts[1:])
                path = source_root / relative
                alias = build_root / "src" / relative
                if (not path.resolve().is_relative_to(source_root) or not alias.is_file()
                        or not path.is_file() or digest(alias) != digest(path)):
                    raise RuntimeError(f"Native compiled dependency alias differs from pinned source: {alias}")
            else:
                path = Path(name)
                if not path.is_absolute():
                    path = build_root / path
            path = path.resolve()
            if path.is_relative_to(source_root) and path.is_file():
                entries[path.relative_to(source_root).as_posix()] = digest(path)
            elif path.is_relative_to(build_root) and path.is_file():
                generated[path.relative_to(build_root).as_posix()] = digest(path)
        if not entries:
            raise RuntimeError(f"Native dependency record has no source provenance: {dep}")
        closure[dep.relative_to(build_root).as_posix()] = dict(sorted(entries.items()))
    if not closure:
        raise RuntimeError("Native build has no compiler dependency/source closure")
    return closure, dict(sorted(generated.items()))


def build(target, output, jobs):
    if target not in TARGETS:
        raise ValueError(f"Unsupported native fingerprint target: {target}")
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if " " in str(output):
        raise ValueError("FFmpeg source build directory must not contain spaces; set OPENTAG_AUDIO_BUILD_ROOT")
    spec = json.loads(SOURCES.read_text())
    inputs = recipe_inputs(spec)
    recipe = hashlib.sha256(json.dumps(inputs, sort_keys=True).encode() + target.encode()).hexdigest()
    work = output / target / recipe[:20]
    prefix = work / "prefix"
    manifest = prefix / "opentag-audio-manifest.json"
    if manifest.exists():
        validate_prefix(prefix, spec, recipe, inputs, target)
        return prefix
    for directory in [output / "downloads", work / "sources", prefix / "include", prefix / "lib"]:
        directory.mkdir(parents=True, exist_ok=True)
    native = {name: acquire(component, output / "downloads", work / "sources")
              for name, component in spec["components"].items()}
    apply_patches(spec, native)
    env = os.environ.copy()
    cmake_options, ffmpeg_target, bash, make = target_tools(target, env)
    commands = []

    def cmake(name, source, options=()):
        directory = work / (name + "-build")
        command = ["cmake", "-S", source, "-B", directory, "-DCMAKE_INSTALL_PREFIX=" + str(prefix),
                   "-DBUILD_SHARED_LIBS=OFF", *cmake_options, *options]
        commands.append([str(x) for x in command])
        run(command, work, work / (name + ".log"), env)
        run(["cmake", "--build", directory, "--config", "Release", "--parallel", str(jobs)], work, work / (name + ".log"), env)
        run(["cmake", "--install", directory, "--config", "Release"], work, work / (name + ".log"), env)

    cmake("zlib", native["zlib"], ["-DZLIB_BUILD_TESTING=OFF", "-DZLIB_BUILD_SHARED=OFF", "-DZLIB_BUILD_STATIC=ON"])
    if target.endswith("windows-msvc"):
        # Pinned zlib 1.3.2 names its Windows static target z + suffix "s".
        stage_windows_zlib(prefix)
    cmake("bzip2", HERE / "bzip2", ["-DBZIP2_SOURCE_DIR=" + str(native["bzip2"])])
    ffmpeg_build = work / "ffmpeg-build"
    ffmpeg_build.mkdir(exist_ok=True)
    command = ffmpeg_command(bash, native["ffmpeg"], prefix, spec, ffmpeg_target)
    commands.append([str(x) for x in command])
    run(command, ffmpeg_build, work / "ffmpeg.log", env)
    run([make, "-j" + str(jobs)], ffmpeg_build, work / "ffmpeg.log", env)
    run([make, "install"], ffmpeg_build, work / "ffmpeg.log", env)
    config = (ffmpeg_build / "config.h").read_text()
    components = (ffmpeg_build / "config_components.h").read_text()
    for item in ["GPL", "NONFREE", "VERSION3", "NETWORK"]:
        if not re.search(rf"^#define CONFIG_{item} 0$", config, re.M):
            raise RuntimeError(f"Unexpected FFmpeg feature/license setting: {item}")
    if re.search(r"^#define CONFIG_\w+_PROTOCOL 1$", components, re.M):
        raise RuntimeError("Embedded FFmpeg must not contain URL/file protocols")
    actual_decoders = sorted(x.lower() for x in re.findall(r"^#define CONFIG_(\w+)_DECODER 1$", components, re.M))
    missing = sorted(set(spec["ffmpeg"]["decoders"]) - set(actual_decoders))
    if missing:
        raise RuntimeError("Requested native audio decoders were not built: " + ", ".join(missing))
    fft = "vdsp" if "apple" in target else "kissfft"
    cmake("chromaprint", native["chromaprint"], ["-DBUILD_TOOLS=OFF", "-DBUILD_TESTS=OFF",
          "-DUSE_INTERNAL_AVRESAMPLE=OFF", "-DFFT_LIB=" + fft,
          "-DKISSFFT_SOURCE_DIR=" + str(native["chromaprint"] / "src/3rdparty/kissfft"),
          "-DCMAKE_DISABLE_FIND_PACKAGE_FFmpeg=TRUE"])
    notice_dir = prefix / "notices"
    notice_dir.mkdir(exist_ok=True)
    for name, component in spec["components"].items():
        directory = notice_dir / name
        directory.mkdir(exist_ok=True)
        for filename in component["license_files"]:
            path = native[name] / filename
            shutil.copyfile(path, directory / (filename.replace("/", "_")))
    (prefix / "ffmpeg-config.h").write_text(config)
    (prefix / "ffmpeg-components.h").write_text(components)
    # Bind compiled objects to their actual original/modified source and header
    # closure. System SDK/compiler headers are intentionally outside this set.
    closure, generated = source_closure(ffmpeg_build, native["ffmpeg"], msvc=target.endswith("windows-msvc"))
    (prefix / "ffmpeg-source-closure.json").write_text(json.dumps(closure, indent=2) + "\n")
    for name, expected in generated.items():
        output = prefix / "ffmpeg-generated" / name
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ffmpeg_build / name, output)
        verify_archive(output, expected)
    (prefix / "ffmpeg-generated-closure.json").write_text(json.dumps(generated, indent=2) + "\n")
    shutil.copyfile(native["ffmpeg"] / "configure", prefix / "ffmpeg-patched-configure")
    for patch in spec.get("patches", []):
        shutil.copyfile(HERE / patch["file"], notice_dir / Path(patch["file"]).name)
    libraries = {p.relative_to(prefix).as_posix(): digest(p) for p in sorted((prefix / "lib").iterdir())
                 if p.suffix in {".a", ".lib"}}
    result = {"format": 1, "target": target, "recipe_sha256": recipe,
              "components": spec["components"], "configure_commands": commands,
              "toolchain": {"ffmpeg_compiler": env.get("OPENTAG_AUDIO_SELECTED_COMPILER",
                  next((flag[5:] for flag in ffmpeg_target if flag.startswith("--cc=")), env.get("CC", "cc")))},
              "build_inputs": inputs, "patch_metadata": spec.get("patches", []),
              "ffmpeg_decoders": actual_decoders, "ffmpeg_demuxers": spec["ffmpeg"]["demuxers"],
              "fft": fft, "runtime_programs": [], "decoder_protocols": [],
              "static_libraries": libraries,
              "prefix_files": {p.relative_to(prefix).as_posix(): digest(p) for p in sorted(prefix.rglob("*")) if p.is_file()}}
    manifest.write_text(json.dumps(result, indent=2) + "\n")
    return prefix


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--jobs", type=int, default=min(4, os.cpu_count() or 1))
    args = parser.parse_args()
    if not 1 <= args.jobs <= 16:
        parser.error("--jobs must be between 1 and 16")
    print(build(args.target, args.output, args.jobs))


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
