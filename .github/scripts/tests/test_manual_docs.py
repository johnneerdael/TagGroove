"""Keep manual Pages publication separate from application execution."""
import os
from pathlib import Path
import re
import subprocess
import tempfile
import textwrap
import unittest

WORKFLOW = Path(__file__).resolve().parents[2] / "workflows/docs-manual.yml"
SOURCE = "a" * 40


class ManualDocsTests(unittest.TestCase):
    def source(self):
        self.assertTrue(WORKFLOW.exists(), "Manual docs-only workflow is missing")
        return WORKFLOW.read_text()

    def job(self, name):
        match = re.search(rf"^  {name}:\n(.*?)(?=^  [a-z0-9_-]+:|\Z)",
                          self.source().split("jobs:\n", 1)[1], re.M | re.S)
        self.assertIsNotNone(match)
        return match[1]

    def test_only_manual_trigger_and_docs_jobs(self):
        source = self.source()
        triggers = source.split("on:\n", 1)[1].split("\npermissions:", 1)[0]
        self.assertEqual(triggers.strip(), "workflow_dispatch:")
        jobs = re.findall(r"^  ([a-z0-9_-]+):$", source.split("jobs:\n", 1)[1], re.M)
        self.assertEqual(set(jobs), {"build", "deploy"})
        for forbidden in ["cargo ", "npm ", "tauri", "e2e-native", "docs-screenshots", "release.mjs", "secrets."]:
            self.assertNotIn(forbidden, source)

    def test_trusted_owner_main_source_and_scoped_deployment_permissions(self):
        source = self.source()
        for name in ["build", "deploy"]:
            block = self.job(name)
            for guard in ["github.event_name == 'workflow_dispatch'", "github.ref == 'refs/heads/main'",
                          "github.actor == github.repository_owner", "docs-manual.yml@refs/heads/main"]:
                self.assertIn(guard, block)
            self.assertIn("ref: ${{ github.sha }}", block)
            self.assertIn("persist-credentials: false", block)
        self.assertNotIn("pages: write", self.job("build"))
        self.assertNotIn("id-token: write", self.job("build"))
        deploy = self.job("deploy")
        for required in ["needs: build", "pages: write", "id-token: write", "name: github-pages",
                         "group: opentag-pages", "cancel-in-progress: false",
                         "if: steps.source.outputs.current == 'true'", "uses: actions/deploy-pages@v4"]:
            self.assertIn(required, deploy)
        self.assertIn("python -m mkdocs build --strict", self.job("build"))
        self.assertIn("taggroove.neerdael.nl", self.job("build"))
        self.assertIn("uses: actions/upload-pages-artifact@v3", self.job("build"))

    def guard(self, current, api_exit=0):
        block = self.job("deploy")
        step = block.split("      - name: Verify main before deployment\n", 1)[1].split("      - uses:", 1)[0]
        script = textwrap.dedent(step.split("run: |\n", 1)[1])
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            fake_gh = root / "gh"
            fake_gh.write_text("#!/usr/bin/env python3\nimport json, os, sys\n"
                               "assert sys.argv[1:] == ['api', 'repos/johnneerdael/TagGroove/git/ref/heads/main', '--jq', '.object.sha']\n"
                               "print(os.environ['DOCS_TEST_CURRENT'])\n"
                               "sys.exit(int(os.environ['DOCS_TEST_EXIT']))\n")
            fake_gh.chmod(0o755)
            output = root / "output"
            result = subprocess.run(["bash", "-c", script], capture_output=True, text=True,
                                    env=dict(os.environ, PATH=str(root)+os.pathsep+os.environ["PATH"],
                                             GITHUB_OUTPUT=str(output), GITHUB_SHA=SOURCE,
                                             GITHUB_REPOSITORY="johnneerdael/TagGroove",
                                             DOCS_TEST_CURRENT=current, DOCS_TEST_EXIT=str(api_exit)))
            return result, output.read_text() if output.exists() else ""

    def test_current_main_can_deploy(self):
        result, output = self.guard(SOURCE)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "current=true\n")

    def test_newer_main_prevents_old_deployment(self):
        result, output = self.guard("b" * 40)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "current=false\n")

    def test_api_failure_cannot_authorize_deployment(self):
        result, output = self.guard(SOURCE, 1)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("current=true", output)


if __name__ == "__main__":
    unittest.main()
