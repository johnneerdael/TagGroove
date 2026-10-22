#!/usr/bin/env python3
"""Reconcile a required Codex status using trusted default-branch code.

Adapted from ProjectM-TV's review_gate.py. Never execute PR code here.
"""
import argparse
from datetime import datetime
import json
import os
import re
import subprocess
import sys
import time


CONTEXT = "Codex review"
CODEX = "chatgpt-codex-connector[bot]"


def timestamp(value):
    # Summary timestamps have microseconds; GitHub reactions/reviews have seconds.
    return datetime.fromisoformat(value.replace("Z", "+00:00")).replace(microsecond=0) if value else None


def connector(record):
    return record["user"]["login"] == CODEX and record["user"].get("type") == "Bot"


def code_review_request(body):
    """Recognize the code-review command, including trailing instructions."""
    return bool(re.match(r'@codex\s+review(?:\s|$)', (body or '').strip(), re.I))


def reviewed_commit_matches(record, written, head):
    if not re.fullmatch(r"[0-9a-f]{7,40}", written):
        return False
    resolved = record.get("_reviewed_commits", {}).get(written, written if len(written) == 40 else None)
    return isinstance(resolved, str) and resolved == head and resolved.startswith(written)


def eligibility(pr, reviews, comments, threads, reactions=(), inline=()):
    if pr["state"] != "open" or pr["draft"] or pr["base"]["ref"] != "main":
        return False, "Waiting for an open, ready PR targeting main"
    if pr.get("requested_reviewers") or pr.get("requested_teams"):
        return False, "Waiting for requested reviews"
    if any(not thread["isResolved"] for thread in threads):
        return False, "Waiting for all review threads to be resolved"
    decisions = {}
    for review in reviews:
        if review["state"] == "PENDING":
            return False, "Waiting for a pending review"
        if review["state"] in {"APPROVED", "CHANGES_REQUESTED", "DISMISSED"}:
            decisions[review["user"]["login"]] = review["state"]
    if "CHANGES_REQUESTED" in decisions.values():
        return False, "Waiting for requested changes to be accepted"

    head = pr["head"]["sha"]
    completions, clean_reviews, clean_receipts, findings, requests = [], [], [], [], []
    finding_reviews = {item["pull_request_review_id"] for item in inline
                       if connector(item) and not item.get("in_reply_to_id")}
    for item in inline:
        if connector(item) and not item.get("in_reply_to_id"):
            findings.append(timestamp(item["created_at"]))
    for review in reviews:
        if not connector(review) or review.get("commit_id") != head or review["state"] == "DISMISSED":
            continue
        body = review.get("body") or ""
        if (not re.search(r"\bCodex Review\b", body) or re.search(r"Codex Security Review", body, re.I)
                or not re.fullmatch(r"[0-9a-f]{40}", review.get("commit_id") or "")):
            continue
        marker = re.search(r"\*\*Reviewed commit:\*\*\s*`([0-9a-f]{7,40})`", body)
        if marker and not head.startswith(marker[1]):
            continue
        when = timestamp(review.get("submitted_at"))
        if not when:
            continue
        if review.get("id") in finding_reviews or re.search(r"\bP[0-3]\b|\b[1-9]\d* findings?\b", body):
            findings.append(when)
        elif review["state"] in {"APPROVED", "COMMENTED"}:
            completions.append(when)
            clean_reviews.append(when)

    for comment in comments:
        body = (comment.get("body") or "").strip()
        trusted = (comment["user"]["login"] == pr["user"]["login"]
                   or comment.get("author_association") in {"OWNER", "MEMBER", "COLLABORATOR"})
        if trusted and code_review_request(body):
            requests.append(timestamp(comment.get("updated_at") or comment.get("created_at")))
        if not connector(comment):
            continue
        if "<!-- codex-pull-request-review-summary -->" in body:
            structured = re.search(r"<!-- codex-review-state:v1 (.*?) -->", body, re.S)
            clean_state = False
            if structured:
                state = json.loads(structured[1]).get("code_review", {})
                if state.get("commit_sha") != head:
                    continue
                if state.get("status") != "completed":
                    return False, "Waiting for Codex code review to finish"
                if state.get("has_any_findings") is not False or state.get("has_unresolved_findings") is not False:
                    return False, "Waiting for a new Codex code review with no findings"
                clean_state = True
            for row in body.splitlines():
                if "**Code Review**" not in row and "**Security Review**" not in row:
                    continue
                commit = re.search(r"`([0-9a-f]{7,40})`", row)
                if commit and not reviewed_commit_matches(comment, commit[1], head):
                    continue
                if "**Completed**" not in row:
                    return False, "Waiting for Codex review to finish"
                date = re.search(r'datetime="([^"]+)"', row)
                if commit and date and "**Code Review**" in row:
                    when = timestamp(date[1])
                    completions.append(when)
                    if clean_state:
                        clean_reviews.append(when)
        else:
            marker = re.search(r"\*\*Reviewed commit:\*\*\s*`([0-9a-f]{7,40})`", body)
            if (marker and reviewed_commit_matches(comment, marker[1], head)
                    and re.search(r"\bCodex Review\b", body)
                    and re.search(r"\bno findings\b|didn['’]t find any (?:major )?issues", body, re.I)):
                try:
                    when = timestamp(comment.get("created_at"))
                except (ValueError, TypeError):
                    return False, "Waiting for a valid Codex review receipt timestamp"
                if when and when.tzinfo is None:
                    return False, "Waiting for a valid Codex review receipt timestamp"
                if when:
                    completions.append(when)
                    clean_reviews.append(when)
                    if ("<!-- codex-review-receipt:v1 -->" in body
                            and not re.search(r"Codex Security Review", body, re.I)):
                        clean_receipts.append(when)

    completions = [value for value in completions if value]
    requests = [value for value in requests if value]
    findings = [value for value in findings if value]
    if not completions:
        return False, "Waiting for a Codex code review of the current head"
    completed = max(completions)
    if requests and completed <= max(requests):
        return False, "Waiting for the requested Codex code review"
    if findings and (not clean_reviews or max(clean_reviews) <= max(findings)):
        return False, "Waiting for a new clean Codex code review after findings"
    # GitHub returns an existing reaction when a connector repeats +1; created_at
    # then predates its latest review. Only an explicit clean receipt for this
    # exact head, at least as recent as every completion, can supply freshness.
    receipt_approval = bool(clean_receipts and max(clean_receipts) >= completed)
    # Reactions expose this reserved connector login as type=User on GitHub.
    approved = any(item["user"]["login"] == CODEX and item.get("content") == "+1"
                   and timestamp(item.get("created_at")) is not None
                   and (timestamp(item["created_at"]) >= completed or receipt_approval) for item in reactions)
    if not approved:
        return False, "Waiting for the connector's clean-review approval"
    return True, "Clean Codex code review of the current head; no open findings"


class GitHub:
    def __init__(self, repo):
        if not re.fullmatch(r"[\w.-]+/[\w.-]+", repo):
            raise ValueError("Invalid GitHub repository")
        self.repo = repo

    def request(self, endpoint, data=None, paginate=False):
        command = ["gh", "api", endpoint]
        if data is not None:
            command += ["--method", "POST", "--input", "-"]
        if paginate:
            command += ["--paginate", "--slurp"]
        options = dict(input=json.dumps(data) if data is not None else None,
                       text=True, capture_output=True, check=True)
        if data is None:
            options['timeout'] = 120
        # Only repeat idempotent reads. A failed write can already have taken
        # effect, and every authorization check still requires valid metadata.
        for attempt in range(3 if data is None else 1):
            try:
                result = subprocess.run(command, **options)
                return json.loads(result.stdout) if result.stdout.strip() else None
            except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
                if data is not None:
                    raise
                if isinstance(error, subprocess.TimeoutExpired):
                    detail = 'timed out'
                else:
                    status = re.search(r'HTTP (\d{3})', str(error.stderr or ''))
                    detail = 'HTTP '+status[1] if status else 'CLI exit '+str(error.returncode)
                # Never echo response bodies, credentials or private diagnostics.
                print(f'GitHub API read attempt {attempt+1} failed ({detail})', file=sys.stderr, flush=True)
                if attempt == 2:
                    raise
                time.sleep(5*(attempt+1))

    def get(self, path):
        return self.request(f"repos/{self.repo}/{path}")

    def pages(self, path):
        return [item for page in self.request(f"repos/{self.repo}/{path}", paginate=True) for item in page]

    def resolve_review_commit(self, written):
        if not re.fullmatch(r"[0-9a-f]{7,40}", written):
            return None
        if len(written) == 40:
            return written
        for namespace in ("heads", "tags"):
            refs = self.get(f"git/matching-refs/{namespace}/{written}")
            if any(ref["ref"] == f"refs/{namespace}/{written}" for ref in refs):
                return None
        resolved = self.get(f"commits/{written}")["sha"]
        if not re.fullmatch(r"[0-9a-f]{40}", resolved) or not resolved.startswith(written):
            raise RuntimeError("GitHub did not resolve the reviewed commit abbreviation")
        return resolved

    def threads(self, number):
        owner, name = self.repo.split("/")
        result, cursor = [], None
        while True:
            data = self.request("graphql", dict(query="""
              query($owner:String!, $name:String!, $number:Int!, $cursor:String) {
                repository(owner:$owner,name:$name) {
                  pullRequest(number:$number) {
                    reviewThreads(first:100,after:$cursor) {
                      nodes { isResolved }
                      pageInfo { hasNextPage endCursor }
                    }
                  }
                }
              }""", variables=dict(owner=owner, name=name, number=number, cursor=cursor)))
            if data.get("errors"):
                raise RuntimeError(f"Cannot read review threads: {data['errors']}")
            connection = data["data"]["repository"]["pullRequest"]["reviewThreads"]
            result.extend(connection["nodes"])
            if not connection["pageInfo"]["hasNextPage"]:
                return result
            cursor = connection["pageInfo"]["endCursor"]

    def snapshot(self, number, merged=False):
        pr = self.get(f"pulls/{number}")
        reviews = self.pages(f"pulls/{number}/reviews?per_page=100")
        comments = self.pages(f"issues/{number}/comments?per_page=100")
        reactions = self.pages(f"issues/{number}/reactions?per_page=100")
        inline = self.pages(f"pulls/{number}/comments?per_page=100")
        resolved = {}
        for comment in comments:
            if not connector(comment):
                continue
            for written in re.findall(r"`([0-9a-f]{7,40})`", comment.get("body") or ""):
                if pr["head"]["sha"].startswith(written) and written not in resolved:
                    resolved[written] = self.resolve_review_commit(written)
            comment["_reviewed_commits"] = resolved
        threads = self.threads(number)
        current = self.get(f"pulls/{number}")
        if (current["head"]["sha"], current["state"], current["draft"], current["base"]["ref"], current.get("body"),
                current.get("requested_reviewers"), current.get("requested_teams")) != (
                pr["head"]["sha"], pr["state"], pr["draft"], pr["base"]["ref"], pr.get("body"),
                pr.get("requested_reviewers"), pr.get("requested_teams")):
            raise RuntimeError("PR changed while reading review state")
        review_pr = dict(pr)
        if merged and pr.get("merged") and pr["state"] == "closed" and pr["base"]["ref"] == "main":
            review_pr["state"] = "open"
        return pr, eligibility(review_pr, reviews, comments, threads, reactions, inline)

    def status(self, head, state, description, context=CONTEXT, url=None):
        previous = next((item for item in self.pages(f"commits/{head}/statuses?per_page=100")
                         if item["context"] == context), None)
        if previous and (previous["state"], previous["description"]) == (state, description):
            return
        payload = dict(state=state, context=context, description=description[:140])
        if url:
            payload["target_url"] = url
        self.request(f"repos/{self.repo}/statuses/{head}", payload)
