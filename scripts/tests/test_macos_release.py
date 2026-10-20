"""Focused release-signing controls; never build, launch or submit a real app."""
import base64
import importlib.util
import json
from pathlib import Path
import plistlib
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1]/'macos-release.py'
IDENTITY = 'Developer ID Application: John Neerdael (6DF93UDRBJ)'


class MacOSRelease(unittest.TestCase):
    def module(self):
        self.assertTrue(SCRIPT.is_file(), 'Missing gated macOS signing/release helper')
        spec = importlib.util.spec_from_file_location('macos_release', SCRIPT)
        module = importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        return module

    def fixture(self, root):
        config = {'productName': 'TagGroove', 'identifier': 'org.opentag.editor', 'version': '0.1.0'}
        for name, data in [('src-tauri/tauri.conf.json', config), ('src-tauri/macos/signing-policy.json',
                           {'schema': 1, 'team_id': '6DF93UDRBJ', 'identity': IDENTITY})]:
            path = root/name;path.parent.mkdir(parents=True, exist_ok=True);path.write_text(json.dumps(data))
        return {'APPLE_CERTIFICATE': base64.b64encode(b'fixture-p12').decode(),
                'APPLE_CERTIFICATE_PASSWORD': 'fixture-password',
                'APPLE_NOTARY_ISSUER_ID': '11111111-2222-4333-8444-555555555555',
                'APPLE_NOTARY_KEY_ID': 'ABCDEFGHIJ',
                'APPLE_NOTARY_PRIVATE_KEY': '-----BEGIN PRIVATE KEY-----\nZmFrZS1rZXk=\n-----END PRIVATE KEY-----\n'}

    def test_prepare_requires_all_credentials_and_keeps_key_outside_repository_with_private_modes(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)/'repo';env = self.fixture(root);directory = Path(folder)/'notary'
            for name in env:
                missing = {**env};missing.pop(name)
                with self.subTest(name=name),self.assertRaises(ValueError):m.prepare(root, directory, missing)
                self.assertFalse(directory.exists())
            m.prepare(root, directory, env)
            self.assertEqual((directory/'AuthKey.p8').read_text(), env['APPLE_NOTARY_PRIVATE_KEY'])
            self.assertEqual((directory/'AuthKey.p8').stat().st_mode & 0o777, 0o600)
            self.assertEqual(directory.stat().st_mode & 0o777, 0o700)
            config = json.loads((directory/'tauri-release.json').read_bytes())['bundle']['macOS']
            self.assertEqual(config, {'signingIdentity': IDENTITY, 'hardenedRuntime': True})
            with self.assertRaises(ValueError):m.prepare(root, directory, env)
            m.cleanup(directory);self.assertFalse(directory.exists())

    def test_prepare_override_has_no_unknown_pinned_cli_macos_fields(self):
        schema_path = SCRIPT.parents[1]/'node_modules/@tauri-apps/cli/config.schema.json'
        schema = json.loads(schema_path.read_bytes())['definitions']['MacConfig']
        self.assertFalse(schema['additionalProperties'])
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)/'repo';env = self.fixture(root);directory = Path(folder)/'notary'
            m.prepare(root, directory, env)
            config = json.loads((directory/'tauri-release.json').read_bytes())['bundle']['macOS']
            self.assertFalse(set(config) - set(schema['properties']), 'Unknown Tauri macOS config fields')
            m.cleanup(directory)

    def test_cleanup_preserves_changed_or_unknown_material(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)/'repo';env = self.fixture(root);directory = Path(folder)/'notary'
            m.prepare(root, directory, env);key = directory/'AuthKey.p8';original = key.read_bytes()
            key.write_bytes(b'outside change')
            with self.assertRaises(ValueError):m.cleanup(directory)
            self.assertEqual(key.read_bytes(), b'outside change')
            key.write_bytes(original);(directory/'outside').write_bytes(b'keep')
            with self.assertRaises(ValueError):m.cleanup(directory)
            self.assertTrue((directory/'outside').exists())

    def test_failed_partial_key_write_removes_only_the_new_private_staging(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)/'repo';env = self.fixture(root);directory = Path(folder)/'notary'
            original_open = Path.open
            class Partial:
                def __init__(self, target):self.target = target
                def __enter__(self):return self
                def __exit__(self, *_args):self.target.close()
                def write(self, raw):
                    self.target.write(raw[:10]);self.target.flush();raise OSError('fixture disk failure')
            def opened(path, mode='r', *args, **kwargs):
                result = original_open(path, mode, *args, **kwargs)
                return Partial(result) if path.name == 'AuthKey.p8' and mode == 'xb' else result
            with patch.object(Path, 'open', opened),self.assertRaises(OSError):m.prepare(root, directory, env)
            self.assertFalse(directory.exists(), 'Partial private key remained after failed preparation')

    def test_command_keeps_notary_json_separate_from_private_stderr(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder)/'tool.py'
            path.write_text('import sys\nprint(\'{"status":"Accepted"}\')\nprint("fixture diagnostic",file=sys.stderr)\n')
            self.assertEqual(json.loads(m.command([sys.executable, path])), {'status': 'Accepted'})

    def test_signature_metadata_rejects_adhoc_wrong_team_and_missing_runtime_or_timestamp(self):
        m = self.module();policy = {'schema': 1, 'team_id': '6DF93UDRBJ', 'identity': IDENTITY}
        text = f'Identifier=org.opentag.editor\nAuthority={IDENTITY}\nTeamIdentifier=6DF93UDRBJ\nTimestamp=Oct 20, 2026\nCodeDirectory v=20500 flags=0x10000(runtime) hashes=3+7\nCDHash={"a"*40}\n'
        m.signature_metadata(text, policy, 'org.opentag.editor', True)
        for changed in [text.replace(IDENTITY, '-'), text.replace('TeamIdentifier=6DF93UDRBJ', 'TeamIdentifier=OTHERTEAM1'),
                        text.replace('0x10000(runtime)', '0x0(none)'), text.replace('Timestamp=Oct 20, 2026\n', '')]:
            with self.subTest(changed=changed),self.assertRaises(ValueError):
                m.signature_metadata(changed, policy, 'org.opentag.editor', True)

    def build_fixture(self, root, target):
        bundle = root/'target'/target/'release/bundle';app = bundle/'macos/TagGroove.app'
        (app/'Contents').mkdir(parents=True)
        (app/'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier': 'org.opentag.editor',
            'CFBundleShortVersionString': '0.1.0', 'CFBundleVersion': '0.1.0'}))
        dmg = bundle/'dmg/TagGroove.dmg';dmg.parent.mkdir();dmg.write_bytes(b'fixture signed disk image')
        return app, dmg

    def test_notarization_must_be_accepted_and_staples_verified_before_archive_or_receipt(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)/'repo';env = self.fixture(root);directory = Path(folder)/'notary'
            m.prepare(root, directory, env);target = 'aarch64-apple-darwin';app, dmg = self.build_fixture(root, target)
            calls = [];status = ['Accepted'];wrong_inner = [True]
            def run(args, **_kwargs):
                args = list(map(str, args));calls.append(args)
                if args[:2] == ['codesign', '--display']:
                    cdhash = 'b'*40 if wrong_inner[0] and args[-1] != str(app) else 'a'*40
                    return f'Identifier=org.opentag.editor\nAuthority={IDENTITY}\nTeamIdentifier=6DF93UDRBJ\nTimestamp=Oct 20, 2026\nCodeDirectory v=20500 flags=0x10000(runtime)\nCDHash={cdhash}\n'
                if args[:2] == ['hdiutil', 'attach']:
                    mount = Path(args[-1]);inside = mount/'TagGroove.app/Contents';inside.mkdir(parents=True)
                    (inside/'Info.plist').write_bytes((app/'Contents/Info.plist').read_bytes())
                    return plistlib.dumps({'system-entities': [{'mount-point': str(mount)}]}).decode()
                if args[:3] == ['xcrun', 'notarytool', 'submit']:
                    return json.dumps({'status': status[0], 'id': '11111111-2222-4333-8444-555555555555'})
                if args[0] == 'tar':Path(args[2]).write_bytes(b'archive from verified app')
                return ''
            with self.assertRaisesRegex(ValueError, 'contained application'):
                m.verify_and_package(root, target, directory, 'a'*40, '0.1.0', env, run=run)
            self.assertTrue(any(call[:2] == ['hdiutil', 'detach'] for call in calls))
            self.assertFalse(any(call[:3] == ['xcrun', 'notarytool', 'submit'] or call[0] == 'tar' for call in calls))
            wrong_inner[0] = False;status[0] = 'Invalid';calls.clear()
            with self.assertRaises(ValueError):m.verify_and_package(root, target, directory, 'a'*40, '0.1.0', env, run=run)
            self.assertFalse(any(call[0] == 'tar' for call in calls))
            status[0] = 'Accepted';calls.clear()
            result = m.verify_and_package(root, target, directory, 'a'*40, '0.1.0', env, run=run)
            self.assertTrue(Path(result['receipt_path']).is_file())
            receipt = json.loads(Path(result['receipt_path']).read_bytes())
            self.assertEqual(receipt['dmg_notarization']['status'], 'Accepted')
            self.assertEqual(set(receipt['artifacts']), {'.dmg', '.app.tar.gz'})
            self.assertTrue(callable(getattr(m, 'validate_receipt', None)), 'Missing offline signing receipt validator')
            m.validate_receipt(receipt, (root/m.POLICY).read_bytes(), 'a'*40, '0.1.0', 'macos-arm64', 'org.opentag.editor')
            for field, value in [('team_id', 'OTHERTEAM1'), ('source_commit', 'b'*40), ('version', '0.2.0'),
                                 ('target', 'x86_64-apple-darwin'), ('policy_sha256', '0'*64)]:
                with self.subTest(field=field),self.assertRaises(ValueError):
                    m.validate_receipt({**receipt, field: value}, (root/m.POLICY).read_bytes(),
                                       'a'*40, '0.1.0', 'macos-arm64', 'org.opentag.editor')
            tar = next(i for i, call in enumerate(calls) if call[0] == 'tar')
            self.assertLess(next(i for i, call in enumerate(calls) if call[:3] == ['xcrun', 'stapler', 'staple']), tar)
            self.assertTrue(any(call[:3] == ['xcrun', 'stapler', 'validate'] and call[-1] == str(app) for call in calls[:tar]))
            self.assertTrue(any(call[:3] == ['xcrun', 'stapler', 'validate'] and call[-1] == str(dmg) for call in calls[:tar]))


if __name__ == '__main__':unittest.main()
