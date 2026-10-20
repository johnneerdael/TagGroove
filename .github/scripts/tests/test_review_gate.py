"""No application code runs before an authenticated final-head Codex approval."""
import importlib.util
from pathlib import Path
import sys
import unittest
from unittest.mock import Mock, patch

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
                self.api.status.side_effect = lambda head, state, reason, context, url=None: statuses.update({context: state})
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
                self.api.status.side_effect = lambda head, state, reason, context, url=None: statuses.update({context: state})
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
                self.api.status.side_effect = lambda head, state, reason, context, url=None: statuses.update({context: state})
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

    def test_shared_head_stays_pending_through_reconcile_and_completion_until_sibling_closes(self):
        approved = pull()
        sibling = {**pull(), 'number': 18}
        open_prs = [approved, sibling]
        statuses = dict.fromkeys(('Codex review', 'Release notes', 'Reviewed PR builds'),
                                 dict(state='success', description=f'Passed {BASE} {MERGE}'))
        def status(head, state, reason, context='Codex review', url=None):
            self.assertEqual(head, HEAD)
            statuses[context] = dict(state=state, description=reason)
        def pages(path):
            if path == 'pulls?state=open&per_page=100':
                return open_prs
            return [dict(context=context, **value) for context, value in statuses.items()]
        def get(path):
            if path == 'pulls/18':
                return sibling
            if path == 'git/ref/heads/main':
                return dict(object=dict(sha=BASE))
            return dict(parents=[dict(sha=BASE), dict(sha=HEAD)])
        self.api.get.side_effect = get
        self.api.pages.side_effect = pages
        self.api.status.side_effect = status
        self.api.snapshot.return_value = (approved, (True, 'Clean own-PR review'))
        self.gate.invalidate_metadata(self.api, dict(number=18, action='opened', pull_request=sibling))
        self.assertTrue(all(value['state'] == 'pending' for value in statuses.values()))
        self.gate.reconcile(self.api, 17)
        self.gate.finish(self.api, 17, (HEAD, BASE, MERGE), 'success', 'success', 'url')
        self.assertTrue(all(value['state'] == 'pending' for value in statuses.values()), statuses)
        self.api.post.assert_not_called()
        with self.assertRaisesRegex(RuntimeError, 'shares.*open PR'):
            self.gate.check_pr(self.api, 17, (HEAD, BASE, MERGE))
        open_prs.pop()
        self.gate.reconcile(self.api, 17)
        self.api.post.assert_called_once()
        self.gate.finish(self.api, 17, (HEAD, BASE, MERGE), 'success', 'success', 'url')
        self.assertTrue(all(value['state'] == 'success' for value in statuses.values()), statuses)

    def test_other_base_sibling_opening_also_withdraws_shared_statuses(self):
        sibling = {**pull(), 'number': 18, 'base': dict(ref='release')}
        self.api.get.side_effect = None
        self.api.get.return_value = sibling
        self.gate.invalidate_metadata(self.api, dict(number=18, action='opened', pull_request=sibling))
        self.assertEqual({call.kwargs.get('context') for call in self.api.status.call_args_list},
                         {'Codex review', 'Release notes', 'Reviewed PR builds'})
        self.assertTrue(all(call.args[1] == 'pending' for call in self.api.status.call_args_list))
        self.api.snapshot.assert_not_called()
        self.api.post.assert_not_called()

    def test_open_head_listing_failure_cannot_restore_any_success_or_dispatch(self):
        self.api.pages.side_effect = RuntimeError('Temporary listing failure')
        self.gate.reconcile(self.api, 17)
        self.gate.finish(self.api, 17, (HEAD, BASE, MERGE), 'success', 'success', 'url')
        self.assertEqual({call.kwargs.get('context') for call in self.api.status.call_args_list},
                         {'Codex review', 'Release notes', 'Reviewed PR builds'})
        self.assertTrue(all(call.args[1] == 'pending' for call in self.api.status.call_args_list))
        self.api.post.assert_not_called()

    def test_sibling_opened_during_reconcile_withdraws_earlier_success_before_dispatch(self):
        sibling = dict(number=18, head=dict(sha=HEAD))
        self.api.pages.side_effect = [[], [], [], [sibling]]
        statuses = {}
        self.api.status.side_effect = lambda head, state, reason, context, url=None: statuses.update({context: state})
        self.gate.reconcile(self.api, 17)
        self.assertEqual(statuses, dict.fromkeys(('Codex review', 'Release notes', 'Reviewed PR builds'), 'pending'))
        self.api.post.assert_not_called()

    def test_sibling_opened_before_result_success_withdraws_all_approval(self):
        sibling = dict(number=18, head=dict(sha=HEAD))
        self.api.pages.side_effect = [[], [], [sibling]]
        self.gate.finish(self.api, 17, (HEAD, BASE, MERGE), 'success', 'success', 'url')
        self.assertEqual({call.kwargs.get('context') for call in self.api.status.call_args_list},
                         {'Codex review', 'Release notes', 'Reviewed PR builds'})
        self.assertTrue(all(call.args[1] == 'pending' for call in self.api.status.call_args_list))
        self.api.post.assert_not_called()

    def test_duplicate_on_second_open_pr_page_blocks_authorization(self):
        api = self.gate.GitHub('owner/repo')
        pages = [[dict(number=17, head=dict(sha=HEAD))], [dict(number=18, head=dict(sha=HEAD))]]
        with patch.object(api, 'request', return_value=pages) as request:
            with self.assertRaisesRegex(RuntimeError, 'shares.*open PR'):
                self.gate.require_unique_open_head(api, 17, HEAD)
            request.assert_called_once_with('repos/owner/repo/pulls?state=open&per_page=100', paginate=True)
            pages[1][0]['head']['sha'] = SOURCE
            self.gate.require_unique_open_head(api, 17, HEAD)

    def test_release_authorization_rejects_open_sibling_at_same_head(self):
        item = pull()
        item.update(state='closed', merged=True, merge_commit_sha=SOURCE)
        self.api.snapshot.return_value = (item, (True, 'Clean'))
        self.api.pages.return_value = [dict(number=18, head=dict(sha=HEAD), base=dict(ref='release'))]
        with self.assertRaisesRegex(RuntimeError, 'shares.*open PR'):
            self.gate.check_release(self.api, 17, HEAD, SOURCE)
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
        proof = dict(context='Reviewed PR builds', state='success', description=f'Passed {BASE} {MERGE}', target_url='https://github.com/owner/repo/actions/runs/7', creator=dict(login='github-actions[bot]', type='Bot'))
        self.api.pages.side_effect = lambda path: [proof] if path.startswith('commits/') else []
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
        proof['description'] = f'Passed {HEAD} {MERGE}'
        with self.assertRaises(RuntimeError):
            self.gate.check_release(self.api, 17, HEAD, SOURCE)


class MergedProofRaceTests(unittest.TestCase):
    def setUp(self):
        self.gate = load()
        self.api = Mock(repo='owner/repo')
        item = pull()
        item.update(state='closed', merged=True, merge_commit_sha=SOURCE)
        self.api.snapshot.return_value = (item, (True, 'Clean'))
        self.bot = dict(login='github-actions[bot]', type='Bot')
        self.race = dict(context='Reviewed PR builds', state='pending',
                         description='PR changed while reading review state', creator=self.bot)
        self.proof = dict(context='Reviewed PR builds', state='success',
                          description=f'Passed {BASE} {MERGE}', creator=self.bot,
                          target_url='https://github.com/owner/repo/actions/runs/7')
        self.statuses = [self.race, self.proof]
        self.api.pages.side_effect = lambda path: self.statuses if path.startswith('commits/') else []
        def get(path):
            if path == 'actions/runs/7':
                return dict(path='.github/workflows/reviewed-pr.yml', event='workflow_dispatch',
                            head_branch='main', head_sha=BASE, status='completed', conclusion='success',
                            display_title=f'Reviewed PR #17 @ {HEAD} + {BASE}')
            if path.startswith('compare/'):
                return dict(status='ahead')
            return dict(parents=[dict(sha=BASE), dict(sha=HEAD)], tree=dict(sha='e' * 40))
        self.api.get.side_effect = get

    def test_merge_snapshot_race_preserves_the_exact_trusted_validation_proof(self):
        self.gate.check_release(self.api, 17, HEAD, SOURCE)
        self.api.snapshot.assert_called_once_with(17, merged=True)
        self.api.post.assert_not_called()

    def test_other_invalidations_and_untrusted_race_statuses_still_refuse(self):
        for field, value in [('state', 'failure'), ('description', 'Validation failed'),
                             ('creator', dict(login='owner', type='User'))]:
            original = self.race[field]
            with self.subTest(field=field):
                self.race[field] = value
                with self.assertRaises(RuntimeError):
                    self.gate.check_release(self.api, 17, HEAD, SOURCE)
            self.race[field] = original
        self.proof['creator'] = dict(login='owner', type='User')
        with self.assertRaisesRegex(RuntimeError, 'trusted Actions identity'):
            self.gate.check_release(self.api, 17, HEAD, SOURCE)

    def test_race_does_not_hide_an_intervening_failure_or_missing_proof(self):
        self.statuses.insert(1, dict(self.proof, state='failure'))
        with self.assertRaises(RuntimeError):
            self.gate.check_release(self.api, 17, HEAD, SOURCE)
        self.statuses[:] = [self.race]
        with self.assertRaises(RuntimeError):
            self.gate.check_release(self.api, 17, HEAD, SOURCE)


class MainRepairTests(unittest.TestCase):
    def setUp(self):
        self.gate = load()
        self.api = Mock()
        self.commit = dict(message='Fix failed main build\n\nCI-Repair-Run: 42\n', parents=[dict(sha=BASE)])
        self.run = dict(path='.github/workflows/reviewed-release.yml', event='workflow_dispatch',
                        head_branch='main', head_sha=BASE, status='completed', conclusion='failure',
                        display_title=f'Reviewed release {BASE}')
        self.comparisons = {}
        self.associated = []
        self.jobs = [dict(name=name, status='completed', conclusion='success')
                     for name in ('authorize', 'Validate version, tag and source commit')]
        self.api.jobs.side_effect = lambda run_id: self.jobs
        self.api.pages.side_effect = lambda path: self.associated if path == f'commits/{SOURCE}/pulls?per_page=100' else []
        self.api.runs.return_value = []
        def get(path):
            if path == f'git/commits/{SOURCE}':
                return self.commit
            if path == 'actions/runs/42':
                return self.run
            if path.startswith('compare/'):
                return dict(status=self.comparisons.get(path, 'ahead'))
            raise AssertionError(f'Unexpected API request: {path}')
        self.api.get.side_effect = get

    def test_direct_main_repair_dispatches_without_fabricating_pr_review(self):
        self.gate.reconcile_main(self.api, SOURCE)
        self.api.post.assert_called_once_with('actions/workflows/reviewed-release.yml/dispatches',
            dict(ref='main', inputs=dict(pr_number='0', head_sha=SOURCE, source_sha=SOURCE)))
        self.gate.check_release(self.api, 0, SOURCE, SOURCE)
        self.api.snapshot.assert_not_called()
        self.api.status.assert_not_called()

    def test_direct_main_repair_dispatch_is_idempotent(self):
        self.api.runs.return_value = [dict(display_title=f'Reviewed release {SOURCE}')]
        self.gate.reconcile_main(self.api, SOURCE)
        self.api.post.assert_not_called()

    def test_failed_release_controller_can_be_a_later_main_commit(self):
        self.run['head_sha'] = HEAD
        self.gate.check_release(self.api, 0, SOURCE, SOURCE)

    def test_failed_run_must_have_successfully_authorized_and_prepared_its_source(self):
        for index in (0, 1):
            for field, value in [('status', 'queued'), ('status', 'in_progress'),
                                 ('conclusion', 'failure'), ('conclusion', 'skipped'),
                                 ('conclusion', 'cancelled'), ('conclusion', None)]:
                with self.subTest(job=self.jobs[index]['name'], field=field, value=value):
                    original = self.jobs[index][field]
                    self.jobs[index][field] = value
                    with self.assertRaisesRegex(RuntimeError, 'authorize|prepar'):
                        self.gate.check_release(self.api, 0, SOURCE, SOURCE)
                    self.jobs[index][field] = original
        for index in (0, 1):
            with self.subTest(missing=self.jobs[index]['name']):
                original = self.jobs.pop(index)
                with self.assertRaisesRegex(RuntimeError, 'authorize|prepar'):
                    self.gate.check_release(self.api, 0, SOURCE, SOURCE)
                self.jobs.insert(index, original)
        self.api.post.assert_not_called()

    def test_failed_run_job_api_errors_deny_repair(self):
        self.api.jobs.side_effect = RuntimeError('Job evidence unavailable')
        with self.assertRaisesRegex(RuntimeError, 'Job evidence unavailable'):
            self.gate.check_release(self.api, 0, SOURCE, SOURCE)
        self.api.post.assert_not_called()

    def test_failed_run_job_evidence_reads_all_pages(self):
        self.api.repo = 'owner/repo'
        self.api.request.return_value = [dict(total_count=102, jobs=[self.jobs[0]]),
                                         dict(total_count=102, jobs=[self.jobs[1]])]
        self.api.jobs.side_effect = lambda run_id: self.gate.GitHub.jobs(self.api, run_id)
        self.gate.check_release(self.api, 0, SOURCE, SOURCE)
        self.api.request.assert_called_once_with(
            'repos/owner/repo/actions/runs/42/jobs?per_page=100', paginate=True)

    def test_failed_run_rejects_ambiguous_job_evidence(self):
        self.jobs.append(dict(self.jobs[0], conclusion='failure'))
        with self.assertRaisesRegex(RuntimeError, 'authorize|prepar'):
            self.gate.check_release(self.api, 0, SOURCE, SOURCE)

    def test_repair_rejects_untrusted_or_unsuccessful_run_metadata(self):
        for field, value in [('path', '.github/workflows/reviewed-pr.yml'),
                             ('path', '.github/workflows/reviewed-release.yml@refs/heads/other'),
                             ('event', 'push'), ('head_branch', 'other'),
                             ('status', 'in_progress'), ('conclusion', 'success'),
                             ('conclusion', 'cancelled'), ('display_title', 'Unrelated run'),
                             ('display_title', f'Reviewed release {HEAD}'), ('head_sha', 'invalid')]:
            with self.subTest(field=field, value=value):
                original = self.run[field]
                self.run[field] = value
                with self.assertRaises(RuntimeError):
                    self.gate.check_release(self.api, 0, SOURCE, SOURCE)
                self.run[field] = original
        self.api.post.assert_not_called()

    def test_repair_rejects_detached_sources_and_untrusted_run_controller(self):
        for source in (SOURCE, BASE, HEAD):
            with self.subTest(source=source):
                self.run['head_sha'] = HEAD
                self.comparisons = {f'compare/{source}...main': 'diverged'}
                with self.assertRaises(RuntimeError):
                    self.gate.check_release(self.api, 0, SOURCE, SOURCE)
        self.api.post.assert_not_called()

    def test_repair_requires_exact_single_parent_and_matching_input(self):
        for parents in ([], [dict(sha=HEAD)], [dict(sha=BASE), dict(sha=HEAD)]):
            with self.subTest(parents=parents):
                self.commit['parents'] = parents
                with self.assertRaises(RuntimeError):
                    self.gate.check_release(self.api, 0, SOURCE, SOURCE)
        self.commit['parents'] = [dict(sha=BASE)]
        with self.assertRaises(RuntimeError):
            self.gate.check_release(self.api, 0, HEAD, SOURCE)

    def test_repair_trailer_cannot_bypass_a_merged_pr(self):
        for base in ('main', 'other'):
            with self.subTest(base=base):
                self.associated = [dict(number=17, merged_at='2026-10-19T10:00:00Z',
                                        base=dict(ref=base), merge_commit_sha=HEAD)]
                with self.assertRaises(RuntimeError):
                    self.gate.check_release(self.api, 0, SOURCE, SOURCE)
        self.api.snapshot.assert_not_called()
        self.api.post.assert_not_called()

    def test_repair_requires_one_final_numeric_trailer(self):
        for message in ('Fix build', 'Fix build\n\nCI-Repair-Run: nope',
                        'Fix build\n\nCI-Repair-Run: 0',
                        'Fix build\n\nCI-Repair-Run: 42\nCI-Repair-Run: 43',
                        'CI-Repair-Run: 42\n\nUnrelated final paragraph'):
            with self.subTest(message=message):
                self.commit['message'] = message
                with self.assertRaises(RuntimeError):
                    self.gate.check_release(self.api, 0, SOURCE, SOURCE)

    def test_publication_rechecks_failed_run_and_main_membership(self):
        self.gate.check_release(self.api, 0, SOURCE, SOURCE)
        self.run['conclusion'] = 'success'
        with self.assertRaises(RuntimeError):
            self.gate.check_release(self.api, 0, SOURCE, SOURCE)
        self.run['conclusion'] = 'failure'
        self.comparisons[f'compare/{SOURCE}...main'] = 'behind'
        with self.assertRaises(RuntimeError):
            self.gate.check_release(self.api, 0, SOURCE, SOURCE)


class WorkflowTrustTests(unittest.TestCase):
    def test_workload_paths_have_no_automatic_trigger(self):
        workflows = ROOT.parent / 'workflows'
        self.assertFalse((workflows / 'build.yml').exists())
        self.assertFalse((workflows / 'release.yml').exists())
        for name in ['reviewed-pr.yml', 'reviewed-release.yml', '_reviewed-build.yml', '_reviewed-native.yml']:
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
        for denied in ['actions: write', 'secrets.', 'inputs.merge_sha', 'review_gate.py reconcile', 'workflow_run:', 'branches: [main]']:
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
        self.assertEqual(re.findall(r'^  ([a-z0-9_-]+):$', source.split('jobs:')[1], re.M),
                         ['authorize', 'tooling'])
        for job in ['tooling']:
            block = re.search(rf'^  {job}:\n(.*?)(?=^  [a-z0-9_-]+:|\Z)', source, re.M | re.S)[1]
            self.assertIn('needs: authorize', block)
            self.assertIn("if: needs.authorize.outputs.ready == 'true'", block)
            self.assertIn('ref: ${{ inputs.merge_sha }}', block)
            self.assertIn('persist-credentials: false', block)

    def test_main_native_validation_requires_release_authorization_and_exact_source(self):
        import re
        source = (ROOT.parent / 'workflows/_reviewed-native.yml').read_text()
        self.assertIn("github.ref == 'refs/heads/main'", source)
        self.assertIn("github.workflow_ref == format('{0}/.github/workflows/reviewed-release.yml@refs/heads/main', github.repository)", source)
        self.assertIn('review_gate.py release-check', source)
        for denied in ('secrets.', 'rust-cache', 'cache:', 'cargo test', 'cargo fmt', 'cargo clippy'):
            self.assertNotIn(denied, source)
        for job in ('desktop', 'android'):
            block = re.search(rf'^  {job}:\n(.*?)(?=^  [a-z0-9_-]+:|\Z)', source, re.M | re.S)[1]
            self.assertIn('needs: authorize', block)
            self.assertIn("if: needs.authorize.outputs.ready == 'true'", block)
            self.assertIn('ref: ${{ inputs.source_sha }}', block)
            self.assertIn('persist-credentials: false', block)
        release = (ROOT.parent / 'workflows/reviewed-release.yml').read_text()
        native = re.search(r'^  native-validation:\n(.*?)(?=^  [a-z0-9_-]+:|\Z)', release, re.M | re.S)[1]
        self.assertIn('needs: [authorize, prepare]', native)
        self.assertIn("if: needs.prepare.outputs.skip != 'true'", native)
        self.assertIn('uses: ./.github/workflows/_reviewed-native.yml', native)
        self.assertIn('source_sha: ${{ inputs.source_sha }}', native)
        publish = re.search(r'^  publish:\n(.*?)(?=^  [a-z0-9_-]+:|\Z)', release, re.M | re.S)[1]
        self.assertRegex(publish, r'needs: \[[^\]\n]*native-validation[^\]\n]*\]')

    def test_native_desktop_jobs_install_their_toolchains_before_building(self):
        import re
        for workflow, jobs in [('_reviewed-native.yml', ('desktop',)), ('reviewed-release.yml', ('desktop',))]:
            source = (ROOT.parent / 'workflows' / workflow).read_text()
            for job in jobs:
                with self.subTest(workflow=workflow, job=job):
                    block = re.search(rf'^  {job}:\n(.*?)(?=^  [a-z0-9_-]+:|\Z)', source, re.M | re.S)[1]
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
                    windows = selected('choco install nasm' if workflow == 'reviewed-release.yml' else 'choco install ffmpeg nasm', "if: runner.os == 'Windows'")
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
        for workflow, job in [('_reviewed-native.yml', 'android'), ('reviewed-release.yml', 'android'), ('reviewed-release.yml', 'ios')]:
            with self.subTest(workflow=workflow, job=job):
                source = (ROOT.parent / 'workflows' / workflow).read_text()
                block = re.search(rf'^  {job}:\n(.*?)(?=^  [a-z0-9_-]+:|\Z)', source, re.M | re.S)[1]
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
        block = re.search(r'^  tooling:\n(.*?)(?=^  [a-z0-9_-]+:|\Z)', source, re.M | re.S)[1]
        self.assertEqual(block.count("if: hashFiles('mkdocs.yml') != ''"), 4)
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
