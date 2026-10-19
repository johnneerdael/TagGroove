# Review-first CI/CD containment

The legacy `build.yml` and `release.yml` workflows were deleted from `main` in emergency commit `743996fc0eb859d430fc84910b774b1b70f64586`. Keep their old workflow IDs disabled: older PR branches still contain automatic triggers. Keep the separate legacy Docs, Docs screenshots and CI tooling workflows disabled until their execution is routed through the reviewed controller. Do not blanket-enable repository workflows.

Only trusted metadata authorization can run before review. `review-controller.yml` checks out `main`, reads GitHub reviews/threads/reactions, cancels stale validation and dispatches `reviewed-pr.yml` **on main only** after an authenticated Codex code review of the final full head SHA completes with no findings and approval. Human, local and security-only reviews cannot substitute. No timeout approves a PR.

Approval accepts the connector's issue `+1` at or after the latest qualifying code-review completion. GitHub [returns an existing reaction for a repeated create request](https://docs.github.com/en/rest/reactions/reactions#create-reaction-for-an-issue), retaining its original creation timestamp. A retained connector `+1` therefore also qualifies with an authenticated connector Bot's explicit `<!-- codex-review-receipt:v1 -->` comment containing recognized clean code-review text, a valid creation timestamp and a reviewed commit resolved to the exact current full head SHA. This receipt must be at least as recent as every qualifying completion and contain no security-review text. A retained reaction alone, generic completion or unstructured summary cannot use this path. New or equally recent requests/findings, unresolved threads, pending reviews and all other blockers remain enforced; a later ambiguous completion cannot borrow an earlier receipt's approval.

The dispatched workflow repeats authorization against the exact current main/head/test-merge triple. Its reusable build independently authenticates the calling main workflow and repeats the same read-only preflight before any application code, dependencies or tests run. PR code receives no repository signing secrets, checkout credentials or shared build caches. Successful Linux unit, lint, tooling and documentation checks receive `Reviewed PR builds` proof bound to the reviewed head, current base and immutable test merge; every required workload must succeed.

Conflicting PRs and pending test-merge calculations remain blocked without dispatching application jobs. When the reviewed source contains `mkdocs.yml`, validation also runs the pinned guide toolchain, publication tests, writing-plan check and strict documentation build. This build has no Pages upload or deployment permissions.

On a main push, `reviewed-release.yml` is dispatched only for a uniquely associated merged PR with clean final-head Codex review and successful trusted validation proof. The published source must have the exact validated parents and tree. The release workflow verifies this before building/signing and again before publication. Owner-authorized direct-main failed-build repairs use a distinct path: a single-parent commit directly after the failed release source must carry `CI-Repair-Run: <failed-run-id>`. The gate requires that exact completed failed trusted main release run with successful authorization and preparation, main ancestry and no associated merged PR, and rechecks this evidence before publication. This does not supply PR review proof. Direct main containment/bootstrap commits without this evidence authorize no release. Existing platform, version, signature, checksum and complete-asset checks remain in the new release workflow.

Enable only the controller, metadata signal, review-metadata invalidation, reviewed PR workflow, reusable pre-merge checks, post-merge native integration and reviewed release paths after installation. Require `Codex review` and `Reviewed PR builds` plus applicable checks through main branch protection/rulesets. Run a manual metadata recheck with:

```sh
gh workflow run review-controller.yml --ref main -f pr_number=17
```

An actual failed build requires an explicit `retry_build=true` dispatch after investigating it. API failures and stale heads deny application execution. Reactions have no direct Actions event and nothing polls for them: the controller has no schedule and does not run on PR pushes or comments. It runs when a review is posted or dismissed, when a PR description is edited (the release notes are checked), when a validation it dispatched finishes, on a main push, or when you dispatch it for one PR. The separate `review-metadata-invalidation.yml` job checks recognized code-review commands (including trailing instructions), reviewer/team requests, draft conversion and connector comments against trusted `main` metadata. It withdraws stale review and build statuses, including after an edited request or failed metadata read, but cannot dispatch application builds or restore validation success. Ordinary PR discussion skips that job. Opening or reopening a PR, changing its head, marking it ready, or changing its base resets all three commit statuses to pending to withdraw approval and validation inherited from another PR. Base changes run only this invalidation path; dispatch the controller for a fresh review, notes and build check. Dispatch the controller once a PR has a clean Codex review and again immediately before merging, then wait for `Reviewed PR builds`. No Pages/site or release-note changes are included in this emergency bootstrap. Rebase later workflow PRs onto these paths so they cannot restore automatic execution.

Local checks:

```sh
python3 -m unittest discover -s .github/scripts/tests -v
node --test tests/release.test.mjs
actionlint -ignore 'unexpected key "queue" for "concurrency" section' .github/workflows/*.yml
git diff --check
```

Commit statuses are shared by SHA. Withhold review, release-note and build success while another open PR anywhere in the repository has the exact same head, including drafts and PRs targeting another base. Check the paginated live open-PR list before reconciliation success, validation authorization/completion and release authorization; API failures deny approval. Close one PR or advance one head before rechecking. Lifecycle invalidation watches all base branches so opening a sibling PR also withdraws inherited statuses. These live checks are not atomic PR identity isolation; repeat the live merge-readiness check immediately before merging.

## User guide publication

Reviewed PR validation includes strict documentation checks whenever the
reviewed source contains `mkdocs.yml`. After merging an approved and validated
source, `reviewed-release.yml` calls `_reviewed-pages.yml` independently of
application-release skipping. The reusable workflow authenticates its trusted
main caller and checks the merged source proof before building and again before
publication. Only the trusted deployment job receives Pages/OIDC permissions
in the `github-pages` environment, and it deploys the build job's Pages
artifact for `taggroove.neerdael.nl` only while the approved source is still
current main. No downloaded artifact is executed, no deployment key, PAT or
broad secret inheritance is used, and no new automatic source trigger or
schedule is added. See `docs/PUBLISHING.md` for setup and recovery.
The former automatic docs and screenshot workflow files are removed; their
legacy IDs remain disabled.

## Test and build execution split

After clean review, `_reviewed-build.yml` runs unit tests, TypeScript checks, formatting, Clippy, tooling and strict guide checks on one Linux worker. It produces no application packages and runs no native UI or Android integration. The legacy `Reviewed PR builds` status name remains for protection compatibility.

After merge, `reviewed-release.yml` calls `_reviewed-native.yml` for desktop native UI, screenshots and Android integration, alongside release packaging. That reusable workflow authenticates its trusted main caller and release source before executing code, receives no signing secrets or shared caches, and checks out the immutable release source. Its debug harness uses a test identity; it is separate from the shipped package. Publication requires successful native integration as well as every package and source archive. Unit tests and lint run only before merge. Docs-only releases that resolve to an already published version skip both native integration and package builds; guide rendering/deployment remains independent.
