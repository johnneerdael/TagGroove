import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[3]
WORKFLOW = ROOT / '.github/workflows/_reviewed-build.yml'


@unittest.skipUnless(shutil.which('pwsh'), 'PowerShell is needed to execute the fallback')
class FfmpegFallbackTests(unittest.TestCase):
    def blocks(self):
        blocks = re.findall(
            r'(?ms)^ {12}# Chocolatey unavailable:.*?^ {12}\$env:PATH[^\n]*',
            WORKFLOW.read_text(),
        )
        self.assertEqual(len(blocks), 2)
        return blocks

    def run_block(self, block, payload, directory):
        # Execute the real workflow block with fixture bytes instead of a network
        # download. Keep the real Get-FileHash and fail-before-extraction behavior.
        digest = hashlib.sha256(b'fixture archive').hexdigest()
        block = re.sub(r"\$expectedSha256 = '[0-9a-f]{64}'", f"$expectedSha256 = '{digest}'", block)
        harness = r'''
$ErrorActionPreference = 'Stop'
function Invoke-WebRequest {
  param($Uri, $OutFile, $MaximumRetryCount, $RetryIntervalSec)
  [IO.File]::WriteAllBytes($OutFile, [Text.Encoding]::UTF8.GetBytes($env:OPENTAG_FFMPEG_TEST_PAYLOAD))
}
function Expand-Archive {
  param($Path, $DestinationPath)
  [IO.File]::WriteAllText((Join-Path $env:RUNNER_TEMP 'expanded'), 'yes')
  $bin = Join-Path $DestinationPath 'bin'
  New-Item -ItemType Directory -Force $bin | Out-Null
  [IO.File]::WriteAllText((Join-Path $bin 'ffmpeg.exe'), 'fixture')
}
'''
        env = dict(os.environ, RUNNER_TEMP=str(directory), GITHUB_PATH=str(directory / 'github-path'),
                   OPENTAG_FFMPEG_TEST_PAYLOAD=payload)
        return subprocess.run(['pwsh', '-NoProfile', '-NonInteractive', '-Command', harness + block],
                              env=env, text=True, capture_output=True, check=False)

    def test_both_jobs_reject_tampered_archives_before_extraction(self):
        for index, block in enumerate(self.blocks()):
            with self.subTest(job=index), tempfile.TemporaryDirectory() as directory:
                directory = Path(directory)
                result = self.run_block(block, 'tampered archive', directory)
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertIn('FFmpeg archive SHA-256 mismatch', result.stderr)
                self.assertFalse((directory / 'expanded').exists())
                self.assertFalse((directory / 'github-path').exists())

    def test_both_jobs_expand_only_the_matching_pinned_archive(self):
        for index, block in enumerate(self.blocks()):
            with self.subTest(job=index), tempfile.TemporaryDirectory() as directory:
                self.assertNotIn('/latest/', block)
                self.assertRegex(block, r"\$expectedSha256 = '[0-9a-f]{64}'")
                directory = Path(directory)
                result = self.run_block(block, 'fixture archive', directory)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertTrue((directory / 'expanded').exists())
                self.assertIn(str(directory / 'ffmpeg/bin'), (directory / 'github-path').read_text())
