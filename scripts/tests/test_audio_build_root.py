"""Execute the build script's actual root selection without compiling native audio."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]


class NativeBuildRootTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.directory.cleanup)
        folder = Path(cls.directory.name)
        source = (ROOT / 'crates/audio-fingerprint/build.rs').read_text()
        # Compile the production selection block, so precedence and validation
        # changes are exercised without fetching or building native dependencies.
        selection = source.split('    let root =', 1)[1].split(
            '    println!("cargo:rerun-if-changed=', 1)[0]
        probe = folder / 'root.rs'
        probe.write_text('use std::{env, path::PathBuf};\nfn main() { let root =' + selection
                         + '\nprintln!("{}", root.display()); }\n')
        cls.probe = folder / ('root.exe' if os.name == 'nt' else 'root')
        subprocess.run(['rustc', str(probe), '-o', str(cls.probe)], check=True,
                       capture_output=True, text=True)

    def select(self, original=None, alias=None):
        env = dict(os.environ)
        for key in ['OPENTAG_AUDIO_BUILD_ROOT', 'TAURI_OPENTAG_AUDIO_BUILD_ROOT']:
            env.pop(key, None)
        env['OUT_DIR'] = str(Path(self.directory.name) / 'cargo-out')
        if original is not None:
            env['OPENTAG_AUDIO_BUILD_ROOT'] = original
        if alias is not None:
            env['TAURI_OPENTAG_AUDIO_BUILD_ROOT'] = alias
        return subprocess.run([str(self.probe)], env=env, capture_output=True, text=True)

    def test_alias_alone_selects_exact_mobile_evidence_root(self):
        root = str(Path(self.directory.name) / 'mobile-native')
        result = self.select(alias=root)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), root)

    def test_original_override_takes_precedence_even_with_relative_alias(self):
        root = str(Path(self.directory.name) / 'original-native')
        result = self.select(original=root, alias='relative-alias')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), root)

    def test_relative_selected_override_is_rejected(self):
        for values in [{'alias': 'relative-native'},
                       {'original': 'relative-native', 'alias': self.directory.name}]:
            with self.subTest(values=values):
                result = self.select(**values)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('must be absolute', result.stderr)

    def test_omitting_both_overrides_keeps_cargo_owned_output(self):
        result = self.select()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), str(Path(self.directory.name) / 'cargo-out/audio-native'))
