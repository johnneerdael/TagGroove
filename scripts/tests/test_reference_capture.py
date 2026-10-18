"""Synthetic integrity and pure filesystem probe tests; no reference/native APIs."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location("reference_collector", Path(__file__).parents[1] / "reference-capture/collector.py")
collector = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(collector)


class ReferenceCapture(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def artifact(self, name, data):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        return {"path": name, "sha256": collector.digest(path), "bytes": len(data)}

    def capture(self):
        # This manifest exercises the validator, not the truth of runtime claims.
        input_hash = hashlib.sha256(b"independent test input").hexdigest()
        self.contract = self.root / "contract.json"
        collector.write_json(self.contract, {"schema": 1, "baseline": {"café.flac": {"input_sha256": input_hash}}})
        cfg = self.root / "runtime/application/mp3tag.cfg"
        cfg.parent.mkdir(parents=True)
        cfg.write_bytes(b"synthetic configuration control")
        artifacts = [self.artifact("evidence/control.json", b'{"observation_type":"synthetic unit control"}')]
        cases = []
        for operation in collector.OPERATIONS:
            stages = {}
            for stage in collector.STAGES:
                relative = f"evidence/{operation}/{stage}/café.flac"
                artifacts.append(self.artifact(relative, b"fLaCsynthetic header control"))
                stages[stage] = relative
            cases.append({"operation": operation, "file": "café.flac", "input_sha256": input_hash, "stages": stages,
                "operator_steps": [{"action": action, "observed_at_utc": "2026-10-18T00:00:00Z", "evidence": "evidence/control.json"}
                                   for action in ("edit", "save", "undo", "redo", "reload")]})
        record = {"schema": collector.SCHEMA, "observation_type": "runtime_save_capture", "status": "complete",
            "recorded_at_utc": "2026-10-18T00:00:00Z", "contract_sha256": collector.digest(self.contract),
            "reference": {"application": "Mp3tag", "version": "3.36.1", "installer_url": collector.INSTALLER_URL,
                "installer_sha256": collector.INSTALLER_SHA256, "executable_sha256": collector.EXECUTABLE_SHA256},
            "runtime": {"os": "Windows", "os_version": "SYNTHETIC UNIT CONTROL; Windows was not run", "session_id": 1,
                "process_id": 123, "process_creation_time_utc": "2026-10-18T00:00:00Z", "collector_source_sha256": "a" * 64},
            "configuration": {"mode": "portable", "observed_path": "runtime/application/mp3tag.cfg", "sha256": collector.digest(cfg), "evidence": "evidence/control.json", "observed_at_utc": "2026-10-18T00:00:00Z"},
            "defaults": {**{key: False for key in collector.DEFAULT_SWITCHES}, "id3v2_format": "v23_utf16", "evidence": "evidence/control.json", "observed_at_utc": "2026-10-18T00:00:00Z"},
            "artifacts": artifacts, "cases": cases}
        self.record = record
        self.save()

    def save(self):
        collector.write_json(self.root / "capture.json", self.record)

    def test_actual_pins_and_generic_artifact_verification(self):
        self.assertEqual(collector.INSTALLER_SHA256, "a977861003a87b1f00f2e0c91263c61bf12d457c6d457b367ce5f69de28fbea0")
        self.assertEqual(collector.EXECUTABLE_SHA256, "484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911")
        path = self.root / "owned.data"
        path.write_bytes(b"independent bytes")
        collector.verify_file(path, collector.digest(path), path.stat().st_size)
        with self.assertRaisesRegex(ValueError, "identity mismatch"):
            collector.verify_file(path, "0" * 64, path.stat().st_size)

    def test_wrong_installer_never_reaches_extraction(self):
        path = self.root / "fake.exe"
        path.write_bytes(b"MZ test control")
        with mock.patch.object(collector.subprocess, "run") as execute:
            with self.assertRaisesRegex(ValueError, "identity mismatch"):
                collector.prepare_runtime(self.root / "fresh", path, "7z")
        execute.assert_not_called()

    def test_redirect_to_a_mirror_is_refused(self):
        with self.assertRaisesRegex(ValueError, "unapproved URL"):
            collector.PinnedRedirect().redirect_request(None, None, 302, "", {}, "http://example.invalid/reference.exe")

    def test_wrong_extracted_executable_leaves_no_qualified_runtime(self):
        package = self.root / "synthetic-package"
        package.write_bytes(b"independent archive control")
        def extract(command, **kwargs):
            target = Path(command[3][2:])
            target.mkdir(parents=True)
            (target / "Mp3tag.exe").write_bytes(b"MZ wrong executable control")
        target = self.root / "fresh-extract"
        with mock.patch.object(collector, "INSTALLER_SHA256", collector.digest(package)), mock.patch.object(collector, "INSTALLER_BYTES", package.stat().st_size), mock.patch.object(collector.subprocess, "run", side_effect=extract):
            with self.assertRaisesRegex(ValueError, "identity mismatch"):
                collector.prepare_runtime(target, package, "synthetic-extractor")
        self.assertFalse((target / "runtime.json").exists())
        self.assertFalse((target / "runtime/application/mp3tag.cfg").exists())

    def test_paths_and_runtime_reuse_fail_closed(self):
        for path in ("../capture.json", "evidence/../../runtime/Mp3tag.exe", "C:/runtime/Mp3tag.exe", "evidence\\file.json", "evidence/stream:payload.json", "evidence/trailing ./file.json"):
            with self.subTest(path=path), self.assertRaises(ValueError):
                collector.owned_path(self.root, path)
        with self.assertRaises(FileExistsError):
            collector.new_temporary_workspace(self.root)
        with mock.patch.object(collector.tempfile, "gettempdir", return_value=str(self.root / "other")), mock.patch.dict(collector.os.environ, {}, clear=True):
            with self.assertRaisesRegex(ValueError, "temporary directory"):
                collector.new_temporary_workspace(self.root / "outside")

    def test_static_probe_and_synthetic_records_never_qualify(self):
        self.capture()
        for observation in ("static_runtime_preparation", "runtime_probe", "synthetic_test_control"):
            self.record["observation_type"] = observation
            self.save()
            with self.assertRaisesRegex(ValueError, "cannot qualify"):
                collector.validate_capture(self.root, self.contract)

    def test_valid_manifest_exports_only_json_identity_without_runtime_bodies(self):
        self.capture()
        collector.validate_capture(self.root, self.contract)
        destination = self.root / "identity.json"
        collector.export_identity(self.root, self.contract, destination)
        identity = json.loads(destination.read_text())
        self.assertEqual(identity["executable_sha256"], collector.EXECUTABLE_SHA256)
        self.assertEqual(identity["input_sha256"], {"café.flac": self.record["cases"][0]["input_sha256"]})
        self.assertNotIn("configuration control", destination.read_text())
        self.assertFalse((self.root / "reference").exists())
        with self.assertRaisesRegex(ValueError, "must not exist"):
            collector.export_identity(self.root, self.contract, destination)

    def test_mutated_artifacts_contract_and_configuration_are_rejected(self):
        self.capture()
        for relative in ("evidence/control.json", "evidence/edit-one/saved/café.flac", "runtime/application/mp3tag.cfg", "contract.json"):
            path = self.root / relative
            original = path.read_bytes()
            path.write_bytes(original + b" ")
            with self.subTest(path=relative), self.assertRaises(ValueError):
                collector.validate_capture(self.root, self.contract)
            path.write_bytes(original)

    def test_missing_duplicate_misidentified_and_unobserved_cases_are_rejected(self):
        self.capture()
        mutations = [lambda r: r["cases"].pop(), lambda r: r["cases"].append(copy.deepcopy(r["cases"][0])),
            lambda r: r["cases"][0]["stages"].pop("undone"), lambda r: r["cases"][0].update(input_sha256="0" * 64),
            lambda r: r["cases"][0].update(operator_steps=[]), lambda r: r["defaults"].pop("write_id3v2"),
            lambda r: r["configuration"].update(mode="unverified"), lambda r: r["reference"].update(executable_sha256="0" * 64),
            lambda r: r["cases"][0]["operator_steps"][1].update(observed_at_utc="unknown"),
            lambda r: r["defaults"].update(observed_at_utc="2027-01-01T00:00:00Z")]
        original = copy.deepcopy(self.record)
        for mutate in mutations:
            self.record = copy.deepcopy(original)
            mutate(self.record)
            self.save()
            with self.assertRaises(ValueError):
                collector.validate_capture(self.root, self.contract)

    def test_vendor_files_and_renamed_executables_cannot_enter_artifact_set(self):
        self.capture()
        for relative in ("runtime/application/Mp3tag.exe", "evidence/Mp3tag.exe", "evidence/renamed.mp3"):
            original = list(self.record["artifacts"])
            self.record["artifacts"].append(self.artifact(relative, b"MZ synthetic executable control"))
            self.save()
            with self.subTest(path=relative), self.assertRaises(ValueError):
                collector.validate_capture(self.root, self.contract)
            self.record["artifacts"] = original

    def test_symlinked_artifacts_cannot_escape(self):
        self.capture()
        outside = self.root / "outside.json"
        outside.write_text("{}")
        link = self.root / "evidence/link.json"
        link.symlink_to(outside)
        self.record["artifacts"].append({"path": "evidence/link.json", "sha256": collector.digest(outside), "bytes": 2})
        self.save()
        with self.assertRaisesRegex(ValueError, "escapes"):
            collector.validate_capture(self.root, self.contract)


@unittest.skipUnless(shutil.which("pwsh") or shutil.which("powershell"), "PowerShell is unavailable")
class ProbeFinalization(unittest.TestCase):
    def finalize(self, candidate, status="probe_only"):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cfg = root / "mp3tag.cfg"
            cfg.write_bytes(candidate if isinstance(candidate, bytes) else b"")
            if candidate == "deleted":
                cfg.unlink()
            elif candidate == "renamed":
                cfg.rename(root / "mp3tag.cfg.invalid")
            elif candidate == "directory":
                cfg.unlink()
                cfg.mkdir()
            # Parse the real script, then execute only Write-Record and its final
            # observation block. Startup, native declarations and API calls never
            # execute. The already-exited process is a fixture, not a real process.
            script = r"""
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$tokens = $null
$parseErrors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($env:PROBE_SOURCE, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count) { throw $parseErrors[0] }
$writer = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Write-Record' }, $false)
. ([scriptblock]::Create($writer.Extent.Text))
$final = $ast.Find({ param($node) $node -is [Management.Automation.Language.TryStatementAst] -and $null -ne $node.Finally }, $false).Finally.Extent.Text
$cfg = Join-Path $env:PROBE_CASE_ROOT 'mp3tag.cfg'
$output = Join-Path $env:PROBE_CASE_ROOT 'record.json'
$errors = [Collections.Generic.List[string]]::new()
$errors.Add('Earlier observation failed')
$record = @{
    status = $env:PROBE_CASE_STATUS
    runtime = @{ process_id = 123 }
    native_windows = @(@{ handle = 456 })
    automation_elements = @(@{ automation_id = 'observed-control' })
    configuration = @{ mode = 'unverified'; path_observed = $false; bytes_after_exit = $null; sha256_after_exit = $null }
    shutdown = @{ exit_code = $null }
    errors = $errors
}
$started = [pscustomobject]@{ HasExited = $true; ExitCode = 7 }
function Confirm-OwnedProcess { return $null }
if ($env:PROBE_CASE_ACTION -in @('deleted-during-hash', 'directory-during-hash')) {
    # Mutate the real file after Get-Item; retain the actual hash cmdlet's errors.
    function Get-FileHash {
        param($LiteralPath, $Algorithm)
        Remove-Item -LiteralPath $LiteralPath
        if ($env:PROBE_CASE_ACTION -eq 'directory-during-hash') {
            [void](New-Item -ItemType Directory -Path $LiteralPath)
        }
        Microsoft.PowerShell.Utility\Get-FileHash -LiteralPath $LiteralPath -Algorithm $Algorithm
    }
}
. ([scriptblock]::Create($final.Substring(1, $final.Length - 2)))
"""
            result = subprocess.run(
                [shutil.which("pwsh") or shutil.which("powershell"), "-NoProfile", "-NonInteractive", "-Command", script],
                env={**os.environ, "PROBE_SOURCE": str(Path(__file__).parents[1] / "reference-capture/probe.ps1"),
                     "PROBE_CASE_ROOT": str(root), "PROBE_CASE_STATUS": status,
                     "PROBE_CASE_ACTION": candidate if isinstance(candidate, str) else ""},
                capture_output=True, text=True, timeout=30,
            )
            output = root / "record.json"
            self.assertTrue(output.is_file(), f"probe observations were lost: {result.stderr}")
            self.assertEqual(result.returncode, 0, result.stderr)
            record = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(record["runtime"]["process_id"], 123)
            self.assertEqual(record["native_windows"], [{"handle": 456}])
            self.assertEqual(record["automation_elements"], [{"automation_id": "observed-control"}])
            self.assertEqual(record["shutdown"]["exit_code"], 7)
            self.assertEqual(record["errors"][0], "Earlier observation failed")
            return record

    def test_deleted_or_renamed_candidate_retains_probe_observations(self):
        for candidate in ("deleted", "renamed"):
            with self.subTest(candidate=candidate):
                record = self.finalize(candidate)
                configuration = record["configuration"]
                self.assertIs(configuration.get("candidate_present_after_exit"), False)
                self.assertFalse(configuration["path_observed"])
                self.assertIsNone(configuration["bytes_after_exit"])
                self.assertIsNone(configuration["sha256_after_exit"])
                self.assertEqual(configuration["mode"], "unverified")
                self.assertEqual(record["status"], "probe_only")
                self.assertEqual(len(record["errors"]), 2)

    def test_present_candidates_record_actual_size_hash_and_write_observation(self):
        for data, expected_hash in ((b"", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
                                    (b"abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")):
            with self.subTest(data=data):
                record = self.finalize(data)
                configuration = record["configuration"]
                self.assertIs(configuration.get("candidate_present_after_exit"), True)
                self.assertEqual(configuration["bytes_after_exit"], len(data))
                self.assertEqual(configuration["sha256_after_exit"], expected_hash)
                self.assertEqual(configuration["path_observed"], bool(data))
                self.assertEqual(configuration["mode"], "unverified")
                self.assertEqual(record["errors"], ["Earlier observation failed"])

    def test_configuration_inspection_error_retains_record_and_blocks_probe(self):
        record = self.finalize("directory")
        self.assertEqual(record["status"], "blocked")
        self.assertFalse(record["configuration"]["path_observed"])
        self.assertIsNone(record["configuration"]["sha256_after_exit"])
        self.assertEqual(len(record["errors"]), 2)

    def test_candidate_deleted_during_hash_retains_absence_and_observations(self):
        record = self.finalize("deleted-during-hash")
        configuration = record["configuration"]
        self.assertIs(configuration.get("candidate_present_after_exit"), False)
        self.assertIsNone(configuration["bytes_after_exit"])
        self.assertIsNone(configuration["sha256_after_exit"])
        self.assertFalse(configuration["path_observed"])
        self.assertEqual(record["status"], "probe_only")
        self.assertEqual(len(record["errors"]), 2)

    def test_nonmissing_hash_error_retains_observations_and_blocks_probe(self):
        record = self.finalize("directory-during-hash")
        self.assertEqual(record["status"], "blocked")
        self.assertFalse(record["configuration"]["path_observed"])
        self.assertIsNone(record["configuration"]["bytes_after_exit"])
        self.assertIsNone(record["configuration"]["sha256_after_exit"])
        self.assertEqual(len(record["errors"]), 2)

    def test_absent_candidate_does_not_clear_an_earlier_block(self):
        record = self.finalize("deleted", status="blocked")
        self.assertEqual(record["status"], "blocked")
        self.assertEqual(len(record["errors"]), 2)


if __name__ == "__main__":
    unittest.main()
