# Guide evidence and remaining work

This directory is authoring evidence. It is excluded from the published site.
It records where facts came from, what was checked, and what remains uncertain.
It does not certify full coverage, runtime parity, release availability, or a
clean GitHub review of the final integrated branch.

## Source snapshot

`source-snapshot-2026-10-14.json` identifies the 94 HTML pages retrieved from
the official documentation on 2026-10-14, with their URLs and SHA-256 hashes.
The snapshot is research material, not OpenTag's behavior specification. The
guide uses independent explanations and invented practice data. Original
article prose and screenshots are not republished here.

The snapshot's 893 headings are a structural inventory. A matching heading,
function name, or page count does not show that its meaning has been taught.
The 187-topic navigation plan remains the writing backlog. Its empty pages
remain drafts, and publication checks exclude them from HTML and search.

## Integrated authoring batches

The first combined checkpoint (`23ecba0`) contains 75 completed pages, including
the eight initial publication pages. The export batch raises the local total
to 80. These are integration milestones, not completion of the whole guide. The language options page remains a draft because its actual
implementation is not yet part of this source baseline.

| Batch | Preserved source revision | Evidence and limits |
| --- | --- | --- |
| Initial publication | `93cc2a92d8e6d05adeb9415bb5335516ed413460` | Eight pages; regex tables, inline expressions, tutorial and recipe fences run against the core. The written action order is tested on a bracketed guest credit. The Windows newline failure was reproduced before correction. |
| Actions | `5614494d0a7ec7389de8f7a22ff832a7f4ee80fc` | Nineteen pages describe groups and all eighteen current action types. Six tests execute representative case, regex, codepage, guessing, cover-size and export examples. A finite independent source review found the multiline export-input mistake; the recipe now saves a template file and uses Browse. |
| Converters | `1e5036c0dc88c8158fd12221ac1d7c1873135274` | Seven pages and seven tests cover each converter, numbering, positional/keyed text import and a combined folder-capture/function/quoting walkthrough. A finite independent canonical/source review found no established defect. |
| Scripting and beginner lessons | `d5cae7dd37b280021ea0b6c23e735b4805196b55` | Seven function-reference pages and two lessons. The function index equals the 64-name core registry. Tests execute 197 function examples, all twenty date tokens, seventeen lesson fences and eleven lesson table examples. A finite independent review checked all nine pages; the rendered-column context caveat was clarified afterward. |
| Concepts and workflow lessons | `803d8d11696064f8c896ba703382780cec97be12` | Multi-value and encoding concepts, first action group and filename lessons; three core tests exercise written examples and action order. Added regex escape detail is exercised by the regex table tests. A finite independent source review corrected the dialog-closing step. |
| Options | `312b0f0` | Twenty-two completed pages and one held language draft. The accompanying ledger maps 150 factual topics to authored sections and records current limitations. Independent source review corrected autosave and retained historical comment/Matroska details. |
| Export controls, templates and catalogue lesson | `c07bf16` | Five pages, four tests executing actual templates/results under LF and CRLF, plus a canonical fact ledger. Root finite source/canonical review found TRACK quote escaping; its regression failed before correction and passes afterward. |
| Filter and data reference | `65f3898` | Six pages. Four tests execute 24 filter queries, check 118 field definitions and 236 native table rows, and retain invalid-query cases. The ledger groups repeated canonical native-cell facts. A finite independent semantic review found no established defect; it did not recompute every cell. |

These revisions preserve batch provenance. Current page hashes and later
corrections can differ after integration. The final PR needs its own exact-head
GitHub Codex code review and trusted validation after all source dependencies
are integrated. Earlier local reviews do not replace that gate.

## Semantic evidence

`options-2026-10-16.json` retains factual topics, the authored section for each,
source hashes, held material and independent-review notes. Its source baseline
is `007a806`; the author compared the relevant later option implementations.
Do not assume that future language controls or online-service behavior exists
until the application source actually lands.

`filter-reference-2026-10-16.json` retains explanations and capability facts,
source hashes, limitations, and grouped counts for native identifier cells.
Identifiers are stored once in the written mapping tables and checked against
the current core vocabulary by `user_guide_filter_reference.rs`. The full author
ledger digest identifies the pre-integration receipt; grouped cell counts are
not evidence that every descriptive sentence was independently reviewed.

The other batches teach a concept, show a worked input/expression/output,
explain the intermediate steps and give pitfalls or follow-on recipes. Their
test files read the written tables and fences rather than maintaining a
parallel set of example formulas. Selected examples establish those core
results; they do not establish the complete UI workflow or native-format
save-back behavior on every platform.

## Combined verification

At the newline-portability integration milestone, 45 guide tests passed:
`action_guide`, `concept_guide`, `converter_guide`, `guide_tutorials`,
`script_reference_guide`, `initial_guide_examples`, `user_guide`,
`user_guide_filter_reference`, and `guide_example_newlines`. Focused clippy
passed for those targets. The shared fence parser preserves body spaces,
removes only the opening fence's indentation, and normalizes line endings.
The actual fenced corpus from sixteen pages has equal literal contents under
LF and CRLF. Initial-publication semantic assertions also execute both forms.

The final local navigation/search/table integration passed the strict MkDocs
build, all eight publication tests, the 75/187 writing-plan check and full Rust
formatting. Rerun affected checks after navigation or content changes. Browser checks concern the generated documentation only;
they are separate from application runtime checks and public deployment.
The final integration receipt must record those outcomes before opening a PR.

## Remaining work

The advanced export batch covers template commands, grouping and sorting,
filenames, encoding, side effects and progressive reports. Reconcile its
source-dependent claims when the outstanding application PRs land.
Web-source parser commands, source structure, results, authentication and
troubleshooting need independent explanations checked against actual OpenTag
behavior, including explicit unsupported or different behavior.

The separate command-line, shortcuts, menus and placeholder reference batch
is not yet integrated here. Getting started, general file-list/tag-panel
workflows, cover management, playlists, source workflows, troubleshooting and
the remaining cookbook topics still require completion and semantic checks.
Review each canonical paragraph, control and note; do not close these gaps by
removing draft status or matching headings alone.

Keep this expansion local until the existing application/publication PRs have
merged, then integrate their real source and recheck dependent instructions.
Actual GitHub Pages publication requires an approved merged source, a successful
deployment and a checked public URL. A local build or uploaded artifact does
not establish publication.

## Initial live publication

`publication-2026-10-16.json` records the approved source, successful reviewed
Pages deployment and public HTTPS checks. Only the initial eight pages are
live at that milestone; all eight exactly match the approved build. The search
index contains only those pages, and the logo is served successfully. Live
browser interaction remains unverified after bounded browser-tool timeouts.
This limitation is separate from the successful deployment and fetched public
content. The 80-page local expansion has not been pushed or published.

## Export integration verification

After the CSV review correction, all 49 combined guide tests and all eight
publication tests passed on the 80-page local source. Focused export clippy and
the strict guide build passed as well. This is local evidence; the expansion
still requires source reconciliation and its own exact-head external review
and trusted validation before publication.
