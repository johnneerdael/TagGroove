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
        workflow = (ROOT / ".github/workflows/_reviewed-build.yml").read_text()
        native = workflow.split("  native-e2e:\n", 1)[1].split("  android:\n", 1)[0]
        self.assertIn("needs: authorize", native)
        self.assertIn("if: needs.authorize.outputs.ready == 'true'", native)
        self.assertIn("python3 scripts/native-test-config.py", native)
        self.assertIn("if: runner.os == 'Linux'\n        run: scripts/docs-screenshots.sh", native)
        self.assertIn("name: docs-screens\n", native)
        self.assertIn("if-no-files-found: error", native)
        self.assertIn("xdotool imagemagick x11-utils", native)

    def test_window_selection_handles_directory_titles_and_refuses_root_fallback(self):
        script = (ROOT / "scripts/docs-screenshots.sh").read_text()
        self.assertIn('re.escape(sys.argv[1]) + "( — .*)?$"', script)
        self.assertIn('if [[ -z "$window" ]]; then', script)
        self.assertIn('refusing a desktop capture', script)
        self.assertNotIn('window=root', script)

    def test_capture_refuses_local_execution_before_creating_fixtures(self):
        result = subprocess.run(["bash", str(ROOT / "scripts/docs-screenshots.sh")],
                                env={**os.environ, "CI": ""}, capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn("reviewed Linux CI job", result.stderr)


if __name__ == "__main__":
    unittest.main()
