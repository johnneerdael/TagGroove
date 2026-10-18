"""Exercise the paired consumer with explicitly synthetic captures, never Mp3tag proof."""
import copy
import base64
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location("save_acceptance", Path(__file__).parents[1] / "save-acceptance.py")
harness = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(harness)
COLLECTOR_SPEC = importlib.util.spec_from_file_location(
    "reference_collector", Path(__file__).parents[1] / "reference-capture/collector.py")
collector = importlib.util.module_from_spec(COLLECTOR_SPEC)
COLLECTOR_SPEC.loader.exec_module(collector)


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
            "stage_sha256": {operation: {stage: {
                name: harness.sha256((self.root / "taggroove" / operation / stage / name).read_bytes())
                for name in self.contract["baseline"]} for stage in ("saved", "undone", "redone")}
                for operation in harness.OPERATIONS},
        })

    def synthetic_reference(self):
        for operation in harness.OPERATIONS:
            for stage in ("saved", "undone", "redone"):
                shutil.copytree(self.root / "taggroove" / operation / stage, self.root / "reference" / operation / stage)

    def change_unprojected_bytes(self, path):
        before = harness.normalize(harness.invoke(self.executable, "observe", path))
        data = bytearray(path.read_bytes())
        if path.suffix == ".flac":
            at = 4
            while True:
                kind = data[at] & 0x7f
                length = int.from_bytes(data[at + 1:at + 4], "big")
                if kind == 1 and length:
                    data[at + 4] ^= 1
                    break
                self.assertFalse(data[at] & 0x80, "fixture must retain a padding block")
                at += 4 + length
        else:
            self.assertEqual(path.suffix, ".mp3")
            self.assertEqual(data[3], 3)
            frames = []
            at = 10
            while data[at] != 0:
                length = 10 + int.from_bytes(data[at + 4:at + 8], "big")
                frames.append(bytes(data[at:at + length]))
                at += length
            candidates = [i for i, frame in enumerate(frames) if frame[:1] == b"T" and frame[:4] != b"TXXX"]
            first, second = candidates[:2]
            self.assertNotEqual(frames[first][:4], frames[second][:4])
            frames[first], frames[second] = frames[second], frames[first]
            data[10:at] = b"".join(frames)
        path.write_bytes(data)
        self.assertEqual(before, harness.normalize(harness.invoke(self.executable, "observe", path)),
                         "control must change bytes outside the observer's projection")

    def test_collector_exported_identity_binds_the_transferred_consumer_stages(self):
        capture = Path(self.temporary.name) / "capture"
        (capture / "runtime/application").mkdir(parents=True)
        cfg = capture / "runtime/application/mp3tag.cfg"
        cfg.write_bytes(b"SYNTHETIC configuration; no original runtime")
        (capture / "evidence").mkdir()
        note = capture / "evidence/control.json"
        note.write_text('{"scope": "synthetic consumer integration only"}')
        def artifact(path):
            return {"path": path.relative_to(capture).as_posix(),
                    "sha256": collector.digest(path), "bytes": path.stat().st_size}
        artifacts = [artifact(note)]
        cases = []
        at = "2026-10-18T00:00:00Z"
        for operation in harness.OPERATIONS:
            for name, baseline in self.contract["baseline"].items():
                stages = {}
                for stage in ("saved", "undone", "redone"):
                    target = capture / "evidence" / operation / stage / name
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(self.root / "taggroove" / operation / stage / name, target)
                    artifacts.append(artifact(target))
                    stages[stage] = target.relative_to(capture).as_posix()
                cases.append({"operation": operation, "file": name, "input_sha256": baseline["input_sha256"],
                              "stages": stages, "operator_steps": [
                                  {"action": action, "observed_at_utc": at, "evidence": "evidence/control.json"}
                                  for action in ("edit", "save", "undo", "redo", "reload")]})
        reference = json.loads(self.identity_path.read_text())
        collector.write_json(capture / "capture.json", {
            "schema": collector.SCHEMA, "observation_type": "runtime_save_capture", "status": "complete",
            "recorded_at_utc": at, "contract_sha256": collector.digest(self.root / "contract.json"),
            "reference": {**reference, "installer_url": collector.INSTALLER_URL,
                          "installer_sha256": collector.INSTALLER_SHA256},
            "runtime": {"os": "Windows", "os_version": "SYNTHETIC; Windows was not run", "session_id": 1,
                        "process_id": 123, "process_creation_time_utc": at, "collector_source_sha256": "a" * 64},
            "configuration": {"mode": "portable", "observed_path": "runtime/application/mp3tag.cfg",
                              "sha256": collector.digest(cfg), "evidence": "evidence/control.json", "observed_at_utc": at},
            "defaults": {**{key: False for key in collector.DEFAULT_SWITCHES}, "id3v2_format": "v23_utf16",
                         "evidence": "evidence/control.json", "observed_at_utc": at},
            "artifacts": artifacts, "cases": cases,
        })
        exported = capture / "exported-identity.json"
        collector.export_identity(capture, self.root / "contract.json", exported)
        for operation in harness.OPERATIONS:
            shutil.copytree(capture / "evidence" / operation, self.root / "reference" / operation, dirs_exist_ok=True)
        self.assertTrue(harness.compare(self.root, self.executable, exported)["passed"])
        result = json.loads((self.root / "paired-results.json").read_text())
        for case in result["cases"]:
            self.assertEqual(case["reference_file_sha256"],
                             reference["stage_sha256"][case["operation"]][case["stage"]][case["file"]])

    def test_compare_rejects_modified_reference_bytes_with_identical_decoding(self):
        self.synthetic_reference()
        path = self.root / "reference/edit-one/saved/café 日本.flac"
        self.change_unprojected_bytes(path)
        with self.assertRaisesRegex(ValueError, "reference stage.*hash"):
            harness.compare(self.root, self.executable, self.identity_path)
        self.assertFalse((self.root / "paired-results.json").exists())

    def test_compare_requires_exact_valid_reference_stage_bindings(self):
        self.synthetic_reference()
        original = json.loads(self.identity_path.read_text())
        mutations = [lambda r: r.pop("stage_sha256"),
                     lambda r: r.update(stage_sha256=[]),
                     lambda r: r["stage_sha256"].pop("edit-many"),
                     lambda r: r["stage_sha256"].update(extra={}),
                     lambda r: r["stage_sha256"]["edit-one"].update(extra={}),
                     lambda r: r["stage_sha256"]["edit-one"].pop("undone"),
                     lambda r: r["stage_sha256"]["edit-one"].update(saved=[]),
                     lambda r: r["stage_sha256"]["edit-one"]["saved"].pop("café 日本.flac"),
                     lambda r: r["stage_sha256"]["edit-one"]["saved"].update({"extra.flac": "a" * 64}),
                     lambda r: r["stage_sha256"]["edit-one"]["saved"].update({"café 日本.flac": None}),
                     lambda r: r["stage_sha256"]["edit-one"]["saved"].update({"café 日本.flac": "invalid"}),
                     lambda r: r["stage_sha256"]["edit-one"].update(saved=r["stage_sha256"]["edit-one"]["undone"])]
        for mutate in mutations:
            reference = copy.deepcopy(original)
            mutate(reference)
            harness.write_json(self.identity_path, reference)
            with self.subTest(reference=reference["stage_sha256"] if "stage_sha256" in reference else "missing"):
                with self.assertRaisesRegex(ValueError, "reference stage"):
                    harness.compare(self.root, self.executable, self.identity_path)
                self.assertFalse((self.root / "paired-results.json").exists())

    def test_verified_observer_removes_its_private_copy_after_failure(self):
        source = self.root / "inputs/café 日本.flac"
        captured = source.read_bytes()
        observed = []
        def fail_observation(executable, *args):
            snapshot = Path(args[1])
            observed.append(snapshot)
            self.assertNotEqual(snapshot, source)
            self.assertEqual(snapshot.read_bytes(), captured)
            self.assertFalse(snapshot.stat().st_mode & 0o222)
            raise OSError("synthetic observer failure")
        with mock.patch.object(harness, "invoke", fail_observation):
            with self.assertRaisesRegex(OSError, "synthetic observer failure"):
                harness.observe_verified(self.executable, source,
                                         self.contract["baseline"][source.name]["input_sha256"], "retained input")
        self.assertEqual(len(observed), 1)
        self.assertFalse(observed[0].parent.exists())
        self.assertEqual(source.read_bytes(), captured)

    def test_transient_input_substitution_cannot_hide_a_fabricated_baseline(self):
        target = self.root / "inputs/café 日本.flac"
        captured = target.read_bytes()
        changed = self.root / "taggroove/edit-one/saved" / target.name
        contract = copy.deepcopy(self.contract)
        contract["baseline"][target.name]["snapshot"] = harness.normalize(
            harness.invoke(self.executable, "observe", changed))
        with self.assertRaisesRegex(ValueError, "baseline does not match"):
            harness.validate_inputs(self.root, self.executable, contract)
        invoke = harness.invoke
        substitutions = []
        def transient_substitution(executable, *args):
            if args[0] == "observe" and Path(args[1]).name == target.name:
                target.write_bytes(changed.read_bytes())
                substitutions.append(str(args[1]))
                try:
                    return invoke(executable, *args)
                finally:
                    target.write_bytes(captured)
            return invoke(executable, *args)
        with mock.patch.object(harness, "invoke", transient_substitution):
            with self.assertRaisesRegex(ValueError, "baseline does not match"):
                harness.validate_inputs(self.root, self.executable, contract)
        self.assertTrue(substitutions)
        self.assertEqual(target.read_bytes(), captured)

    def test_transient_stage_substitution_cannot_hide_captured_audio_difference(self):
        self.synthetic_reference()
        target = self.root / "reference/edit-one/saved/café 日本.flac"
        local = self.root / "taggroove/edit-one/saved/café 日本.flac"
        data = bytearray(target.read_bytes())
        data[-1] ^= 1
        target.write_bytes(data)
        captured = target.read_bytes()
        reference = json.loads(self.identity_path.read_text())
        reference["stage_sha256"]["edit-one"]["saved"][target.name] = harness.sha256(captured)
        harness.write_json(self.identity_path, reference)
        self.assertFalse(harness.compare(self.root, self.executable, self.identity_path)["passed"],
                         "captured audio must differ before the transient substitution")
        invoke = harness.invoke
        substitutions = []
        def transient_substitution(executable, *args):
            if args[0] == "observe" and Path(args[1]).name == target.name:
                target.write_bytes(local.read_bytes())
                substitutions.append(str(args[1]))
                try:
                    return invoke(executable, *args)
                finally:
                    target.write_bytes(captured)
            return invoke(executable, *args)
        with mock.patch.object(harness, "invoke", transient_substitution):
            result = harness.compare(self.root, self.executable, self.identity_path)
        self.assertTrue(substitutions, "the original stage must be substituted during observation")
        self.assertEqual(target.read_bytes(), captured, "substitution must be restored before the end checks")
        self.assertFalse(result["passed"], "transient substitution hid the captured audio difference")
        receipt = json.loads((self.root / "paired-results.json").read_text())
        case = next(case for case in receipt["cases"] if case["operation"] == "edit-one"
                    and case["stage"] == "saved" and case["file"] == target.name)
        self.assertEqual(case["reference_file_sha256"], harness.sha256(captured))
        self.assertIn("/audio_payload_sha256", case["differences_from_taggroove"])

    def test_compare_rejects_reference_changes_during_observation(self):
        self.synthetic_reference()
        observe_verified = harness.observe_verified
        target = self.root / "reference/edit-one/saved/café 日本.flac"
        mutations = []
        def change_after_reference_observation(executable, path, expected_hash, context):
            result = observe_verified(executable, path, expected_hash, context)
            if path == target:
                self.change_unprojected_bytes(target)
                mutations.append(path)
            return result
        with mock.patch.object(harness, "observe_verified", change_after_reference_observation):
            with self.assertRaisesRegex(ValueError, "reference stage.*hash"):
                harness.compare(self.root, self.executable, self.identity_path)
        self.assertEqual(mutations, [target], "mutate only after the actual reference decode")
        self.assertFalse((self.root / "paired-results.json").exists())

    def test_compare_rejects_changed_backend_before_observing_captures(self):
        self.synthetic_reference()
        changed = Path(self.temporary.name) / "changed-backend"
        data = bytearray(self.executable.read_bytes())
        data[-1] ^= 1
        changed.write_bytes(data)
        with mock.patch.object(harness, "invoke", side_effect=AssertionError("changed backend must never execute")):
            with self.assertRaisesRegex(ValueError, "identity.*backend_sha256"):
                harness.compare(self.root, changed, self.identity_path)

    def test_compare_rejects_an_actual_different_repository_identity(self):
        self.synthetic_reference()
        repository = Path(self.temporary.name) / "different-repository"
        repository.mkdir()
        original = harness.REPO
        for relative in harness.identity(self.executable)["source_files_sha256"]:
            destination = repository / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(original / relative, destination)
        subprocess.run(["git", "init", "-q", str(repository)], check=True)
        subprocess.run(["git", "-C", str(repository), "add", "."], check=True)
        subprocess.run(["git", "-C", str(repository), "-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false",
                        "-c", "user.name=Acceptance control", "-c", "user.email=control@example.invalid",
                        "commit", "-qm", "Independent identity negative control"], check=True)
        with mock.patch.object(harness, "REPO", repository):
            with self.assertRaisesRegex(ValueError, "identity.*source_head"):
                harness.compare(self.root, self.executable, self.identity_path)

    def test_operator_corpus_run_rejects_unprojected_undo_and_redo_changes(self):
        for stage, extension in [("undone", "flac"), ("redone", "mp3")]:
            with self.subTest(stage=stage):
                root = Path(self.temporary.name) / f"operator-{stage}"
                harness.prepare(root, self.executable, self.root / "inputs")
                invoke = harness.invoke
                def corrupt(executable, *args):
                    result = invoke(executable, *args)
                    if args[0] == "exercise" and args[2] == "edit-one" and Path(args[1]).suffix == f".{extension}":
                        path = Path(args[3]) / stage / Path(args[1]).name
                        with mock.patch.object(harness, "invoke", invoke):
                            self.change_unprojected_bytes(path)
                    return result
                with mock.patch.object(harness, "invoke", corrupt):
                    self.assertFalse(harness.run(root, self.executable)["passed"])

    def test_compare_rejects_unprojected_retained_stage_changes(self):
        self.synthetic_reference()
        for stage, extension in [("undone", "flac"), ("redone", "mp3")]:
            with self.subTest(stage=stage):
                path = self.root / "taggroove" / "edit-one" / stage / f"café 日本.{extension}"
                original = path.read_bytes()
                self.change_unprojected_bytes(path)
                with self.assertRaisesRegex(ValueError, "retained local.*bytes"):
                    harness.compare(self.root, self.executable, self.identity_path)
                path.write_bytes(original)
        saved = self.root / "taggroove" / "edit-one" / "saved" / "café 日本.flac"
        redone = self.root / "taggroove" / "edit-one" / "redone" / "café 日本.flac"
        self.change_unprojected_bytes(saved)
        shutil.copyfile(saved, redone)
        with self.assertRaisesRegex(ValueError, "retained local capture bytes changed"):
            harness.compare(self.root, self.executable, self.identity_path)

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
        paired = json.loads((self.root / "paired-results.json").read_text())
        local = json.loads((self.root / "taggroove-results.json").read_text())
        self.assertEqual(paired["producer"], local["producer"])
        self.assertEqual(paired["prepared_by"], self.contract["producer"])
        path = self.root / "reference" / "edit-one" / "saved" / "café 日本.flac"
        data = bytearray(path.read_bytes())
        data[-1] ^= 1
        path.write_bytes(data)
        with self.assertRaisesRegex(ValueError, "reference stage.*hash"):
            harness.compare(self.root, self.executable, self.identity_path)
        # Rebind this explicitly synthetic damaged capture to exercise the
        # structural comparison separately from transfer-integrity rejection.
        reference = json.loads(self.identity_path.read_text())
        reference["stage_sha256"]["edit-one"]["saved"][path.name] = harness.sha256(path.read_bytes())
        harness.write_json(self.identity_path, reference)
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
        with self.assertRaisesRegex(ValueError, "retained local"):
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

    def test_native_id3_fixture_matches_pinned_raw_frame_payloads(self):
        profile = json.loads((harness.REPO / "crates/tag-core/tests/fixtures/save-corpus/native-id3-profile.json").read_text())
        frames = self.contract["baseline"]["native-groups.mp3"]["snapshot"]["native_id3"]
        def key(frame):
            if frame["frame"] == "USLT":
                return "|".join(("USLT", frame["language"], frame["description"]))
            return "|".join((frame["frame"], frame.get("owner", frame.get("description", ""))))
        observed = {key(frame): frame["raw_body_sha256"] for frame in frames}
        self.assertEqual(observed, profile["expected_native_body_sha256"])
        self.assertEqual(len(observed), len(frames), "native groups must not silently collapse")
        for cover in profile["covers"]:
            frame = next(frame for frame in frames if frame["frame"] == "APIC" and frame["description"] == cover["description"])
            self.assertEqual(frame["image_sha256"], cover["image_sha256"])

    def test_paired_consumer_detects_native_counter_loss_hidden_by_star_fields(self):
        self.synthetic_reference()
        path = self.root / "reference" / "edit-one" / "saved" / "native-groups.mp3"
        before = harness.normalize(harness.invoke(self.executable, "observe", path))
        frame = next(frame for frame in before["native_id3"] if frame["frame"] == "POPM" and frame["owner"] == "Windows Media Player 9 Series")
        body = base64.b64decode(frame["raw_body_base64"], validate=True)
        data = bytearray(path.read_bytes())
        self.assertEqual(data.count(body), 1)
        at = data.index(body)
        data[at + len(body) - 1] ^= 1
        path.write_bytes(data)
        after = harness.normalize(harness.invoke(self.executable, "observe", path))
        self.assertEqual(before["fields"], after["fields"])
        self.assertEqual(before["audio_payload_sha256"], after["audio_payload_sha256"])
        self.assertNotEqual(before["native_id3_sha256"], after["native_id3_sha256"])
        # This deliberately synthetic capture must first fail the original
        # transfer binding. Rebind its test-only declared bytes before testing
        # native semantics, without relaxing the production integrity gate.
        with self.assertRaisesRegex(ValueError, "reference stage.*hash"):
            harness.compare(self.root, self.executable, self.identity_path)
        self.assertFalse((self.root / "paired-results.json").exists())
        reference = json.loads(self.identity_path.read_text())
        reference["stage_sha256"]["edit-one"]["saved"][path.name] = harness.sha256(path.read_bytes())
        harness.write_json(self.identity_path, reference)
        self.assertFalse(harness.compare(self.root, self.executable, self.identity_path)["passed"])
        receipt = json.loads((self.root / "paired-results.json").read_text())
        failed = next(case for case in receipt["cases"] if case["file"] == path.name and case["operation"] == "edit-one" and case["stage"] == "saved")
        self.assertIn("/native_id3_sha256", failed["differences_from_taggroove"])
        self.assertEqual(failed["native_id3_difference_kind"], "semantic_observation")

    def test_native_serialization_difference_is_distinct_from_changed_counter_semantics(self):
        self.synthetic_reference()
        path = self.root / "reference" / "edit-one" / "saved" / "native-groups.mp3"
        before = harness.normalize(harness.invoke(self.executable, "observe", path))
        frame = next(frame for frame in before["native_id3"] if frame["frame"] == "POPM" and frame["owner"] == "Windows Media Player 9 Series")
        body = base64.b64decode(frame["raw_body_base64"], validate=True)
        data = bytearray(path.read_bytes())
        at = data.index(body)
        end = 10 + sum(value << (7 * (3 - i)) for i, value in enumerate(data[6:10]))
        self.assertEqual(data[end - 1], 0, "control needs one padding byte")
        data[at - 6:at - 2] = (len(body) + 1).to_bytes(4, "big")
        data.insert(at + body.index(0) + 2, 0)  # Same numeric counter, one leading zero byte.
        del data[end]  # Consume one old padding byte; preserve the media offset.
        path.write_bytes(data)
        after = harness.normalize(harness.invoke(self.executable, "observe", path))
        self.assertEqual(harness.native_id3_semantics(before), harness.native_id3_semantics(after))
        self.assertEqual(before["fields"], after["fields"])
        self.assertEqual(before["audio_payload_sha256"], after["audio_payload_sha256"])
        # This deliberately synthetic capture must first fail the original
        # transfer binding. Rebind its test-only declared bytes before testing
        # native semantics, without relaxing the production integrity gate.
        with self.assertRaisesRegex(ValueError, "reference stage.*hash"):
            harness.compare(self.root, self.executable, self.identity_path)
        self.assertFalse((self.root / "paired-results.json").exists())
        reference = json.loads(self.identity_path.read_text())
        reference["stage_sha256"]["edit-one"]["saved"][path.name] = harness.sha256(path.read_bytes())
        harness.write_json(self.identity_path, reference)
        self.assertFalse(harness.compare(self.root, self.executable, self.identity_path)["passed"])
        receipt = json.loads((self.root / "paired-results.json").read_text())
        changed = next(case for case in receipt["cases"] if case["file"] == path.name and case["operation"] == "edit-one" and case["stage"] == "saved")
        self.assertEqual(changed["native_id3_difference_kind"], "serialization_only")
        self.assertIn("strict observed equality", receipt["comparison_interpretation"])
    def test_policy_verdict_rejects_lost_or_changed_displayed_gain_fields(self):
        invoke = harness.invoke
        for policy in ("default", "explicit_write_ape"):
            for field in ("MP3GAIN_UNDO", "MP3GAIN_MINMAX", "REPLAYGAIN_TRACK_GAIN"):
                for fault in ("missing", "changed"):
                    with self.subTest(policy=policy, field=field, fault=fault):
                        root = Path(self.temporary.name) / f"gain-{policy}-{field}-{fault}"
                        harness.prepare(root, self.executable)

                        def corrupt_report(executable, *args):
                            result = invoke(executable, *args)
                            if args[0] == "policy":
                                result = copy.deepcopy(result)
                                displayed = result[policy]["displayed_after"]
                                if fault == "missing":
                                    displayed.pop(field)
                                else:
                                    displayed[field] = ["changed recovery data"]
                            return result

                        with mock.patch.object(harness, "invoke", corrupt_report):
                            self.assertFalse(harness.run(root, self.executable)["passed"])
                        receipt = json.loads((root / "taggroove-results.json").read_text())
                        self.assertTrue(all(not any(case["errors"].values()) for case in receipt["cases"]),
                                        "ordinary captures must still pass in this policy-only control")
                        self.assertFalse(receipt["mpeg_policy_passed"])
                        self.assertIn("/" + field, receipt["mpeg_policy_gain_errors"][policy])

    def source_repository(self, ignore=None):
        repository = Path(self.temporary.name) / "workspace-repository"
        repository.mkdir()
        for relative in harness.identity(self.executable)["source_files_sha256"]:
            destination = repository / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(harness.REPO / relative, destination)
        if ignore is not None:
            (repository / ".gitignore").write_text(ignore)
        subprocess.run(["git", "init", "-q", str(repository)], check=True)
        subprocess.run(["git", "-C", str(repository), "add", "."], check=True)
        subprocess.run(["git", "-C", str(repository), "-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false",
                        "-c", "user.name=Acceptance control", "-c", "user.email=control@example.invalid",
                        "commit", "-qm", "Workspace placement control"], check=True)
        return repository

    def test_repository_workspace_is_data_but_other_source_changes_still_reject(self):
        repository = self.source_repository()
        workspace = repository / "save-pair"
        with mock.patch.object(harness, "REPO", repository):
            harness.prepare(workspace, self.executable)
            producer = json.loads((workspace / "contract.json").read_text())["producer"]
            self.assertEqual(producer["untracked_capture_workspace"], "save-pair")
            self.assertEqual(producer["source_status"], "")
            self.assertIn("save-pair", subprocess.check_output(
                ["git", "-C", str(repository), "status", "--porcelain"], text=True))
            source = repository / "scripts/save-acceptance.py"
            original = source.read_bytes()
            source.write_bytes(original + b"\n# Changed source control\n")
            with mock.patch.object(harness, "invoke", side_effect=AssertionError("changed source must not execute")):
                with self.assertRaisesRegex(ValueError, "identity differs"):
                    harness.run(workspace, self.executable)
            source.write_bytes(original)
            untracked = repository / "untracked caf\u00e9\nsource.rs"
            untracked.write_text("unrelated source control")
            with mock.patch.object(harness, "invoke", side_effect=AssertionError("changed source must not execute")):
                with self.assertRaisesRegex(ValueError, "identity differs.*source_status"):
                    harness.run(workspace, self.executable)
            untracked.unlink()
            tracked_capture = "save-pair/REFERENCE-INSTRUCTIONS.txt"
            subprocess.run(["git", "-C", str(repository), "add", "--", tracked_capture], check=True)
            self.assertIn("REFERENCE-INSTRUCTIONS.txt", harness.identity(self.executable, workspace)["source_status"])
            with mock.patch.object(harness, "invoke", side_effect=AssertionError("tracked changes must not execute")):
                with self.assertRaisesRegex(ValueError, "identity differs"):
                    harness.run(workspace, self.executable)
            subprocess.run(["git", "-C", str(repository), "reset", "-q", "HEAD", "--", tracked_capture], check=True)
            self.assertTrue(harness.run(workspace, self.executable)["passed"])
            for operation in harness.OPERATIONS:
                for stage in ("saved", "undone", "redone"):
                    shutil.copytree(workspace / "taggroove" / operation / stage, workspace / "reference" / operation / stage)
            reference = json.loads(self.identity_path.read_text())
            contract = json.loads((workspace / "contract.json").read_text())
            reference["input_sha256"] = {name: record["input_sha256"] for name, record in contract["baseline"].items()}
            reference["stage_sha256"] = {operation: {stage: {
                name: harness.sha256((workspace / "reference" / operation / stage / name).read_bytes())
                for name in contract["baseline"]} for stage in ("saved", "undone", "redone")}
                for operation in harness.OPERATIONS}
            reference_path = workspace / "synthetic-reference-control.json"
            harness.write_json(reference_path, reference)
            self.assertTrue(harness.compare(workspace, self.executable, reference_path)["passed"])

    def test_partly_ignored_repository_workspace_keeps_capture_data_out_of_source_identity(self):
        repository = self.source_repository("save-pair/*\n!save-pair/contract.json\n")
        workspace = repository / "save-pair"
        with mock.patch.object(harness, "REPO", repository):
            harness.prepare(workspace, self.executable)
            self.assertTrue(harness.run(workspace, self.executable)["passed"])

    def test_ignored_repository_workspace_works_and_keeps_source_identity_checks(self):
        repository = self.source_repository("save-pair/\n")
        workspace = repository / "save-pair"
        with mock.patch.object(harness, "REPO", repository):
            harness.prepare(workspace, self.executable)
            source = repository / "scripts/save-acceptance.py"
            original = source.read_bytes()
            source.write_bytes(original + b"\n# Changed source identity control\n")
            with mock.patch.object(harness, "invoke", side_effect=AssertionError("changed source must not execute")):
                with self.assertRaisesRegex(ValueError, "identity differs"):
                    harness.run(workspace, self.executable)
            source.write_bytes(original)
            self.assertTrue(harness.run(workspace, self.executable)["passed"])

    def test_ignored_workspace_cannot_live_in_frozen_carrier_inventory(self):
        relative = "crates/tag-core/tests/fixtures/save-corpus/save-pair"
        repository = self.source_repository(relative + "/\n")
        workspace = repository / relative
        with mock.patch.object(harness, "REPO", repository):
            with self.assertRaisesRegex(ValueError, "workspace.*source inputs"):
                harness.prepare(workspace, self.executable)
        self.assertFalse(workspace.exists())

    def test_symlink_alias_repository_workspace_uses_the_same_capture_data_scope(self):
        repository = self.source_repository()
        alias = Path(self.temporary.name) / "repository-alias"
        try:
            alias.symlink_to(repository, target_is_directory=True)
        except OSError as error:
            self.skipTest(f"directory symlinks unavailable: {error}")
        with mock.patch.object(harness, "REPO", repository):
            harness.prepare(alias / "save-pair", self.executable)
            self.assertTrue(harness.run(repository / "save-pair", self.executable)["passed"])
            producer = json.loads((repository / "save-pair/contract.json").read_text())["producer"]
            self.assertEqual(producer["untracked_capture_workspace"], "save-pair")

    def test_source_and_git_administration_workspaces_reject_before_backend_build(self):
        repository = self.source_repository()
        paths = [repository, repository.parent, repository / ".git/save-pair",
                 repository / "crates/tag-core/tests/fixtures/save-corpus/save-pair"]
        with mock.patch.object(harness, "REPO", repository):
            for workspace in paths:
                with self.subTest(workspace=workspace):
                    with mock.patch.object(harness.sys, "argv", ["save-acceptance.py", "prepare", str(workspace)]):
                        with mock.patch.object(harness, "backend", side_effect=AssertionError("backend must not build")) as backend:
                            with self.assertRaisesRegex(ValueError, "workspace"):
                                harness.main()
                            backend.assert_not_called()
        self.assertFalse((repository / ".git/save-pair").exists())


if __name__ == "__main__":
    unittest.main()
