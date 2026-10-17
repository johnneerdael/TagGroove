"""Exercise the paired consumer with explicitly synthetic captures, never Mp3tag proof."""
import copy
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("save_acceptance", Path(__file__).parents[1] / "save-acceptance.py")
harness = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(harness)


class CarrierIdentity(unittest.TestCase):
    def test_retained_audio_matches_the_frozen_manifest(self):
        directory = harness.REPO / "crates/tag-core/tests/fixtures/save-corpus"
        manifest = json.loads((directory / "manifest.json").read_text())
        self.assertEqual(set(manifest["sha256"]), {"tone.mp3", "tone.flac", "tone.m4a"})
        for name, digest in manifest["sha256"].items():
            self.assertEqual(harness.sha256((directory / name).read_bytes()), digest, name)


class SaveAcceptance(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.executable = harness.backend()

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "paired"
        harness.prepare(self.root, self.executable)
        self.assertTrue(harness.run(self.root, self.executable)["passed"])
        self.contract = json.loads((self.root / "contract.json").read_text())
        self.identity_path = self.root / "synthetic-consumer-control.json"
        harness.write_json(self.identity_path, {
            "application": "Mp3tag", "version": "3.36.1", "executable_sha256": harness.REFERENCE_SHA256,
            "platform": "SYNTHETIC CONTROL; original was not executed", "runtime": "TagGroove copies used to test consumer only",
            "recorded_at_utc": "2026-10-18T00:00:00Z", "defaults": {"synthetic": True},
            "evidence": ["test-only input/output copies; no reference evidence"],
            "input_sha256": {name: data["input_sha256"] for name, data in self.contract["baseline"].items()},
        })

    def synthetic_reference(self):
        for operation in harness.OPERATIONS:
            for stage in ("saved", "undone", "redone"):
                shutil.copytree(self.root / "taggroove" / operation / stage, self.root / "reference" / operation / stage)

    def test_consumer_rejects_missing_capture_and_mismatched_reference(self):
        with self.assertRaisesRegex(ValueError, "missing original-runtime capture"):
            harness.compare(self.root, self.executable, self.identity_path)
        self.assertFalse((self.root / "paired-results.json").exists())
        identity = json.loads(self.identity_path.read_text())
        identity["executable_sha256"] = "0" * 64
        harness.write_json(self.identity_path, identity)
        with self.assertRaisesRegex(ValueError, "pinned original"):
            harness.compare(self.root, self.executable, self.identity_path)
        contract = copy.deepcopy(self.contract)
        contract["baseline"] = {}
        with self.assertRaisesRegex(ValueError, "nonempty captured input"):
            harness.validate_inputs(self.root, self.executable, contract)

    def test_consumer_reads_real_retained_files_and_detects_audio_damage(self):
        self.synthetic_reference()
        self.assertTrue(harness.compare(self.root, self.executable, self.identity_path)["passed"])
        path = self.root / "reference" / "edit-one" / "saved" / "café 日本.flac"
        data = bytearray(path.read_bytes())
        data[-1] ^= 1
        path.write_bytes(data)
        self.assertFalse(harness.compare(self.root, self.executable, self.identity_path)["passed"])
        receipt = json.loads((self.root / "paired-results.json").read_text())
        self.assertIn("/audio_payload_sha256", receipt["cases"][0]["differences_from_taggroove"])

    def test_baseline_and_local_receipt_cannot_mask_changed_files(self):
        self.synthetic_reference()
        contract = copy.deepcopy(self.contract)
        name = next(iter(contract["baseline"]))
        contract["baseline"][name]["snapshot"]["fields"]["TITLE"] = ["fabricated"]
        harness.write_json(self.root / "contract.json", contract)
        with self.assertRaisesRegex(ValueError, "baseline does not match"):
            harness.compare(self.root, self.executable, self.identity_path)
        harness.write_json(self.root / "contract.json", self.contract)
        path = self.root / "taggroove" / "edit-one" / "saved" / "café 日本.flac"
        data = bytearray(path.read_bytes())
        data[-1] ^= 1
        path.write_bytes(data)
        with self.assertRaisesRegex(ValueError, "local output failed preservation"):
            harness.compare(self.root, self.executable, self.identity_path)

    def test_captured_inputs_are_copied_and_originals_remain_unchanged(self):
        source = self.root / "inputs"
        hashes = {p.name: harness.sha256(p.read_bytes()) for p in source.iterdir()}
        imported = Path(self.temporary.name) / "captured"
        harness.prepare(imported, self.executable, source)
        self.assertEqual(hashes, {p.name: harness.sha256(p.read_bytes()) for p in source.iterdir()})
        self.assertTrue(harness.run(imported, self.executable)["passed"])

    def test_comparator_detects_each_preservation_dimension(self):
        baseline = next(iter(self.contract["baseline"].values()))["snapshot"]
        for key in ("fields", "covers", "opaque_native", "opaque_marker_count", "audio_payload_sha256", "tag_kinds"):
            damaged = copy.deepcopy(baseline)
            damaged[key] = None
            self.assertTrue(harness.differences(baseline, damaged), key)


if __name__ == "__main__":
    unittest.main()
