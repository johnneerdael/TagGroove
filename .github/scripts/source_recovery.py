#!/usr/bin/env python3
"""Authorize source-only recovery from an immutable failed release attempt.

Read metadata only; preserve the original binary artifacts and release proof.
Execute the CLI only from trusted main, independently of the old source checkout.
"""
import argparse
from datetime import datetime
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys

from review_gate import GitHub, check_release, shas

SUCCESS_JOBS = (
    'authorize', 'Validate version, tag and source commit',
    'Desktop windows-x64', 'Desktop macos-arm64', 'Desktop macos-x64',
    'Desktop linux-x64', 'Desktop linux-arm64',
    'Android arm64 signed release APK',
    'iOS arm64 simulator preview (debug, unsigned)',
    'Native UI and Android integration / authorize',
    'Native UI and Android integration / desktop (ubuntu-24.04)',
    'Native UI and Android integration / desktop (windows-latest)',
    'Native UI and Android integration / desktop (macos-latest)',
    'Native UI and Android integration / Android document integration',
)
SOURCE_JOB = 'Assemble complete corresponding source'
PUBLISH_JOB = 'Publish complete verified release'
PLATFORMS = ('windows-x64', 'macos-arm64', 'macos-x64', 'linux-x64', 'linux-arm64',
             'android-arm64', 'ios-arm64-simulator-preview')
ARTIFACT_GROUPS = (
    ('native_ids', 'native-source', PLATFORMS),
    ('linux_ids', 'linux-source', ('linux-x64', 'linux-arm64')),
    ('macos_ids', 'macos-signing', ('macos-arm64', 'macos-x64')),
    ('release_ids', 'release', PLATFORMS),
)


def positive_id(value):
    if isinstance(value, bool) or not re.fullmatch(r'[1-9][0-9]*', str(value)):
        raise ValueError('Run and attempt IDs must be positive integers')
    return int(value)


def artifact_timestamp(value):
    if not isinstance(value, str) or not re.fullmatch(r'\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z', value):
        raise ValueError('Artifact provenance requires UTC timestamps')
    return datetime.fromisoformat(value.replace('Z', '+00:00'))


def original_artifacts(api, run_id, source, run, source_job):
    """Pin artifact IDs uploaded before the original corresponding-source job."""
    started = artifact_timestamp(run.get('run_started_at'))
    cutoff = artifact_timestamp(source_job.get('started_at'))
    if cutoff < started:
        raise RuntimeError('Source job predates its release attempt')
    pages = api.request(f'repos/{api.repo}/actions/runs/{run_id}/artifacts?per_page=100', paginate=True)
    artifacts = [artifact for page in pages for artifact in page['artifacts']]
    result, selected_ids = {}, set()
    for key, prefix, platforms in ARTIFACT_GROUPS:
        ids = []
        for platform in platforms:
            matching = [artifact for artifact in artifacts if artifact.get('name') == f'{prefix}-{platform}']
            if len(matching) != 1:
                raise RuntimeError('Require one original artifact for every release platform and source binding')
            artifact = matching[0]
            identity = artifact.get('workflow_run') or {}
            created = artifact_timestamp(artifact.get('created_at'))
            updated = artifact_timestamp(artifact.get('updated_at'))
            if (artifact.get('expired') is not False or identity.get('id') != run_id
                    or identity.get('head_sha') != source
                    or not started <= created <= updated <= cutoff):
                raise RuntimeError('Artifact does not belong to the original successful build attempt')
            artifact_id = positive_id(artifact.get('id'))
            if artifact_id in selected_ids:
                raise RuntimeError('Artifact IDs must identify distinct original uploads')
            selected_ids.add(artifact_id)
            ids.append(str(artifact_id))
        result[key] = ','.join(ids)
    return result


def authorize(api, run_id, attempt):
    """Return the original source identity only after fresh release authorization."""
    run_id, attempt = positive_id(run_id), positive_id(attempt)
    run = api.get(f'actions/runs/{run_id}/attempts/{attempt}')
    title = re.fullmatch(r'Reviewed release ([0-9a-f]{40})', run.get('display_title', ''))
    if (run.get('id') != run_id or run.get('run_attempt') != attempt
            or run.get('path') not in {'.github/workflows/reviewed-release.yml',
                                       '.github/workflows/reviewed-release.yml@refs/heads/main'}
            or run.get('event') != 'workflow_dispatch' or run.get('head_branch') != 'main'
            or run.get('status') != 'completed' or run.get('conclusion') != 'failure'
            or not title or run.get('head_sha') != title[1]):
        raise RuntimeError('Require the exact failed trusted-main release attempt and source')
    source = title[1]
    pages = api.request(
        f'repos/{api.repo}/actions/runs/{run_id}/attempts/{attempt}/jobs?per_page=100',
        paginate=True)
    jobs = [job for page in pages for job in page['jobs']]
    expected = {name: 'success' for name in SUCCESS_JOBS}
    expected.update({SOURCE_JOB: 'failure', PUBLISH_JOB: 'skipped'})
    for name, conclusion in expected.items():
        matching = [job for job in jobs if job.get('name') == name]
        if (len(matching) != 1 or matching[0].get('status') != 'completed'
                or matching[0].get('conclusion') != conclusion):
            raise RuntimeError('Release attempt lacks unique successful platform/native or source-failure evidence')
    # Pages is optional; every additional workload must have succeeded too.
    if any(job.get('name') not in expected and (
            job.get('status') != 'completed' or job.get('conclusion') != 'success') for job in jobs):
        raise RuntimeError('Release attempt has another unsuccessful or incomplete job')
    artifacts = original_artifacts(api, run_id, source, run,
                                   next(job for job in jobs if job['name'] == SOURCE_JOB))
    candidates = [pr for pr in api.pages(f'commits/{source}/pulls?per_page=100')
                  if pr.get('merged_at') and pr.get('base', {}).get('ref') == 'main'
                  and pr.get('merge_commit_sha') == source]
    if len(candidates) > 1:
        raise RuntimeError('Release source has ambiguous merged PR authorization')
    if candidates:
        number, head = positive_id(candidates[0]['number']), candidates[0]['head']['sha']
    else:
        number, head = 0, source
    shas(head, source)
    # Reuse the ordinary exact-head Codex/build proof or owner main-repair proof.
    # Both paths also require source ancestry on main. The workflow head equals source.
    check_release(api, number, head, source)
    return dict(run_id=run_id, attempt=attempt, source_sha=source, pr_head=head, pr_number=number, **artifacts)


def write_outputs(path, result):
    # Open only an existing regular file and reject symlinks without a TOCTOU gap.
    descriptor = os.open(path, os.O_WRONLY | os.O_APPEND | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, 'a') as output:
        if not stat.S_ISREG(os.fstat(output.fileno()).st_mode):
            raise RuntimeError('GitHub output must be an existing regular file')
        output.write(''.join(f'{key}={value}\n' for key, value in result.items()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run-id', type=positive_id, required=True)
    parser.add_argument('--attempt', type=positive_id, default=1)
    parser.add_argument('--github-output')
    args = parser.parse_args()
    repo = os.environ['GITHUB_REPOSITORY']
    if (os.environ.get('GITHUB_REF') != 'refs/heads/main'
            or os.environ.get('GITHUB_WORKFLOW_REF') !=
            f'{repo}/.github/workflows/source-recovery.yml@refs/heads/main'):
        raise RuntimeError('Source recovery requires the trusted main workflow')
    api = GitHub(repo)
    tool = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=Path(__file__).resolve().parents[2],
                          text=True, capture_output=True, check=True).stdout.strip()
    shas(tool)
    if api.get(f'compare/{tool}...main').get('status') not in {'ahead', 'identical'}:
        raise RuntimeError('Recovery tools must be on main')
    result = dict(authorize(api, args.run_id, args.attempt), tool_sha=tool)
    if args.github_output:
        write_outputs(args.github_output, result)
    print(json.dumps(result))


if __name__ == '__main__':
    try:
        main()
    except Exception:
        # API and process diagnostics can contain URLs or credential-bearing input.
        print('Source recovery authorization failed; no recovery authorized', file=sys.stderr)
        sys.exit(1)
