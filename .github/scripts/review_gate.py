#!/usr/bin/env python3
"""Trusted metadata controller: authorize reviewed PR builds and main releases.

Adapted from ProjectM-TV's review_gate.py; execute only from trusted main.
"""
import argparse
from datetime import datetime, timedelta, timezone
import json
import os
from pathlib import Path
import re
import sys
import subprocess

from codex_review import GitHub as BaseGitHub, connector, timestamp
from release_notes import validate_release_notes

CONTEXT = 'Reviewed PR builds'


class GitHub(BaseGitHub):
    def post(self, path, data):
        return self.request(f'repos/{self.repo}/{path}', data)

    def runs(self, workflow):
        try:
            pages = self.request(f'repos/{self.repo}/actions/workflows/{workflow}/runs?event=workflow_dispatch&per_page=100', paginate=True)
        except subprocess.CalledProcessError as error:
            # A just-landed workflow can have no registered run listing yet.
            if 'HTTP 404' in (error.stderr or ''):
                return []
            raise
        return [run for page in pages for run in page['workflow_runs'] if run['head_branch'] == 'main']


def shas(*values):
    if any(not re.fullmatch(r'[0-9a-f]{40}', value or '') for value in values):
        raise ValueError('Full commit SHAs are required')


def check_notes(pr):
    try:
        validate_release_notes(pr.get('body') or '')
    except ValueError as error:
        raise RuntimeError(str(error)) from error


def check_pr(api, number, source=None):
    pr, (ready, reason) = api.snapshot(number)
    if not ready:
        raise RuntimeError(reason)
    check_notes(pr)
    if pr.get('mergeable') is not True:
        raise RuntimeError('PR is not mergeable')
    head, base, merge = pr['head']['sha'], api.get('git/ref/heads/main')['object']['sha'], pr.get('merge_commit_sha')
    shas(head, base, merge)
    if source is not None and tuple(source) != (head, base, merge):
        raise RuntimeError('Dispatch revision changed; fresh review/validation required')
    parents = api.get(f'git/commits/{merge}')['parents']
    if [item['sha'] for item in parents] != [base, head]:
        raise RuntimeError('Test merge does not bind current main and reviewed head')
    if api.get('git/ref/heads/main')['object']['sha'] != base:
        raise RuntimeError('Main changed while verifying the test merge')
    pr['base']['sha'] = base
    return pr


def title(pr):
    return f"Reviewed PR #{pr['number']} @ {pr['head']['sha']} + {pr['base']['sha']}"


def reconcile(api, number, retry=False):
    raw, (eligible, reason) = api.snapshot(number)
    api.status(raw['head']['sha'], 'success' if eligible else 'pending', reason, context='Codex review')
    try:
        check_notes(raw)
        notes_ready, notes_reason = True, 'Release notes describe a concrete change'
    except RuntimeError as error:
        notes_ready, notes_reason = False, str(error)
    api.status(raw['head']['sha'], 'success' if notes_ready else 'failure', notes_reason, context='Release notes')
    if not notes_ready:
        reason = notes_reason
    runs = [run for run in api.runs('reviewed-pr.yml') if run['display_title'].startswith(f'Reviewed PR #{number} @ ')]
    try:
        pr = check_pr(api, number) if eligible and notes_ready else None
    except RuntimeError as error:
        pr, reason = None, str(error)
    for run in runs:
        if run['status'] != 'completed' and (pr is None or run['display_title'] != title(pr)):
            api.post(f"actions/runs/{run['id']}/cancel", {})
    if pr is None:
        if raw['state'] == 'open':
            api.status(raw['head']['sha'], 'pending', reason, context=CONTEXT)
        return
    head, base, merge = pr['head']['sha'], pr['base']['sha'], pr['merge_commit_sha']
    same = [run for run in runs if run['display_title'] == title(pr)]
    if any(run['status'] != 'completed' for run in same):
        return
    previous = next((item for item in api.pages(f'commits/{head}/statuses?per_page=100') if item['context'] == CONTEXT), None)
    if previous and previous['state'] == 'success' and previous['description'] == f'Passed {base} {merge}':
        return
    if same and not retry and max(same, key=lambda run: run['id']).get('conclusion') not in {'success', 'cancelled', 'skipped'}:
        api.status(head, 'failure', 'Validation failed; request an explicit retry', context=CONTEXT)
        return
    reservation = f'Queued {base} {merge}'
    if previous and previous['state'] == 'pending' and previous['description'] == reservation:
        if datetime.now(timezone.utc) - timestamp(previous['created_at']) < timedelta(minutes=20):
            return
    api.status(head, 'pending', reservation, context=CONTEXT)
    api.post('actions/workflows/reviewed-pr.yml/dispatches', dict(ref='main', inputs=dict(
        pr_number=str(number), head_sha=head, base_sha=base, merge_sha=merge)))


def invalidate_comment(api, event):
    """Withdraw stale approval for review comments; never authorize or dispatch."""
    issue = event.get('issue', {})
    if 'pull_request' not in issue:
        return
    number = issue['number']
    pr = api.get(f'pulls/{number}')
    if pr['state'] != 'open' or pr['base']['ref'] != 'main':
        return
    comment = event['comment']
    trusted = (comment['user']['login'] == pr['user']['login']
               or comment.get('author_association') in {'OWNER', 'MEMBER', 'COLLABORATOR'})
    bodies = [comment.get('body') or '', event.get('changes', {}).get('body', {}).get('from') or '']
    request = trusted and any(re.fullmatch(r'@codex\s+review', body.strip(), re.I) for body in bodies)
    if not request and not connector(comment):
        return
    try:
        pr, (eligible, reason) = api.snapshot(number)
    except Exception:
        # A failed metadata read cannot retain an earlier green approval.
        eligible, reason = False, 'Cannot verify review; application execution denied'
    if not eligible:
        api.status(pr['head']['sha'], 'pending', reason, context='Codex review')
        api.status(pr['head']['sha'], 'pending', reason, context=CONTEXT)


def finish(api, number, source, preflight, result, url):
    head, base, merge = source
    try:
        check_pr(api, number, source)
    except RuntimeError:
        api.status(head, 'pending', 'Review or revision changed; fresh validation required', context=CONTEXT, url=url)
        return
    if preflight != 'success':
        state, reason = 'pending', 'Preflight did not authorize application execution'
    elif result != 'success':
        state, reason = 'failure', 'Required builds failed, were cancelled or skipped'
    else:
        state, reason = 'success', f'Passed {base} {merge}'
    api.status(head, state, reason, context=CONTEXT, url=url)


def check_release(api, number, head, source):
    shas(head, source)
    pr, (ready, reason) = api.snapshot(number, merged=True)
    if (not ready or not pr.get('merged') or pr['state'] != 'closed' or pr['base']['ref'] != 'main'
            or pr['head']['sha'] != head or pr.get('merge_commit_sha') != source):
        raise RuntimeError(f'Main source has no approved merged PR: {reason}')
    check_notes(pr)
    proof = next((item for item in api.pages(f'commits/{head}/statuses?per_page=100') if item['context'] == CONTEXT), None)
    match = re.fullmatch(r'Passed ([0-9a-f]{40}) ([0-9a-f]{40})', proof.get('description', '')) if proof and proof['state'] == 'success' else None
    if not match:
        raise RuntimeError('Merged source has no successful reviewed-build proof')
    if proof.get('creator', {}).get('login') != 'github-actions[bot]' or proof.get('creator', {}).get('type') != 'Bot':
        raise RuntimeError('Build proof was not published by the trusted Actions identity')
    base, tested = match.groups()
    run_url = re.fullmatch(rf'https://github\.com/{re.escape(api.repo)}/actions/runs/([0-9]+)', proof.get('target_url', ''))
    if not run_url:
        raise RuntimeError('Build proof is not linked to a trusted run')
    run = api.get(f'actions/runs/{run_url[1]}')
    if (run.get('path', '').split('@')[0] != '.github/workflows/reviewed-pr.yml'
            or run.get('event') != 'workflow_dispatch' or run.get('head_branch') != 'main'
            or run.get('head_sha') != base
            or run.get('display_title') != f'Reviewed PR #{number} @ {head} + {base}'
            or run.get('status') != 'completed' or run.get('conclusion') != 'success'):
        raise RuntimeError('Reviewed validation did not finish successfully on main')
    merged, validated = api.get(f'git/commits/{source}'), api.get(f'git/commits/{tested}')
    if ([item['sha'] for item in merged['parents']] != [base, head]
            or [item['sha'] for item in validated['parents']] != [base, head]
            or merged['tree']['sha'] != validated['tree']['sha']):
        raise RuntimeError('Merged source differs from the validated reviewed tree')
    if api.get(f'compare/{source}...main')['status'] not in {'ahead', 'identical'}:
        raise RuntimeError('Release source is not on main')
    return pr


def reconcile_main(api, source):
    shas(source)
    candidates = [pr for pr in api.pages(f'commits/{source}/pulls?per_page=100')
                  if pr.get('merged_at') and pr['base']['ref'] == 'main' and pr.get('merge_commit_sha') == source]
    if len(candidates) != 1:
        print(f'{source}: no unique merged PR; no release authorized')
        return
    pr = candidates[0]
    check_release(api, pr['number'], pr['head']['sha'], source)
    if any(run['display_title'] == f'Reviewed release {source}' for run in api.runs('reviewed-release.yml')):
        return
    api.post('actions/workflows/reviewed-release.yml/dispatches', dict(ref='main', inputs=dict(
        pr_number=str(pr['number']), head_sha=pr['head']['sha'], source_sha=source)))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['reconcile', 'invalidate-comment', 'preflight', 'verify', 'finish', 'main', 'release-check'])
    parser.add_argument('--pr', type=int, default=0)
    parser.add_argument('--retry', action='store_true')
    args = parser.parse_args()
    api = GitHub(os.environ['GITHUB_REPOSITORY'])
    if args.mode == 'invalidate-comment':
        invalidate_comment(api, json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text()))
    elif args.mode == 'reconcile':
        numbers = [args.pr] if args.pr else [pr['number'] for pr in api.pages('pulls?state=open&base=main&per_page=100')]
        errors = []
        for number in numbers:
            try:
                reconcile(api, number, args.retry)
            except Exception as error:
                # API failures must invalidate an old success and cancel active PR builds.
                pr = api.get(f'pulls/{number}')
                api.status(pr['head']['sha'], 'pending', 'Cannot verify review; application execution denied', context=CONTEXT)
                api.status(pr['head']['sha'], 'pending', 'Cannot verify Codex review', context='Codex review')
                api.status(pr['head']['sha'], 'pending', 'Cannot verify release notes', context='Release notes')
                for run in api.runs('reviewed-pr.yml'):
                    if run['status'] != 'completed' and run['display_title'].startswith(f'Reviewed PR #{number} @ '):
                        api.post(f"actions/runs/{run['id']}/cancel", {})
                errors.append(str(error))
        if errors:
            raise RuntimeError('; '.join(errors))
    elif args.mode == 'main':
        reconcile_main(api, os.environ['SOURCE_SHA'])
    elif args.mode == 'release-check':
        check_release(api, args.pr, os.environ['PR_HEAD'], os.environ['SOURCE_SHA'])
    else:
        source = tuple(os.environ[key] for key in ['PR_HEAD', 'PR_BASE', 'PR_MERGE'])
        shas(*source)
        if args.mode in {'preflight', 'verify'}:
            check_pr(api, args.pr, source)
            if args.mode == 'preflight':
                api.status(source[0], 'pending', 'Authorized; reviewed application validation starting', context=CONTEXT)
            with Path(os.environ['GITHUB_OUTPUT']).open('a') as output:
                output.write(f'merge_sha={source[2]}\n')
        else:
            url = f"https://github.com/{api.repo}/actions/runs/{os.environ['GITHUB_RUN_ID']}"
            finish(api, args.pr, source, os.environ['PREFLIGHT_RESULT'], os.environ['BUILD_RESULT'], url)


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        print(error, file=sys.stderr)
        sys.exit(1)
