# Publish and maintain the user guide

The public guide is <https://johnneerdael.github.io/opentag/>. Its source is
`docs/user-guide/`, and `mkdocs.yml` defines the public navigation and theme.

## Write and validate a page

Read `docs/user-guide/_meta/style.md` and the topic's brief before writing.
Keep instructions specific to OpenTag's actual behavior. Check the relevant
source and examples; record runtime checks separately from source or test
evidence. Use the screenshot workflow for app images rather than a personal
music library.

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
```

On Windows, create the environment outside the checkout, for example with
`python -m venv "$env:TEMP\opentag-docs"` in PowerShell, then use
`& "$env:TEMP\opentag-docs\Scripts\python.exe"` for the commands above.
Preview with `/tmp/opentag-docs/bin/python -m mkdocs serve`, then check both color
modes, narrow screens, navigation, search, screenshots and copyable examples.
The generated site is `build/user-guide-site/`; do not commit that output.

Run `python3 scripts/docs-status.py --drafts` to find unfinished pages, or
`--coverage` to compare the plan with the original feature checklist.
Coverage is a writing plan, not a feature-parity claim.

## Review and publication

1. Open a pull request. Include user-visible documentation changes in its
   **Release notes** section and the checks performed in **Validation**.
2. Require a Codex review with no outstanding findings, plus passing checks,
   before merging. The **User guide** workflow builds pull requests with no
   Pages deployment permissions.
3. After merging to `main`, the same strict build creates a Pages artifact.
   **Publish user guide** deploys it through the `github-pages` environment.
   Feature-branch and pull-request builds cannot deploy.
4. Check the deployment job and its environment URL, then open the public
   site. Confirm the changed page, its navigation entry, search, and an image.
   A successful local build is not proof of publication.

The repository's Pages source must be **GitHub Actions** under
**Settings → Pages**. The `github-pages` environment should permit only
`main`. A skipped older revision avoids replacing a newer `main` deployment.

If the site is missing, first check that the docs files and workflows are
actually merged to `main`, then inspect **Actions → User guide**. A green
feature-branch build does not publish the site. To rebuild or recover a
deployment, run **User guide → Run workflow** with branch **main**. To roll
back bad content, revert the docs change on `main`; the workflow rebuilds it.

The workflow also rebuilds when application source changes, allowing stale
links and examples to be caught during review. Structural checks cannot
establish behavioral accuracy: update affected user instructions alongside
feature changes, and review screenshots when the interface changes.
