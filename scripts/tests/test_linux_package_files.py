"""Read-only binary-package controls; no installation or target code execution."""
import hashlib
import importlib.util
import io
from pathlib import Path
import shutil
import sys
import tarfile
import tracemalloc
import unittest
import tempfile


SCRIPT = Path(__file__).resolve().parents[1]/'linux-package-files.py'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def archive(entries):
    stream = io.BytesIO()
    with tarfile.open(fileobj=stream, mode='w:xz') as handle:
        for name, value in entries:
            entry = tarfile.TarInfo(name)
            if isinstance(value, tuple):
                entry.type, entry.linkname = value
                handle.addfile(entry)
            else:
                entry.size = len(value);handle.addfile(entry, io.BytesIO(value))
    return stream.getvalue()


def deb(data, control=None):
    control = control or b'Package: libexample1\nVersion: 1.2-3+b1\nArchitecture: amd64\nSource: example (1.2-3)\nMaintainer: Fixture <fixture@example.invalid>\nDescription: Read-only package fixture\n'
    parts = [('debian-binary', b'2.0\n'), ('control.tar.xz', archive([('./control', control)])),
             ('data.tar.xz', archive(data))]
    raw = b'!<arch>\n'
    for name, body in parts:
        header = f'{name:<16}{0:<12}{0:<6}{0:<6}{"100644":<8}{len(body):<10}`\n'.encode()
        assert len(header) == 60
        raw += header + body + (b'\n' if len(body)%2 else b'')
    return raw


def selection(raw):
    return {'package': 'libexample1', 'version': '1.2-3+b1', 'architecture': 'amd64',
            'source_package': 'example', 'source_version': '1.2-3',
            'size': len(raw), 'sha256': digest(raw)}


@unittest.skipUnless(shutil.which('dpkg-deb'), 'Debian archive reader required')
class LinuxPackageFiles(unittest.TestCase):
    def module(self):
        self.assertTrue(SCRIPT.is_file(), 'Missing binary-package member verifier')
        spec = importlib.util.spec_from_file_location('linux_package_files', SCRIPT)
        module = importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        return module

    def test_authenticated_package_identity_and_in_archive_aliases_bind_original_bytes(self):
        m = self.module();payload = b'Original ELF input bytes; never executed'
        raw = deb([('./usr/lib/libexample.so.1.2', payload),
                   ('./usr/lib/libexample.so.1', (tarfile.SYMTYPE, 'libexample.so.1.2')),
                   ('./usr/lib/hardlink', (tarfile.LNKTYPE, './usr/lib/libexample.so.1.2'))])
        result = m.package_members(raw, selection(raw), ['usr/lib/libexample.so.1', 'usr/lib/hardlink'])
        self.assertEqual(result['usr/lib/libexample.so.1'], {'path': 'usr/lib/libexample.so.1.2', 'bytes': payload})
        self.assertEqual(result['usr/lib/hardlink']['bytes'], payload)
        with self.assertRaises(ValueError):m.package_members(raw+b'changed', selection(raw), ['usr/lib/libexample.so.1'])

    def test_member_lookup_never_follows_an_archive_link_into_the_host_filesystem(self):
        m = self.module()
        for target in ['../../../outside', '/etc/passwd', 'missing', 'loop']:
            with self.subTest(target=target):
                raw = deb([('./usr/lib/loop', (tarfile.SYMTYPE, target))])
                with self.assertRaises(ValueError):m.package_members(raw, selection(raw), ['usr/lib/loop'])
        raw = deb([('./usr/lib/a', b'first'), ('usr/lib/a', b'second')])
        with self.assertRaises(ValueError):m.package_members(raw, selection(raw), ['usr/lib/a'])

    def test_repository_and_package_control_identity_must_agree(self):
        m = self.module();raw = deb([('./usr/lib/a', b'bytes')]);selected = selection(raw)
        for field, value in [('package', 'other'), ('version', '1.2-4'), ('architecture', 'arm64'),
                             ('source_package', 'other'), ('source_version', '1.2-2')]:
            with self.subTest(field=field),self.assertRaises(ValueError):
                m.package_members(raw, {**selected, field: value}, ['usr/lib/a'])
        with self.assertRaises(ValueError):m.package_members(raw, selected, ['../usr/lib/a'])
        with self.assertRaises(ValueError):m.package_members(raw, selected, ['usr/lib/missing'])

    def test_oversized_control_is_refused_before_buffering_its_body(self):
        m = self.module()
        control = (b'Package: libexample1\nVersion: 1.2-3+b1\nArchitecture: amd64\n'
                   b'Source: example (1.2-3)\nDescription: ' + b'x'*(24*1024*1024) + b'\n')
        raw = deb([('./usr/lib/a', b'bytes')], control=control)
        selected = selection(raw)
        tracemalloc.start()
        try:
            with self.assertRaises(ValueError):m.package_members(raw, selected, ['usr/lib/a'])
            _current, peak = tracemalloc.get_traced_memory()
        finally:
            tracemalloc.stop()
        self.assertLess(peak, 12*1024*1024, 'Oversized control was buffered before refusal')

    def tools(self, root):
        code = f'#!{sys.executable}\n' + '''import sys
from pathlib import Path
path = Path(sys.argv[-1]);raw = path.read_bytes()
if Path(sys.argv[0]).name == 'strip':
    path.write_bytes(raw.replace(b'|debug', b''))
elif sys.argv[1] == '--print-rpath':
    print(raw.split(b'rpath=',1)[1].split(b'|',1)[0].decode())
else:
    old = raw.split(b'rpath=',1)[1].split(b'|',1)[0]
    path.write_bytes(raw.replace(b'rpath='+old, b'rpath='+sys.argv[2].encode(),1))
'''
        tools, pins = {}, {}
        for name in ['strip', 'patchelf']:
            path = root/name;path.write_text(code);path.chmod(0o755)
            tools[name] = path;pins[name] = {'sha256': digest(path.read_bytes()), 'size': path.stat().st_size}
        return tools, pins

    def test_exact_packaging_replay_refuses_changed_payload_and_preserves_relative_rpath_policy(self):
        m = self.module()
        self.assertTrue(callable(getattr(m, 'replay_packaging', None)), 'Missing exact packaging transformation replay')
        with tempfile.TemporaryDirectory() as folder:
            tools, pins = self.tools(Path(folder))
            original = b'\x7fELF|debug|rpath=|payload=real-code'
            packaged = b'\x7fELF|rpath=$ORIGIN|payload=real-code'
            result = m.replay_packaging(original, packaged, tools, pins, expected_rpath="$ORIGIN")
            self.assertEqual(result['steps'], ['strip', 'set-rpath'])
            self.assertEqual(result['sha256'], digest(packaged))
            with self.assertRaises(ValueError):
                m.replay_packaging(original, packaged.replace(b'real-code', b'changed-code'), tools, pins, expected_rpath='$ORIGIN')
            relative = b'\x7fELF|debug|rpath=$ORIGIN/old|payload=real-code'
            result = m.replay_packaging(relative, relative.replace(b'$ORIGIN/old', b'$ORIGIN'), tools, pins, expected_rpath='$ORIGIN')
            self.assertEqual(result['steps'], ['set-rpath'])
            self.assertEqual(m.replay_packaging(b'identical', b'identical', {}, {})['steps'], [])

    def test_packaging_replay_refuses_changed_tools_before_executing_them(self):
        m = self.module()
        self.assertTrue(callable(getattr(m, 'replay_packaging', None)), 'Missing pinned packaging tool verification')
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder);tools, pins = self.tools(root)
            tools['strip'].write_text(f'#!{sys.executable}\nfrom pathlib import Path\nPath({str(root/"executed")!r}).touch()\n')
            with self.assertRaises(ValueError):
                m.replay_packaging(b'\x7fELF|debug|rpath=|payload=code', b'\x7fELF|rpath=$ORIGIN|payload=code', tools, pins, expected_rpath='$ORIGIN')
            self.assertFalse((root/'executed').exists())

    def test_webkit_path_rewrite_requires_explicit_recipe_and_exact_final_bytes(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            tools, pins = self.tools(Path(folder))
            original = b'\x7fELF|debug|rpath=|payload=/usr/lib/webkit:/usr/share'
            packaged = b'\x7fELF|rpath=$ORIGIN|payload=././/lib/webkit:././/share'
            with self.assertRaises(ValueError):m.replay_packaging(original, packaged, tools, pins, expected_rpath="$ORIGIN")
            result = m.replay_packaging(original, packaged, tools, pins, expected_rpath="$ORIGIN", webkit_relative_usr=True)
            self.assertEqual(result['steps'], ['strip', 'set-rpath', 'webkit-relative-usr'])
            with self.assertRaises(ValueError):
                m.replay_packaging(original, packaged+b'changed', tools, pins, expected_rpath="$ORIGIN", webkit_relative_usr=True)

    def test_artifact_cannot_choose_an_untrusted_rpath_as_a_replay_argument(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            tools, pins = self.tools(Path(folder))
            original = b'\x7fELF|debug|rpath=|payload=real-code'
            malicious = b'\x7fELF|rpath=/tmp/untrusted|payload=real-code'
            with self.assertRaises(ValueError):m.replay_packaging(original, malicious, tools, pins)
            with self.assertRaises(ValueError):
                m.replay_packaging(original, malicious, tools, pins, expected_rpath='$ORIGIN')


if __name__ == '__main__':unittest.main()
