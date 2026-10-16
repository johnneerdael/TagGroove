"""Exercise static publication, redirects and the cross-repository boundary."""
import importlib.util
import json
import os
from pathlib import Path
import re
import shlex
import stat
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch


SCRIPT = Path(__file__).resolve().parents[1] / "guide_pages.py"
sys.path.insert(0, str(SCRIPT.parent))
TEMPLATE = SCRIPT.parents[1] / "guide-pages-target.yml"
SOURCE, HEAD = "a" * 40, "b" * 40


class Clock:
    def __init__(self):
        self.now = 0

    def sleep(self, seconds):
        self.now += seconds


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
        self.assertTrue(TEMPLATE.is_file(), "The reviewed target Actions control is missing")
        bare, checkout = self.root / "target.git", self.root / "checkout"
        subprocess.run(["git", "init", "--bare", "--initial-branch=main", str(bare)],
                       check=True, capture_output=True)
        subprocess.run(["git", "clone", str(bare), str(checkout)],
                       check=True, capture_output=True)
        control = checkout / ".github/workflows/static-pages.yml"
        control.parent.mkdir(parents=True)
        control.write_bytes(TEMPLATE.read_bytes())
        (checkout / "site").mkdir()
        (checkout / "site/old.html").write_text("Old publication")
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
        self.assertFalse((checkout / "site/old.html").exists())
        self.assertTrue((checkout / ".git").is_dir())
        self.assertEqual((checkout / ".github/workflows/static-pages.yml").read_bytes(), TEMPLATE.read_bytes())
        published = json.loads(self.git(bare, "show", "main:site/publication-source.json"))
        self.assertEqual(published["source_commit"], SOURCE)
        self.assertEqual(self.git(bare, "show", "main:site/.nojekyll"), "")
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
            self.assertTrue((checkout / "site/old.html").exists())

    def test_missing_or_changed_target_control_fails_before_replacing_static_files(self):
        self.prepare()
        bare, checkout = self.repository()
        before = self.git(bare, "rev-parse", "main")
        control = checkout / ".github/workflows/static-pages.yml"
        for value in [None, b"unreviewed deployment control"]:
            if value is None:
                control.unlink()
            else:
                control.write_bytes(value)
            with self.assertRaises(ValueError):
                self.pages.publish_checkout(self.site, checkout, SOURCE, HEAD, 34)
            self.assertTrue((checkout / "site/old.html").exists())
            self.assertEqual(self.git(bare, "rev-parse", "main"), before)

    def source_api(self):
        base, tested = "c" * 40, "d" * 40
        api = Mock(repo="johnneerdael/opentag")
        api.snapshot.return_value = (dict(number=34, state="closed", merged=True,
            base=dict(ref="main"), head=dict(sha=HEAD), merge_commit_sha=SOURCE,
            body="## Release notes\nInternal: Publish the reviewed documentation."), (True, "Clean"))
        api.pages.return_value = [dict(context="Reviewed PR builds", state="success",
            description=f"Passed {base} {tested}", target_url="https://github.com/johnneerdael/opentag/actions/runs/7",
            creator=dict(login="github-actions[bot]", type="Bot"))]

        def get(path):
            if path == "git/ref/heads/main":
                return dict(object=dict(sha=SOURCE))
            if path == "actions/runs/7":
                return dict(path=".github/workflows/reviewed-pr.yml", event="workflow_dispatch",
                    head_branch="main", head_sha=base, status="completed", conclusion="success",
                    display_title=f"Reviewed PR #34 @ {HEAD} + {base}")
            if path.startswith("compare/"):
                return dict(status="identical")
            return dict(parents=[dict(sha=base), dict(sha=HEAD)], tree=dict(sha="e" * 40))
        api.get.side_effect = get
        return api

    def test_source_signing_authentication_reuses_full_source_gate_and_rejects_unapproved_claims(self):
        api = self.source_api()
        self.pages.authorize_source(SOURCE, HEAD, 34, api=api)
        api.snapshot.assert_called_once_with(34, merged=True)
        api.post.assert_not_called()
        api.snapshot.return_value = (api.snapshot.return_value[0], (False, "Unapproved head"))
        with self.assertRaises(RuntimeError):
            self.pages.authorize_source(SOURCE, HEAD, 34, api=api)
        api = self.source_api()
        api.pages.return_value = []
        with self.assertRaises(RuntimeError):
            self.pages.authorize_source(SOURCE, HEAD, 34, api=api)
        api = self.source_api()
        original = api.get.side_effect
        api.get.side_effect = lambda path: dict(object=dict(sha="f" * 40)) if path == "git/ref/heads/main" else original(path)
        with self.assertRaises(RuntimeError):
            self.pages.authorize_source(SOURCE, HEAD, 34, api=api)

    def signing_key(self, name):
        key = self.root / name
        subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(key)],
                       check=True, capture_output=True)
        return key

    def test_actual_ssh_signature_binds_marker_tree_signer_namespace_and_current_main(self):
        self.prepare()
        key = self.signing_key("signer")
        wrong = self.signing_key("wrong-signer")
        allowed = self.root / "allowed-signers"
        allowed.write_text(self.pages.PRINCIPAL + " " + key.with_suffix(".pub").read_text())
        self.pages.sign_publication(self.site, key)
        _, checkout = self.repository()
        self.pages.publish_checkout(self.site, checkout, SOURCE, HEAD, 34)
        site = checkout / "site"
        with patch.object(self.pages, "SIGNERS", allowed), patch.object(self.pages, "current_source", return_value=SOURCE):
            self.pages.authorize_target(site, checkout)
            marker = (site / self.pages.MARKER).read_bytes()
            modified = json.loads(marker)
            modified["reviewed_pr"] = 35
            (site / self.pages.MARKER).write_text(json.dumps(modified))
            with self.assertRaises(subprocess.CalledProcessError):
                self.pages.authorize_target(site, checkout)
            (site / self.pages.MARKER).write_bytes(marker)
            with patch.object(self.pages, "NAMESPACE", "wrong-domain"), self.assertRaises(subprocess.CalledProcessError):
                self.pages.authorize_target(site, checkout)
            allowed.write_text(self.pages.PRINCIPAL + " " + wrong.with_suffix(".pub").read_text())
            with self.assertRaises(subprocess.CalledProcessError):
                self.pages.authorize_target(site, checkout)
            allowed.write_text(self.pages.PRINCIPAL + " " + key.with_suffix(".pub").read_text())
            with patch.object(self.pages, "current_source", return_value="f" * 40), self.assertRaises(RuntimeError):
                self.pages.authorize_target(site, checkout)
            (site / "index.html").write_text("tampered static bytes")
            with self.assertRaises(ValueError):
                self.pages.authorize_target(site, checkout)
            (site / "index.html").write_bytes((self.site / "index.html").read_bytes())
            (site / self.pages.SIGNATURE).unlink()
            with self.assertRaises(ValueError):
                self.pages.authorize_target(site, checkout)

    def test_source_rejection_immediately_before_push_keeps_remote_unchanged(self):
        self.prepare()
        bare, checkout = self.repository()
        original = self.git(bare, "rev-parse", "main")
        gate = Mock(side_effect=RuntimeError("Source approval revoked"))
        with self.assertRaises(RuntimeError):
            self.pages.publish_checkout(self.site, checkout, SOURCE, HEAD, 34, before_push=gate)
        gate.assert_called_once_with()
        self.assertEqual(self.git(bare, "rev-parse", "main"), original)

    def test_current_source_uses_only_fixed_public_git_remote_and_denies_bad_output(self):
        with patch.object(self.pages.subprocess, "run") as run:
            run.return_value.stdout = SOURCE + "\trefs/heads/main\n"
            self.assertEqual(self.pages.current_source(), SOURCE)
            command = run.call_args.args[0]
            self.assertEqual(command[-2:], ["https://github.com/johnneerdael/opentag.git", "refs/heads/main"])
            self.assertNotIn("gh", command)
            run.return_value.stdout = SOURCE + "\trefs/heads/other\n"
            with self.assertRaises(RuntimeError):
                self.pages.current_source()

    def test_publisher_authenticates_before_signing_and_again_before_actual_push(self):
        self.prepare()
        key = self.signing_key("publisher-key")
        bare, _ = self.repository()
        original = self.git(bare, "rev-parse", "main")
        real_run = subprocess.run
        events = []

        def transport(command, **kwargs):
            if "clone" in command and self.pages.TARGET in command:
                command = [str(bare) if part == self.pages.TARGET else part for part in command]
            if "push" in command:
                events.append("push")
                for secret in ("DOCS_PAGES_DEPLOY_KEY", "GH_TOKEN", "GITHUB_TOKEN"):
                    self.assertNotIn(secret, kwargs["env"])
            if "sign" in command:
                events.append("sign")
                for secret in ("DOCS_PAGES_DEPLOY_KEY", "GH_TOKEN", "GITHUB_TOKEN"):
                    self.assertNotIn(secret, kwargs["env"])
            return real_run(command, **kwargs)

        def approved(*args):
            self.assertEqual(os.environ["GH_TOKEN"], "source-only")
            events.append("authorize")

        with patch.dict(os.environ, {"DOCS_PAGES_DEPLOY_KEY": key.read_text(), "GH_TOKEN": "source-only"}), \
                patch.object(self.pages.subprocess, "run", side_effect=transport), \
                patch.object(self.pages, "authorize_source", side_effect=RuntimeError("Unapproved")), \
                self.assertRaises(RuntimeError):
            self.pages.publish(self.site, SOURCE, HEAD, 34)
        self.assertEqual(events, [])
        self.assertEqual(self.git(bare, "rev-parse", "main"), original)
        with patch.dict(os.environ, {"DOCS_PAGES_DEPLOY_KEY": key.read_text(), "GH_TOKEN": "source-only"}), \
                patch.object(self.pages.subprocess, "run", side_effect=transport), \
                patch.object(self.pages, "authorize_source", side_effect=approved):
            self.pages.publish(self.site, SOURCE, HEAD, 34)
        self.assertEqual(events, ["authorize", "sign", "authorize", "push"])
        self.assertTrue(self.git(bare, "show", "main:site/" + self.pages.SIGNATURE).startswith("-----BEGIN SSH SIGNATURE-----"))

    def test_default_wait_allows_a_nine_minute_publication_and_a_full_deadline(self):
        marker = self.prepare()
        clock = Clock()
        with patch.object(self.pages.time, "monotonic", side_effect=lambda: clock.now), \
                patch.object(self.pages.time, "sleep", side_effect=clock.sleep):
            try:
                self.pages.wait_for_publication(marker, fetch=lambda: json.dumps(marker if clock.now >= 540 else {}).encode())
            except TimeoutError:
                self.fail("Marker wait expired before a valid nine-minute Pages publication")
        self.assertGreaterEqual(clock.now, 540)
        clock = Clock()
        with patch.object(self.pages.time, "monotonic", side_effect=lambda: clock.now), \
                patch.object(self.pages.time, "sleep", side_effect=clock.sleep), self.assertRaises(TimeoutError):
            self.pages.wait_for_publication(marker, fetch=lambda: b"{}")
        self.assertGreaterEqual(clock.now, 660)

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
        clock = Clock()
        with patch.object(self.pages.time, "monotonic", side_effect=lambda: clock.now), \
                patch.object(self.pages.time, "sleep", side_effect=clock.sleep):
            self.pages.wait_for_publication(expected, fetch=lambda: next(observed), timeout=16)
        clock = Clock()
        with patch.object(self.pages.time, "monotonic", side_effect=lambda: clock.now), \
                patch.object(self.pages.time, "sleep", side_effect=clock.sleep), self.assertRaises(TimeoutError):
            self.pages.wait_for_publication(expected, fetch=lambda: json.dumps(wrong).encode(), timeout=8)

    def test_credential_is_not_required_for_prepare_or_marker_wait(self):
        with patch.dict(os.environ, {}, clear=True):
            marker = self.prepare()
            self.pages.wait_for_publication(marker, fetch=lambda: json.dumps(marker).encode(), timeout=0)

    def test_only_fixed_target_transport_receives_a_temporary_key_file_and_cleans_on_failure(self):
        self.prepare()
        observed = []

        def fail_transport(command, **kwargs):
            env = kwargs["env"]
            for name in ("DOCS_PAGES_DEPLOY_KEY", "GH_TOKEN", "GITHUB_TOKEN"):
                self.assertNotIn(name, env)
            self.assertIn("git@github.com:johnneerdael/audio-tag-guide.git", command)
            ssh = shlex.split(env["GIT_SSH_COMMAND"])
            key = Path(ssh[ssh.index("-i") + 1])
            observed.append(key)
            self.assertEqual(stat.S_IMODE(key.stat().st_mode), 0o600)
            self.assertIn("StrictHostKeyChecking=yes", ssh)
            self.assertIn("HostKeyAlgorithms=ssh-ed25519", ssh)
            raise OSError("Simulated transport failure; no network request")

        with patch.dict(os.environ, {"DOCS_PAGES_DEPLOY_KEY": "not-a-real-deployment-key", "GH_TOKEN": "source-read-token", "GITHUB_TOKEN": "source-token"}), \
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
