"""Execution-contract checks for the reviewed validation/release split."""
import re
import unittest
from pathlib import Path

WORKFLOWS = Path(__file__).resolve().parents[2] / "workflows"


def job(source, name):
    match = re.search(rf"^  {name}:\n(.*?)(?=^  [a-z0-9_-]+:|\Z)", source, re.M | re.S)
    if not match:
        raise AssertionError(f"Missing job {name}")
    return match[1]


class LeanWorkflows(unittest.TestCase):
    def test_premerge_runs_checks_on_one_linux_worker_without_app_builds(self):
        source = (WORKFLOWS / "_reviewed-build.yml").read_text()
        self.assertEqual(set(re.findall(r"^  ([a-z0-9_-]+):$", source, re.M)), {"workflow_call", "authorize", "tooling"})
        tooling = job(source, "tooling")
        self.assertIn("runs-on: ubuntu-24.04", tooling)
        for check in ("npm run test:ui", "cargo test --workspace --locked",
                      "cargo fmt --all -- --check", "cargo clippy --workspace --all-targets --locked -- -D warnings",
                      "npx tsc --noEmit", "python -m mkdocs build --strict"):
            self.assertIn(check, tooling)
        self.assertNotIn("node --test tests/release.test.mjs", source)
        for build in ("tauri build", "android build", "e2e-native.sh", "docs-screenshots.sh", "matrix:", "secrets.", "cache:"):
            self.assertNotIn(build, source)

    def test_native_suites_run_only_in_authorized_postmerge_release(self):
        source = (WORKFLOWS / "_reviewed-native.yml").read_text()
        authorize = job(source, "authorize")
        self.assertIn("reviewed-release.yml@refs/heads/main", authorize)
        self.assertIn("review_gate.py release-check", authorize)
        self.assertNotIn("secrets.", source)
        self.assertNotIn("cache:", source)
        for name in ("desktop", "android"):
            block = job(source, name)
            self.assertIn("needs: authorize", block)
            self.assertIn("if: needs.authorize.outputs.ready == 'true'", block)
            self.assertIn("ref: ${{ inputs.source_sha }}", block)
            for check in ("cargo test", "cargo clippy", "cargo fmt", "npm run test:ui"):
                self.assertNotIn(check, block)
        desktop = job(source, "desktop")
        self.assertIn("fail-fast: true", desktop)
        self.assertNotIn("status=1", desktop)
        for check in ("Build the debug app once", "Run native suites", "--no-build", "OPENTAG_E2E_OFFLINE=1"):
            self.assertIn(check, desktop)
        release = (WORKFLOWS / "reviewed-release.yml").read_text()
        native = job(release, "native-validation")
        self.assertIn("needs: [authorize, prepare]", native)
        self.assertIn("if: needs.prepare.outputs.skip != 'true'", native)
        self.assertIn("uses: ./.github/workflows/_reviewed-native.yml", native)
        self.assertIn("native-validation", job(release, "publish"))
        self.assertNotIn("node --test tests/release.test.mjs", release)

    def test_windows_installer_fixture_still_executes_on_windows(self):
        source = (WORKFLOWS / "_reviewed-native.yml").read_text()
        desktop = job(source, "desktop")
        step = desktop.split("      - name: Check native Windows installer shortcuts\n", 1)[1].split("      - ", 1)[0]
        self.assertIn("if: runner.os == 'Windows'", step)
        self.assertIn("node --test tests/nsis-shortcuts.test.mjs", step)
        self.assertLess(desktop.index("run: npm ci"), desktop.index("node --test tests/nsis-shortcuts.test.mjs"))

    def test_release_packages_validated_source_without_repeating_unit_checks(self):
        source = (WORKFLOWS / "reviewed-release.yml").read_text()
        for name in ("desktop", "android", "ios"):
            block = job(source, name)
            for command in ("npm run test:ui", "cargo test", "cargo clippy", "cargo fmt"):
                self.assertNotIn(command, block, f"{name}: duplicate {command}")
        desktop = job(source, "desktop")
        for required in ("Build desktop packages", "Check macOS document types",
                         "collect-desktop", "--collect-native", "OPENTAG_AUDIO_BUILD_ROOT"):
            self.assertIn(required, desktop)
        self.assertNotIn("choco install ffmpeg", desktop)
        self.assertNotIn("brew install ffmpeg", desktop)
        self.assertEqual(source.count("review_gate.py release-check"), 2)
        self.assertIn("Package and independently verify complete source", source)
        self.assertIn("Verify all platforms, calculate SHA256SUMS", source)


if __name__ == "__main__":
    unittest.main()
