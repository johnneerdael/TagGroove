"""Require an authenticated, clean Codex code review of the final PR head."""
import importlib.util
from copy import deepcopy
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "codex_review.py"
sys.path.insert(0, str(SCRIPT.parent))
HEAD, OLD = "a" * 40, "b" * 40
BOT = "chatgpt-codex-connector[bot]"


def load_gate():
    if not SCRIPT.exists():
        raise AssertionError("Codex review gate is not implemented")
    spec = importlib.util.spec_from_file_location("codex_review", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def pr():
    return dict(number=1, state="open", draft=False, base=dict(ref="main"),
                head=dict(sha=HEAD), user=dict(login="author"), requested_reviewers=[], requested_teams=[],
                body="## Release notes\n- Retain cover values when saving tags.")


def review(sha=HEAD, login=BOT, body=None, timestamp="2026-10-15T10:00:00Z", ident=1):
    return dict(id=ident, state="COMMENTED", commit_id=sha, submitted_at=timestamp,
                user=dict(login=login, type="Bot"), body=body if body is not None else f"### Codex Review\n**Reviewed commit:** `{sha[:10]}`")


def summary(state="Completed", sha=HEAD, kind="Code", timestamp="2026-10-15T10:00:00Z"):
    return dict(user=dict(login=BOT, type="Bot"), body=(
        '<!-- codex-pull-request-review-summary -->\n'
        f'| **{kind} Review** | **{state}** <relative-time datetime="{timestamp}">now</relative-time> | `{sha}` |'))


def thumb(timestamp="2026-10-15T10:00:02Z", login=BOT, content="+1"):
    return dict(user=dict(login=login, type="User"), content=content, created_at=timestamp)


def state_summary(findings=False, unresolved=False, sha=HEAD, status="completed"):
    item = summary(sha=sha, timestamp="2026-10-15T11:00:00Z")
    state = dict(code_review=dict(commit_sha=sha, status=status, has_any_findings=findings,
                                  has_unresolved_findings=unresolved))
    item["body"] = '<!-- codex-review-state:v1 ' + json.dumps(state) + ' -->\n' + item["body"]
    return item


class EligibilityTests(unittest.TestCase):
    def setUp(self):
        self.gate = load_gate()

    def check(self, reviews=None, comments=(), threads=(), reactions=None, inline=(), pull=None):
        return self.gate.eligibility(pull or pr(), [review()] if reviews is None else reviews,
                                     comments, threads, [thumb()] if reactions is None else reactions, inline)[0]

    def test_clean_native_review_and_recent_connector_thumb_qualify(self):
        self.assertTrue(self.check())

    def test_current_full_sha_summary_and_thumb_qualify(self):
        self.assertTrue(self.check(reviews=[], comments=[summary()]))

    def test_no_review_stale_review_or_thumb_alone_never_qualify(self):
        for reviews in ([], [review(OLD)]):
            self.assertFalse(self.check(reviews=reviews))
        self.assertFalse(self.check(reactions=[]))

    def test_human_claude_spoofed_and_security_reviews_never_qualify(self):
        for login in ("author", "reviewer", "claude[bot]", "github-actions[bot]"):
            self.assertFalse(self.check(reviews=[review(login=login)]))
        self.assertFalse(self.check(reviews=[review(body="Codex Security Review")]))
        self.assertFalse(self.check(reviews=[], comments=[summary(kind="Security")]))
        item = review()
        item["user"]["type"] = "User"
        self.assertFalse(self.check(reviews=[item]))

    def test_short_summary_must_resolve_to_exact_head(self):
        item = summary(sha=HEAD[:7])
        self.assertFalse(self.check(reviews=[], comments=[item]))
        item["_reviewed_commits"] = {HEAD[:7]: OLD}
        self.assertFalse(self.check(reviews=[], comments=[item]))
        item["_reviewed_commits"][HEAD[:7]] = HEAD
        self.assertTrue(self.check(reviews=[], comments=[item]))

    def test_conflicting_marker_empty_reply_and_dismissed_review_do_not_qualify(self):
        for item in (review(body=""), review(body=f"Codex Review\n**Reviewed commit:** `{OLD[:10]}`")):
            item["body"] = item["body"] if item["body"] else ""
            self.assertFalse(self.check(reviews=[item]))
        item = review()
        item["state"] = "DISMISSED"
        self.assertFalse(self.check(reviews=[item]))

    def test_only_fresh_connector_plus_one_qualifies(self):
        for item in (thumb(timestamp="2026-10-15T09:00:00Z"), thumb(login="author"), thumb(content="eyes")):
            self.assertFalse(self.check(reactions=[item]))

    def test_connector_millisecond_completion_compares_at_reaction_api_precision(self):
        item = summary(timestamp="2026-10-15T10:00:00.635323Z")
        self.assertTrue(self.check(reviews=[], comments=[item], reactions=[thumb("2026-10-15T10:00:00Z")]))

    def test_native_clean_receipt_phrase_clears_historical_findings(self):
        finding = dict(id=99, user=dict(login=BOT, type="Bot"), commit_id=OLD,
                       pull_request_review_id=1, created_at="2026-10-15T09:00:00Z")
        receipt = dict(user=dict(login=BOT, type="Bot"), created_at="2026-10-15T10:00:00Z",
                       body=f"<!-- codex-review-receipt:v1 -->\nCodex Review: Didn't find any major issues.\n**Reviewed commit:** `{HEAD}`")
        self.assertTrue(self.check(reviews=[], inline=[finding], comments=[receipt]))

    def test_running_code_or_security_review_blocks(self):
        for kind in ("Code", "Security"):
            self.assertFalse(self.check(comments=[summary(state="Running", kind=kind)]))

    def test_unresolved_even_outdated_thread_blocks(self):
        self.assertFalse(self.check(threads=[dict(isResolved=False)]))

    def test_resolving_finding_requires_a_new_clean_review(self):
        finding = dict(id=99, user=dict(login=BOT, type="Bot"), commit_id=HEAD,
                       pull_request_review_id=1, created_at="2026-10-15T10:00:00Z", body="**P2** Fix write loss")
        self.assertFalse(self.check(inline=[finding], threads=[dict(isResolved=True)]))
        self.assertFalse(self.check(reviews=[], inline=[finding], comments=[summary(timestamp="2026-10-15T10:00:01Z")]))
        clean = review(timestamp="2026-10-15T11:00:00Z", ident=2)
        self.assertTrue(self.check(reviews=[review(), clean], inline=[finding], reactions=[thumb("2026-10-15T11:00:02Z")]))

    def test_current_structured_clean_summary_can_clear_old_findings_without_native_review(self):
        finding = dict(id=99, user=dict(login=BOT, type="Bot"), commit_id=OLD,
                       pull_request_review_id=1, created_at="2026-10-15T10:00:00Z")
        self.assertTrue(self.check(reviews=[], inline=[finding], comments=[state_summary()], reactions=[thumb("2026-10-15T11:00:02Z")]))

    def test_structured_findings_unknown_flags_or_running_never_approve(self):
        for item in (state_summary(findings=True), state_summary(unresolved=True), state_summary(findings=None),
                     state_summary(unresolved=None), state_summary(status="running"), state_summary(sha=OLD)):
            self.assertFalse(self.check(reviews=[], comments=[item], reactions=[thumb("2026-10-15T11:00:02Z")]))

    def test_later_changes_requested_pending_or_requested_reviews_block(self):
        for state in ("CHANGES_REQUESTED", "PENDING"):
            item = review(login="human")
            item["state"] = state
            self.assertFalse(self.check(reviews=[review(), item]))
        for key in ("requested_reviewers", "requested_teams"):
            pull = pr()
            pull[key] = [dict(login="reviewer")]
            self.assertFalse(self.check(pull=pull))

    def test_new_or_edited_code_request_needs_a_later_completion(self):
        request = dict(user=dict(login="author"), body="@codex review", created_at="2026-10-15T09:00:00Z")
        self.assertTrue(self.check(comments=[request]))
        request["updated_at"] = "2026-10-15T10:00:00Z"
        self.assertFalse(self.check(comments=[request]))

    def test_draft_closed_and_other_base_block(self):
        for key, value in (("draft", True), ("state", "closed"), ("base", dict(ref="other"))):
            pull = pr()
            pull[key] = value
            self.assertFalse(self.check(pull=pull))


class SnapshotTests(unittest.TestCase):
    def test_review_request_added_during_read_cannot_authorize_stale_clean_snapshot(self):
        gate = load_gate()
        for key, request in (("requested_reviewers", dict(login="reviewer")),
                             ("requested_teams", dict(slug="maintainers"))):
            with self.subTest(key=key):
                initial = pr()
                current = deepcopy(initial)
                current[key] = [request]
                api = gate.GitHub("owner/repo")
                with patch.object(api, "get", side_effect=[initial, current]), \
                        patch.object(api, "pages", side_effect=[[review()], [], [thumb()], []]), \
                        patch.object(api, "threads", return_value=[]):
                    with self.assertRaisesRegex(RuntimeError, "PR changed"):
                        api.snapshot(1)

    def test_unchanged_clean_snapshot_still_authorizes(self):
        gate = load_gate()
        api = gate.GitHub("owner/repo")
        with patch.object(api, "get", side_effect=[pr(), pr()]), \
                patch.object(api, "pages", side_effect=[[review()], [], [thumb()], []]), \
                patch.object(api, "threads", return_value=[]):
            self.assertTrue(api.snapshot(1)[1][0])
