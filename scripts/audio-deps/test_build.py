import hashlib
import importlib.util
import io
import json
import os
from unittest import mock
from pathlib import Path
import tarfile
import tempfile
import unittest

BUILDER = Path(__file__).with_name("build.py")

class SourceSafetyTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(BUILDER.is_file(), "The pinned native source builder is missing")
        spec = importlib.util.spec_from_file_location("audio_build", BUILDER)
        self.build = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.build)

    def test_tampered_source_is_rejected_before_extraction(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "source.tar.gz"
            path.write_bytes(b"tampered source")
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                self.build.verify_archive(path, hashlib.sha256(b"trusted source").hexdigest())

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

    def test_pinned_patch_bytes_do_not_depend_on_windows_text_newlines(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            target = root / "configure"
            needle = 'enabled gpl && enable lgpl_gpl # Files that are marked as LGPL but some developers prefer only building them with --enable-gpl'
            original = ("#!/bin/sh\n" + needle + "\n").encode()
            expected = original.replace(needle.encode(), (needle + '\n# OpenTag opts in to these explicitly LGPL-marked audio components only.\n# GPL/nonfree/version3 remain disabled; the compiled object closure is audited.\nenable lgpl_gpl').encode())
            target.write_bytes(original)
            spec = {"patches":[{"component":"ffmpeg","target":"configure", "original_sha256":hashlib.sha256(original).hexdigest(), "patched_sha256":hashlib.sha256(expected).hexdigest()}]}
            def windows_text_write(path, text, **kwargs):
                return path.write_bytes(text.replace("\n","\r\n").encode("utf-8"))
            with mock.patch.object(Path,"write_text",autospec=True,side_effect=windows_text_write):
                self.build.apply_patches(spec,{"ffmpeg":root})
            self.assertEqual(target.read_bytes(),expected)
            self.assertNotIn(b"\r\n",target.read_bytes())

if __name__ == "__main__":
    unittest.main()
