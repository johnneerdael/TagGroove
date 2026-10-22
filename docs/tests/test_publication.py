"""Exercise the publication boundary through real strict MkDocs builds."""

from html.parser import HTMLParser
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


HOOK = Path(__file__).resolve().parents[1] / "hooks" / "publication.py"
REDIRECTS = HOOK.with_name("redirects.py")


class ScreenshotArtifactTests(unittest.TestCase):
    def test_fetch_requires_an_explicit_run_without_querying_github(self):
        root = HOOK.parents[2]
        with tempfile.TemporaryDirectory() as directory:
            fake_gh = Path(directory) / "gh"
            fake_gh.write_text("#!/bin/sh\necho UNEXPECTED_GITHUB_CALL >&2\nexit 99\n")
            fake_gh.chmod(0o755)
            result = subprocess.run(
                ["bash", str(root / "scripts" / "docs-screens-fetch.sh")],
                capture_output=True,
                text=True,
                env={**os.environ, "PATH": directory + os.pathsep + os.environ["PATH"]},
            )
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("explicit numeric run ID", result.stderr)
        self.assertNotIn("UNEXPECTED_GITHUB_CALL", result.stderr)


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


class PrerequisiteParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.items = []
        self.stack = []

    def handle_starttag(self, tag, attrs):
        if tag == "li":
            self.stack.append({"text": "", "checkboxes": 0, "links": []})
        if not self.stack:
            return
        attrs = dict(attrs)
        if tag == "input" and attrs.get("type") == "checkbox":
            self.stack[-1]["checkboxes"] += 1
        if tag == "a":
            self.stack[-1]["links"].append(attrs.get("href"))

    def handle_data(self, text):
        if self.stack:
            self.stack[-1]["text"] += text

    def handle_endtag(self, tag):
        if tag == "li" and self.stack:
            self.items.append(self.stack.pop())


class CodeParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.codes = []
        self.current = None

    def handle_starttag(self, tag, attrs):
        if tag == "code":
            self.current = ""

    def handle_data(self, text):
        if self.current is not None:
            self.current += text

    def handle_endtag(self, tag):
        if tag == "code" and self.current is not None:
            self.codes.append(self.current)
            self.current = None


class RedirectTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        docs = self.root / "docs"
        (docs / "newhome").mkdir(parents=True)
        (docs / "index.md").write_text("# Welcome\n\n[Moved](newhome/page.md)\n")
        (docs / "newhome" / "page.md").write_text("# Moved page\n\nIt lives here now.\n")

    def build(self, moves):
        (self.root / "redirects.json").write_text(json.dumps(moves))
        config = self.root / "mkdocs.yml"
        config.write_text(
            "site_name: Redirect test\nsite_url: https://example.test/\nhooks:\n"
            f"  - {json.dumps(str(HOOK))}\n  - {json.dumps(str(REDIRECTS))}\n"
            "extra:\n  redirects_file: redirects.json\n"
            "validation:\n  nav:\n    omitted_files: warn\n  links:\n    not_found: warn\n"
            "nav:\n  - Home: index.md\n  - Moved page: newhome/page.md\n"
        )
        return subprocess.run(
            [sys.executable, "-m", "mkdocs", "build", "--strict", "-f", str(config)],
            capture_output=True,
            text=True,
        )

    def test_old_url_forwards_and_keeps_query_and_anchor(self):
        result = self.build({"oldhome/page.md": "newhome/page.md", "oldhome/index.md": "newhome/page.md"})
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        stub = (self.root / "site" / "oldhome" / "page" / "index.html").read_text()
        self.assertIn('content="0; url=../../newhome/page/"', stub)
        self.assertIn('<link rel="canonical" href="https://example.test/newhome/page/">', stub)
        self.assertIn("location.search + location.hash", stub)
        self.assertIn('url=../newhome/page/"', (self.root / "site" / "oldhome" / "index.html").read_text())

    def test_redirect_pages_are_not_searchable(self):
        result = self.build({"oldhome/page.md": "newhome/page.md"})
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        search = (self.root / "site" / "search" / "search_index.json").read_text()
        self.assertNotIn("has moved", search)

    def test_target_must_be_a_published_page(self):
        result = self.build({"oldhome/page.md": "newhome/missing.md"})
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("newhome/missing.md", result.stderr)

    def test_redirect_cannot_hide_a_live_page(self):
        result = self.build({"newhome/page.md": "index.md"})
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("newhome/page.md", result.stderr)


class GuideRedirectMapTests(unittest.TestCase):
    """The shipped map must only describe pages that really moved."""

    def test_every_old_page_is_gone_and_every_target_exists(self):
        root = HOOK.parents[2]
        moves = json.loads(REDIRECTS.with_name("redirects.json").read_text())
        self.assertGreater(len(moves), 50)
        docs = root / "docs" / "user-guide"
        for old, new in moves.items():
            self.assertFalse((docs / old).exists(), f"{old} still exists")
            self.assertTrue((docs / new).is_file(), f"{new} is missing")


class GuideContentTests(unittest.TestCase):
    def test_first_session_has_installation_checkbox_and_download_link(self):
        root = HOOK.parents[2]
        with tempfile.TemporaryDirectory() as output:
            result = subprocess.run(
                [sys.executable, "-m", "mkdocs", "build", "--strict", "-f",
                 str(root / "mkdocs.yml"), "--site-dir", output],
                cwd=root,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            parser = PrerequisiteParser()
            parser.feed((Path(output) / "getting-started/first-tagging-session/index.html").read_text())
        matches = [item for item in parser.items if "TagGroove installed" in item["text"]]
        self.assertEqual(len(matches), 1)
        self.assertEqual(matches[0]["checkboxes"], 1)
        self.assertIn("https://github.com/johnneerdael/TagGroove/releases/latest", matches[0]["links"])


    def test_regex_examples_preserve_copyable_edge_spaces(self):
        root = HOOK.parents[2]
        with tempfile.TemporaryDirectory() as output:
            result = subprocess.run(
                [sys.executable, "-m", "mkdocs", "build", "--strict", "-f",
                 str(root / "mkdocs.yml"), "--site-dir", output],
                cwd=root,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            parser = CodeParser()
            parser.feed((Path(output) / "scripting/regular-expressions/index.html").read_text())
        for expected in [" ", r" \(Interlude\)", "Low Tide "]:
            self.assertIn(expected, parser.codes)


if __name__ == "__main__":
    unittest.main()
