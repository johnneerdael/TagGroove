"""Read-only binary-package controls; no installation or target code execution."""
import hashlib
import importlib.util
import io
from pathlib import Path
import shutil
import tarfile
import tracemalloc
import unittest


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


if __name__ == '__main__':unittest.main()
