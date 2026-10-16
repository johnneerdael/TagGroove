"""Exercise static publication, redirects and the cross-repository boundary."""
import importlib.util
import json
import os
from pathlib import Path
import re
import shlex
import stat
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "guide_pages.py"
SOURCE, HEAD = "a" * 40, "b" * 40


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(SCRIPT.is_file(), "Neutral static publisher is not implemented")
        spec = importlib.util.spec_from_file_location("guide_pages", SCRIPT)
        self.pages = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.pages)
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.site = self.root / "site"
        self.site.mkdir()
        for name in ["index.html", "404.html", "learn/example/index.html"]:
            path = self.site / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("<!doctype html><title>Guide</title>")
        (self.site / "search").mkdir()
        (self.site / "search/search_index.json").write_text(json.dumps({
            "docs": [{"location": "", "title": "Guide"},
                     {"location": "learn/example/#save", "title": "Example"}]}))
        self.redirects = self.root / "redirects"

    def prepare(self, source=SOURCE):
        return self.pages.prepare(self.site, self.redirects, source, HEAD, 34)

    def git(self, directory, *args):
        return subprocess.check_output([
            "git", "-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false",
            "-c", "user.name=Guide test", "-c", "user.email=test@example.invalid",
            "-C", str(directory), *args], text=True).strip()

    def repository(self):
        bare, checkout = self.root / "target.git", self.root / "checkout"
        subprocess.run(["git", "init", "--bare", "--initial-branch=main", str(bare)],
                       check=True, capture_output=True)
        subprocess.run(["git", "clone", str(bare), str(checkout)],
                       check=True, capture_output=True)
        (checkout / "old.html").write_text("Old publication")
        self.git(checkout, "add", ".")
        self.git(checkout, "commit", "-m", "Initial static target")
        self.git(checkout, "push", "origin", "HEAD:main")
        return bare, checkout

    def test_prepare_covers_each_html_route_and_has_no_external_redirect_dependencies(self):
        marker = self.prepare()
        self.assertEqual(marker["source_commit"], SOURCE)
        self.assertEqual(marker["canonical_url"], "https://johnneerdael.github.io/audio-tag-guide/")
        self.assertEqual(marker["completed_pages"], 2)
        self.assertTrue((self.site / ".nojekyll").exists())
        self.assertEqual(sorted(str(p.relative_to(self.redirects)) for p in self.redirects.rglob("*.html")),
                         ["404.html", "index.html", "learn/example/index.html"])
        for path in self.redirects.rglob("*.html"):
            text = path.read_text()
            self.assertNotIn('rel="stylesheet"', text)
            self.assertNotIn("<script src=", text)
            self.assertIn("https://johnneerdael.github.io/audio-tag-guide/", text)

    def test_actual_redirect_script_preserves_route_query_and_fragment_without_open_redirect(self):
        self.prepare()
        script = re.search(r"<script>(.*?)</script>", (self.redirects / "404.html").read_text(), re.S)[1]
        harness = """const vm=require('node:vm');const script=JSON.parse(process.argv[1]);
          const outputs=[];for(const pathname of ['/opentag/learn/example/','/opentag/','/opentag//evil.example/x']){
            let result;const location={pathname,search:'?q=two%20words&x=1',hash:'#save',replace:url=>result=url};
            vm.runInNewContext(script,{location,URL});outputs.push(result);}
          process.stdout.write(JSON.stringify(outputs));"""
        outputs = json.loads(subprocess.check_output(["node", "-e", harness, json.dumps(script)], text=True))
        self.assertEqual(outputs[:2], [
            "https://johnneerdael.github.io/audio-tag-guide/learn/example/?q=two%20words&x=1#save",
            "https://johnneerdael.github.io/audio-tag-guide/?q=two%20words&x=1#save"])
        self.assertTrue(outputs[2].startswith("https://johnneerdael.github.io/audio-tag-guide/"))

    def test_actual_temporary_git_target_replaces_only_static_content_and_reruns_without_a_commit(self):
        self.prepare()
        bare, checkout = self.repository()
        self.pages.publish_checkout(self.site, checkout, SOURCE, HEAD, 34)
        first = self.git(bare, "rev-parse", "main")
        self.assertFalse((checkout / "old.html").exists())
        self.assertTrue((checkout / ".git").is_dir())
        published = json.loads(self.git(bare, "show", "main:publication-source.json"))
        self.assertEqual(published["source_commit"], SOURCE)
        self.assertEqual(self.git(bare, "show", "main:.nojekyll"), "")
        self.pages.publish_checkout(self.site, checkout, SOURCE, HEAD, 34)
        self.assertEqual(self.git(bare, "rev-parse", "main"), first)

    def test_malformed_artifacts_are_rejected_before_touching_the_target(self):
        self.prepare()
        bare, checkout = self.repository()
        original = self.git(bare, "rev-parse", "main")
        for name in [".git/config", ".github/workflows/publish.yml", "_meta/private.json"]:
            bad = self.site / name
            bad.parent.mkdir(parents=True, exist_ok=True)
            bad.write_text("must never be published")
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.pages.publish_checkout(self.site, checkout, SOURCE, HEAD, 34)
            bad.unlink()
            bad.parent.rmdir()
            if name.startswith(".github/"):
                (self.site / ".github").rmdir()
            self.assertEqual(self.git(bare, "rev-parse", "main"), original)
            self.assertTrue((checkout / "old.html").exists())

    def test_symlink_hardlink_wrong_source_and_modified_bytes_fail_closed(self):
        self.prepare()
        target = self.site / "linked"
        target.symlink_to(self.site / "index.html")
        with self.assertRaises(ValueError):
            self.pages.validate_site(self.site, SOURCE, HEAD, 34)
        target.unlink()
        os.link(self.site / "index.html", target)
        with self.assertRaises(ValueError):
            self.pages.validate_site(self.site, SOURCE, HEAD, 34)
        target.unlink()
        with self.assertRaises(ValueError):
            self.pages.validate_site(self.site, "c" * 40, HEAD, 34)
        (self.site / "index.html").write_text("changed after the checked build")
        with self.assertRaises(ValueError):
            self.pages.validate_site(self.site, SOURCE, HEAD, 34)

    def test_missing_html_and_escaping_search_routes_are_rejected(self):
        (self.site / "learn/example/index.html").unlink()
        with self.assertRaises(ValueError):
            self.prepare()

        (self.site / "search/search_index.json").write_text(json.dumps({
            "docs": [{"location": "https://evil.example/"}]}))
        with self.assertRaises(ValueError):
            self.prepare()

    def test_artifact_cannot_change_the_fixed_pages_domain(self):
        (self.site / "CNAME").write_text("other.example")
        with self.assertRaises(ValueError):
            self.prepare()

    def test_non_fast_forward_target_update_is_not_forced(self):
        self.prepare()
        bare, checkout = self.repository()
        other = self.root / "other"
        subprocess.run(["git", "clone", str(bare), str(other)], check=True, capture_output=True)
        (other / "newer.html").write_text("Concurrent target publication")
        self.git(other, "add", ".")
        self.git(other, "commit", "-m", "Concurrent update")
        self.git(other, "push", "origin", "HEAD:main")
        newest = self.git(bare, "rev-parse", "main")
        with self.assertRaises(subprocess.CalledProcessError):
            self.pages.publish_checkout(self.site, checkout, SOURCE, HEAD, 34)
        self.assertEqual(self.git(bare, "rev-parse", "main"), newest)

    def test_public_marker_must_match_source_tree_and_review_before_redirect_publication(self):
        expected = self.prepare()
        wrong = {**expected, "source_commit": "c" * 40}
        observed = iter([b"not json", json.dumps(wrong).encode(), json.dumps(expected).encode()])
        with patch.object(self.pages.time, "sleep"):
            self.pages.wait_for_publication(expected, fetch=lambda: next(observed), attempts=3)
        with patch.object(self.pages.time, "sleep"), self.assertRaises(TimeoutError):
            self.pages.wait_for_publication(expected, fetch=lambda: json.dumps(wrong).encode(), attempts=2)

    def test_credential_is_not_required_for_prepare_or_marker_wait(self):
        with patch.dict(os.environ, {}, clear=True):
            marker = self.prepare()
            self.pages.wait_for_publication(marker, fetch=lambda: json.dumps(marker).encode(), attempts=1)

    def test_only_fixed_target_transport_receives_a_temporary_key_file_and_cleans_on_failure(self):
        self.prepare()
        observed = []

        def fail_transport(command, **kwargs):
            env = kwargs["env"]
            self.assertNotIn("DOCS_PAGES_DEPLOY_KEY", env)
            self.assertIn("git@github.com:johnneerdael/audio-tag-guide.git", command)
            ssh = shlex.split(env["GIT_SSH_COMMAND"])
            key = Path(ssh[ssh.index("-i") + 1])
            observed.append(key)
            self.assertEqual(stat.S_IMODE(key.stat().st_mode), 0o600)
            self.assertIn("StrictHostKeyChecking=yes", ssh)
            self.assertIn("HostKeyAlgorithms=ssh-ed25519", ssh)
            raise OSError("Simulated transport failure; no network request")

        with patch.dict(os.environ, {"DOCS_PAGES_DEPLOY_KEY": "not-a-real-deployment-key"}), \
                patch.object(self.pages.subprocess, "run", side_effect=fail_transport), \
                self.assertRaises(OSError):
            self.pages.publish(self.site, SOURCE, HEAD, 34)
        self.assertEqual(len(observed), 1)
        self.assertFalse(observed[0].exists())

    def test_marker_transport_uses_fixed_https_and_does_not_follow_redirects(self):
        with patch.object(self.pages.http.client, "HTTPSConnection") as connection:
            connection.return_value.getresponse.return_value.status = 302
            with self.assertRaises(OSError):
                self.pages.fetch_marker(SOURCE)
        connection.assert_called_once_with("johnneerdael.github.io", timeout=10)
        self.assertEqual(connection.return_value.request.call_args.args[:2],
                         ("GET", f"/audio-tag-guide/publication-source.json?source={SOURCE}"))
        connection.return_value.close.assert_called_once()


if __name__ == "__main__":
    unittest.main()
