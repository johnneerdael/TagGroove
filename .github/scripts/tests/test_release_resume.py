"""Native-harness recovery never rebuilds or relabels application packages."""
import copy
import sys
import unittest
from pathlib import Path
from unittest.mock import Mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import release_resume

SOURCE = 'a' * 40


class ResumeTests(unittest.TestCase):
    def setUp(self):
        self.api = Mock()
        self.run = dict(id=123, workflow_id=7, event='workflow_dispatch', head_branch='main',
                        head_sha=SOURCE, status='completed', conclusion='failure', run_attempt=1,
                        run_started_at='2026-10-21T00:00:00Z')
        self.jobs = [dict(name=name, status='completed', conclusion='success', completed_at='2026-10-21T00:04:00Z')
                     for name in release_resume.REQUIRED_JOBS]
        self.jobs += [dict(name=name, status='completed', conclusion=state)
                      for name, state in zip(release_resume.DESKTOP_UI, ['failure', 'cancelled', 'cancelled'])]
        self.jobs += [dict(name='Publish complete verified release', status='completed', conclusion='skipped')]
        self.artifacts = [dict(id=index, name=name, expired=False, workflow_run=dict(head_sha=SOURCE, id=123),
                               created_at='2026-10-21T00:01:00Z', updated_at='2026-10-21T00:01:00Z')
                          for index, name in enumerate(release_resume.REQUIRED_ARTIFACTS, 1)]
        self.api.get.side_effect = lambda path: self.run if path == 'actions/runs/123' else dict(id=7)
        self.api.jobs.return_value = self.jobs
        self.api.request.return_value = [dict(artifacts=self.artifacts)]

    def check(self, changes=('tests/e2e/sources-4.js', '.github/workflows/reviewed-release.yml')):
        return release_resume.validate(self.api, '123', SOURCE, changes)

    def test_retains_complete_original_packages_and_only_retries_desktop_validation(self):
        self.assertEqual(self.check(), dict(resume_run_id='123', resume_artifact_ids='1,2,3,4,5,6,7,8'))

    def test_refuses_wrong_source_fork_workflow_incomplete_and_successful_runs(self):
        for field, value in [('head_sha', 'b' * 40), ('event', 'pull_request'),
                             ('head_branch', 'other'), ('workflow_id', 8), ('status', 'in_progress'),
                             ('conclusion', 'success'), ('run_attempt', 2)]:
            with self.subTest(field=field):
                old = self.run[field]
                self.run[field] = value
                with self.assertRaises(RuntimeError):
                    self.check()
                self.run[field] = old

    def test_refuses_application_changes_even_when_all_original_packages_passed(self):
        for path in ['src/tag-sources.ts', 'package.json', 'src-tauri/Cargo.toml', 'crates/tag-core/src/lib.rs']:
            with self.subTest(path=path), self.assertRaises(RuntimeError):
                self.check((path,))

    def test_refuses_missing_failed_and_duplicate_required_jobs(self):
        original = copy.deepcopy(self.jobs)
        for replacement in [original[1:], [dict(original[0], conclusion='failure')] + original[1:], original + [original[0]]]:
            self.api.jobs.return_value = replacement
            with self.assertRaises(RuntimeError):
                self.check()

    def test_refuses_other_failures_and_unfinished_native_validation(self):
        for job in [dict(name='unexpected', status='completed', conclusion='failure'),
                    dict(name='unexpected', status='in_progress', conclusion=None)]:
            self.api.jobs.return_value = self.jobs + [job]
            with self.assertRaises(RuntimeError):
                self.check()

    def test_refuses_expired_missing_duplicate_and_foreign_artifacts(self):
        original = copy.deepcopy(self.artifacts)
        variants = [original[1:], original + [original[0]],
                    [dict(original[0], expired=True)] + original[1:],
                    [dict(original[0], workflow_run=dict(head_sha='b' * 40, id=123))] + original[1:]]
        for artifacts in variants:
            self.api.request.return_value = [dict(artifacts=artifacts)]
            with self.assertRaises(RuntimeError):
                self.check()

    def test_refuses_later_replacements_foreign_uploads_and_duplicate_artifact_ids(self):
        for patch in [dict(updated_at='2026-10-21T00:05:00Z'),
                      dict(created_at='2026-10-20T23:59:00Z'),
                      dict(workflow_run=dict(head_sha=SOURCE, id=999)), dict(id=2)]:
            with self.subTest(patch=patch):
                original = copy.deepcopy(self.artifacts)
                original[0].update(patch)
                self.api.request.return_value = [dict(artifacts=original)]
                with self.assertRaises(RuntimeError):
                    self.check()

    def test_refuses_invalid_ids_and_api_failures(self):
        for run in ['0', '../123', '123\n', '1;echo bad']:
            with self.subTest(run=run), self.assertRaises((ValueError, RuntimeError)):
                release_resume.validate(self.api, run, SOURCE, ())
        self.api.get.side_effect = RuntimeError('unavailable')
        with self.assertRaises(RuntimeError):
            self.check()


if __name__ == '__main__':
    unittest.main()
