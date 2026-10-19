import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import textwrap
import unittest


ROOT = Path(__file__).resolve().parents[3]
WORKFLOWS = {'_reviewed-native.yml': 1}


@unittest.skipUnless(shutil.which('pwsh'), 'PowerShell is needed to execute the fallback')
class FfmpegFallbackTests(unittest.TestCase):
    def blocks(self):
        blocks = []
        for workflow, count in WORKFLOWS.items():
            source = (ROOT / '.github/workflows' / workflow).read_text()
            matches = re.findall(
                r'(?ms)^ {8}# Chocolatey\'s ffmpeg package[^\n]*\n {8}run: \|\n(.*?)(?=^ {6}- )',
                source,
            )
            self.assertEqual(len(matches), count, workflow)
            blocks.extend((f'{workflow}:{index}', textwrap.dedent(block))
                          for index, block in enumerate(matches))
        return blocks

    def run_block(self, block, payload, directory, mode='fallback'):
        # Execute each real Windows install step, including the Actions wrapper's
        # final exit. Stub network/package calls; keep real hashing and NASM checks.
        digest = hashlib.sha256(b'fixture archive').hexdigest()
        block = re.sub(r"\$expectedSha256 = '[0-9a-f]{64}'", f"$expectedSha256 = '{digest}'", block)
        harness = r'''
$ErrorActionPreference = 'Stop'
$global:attempts = 0
function choco {
  $global:attempts++
  [IO.File]::WriteAllText((Join-Path $env:RUNNER_TEMP 'attempts'), "$global:attempts")
  if ($env:OPENTAG_FFMPEG_TEST_MODE -ne 'missing-nasm') {
    $nasm = Join-Path $env:ProgramFiles 'NASM'
    New-Item -ItemType Directory -Force $nasm | Out-Null
    [IO.File]::WriteAllText((Join-Path $nasm 'nasm.exe'), 'fixture')
  }
  if ($env:OPENTAG_FFMPEG_TEST_MODE -eq 'choco-success' -or
      ($env:OPENTAG_FFMPEG_TEST_MODE -eq 'retry-success' -and $global:attempts -eq 2)) {
    [IO.File]::WriteAllText((Join-Path $env:RUNNER_TEMP 'installed'), 'yes')
    $global:LASTEXITCODE = 0
  } else {
    $global:LASTEXITCODE = 1
  }
}
function Start-Sleep { param($Seconds) }
function Get-Command {
  [CmdletBinding()] param($Name)
  if (Test-Path (Join-Path $env:RUNNER_TEMP 'installed')) {
    [PSCustomObject]@{Name = 'ffmpeg'}
  }
}
function Invoke-WebRequest {
  param($Uri, $OutFile, $MaximumRetryCount, $RetryIntervalSec)
  [IO.File]::WriteAllBytes($OutFile, [Text.Encoding]::UTF8.GetBytes($env:OPENTAG_FFMPEG_TEST_PAYLOAD))
}
function Expand-Archive {
  param($Path, $DestinationPath)
  [IO.File]::WriteAllText((Join-Path $env:RUNNER_TEMP 'expanded'), 'yes')
  $bin = Join-Path $DestinationPath 'bin'
  New-Item -ItemType Directory -Force $bin | Out-Null
  if ($env:OPENTAG_FFMPEG_TEST_MODE -ne 'missing-ffmpeg') {
    [IO.File]::WriteAllText((Join-Path $bin 'ffmpeg.exe'), 'fixture')
  }
}
'''
        env = dict(os.environ, RUNNER_TEMP=str(directory), GITHUB_PATH=str(directory / 'github-path'),
                   ProgramFiles=str(directory / 'program-files'),
                   OPENTAG_FFMPEG_TEST_PAYLOAD=payload, OPENTAG_FFMPEG_TEST_MODE=mode)
        return subprocess.run([shutil.which('pwsh'), '-NoProfile', '-NonInteractive', '-Command',
                               harness + block + '\nexit $LASTEXITCODE'],
                              env=env, text=True, capture_output=True, check=False)

    def test_all_windows_jobs_reject_tampered_archives_before_extraction(self):
        for job, block in self.blocks():
            with self.subTest(job=job), tempfile.TemporaryDirectory() as directory:
                directory = Path(directory)
                result = self.run_block(block, 'tampered archive', directory)
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertIn('FFmpeg archive SHA-256 mismatch', result.stderr)
                self.assertFalse((directory / 'expanded').exists())
                self.assertFalse((directory / 'github-path').exists())

    def test_all_windows_jobs_expand_only_the_matching_pinned_archive(self):
        for job, block in self.blocks():
            with self.subTest(job=job), tempfile.TemporaryDirectory() as directory:
                self.assertNotIn('/latest/', block)
                self.assertRegex(block, r"\$expectedSha256 = '[0-9a-f]{64}'")
                directory = Path(directory)
                result = self.run_block(block, 'fixture archive', directory)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertTrue((directory / 'expanded').exists())
                self.assertIn(str(directory / 'ffmpeg/bin'), (directory / 'github-path').read_text())

    def test_chocolatey_success_skips_download_and_retries_stop_when_tools_exist(self):
        for job, block in self.blocks():
            for mode, attempts in [('choco-success', 1), ('retry-success', 2)]:
                with self.subTest(job=job, mode=mode), tempfile.TemporaryDirectory() as directory:
                    directory = Path(directory)
                    result = self.run_block(block, 'fixture archive', directory, mode)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual((directory / 'attempts').read_text(), str(attempts))
                    self.assertFalse((directory / 'ffmpeg.zip').exists())
                    self.assertIn(str(directory / 'program-files/NASM'),
                                  (directory / 'github-path').read_text())

    def test_fallback_does_not_hide_missing_nasm(self):
        for job, block in self.blocks():
            with self.subTest(job=job), tempfile.TemporaryDirectory() as directory:
                directory = Path(directory)
                result = self.run_block(block, 'fixture archive', directory, 'missing-nasm')
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertIn('NASM installation missing (Chocolatey exit code 1)', result.stderr)
                self.assertEqual((directory / 'attempts').read_text(), '3')

    def test_verified_archive_must_include_ffmpeg_before_exporting_path(self):
        for job, block in self.blocks():
            with self.subTest(job=job), tempfile.TemporaryDirectory() as directory:
                directory = Path(directory)
                result = self.run_block(block, 'fixture archive', directory, 'missing-ffmpeg')
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertIn('FFmpeg executable missing from verified archive', result.stderr)
                self.assertFalse((directory / 'github-path').exists())

    def test_release_nasm_step_accepts_presence_and_refuses_missing_assembler(self):
        source = (ROOT / '.github/workflows/reviewed-release.yml').read_text()
        step = source.split('      - name: Windows native assembler\n', 1)[1].split('      - run: npm ci\n', 1)[0]
        block = textwrap.dedent(step.split('        run: |\n', 1)[1])
        self.assertNotIn('ffmpeg', block.lower())
        self.assertNotIn('Invoke-WebRequest', block)
        for mode, expected_attempts, succeeds in [('choco-success', 1, True), ('fallback', 1, True), ('missing-nasm', 3, False)]:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                directory = Path(directory)
                result = self.run_block(block, 'unused', directory, mode)
                self.assertEqual(result.returncode == 0, succeeds, result.stderr)
                self.assertEqual((directory / 'attempts').read_text(), str(expected_attempts))
                if succeeds:
                    self.assertIn(str(directory / 'program-files/NASM'), (directory / 'github-path').read_text())
                else:
                    self.assertIn('NASM installation missing', result.stderr)
                    self.assertFalse((directory / 'github-path').exists())
