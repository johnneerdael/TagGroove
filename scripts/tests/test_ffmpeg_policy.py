"""Exercise pinned patch application and the no-GPL object boundary without app builds."""
import difflib
import hashlib
import importlib.util
from pathlib import Path
import unittest
import base64
import json
import tempfile
import io
import tarfile
from unittest.mock import patch

POLICY = Path(__file__).resolve().parents[1] / 'audio-deps/ffmpeg_policy.py'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


class FFmpegPolicyTests(unittest.TestCase):
    def load_script(self, path, name):
        spec = importlib.util.spec_from_file_location(name, path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def module(self):
        self.assertTrue(POLICY.is_file(), 'Missing shared pinned FFmpeg policy verifier')
        spec = importlib.util.spec_from_file_location('ffmpeg_policy', POLICY)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def fixture(self, target='libavutil/crc.c'):
        original = b'first\n#if ARCH_AARCH64\narch();\n#endif\nlast\n'
        patched = original.replace(b'#if ARCH_AARCH64', b'#if ARCH_AARCH64 && CONFIG_GPL')
        diff = ''.join(difflib.unified_diff(original.decode().splitlines(True),
                       patched.decode().splitlines(True), 'a/' + target, 'b/' + target)).encode()
        entry = {'component': 'ffmpeg', 'target': target, 'file': 'patches/crc.patch',
                 'sha256': sha(diff), 'original_sha256': sha(original), 'patched_sha256': sha(patched)}
        return entry, original, patched, diff

    def test_patch_replay_uses_exact_context_and_all_three_pinned_hashes(self):
        m = self.module()
        entry, original, expected, diff = self.fixture()
        self.assertEqual(m.apply_patch(entry, original, diff), expected)
        for field in ['sha256', 'original_sha256', 'patched_sha256']:
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, 'hash'):
                m.apply_patch(dict(entry, **{field: '0' * 64}), original, diff)
        damaged = diff.replace(b' arch();', b' different();')
        with self.assertRaisesRegex(ValueError, 'context'):
            m.apply_patch(dict(entry, sha256=sha(damaged)), original, damaged)
        shifted = diff.replace(b'@@ -1,5 +1,5 @@', b'@@ -2,5 +2,5 @@')
        with self.assertRaises(ValueError):
            m.apply_patch(dict(entry, sha256=sha(shifted)), original, shifted)

    def test_patch_set_refuses_unknown_duplicate_and_conflicting_targets(self):
        m = self.module()
        entry, original, expected, diff = self.fixture()
        m.validate_patches([entry])
        for entries in [[entry, entry], [dict(entry, target='../crc.c')],
                        [dict(entry, target='libavcodec/decoder.c')],
                        [dict(entry, component='chromaprint')]]:
            with self.subTest(entries=entries), self.assertRaises(ValueError):
                m.validate_patches(entries)
        wrong = diff.replace(b'a/libavutil/crc.c', b'a/libavcodec/ac3dsp.c')
        with self.assertRaisesRegex(ValueError, 'target'):
            m.apply_patch(dict(entry, sha256=sha(wrong)), original, wrong)

    def test_no_gpl_boundary_rejects_each_confirmed_source_even_with_other_valid_sources(self):
        m = self.module()
        m.verify_no_gpl_sources({'libavcodec/ac3dsp.c', 'libavutil/crc.c', 'libavutil/aarch64/cpu.c'})
        for name in ['libavcodec/aarch64/ac3dsp_init_aarch64.c',
                     'libavcodec/aarch64/ac3dsp_neon.S', 'libavutil/aarch64/crc.S']:
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'GPL'):
                m.verify_no_gpl_sources({'libavcodec/ac3dsp.c', name})

    def test_native_builder_applies_non_configure_patch_from_verified_bytes(self):
        builder = self.load_script(POLICY.parent / 'build.py', 'native_builder_policy_test')
        entry, original, expected, diff = self.fixture()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'patches').mkdir()
            (root / entry['file']).write_bytes(diff)
            target = root / 'source' / entry['target']
            target.parent.mkdir(parents=True)
            target.write_bytes(original)
            with patch.object(builder, 'HERE', root):
                builder.apply_patches({'patches': [entry]}, {'ffmpeg': root / 'source'})
            self.assertEqual(target.read_bytes(), expected)

    def test_source_manifest_rejects_gpl_objects_despite_disabled_config(self):
        builder = self.load_script(POLICY.parent.parent / 'audio-source-bundle.py', 'source_policy_test')
        closure = {'libavutil/aarch64/crc.d': {'libavutil/aarch64/crc.S': 'a' * 64}}
        wrapper = {'files': {
            'ffmpeg-source-closure.json': {'base64': base64.b64encode(json.dumps(closure).encode()).decode()},
            'ffmpeg-components.h': {'base64': base64.b64encode(b'').decode()}}}
        with self.assertRaisesRegex(ValueError, 'GPL'):
            builder.compiler_closure(wrapper, {'ffmpeg_decoders': [], 'patch_metadata': []})

    def test_compiler_closure_reconstructs_changed_source_instead_of_trusting_a_declared_hash(self):
        builder = self.load_script(POLICY.parent.parent / 'audio-source-bundle.py', 'source_replay_test')
        entry, original, expected, diff = self.fixture()
        stream = io.BytesIO()
        with tarfile.open(fileobj=stream, mode='w') as archive:
            member = tarfile.TarInfo('ffmpeg-test/' + entry['target'])
            member.size = len(original)
            archive.addfile(member, io.BytesIO(original))
        raw = stream.getvalue()
        builder.verify_upstream_closure(raw, 'ffmpeg-test', {entry['target']: sha(expected)},
                                       {entry['target']: (entry, diff)})
        for reported, altered in [(sha(original), entry), ('0' * 64, dict(entry, patched_sha256='0' * 64))]:
            with self.subTest(reported=reported), self.assertRaises(ValueError):
                builder.verify_upstream_closure(raw, 'ffmpeg-test', {entry['target']: reported},
                                               {entry['target']: (altered, diff)})


if __name__ == '__main__':
    unittest.main()
