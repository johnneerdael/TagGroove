# Publish and maintain the user guide

The public guide is <https://taggroove.neerdael.nl/>. Its source is
`docs/user-guide/`, and `mkdocs.yml` defines the public navigation and theme.
Authoring, building and publishing all happen in `johnneerdael/TagGroove`.
The site is served by this repository's GitHub Pages under the custom domain
`taggroove.neerdael.nl`, which keeps the address off the privacy-list rules that
match the `github.io` project path.
The header uses Material's standard GitHub repository component to display the
latest release tag, stars and forks. Source, edit and release links retain their
normal literal destinations; no proxy or encoded URL is used.

## Write and validate a page

Read `docs/user-guide/_meta/style.md` and the topic's brief before writing.
Keep instructions specific to TagGroove's actual behavior. Check the relevant
source and examples; record runtime checks separately from source or test
evidence. Reuse existing verified guide images where appropriate. New app captures run in the review-authorized Linux CI job against disposable
fixtures; follow the screenshot section below.

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
python3 -m venv /tmp/taggroove-docs
/tmp/taggroove-docs/bin/python -m pip install -r docs/site-requirements.txt
/tmp/taggroove-docs/bin/python -m unittest discover -s docs/tests -v
/tmp/taggroove-docs/bin/python -m mkdocs build --strict
python3 scripts/docs-status.py
cargo test -p tag-core --test user_guide --locked
```

On Windows, create the environment outside the checkout, for example with
`python -m venv "$env:TEMP\taggroove-docs"` in PowerShell, then use
`& "$env:TEMP\taggroove-docs\Scripts\python.exe"` for the commands above.
Preview with `/tmp/taggroove-docs/bin/python -m mkdocs serve`, then check both color
modes, narrow screens, navigation, search, screenshots and copyable examples.
The generated site is `build/user-guide-site/`; do not commit that output.
The Rust documentation tests execute the published regex tables and selected
format-string, date and recipe examples against TagGroove's core. They require
the development prerequisites from README; they do not establish Mp3tag
runtime parity.

Run `python3 scripts/docs-status.py --drafts` to find unfinished pages, or
`--coverage` to compare the plan with the original feature checklist.
Coverage is a writing plan, not a feature-parity claim.

## Move or rename a page

The guide is organised by feature (see `docs/user-guide/_meta/ia.md`). When a
page moves or is renamed, keep its old URL working:

1. `git mv` the page, then update every relative link to it. The strict build
   reports the ones you miss.
2. Add `"old/path.md": "new/path.md"` to `docs/hooks/redirects.json`. The
   build fails if the new page does not exist or if a page still exists at the
   old path, and it writes a forwarding page at the old address that keeps the
   query string and `#anchor`. The README, the app and bookmarks keep working.
3. Update the page in `mkdocs.yml`, in `docs/user-guide/_meta/planned-nav.yml`
   and in any Rust or JavaScript test that reads it
   (`grep -rn "user-guide/<old path>" crates tests src-tauri`).

Anchors inside a page do not redirect. When you split a section onto its own
page, leave a short section with the same heading on the old page that links
to the new one.

## Review and publication

### Manual docs-only publication

The repository owner can publish current `main` without an application release:

```sh
gh workflow run docs-manual.yml --repo johnneerdael/TagGroove --ref main
```

In GitHub, choose **Actions → Publish docs manually → Run workflow**, keeping
the branch set to **main**. This workflow has only a manual trigger; pushes,
pull requests and schedules cannot start it. It refuses other branches and
non-owner dispatches.

The build checks out the dispatch's immutable main revision, installs the pinned
documentation toolchain, runs `mkdocs build --strict` and uploads the Pages
artifact. Deployment uses the existing `github-pages` environment and Pages
concurrency group, and skips publication if main has advanced. Only deployment
receives Pages/OIDC write permissions. It runs no application builds, native
tests, screenshot capture, packaging or release publication, and supplies no
application review or release authorization.

### Reviewed publication with an application release

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
   building the site. The build job receives no write permission. It runs the
   publication tests, the writing-plan check and `mkdocs build --strict`,
   adds a `CNAME` file for `taggroove.neerdael.nl` and uploads the result as the
   Pages artifact.
5. The deployment job checks approval and current main again. If a newer main
   revision exists, it skips deployment and leaves publication to that
   revision. Otherwise it deploys the uploaded artifact with
   `actions/deploy-pages`. No artifact code runs.

The `github-pages` environment should permit only `main`. No deployment key or
other secret is involved. In the repository's **Settings → Pages**, set the
source to **GitHub Actions** and the custom domain to `taggroove.neerdael.nl`,
then enable **Enforce HTTPS** once the certificate is issued. DNS needs a
`CNAME` record from `taggroove.neerdael.nl` to `johnneerdael.github.io`.
GitHub describes the separate artifact and deployment jobs in its
[custom Pages workflow guidance](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages).

Keep legacy build, release, docs and screenshot workflow IDs disabled;
never enable them to recover a deployment. Read `.github/CI_LOCKDOWN.md` for
review-controller operations and the validation retry procedure.

If the site is missing, first check that the docs files and reviewed Pages
workflow are actually merged to `main`, then inspect the approved source's
**Reviewed main release → user-guide** jobs. A feature branch cannot publish.
If the jobs failed, rerun them from the original run:

```sh
gh run rerun SOURCE_RUN_ID --repo johnneerdael/TagGroove --failed
```

Source authorization checks still apply; source or review drift requires fresh
validation. GitHub's [workflow rerun documentation](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/re-run-workflows-and-jobs)
requires repository write permission, preserves the original SHA/ref and limits
reruns to thirty days after the original run. Beyond that window, create a new
reviewed source publication through the normal PR, validation and merge
process. Do not enable legacy workflows or widen token permissions. To roll
back content, submit a revert PR through that same review and validation
process.

The former automatic **Docs screenshots** workflow remains removed. The Linux
native UI job in the reviewed reusable build captures the five core scenes
in light and dark modes only after exact-head review authorization. It uploads
`docs-screens`; it cannot publish the guide or write repository files.

Download images from the verified reviewed-validation run with
`scripts/docs-screens-fetch.sh RUN_ID`. Inspect every image, record its source
run and commit in the verification ledger, then commit the refreshed guide
images and obtain another clean review of the final head. Do not edit old
screenshots to make them look like new runtime evidence.

Structural checks cannot establish complete behavioral accuracy. The selected
Rust example checks cover core behavior; update affected instructions alongside
feature changes, and review screenshots when the interface changes. Complete
coverage of the original documentation remains a separate writing and
verification task, tracked in the writing plan.

## Retiring the former guide address

Before the old `johnneerdael/audio-tag-guide` Pages site is archived or
disabled, replace its published content once with forwarding pages so existing
bookmarks keep working. Commit to that repository's Pages branch an
`index.html` and a `404.html` that both send the visitor to the same path on
`https://taggroove.neerdael.nl/`:

```html
<!doctype html>
<meta charset="utf-8">
<title>Moved to taggroove.neerdael.nl</title>
<link rel="canonical" href="https://taggroove.neerdael.nl/">
<script>
  const path = location.pathname.replace(/^\/audio-tag-guide/, '');
  location.replace('https://taggroove.neerdael.nl' + (path || '/') + location.search + location.hash);
</script>
<p>The guide moved to <a href="https://taggroove.neerdael.nl/">taggroove.neerdael.nl</a>.</p>
```

Keep that repository's Pages site enabled until the forwarding pages are live.
This one-time step is outside this repository's reviewed publication path.
