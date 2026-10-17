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


def receipt(sha=HEAD, timestamp="2026-10-15T10:00:00Z"):
    return dict(user=dict(login=BOT, type="Bot"), created_at=timestamp, body=(
        "<!-- codex-review-receipt:v1 -->\n"
        "Codex Review: Didn't find any major issues. Another round soon, please!\n"
        f"**Reviewed commit:** `{sha}`"))


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

    def test_retained_connector_thumb_with_current_clean_receipt_qualifies(self):
        self.assertTrue(self.check(reviews=[], comments=[receipt()],
                                   reactions=[thumb("2026-10-15T09:00:00Z")]))
        item = receipt(sha=HEAD[:10])
        item["_reviewed_commits"] = {HEAD[:10]: HEAD}
        self.assertTrue(self.check(reviews=[], comments=[item],
                                   reactions=[thumb("2026-10-15T09:00:00Z")]))

    def test_retained_thumb_requires_marked_authenticated_clean_code_receipt(self):
        missing_marker = receipt()
        missing_marker["body"] = missing_marker["body"].replace("<!-- codex-review-receipt:v1 -->", "")
        security = receipt()
        security["body"] += "\nCodex Security Review: no findings"
        fake_connector = receipt()
        fake_connector["user"]["type"] = "User"
        human = receipt()
        human["user"]["login"] = "author"
        generic = receipt()
        generic["body"] = generic["body"].replace("Didn't find any major issues.", "Completed.")
        for item in (missing_marker, security, fake_connector, human, generic, summary(), state_summary()):
            with self.subTest(body=item["body"], user=item["user"]):
                self.assertFalse(self.check(reviews=[], comments=[item],
                                            reactions=[thumb("2026-10-15T09:00:00Z")]))
        self.assertFalse(self.check(reactions=[thumb("2026-10-15T09:00:00Z")]))

    def test_retained_thumb_requires_receipt_resolved_to_current_head(self):
        unresolved = receipt(sha=HEAD[:10])
        conflicting = deepcopy(unresolved)
        conflicting["_reviewed_commits"] = {HEAD[:10]: OLD}
        for item in (receipt(sha=OLD), unresolved, conflicting):
            with self.subTest(item=item):
                self.assertFalse(self.check(reviews=[], comments=[item],
                                            reactions=[thumb("2026-10-15T09:00:00Z")]))
        pushed = pr()
        pushed["head"]["sha"] = OLD
        self.assertFalse(self.check(reviews=[], comments=[receipt()], pull=pushed,
                                    reactions=[thumb("2026-10-15T09:00:00Z")]))

    def test_receipt_still_requires_connector_plus_one(self):
        for reactions in ([], [thumb(login="author")], [thumb(content="eyes")], [thumb(timestamp=None)]):
            with self.subTest(reactions=reactions):
                self.assertFalse(self.check(reviews=[], comments=[receipt()], reactions=reactions))

    def test_receipt_requires_valid_creation_timestamp(self):
        for date in (None, "not-a-date", "2026-10-15T10:00:00"):
            with self.subTest(date=date):
                self.assertFalse(self.check(reviews=[], comments=[receipt(timestamp=date)],
                                            reactions=[thumb("2026-10-15T09:00:00Z")]))

    def test_retained_thumb_cannot_borrow_receipt_before_latest_completion(self):
        for later in (summary(timestamp="2026-10-15T11:00:00Z"), state_summary()):
            with self.subTest(later=later):
                self.assertFalse(self.check(reviews=[], comments=[receipt(), later],
                                            reactions=[thumb("2026-10-15T09:00:00Z")]))
        self.assertFalse(self.check(reviews=[review(timestamp="2026-10-15T11:00:00Z")],
                                    comments=[receipt()], reactions=[thumb("2026-10-15T09:00:00Z")]))
        self.assertTrue(self.check(reviews=[], comments=[receipt(), summary()],
                                   reactions=[thumb("2026-10-15T09:00:00Z")]))

    def test_retained_thumb_does_not_override_new_requests_or_findings(self):
        for date in ("2026-10-15T10:00:00Z", "2026-10-15T11:00:00Z"):
            request = dict(user=dict(login="author"), body="@codex review", created_at=date)
            finding = dict(user=dict(login=BOT, type="Bot"), pull_request_review_id=99, created_at=date)
            with self.subTest(date=date):
                self.assertFalse(self.check(reviews=[], comments=[receipt(), request],
                                            reactions=[thumb("2026-10-15T09:00:00Z")]))
                self.assertFalse(self.check(reviews=[], comments=[receipt()], inline=[finding],
                                            reactions=[thumb("2026-10-15T09:00:00Z")]))

    def test_retained_thumb_does_not_override_other_review_blockers(self):
        old_thumb = [thumb("2026-10-15T09:00:00Z")]
        self.assertFalse(self.check(reviews=[], comments=[receipt()], reactions=old_thumb,
                                    threads=[dict(isResolved=False)]))
        for kind in ("Code", "Security"):
            self.assertFalse(self.check(reviews=[], comments=[receipt(), summary(state="Running", kind=kind)],
                                        reactions=old_thumb))
        for state in ("PENDING", "CHANGES_REQUESTED"):
            pending = review(login="reviewer")
            pending["state"] = state
            self.assertFalse(self.check(reviews=[pending], comments=[receipt()], reactions=old_thumb))
        for key, value in (("draft", True), ("requested_reviewers", [dict(login="reviewer")]),
                           ("requested_teams", [dict(slug="maintainers")])):
            pull = pr()
            pull[key] = value
            self.assertFalse(self.check(reviews=[], comments=[receipt()], reactions=old_thumb, pull=pull))

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
    def test_real_clean_receipt_resolves_short_sha_with_retained_issue_reaction(self):
        fixture = json.loads((SCRIPT.parent / "tests/fixtures/codex-retained-reaction.json").read_text())
        gate = load_gate()
        pull = pr()
        pull["number"] = 42
        pull["head"]["sha"] = fixture["head"]
        responses = {
            "repos/owner/repo/pulls/42": pull,
            "repos/owner/repo/pulls/42/reviews?per_page=100": [[]],
            "repos/owner/repo/issues/42/comments?per_page=100": [[fixture["receipt"]]],
            "repos/owner/repo/issues/42/reactions?per_page=100": [fixture["reactions"]],
            "repos/owner/repo/pulls/42/comments?per_page=100": [[]],
            "repos/owner/repo/git/matching-refs/heads/08233e57a5": [],
            "repos/owner/repo/git/matching-refs/tags/08233e57a5": [],
            "repos/owner/repo/commits/08233e57a5": {"sha": fixture["head"]},
            "graphql": {"data": {"repository": {"pullRequest": {"reviewThreads": {
                "nodes": [], "pageInfo": {"hasNextPage": False}}}}}},
        }
        api = gate.GitHub("owner/repo")
        with patch.object(api, "request", side_effect=lambda endpoint, *args, **kwargs: deepcopy(responses[endpoint])):
            self.assertTrue(api.snapshot(42)[1][0])
            # An ambiguous/prefix-conflicting resolution must remain blocked.
            responses["repos/owner/repo/commits/08233e57a5"] = {"sha": "08233e57a5" + "f" * 30}
            self.assertFalse(api.snapshot(42)[1][0])
            responses["repos/owner/repo/git/matching-refs/heads/08233e57a5"] = [
                {"ref": "refs/heads/08233e57a5"}]
            self.assertFalse(api.snapshot(42)[1][0])

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
