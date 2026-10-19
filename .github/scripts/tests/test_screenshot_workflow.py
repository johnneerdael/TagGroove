"""Reviewed screenshot capture follows the checked-out product, never a brand literal."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import sys
import unittest


ROOT = Path(__file__).resolve().parents[3]


class ScreenshotWorkflowTests(unittest.TestCase):
    def test_native_config_follows_each_checked_out_product_and_os(self):
        script = ROOT / "scripts/native-test-config.py"
        for product in ["OpenTag", "TagGroove"]:
            for platform in ["Linux", "macOS", "Windows"]:
                with self.subTest(product=product, platform=platform), tempfile.TemporaryDirectory() as work:
                    directory = Path(work) / "src-tauri"
                    directory.mkdir()
                    (directory / "tauri.conf.json").write_text(json.dumps({"productName": product}))
                    binary = "taggroove" if product == "TagGroove" else "opentag"
                    cargo = '[package]\nname = "opentag"\n'
                    if product == "TagGroove":
                        cargo += f'[[bin]]\nname = "{binary}"\n'
                    (directory / "Cargo.toml").write_text(cargo)
                    config = json.loads(subprocess.check_output(
                        [sys.executable, str(script)], cwd=work,
                        env={**os.environ, "RUNNER_OS": platform}, text=True))
                    self.assertEqual(config["identifier"], "org.opentag.editor.usertest")
                    self.assertEqual(config["productName"], product + " UserTest")
                    os_name = {"Linux": "linux", "macOS": "macos", "Windows": "windows"}[platform]
                    path = subprocess.check_output([sys.executable, str(script), "--binary-path", os_name], cwd=work, text=True).strip()
                    if platform == "macOS":
                        self.assertEqual(path, f"target/debug/bundle/macos/{product} UserTest.app/Contents/MacOS/{binary}")
                    else:
                        self.assertEqual(path, f"target/debug/{binary}" + (".exe" if platform == "Windows" else ""))
                    if platform == "Linux":
                        self.assertNotIn("app", config)
                    else:
                        window = config["app"]["windows"][0]
                        self.assertEqual(window["title"], product)
                        self.assertEqual(window["backgroundThrottling"], "disabled")

    def test_capture_is_inside_the_review_authorized_linux_job(self):
        workflow = (ROOT / ".github/workflows/_reviewed-native.yml").read_text()
        native = workflow.split("  desktop:\n", 1)[1].split("  android:\n", 1)[0]
        self.assertIn("needs: authorize", native)
        self.assertIn("if: needs.authorize.outputs.ready == 'true'", native)
        self.assertIn("python scripts/native-test-config.py", native)
        self.assertIn("if: runner.os == 'Linux'\n        run: scripts/docs-screenshots.sh", native)
        self.assertIn("name: docs-screens\n", native)
        self.assertIn("if-no-files-found: error", native)
        self.assertIn("xdotool imagemagick x11-utils", native)

    def test_relaunch_suites_consume_the_resolved_executable(self):
        runner = (ROOT / "scripts/e2e-native.sh").read_text()
        self.assertIn('export OPENTAG_E2E_BINARY="$bin"', runner)
        self.assertIn('window.OPENTAG_E2E_BINARY=%s;', runner)
        self.assertIn('python_executable="${PYTHON:-python}"', runner)
        for suite in ["playlists", "chapters", "chapter-fields"]:
            script = (ROOT / f"tests/e2e/{suite}.js").read_text()
            self.assertIn("const binary = window.OPENTAG_E2E_BINARY;", script)
            self.assertNotIn("/MacOS/opentag", script)
        fixture = (ROOT / "tests/e2e/launch-forwarding-fixtures.sh").read_text()
        self.assertIn("${OPENTAG_E2E_BINARY:?", fixture)

    def capture_with_fake_x11(self, child="456", windows="11 12"):
        script = (ROOT / "scripts/docs-screenshots.sh").read_text()
        capture = "capture() {" + script.split("capture() {", 1)[1].split("\n}\n", 1)[0] + "\n}\n"
        with tempfile.TemporaryDirectory() as directory:
            trace = Path(directory) / "captured-window"
            # Model only X11/process queries and image export; run the actual
            # production selection function without creating desktop windows.
            harness = r'''
set -euo pipefail
pid=123
product=TagGroove
window_pattern='^TagGroove( — .*)?$'
pgrep() { [[ "$*" == '-P 123' && -n "$CHILD" ]] && printf '%s\n' "$CHILD"; }
xdotool() {
  # The app's runtime title deliberately differs from productName.
  [[ "$*" == 'search --onlyvisible --pid 456' ]] && printf '%s\n' $WINDOWS
}
xwininfo() {
  if [[ "$2" == 11 ]]; then printf 'Width: 320\nHeight: 200\n';
  else printf 'Width: 1240\nHeight: 800\n'; fi
}
import() { printf '%s\n' "$2" > "$TRACE"; }
pngquant() { :; }
identify() { printf '1240x800'; }
''' + capture + '\ncapture output.png\n'
            result = subprocess.run(["bash", "-c", harness], capture_output=True, text=True,
                                    env={**os.environ, "CHILD": child, "WINDOWS": windows,
                                         "TRACE": str(trace)})
            return result, trace.read_text().strip() if trace.exists() else None

    def test_capture_uses_owned_process_when_runtime_title_differs(self):
        result, window = self.capture_with_fake_x11()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(window, "12", "Capture the largest window owned by the test app")

    def test_capture_refuses_desktop_fallback_without_an_owned_window(self):
        result, window = self.capture_with_fake_x11(windows="")
        self.assertNotEqual(result.returncode, 0)
        self.assertIsNone(window)

    def test_capture_refuses_export_without_the_native_child(self):
        result, window = self.capture_with_fake_x11(child="")
        self.assertNotEqual(result.returncode, 0)
        self.assertIsNone(window)

    def test_capture_refuses_local_execution_before_creating_fixtures(self):
        result = subprocess.run(["bash", str(ROOT / "scripts/docs-screenshots.sh")],
                                env={**os.environ, "CI": ""}, capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn("reviewed Linux CI job", result.stderr)


if __name__ == "__main__":
    unittest.main()
