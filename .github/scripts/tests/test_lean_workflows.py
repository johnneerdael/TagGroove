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
    def test_frontend_suite_runs_once_before_expensive_native_work(self):
        source = (WORKFLOWS / "_reviewed-build.yml").read_text()
        self.assertEqual(source.count("run: npm run test:ui"), 1)
        tooling = job(source, "tooling")
        self.assertIn("run: npm ci", tooling)
        self.assertIn("run: npm run test:ui", tooling)
        for name in ("desktop", "android"):
            block = job(source, name)
            self.assertIn("needs: [authorize, tooling]", block)
            self.assertNotIn("npm run test:ui", block)

    def test_each_desktop_reuses_its_workspace_for_rust_and_native_ui(self):
        source = (WORKFLOWS / "_reviewed-build.yml").read_text()
        self.assertNotIn("\n  native-e2e:", source)
        desktop = job(source, "desktop")
        for os in ("ubuntu-24.04", "windows-latest", "macos-latest"):
            self.assertIn(os, desktop)
        for check in ("cargo fmt --all -- --check", "cargo test --workspace --locked",
                      "cargo clippy --workspace --all-targets --locked -- -D warnings",
                      "Build the debug app once", "scripts/native-test-config.py",
                      "Run native suites", "--no-build", "OPENTAG_E2E_OFFLINE=1",
                      "Capture user-guide screenshots", "native-e2e-output-"):
            self.assertIn(check, desktop)
        self.assertNotIn("run: npm run tauri build", desktop)
        self.assertEqual(desktop.count("OPENTAG_AUDIO_BUILD_ROOT="), 1)
        for dependency in ("xvfb", "dbus-x11", "xdotool", "imagemagick"):
            self.assertIn(dependency, desktop)

    def test_windows_installer_fixture_still_executes_on_windows(self):
        source = (WORKFLOWS / "_reviewed-build.yml").read_text()
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
