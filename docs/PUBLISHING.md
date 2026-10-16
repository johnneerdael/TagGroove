# Publish and maintain the user guide

The public guide is <https://johnneerdael.github.io/audio-tag-guide/>. Its source is
`docs/user-guide/`, and `mkdocs.yml` defines the public navigation and theme.
Keep authoring in `johnneerdael/opentag`. The separate public
`johnneerdael/audio-tag-guide` repository contains generated static output and
one reviewed deployment workflow.
The neutral publication path avoids privacy-list URL rules that also match
the project's name. Readers do not need to disable their privacy protection.
The header keeps ordinary source navigation while omitting optional automatic
repository-statistics/release-badge requests. Source, edit and release links
retain their normal literal destinations; no proxy or encoded URL is used.

## Write and validate a page

Read `docs/user-guide/_meta/style.md` and the topic's brief before writing.
Keep instructions specific to OpenTag's actual behavior. Check the relevant
source and examples; record runtime checks separately from source or test
evidence. Reuse existing verified guide images where appropriate. New app
captures are pending review-gated CI support; follow the screenshot section
below rather than launching the app against a personal music library.

The complete writing plan lives in `docs/user-guide/_meta/planned-nav.yml`.
A page marked `status: draft` stays in the repository but is excluded from
the built site and search by `docs/hooks/publication.py`.

To publish a topic, finish its instructions and examples, remove
`status: draft`, and add it to the public `nav` in `mkdocs.yml`. Link only to
published pages. A completed page missing from navigation, a navigation
entry pointing to a draft, or a published link pointing to a draft fails
the strict build.

From the repository root, create a virtual environment and install the
pinned documentation toolchain:

```sh
python3 -m venv /tmp/opentag-docs
/tmp/opentag-docs/bin/python -m pip install -r docs/site-requirements.txt
/tmp/opentag-docs/bin/python -m unittest discover -s docs/tests -v
/tmp/opentag-docs/bin/python -m mkdocs build --strict
python3 scripts/docs-status.py
cargo test -p tag-core --test user_guide --locked
```

On Windows, create the environment outside the checkout, for example with
`python -m venv "$env:TEMP\opentag-docs"` in PowerShell, then use
`& "$env:TEMP\opentag-docs\Scripts\python.exe"` for the commands above.
Preview with `/tmp/opentag-docs/bin/python -m mkdocs serve`, then check both color
modes, narrow screens, navigation, search, screenshots and copyable examples.
The generated site is `build/user-guide-site/`; do not commit that output.
The Rust documentation tests execute the published regex tables and selected
format-string, date and recipe examples against OpenTag's core. They require
the development prerequisites from README; they do not establish Mp3tag
runtime parity.

Run `python3 scripts/docs-status.py --drafts` to find unfinished pages, or
`--coverage` to compare the plan with the original feature checklist.
Coverage is a writing plan, not a feature-parity claim.

## Review and publication

1. Open a pull request. Include user-visible documentation changes in its
   **Release notes** section and the checks performed in **Validation**.
2. Obtain a clean Codex review of the final head commit. The trusted review
   controller then authorizes **Reviewed PR validation** for the exact head,
   current main and immutable test merge. Its documentation workload runs
   the publication tests, writing-plan check and strict build. A new commit
   requires a fresh review. PR validation cannot deploy Pages.
3. Merge only after **Codex review** and **Reviewed PR builds** pass for that
   revision. The controller authorizes **Reviewed main release** from the
   approved merged tree. Its **user-guide** job runs independently of the
   application release, including when application packaging is skipped for
   a documentation-only change.
4. The guide checks the merged-PR review and trusted validation proof before
   building the canonical site. The build uploads static output and a separate
   old-site redirect artifact. It receives no deployment key or write permission.
5. The deployment job checks approval and current main again. Trusted main's
   `.github/scripts/guide_pages.py` validates regular static files, matching
   source/review provenance and a content digest before committing the output
   to the fixed neutral repository. It rejects repository-control directories,
   symlinks, hardlinks, custom-domain overrides and leaked writer-only material.
   The output replaces only `site/`; the separately installed target workflow must
   match the reviewed source template byte for byte. No artifact code runs.
6. The target workflow checks the source repository's merged-PR review, trusted
   validation proof, current main and static-tree digest using trusted source
   main code before uploading a Pages artifact. Its read-only package job must
   succeed before a separate Pages/OIDC deployment job can start; that job
   repeats the source check before deployment.
7. The source job waits for the neutral site's HTTPS
   `publication-source.json` to match
   the expected source, review and complete static-tree digest. Only then does
   it recheck approval/current main and deploy old-site redirects through the
   existing Pages action. A missing/stale marker or source drift prevents that
   redirect deployment; a concurrent target update is never force-pushed away.
   The marker wait uses an eleven-minute monotonic deadline within the source
   deployment job's thirty-minute limit.
8. Open the canonical public site. Confirm the changed page, navigation,
   search, styles and an image. Also check an old deep link with its query and
   fragment. A local build, Git push or uploaded artifact is not publication.

The source repository's Pages source remains **GitHub Actions** under
**Settings → Pages**. It serves small redirect pages for every built HTML route
and a dependency-free 404 fallback. Inline navigation preserves the old path,
query and fragment under the fixed canonical host/base; with JavaScript off,
the plain link still provides a route to the guide. No external stylesheet or
script is required for these redirects.

The neutral repository uses **GitHub Actions** as its Pages source. After the
source change has passed review and validation and merged normally, an
administrator installs the exact reviewed `.github/guide-pages-target.yml` from
source main as `.github/workflows/static-pages.yml` in the target repository.
Source main must already contain the `verify-target` command. Configure the
target `github-pages` environment to allow only `main`, then select **GitHub
Actions** under the target's **Settings → Pages**. Keep the existing public
site until the first authorized target deployment succeeds.

The target workflow runs only on changes under `site/` on `main`, so installing
its control file alone does not start a deployment. The source publisher's next
approved run creates or replaces `site/` and triggers that workflow over SSH.
The publisher fails closed if the control file is missing, altered or accompanied
by another file under `.github`; it cannot stage or replace that control file.
Target uploads contain only `site/`, excluding the control workflow and any old
bootstrap files at the repository root. Do not add application source or
credentials to the target repository. This route uses ordinary push-triggered
Actions because GitHub documents that deploy-key pushes can fail to trigger
[branch-based Pages publication](https://docs.github.com/en/pages/setting-up-a-github-pages-site-with-jekyll/about-jekyll-build-errors-for-github-pages-sites).
No PAT or scheduled target publication is needed. GitHub describes the separate
artifact and deployment jobs in its [custom Pages workflow guidance](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages).

The source `github-pages` environment should permit only `main` and hold
`DOCS_PAGES_DEPLOY_KEY`. The corresponding public deploy key grants write access
only to `johnneerdael/audio-tag-guide`. The caller passes that secret's name
explicitly; the actual value comes from the deployment job's environment.
GitHub's [reusable-workflow secret rules](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows)
require the explicit mapping even for an environment-only secret. No broad
secret inheritance is used. Only the trusted publisher step consumes the key;
read/build and marker-wait steps do not receive it. The key is written to a
temporary mode-0600 file, excluded from Git's environment and removed on exit.
SSH checks the pinned public GitHub Ed25519 host key in
`.github/guide-pages-known-hosts`; verify any rotation against
[GitHub's published fingerprints](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/githubs-ssh-key-fingerprints)
through a reviewed source change rather than disabling host verification.

Keep legacy build, release, docs and screenshot workflow IDs disabled;
never enable them to recover a deployment. Read `.github/CI_LOCKDOWN.md` for
review-controller operations and the validation retry procedure.

If the site is missing, first check that the docs files and reviewed Pages
workflow are actually merged to `main`, then inspect the approved source's
**Reviewed main release → user-guide** jobs. A feature branch cannot publish.
Check the neutral repository's **Deploy reviewed static guide** workflow and
compare its live publication marker with the approved source. An SSH failure needs deployment
key/host-key investigation; a marker timeout can mean the target publication
failed or has not reached the public site. Do not deploy redirects to conceal
that failure, enable legacy workflows or widen token permissions.
To retry publication, rerun the approved release workflow after resolving
its failure. Its authorization checks still apply; source or review drift
requires fresh validation. To roll back content, submit a revert PR through
that same review and validation process.

Existing guide images are retained. The former automatic **Docs screenshots**
workflow is removed; do not restore its triggers. New app-image capture must
be routed through the review gate before it can run in CI.

Structural checks cannot establish complete behavioral accuracy. The selected
Rust example checks cover core behavior; update affected instructions alongside
feature changes, and review screenshots when the interface changes. Complete
coverage of the original documentation remains a separate writing and
verification task, tracked in the writing plan.
