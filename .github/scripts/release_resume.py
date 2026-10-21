#!/usr/bin/env python3
"""Reuse a failed release's unchanged packages after repairing its UI harness.

Run only from trusted main. Existing review authorization and independent package
and source verifiers remain mandatory; this check only selects the artifact run.
"""
import os
import re
import subprocess

from review_gate import GitHub, shas
from source_recovery import artifact_timestamp, positive_id, write_outputs

DESKTOP_UI = (
    'Native UI and Android integration / desktop (ubuntu-24.04)',
    'Native UI and Android integration / desktop (windows-latest)',
    'Native UI and Android integration / desktop (macos-latest)',
)
REQUIRED_JOBS = (
    'authorize', 'Validate version, tag and source commit',
    'Desktop windows-x64', 'Desktop macos-arm64', 'Desktop macos-x64',
    'Desktop linux-x64', 'Desktop linux-arm64', 'Android arm64 signed release APK',
    'iOS arm64 simulator preview (debug, unsigned)', 'Assemble complete corresponding source',
    'Native UI and Android integration / authorize',
    'Native UI and Android integration / Android document integration',
)
REQUIRED_ARTIFACTS = tuple('release-' + platform for platform in (
    'windows-x64', 'macos-arm64', 'macos-x64', 'linux-x64', 'linux-arm64',
    'android-arm64', 'ios-arm64-simulator-preview', 'corresponding-source',
))


def validate(api, run_id, source, changes):
    if not re.fullmatch(r'[1-9][0-9]*', run_id):
        raise ValueError('A numeric original release run is required')
    shas(source)
    if any(not (path.startswith(('.github/', 'docs/')) or path in {'scripts/release.mjs', 'tests/release.test.mjs', 'tests/e2e/sources-4.js'})
           for path in changes):
        raise RuntimeError('Native recovery cannot reuse packages after application changes')
    run = api.get(f'actions/runs/{run_id}/attempts/1')
    workflow = api.get('actions/workflows/reviewed-release.yml')
    if (run.get('id') != int(run_id) or run.get('workflow_id') != workflow['id']
            or run.get('event') != 'workflow_dispatch' or run.get('head_branch') != 'main'
            or run.get('head_sha') != source or run.get('status') != 'completed'
            or run.get('conclusion') != 'failure' or run.get('run_attempt') != 1):
        raise RuntimeError('Recovery requires the original completed failed main release for this source')
    pages = api.request(f'repos/{api.repo}/actions/runs/{run_id}/attempts/1/jobs?per_page=100', paginate=True)
    jobs = [job for page in pages for job in page['jobs']]
    for name in REQUIRED_JOBS:
        found = [job for job in jobs if job['name'] == name]
        if len(found) != 1 or found[0].get('conclusion') != 'success':
            raise RuntimeError(f'Original release did not complete {name}')
    ui = [job for job in jobs if job['name'] in DESKTOP_UI]
    if (len(ui) != len(DESKTOP_UI) or {job['name'] for job in ui} != set(DESKTOP_UI)
            or not any(job.get('conclusion') == 'failure' for job in ui)):
        raise RuntimeError('No complete original desktop UI failure set')
    publication = [job for job in jobs if job['name'] == 'Publish complete verified release']
    if len(publication) != 1 or publication[0].get('conclusion') != 'skipped':
        raise RuntimeError('Original publication must be skipped after native validation failed')
    for job in jobs:
        if job.get('status') != 'completed':
            raise RuntimeError('Original release still has running jobs')
        if job.get('conclusion') in {'failure', 'cancelled'} and job['name'] not in DESKTOP_UI:
            raise RuntimeError('Recovery is limited to failed or cancelled desktop UI validation')
    started = artifact_timestamp(run.get('run_started_at'))
    source_job = next(job for job in jobs if job['name'] == 'Assemble complete corresponding source')
    completed = artifact_timestamp(source_job.get('completed_at'))
    if completed < started:
        raise RuntimeError('Invalid original release lifetime')
    pages = api.request(f'repos/{api.repo}/actions/runs/{run_id}/artifacts?per_page=100', paginate=True)
    artifacts = [item for page in pages for item in page['artifacts']]
    selected_ids = []
    for name in REQUIRED_ARTIFACTS:
        found = [item for item in artifacts if item['name'] == name]
        if (len(found) != 1 or found[0].get('expired') is not False
                or found[0].get('workflow_run', {}).get('head_sha') != source
                or found[0].get('workflow_run', {}).get('id') != int(run_id)):
            raise RuntimeError(f'Original release artifact is missing, expired or ambiguous: {name}')
        item = found[0]
        created, updated = artifact_timestamp(item.get('created_at')), artifact_timestamp(item.get('updated_at'))
        artifact_id = str(positive_id(item.get('id')))
        if not started <= created <= updated <= completed or artifact_id in selected_ids:
            raise RuntimeError('Artifacts must be distinct original uploads from the completed attempt')
        selected_ids.append(artifact_id)
    return dict(resume_run_id=run_id, resume_artifact_ids=','.join(selected_ids))


def main():
    run_id = os.environ.get('RESUME_RUN_ID', '')
    source = os.environ['SOURCE_SHA']
    harness = os.environ['GITHUB_SHA']
    selected = dict(resume_run_id='', resume_artifact_ids='')
    if run_id:
        shas(source, harness)
        if subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip() != harness:
            raise RuntimeError('Trusted main changed; dispatch recovery from its current head')
        subprocess.run(['git', 'merge-base', '--is-ancestor', source, harness], check=True)
        changes = subprocess.check_output(
            ['git', 'diff', '--name-only', '--no-renames', '-z', source, harness], text=True).split('\0')
        selected = validate(GitHub(os.environ['GITHUB_REPOSITORY']), run_id, source,
                            [path for path in changes if path])
    selected['harness_sha'] = harness if selected['resume_run_id'] else ''
    write_outputs(os.environ['GITHUB_OUTPUT'], selected)


if __name__ == '__main__':
    main()
