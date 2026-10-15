"""Public notes validation and complete first-parent release ranges."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "release_notes.py"


def load_notes():
    if not SCRIPT.exists():
        raise AssertionError("Release note collection is not implemented")
    spec = importlib.util.spec_from_file_location("release_notes", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class ValidationTests(unittest.TestCase):
    def setUp(self):
        self.notes = load_notes()

    def test_preserves_public_markdown_without_validation_or_html_comments(self):
        body = '## Release notes\n### Fixed\n- Preserve **multi-value** tags.\n<!-- private -->\n## Validation\nTests passed.'
        self.assertEqual(self.notes.validate_release_notes(body), '### Fixed\n- Preserve **multi-value** tags.')

    def test_missing_empty_vague_and_placeholder_notes_fail(self):
        for body in ('', '## Release notes\n- ', '## Release notes\nTODO', '## Release notes\nN/A',
                     '## Release notes\n[Describe the changes]', '## Release notes\nNo user-visible changes.',
                     '## Release notes\nNo user-visible changes.\n### Internal\nMaintenance'):
            with self.subTest(body=body), self.assertRaises(ValueError):
                self.notes.validate_release_notes(body)

    def test_concrete_internal_notes_are_accepted(self):
        for explanation in ('### Internal\n- Validate macOS document associations in CI.',
                            '**Internal:** Pin the documentation toolchain.'):
            self.notes.validate_release_notes(f'## Release notes\nNo user-visible changes.\n{explanation}')

    def test_fake_heading_in_code_is_not_a_release_section(self):
        with self.assertRaises(ValueError):
            self.notes.validate_release_notes('```markdown\n## Release notes\nFixed tags.\n```')

    def test_substantive_markdown_links_are_not_template_placeholders(self):
        for label in ('Add cover-art support', 'Replace saved covers', 'Write tag values',
                      'Describe the changes'):
            with self.subTest(label=label):
                body = f'## Release notes\n- [{label}](https://example.invalid/change)'
                self.assertEqual(self.notes.validate_release_notes(body), body.split('\n', 1)[1])
        self.notes.validate_release_notes('## Release notes\n- [Add cover-art support][covers]\n\n[covers]: https://example.invalid/covers')

    def test_known_bracket_and_angle_template_placeholders_still_fail(self):
        for placeholder in ('[Describe the changes]', '<Insert release notes here>',
                            '[Replace with actual changes]', '[Write your release notes]'):
            with self.subTest(placeholder=placeholder), self.assertRaisesRegex(ValueError, 'template placeholders'):
                self.notes.validate_release_notes(f'## Release notes\n- {placeholder}')

    def test_literal_todo_and_tbd_values_in_prose_and_examples_are_accepted(self):
        for text in ('- Preserve TODO values in title tags.', '- Write the literal `TBD` album name.',
                     '- Retain placeholder-like title values.\n```text\nTODO\nTBD\n[Describe the changes]\n```'):
            with self.subTest(text=text):
                self.assertEqual(self.notes.validate_release_notes('## Release notes\n' + text), text)

    def test_placeholder_only_todo_and_tbd_lines_still_fail(self):
        for placeholder in ('TODO', '- TBD', '**TODO**', '[TBD]', '<TODO>'):
            with self.subTest(placeholder=placeholder), self.assertRaises(ValueError):
                self.notes.validate_release_notes('## Release notes\n' + placeholder)

    def test_only_level_two_heading_satisfies_dedicated_release_section(self):
        for depth in (1, 3, 4, 5, 6):
            body = f'## Validation\n{"#" * depth} Release notes\n- Validate cover-art fixtures.'
            with self.subTest(depth=depth), self.assertRaisesRegex(ValueError, '## Release notes'):
                self.notes.validate_release_notes(body)
        body = ('## Validation\n### Release notes\n- Private test result.\n'
                '## Release notes\n- Preserve album covers.\n## Validation\n- More tests.')
        self.assertEqual(self.notes.validate_release_notes(body), '- Preserve album covers.')


class GenerationTests(unittest.TestCase):
    def setUp(self):
        self.notes = load_notes()

    def test_collects_notes_once_for_merged_prs_and_retains_literal_text(self):
        commits = [self.notes.Commit('a' * 40, 'Merge changes', ''), self.notes.Commit('b' * 40, 'Merge again', '')]
        pr = dict(number=17, title='Retain cover art', merged_at='2026-10-15T10:00:00Z',
                  body='## Release notes\n### Fixed\n- Keep `$(touch unsafe)` literal.\n## Validation\nPrivate test result.')
        result = self.notes.render_release_notes('owner/repo', commits, lambda sha: [pr], {item.sha for item in commits})
        self.assertEqual(result.count('/pull/17'), 1)
        self.assertIn('$(touch unsafe)', result)
        self.assertIn('#### Fixed', result)
        self.assertNotIn('Private test', result)

    def test_new_pr_missing_notes_fails_instead_of_falling_back_to_title(self):
        commit = self.notes.Commit('a' * 40, 'Merge new PR', '')
        pr = dict(number=18, title='Fix tags', body='## Summary\nFix tags.', merged_at='now')
        with self.assertRaisesRegex(ValueError, '#18'):
            self.notes.render_release_notes('owner/repo', [commit], lambda sha: [pr], {commit.sha})

    def test_nested_validation_notes_cannot_be_published_as_a_dedicated_section(self):
        commit = self.notes.Commit('a' * 40, 'Merge PR', '')
        pull = dict(number=18, title='Fix tags', merged_at='now',
                    body='## Validation\n### Release notes\n- Private cover-art test result.')
        with self.assertRaisesRegex(ValueError, '#18'):
            self.notes.render_release_notes('owner/repo', [commit], lambda sha: [pull], {commit.sha})
        historical = self.notes.render_release_notes('owner/repo', [commit], lambda sha: [pull], set())
        self.assertNotIn('Private cover-art test result', historical)

    def test_historical_pr_and_direct_commit_keep_sanitized_explanation(self):
        commits = [self.notes.Commit('a' * 40, 'Merge old PR', ''), self.notes.Commit('b' * 40, 'Repair tags', 'Retain original cover values.')]
        historical = dict(number=1, title='Old change', body='## Summary\nKeep the selected album.\n## Validation\nTests passed.', merged_at='now')
        result = self.notes.render_release_notes('owner/repo', commits, lambda sha: [historical] if sha.startswith('a') else [], set())
        self.assertIn('Keep the selected album.', result)
        self.assertIn('Retain original cover values.', result)
        self.assertIn('/commit/' + 'b' * 40, result)
        self.assertNotIn('Tests passed', result)

    def test_legacy_placeholder_section_does_not_apply_post_baseline_validation(self):
        commit = self.notes.Commit('a' * 40, 'Merge old PR', '')
        for legacy in ('N/A', 'TODO', '[Describe the changes]'):
            with self.subTest(legacy=legacy):
                historical = dict(number=1, title='Old change', merged_at='now',
                                  body=f'## Summary\nKeep the selected album.\n## Release notes\n{legacy}\n## Validation\nPrivate results.')
                result = self.notes.render_release_notes('owner/repo', [commit], lambda sha: [historical], set())
                self.assertIn('Keep the selected album.', result)
                self.assertNotIn('Private results', result)
                self.assertNotIn(legacy, result)
                self.assertNotIn('Release notes', result)
                with self.assertRaises(ValueError):
                    self.notes.render_release_notes('owner/repo', [commit], lambda sha: [historical], {commit.sha})

    def test_historical_substantive_notes_and_literal_examples_remain_published(self):
        commit = self.notes.Commit('a' * 40, 'Merge old PR', '')
        body = ('## Summary\nKeep the selected album.\n## Release notes\n'
                '- Preserve TODO title values.\n```text\nTODO\n```\n## Validation\nPrivate results.')
        pull = dict(number=1, title='Old change', merged_at='now', body=body)
        result = self.notes.render_release_notes('owner/repo', [commit], lambda sha: [pull], set())
        self.assertIn('Keep the selected album.', result)
        self.assertIn('Preserve TODO title values.', result)
        self.assertIn('```text\nTODO\n```', result)
        self.assertNotIn('Private results', result)

    def test_first_parent_range_keeps_docs_only_changes_and_excludes_feature_commits(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                return subprocess.run(['git', '-C', str(root), *args], check=True, capture_output=True, text=True).stdout.strip()
            git('init', '-q', '-b', 'main')
            git('config', 'user.name', 'Test')
            git('config', 'user.email', 'test@example.invalid')
            git('commit', '--allow-empty', '-qm', 'Published')
            baseline = git('rev-parse', 'HEAD')
            git('tag', 'v0.2.0')
            git('commit', '--allow-empty', '-qm', 'Docs only')
            git('checkout', '-qb', 'feature')
            git('commit', '--allow-empty', '-qm', 'Feature implementation detail')
            git('checkout', '-q', 'main')
            git('merge', '--no-ff', '-qm', 'Merge feature', 'feature')
            commits = self.notes.collect_commits(root, git('rev-parse', 'HEAD'), 'v0.2.0')
            self.assertEqual([item.subject for item in commits], ['Docs only', 'Merge feature'])
            self.assertEqual(self.notes.required_note_commits(root, git('rev-parse', 'HEAD'), baseline), {item.sha for item in commits})
            with self.assertRaises(ValueError):
                self.notes.collect_commits(root, git('rev-parse', 'HEAD'), 'feature')

    def test_validate_cli_uses_event_body_as_data(self):
        with tempfile.TemporaryDirectory() as directory:
            event = Path(directory) / 'event.json'
            for body, status in [('## Release notes\n- Retain covers after saving.', 0), ('## Summary\nChange tags.', 1)]:
                event.write_text(json.dumps(dict(pull_request=dict(body=body))))
                result = subprocess.run([sys.executable, str(SCRIPT), 'validate', '--event-file', str(event)], capture_output=True)
                self.assertEqual(result.returncode, status, result.stderr)

    def test_generation_cli_keeps_pr_notes_literal_and_api_failures_are_fatal(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                return subprocess.run(['git', '-C', str(root), *args], check=True, capture_output=True, text=True).stdout.strip()
            git('init', '-q', '-b', 'main')
            git('config', 'user.name', 'Test')
            git('config', 'user.email', 'test@example.invalid')
            git('commit', '--allow-empty', '-qm', 'Published')
            baseline = git('rev-parse', 'HEAD')
            git('tag', 'v0.2.0')
            git('commit', '--allow-empty', '-qm', 'Merge documentation')
            (root / '.github').mkdir()
            (root / '.github/release-notes-baseline').write_text(baseline)
            marker = root / 'must-not-exist'
            pull = dict(number=30, title='Explain covers', merged_at='now',
                        body=f'## Release notes\nInternal: Explain the literal `$(touch {marker})` cover example.\n## Validation\nPrivate test logs.')
            gh = root / 'gh'
            gh.write_text(f'#!{sys.executable}\nimport json, os, sys\nif os.environ.get("FAIL_GH"): sys.exit(1)\nprint(json.dumps([[json.loads(os.environ["PULL_JSON"])]]))\n')
            gh.chmod(0o755)
            env = dict(os.environ, PATH=str(root) + os.pathsep + os.environ['PATH'], PULL_JSON=json.dumps(pull))
            command = [sys.executable, str(SCRIPT), 'generate', '--checkout', str(root), '--repo', 'owner/repo',
                       '--sha', git('rev-parse', 'HEAD'), '--previous-tag', 'v0.2.0']
            result = subprocess.run(command, env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn('$(touch', result.stdout)
            self.assertIn('/pull/30', result.stdout)
            self.assertNotIn('Private test logs', result.stdout)
            self.assertFalse(marker.exists())
            env['FAIL_GH'] = '1'
            result = subprocess.run(command, env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 1)
            self.assertNotIn('Merge documentation', result.stdout)


if __name__ == '__main__':
    unittest.main()
