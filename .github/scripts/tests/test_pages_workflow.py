"""Keep user-guide execution and deployment inside reviewed publication."""
from pathlib import Path
import re
import unittest

WORKFLOWS = Path(__file__).resolve().parents[2] / "workflows"


class PagesTrustTests(unittest.TestCase):
    def setUp(self):
        self.source = (WORKFLOWS / "_reviewed-pages.yml").read_text()

    def block(self, name):
        return re.search(rf"^  {name}:\n(.*?)(?=^  [a-z-]+:|\Z)", self.source, re.M | re.S)[1]

    def test_legacy_docs_and_screenshot_automatic_workflows_are_removed(self):
        for name in ["docs.yml", "docs-build.yml", "pages-deploy.yml", "docs-screenshots.yml"]:
            self.assertFalse((WORKFLOWS / name).exists(), name)
        header = self.source.split("jobs:")[0]
        self.assertIn("  workflow_call:", header)
        for trigger in ["push", "pull_request", "pull_request_target", "workflow_run", "workflow_dispatch", "schedule"]:
            self.assertNotIn(f"  {trigger}:", header)

    def test_only_the_trusted_main_release_caller_can_authorize_pages(self):
        authorize = self.block("authorize")
        self.assertIn("github.ref == 'refs/heads/main'", authorize)
        self.assertIn("github.workflow_ref == format('{0}/.github/workflows/reviewed-release.yml@refs/heads/main', github.repository)", authorize)
        self.assertIn("ref: main", authorize)
        self.assertIn("persist-credentials: false", authorize)
        self.assertIn("review_gate.py release-check", authorize)
        for key in ["inputs.pr_number", "inputs.head_sha", "inputs.source_sha"]:
            self.assertIn(key, authorize)

    def test_source_execution_needs_authorization_and_cannot_deploy(self):
        build = self.block("build")
        self.assertIn("needs: authorize", build)
        self.assertIn("ref: ${{ inputs.source_sha }}", build)
        self.assertIn("persist-credentials: false", build)
        for command in ["python -m pip install -r docs/site-requirements.txt",
                        "python -m mkdocs build --strict"]:
            self.assertIn(command, build)
        self.assertNotIn("unittest discover", build)
        self.assertNotIn("docs-status.py", build)
        self.assertNotIn("GH_TOKEN", build)
        self.assertNotIn("pages: write", build)
        self.assertNotIn("id-token: write", build)
        self.assertNotIn("cache:", self.source)
        self.assertNotIn("secrets.", self.source)

    def test_deployment_rechecks_source_and_uses_only_the_checked_artifact(self):
        deploy = self.block("deploy")
        self.assertIn("needs: [authorize, build]", deploy)
        self.assertIn("ref: main", deploy)
        self.assertIn("persist-credentials: false", deploy)
        self.assertIn("review_gate.py release-check", deploy)
        self.assertLess(deploy.index("review_gate.py release-check"), deploy.index("actions/deploy-pages"))
        self.assertIn('if [ "$SOURCE_SHA" = "$current_sha" ]', deploy)
        self.assertIn("if: steps.source.outputs.current == 'true'", deploy)
        self.assertIn("name: github-pages", deploy)
        self.assertIn("pages: write", deploy)
        self.assertIn("id-token: write", deploy)
        self.assertNotIn("inputs.source_sha }}\n          persist-credentials", deploy)
        self.assertEqual(self.source.count("review_gate.py release-check"), 2)
        self.assertNotIn("guide_pages.py", self.source)

    def test_build_publishes_the_site_under_the_custom_domain(self):
        build = self.block("build")
        self.assertIn("echo 'taggroove.neerdael.nl' > build/user-guide-site/CNAME", build)
        upload = build.split("actions/upload-pages-artifact@v3", 1)[1]
        self.assertIn("path: build/user-guide-site", upload)

    def test_no_deploy_key_or_cross_repository_publication_remains(self):
        release = (WORKFLOWS / "reviewed-release.yml").read_text()
        for text in (self.source, release):
            self.assertNotIn("DOCS_PAGES_DEPLOY_KEY", text)
            self.assertNotIn("audio-tag-guide", text)
        for name in ("guide-pages-target.yml", "guide-pages-allowed-signers", "guide-pages-known-hosts"):
            self.assertFalse((WORKFLOWS.parent / name).exists(), name)

    def test_docs_only_sources_publish_independently_of_application_release_skip(self):
        release = (WORKFLOWS / "reviewed-release.yml").read_text()
        job = re.search(r"^  user-guide:\n(.*?)(?=^  [a-z-]+:|\Z)", release, re.M | re.S)[1]
        self.assertIn("needs: authorize", job)
        self.assertIn("uses: ./.github/workflows/_reviewed-pages.yml", job)
        self.assertNotIn("prepare", job)
        for key in ["pr_number", "head_sha", "source_sha"]:
            self.assertIn(f"{key}: ${{{{ inputs.{key} }}}}", job)


if __name__ == "__main__":
    unittest.main()
