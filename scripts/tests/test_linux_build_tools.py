"""Pinned packaging inputs are staged only into owned checkout paths."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1]/'linux-build-tools.py'


def digest(raw):return hashlib.sha256(raw).hexdigest()


class LinuxBuildTools(unittest.TestCase):
    def module(self):
        self.assertTrue(SCRIPT.is_file(), 'Missing pinned Linux packaging preparation')
        spec = importlib.util.spec_from_file_location('linux_build_tools', SCRIPT)
        module = importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        return module

    def fixture(self, root):
        tool_bytes = {'strip': b'pinned-strip', 'patchelf': b'pinned-patchelf'}
        assets = {'https://example.invalid/linuxdeploy': b'linuxdeploy asset',
                  'https://example.invalid/runtime': b'pinned runtime',
                  'https://example.invalid/launcher': b'pinned launcher'}
        tools = {name: {'member': 'usr/bin/'+name, 'size': len(raw), 'sha256': digest(raw)}
                 for name, raw in tool_bytes.items()}
        cli = {'version': '2.12.2', 'integrity': 'locked-cli-integrity'}
        linuxdeploy = {'url': 'https://example.invalid/linuxdeploy', 'asset': 'linuxdeploy-x86_64.AppImage',
                       'size': len(assets['https://example.invalid/linuxdeploy']),
                       'sha256': digest(assets['https://example.invalid/linuxdeploy']),
                       'cli_modified_sha256': digest(b'modified-by-tauri'), 'tools': tools}
        catalog = {'schema': 1, 'tauri_cli': cli, 'linuxdeploy_commit': '07333c6',
                   'architectures': {'amd64': linuxdeploy}}
        runtime = {kind: {'url': 'https://example.invalid/'+kind, 'byte_length': len(raw), 'sha256': digest(raw)}
                   for kind, raw in [('runtime', b'pinned runtime'), ('launcher', b'pinned launcher')]}
        files = {'package-lock.json': {'packages': {'node_modules/@tauri-apps/cli': cli}},
                 'scripts/notices/linux-packager-tools.json': catalog,
                 'scripts/notices/linux-runtime-material.json': {'asset_pins': {'x86_64': runtime}},
                 'src/licenses/appimage-common/SOURCES.json': {'inspector': {'archives': [
                     {'arch': 'x86_64', 'binary_sha256': digest(b'host-inspector')}]}}}
        for name, value in files.items():
            path = root/name;path.parent.mkdir(parents=True, exist_ok=True);path.write_text(json.dumps(value))
        inspector = root/'7zz';inspector.write_bytes(b'host-inspector')
        return inspector, assets, tool_bytes

    def test_prepare_pins_all_inputs_and_reuses_only_verified_managed_cache(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder);inspector, assets, tools = self.fixture(root)
            with patch.object(m, 'extract_tools', return_value=tools):
                result = m.prepare(root, 'amd64', inspector, fetch=assets.__getitem__)
            self.assertEqual(result['runtime'].read_bytes(), b'pinned runtime')
            self.assertEqual(result['tools']['strip'].read_bytes(), tools['strip'])
            self.assertEqual((result['cache']/'tauri/AppRun-x86_64').read_bytes(), b'pinned launcher')
            cached = result['cache']/'tauri/linuxdeploy-07333c6-x86_64.AppImage'
            cached.write_bytes(b'modified-by-tauri')
            with patch.object(m, 'extract_tools', side_effect=AssertionError('Unexpected extraction')):
                reused = m.prepare(root, 'amd64', inspector, fetch=lambda _url: self.fail('Unexpected network'))
            self.assertEqual(reused['tools'], result['tools'])
            self.assertEqual(cached.read_bytes(), b'modified-by-tauri')

    def test_bad_pin_does_not_publish_partial_inputs_and_unknown_cache_is_retained(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder);inspector, assets, tools = self.fixture(root)
            bad = {**assets, 'https://example.invalid/launcher': b'changed'}
            with patch.object(m, 'extract_tools', return_value=tools),self.assertRaises(ValueError):
                m.prepare(root, 'amd64', inspector, fetch=bad.__getitem__)
            self.assertFalse((root/'target/linux-source-tools/amd64/runtime').exists())
            with patch.object(m, 'extract_tools', return_value=tools):
                result = m.prepare(root, 'amd64', inspector, fetch=assets.__getitem__)
            cached = result['cache']/'tauri/linuxdeploy-07333c6-x86_64.AppImage';cached.write_bytes(b'outside edit')
            with self.assertRaises(ValueError):m.prepare(root, 'amd64', inspector, fetch=assets.__getitem__)
            self.assertEqual(cached.read_bytes(), b'outside edit')

    def test_changed_cli_inspector_and_tool_bytes_are_refused(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder);inspector, assets, tools = self.fixture(root)
            with patch.object(m, 'extract_tools', return_value={**tools, 'strip': b'changed'}),self.assertRaises(ValueError):
                m.prepare(root, 'amd64', inspector, fetch=assets.__getitem__)
            inspector.write_bytes(b'changed')
            with self.assertRaises(ValueError):m.prepare(root, 'amd64', inspector, fetch=assets.__getitem__)
            inspector.write_bytes(b'host-inspector')
            lock = root/'package-lock.json';data = json.loads(lock.read_text())
            data['packages']['node_modules/@tauri-apps/cli']['version'] = 'new-cli';lock.write_text(json.dumps(data))
            with self.assertRaises(ValueError):m.prepare(root, 'amd64', inspector, fetch=assets.__getitem__)

    def test_symlink_output_parent_never_receives_staged_tools(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)/'repo';root.mkdir();inspector, assets, tools = self.fixture(root)
            outside = Path(folder)/'outside';outside.mkdir();(root/'target').symlink_to(outside, target_is_directory=True)
            with patch.object(m, 'extract_tools', return_value=tools),self.assertRaises(ValueError):
                m.prepare(root, 'amd64', inspector, fetch=assets.__getitem__)
            self.assertEqual(list(outside.iterdir()), [])

    def test_oversized_existing_cache_is_refused_before_reading_its_body(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder);inspector, assets, _tools = self.fixture(root)
            cache = root/'target/linux-source-tools/xdg-cache/tauri/linuxdeploy-07333c6-x86_64.AppImage'
            cache.parent.mkdir(parents=True)
            with cache.open('wb') as output:output.truncate(m.MAX_ASSET+1)
            original_read = Path.read_bytes
            def read(path):
                self.assertNotEqual(path, cache, 'Oversized cache body was read before checking its size')
                return original_read(path)
            with patch.object(Path, 'read_bytes', read),self.assertRaises(ValueError):
                m.prepare(root, 'amd64', inspector, fetch=assets.__getitem__)


if __name__ == '__main__':unittest.main()
