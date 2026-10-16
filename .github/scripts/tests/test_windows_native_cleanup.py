"""Execute the actual cleanup block with shell stubs; never launch or kill an app."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]


class WindowsNativeCleanupTests(unittest.TestCase):
    def test_hosted_only_webview_termination_and_delayed_profile_release(self):
        source = (ROOT / "scripts/e2e-native.sh").read_text()
        block = source.split("# Start every run from empty app state;", 1)[1].split('work="$(mktemp -d)"', 1)[0]
        block = block[block.index('case "$identifier" in'):]
        cases = [
            ("hosted", "true", "github-hosted", "org.opentag.editor.usertest", True),
            ("self-hosted", "true", "self-hosted", "org.opentag.editor.usertest", False),
            ("local", "", "", "org.opentag.editor.usertest", False),
            ("not-actions", "false", "github-hosted", "org.opentag.editor.usertest", False),
            ("production-id", "true", "github-hosted", "org.opentag.editor", False),
        ]
        for label, actions, environment, identifier, terminates in cases:
            with self.subTest(label=label), tempfile.TemporaryDirectory() as directory:
                folder = Path(directory)
                scripts = {
                    "cygpath": 'printf "%s\\n" "$2"\n',
                    "taskkill": 'printf "taskkill %s\\n" "$*" >> "$TRACE"\n',
                    "sleep": 'printf "sleep %s\\n" "$*" >> "$TRACE"\n',
                    "seq": 'for ((i=1;i<=$1;i++)); do printf "%s\\n" "$i"; done\n',
                    "rm": 'n=0; if [[ -f "$COUNTER" ]]; then read -r n < "$COUNTER"; fi; n=$((n+1)); printf "%s\\n" "$n" > "$COUNTER"; printf "rm:%s %s\\n" "$n" "$*" >> "$TRACE"; (( n > 7 ))\n',
                }
                for name, body in scripts.items():
                    path = folder / name
                    path.write_text("#!/bin/bash\n" + body)
                    path.chmod(0o755)
                trace, counter = folder / "trace", folder / "counter"
                env = {**os.environ, "PATH": str(folder) + os.pathsep + os.environ["PATH"],
                       "CI": "true", "GITHUB_ACTIONS": actions, "RUNNER_ENVIRONMENT": environment,
                       "TEST_IDENTIFIER": identifier, "APPDATA": "/owned-fixture/roaming",
                       "LOCALAPPDATA": "/owned-fixture/local", "TRACE": str(trace), "COUNTER": str(counter)}
                result = subprocess.run(["bash", "-c", 'set -euo pipefail\nos=windows\nidentifier="$TEST_IDENTIFIER"\n' + block],
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                lines = trace.read_text().splitlines() if trace.exists() else []
                kills = [line for line in lines if line.startswith("taskkill ")]
                self.assertEqual(len(kills), int(terminates))
                if identifier == "org.opentag.editor":
                    self.assertEqual(lines, [])
                    continue
                removals = [line for line in lines if line.startswith("rm:")]
                self.assertEqual(len(removals), 9, "Must tolerate seven busy attempts, then final cleanup")
                self.assertTrue(all(identifier in line and "/owned-fixture/" in line for line in removals))
                self.assertEqual([line for line in lines if line.startswith("sleep ")], ["sleep 2"] * 7)
                if terminates:
                    self.assertEqual(lines[0], "taskkill //F //IM msedgewebview2.exe //T")
