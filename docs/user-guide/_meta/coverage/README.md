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
to 80, the navigation-reference batch raises it to 88, and the web-source
batch raises it to 91. The remaining Actions, Help and context-menu references
raise the total to 94. File-list, Extended tags and cover-management guides
raise the current milestone to 97. These are integration milestones, not completion of the whole guide. The language options page remains a draft because its actual
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
| Navigation reference | `273020e` | Eight pages, one test executing nine written focus/platform examples and 41 existing routing/file/list/history contracts. Root read all pages and canonical keyboard rows with source spot checks; current5f only, not every ledger mapping independently recomputed. |
| Web-source installation, tutorial and command reference | `a687ab5` | Three pages teach all 15 source headers, settings, a complete local JSON lookup/search tutorial and all 59 accepted parser commands. Four tests execute the actual written definitions and examples with LF and CRLF; 19 unchanged pure source-module tests also run. Source and dependency features are unchanged. A finite root review clarified SayRegexp’s optional stop-text rule. |
| Editing workflows | `cb64e25`, `6f727cd` | Three guides teach file selection/order/cell editing, Extended tags and cover management. Three tests execute written order, mixed-field rename and cover-presence examples; 47 related contracts pass. Finite independent source/model review found no concrete mismatch. |
| Remaining menus and contexts | `568b559` | Three pages cover Actions, Help, cover/row/panel/field menus, insertion helpers and history. Two tests execute written caret and cover-state tables; 50 related contracts pass. Finite independent source review found no concrete mismatch; no native runtime or independent canonical re-crawl was claimed. |
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

The advanced export and web-source batches are integrated. Their ledgers
record canonical topics, working examples and source limitations. Current
claims have been reconciled with main `a9cd900`, including merged PRs 26, 27
and 29; the incoming Windows-undo draft note and verification are retained. Pending language, provider, gnudb, web-source and action changes require
a new source comparison after their own merges; this guide does not publish
those branch-only behaviors as current.

The command-line, keyboard and six menu-reference pages are integrated.
The Actions, Help, cover, field and format-helper contexts are now integrated;
the standalone Tools reference and broader placeholder audit remain. Getting started, general file-list/tag-panel
workflows, cover management, playlists, source workflows, troubleshooting and
the remaining cookbook topics still require completion and semantic checks.
Review each canonical paragraph, control and note; do not close these gaps by
removing draft status or matching headings alone.

The user authorized publishing this expansion alongside the application PRs.
Keep its review and validation bound to the final head and current main;
reconcile later application changes in their affected guide pages.
Actual GitHub Pages publication requires an approved merged source, a successful
deployment and a checked public URL. A local build or uploaded artifact does
not establish publication.

## Initial live publication

`publication-2026-10-16.json` records the approved source, successful reviewed
Pages deployment and public HTTPS checks. Only the initial eight pages are
live at that milestone; all eight exactly match the approved build. The search
index contains only those pages, and the logo is served successfully. A later bounded independent headless-Chrome check verified actual live
tutorial navigation, regex search, palette changes and the 390px mobile
drawer without page errors or document-level horizontal overflow. See
`/tmp/opentag-public-guide-browser-qa/results.json` for the retained local
receipt; the integration receipt records the bounded scope. That check
concerns the initial eight pages, not deployment of this expansion.

## Export integration verification

After the CSV review correction, all 49 combined guide tests and all eight
publication tests passed on the 80-page local source. Focused export clippy and
the strict guide build passed as well. This is local evidence; the expansion
still requires source reconciliation and its own exact-head external review
and trusted validation before publication.

The initial publication receipt also records a bounded follow-up HTTP check
of 19 same-origin scripts, stylesheets and images, including the search worker
resolved from all eight page configurations. All returned 200 with nonempty
bodies. This is asset availability evidence, not live browser execution.

## Current integration and exact gaps

`integration-2026-10-16.json` records the current source baseline, conflict
resolutions, page hashes, local validation and remaining drafts. Its 94-row
canonical map reports whether mapped guide pages are written or draft. This
is a routing/backlog record: a written destination alone does not establish
that every sentence, caveat and original example has been taught. Semantic
ledgers and executable examples supply narrower evidence.

This milestone publishes 97 of the 187 planned pages. The other 90 stay
excluded from public HTML and search. The largest gaps are getting started
and platform workflows, tag-panel, save/remove-tag, playlist and chapter
workflows, customization, help/troubleshooting and 32 cookbook
pages. The broader placeholder reference still needs a semantic completeness
audit despite already being published. Language and new-provider instructions
remain dependent on their feature merges. Completing this milestone does not
complete the user's request for every detail of the original documentation.

The earlier 91-page local site contained 91 searchable routes and 769 section
entries, with no draft HTML. The integration receipt records the final
94-page build separately. A bounded isolated Chrome check verified the
actual search worker/result navigation, light/dark rendering and sampled
390px tutorial/table layouts. A later pointer-only follow-up also navigated the nested mobile drawer
through Web sources, Actions, Action types and Format Value. Click centers
were clipped to the actual sidebar and hit-tested against the visible
label/link; no checkbox state was set through JavaScript. The expected
page opened, its drawer closed and no page errors occurred. Temporary
browser profiles were removed. See the integration receipt for exact
scope and local evidence paths.

After adding or changing written examples, run the complete tag-core crate:
`cargo test -p tag-core --locked`. This includes every documentation corpus
guard automatically. A later partial-import fence exposed a stale expected
count when only the converter target was rerun; the final correction keeps
LF/CRLF equality and significant-space checks and identifies the page in
future assertion failures.

The final 97-page revision includes main `a9cd900`, the independently reviewed editing
batch and a documented Find-engine distinction. The complete tag-core
package passed 498 tests with no failures or ignored cases; all 336 Node UI
tests and eight publication tests passed. Strict build, writing plan, formatting
and focused Clippy passed. These are local checks; the final head still
requires its own clean GitHub Codex review, trusted CI and deployment proof.

## Expanded live publication

`publication-expanded-2026-10-16.json` records PR 30’s reviewed deployment from
main `432d354`. All 97 public routes return HTTP 200 and exactly match the
approved build. Search contains exactly those 97 routes and 825 section
entries; its bytes match too. All nineteen referenced same-origin assets,
including the actual search worker, return nonempty HTTP 200 responses.
Actual isolated-profile Chrome checks verified tutorial palettes, command
search and result navigation, representative 390px tables and tutorials, and
pointer-only nested mobile navigation to Format Value. No page errors were
observed. This does not establish native application behavior or completeness
of the remaining ninety drafts. The next local three-page batch is separate
from this public milestone.

## Next local basic-editing checkpoint

`basic-editing-2026-10-16.json` records four authored pages on editing and
saving, the tag panel, tag copy/cut/paste and removal. They build on the three
preserved customization/Find pages. The local navigation has 104 completed
pages and 83 drafts; the public site still has 97 completed pages. This batch
is held locally for finite independent review and later queue integration.
Examples read the actual written tables under LF and CRLF, exercising the
application models and real core tag-block removal on generated MP3 fixtures.
The evidence does not claim native UI verification or completion of every
canonical topic. The source baseline includes merged PR 28 (`dbcd9ce`), while
unmerged language/provider/freedb/web-source changes remain out of scope.

## Local cleanup recipes

`cleanup-recipes-2026-10-16.json` records four new text/credit recipes, bringing
the local checkpoint to 108 authored pages and 79 drafts. The public site
remains at 97 pages. Six tests execute the actual written action tables in
order and their before/after cases under LF and CRLF. This includes deliberate
nonmatches, multiline anchors, existing value lists, exact deduplication and
Guess Values ambiguity. Independent review is pending; these are source/core
examples, not native UI or player-compatibility verification.

## Local number and date recipes

`number-recipes-2026-10-16.json` records four further cookbook pages, bringing
the local checkpoint to 112 authored pages and 75 drafts. They explain
guarded track/disc extraction, per-folder totals, and date-shape cleanup,
including native MP4 pair/custom-field distinctions and explicit preview,
staging, save and undo timing. Six actual core tests and one frontend model
test read the written examples under LF and CRLF. The public baseline
remains 97 pages; independent review and queue integration are still pending.
These chapters do not establish full canonical coverage or native UI parity.

## Local cover recipes

`cover-recipes-2026-10-16.json` records three practical artwork recipes,
bringing the local checkpoint to 115 authored pages and 72 drafts. Four
core tests execute the written import counts, filename candidates, resize
dimensions and per-file front-cover membership on generated image/MP3
fixtures. Two model tests verify the written roles, partial membership and
planned-cover flags under LF and CRLF. These are local source/core/model
checks, not installed UI, universal native-format or player certification.
The public baseline remains 97 pages and independent review is pending.

## Local text-quality recipes

`text-quality-recipes-2026-10-16.json` records three further recipes, bringing
the local checkpoint to 118 authored pages and 69 drafts. They teach an
explicit capitalization policy, a diagnosed UTF-8 repair, and a reviewed
field keep-list with artwork selectors. Five tests read the actual ordered
actions, formulas, field inventory and error cases under LF and CRLF.
These core/source checks do not certify native UI, universal encoding
recovery or complete container cleanup. The public baseline remains 97
pages and finite independent review is pending.

## Local foundational tag concepts

`tag-concepts-2026-10-16.json` records three explanation pages, bringing the
local checkpoint to 121 authored pages and 66 drafts. Three core tests read
the actual field snapshots, expressions and tag-priority tables under LF and
CRLF. An independently generated MP3 carries three conflicting tag blocks;
read options choose the expected title/album without merging or changing
file bytes. This does not certify other players or every native format.
The public baseline is still 97 pages, and finite peer review is pending.

## Local cover, sorting and loudness concepts

Three independently authored concept chapters raise the local checkpoint to
124 completed pages and 63 drafts. Four written-table core tests exercise
sort-field roundtrips/fallbacks, compilation validation, ReplayGain field
roundtrip/deletion and conditional undisplayed-APE migration in generated MP3
files. Two frontend-model tests cover six picture-identity/order cases and
separate column display/sort keys. LF and CRLF variants use the same written
examples. The ledger records source/topic evidence and a bounded local browser
sample; it does not establish player behavior or complete canonical coverage.
The public baseline remains 97 pages. Finite peer review is pending.

## Local inspection recipes

Three recipes for missing fields, embedded-cover counts and bitrate thresholds
bring the local checkpoint to 127 authored pages and 60 drafts. Four core tests
execute fourteen written query-table cases, three main solution fences and
additional edge cases under LF/CRLF. A generated MP3 supplies real codec,
bitrate and zero/one-cover information while retaining the audio payload and
external PNG. The recipes distinguish absence from empty values, external art
from embedded counts, zero readings from positive bitrates, and filtering
visibility from selection. Narrow query tables scroll without splitting
expressions. This is not a native UI/player/quality test or a full canonical
audit. The public baseline remains 97 pages; finite peer review is pending.

## Cumulative 127-page publication candidate

The publication branch collects thirty completed chapters on main `dbcd9ce`,
retaining all 97 mature pages and their navigation. The new candidate ledger
`publication-candidate-127-2026-10-16.json` records 127 completed pages, 60
remaining drafts, current page/source hashes and final validation. Original
finite peer receipts are preserved unchanged in `reviews/`; they do not replace
the required clean GitHub CODE review of the final PR head. The single filter
recipe wording finding is corrected separately and its original receipt kept.
Historical local-only milestones above remain provenance, not current approval
or publication claims. The candidate still requires external review, trusted
validation, approved merge and actual expanded Pages verification.

## Local folder-layout recipes after PR31 preparation

Three local recipes cover artist/album, multi-disc and compilation paths,
raising authored content to 130 pages with 57 drafts. They remain separate
from frozen PR31's 127-page source. Four tests execute fourteen written
primary cases and six variation results under LF/CRLF through actual core
filename mode and the actual pure host path helper. The host helper is
extracted into a temporary std-only Rust harness using the existing compiler;
no Tauri app or real rename/move is run. The ledger records this test-only
requirement and the narrow limits. Missing first components, relative roots,
extension retention, write timing and re-nesting are explained explicitly;
no all-platform collision/alias guarantee is made. The public baseline remains
97 pages at this checkpoint; PR31 is awaiting its existing trusted CI.

## Actual 127-page publication and local recipe correction

PR31 merged as `3f6639c`; GitHub Pages deployment 7115366569 succeeded for
that source. `publication-127-2026-10-16.json` records the deployment API
evidence and actual public Chrome checks: new homepage links, two new
chapters, search/result navigation, light/dark, sampled 390px tables with
real horizontal scrolling, and clipped pointer-only nested drawer navigation.
The same-profile search response matches 127 routes / 1,040 entries and the
verified public hash. Two initial harness waits selected search-panel
headings instead of the main article; the receipt retains that diagnosis.
The public guide has 127 completed pages and 60 drafts remain excluded. The
three subsequent folder-layout recipes remain local-only (130 authored / 57 drafts).
Their finite peer receipt is preserved unchanged; its single P3 finding is
corrected by qualifying a missing album as empty or year-suffix-only.
