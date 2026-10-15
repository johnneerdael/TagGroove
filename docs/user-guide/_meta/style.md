# OpenTag user guide: style guide

This guide is for people writing pages of the OpenTag user guide. It is not
published (`_meta/` is excluded in `mkdocs.yml`). Read it once before you write
your first page, then keep the templates at the end open while you work.

The pages that set the bar are:

| Kind | Exemplar |
|---|---|
| Home | `index.md` |
| Tutorial | `learn/first-tagging-session.md` |
| Reference (syntax) | `reference/format-strings.md` |
| Reference (catalogue) | `reference/placeholders.md` |
| Reference + teaching (function) | `reference/regular-expressions.md` |
| Recipe | `cookbook/rename-zero-padded-track-numbers.md`, `cookbook/feat-artists-to-title.md`, `cookbook/tags-from-folder-and-filename.md` |

When this guide and an exemplar disagree, the exemplar wins. Fix this guide.

## 1. Ground rules

1. **The code is the truth.** Describe what OpenTag does, verified in `src/`,
   `src-tauri/`, `crates/` and their tests. Do not describe Mp3tag and assume
   OpenTag matches. If you can't verify a behaviour, leave it out or mark it
   with an `<!-- UNVERIFIED: … -->` comment and tell the docs lead.
2. **Write everything yourself.** The private Mp3tag documentation crawl is a
   checklist of topics, nothing more. Never copy, closely paraphrase or
   translate its prose, examples, tables or screenshots. Facts are fine:
   function names, placeholder names, syntax, option names. Your sentences,
   your examples, your structure.
3. **Use invented example data.** The house album is *Juniper Vale – Harbour
   Lights* (2024, Indie Folk, eight tracks). The screenshots use it too
   (`tests/docs-screens/fixtures.sh`). Other invented artists you may use:
   *The Lanterns*, *Mira Solberg*, *Kofi & the Tidewater Band*, *DJ Nimbus*,
   *Orquesta Alba*. Never quote real lyrics, and never use real albums as
   tagging examples.
4. **Never launch the app on your own machine for docs work.** That includes
   screenshots. They are made in CI (see section 8).
5. **Write for one person who is trying to get something done.** Every page
   answers: what is this, when would I use it, how do I do it, what can go
   wrong.

## 2. Voice

- **Friendly, plain, second person.** "Select the files you want to rename",
  not "The user selects…" or "One may select…".
- **Short sentences, active voice, present tense.** "OpenTag keeps the
  extension", not "The extension will be kept by OpenTag".
- **Explain the why in one sentence** when a rule is surprising. "Quote the
  pattern: an unquoted `)` would end the function early."
- **No hype and no filler.** Avoid "simply", "just", "easy", "powerful",
  "seamless", "robust", "leverage", "utilize". If it were simple, the reader
  wouldn't be here.
- **Don't apologise for gaps.** If OpenTag doesn't do something, say so
  plainly and offer the workaround.
- **British or American spelling?** American, matching the app ("color
  mode", "customize"). The brand palette names keep their own spelling.
- **Numbers:** digits for anything you count in the UI ("3 files"), words
  for one to nine in running prose.

### Terms

Use these exact terms. The [glossary](../help/glossary.md) defines them for readers.

| Use | Not |
|---|---|
| tag panel | tag editor, sidebar, left panel |
| file list | file view, grid, table |
| field (`ARTIST`), value (`Juniper Vale`) | tag (for a single field), attribute |
| tag (the block of metadata), tag type/format (ID3v2.3, APEv2…) | |
| format string | mask, template (except export templates) |
| placeholder (`%artist%`) | variable, token |
| scripting function (`$upper`) | macro, command |
| action, action group | script, macro |
| converter (Tag → Filename…) | wizard |
| pending edits / unsaved changes | drafts, dirty files |
| folder | directory (except where the UI says *directory*) |

Write **UI labels in bold**, exactly as the app shows them in English:
**Save tags**, **Tag → Filename…** (keep the ellipsis when the app has one).
Menu paths use `→`: **File → Options…** or "choose **Convert → Tag → Filename**".
Field names are upper case in code style: `ALBUMARTIST`. Placeholders,
functions and anything the reader types go in code style: `%albumartist%`,
`$num(%track%,2)`.

## 3. Page types and templates

Every page is one of five types. Pick one, and follow its skeleton.

### Tutorial (Learn → Tutorials)

Learning by doing, start to finish, with a guaranteed success at the end.

- Opens with a one-paragraph promise: what you'll make, how long it takes,
  what you need (a checklist using `- [ ]`).
- Numbered `##` steps ("## 1. Open the album"), each with: what to do, what
  you should see (screenshot or description), why it works.
- One path only; no alternatives mid-step (mention them in "Going further").
- Ends with "## What you learned" (bullets) and "## Next steps" (2–4 links).
- Uses the Juniper Vale fixture so the screenshots match the reader's screen.

### How-to guide (Guides, Automate)

Task-oriented, for readers who already know the basics.

- Title is the task or the feature ("Working with covers", "Tag → Filename").
- Opens with what the feature is for, in two or three sentences, and a screenshot.
- `##` sections per task, each self-contained, with steps and an example.
- A "## Gotchas" section (use `!!! warning` for data-loss risks).
- Ends with "## Related".

### Reference (Reference)

Complete and exact; for looking things up.

- Opens with a short orientation (what this is, where it is used) and a
  `!!! tip "New to this?"` admonition pointing to the matching tutorial.
- Covers every item. Completeness beats prose: tables are good.
- Every syntax rule gets at least one example, and every non-obvious rule
  gets an input → output table.
- Ends with "## Related".

### Recipe (Cookbook)

One problem, one copy-paste solution, then an explanation.

- Title is the goal in the reader's words ("Move “feat.” artists out of the
  artist field").
- A `recipe-meta` box: **Uses**, **Works on**, **Time**, **Undo**.
- `## The problem` with a before → after table.
- `## The solution`, the copy-paste part. Use numbered steps for UI work and
  a code block for each format string or pattern.
- `## How it works`: take the solution apart piece by piece.
- `## Variations` with two to four tweaks, plus "## Related".
- Always tell the reader to try it on a copy or a few files first, and how
  to undo it.

### Concept explainer (Learn → Concepts)

Builds understanding, with no procedure.

- Starts with the question a confused reader would ask ("Why does my player
  show a different artist than OpenTag?").
- Uses diagrams (Mermaid is not enabled; use tables or simple ASCII in a
  `text` block), analogies and short examples.
- Ends with "what this means in OpenTag" and links to the relevant guides.

## 4. Documenting a scripting function

Each function gets a `###` heading with its name and the same block. Copy
this skeleton:

````markdown
### `$num(x,n)` { #num }

Pads the number at the start of `x` with zeros to at least `n` digits.

| Argument | Meaning |
|---|---|
| `x` | Text that starts with a number, for example `%track%`. |
| `n` | Minimum number of digits. |

**Returns** the number with leading zeros; text after the number is dropped.

| Format string | With these values | Result |
|---|---|---|
| `$num(%track%,2)` | `TRACK` = `3` | `03` |
| `$num(%track%,3)` | `TRACK` = `3/12` | `003` |
| `$num(%track%,2)` | `TRACK` = `B2` | `00` |

!!! warning "Gotcha"
    Text that doesn't start with a digit counts as 0, so vinyl sides like
    `B2` become `00`. Check for them first with the filter `track MATCHES ^[^0-9]`.

**Related:** [`$fmtnum`](#fmtnum), [auto-numbering](../converters/auto-numbering.md)
````

Rules:

- The signature uses the argument names from the reference table and shows
  optional arguments in brackets: `$regexp(x,expr,repl[,c])`.
- At least **three examples**: the common case, an edge case (empty value,
  multi-value field, Unicode, text where a number is expected), and a
  combination with another function.
- Example tables have the columns *Format string | With these values |
  Result*. Show empty results as *(empty)* in italics, not as a blank cell.
  Show spaces that matter with `·` and say so in the caption.
- **Verify every example** against the code or a test. Most functions have
  tests in `crates/tag-core/tests/script.rs`. Leave an HTML comment naming
  the test or the code line:
  `<!-- verified: crates/tag-core/tests/script.rs:57 -->`.
- Errors are behaviour too. Say what happens with a wrong argument count or
  an invalid value.

## 5. Keyboard shortcuts and platform differences

Use `pymdownx.keys` syntax. Its output follows the platform conventions: ⌘,
⌥, ⇧ and so on.

- **Primary-modifier shortcuts** (⌘ on macOS, Ctrl elsewhere): write the Mac
  form first, then the Windows/Linux form, joined by " / ".
  `++cmd+s++ / ++ctrl+s++` renders as ⌘S / Ctrl+S.
- **Control-only shortcuts** (Control on every platform, including macOS):
  write `++ctrl+q++` once and add "(Control on every platform)" the first
  time a page uses one. These are Ctrl Q, Ctrl H and Ctrl M, the ones macOS
  reserves for ⌘.
- **Alt shortcuts:** `++alt+1++`. Say once per page that Alt is ⌥ Option
  on a Mac.
- Keys alone: `++f2++`, `++enter++`, `++del++`, `++esc++`.
- The full list lives in `reference/keyboard-shortcuts.md`. Pages may repeat
  a shortcut, but must never contradict that page.

When steps differ by platform, use linked content tabs, always in this order
and with these exact labels (identical labels keep the tabs in sync across a
page and across pages):

```markdown
=== "macOS"

    …

=== "Windows"

    …

=== "Linux"

    …
```

Add `=== "Android"` only where Android behaves differently. Don't use tabs
for a single-word difference; write "Finder (macOS), Explorer (Windows) or
your file manager (Linux)" instead.

## 6. Admonitions

| Type | Use for |
|---|---|
| `!!! tip` | A better or faster way. "New to this?" pointers to tutorials. |
| `!!! note` | Background that helps but isn't needed to finish the task. |
| `!!! warning` | Gotchas: things that silently produce the wrong result. Title it `"Gotcha"` or with the specific risk. |
| `!!! danger` | Anything that can lose data and can't be undone (permanent delete, Remove tag on many files, overwriting covers). |
| `!!! example` | A worked example that is longer than a table row. |
| `!!! info "Coming from Mp3tag"` | Where OpenTag behaves differently from Mp3tag. Use only this title for that purpose. |
| `!!! abstract` | Page or section summaries; also used by draft stubs. |
| `??? question "Try it"` / `??? success "Answer"` | Collapsible exercises and their answers in tutorials. |

Use no more than one admonition per screen of text. If everything is a
warning, nothing is.

## 7. Links

- Link with relative paths to `.md` files: `[Tag → Filename](../converters/tag-to-filename.md)`.
  MkDocs checks them, and `mkdocs build --strict` fails on a broken link or anchor.
- Link a term the **first time** it appears on a page, not every time.
- Tutorials link to reference pages for details. Reference pages link back to
  the tutorial in their opening `!!! tip "New to this?"`.
- Every page ends with a `## Related` section (tutorials: `## Next steps`)
  that has two to five links.
- Use explicit anchors for anything other pages will link to, such as
  `### $num(x,n) { #num }`. Don't rely on generated slugs for functions and
  placeholders.
- Link outside the guide only to stable official sources (MusicBrainz,
  Discogs, the ID3 specification, the OpenTag repository). Do not link to
  the Mp3tag documentation as an explanation. The guide must stand on its own.

## 8. Screenshots

Screenshots come from CI, never from a writer's machine.

- **Pipeline:** `.github/workflows/docs-screenshots.yml` builds the debug app
  on Linux and runs it under xvfb with the Juniper Vale fixtures
  (`tests/docs-screens/fixtures.sh`). `tests/docs-screens/screens.js` drives
  the app through named *scenes*, and `scripts/docs-screenshots.sh` captures
  each scene in Light and Dark at 2× scale. The PNGs are uploaded as the
  `docs-screens` artifact.
- **Getting them into the guide:** `scripts/docs-screens-fetch.sh [run-id] [scene…]`
  downloads the artifact and copies `<scene>-light.png` and `<scene>-dark.png`
  into `docs/user-guide/assets/screens/`. Commit only the scenes your pages use.
- **Requesting a new screen:** add a scene object (`name` and `run`) to
  `tests/docs-screens/screens.js`. Use the existing scenes as models:
  `op('<data-op>')` runs a menu command, `clickRow(i)` and `selectAll()`
  select files, and `menu('Convert').open = true` opens a menu. Push your
  branch; the workflow runs on pushes that touch the scene files, or start it
  by hand with `gh workflow run docs-screenshots.yml --ref <branch> -f scenes="<name>"`.
  If you can't script the scene yourself, add it to the "Screens wanted"
  table in `_meta/ia.md`.
- **Embedding:** always both variants, so each colour scheme shows the
  matching screenshot:

  ```markdown
  <figure markdown>
  ![The tag panel with eight files selected; Title shows < keep >](../assets/screens/tag-panel-keep-light.png#only-light)
  ![The tag panel with eight files selected; Title shows < keep >](../assets/screens/tag-panel-keep-dark.png#only-dark)
  <figcaption>With several files selected, fields that differ show <code>&lt; keep &gt;</code>.</figcaption>
  </figure>
  ```

- **Alt text** describes what the reader needs to see, not "screenshot".
  Add a caption whenever the image makes a point.
- Show the smallest state that makes the point; don't add a screenshot that
  only repeats the text. Never edit screenshots by hand (no arrows or blur).
  If something needs pointing out, describe it in the caption, or ask for a
  scene that highlights it.
- Scene names are kebab-case and describe the state: `tag-to-filename`,
  `extended-tags`, `options-tags-mpeg`.

## 9. Code blocks and examples

- Format strings, patterns and filter expressions go in fenced `text` blocks
  (there is no highlighter for them) so the copy button gives the exact text:

  ````markdown
  ```text
  $num(%track%,2) - %title%
  ```
  ````

- Short inline values use code style. Leave literal spaces visible: `' - '`.
- Show results the way the app shows them. For a file name, include the
  extension only if the step changes it.
- Write paths with `/` in prose and give the Windows form only where it
  matters. Inside format strings, use `\` to make folders (it works on every
  platform), and say so when it first appears.

## 10. Abbreviations and glossary

`_snippets/abbreviations.md` is appended to every page. Any abbreviation
listed there (ID3, APE, MP4, CDDB, …) gets a hover tooltip on every page.
Add one when you introduce a new abbreviation. Define terms in
`help/glossary.md`.

## 11. Page status and the coverage map

- New pages start with `status: draft` in the front matter. The navigation
  shows a pencil next to them. Remove the line when the page meets this guide.
- `_meta/ia.md` maps every Mp3tag documentation topic to our pages. When you
  finish a page, update its **Status** cell there in the same commit.
- `scripts/docs-status.py` lists drafts and checks that every page in the
  navigation exists. Run it before you push.

## 12. Building locally

```bash
python3 -m venv .venv && .venv/bin/pip install -r docs/site-requirements.txt
.venv/bin/mkdocs build --strict      # what CI runs; must pass
.venv/bin/mkdocs serve               # live preview at http://127.0.0.1:8000
```

Building and previewing the guide never starts OpenTag.

## 13. Language

The guide is written in English only for now. Write UI labels as the English
app shows them. Translations of the app (Deutsch, Nederlands) exist; the
guide may get translated later, so:

- don't build sentences around a UI label's grammar ("click **Save tags** to
  save" works in any language; avoid "the **Save**d files"),
- don't put text that needs translating in images,
- keep idioms and wordplay out of instructions.
