"""Source-only recovery reuses only a fully successful binary/native attempt."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
SOURCE, HEAD, TOOL = ('a' * 40, 'b' * 40, 'c' * 40)
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
ARTIFACT_NAMES = {key: [f'{prefix}-{platform}' for platform in platforms] for key, prefix, platforms in (
    ('native_ids', 'native-source', PLATFORMS),
    ('linux_ids', 'linux-source', ('linux-x64', 'linux-arm64')),
    ('macos_ids', 'macos-signing', ('macos-arm64', 'macos-x64')),
    ('release_ids', 'release', PLATFORMS),
)}


def load():
    path = ROOT / 'source_recovery.py'
    if not path.is_file():
        raise AssertionError('Source recovery metadata gate is not implemented')
    spec = importlib.util.spec_from_file_location('source_recovery', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def job(name, conclusion='success', status='completed'):
    return dict(name=name, status=status, conclusion=conclusion)


class RecoveryTests(unittest.TestCase):
    def setUp(self):
        self.gate = load()
        self.run = dict(id=42, run_attempt=1, path='.github/workflows/reviewed-release.yml',
                        event='workflow_dispatch', head_branch='main', head_sha=SOURCE,
                        status='completed', conclusion='failure', run_started_at='2026-10-20T10:00:00Z',
                        display_title=f'Reviewed release {SOURCE}')
        self.jobs = [job(name) for name in SUCCESS_JOBS]
        self.jobs += [dict(job(SOURCE_JOB, 'failure'), started_at='2026-10-20T11:00:00Z'),
                      job(PUBLISH_JOB, 'skipped')]
        self.artifacts = []
        for names in ARTIFACT_NAMES.values():
            for name in names:
                self.artifacts.append(dict(id=100 + len(self.artifacts), name=name, expired=False,
                                           created_at='2026-10-20T10:30:00Z', updated_at='2026-10-20T10:30:00Z',
                                           workflow_run=dict(id=42, head_sha=SOURCE)))
        self.ids = {key: ','.join(str(next(item['id'] for item in self.artifacts if item['name'] == name))
                                 for name in names) for key, names in ARTIFACT_NAMES.items()}
        self.prs = [dict(number=17, merged_at='2026-10-20T12:00:00Z',
                         base=dict(ref='main'), head=dict(sha=HEAD), merge_commit_sha=SOURCE)]
        self.api = Mock(repo='owner/repo')
        self.api.get.side_effect = lambda path: self.run if path == 'actions/runs/42/attempts/1' else dict(status='ahead')
        self.api.request.side_effect = lambda endpoint, **kw: (
            [dict(artifacts=self.artifacts[:8]), dict(artifacts=self.artifacts[8:])]
            if '/artifacts?' in endpoint else [dict(jobs=self.jobs[:8]), dict(jobs=self.jobs[8:])])
        self.api.pages.side_effect = lambda path: self.prs
        self.release = patch.object(self.gate, 'check_release').start()
        self.addCleanup(patch.stopall)

    def authorize(self):
        return self.gate.authorize(self.api, 42, 1)

    def test_authorizes_exact_attempt_and_all_pages_without_mutations(self):
        self.assertEqual(self.authorize(), dict(run_id=42, attempt=1, source_sha=SOURCE,
                                               pr_head=HEAD, pr_number=17, **self.ids))
        self.api.get.assert_called_with('actions/runs/42/attempts/1')
        self.assertEqual(self.api.request.call_count, 2)
        self.api.request.assert_any_call(
            'repos/owner/repo/actions/runs/42/attempts/1/jobs?per_page=100', paginate=True)
        self.api.request.assert_any_call(
            'repos/owner/repo/actions/runs/42/artifacts?per_page=100', paginate=True)
        self.api.pages.assert_called_once_with(f'commits/{SOURCE}/pulls?per_page=100')
        self.release.assert_called_once_with(self.api, 17, HEAD, SOURCE)
        self.api.post.assert_not_called()
        self.api.status.assert_not_called()
        self.api.jobs.assert_not_called()

    def test_workflow_path_may_include_trusted_main_ref(self):
        self.run['path'] += '@refs/heads/main'
        self.authorize()

    def test_optional_pages_must_succeed(self):
        self.jobs.append(job('Publish reviewed user guide to Pages'))
        self.authorize()
        for state in ('failure', 'skipped', 'cancelled', None):
            with self.subTest(state=state):
                self.jobs[-1]['conclusion'] = state
                with self.assertRaises(RuntimeError):
                    self.authorize()

    def test_each_platform_and_native_job_is_required_once_and_successful(self):
        for name in SUCCESS_JOBS:
            with self.subTest(name=name):
                selected = next(item for item in self.jobs if item['name'] == name)
                self.jobs.remove(selected)
                with self.assertRaises(RuntimeError):
                    self.authorize()
                self.jobs.append(selected)
                self.jobs.append(dict(selected))
                with self.assertRaises(RuntimeError):
                    self.authorize()
                self.jobs.pop()
                for field, value in (('conclusion', 'failure'), ('conclusion', 'cancelled'),
                                     ('conclusion', 'skipped'), ('status', 'in_progress')):
                    original = selected[field]
                    selected[field] = value
                    with self.assertRaises(RuntimeError):
                        self.authorize()
                    selected[field] = original

    def test_source_and_publication_have_exact_expected_outcomes(self):
        for name in (SOURCE_JOB, PUBLISH_JOB):
            selected = next(item for item in self.jobs if item['name'] == name)
            self.jobs.remove(selected)
            with self.assertRaises(RuntimeError):
                self.authorize()
            self.jobs.append(selected)
            self.jobs.append(dict(selected))
            with self.assertRaises(RuntimeError):
                self.authorize()
            self.jobs.pop()
            for field, value in (('conclusion', 'success'), ('conclusion', 'cancelled'),
                                 ('status', 'queued')):
                with self.subTest(name=name, field=field, value=value):
                    original = selected[field]
                    selected[field] = value
                    with self.assertRaises(RuntimeError):
                        self.authorize()
                    selected[field] = original

    def test_any_other_failure_cancel_or_incomplete_job_denies_reuse(self):
        for status, conclusion in [('completed', 'failure'), ('completed', 'cancelled'),
                                   ('in_progress', None), ('queued', None)]:
            with self.subTest(status=status, conclusion=conclusion):
                self.jobs.append(job('Unexpected workload', conclusion, status))
                with self.assertRaises(RuntimeError):
                    self.authorize()
                self.jobs.pop()

    def test_untrusted_run_or_wrong_source_attempt_is_rejected(self):
        for field, value in [('id', 43), ('run_attempt', 2), ('run_attempt', None),
                             ('path', '.github/workflows/reviewed-pr.yml'),
                             ('path', '.github/workflows/reviewed-release.yml@refs/heads/topic'),
                             ('event', 'push'), ('head_branch', 'topic'),
                             ('status', 'in_progress'), ('conclusion', 'cancelled'),
                             ('conclusion', 'success'), ('head_sha', HEAD),
                             ('display_title', f'Reviewed release {SOURCE} extra'),
                             ('display_title', 'Reviewed release short')]:
            with self.subTest(field=field, value=value):
                original = self.run[field]
                self.run[field] = value
                with self.assertRaises((RuntimeError, ValueError)):
                    self.authorize()
                self.run[field] = original

    def test_ids_are_positive_numeric_values(self):
        for invalid in (0, -1, '0', '01', '+1', '1/attempts/2', True, None, 1.2):
            for run_id, attempt in ((invalid, 1), (42, invalid)):
                with self.subTest(run_id=run_id, attempt=attempt):
                    with self.assertRaises((RuntimeError, ValueError)):
                        self.gate.authorize(self.api, run_id, attempt)
        self.assertEqual(self.gate.authorize(self.api, '42', '1')['run_id'], 42)

    def test_direct_main_source_rechecks_owner_repair_authorization(self):
        self.prs.clear()
        self.assertEqual(self.authorize()['pr_number'], 0)
        self.release.assert_called_once_with(self.api, 0, SOURCE, SOURCE)

    def test_only_exact_merged_pr_targeting_main_selects_identity(self):
        self.prs += [dict(self.prs[0], number=18, merge_commit_sha=HEAD),
                     dict(self.prs[0], number=19, base=dict(ref='other')),
                     dict(self.prs[0], number=20, merged_at=None)]
        self.authorize()
        self.release.assert_called_once_with(self.api, 17, HEAD, SOURCE)

    def test_ambiguous_merged_sources_are_rejected(self):
        self.prs.append(dict(self.prs[0], number=18))
        with self.assertRaises(RuntimeError):
            self.authorize()
        self.release.assert_not_called()

    def test_invalid_pr_identity_and_release_proof_fail_closed(self):
        self.prs[0]['head']['sha'] = 'short'
        with self.assertRaises(ValueError):
            self.authorize()
        self.prs[0]['head']['sha'] = HEAD
        self.release.side_effect = RuntimeError('Review proof revoked')
        with self.assertRaisesRegex(RuntimeError, 'Review proof revoked'):
            self.authorize()

    def test_metadata_api_failures_deny_recovery(self):
        for method in ('get', 'request', 'pages'):
            with self.subTest(method=method):
                call = getattr(self.api, method)
                original = call.side_effect
                call.side_effect = RuntimeError('Metadata unavailable')
                with self.assertRaisesRegex(RuntimeError, 'Metadata unavailable'):
                    self.authorize()
                call.side_effect = original

    def test_each_original_artifact_is_required_unique_and_unexpired(self):
        for artifact in list(self.artifacts):
            with self.subTest(name=artifact['name']):
                self.artifacts.remove(artifact)
                with self.assertRaises(RuntimeError):
                    self.authorize()
                self.artifacts.append(artifact)
                self.artifacts.append(dict(artifact, id=999))
                with self.assertRaises(RuntimeError):
                    self.authorize()
                self.artifacts.pop()
                artifact['expired'] = True
                with self.assertRaises(RuntimeError):
                    self.authorize()
                artifact['expired'] = False

    def test_later_attempt_and_replacement_artifacts_are_rejected(self):
        selected = self.artifacts[0]
        for field in ('created_at', 'updated_at'):
            for value in ('2026-10-20T11:00:01Z', '2026-10-20T09:59:59Z', None, 'invalid'):
                with self.subTest(field=field, value=value):
                    original = selected[field]
                    selected[field] = value
                    with self.assertRaises((RuntimeError, ValueError)):
                        self.authorize()
                    selected[field] = original
        for workflow in (dict(id=43, head_sha=SOURCE), dict(id=42, head_sha=HEAD), {}):
            original = selected['workflow_run']
            selected['workflow_run'] = workflow
            with self.assertRaises(RuntimeError):
                self.authorize()
            selected['workflow_run'] = original
        selected['id'] = 0
        with self.assertRaises(ValueError):
            self.authorize()

    def test_artifact_window_requires_valid_original_job_timestamps(self):
        source = next(item for item in self.jobs if item['name'] == SOURCE_JOB)
        for item, field in ((self.run, 'run_started_at'), (source, 'started_at')):
            original = item[field]
            for value in (None, 'invalid', '2026-10-20T10:00:00'):
                item[field] = value
                with self.assertRaises((RuntimeError, ValueError)):
                    self.authorize()
            item[field] = original
        source['started_at'] = '2026-10-20T09:00:00Z'
        with self.assertRaises(RuntimeError):
            self.authorize()


class CliTests(unittest.TestCase):
    def setUp(self):
        self.gate = load()
        self.env = dict(GITHUB_REPOSITORY='owner/repo', GITHUB_REF='refs/heads/main',
                        GITHUB_WORKFLOW_REF='owner/repo/.github/workflows/source-recovery.yml@refs/heads/main')
        self.result = dict(run_id=42, attempt=1, source_sha=SOURCE, pr_head=HEAD, pr_number=17)
        self.api = Mock()
        self.api.get.return_value = dict(status='ahead')

    def invoke(self, extra=()):
        with patch.dict(os.environ, self.env, clear=True), \
                patch.object(sys, 'argv', ['source_recovery.py', '--run-id', '42', *extra]), \
                patch.object(self.gate, 'GitHub', return_value=self.api), \
                patch.object(self.gate, 'authorize', return_value=dict(self.result)), \
                patch.object(self.gate.subprocess, 'run', return_value=Mock(stdout=TOOL + '\n')) as git, \
                contextlib.redirect_stdout(io.StringIO()) as output:
            self.gate.main()
        return json.loads(output.getvalue()), git

    def test_emits_tool_commit_and_safe_outputs(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'github-output'
            output.touch()
            result, git = self.invoke(['--github-output', str(output)])
            self.assertEqual(result, dict(self.result, tool_sha=TOOL))
            self.assertEqual(output.read_text(), ''.join(f'{key}={value}\n' for key, value in result.items()))
        git.assert_called_once_with(['git', 'rev-parse', 'HEAD'], cwd=ROOT.parents[1],
                                    text=True, capture_output=True, check=True)
        self.api.get.assert_called_once_with(f'compare/{TOOL}...main')

    def test_rejects_untrusted_caller(self):
        for key, value in [('GITHUB_REF', 'refs/heads/topic'),
                           ('GITHUB_WORKFLOW_REF', 'owner/repo/.github/workflows/source-recovery.yml@refs/heads/topic'),
                           ('GITHUB_WORKFLOW_REF', 'elsewhere/repo/.github/workflows/source-recovery.yml@refs/heads/main')]:
            with self.subTest(key=key):
                self.env[key] = value
                with self.assertRaises(RuntimeError):
                    self.invoke()
                self.env = dict(GITHUB_REPOSITORY='owner/repo', GITHUB_REF='refs/heads/main',
                                GITHUB_WORKFLOW_REF='owner/repo/.github/workflows/source-recovery.yml@refs/heads/main')

    def test_rejects_tools_not_on_main(self):
        for status in ('behind', 'diverged', None):
            self.api.get.return_value = dict(status=status)
            with self.assertRaises(RuntimeError):
                self.invoke()

    def test_output_must_exist_and_be_regular_without_symlinks(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            regular = base / 'regular'
            regular.write_text('unchanged')
            link = base / 'link'
            link.symlink_to(regular)
            for target in (base / 'missing', base, link):
                with self.subTest(target=target):
                    with self.assertRaises((OSError, RuntimeError)):
                        self.invoke(['--github-output', str(target)])
            self.assertEqual(regular.read_text(), 'unchanged')


if __name__ == '__main__':
    unittest.main()
