import hashlib
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import importlib.util
import io
import json
import os
from unittest import mock
from pathlib import Path, PurePosixPath, PureWindowsPath
import subprocess
import sys
import tarfile
import tempfile
import threading
import unittest
import urllib.error

BUILDER = Path(__file__).with_name("build.py")


@contextmanager
def source_server(payload, status=200):
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            body = payload() if callable(payload) else payload
            self.send_response(status)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *args):
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}/source.tar.gz"
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)


class SourceSafetyTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(BUILDER.is_file(), "The pinned native source builder is missing")
        spec = importlib.util.spec_from_file_location("audio_build", BUILDER)
        self.build = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.build)

    def test_apple_device_and_simulator_use_their_own_sdk_and_platform_flags(self):
        self.assertIn("aarch64-apple-ios", self.build.TARGETS)
        for target, sdk, flag in [
            ("aarch64-apple-ios", "iphoneos", "-target arm64-apple-ios13.0"),
            ("aarch64-apple-ios-sim", "iphonesimulator", "-target arm64-apple-ios13.0-simulator"),
            ("aarch64-apple-darwin", "macosx", "-mmacosx-version-min=11.0"),
        ]:
            def xcrun(command, **kwargs):
                self.assertEqual(command[0], "xcrun")
                requested = command[2]
                return (f"/sdk/{requested}\n" if command[-1] == "--show-sdk-path" else "/toolchain/clang\n")
            with self.subTest(target=target), mock.patch.object(self.build.subprocess, "check_output", side_effect=xcrun):
                cmake, ffmpeg, _, _ = self.build.target_tools(target, {})
                self.assertIn("-DCMAKE_OSX_SYSROOT=/sdk/" + sdk, cmake)
                self.assertIn("--cc=/toolchain/clang", ffmpeg)
                cflags = next(arg for arg in ffmpeg if arg.startswith("--extra-cflags="))
                self.assertIn(flag, cflags)
                if sdk != "macosx":
                    self.assertIn("-DCMAKE_SYSTEM_NAME=iOS", cmake)
                    self.assertIn("--enable-cross-compile", ffmpeg)
                    self.assertNotIn("-mmacosx-version-min", cflags)

    def test_tampered_source_is_rejected_before_extraction(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "source.tar.gz"
            path.write_bytes(b"tampered source")
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                self.build.verify_archive(path, hashlib.sha256(b"trusted source").hexdigest())

    def test_empty_cache_downloads_verifies_and_extracts_source(self):
        payload = io.BytesIO()
        with tarfile.open(fileobj=payload, mode="w:gz") as archive:
            data = b"project-authored source fixture\n"
            member = tarfile.TarInfo("source/configure")
            member.size = len(data)
            archive.addfile(member, io.BytesIO(data))
        body = payload.getvalue()
        with tempfile.TemporaryDirectory() as folder, source_server(body) as url:
            root = Path(folder)
            downloads, sources = root / "downloads", root / "sources"
            downloads.mkdir()
            sources.mkdir()
            spec = {"archive": "source.tar.gz", "directory": "source",
                    "sha256": hashlib.sha256(body).hexdigest(), "url": url}
            with mock.patch.dict(os.environ, {"OPENTAG_AUDIO_SOURCE_DIR": ""}):
                result = self.build.acquire(spec, downloads, sources)
            self.assertEqual(result, sources / "source")
            self.assertEqual((result / "configure").read_bytes(), data)
            self.assertEqual((downloads / "source.tar.gz").read_bytes(), body)
            self.assertEqual(list(downloads.iterdir()), [downloads / "source.tar.gz"])

    def test_failed_downloads_leave_no_cache_or_temporary_files(self):
        for status, expected in [(200, ValueError), (503, urllib.error.HTTPError)]:
            with self.subTest(status=status), tempfile.TemporaryDirectory() as folder, source_server(b"untrusted", status) as url:
                root = Path(folder)
                downloads, sources = root / "downloads", root / "sources"
                downloads.mkdir()
                sources.mkdir()
                spec = {"archive": "source.tar.gz", "directory": "source",
                        "sha256": "0" * 64, "url": url}
                with mock.patch.dict(os.environ, {"OPENTAG_AUDIO_SOURCE_DIR": ""}), self.assertRaises(expected):
                    self.build.acquire(spec, downloads, sources)
                self.assertEqual(list(downloads.iterdir()), [])
                self.assertEqual(list(sources.iterdir()), [])

    def test_transient_bad_download_retries_without_accepting_wrong_bytes(self):
        payload = io.BytesIO()
        with tarfile.open(fileobj=payload, mode="w:gz") as archive:
            content = b"verified retry source"
            member = tarfile.TarInfo("source/configure")
            member.size = len(content)
            archive.addfile(member, io.BytesIO(content))
        body = payload.getvalue()
        attempts = []
        def response():
            attempts.append(1)
            return b"transient invalid response" if len(attempts) == 1 else body
        with tempfile.TemporaryDirectory() as folder, source_server(response) as url:
            root = Path(folder)
            downloads, sources = root / "downloads", root / "sources"
            downloads.mkdir()
            sources.mkdir()
            spec = {"archive":"source.tar.gz", "directory":"source",
                    "sha256":hashlib.sha256(body).hexdigest(), "url":url}
            with mock.patch.dict(os.environ, {"OPENTAG_AUDIO_SOURCE_DIR":""}):
                result = self.build.acquire(spec, downloads, sources)
            self.assertEqual(len(attempts), 2)
            self.assertEqual((downloads / spec["archive"]).read_bytes(), body)
            self.assertEqual((result / "configure").read_bytes(), content)
            self.assertEqual(list(downloads.iterdir()), [downloads / spec["archive"]])

    def test_persistently_bad_download_is_bounded_and_names_the_archive(self):
        attempts = []
        def response():
            attempts.append(1)
            return b"untrusted response"
        with tempfile.TemporaryDirectory() as folder, source_server(response) as url:
            root = Path(folder)
            downloads, sources = root / "downloads", root / "sources"
            downloads.mkdir()
            sources.mkdir()
            expected = "0" * 64
            actual = hashlib.sha256(b"untrusted response").hexdigest()
            spec = {"archive":"identified-source.tar.gz", "directory":"source",
                    "sha256":expected, "url":url}
            with mock.patch.dict(os.environ, {"OPENTAG_AUDIO_SOURCE_DIR":""}), self.assertRaises(ValueError) as error:
                self.build.acquire(spec, downloads, sources)
            self.assertIn(spec["archive"], str(error.exception))
            self.assertIn(expected, str(error.exception))
            self.assertIn(actual, str(error.exception))
            self.assertEqual(len(attempts), 3)
            self.assertEqual(list(downloads.iterdir()), [])
            self.assertEqual(list(sources.iterdir()), [])

    def test_bad_primary_uses_a_declared_mirror_with_the_same_archive_pin(self):
        payload = io.BytesIO()
        with tarfile.open(fileobj=payload, mode="w:gz") as archive:
            member = tarfile.TarInfo("source/configure")
            member.size = 8
            archive.addfile(member, io.BytesIO(b"verified"))
        body = payload.getvalue()
        attempts = []
        def primary():
            attempts.append("primary")
            return b"bad cached response"
        def mirror():
            attempts.append("mirror")
            return body
        for status in [200, 503]:
            with self.subTest(status=status), tempfile.TemporaryDirectory() as folder, source_server(primary, status) as url, source_server(mirror) as alternate:
                attempts.clear()
                root = Path(folder)
                downloads, sources = root / "downloads", root / "sources"
                downloads.mkdir()
                sources.mkdir()
                spec = {"archive":"source.tar.gz", "directory":"source",
                        "sha256":hashlib.sha256(body).hexdigest(), "url":url,
                        "mirrors":[alternate]}
                with mock.patch.dict(os.environ, {"OPENTAG_AUDIO_SOURCE_DIR":""}), mock.patch.object(self.build.time, "sleep"):
                    result = self.build.acquire(spec, downloads, sources)
                self.assertEqual(attempts, ["primary", "mirror"])
                self.assertEqual((downloads / spec["archive"]).read_bytes(), body)
                self.assertEqual((result / "configure").read_bytes(), b"verified")
                self.assertEqual(list(downloads.iterdir()), [downloads / spec["archive"]])

    def test_bad_mirrors_share_the_three_attempt_limit_and_never_publish_bytes(self):
        attempts = []
        def response(name):
            def body():
                attempts.append(name)
                return name.encode()
            return body
        with tempfile.TemporaryDirectory() as folder, source_server(response("primary")) as url, source_server(response("mirror")) as alternate:
            root = Path(folder)
            downloads, sources = root / "downloads", root / "sources"
            downloads.mkdir()
            sources.mkdir()
            spec = {"archive":"source.tar.gz", "directory":"source",
                    "sha256":"0" * 64, "url":url, "mirrors":[alternate]}
            with mock.patch.dict(os.environ, {"OPENTAG_AUDIO_SOURCE_DIR":""}), mock.patch.object(self.build.time, "sleep"), self.assertRaises(ValueError) as error:
                self.build.acquire(spec, downloads, sources)
            self.assertEqual(attempts, ["primary", "mirror", "primary"])
            self.assertIn(spec["sha256"], str(error.exception))
            self.assertEqual(list(downloads.iterdir()), [])
            self.assertEqual(list(sources.iterdir()), [])

    def test_tampered_existing_cache_is_not_replaced_by_a_network_retry(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            downloads, sources = root / "downloads", root / "sources"
            downloads.mkdir()
            sources.mkdir()
            cached = downloads / "source.tar.gz"
            cached.write_bytes(b"tampered cache")
            spec = {"archive":cached.name, "directory":"source", "sha256":"0" * 64,
                    "url":"https://example.invalid/source"}
            with mock.patch.object(self.build.urllib.request, "urlopen") as request, self.assertRaises(ValueError):
                self.build.acquire(spec, downloads, sources)
            request.assert_not_called()
            self.assertEqual(cached.read_bytes(), b"tampered cache")
            self.assertEqual(list(sources.iterdir()), [])

    def test_archive_cannot_escape_owned_source_directory(self):
        for name, link in [("../escaped", None), ("root/../../escaped", None), ("root/link", "/tmp/escaped"), ("C:/escaped", None), ("root\\escaped", None)]:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as folder:
                path = Path(folder) / "source.tar"
                with tarfile.open(path, "w") as archive:
                    member = tarfile.TarInfo(name)
                    if link:
                        member.type = tarfile.SYMTYPE
                        member.linkname = link
                        archive.addfile(member)
                    else:
                        member.size = 1
                        archive.addfile(member, io.BytesIO(b"x"))
                with self.assertRaisesRegex(ValueError, "Unsafe archive"):
                    self.build.extract_archive(path, Path(folder) / "out")
                self.assertFalse((Path(folder) / "escaped").exists())

    def test_regular_source_is_extracted_without_running_it(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "source.tar"
            with tarfile.open(path, "w") as archive:
                member = tarfile.TarInfo("source/configure")
                content = b"do not execute this during extraction"
                member.size = len(content)
                archive.addfile(member, io.BytesIO(content))
            output = Path(folder) / "out"
            self.build.extract_archive(path, output)
            self.assertEqual((output / "source/configure").read_bytes(), content)

    def test_explicit_offline_source_directory_never_falls_back_to_network(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            downloads = root / "downloads"
            sources = root / "sources"
            downloads.mkdir()
            sources.mkdir()
            spec = {"archive":"missing.tar.gz", "directory":"missing", "sha256":"0" * 64, "url":"https://example.invalid/source"}
            with mock.patch.dict(os.environ, {"OPENTAG_AUDIO_SOURCE_DIR":str(root / "offline")}), mock.patch.object(self.build.urllib.request, "urlopen") as request:
                with self.assertRaises(FileNotFoundError):
                    self.build.acquire(spec, downloads, sources)
                request.assert_not_called()

    def test_completed_cache_rejects_changed_headers_and_inventory(self):
        with tempfile.TemporaryDirectory() as folder:
            prefix = Path(folder) / "target" / "recipe" / "prefix"
            (prefix / "include").mkdir(parents=True)
            (prefix / "lib").mkdir()
            header = prefix / "include/audio.h"
            library = prefix / "lib/audio.a"
            header.write_bytes(b"original header")
            library.write_bytes(b"original archive")
            files = {p.relative_to(prefix).as_posix():self.build.digest(p) for p in [header, library]}
            manifest = {"format":1,"target":"test-target","recipe_sha256":"recipe", "build_inputs":{"recipe":"sha"},"components":{},"prefix_files":files,"static_libraries":{"lib/audio.a":files["lib/audio.a"]}}
            (prefix / "opentag-audio-manifest.json").write_text(json.dumps(manifest))
            self.build.validate_prefix(prefix, {"components":{}}, "recipe", {"recipe":"sha"}, "test-target")
            header.write_bytes(b"tampered header")
            with self.assertRaisesRegex(RuntimeError,"installed-file verification"):
                self.build.validate_prefix(prefix, {"components":{}}, "recipe", {"recipe":"sha"}, "test-target")
            header.write_bytes(b"original header")
            (prefix / "unexpected.h").write_bytes(b"extra")
            with self.assertRaisesRegex(RuntimeError,"inventory changed"):
                self.build.validate_prefix(prefix, {"components":{}}, "recipe", {"recipe":"sha"}, "test-target")

    def test_source_closure_resolves_real_ffmpeg_src_aliases_and_generated_inputs(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / "source"
            build = root / "build"
            (source / "libavcodec").mkdir(parents=True)
            (build / "libavcodec").mkdir(parents=True)
            original = source / "libavcodec/adpcm.c"
            original.write_bytes(b"LGPL source")
            alias = build / "src/libavcodec/adpcm.c"
            alias.parent.mkdir(parents=True)
            alias.write_bytes(original.read_bytes())
            (build / "config.h").write_bytes(b"generated config")
            (build / "libavcodec/adpcm.d").write_text("libavcodec/adpcm.o: src/libavcodec/adpcm.c config.h\n")
            closure, generated = self.build.source_closure(build, source)
            self.assertEqual(closure, {"libavcodec/adpcm.d":{"libavcodec/adpcm.c":self.build.digest(original)}})
            self.assertEqual(generated, {"config.h":self.build.digest(build / "config.h")})
            (build / "libavcodec/adpcm.d").write_text("libavcodec/adpcm.o: config.h\n")
            with self.assertRaisesRegex(RuntimeError,"no source provenance"):
                self.build.source_closure(build, source)

    def test_windows_compiler_with_spaces_is_selected_through_explicit_path(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            msys = root / "selected-msys"
            (msys / "usr/bin").mkdir(parents=True)
            for name in ["bash.exe", "make.exe"]:
                (msys / "usr/bin" / name).touch()
            compiler = root / "Program Files/Visual Studio/bin/cl.exe"
            compiler.parent.mkdir(parents=True)
            compiler.touch()
            env = {"OPENTAG_MSYS2":str(msys),"CC":str(compiler),"PATH":"old-search-path"}
            with mock.patch.object(self.build.shutil,"which",return_value=str(compiler)):
                _, flags, _, _ = self.build.target_tools("x86_64-pc-windows-msvc",env)
            self.assertIn("--cc=cl.exe", flags)
            self.assertFalse(any(flag.startswith("--cc=") and " " in flag for flag in flags))
            self.assertEqual(env["PATH"].split(os.pathsep)[0], str(compiler.parent))

    def test_msvc_header_records_bind_compiled_translation_unit(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source, build = root / "source", root / "build"
            (source / "libavcodec").mkdir(parents=True)
            (build / "libavcodec").mkdir(parents=True)
            unit = source / "libavcodec/adpcm_data.c"
            unit.write_bytes(b"#include <stdint.h>\nconst int table[] = {1};\n")
            header = source / "libavcodec/local.h"
            header.write_bytes(b"original pinned header\n")
            alias = build / "src/libavcodec/local.h"
            alias.parent.mkdir(parents=True)
            alias.write_bytes(header.read_bytes())
            dep = build / "libavcodec/adpcm_data.d"
            (build / "libavcodec/adpcm_data.o").write_bytes(b"synthetic compiled-object fixture")
            for record in ["", "libavcodec/adpcm_data.o: src/libavcodec/local.h\n"]:
                with self.subTest(record=record):
                    dep.write_text(record)
                    closure, generated = self.build.source_closure(build, source, msvc=True)
                    expected = {"libavcodec/adpcm_data.c": self.build.digest(unit)}
                    if record:
                        expected["libavcodec/local.h"] = self.build.digest(header)
                    self.assertEqual(closure, {"libavcodec/adpcm_data.d": expected})
                    self.assertEqual(generated, {})

    def test_msvc_empty_record_requires_object_source_and_matching_src_alias(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source, build = root / "source", root / "build"
            (source / "libavcodec").mkdir(parents=True)
            (build / "libavcodec").mkdir(parents=True)
            unit = source / "libavcodec/adpcm_data.c"
            unit.write_bytes(b"original source")
            (build / "libavcodec/adpcm_data.d").write_bytes(b"")
            with self.assertRaisesRegex(RuntimeError, "compiled object"):
                self.build.source_closure(build, source, msvc=True)
            (build / "libavcodec/adpcm_data.o").write_bytes(b"synthetic compiled-object fixture")
            unit.unlink()
            with self.assertRaisesRegex(RuntimeError, "translation unit"):
                self.build.source_closure(build, source, msvc=True)
            unit.write_bytes(b"original source")
            alternate = unit.with_suffix(".asm")
            alternate.write_bytes(b"ambiguous source")
            with self.assertRaisesRegex(RuntimeError, "translation unit"):
                self.build.source_closure(build, source, msvc=True)
            alternate.unlink()
            alias = build / "src/libavcodec/adpcm_data.c"
            alias.parent.mkdir(parents=True)
            alias.write_bytes(b"tampered source copy")
            with self.assertRaisesRegex(RuntimeError, "source alias"):
                self.build.source_closure(build, source, msvc=True)

    def test_dependency_header_alias_must_match_compiler_input(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source, build = root / "source", root / "build"
            (source / "libavcodec").mkdir(parents=True)
            (build / "libavcodec").mkdir(parents=True)
            (source / "libavcodec/unit.c").write_bytes(b"original translation unit")
            header = source / "libavcodec/local.h"
            header.write_bytes(b"original pinned header")
            (build / "libavcodec/unit.o").write_bytes(b"synthetic compiled-object fixture")
            (build / "libavcodec/unit.d").write_text("libavcodec/unit.o: src/libavcodec/local.h\n")
            alias = build / "src/libavcodec/local.h"
            alias.parent.mkdir(parents=True)
            for msvc, prefix in [(False, "src/"), (True, "src/"), (False, "src//"), (True, "src//")]:
                with self.subTest(msvc=msvc, prefix=prefix):
                    (build / "libavcodec/unit.d").write_text(f"libavcodec/unit.o: {prefix}libavcodec/local.h\n")
                    alias.write_bytes(b"changed compiler input")
                    with self.assertRaisesRegex(RuntimeError, "dependency alias"):
                        self.build.source_closure(build, source, msvc=msvc)
                    alias.unlink()
                    with self.assertRaisesRegex(RuntimeError, "dependency alias"):
                        self.build.source_closure(build, source, msvc=msvc)
                    alias.write_bytes(header.read_bytes())
                    closure, _ = self.build.source_closure(build, source, msvc=msvc)
                    self.assertEqual(closure["libavcodec/unit.d"]["libavcodec/local.h"], self.build.digest(alias))

    def test_windows_configure_paths_keep_source_directory_in_bash(self):
        source = PureWindowsPath(r"D:\a\opentag\sources\ffmpeg")
        prefix = PureWindowsPath(r"D:\a\opentag\prefix")
        spec = {"ffmpeg": {"decoders": [], "demuxers": [], "parsers": []}}
        command = self.build.ffmpeg_command("bash", source, prefix, spec, ["--toolchain=msvc"])
        self.assertEqual(command[1], "D:/a/opentag/sources/ffmpeg/configure")
        self.assertIn("--prefix=D:/a/opentag/prefix", command)
        self.assertIn("--extra-cflags=-ID:/a/opentag/prefix/include", command)
        self.assertIn("--extra-ldflags=-libpath:D:/a/opentag/prefix/lib", command)
        self.assertFalse(any(flag.startswith("--extra-ldflags=-L") for flag in command))
        if os.name != "nt":
            # The same Unix dirname operation used by upstream configure.
            actual = subprocess.check_output(["bash", "-c", 'dirname "$1"', "path-test", command[1]], text=True)
            self.assertEqual(actual.strip(), source.as_posix())

    def test_failed_configure_reports_its_compiler_diagnostic(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "ffbuild").mkdir()
            (root / "ffbuild/config.log").write_text("compiler probe: linker missing\n")
            with self.assertRaisesRegex(RuntimeError, "compiler probe: linker missing"):
                self.build.run([sys.executable, "-c", "raise SystemExit(1)"], root, root / "build.log")

    def test_posix_configure_keeps_unix_library_search_flag(self):
        spec = {"ffmpeg": {"decoders": [], "demuxers": [], "parsers": []}}
        command = self.build.ffmpeg_command("bash", PurePosixPath("/src/ffmpeg"), PurePosixPath("/prefix"), spec, [])
        self.assertIn("--extra-ldflags=-L/prefix/lib", command)

    def test_windows_zlib_stages_same_archive_for_ffmpeg_and_cargo(self):
        with tempfile.TemporaryDirectory() as folder:
            prefix = Path(folder)
            (prefix / "lib").mkdir()
            payload = b"pinned static zlib archive"
            (prefix / "lib/zs.lib").write_bytes(payload)
            self.build.stage_windows_zlib(prefix)
            self.assertEqual((prefix / "lib/zlib.lib").read_bytes(), payload)
            self.assertEqual((prefix / "lib/z.lib").read_bytes(), payload)

    def test_pinned_patch_bytes_do_not_depend_on_windows_text_newlines(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            target = root / "configure"
            needle = 'enabled gpl && enable lgpl_gpl # Files that are marked as LGPL but some developers prefer only building them with --enable-gpl'
            original = ("#!/bin/sh\n" + needle + "\n").encode()
            expected = original.replace(needle.encode(), (needle + '\n# OpenTag opts in to these explicitly LGPL-marked audio components only.\n# GPL/nonfree/version3 remain disabled; the compiled object closure is audited.\nenable lgpl_gpl').encode())
            target.write_bytes(original)
            patch = ('--- a/configure\n+++ b/configure\n@@ -1,2 +1,5 @@\n #!/bin/sh\n '
                     + needle + '\n+# OpenTag opts in to these explicitly LGPL-marked audio components only.\n'
                     '+# GPL/nonfree/version3 remain disabled; the compiled object closure is audited.\n'
                     '+enable lgpl_gpl\n').encode()
            (root / "patches").mkdir()
            (root / "patches/newline-fixture.patch").write_bytes(patch)
            spec = {"patches":[{"component":"ffmpeg","target":"configure",
                    "file":"patches/newline-fixture.patch", "sha256":hashlib.sha256(patch).hexdigest(),
                    "original_sha256":hashlib.sha256(original).hexdigest(), "patched_sha256":hashlib.sha256(expected).hexdigest()}]}
            def windows_text_write(path, text, **kwargs):
                return path.write_bytes(text.replace("\n","\r\n").encode("utf-8"))
            with mock.patch.object(self.build,"HERE",root), mock.patch.object(Path,"write_text",autospec=True,side_effect=windows_text_write):
                self.build.apply_patches(spec,{"ffmpeg":root})
            self.assertEqual(target.read_bytes(),expected)
            self.assertNotIn(b"\r\n",target.read_bytes())

if __name__ == "__main__":
    unittest.main()
