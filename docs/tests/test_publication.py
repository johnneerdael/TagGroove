"""Exercise the publication boundary through real strict MkDocs builds."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


HOOK = Path(__file__).resolve().parents[1] / "hooks" / "publication.py"


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "docs").mkdir()
        (self.root / "docs" / "index.md").write_text("# Welcome\n\nReady to read.\n")
        (self.root / "docs" / "draft.md").write_text(
            "---\nstatus: draft\n---\n# Unfinished topic\n\nSECRET_DRAFT_WORDS\n"
        )

    def build(self, nav="  - Home: index.md\n"):
        config = self.root / "mkdocs.yml"
        config.write_text(
            "site_name: Publication test\nhooks:\n"
            f"  - {json.dumps(str(HOOK))}\n"
            "validation:\n  nav:\n    omitted_files: warn\n"
            "  links:\n    not_found: warn\n    anchors: warn\n"
            "nav:\n" + nav
        )
        return subprocess.run(
            [sys.executable, "-m", "mkdocs", "build", "--strict", "-f", str(config)],
            capture_output=True,
            text=True,
        )

    def test_draft_is_absent_from_html_and_search(self):
        result = self.build()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertTrue((self.root / "site" / "index.html").exists())
        self.assertFalse((self.root / "site" / "draft").exists())
        search = (self.root / "site" / "search" / "search_index.json").read_text()
        self.assertNotIn("SECRET_DRAFT_WORDS", search)
        self.assertNotIn("draft/", search)

    def test_link_to_draft_fails_strict_build(self):
        (self.root / "docs" / "index.md").write_text("# Welcome\n[Draft](draft.md)\n")
        result = self.build()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("draft.md", result.stderr)

    def test_draft_in_public_navigation_fails_strict_build(self):
        result = self.build("  - Home: index.md\n  - Draft: draft.md\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("draft.md", result.stderr)

    def test_completed_page_requires_navigation(self):
        (self.root / "docs" / "ready.md").write_text("# Completed topic\nUseful help.\n")
        result = self.build()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("ready.md", result.stderr)

    def test_completed_page_can_be_published(self):
        (self.root / "docs" / "draft.md").write_text("# Completed topic\nUseful help.\n")
        result = self.build("  - Home: index.md\n  - Completed topic: draft.md\n")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertTrue((self.root / "site" / "draft" / "index.html").exists())
        search = (self.root / "site" / "search" / "search_index.json").read_text()
        self.assertIn("Completed topic", search)


if __name__ == "__main__":
    unittest.main()
