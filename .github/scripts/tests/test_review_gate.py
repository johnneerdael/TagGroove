"""No application code runs before an authenticated final-head Codex approval."""
import importlib.util
from pathlib import Path
import sys
import unittest
from unittest.mock import Mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
HEAD, BASE, MERGE, SOURCE = ('a' * 40, 'b' * 40, 'c' * 40, 'd' * 40)


def load():
    path = ROOT / 'review_gate.py'
    if not path.exists():
        raise AssertionError('Review-controlled build dispatch is not implemented')
    spec = importlib.util.spec_from_file_location('review_gate', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def pull():
    return dict(number=17, state='open', base=dict(ref='main', sha=BASE), head=dict(sha=HEAD),
                merge_commit_sha=MERGE, mergeable=True, body='## Release notes\nInternal: Collect merged PR descriptions for published releases.')


class GateTests(unittest.TestCase):
    def setUp(self):
        self.gate = load()
        self.api = Mock()
        self.api.snapshot.return_value = (pull(), (True, 'Clean'))
        self.api.get.side_effect = lambda path: ({'object': {'sha': BASE}} if path == 'git/ref/heads/main'
                                               else {'parents': [{'sha': BASE}, {'sha': HEAD}], 'tree': {'sha': 'e' * 40}})
        self.api.pages.return_value = []
        self.api.runs.return_value = []

    def test_unreviewed_pr_never_dispatches(self):
        self.api.snapshot.return_value = (pull(), (False, 'No current Codex review'))
        self.gate.reconcile(self.api, 17)
        self.api.post.assert_not_called()

    def test_conflicting_or_pending_merge_stays_blocked_without_controller_error(self):
        for mergeable in (False, None):
            with self.subTest(mergeable=mergeable):
                item = pull()
                item.update(mergeable=mergeable, merge_commit_sha=None)
                self.api.snapshot.return_value = (item, (True, 'Clean'))
                self.gate.reconcile(self.api, 17)
                self.api.post.assert_not_called()
                self.assertEqual(self.api.status.call_args.args[:3],
                                 (HEAD, 'pending', 'PR is not mergeable'))

    def test_clean_exact_head_dispatches_only_trusted_main(self):
        self.gate.reconcile(self.api, 17)
        endpoint, body = self.api.post.call_args.args
        self.assertEqual(endpoint, 'actions/workflows/reviewed-pr.yml/dispatches')
        self.assertEqual(body, dict(ref='main', inputs=dict(pr_number='17', head_sha=HEAD, base_sha=BASE, merge_sha=MERGE)))

    def test_placeholder_notes_block_clean_review_dispatch_and_cancel_active_build(self):
        item = pull()
        item['body'] = '## Release notes\nTODO'
        self.api.snapshot.return_value = (item, (True, 'Clean'))
        self.api.runs.return_value = [dict(id=7, status='in_progress', display_title=f'Reviewed PR #17 @ {HEAD} + {BASE}')]
        self.gate.reconcile(self.api, 17)
        self.api.post.assert_called_once_with('actions/runs/7/cancel', {})
        notes = [call for call in self.api.status.call_args_list if call.kwargs.get('context') == 'Release notes']
        self.assertEqual(notes[0].args[1], 'failure')
        with self.assertRaisesRegex(RuntimeError, 'TODO'):
            self.gate.check_pr(self.api, 17)

    def test_merged_release_rechecks_live_notes_before_publication(self):
        item = pull()
        item.update(state='closed', merged=True, merge_commit_sha=SOURCE, body='## Release notes\nN/A')
        self.api.snapshot.return_value = (item, (True, 'Clean'))
        with self.assertRaisesRegex(RuntimeError, 'N/A'):
            self.gate.check_release(self.api, 17, HEAD, SOURCE)
        self.api.pages.assert_not_called()

    def test_wrong_merge_parents_and_stale_base_are_rejected(self):
        self.api.get.side_effect = lambda path: ({'object': {'sha': BASE}} if path == 'git/ref/heads/main'
                                               else {'parents': [{'sha': HEAD}, {'sha': BASE}]})
        with self.assertRaises(RuntimeError):
            self.gate.check_pr(self.api, 17)

    def test_preflight_rejects_a_changed_dispatch_sha(self):
        with self.assertRaises(RuntimeError):
            self.gate.check_pr(self.api, 17, source=(SOURCE, BASE, MERGE))

    def test_loss_of_review_cancels_running_validation(self):
        self.api.snapshot.return_value = (pull(), (False, 'Findings'))
        self.api.runs.return_value = [dict(id=7, status='in_progress', display_title=f'Reviewed PR #17 @ {HEAD} + {BASE}', head_branch='main')]
        self.gate.reconcile(self.api, 17)
        self.api.post.assert_called_once_with('actions/runs/7/cancel', {})

    def test_success_requires_all_jobs_and_a_still_matching_reviewed_merge(self):
        self.gate.finish(self.api, 17, (HEAD, BASE, MERGE), 'success', 'success', 'https://github.com/owner/repo/actions/runs/7')
        self.assertEqual(self.api.status.call_args.args[:3], (HEAD, 'success', f'Passed {BASE} {MERGE}'))
        self.api.status.reset_mock()
        self.gate.finish(self.api, 17, (HEAD, BASE, MERGE), 'success', 'skipped', 'url')
        self.assertEqual(self.api.status.call_args.args[1], 'failure')

    def test_direct_main_commit_has_no_release_authorization(self):
        self.api.pages.return_value = []
        self.gate.reconcile_main(self.api, SOURCE)
        self.api.post.assert_not_called()

    def test_merged_release_needs_clean_head_validated_tree_and_trusted_run(self):
        item = pull()
        item.update(state='closed', merged=True, merge_commit_sha=SOURCE)
        self.api.snapshot.return_value = (item, (True, 'Clean'))
        self.api.pages.return_value = [dict(context='Reviewed PR builds', state='success', description=f'Passed {BASE} {MERGE}', target_url='https://github.com/owner/repo/actions/runs/7', creator=dict(login='github-actions[bot]', type='Bot'))]
        self.api.repo = 'owner/repo'
        def get(path):
            if path == 'actions/runs/7':
                return dict(path='.github/workflows/reviewed-pr.yml', event='workflow_dispatch', head_branch='main', conclusion='success', status='completed', head_sha=BASE, display_title=f'Reviewed PR #17 @ {HEAD} + {BASE}')
            if path.startswith('compare/'):
                return dict(status='ahead')
            return dict(parents=[dict(sha=BASE), dict(sha=HEAD)], tree=dict(sha='e' * 40))
        self.api.get.side_effect = get
        self.gate.check_release(self.api, 17, HEAD, SOURCE)
        for key, value in [('head_sha', HEAD), ('display_title', 'Unrelated run'), ('path', '.github/workflows/other.yml'), ('conclusion', 'failure')]:
            def changed(path, key=key, value=value):
                result = get(path)
                if path == 'actions/runs/7':
                    result[key] = value
                return result
            self.api.get.side_effect = changed
            with self.subTest(field=key), self.assertRaises(RuntimeError):
                self.gate.check_release(self.api, 17, HEAD, SOURCE)
        self.api.get.side_effect = get
        self.api.pages.return_value[0]['description'] = f'Passed {HEAD} {MERGE}'
        with self.assertRaises(RuntimeError):
            self.gate.check_release(self.api, 17, HEAD, SOURCE)


class WorkflowTrustTests(unittest.TestCase):
    def test_workload_paths_have_no_automatic_trigger(self):
        workflows = ROOT.parent / 'workflows'
        self.assertFalse((workflows / 'build.yml').exists())
        self.assertFalse((workflows / 'release.yml').exists())
        for name in ['reviewed-pr.yml', 'reviewed-release.yml', '_reviewed-build.yml']:
            source = (workflows / name).read_text()
            header = source.split('jobs:')[0]
            self.assertNotIn('  push:', header)
            self.assertNotIn('  pull_request:', header)
            self.assertNotIn('  pull_request_target:', header)

    def test_reusable_build_rechecks_review_and_authenticates_main_caller(self):
        import re
        source = (ROOT.parent / 'workflows/_reviewed-build.yml').read_text()
        self.assertIn("github.workflow_ref == format('{0}/.github/workflows/reviewed-pr.yml@refs/heads/main', github.repository)", source)
        self.assertIn('review_gate.py verify', source)
        self.assertNotIn('secrets.', source)
        self.assertNotIn('rust-cache', source)
        self.assertNotIn('cache:', source)
        for job in ['tooling', 'desktop', 'native-e2e', 'android', 'docs']:
            block = re.search(rf'^  {job}:\n(.*?)(?=^  [a-z-]+:|\Z)', source, re.M | re.S)[1]
            self.assertIn('needs: authorize', block)
            self.assertIn("if: needs.authorize.outputs.ready == 'true'", block)
            self.assertIn('ref: ${{ inputs.merge_sha }}', block)
            self.assertIn('persist-credentials: false', block)

    def test_native_desktop_jobs_install_their_toolchains_before_building(self):
        import re
        for workflow, jobs in [('_reviewed-build.yml', ('desktop', 'native-e2e')), ('reviewed-release.yml', ('desktop',))]:
            source = (ROOT.parent / 'workflows' / workflow).read_text()
            for job in jobs:
                with self.subTest(workflow=workflow, job=job):
                    block = re.search(rf'^  {job}:\n(.*?)(?=^  [a-z-]+:|\Z)', source, re.M | re.S)[1]
                    before_build = re.split(r'run: cargo |npx tauri build|run: npm run tauri --', block, maxsplit=1)[0]
                    self.assertIn("python-version: '3.12'", before_build)
                    steps = re.split(r'^      - ', before_build, flags=re.M)
                    def selected(fragment, condition):
                        matches = [step for step in steps if fragment in step]
                        self.assertEqual(len(matches), 1, fragment)
                        self.assertIn(condition, matches[0])
                        return matches[0]
                    msvc = selected('uses: ilammy/msvc-dev-cmd@v1', "if: runner.os == 'Windows'")
                    self.assertIn('arch: x64', msvc)
                    msys = selected('uses: msys2/setup-msys2@v2', "if: runner.os == 'Windows'")
                    self.assertIn('install: make', msys)
                    windows = selected('choco install ffmpeg nasm', "if: runner.os == 'Windows'")
                    for required in ('shell: pwsh', '$LASTEXITCODE', "'nasm.exe'", '$env:GITHUB_PATH'):
                        self.assertIn(required, windows)
                    linux = selected('apt-get install', "if: runner.os == 'Linux'")
                    for tool in ('build-essential', 'cmake', 'make', 'nasm'):
                        self.assertRegex(linux, rf'\b{tool}\b')
                    macos = selected('brew install', "if: runner.os == 'macOS'")
                    for tool in ('cmake', 'nasm'):
                        self.assertRegex(macos, rf'\b{tool}\b')

    def test_mobile_jobs_prepare_python_cmake_and_keep_target_sdks(self):
        import re
        for workflow, job in [('_reviewed-build.yml', 'android'), ('reviewed-release.yml', 'android'), ('reviewed-release.yml', 'ios')]:
            with self.subTest(workflow=workflow, job=job):
                source = (ROOT.parent / 'workflows' / workflow).read_text()
                block = re.search(rf'^  {job}:\n(.*?)(?=^  [a-z-]+:|\Z)', source, re.M | re.S)[1]
                before_build = block.split('run: npm run tauri --')[0]
                self.assertIn("python-version: '3.12'", before_build)
                self.assertIn('cmake', before_build)
                if job == 'android':
                    for required in ('build-essential', 'make', 'ndk;28.2.13676358', 'NDK_HOME='):
                        self.assertIn(required, before_build)
                else:
                    self.assertIn('xcodegen', before_build)
                    self.assertIn('targets: aarch64-apple-ios-sim', before_build)

    def test_documentation_is_checked_only_after_review_when_present(self):
        import re
        source = (ROOT.parent / 'workflows/_reviewed-build.yml').read_text()
        block = re.search(r'^  docs:\n(.*?)(?=^  [a-z-]+:|\Z)', source, re.M | re.S)[1]
        self.assertEqual(block.count("if: hashFiles('mkdocs.yml') != ''"), 5)
        self.assertIn("python-version: '3.12'", block)
        for command in ['python -m pip install -r docs/site-requirements.txt',
                        'python -m unittest discover -s docs/tests -v',
                        'python scripts/docs-status.py', 'python -m mkdocs build --strict']:
            self.assertIn(command, block)
        self.assertNotIn('upload-pages-artifact', block)
        self.assertNotIn('deploy-pages', block)

    def test_release_checks_authorization_before_build_and_publication(self):
        source = (ROOT.parent / 'workflows/reviewed-release.yml').read_text()
        self.assertIn("github.workflow_ref == format('{0}/.github/workflows/reviewed-release.yml@refs/heads/main', github.repository)", source)
        self.assertIn('  prepare:\n    needs: authorize', source)
        self.assertEqual(source.count('review_gate.py release-check'), 2)
        self.assertLess(source.index('Recheck review before publishing'), source.index('Verify all platforms, calculate SHA256SUMS'))


if __name__ == '__main__':
    unittest.main()
