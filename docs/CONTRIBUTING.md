# Contributing to OpenTag

Open changes through a feature branch and pull request. The PR template separates public release notes, validation and documentation impact. Keep all three aligned with the final implementation.

## Review and merge

Request `@codex review` on the PR's final commit. Address findings, resolve their review threads and request a new code review after the last change. Merge only after that exact head has a clean GitHub Codex code review and all required build, native UI, Android, tooling and documentation checks have passed. A local review or a security-only review does not satisfy the code-review requirement.

The trusted `PR review gate` workflow publishes two commit statuses:

| Required status | Meaning |
| --- | --- |
| `Codex review` | The authenticated Codex connector completed a code review of the current full head SHA, approved it with a sufficiently recent `+1`, and no outstanding findings, review requests or requested changes remain. |
| `Release notes` | The current PR body contains a substantive public release-note section. |

The gate reads GitHub's full review/comment/thread history. A short displayed SHA must resolve to the exact full commit; a thumbs-up without revision metadata is insufficient. Resolving an earlier finding does not substitute for a newer clean review. New pushes invalidate the review, and a newly edited review request requires a later completion. API failures keep the gate pending.

Review events use an unprivileged relay; a separate controller checks out only trusted `main`, reads live PR state and posts statuses. It never runs PR code or receives application signing secrets. Reactions have no Actions event, so the controller also reconciles every five minutes. GitHub may delay scheduled workflows; use the manual workflow input to recheck a PR when needed:

```sh
gh workflow run review-gate.yml --ref main -f pr_number=42
```

### Repository setup and bootstrap

After the workflow is merged to `main`, enable a ruleset or branch protection requiring `Codex review`, `Release notes`, the CI tooling check and the existing build checks before merging. Require PRs, block force pushes/deletion and keep any bypass restricted to intentional repository recovery. A green optional status does not enforce a merge policy.

The PR introducing this controller cannot execute its new trusted code until it lands. Verify its final-head Codex review and build checks manually, merge it, dispatch the controller, verify both statuses on an open PR and then configure the required statuses. No Codex API key is needed: the installed GitHub connector performs reviews; the controller uses the workflow token with read access to PRs/issues and write access to commit statuses.

## Public notes and documentation

Write `## Release notes` for people using the app: what changes, the practical effect and relevant limits. For example, “Filename → Tag skips non-matching files while importing matching files in the selection” is more useful than “Fix converter.” For internal work, name the actual change: “Internal: Validate documentation links before publication.” Empty sections, `TODO`, `N/A`, vague maintenance text and a bare “No user-visible changes” fail validation.

Put test commands, measured results, coverage gaps and review evidence in `## Validation`. Update affected user-guide pages and explain the documentation impact in the PR. A strict MkDocs build catches broken links, anchors and navigation; reviewers still check whether instructions match implemented behavior, screenshots remain useful and platform limitations are accurate. Publication requires a successful Pages deployment and a working public site.

The release publisher collects each merged PR's public notes since the previous published release, including intervening docs-only merges. It excludes draft/unpublished tags and fails on unavailable metadata or missing newly required notes. Historical PRs at or before `.github/release-notes-baseline` retain their sanitized descriptions; later merges require the dedicated section. Tests, badges and agent transcripts belong outside that section. See [RELEASING.md](RELEASING.md) for artifact and publication safeguards.

## Validation

For application work, run `npm test` and `npm run build`, plus the relevant native UI/platform suites. For CI and release-tooling changes, also run:

```sh
python3 -m unittest discover -s .github/scripts/tests -v
node --test tests/release.test.mjs
actionlint -ignore 'unexpected key "queue" for "concurrency" section' .github/workflows/review-gate.yml .github/workflows/review-signal.yml .github/workflows/ci-tooling.yml .github/workflows/release.yml
git diff --check
```

Python 3.10 or newer is required for release-note tooling; CI pins Python 3.12. The actionlint exception covers the documented concurrency queue key, unsupported by actionlint 1.7.12. Do not suppress other workflow errors.
