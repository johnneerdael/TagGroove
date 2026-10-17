"""No application code runs before an authenticated final-head Codex approval."""
import importlib.util
from pathlib import Path
import sys
import unittest
from unittest.mock import Mock

from test_codex_review import pr as reviewed_pull, review, thumb

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

    def test_new_or_edited_request_invalidates_an_unchanged_validated_head_without_dispatch(self):
        from codex_review import eligibility
        item = reviewed_pull()
        event = dict(issue=dict(number=17, pull_request={}), comment=dict(
            user=dict(login='author', type='User'), body='@codex review',
            created_at='2026-10-15T11:00:00Z'))
        self.assertTrue(eligibility(item, [review()], [], [], [thumb()])[0])
        self.api.get.side_effect = None
        self.api.get.return_value = item
        self.api.snapshot.return_value = (item, eligibility(item, [review()], [event['comment']], [], [thumb()]))
        for edited in (False, True):
            with self.subTest(edited=edited):
                if edited:
                    event['changes'] = dict(body={'from': 'Please check this PR'})
                    event['comment']['created_at'] = '2026-10-15T09:00:00Z'
                    event['comment']['updated_at'] = '2026-10-15T11:00:00Z'
                self.api.snapshot.return_value = (item, eligibility(item, [review()], [event['comment']], [], [thumb()]))
                statuses = {'Codex review': 'success', 'Reviewed PR builds': 'success'}
                self.api.status.side_effect = lambda head, state, reason, context: statuses.update({context: state})
                self.api.status.reset_mock()
                self.gate.invalidate_metadata(self.api, event)
                self.assertEqual(statuses, {'Codex review': 'pending', 'Reviewed PR builds': 'pending'})
                self.assertEqual(self.api.status.call_count, 2)
                for call in self.api.status.call_args_list:
                    self.assertEqual(call.args[:2], (HEAD, 'pending'))
                self.api.post.assert_not_called()
                self.api.runs.assert_not_called()

    def test_review_request_with_instructions_withdraws_unchanged_head_approval(self):
        from codex_review import eligibility
        item = reviewed_pull()
        comment = dict(user=dict(login='author', type='User'),
                       body='@codex review the complete final code diff at this head',
                       created_at='2026-10-15T11:00:00Z')
        self.assertTrue(eligibility(item, [review()], [], [], [thumb()])[0])
        eligible = eligibility(item, [review()], [comment], [], [thumb()])
        self.assertFalse(eligible[0])
        self.api.get.side_effect = None
        self.api.get.return_value = item
        self.api.snapshot.return_value = (item, eligible)
        self.gate.invalidate_metadata(self.api, dict(issue=dict(number=17, pull_request={}), comment=comment))
        self.assertEqual(self.api.status.call_count, 2)
        for call in self.api.status.call_args_list:
            self.assertEqual(call.args[:2], (HEAD, 'pending'))
        self.api.post.assert_not_called()
        self.api.runs.assert_not_called()

    def test_reviewer_team_and_draft_changes_withdraw_unchanged_head_approval_without_dispatch(self):
        from codex_review import eligibility
        for field, value, action in [
                ('requested_reviewers', [dict(login='reviewer')], 'review_requested'),
                ('requested_teams', [dict(slug='maintainers')], 'review_requested'),
                ('draft', True, 'converted_to_draft')]:
            with self.subTest(field=field):
                item = reviewed_pull()
                self.assertTrue(eligibility(item, [review()], [], [], [thumb()])[0])
                item[field] = value
                self.api.get.side_effect = None
                self.api.get.return_value = item
                self.api.snapshot.return_value = (item, eligibility(item, [review()], [], [], [thumb()]))
                statuses = {'Codex review': 'success', 'Reviewed PR builds': 'success'}
                self.api.status.side_effect = lambda head, state, reason, context: statuses.update({context: state})
                self.api.status.reset_mock()
                self.gate.invalidate_metadata(self.api, dict(number=17, action=action, pull_request=item))
                self.assertEqual(statuses, {'Codex review': 'pending', 'Reviewed PR builds': 'pending'})
                for call in self.api.status.call_args_list:
                    self.assertEqual(call.args[:2], (HEAD, 'pending'))
                self.api.post.assert_not_called()
                self.api.runs.assert_not_called()

    def test_lifecycle_cannot_inherit_green_statuses_from_another_pr_or_revision(self):
        from codex_review import eligibility
        for action in ('opened', 'reopened', 'synchronize', 'ready_for_review', 'edited'):
            with self.subTest(action=action):
                item = reviewed_pull()
                item['number'] = 17
                # A new PR has no review; even a later transition with its own
                # clean review must not inherit another PR's notes/build proof.
                reviews = [] if action == 'opened' else [review()]
                eligible = eligibility(item, reviews, [], [], [thumb()])
                self.assertEqual(eligible[0], action != 'opened')
                self.api.get.side_effect = None
                self.api.get.return_value = item
                self.api.snapshot.return_value = (item, eligible)
                statuses = {'Codex review': 'success', 'Release notes': 'success', 'Reviewed PR builds': 'success'}
                self.api.status.side_effect = lambda head, state, reason, context: statuses.update({context: state})
                self.api.status.reset_mock()
                event = dict(number=17, action=action, pull_request=dict(head=dict(sha=SOURCE)))
                if action == 'edited':
                    event['changes'] = dict(base=dict(ref={'from': 'release'}, sha={'from': BASE}))
                self.gate.invalidate_metadata(self.api, event)
                self.assertEqual(statuses, dict.fromkeys(statuses, 'pending'))
                for call in self.api.status.call_args_list:
                    self.assertEqual(call.args[:2], (HEAD, 'pending'))
                self.api.snapshot.assert_not_called()
                self.api.post.assert_not_called()
                self.api.runs.assert_not_called()

    def test_body_or_title_edits_remain_with_the_existing_notes_controller(self):
        for field in ('body', 'title'):
            self.gate.invalidate_metadata(self.api, dict(number=17, action='edited',
                pull_request=reviewed_pull(), changes={field: {'from': 'Previous text'}}))
        self.api.get.assert_not_called()
        self.api.snapshot.assert_not_called()
        self.api.status.assert_not_called()
        self.api.post.assert_not_called()

    def test_unrelated_and_untrusted_comments_do_not_invalidate_or_dispatch(self):
        item = reviewed_pull()
        self.api.get.side_effect = None
        self.api.get.return_value = item
        for body, login in [('Looks good', 'author'), ('@codex review', 'outsider')]:
            self.gate.invalidate_metadata(self.api, dict(issue=dict(number=17, pull_request={}),
                comment=dict(body=body, user=dict(login=login, type='User'))))
        self.api.snapshot.assert_not_called()
        self.api.status.assert_not_called()
        self.api.post.assert_not_called()

    def test_connector_finding_invalidates_unchanged_head_without_dispatch(self):
        from codex_review import eligibility
        item = reviewed_pull()
        comment = dict(user=dict(login='chatgpt-codex-connector[bot]', type='Bot'), body=(
            '<!-- codex-review-state:v1 {"code_review":{"commit_sha":"' + HEAD +
            '","status":"completed","has_any_findings":true,"has_unresolved_findings":true}} -->\n'
            '<!-- codex-pull-request-review-summary -->'))
        self.api.get.side_effect = None
        self.api.get.return_value = item
        self.api.snapshot.return_value = (item, eligibility(item, [review()], [comment], [], [thumb()]))
        self.gate.invalidate_metadata(self.api, dict(issue=dict(number=17, pull_request={}), comment=comment))
        self.assertEqual(self.api.status.call_count, 2)
        self.api.post.assert_not_called()

    def test_initial_pr_lookup_retries_before_withdrawing_stale_approval(self):
        from codex_review import eligibility
        item = reviewed_pull()
        comment = dict(user=dict(login='author', type='User'), body='@codex review',
                       created_at='2026-10-15T11:00:00Z')
        self.api.get.side_effect = [RuntimeError('Temporary PR lookup failure'), item]
        self.api.snapshot.return_value = (item, eligibility(item, [review()], [comment], [], [thumb()]))
        self.gate.invalidate_metadata(self.api, dict(issue=dict(number=17, pull_request={}), comment=comment))
        self.assertEqual(self.api.get.call_count, 2)
        self.assertEqual(self.api.status.call_count, 2)
        for call in self.api.status.call_args_list:
            self.assertEqual(call.args[:2], (HEAD, 'pending'))
        self.api.post.assert_not_called()
        self.api.runs.assert_not_called()

    def test_connector_metadata_api_failure_withdraws_old_approval(self):
        item = reviewed_pull()
        self.api.get.side_effect = None
        self.api.get.return_value = item
        self.api.snapshot.side_effect = RuntimeError('Cannot read review history')
        self.gate.invalidate_metadata(self.api, dict(issue=dict(number=17, pull_request={}),
            comment=dict(body='Review finding', user=dict(login='chatgpt-codex-connector[bot]', type='Bot'))))
        self.assertEqual(self.api.status.call_count, 2)
        self.api.post.assert_not_called()

    def test_clean_completion_does_not_restore_or_dispatch_validation(self):
        item = reviewed_pull()
        self.api.get.side_effect = None
        self.api.get.return_value = item
        self.api.snapshot.return_value = (item, (True, 'Clean'))
        self.gate.invalidate_metadata(self.api, dict(issue=dict(number=17, pull_request={}),
            comment=dict(body='Review complete', user=dict(login='chatgpt-codex-connector[bot]', type='Bot'))))
        self.api.status.assert_not_called()
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

    def test_review_controller_has_no_polling_or_per_push_triggers(self):
        workflows = ROOT.parent / 'workflows'
        controller = (workflows / 'review-controller.yml').read_text().split('jobs:')[0]
        for trigger in ['push', 'workflow_run', 'workflow_dispatch']:
            self.assertIn(f'  {trigger}:', controller)
        for trigger in ['pull_request', 'issue_comment', 'schedule']:
            self.assertNotIn(f'  {trigger}:', controller)
        self.assertNotIn('cron:', controller)
        # Only an edited description re-checks the release notes; pushes and comments never run it.
        source = (workflows / 'review-controller.yml').read_text()
        # An edit reconciles only the edited PR, never every open PR.
        self.assertIn("PR_NUMBER: ${{ inputs.pr_number || github.event.pull_request.number || '0' }}", source)
        self.assertRegex(controller, r'pull_request_target:\n    branches: \[main\]\n    types: \[edited\]\n')
        # Review events still invalidate an earlier approval, through the unprivileged relay.
        self.assertIn('workflows: [Review metadata signal, Reviewed PR validation]', controller)
        signal = (workflows / 'review-metadata-signal.yml').read_text().split('jobs:')[0]
        for trigger in ['pull_request_review', 'pull_request_review_comment']:
            self.assertIn(f'  {trigger}:', signal)
        for trigger in ['pull_request', 'issue_comment', 'schedule']:
            self.assertNotIn(f'  {trigger}:', signal)

    def test_metadata_invalidation_uses_trusted_main_without_build_permissions(self):
        source = (ROOT.parent / 'workflows/review-metadata-invalidation.yml').read_text()
        for required in ['issue_comment:', 'types: [created, edited, deleted]',
                         'pull_request_target:', 'types: [opened, reopened, synchronize, ready_for_review, edited, review_requested, converted_to_draft]',
                         "github.event_name == 'pull_request_target'",
                         "github.event.action != 'edited' || github.event.changes.base",
                         'github.event.issue.pull_request', "contains(github.event.comment.body, '@codex')",
                         "contains(github.event.changes.body.from, '@codex')",
                         "github.event.comment.user.login == 'chatgpt-codex-connector[bot]'",
                         'ref: main', 'persist-credentials: false', 'review_gate.py invalidate-metadata']:
            self.assertIn(required, source)
        for denied in ['actions: write', 'secrets.', 'inputs.merge_sha', 'review_gate.py reconcile', 'workflow_run:']:
            self.assertNotIn(denied, source)
        controller = (ROOT.parent / 'workflows/review-controller.yml').read_text()
        self.assertNotIn('Review metadata invalidation', controller)
        self.assertIn("if: github.event_name != 'pull_request_target' || !github.event.changes.base", controller)

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
                    self.assertIn('id: msys2', msys)
                    binding = selected('steps.msys2.outputs.msys2-location', "if: runner.os == 'Windows'")
                    for required in ("'usr/bin/bash.exe'", "'usr/bin/make.exe'", 'OPENTAG_MSYS2=$env:MSYS2_LOCATION', '$env:GITHUB_ENV'):
                        self.assertIn(required, binding)
                    windows = selected('choco install ffmpeg nasm', "if: runner.os == 'Windows'")
                    for required in ('shell: pwsh', '$LASTEXITCODE', "'nasm.exe'", '$env:GITHUB_PATH'):
                        self.assertIn(required, windows)
                    # The retried install can leave a failing Chocolatey code behind a recovered fallback.
                    self.assertIn('$global:LASTEXITCODE = 0', windows)
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
                before_build = block.split('npm run tauri --')[0]
                self.assertIn("python-version: '3.12'", before_build)
                self.assertIn('cmake', before_build)
                self.assertIn('export TAURI_OPENTAG_AUDIO_BUILD_ROOT="$OPENTAG_AUDIO_BUILD_ROOT"', before_build)
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
