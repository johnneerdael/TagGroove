# Publish and maintain the user guide

The public guide is <https://johnneerdael.github.io/opentag/>. Its source is
`docs/user-guide/`, and `mkdocs.yml` defines the public navigation and theme.

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
   building the site. The deployment job checks them again, then publishes
   the built artifact through the `github-pages` environment only if the
   source is still current main.
5. Check the deployment job and its environment URL, then open the public
   site. Confirm the changed page, its navigation entry, search, and an image.
   A successful local build or uploaded artifact is not proof of publication.

The repository's Pages source must be **GitHub Actions** under
**Settings → Pages**. The `github-pages` environment should permit only
`main`. Keep legacy build, release, docs and screenshot workflow IDs disabled;
never enable them to recover a deployment. Read `.github/CI_LOCKDOWN.md` for
review-controller operations and the validation retry procedure.

If the site is missing, first check that the docs files and reviewed Pages
workflow are actually merged to `main`, then inspect the approved source's
**Reviewed main release → user-guide** jobs. A feature branch cannot publish.
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
