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
raise the guide30 milestone to 97. These are integration milestones, not completion of the whole guide. That baseline held Language as a draft until its implementation arrived. The Language implementation in PR17 now supplies the source-backed options reference and customization guide; only those two additional pages become current, while all other unfinished pages remain gated.

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
and 29; the incoming Windows-undo draft note and verification are retained. PR17 separately reconciles the implemented Language controls and its two current pages. The options/integration JSON receipts preserve their original baseline and held-language observations as historical evidence. Pending provider, gnudb, web-source and action changes require a new source comparison after their own merges; this guide does not publish those branch-only behaviors as current.

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

## Local playlists and chapter-row teaching

One three-page local batch adds a full playlist/CUE guide, a chapter-row
explanation and a per-album playlist recipe. The local writing checkpoint
reaches 133 completed pages and 54 drafts; public content remains 127 pages.
The canonical Playlists page's load/add/save/generator topics map to the
committed playlist-chapters ledger. Incoming Nero/artwork and safe M3U
encoding notes are preserved. Six core tests execute written M3U8/PLS/CUE
text, saved WAV fields, encoding choices, row edits and rejection cases with
LF/CRLF. Three production controller/model tests use request/control seams,
including a deterministic clock, to verify scope and row restrictions. The
real CUE edits change the sheet and retain referenced audio bytes. Broader
chapter/container/player behavior remains source evidence, not a native UI
or every-format qualification. Filesystem overwrite/alias risks are bounded
and original deferred P2 work remains outside this documentation batch.

## Local first-use foundation

Four completed first-use pages add installation, orientation, opening files
and a beginner starting path, bringing the local writing checkpoint to 137
completed pages and 50 drafts. Public content remains at 127 pages. The
previous playlist/chapter peer receipt is preserved unchanged in `reviews/`.
The first-use ledger separates current v0.2.12 package/source evidence and
official OS guidance from native installation qualification. Written scan
examples and loading/count contracts use actual core/controllers; installation
commands are documentation, not agent-executed installers. Draft platform
chapters, migration and remaining canonical details remain unfinished.

## Cumulative 137-page publication candidate

The next publication candidate on main `3f6639c` preserves all 127 public
chapters and adds ten completed pages: three folder-layout recipes, three
playlist/CUE/chapter pages and four first-use chapters. Fifty planned topics
remain drafts. The `publication-candidate-137-2026-10-16.json` manifest records
that final scope, source identity and validation. Original finite six-page
peer receipts remain unchanged; foundation/cumulative peer review is separate.
Native installers, mobile save-back and universal filesystem safety remain
unqualified. Current release facts are explicitly pinned to v0.2.12. This
is a writing/publication milestone, not complete canonical semantic coverage
or authorization to describe unmerged PR17–20 features as available. Actual
publication still requires final-head GitHub CODE review, trusted CI, merge
and a new live Pages receipt.

The finite independent cumulative peer review at `527d00d` found no actionable
issue; its original receipt is preserved as
`reviews/publication-137-independent-review.json`. It checked all four
foundation pages, reused the prior six-page receipts, matched source/page/
provenance hashes, independently inspected generated navigation/search/HTML
and current release metadata, and ran the supplied focused core/model checks.
Copying that receipt changes only authoring evidence. Final-head GitHub CODE,
trusted CI and actual publication proof remain separate requirements.

## Language integration on the reviewed 137-page guide

PR17 integrates main `325007c` once, preserving all forty PR31/32 chapters,
their tests and navigation. Its two implemented Language pages bring the
combined source to 139 current pages and 48 drafts. The existing guide JSON
receipts retain their historical source hashes and counts. This integration
requires its own exact-head review and trusted validation before publication.
## Local Help and recovery explanations

Three local Help pages provide sixteen FAQ answers, ordered troubleshooting
and an About/support/contribution orientation. They raise authored content
to 140 completed pages with 47 drafts; public content remains 127 at this checkpoint
while PR32's 137-page source is in trusted validation. The broad glossary
remains draft. The Help ledger maps source and canonical-topic evidence,
keeps current session-history/provider/regex/platform limits explicit, and
records four existing focused history checks plus eight publication checks
and seven generated-doc browser samples. No tests were added to mirror
plain prose, and no full suite was repeated. These pages are local-only,
not native/player/runtime parity or public 140 qualification.

The finite independent Help review at `eb37f30` found no actionable issue.
Its original receipt is preserved in `reviews/help-independent-review.json`;
it verified the source/page hashes, recovery and identity boundaries, focused
existing history checks and generated navigation/search/HTML exclusion.
These three pages remain local-only and require actual-main reconciliation
and a fresh final-head publication review when later integrated.

## Actual 137-page publication and local credit recipes

`publication-137-2026-10-16.json` preserves the actual PR32 Pages/public
receipt for source `325007c`: successful exact-source deployment, all 137 HTML
and 19 referenced assets matching the reviewed build, the 137-route/1,116-entry
search index, and eleven actual public Chrome checks with no observed errors.
Root independently accepted the final closed-drawer mobile visual and search/
ten-new-route proof. The first homepage locator assumed a relative attribute;
observed response/DOM evidence identified Material's absolute-URL normalization,
and the corrected actual navigation passed. No site/source change was needed.
Application release/packages remain separate evidence.

Three further local recipes explain a reviewed primary album credit, a strict
numbered-title prefix and a leading-article sort field. The writing milestone
is 143 completed / 44 drafts, while the public site remains 137 completed / 50 excluded
drafts. Five real core tests read thirty-six table rows and eight literal
filter/formula/pattern blocks under LF/CRLF, including empty/nonmatch/list
behavior and actual source-preservation boundaries. They caught and corrected
a local empty-replacement quoting error before any publication. Strict 143,
eight publication checks and eight bounded generated-doc browser checks pass.
The credit ledger records source/canonical examples and explicit limits.
These three recipes, like the prior three Help pages, remain local-only pending source
queue reconciliation and final-head review/publication.

## Local configuration, toolbar and external-tool workflows

`configuration-tools-2026-10-16.json` records three independently authored
chapters on named configuration backups/restores, main/custom toolbar setup,
and external tool definitions. The local checkpoint has 146 written pages and
41 drafts. The actual public milestone remains 137 pages; Help, credit recipes
and these customization chapters are nine local unpublished pages. All 143
previous local navigation entries retain their order. Generated HTML and search
contain exactly the 146 ready routes and 1,207 entries, with no draft leakage.

The examples execute the actual standalone toolbar/ToolsUI models and exact
pure Rust argument/placeholder/invocation helpers, under LF and CRLF. Three new
checks plus ten related existing checks pass. Configuration validation and
sequential replacement paths were read; no native configuration import,
personal-data access or configured external program was executed. Eight
publication tests, strict build and eight sampled local documentation browser
checks pass. Long JSON output cells wrap within bounded columns; literal
Parameter inputs remain separate and exact. An earlier browser/build overlap
caused a missing search-index request; its failed receipt is retained and the
sequential final run has no page errors. This is not public146 verification or
a native/all-platform tool qualification.

`reviews/credit-recipes-independent-review.json` preserves the unchanged finite
independent receipt for the preceding three recipes at exact `2ff5991`. It does
not review this customization trio. The trio still needs its own finite peer
review and eventual source reconciliation, exact-head GitHub CODE review,
trusted validation and actual deployment before publication.

`reviews/scripting-canonical-inventory-2026-10-16.json` preserves the root's
read-only live-primary inventory: 64 unique scripting function names and twenty
exact date-format tokens, including the separate EEEE/EE alias row, are
represented in the seven mature function references. This is **structural
coverage only**. It does not prove that overloads, optional arguments, examples,
edge cases, platform differences or every canonical semantic detail have been
taught. It does not close the broader placeholder or full-guide audit.

## Local file operations, recovery and utilities

`file-recovery-2026-10-16.json` records three completed guides for desktop
copy/move/list removal/trash/permanent deletion, session Undo/Redo/Repeat, and
built-in stored-metadata/container utilities. The 149-page local checkpoint
retains every prior 146 navigation entry in order and leaves 38 drafts. Search
and generated HTML contain exactly 149 ready routes and 1,235 entries, with
no draft leakage. The public milestone remains 137 pages; twelve completed
local chapters await source-queue reconciliation and publication review.

Forty actual production model/controller checks pass, including four written
mixed-format utility examples under LF and CRLF. Seven existing history
fixture tests and one unknown-ID3v2-frame cleanup fixture pass through retained
core executables; no cold Cargo build or full matrix was repeated. These use
temporary generated files, not personal audio. File-operation controller
checks stop at a stubbed IPC seam: no native OS trash, playback, reveal or
configured external tool was executed. Source reads establish narrower
transfer, journal and format-utility contracts; they are not universal
filesystem-alias, overwrite, rollback, repair or player guarantees.

The strict build and eight publication checks pass. Seven sampled local
browser checks cover related navigation, search, light/dark reading and
390px tables without page errors or document overflow. This does not qualify
public149 or native behavior. The existing same-size/preserved-date Undo
explanation is retained, with pending edits, partial failure, unrecorded file
operations and captured side-image limits explained separately.

`reviews/config-tools-independent-review.json` preserves the unchanged finite
peer receipt at exact `19cb59a` for the preceding customization trio. It does
not review these three new pages. Current app source remains byte-identical
main `325007c`; pending feature branches and original deferred P2 work are
untouched. Full canonical semantic completeness remains unproven, including
overloads, edge cases, platform qualifications and the remaining drafts.

## Local field values, Library and Tools-menu checkpoint

`library-values-2026-10-16.json` records three completed chapters: reusable
field-value choices, the Library cache workflow and the full current Tools
menu. The local milestone has 152 ready pages and 35 drafts. All prior 149
local navigation entries remain in order; generated HTML/search contain
exactly 152 routes and 1,258 entries with no draft leakage. Public coverage
remains 137 pages. Fifteen completed local chapters are unpublished pending
source-queue reconciliation and qualification.

Twenty-eight production model checks pass, including actual written
normalization/preference examples under LF and CRLF and an inventory guard
for all five fixed Tools-menu entries. That guard is structural, not a
semantic-completeness or native-UI certificate. Five focused temporary-DB
fixtures pass against a narrow Library test target freshly built from this
main `325007c` source. Retained binaries in feature worktrees were not used
because their core source differs. No full/native Cargo matrix, personal
collection scanning, configured program launch or OS action was performed.

Strict build, eight publication tests and eight sampled local browser checks
pass. The browser sample covers light/dark reading, related navigation,
search and 390px tables, including an actual horizontal wheel to reveal the
Tools scope column; no page errors or document-level overflow were observed.
The author viewed the final samples. This establishes local documentation
behavior, not public152, native performance, provider permission or file
save-back qualification. Temporary browser profiles and server were closed.

The chapters distinguish saved list preferences from pending/saved metadata,
cache lookups from loaded-list Find/filter, and immediate maintenance/export
from the outer Options dialog. The Library status counters are explained as
cache-session hints rather than a complete I/O audit. Configuration backups
retain settings and custom lists but exclude the cache and audio files.

`reviews/file-recovery-independent-review.json` preserves the unchanged clean
finite receipt at exact `9e0d7d7` for the preceding trio, not this new batch.
The actual public137 proof and structural-only scripting inventory remain
unchanged. This checkpoint needs finite independent review and later current
source reconciliation, final-head CODE, trusted CI and actual publication.
Full canonical semantic completion and the remaining 35 drafts are still open.


## Local text-exchange recipe checkpoint

The next bounded batch completes three recipes: structured tracklists,
per-file lyrics sidecars and spreadsheet CSV reports. The local milestone
has 155 authored pages and 32 drafts. It preserves every prior 152-page
navigation entry in order. Generated navigation and search agree on all
155 routes, with 1,289 indexed sections and no draft navigation, HTML or
search leakage. The actual public site remains the separately verified
137-page publication; eighteen additional chapters are local only.

The tracklist recipe teaches literal extraction masks, discarded numbering,
selection-order association, per-album source-file evaluation, reordered
filename keys, partial selection and the limits of CSV-like input. Lyrics
teaching distinguishes a whole-text, per-file immediate action from that
record importer, including naming, replacement, empty/missing files,
encoding and continued action processing after warnings. The spreadsheet
workflow keeps stable file keys, quotes every arbitrary cell, preserves
Unicode and record endings, and explains text import and the absence of an
automatic write-back link. Native dialog/player/spreadsheet execution is
not inferred from core output tests.

Six new focused core tests execute the actual written records, formats and
outputs under both LF and CRLF Markdown checkouts. They cover reordered and
partial keys, duplicates, source/sidecar paths, whole-text decoding,
Unicode/quote escaping and multiline CSV cells. The narrow target is built
from application source byte-identical to actual main325 in the shared
guide-only target. Its Clippy check passes; no modified17–20 native binaries
are reused. Twenty-four existing action/export UI model tests, eight
publication tests, the strict guide build and whitespace checks pass.

Browser QA uses a separate temporary headless Chrome profile against the
local generated guide. It verifies palettes, actual reference navigation,
390px tables/document bounds, horizontal template scrolling, search,
complete numbered procedures and copyable lyric-filename code. An initial
Markdown indentation error in the lyrics steps was corrected before the
final checks. A later harness selector counted breadcrumb list items too;
the selector was narrowed to the actual procedure. Neither an application
change nor a claim of native UI/import or public155 validation is involved.
Owned browser profiles and the local server are closed after validation.

The ledger `text-exchange-2026-10-16.json` records exact source/page/test and
binary hashes, canonical topic pointers, commands, results and limits.
The preceding finite clean Library/value/menu review is retained unchanged
in `reviews/library-values-independent-review.json`. Full canonical
semantic coverage, the remaining32 drafts and actual publication still
require further work. This local batch awaits its own independent review,
then eventual source reconciliation, final-head CODE and trusted CI.


## Local learning-workflow checkpoint

Three more source-grounded tutorials complete the local learning sequence
for tags from filenames, a bounded album cleanup and per-album cover art.
The local milestone is158 authored pages and29drafts, while the verified
public site remains137. All prior155navigation entries retain their order;
search/navigation contain exactly158routes and1321sections without draft
HTML, search or navigation leakage. These counts do not prove canonical
semantic completion, and21completed local chapters remain unpublished.

The filename tutorial teaches matching versus meaning, trailing folder
levels, literal separators, discarded values and a Deluxe-name miscapture
before relating that extraction to Guess Values in an existing tag. The
cleanup tutorial surveys a copied album, filters absent Album Artist,
builds six explicit ordered actions and verifies retained metadata before
separately staging/saving track numbers and applying a reviewed rename.
The cover tutorial distinguishes embedded pictures from sidecars, clears
hidden selections, imports each album's own image, avoids accidental repeat
imports and separates all-cover resizing from selective Manage covers plans.
The online covers-only route is source-grounded; no live provider success
or player compatibility is inferred from the local examples.

Seven focused actual-core tests execute the written patterns, table rows
and controls under LF/CRLF Markdown, including temporary MP3 files with
different images per album. They verify that embedded artwork changes while
audio bytes and source images remain intact, plus count filters and actual
resize dimensions/no-op decisions. Thirty-four existing action/cover/column/
numbering models,16source-routing/matching models, eight publication tests, strict build, focused new-test
Clippy, formatting and whitespace checks pass. The narrow test target uses
application/dependency/workflow source byte-identical to main325; it does
not reuse the altered17–20 native binaries or claim a fullworkspace matrix.

Ten generated-doc browser checks cover palettes, the Deluxe counterexample,
actual Guess Values navigation, all seven/eight/eight lesson stages,390px
bounds/tables, the horizontally scrollable six-action table, the copyable
cover filter and actual search navigation. Visual inspection prompted use
of the existing native-mapping-table wrapper so regexes and field identifiers
stay intact when the table scrolls on a narrow screen. No CSS/app change was
needed. A numeric-heading CSS selector and a programmatic-focus search
interaction were corrected in the isolated test harness before the final
clean run; failures are retained alongside the local browser receipt.
Owned temporary browser profiles and the HTTP server are closed.

The ledger `learn-workflows-2026-10-16.json` records source/page/test and
binary hashes, canonical topic pointers, commands, outputs and limits.
The preceding finite clean text-exchange peer and root visual receipts are
retained unchanged under `reviews/`. This trio awaits its own independent
review; publication still requires later source reconciliation, exact
final-head CODE and trusted CI. Original deferred P2/REA work is untouched.


## Local path and folder-image recipe checkpoint

The next three completed recipes teach lowercase/hyphenated filename stems,
an explicit component-character/length policy and embedded-cover export to
an album-folder image. The local milestone is161authored pages and26drafts;
public137remains separate. All prior158navigation entries retain their order.
Generated navigation and search agree on161routes/1352sections, with no
remaining draft navigation, HTML or search leakage. Twenty-four completed
local chapters remain unpublished; page counts do not establish full
canonical semantic completion.

Lowercase teaching uses the actual Filename→Filename numbered-capture
converter and retains the extension's spelling. Its tag-based variation is
a separately named Tag→Filename route. Portable-name teaching follows the
actual reviewed preview_rename/rename_files UI route and distinguishes raw
$validate character replacement, backslashes, filename-mode sanitizing,
trailing-character cleanup, Unicode scalar truncation and full-path limits.
It makes no universal validity, case-only rename, alias or collision promise.
The original deferred P2s and application behavior are unchanged.

The folder-image recipe states the JPEG/.jpg versus PNG/.png distinction
before its steps: exporting copies bytes and does not convert encoding.
It separates per-input-folder outputs from report partitioning, repeated
content from existing-file replacement and action-specific external-image
undo from other export paths. The real NativeHost extension/ordinal/write/
ownership/capture flow and session-journal caller were read. No private
host behavior is replaced with a mirrored test implementation or presented
as a new native-app/OS export run.

Five focused core tests execute the actual written patterns, case tables,
Unicode shortening and ExportCover host requests under LF/CRLF Markdown.
Two exact existing temporary side-file journal tests verify creation/
replacement undo and unchanged-output no-op recording, without exercising
the unrelated original P2 cases. Thirty-one actual converter/action models,
eight publication tests, strict build, focused new-test Clippy, formatting
and whitespace checks pass. All175production/dependency/workflow files in
the identity receipt remain byte-identical to main325.

Nine isolated generated-doc browser checks verify the real procedure lists,
correct converter-reference link,390px tables/document bounds, actual code
scrolling, palettes, copyable per-track cover format and actual search.
Owner viewed the nested-expression mobile procedure and JPEG/PNG teaching
in dark mode. All temporary profiles and the HTTP server are closed.

All161completed pages were scanned across their entire contents for the
known original draft-brief markers, with no findings. The final aggregate
binds every ready path/content hash; each new file was replaced completely
and read to its end. The ledger `path-recipes-2026-10-16.json` records these
receipts, exact source/page/test/binary hashes, source pointers and limits.
The previous clean25bdlearning followup and footer-repair checkpoint are
retained unchanged under reviews/. This trio awaits independent peer, later
main reconciliation, exact final-head CODE and trusted publication checks.


## Local desktop-platform checkpoint

The macOS, Windows and Linux platform guides are now substantive local
chapters. The candidate has 164 authored pages and 23 drafts, preserving every
prior 161 navigation entry in order. Navigation/search agree on 164 routes and
1,378 sections with no draft leakage. All 164 completed pages have a clean
whole-content original-brief-marker scan. Public 137 remains separate;
27 completed local chapters are unpublished and counts do not establish full
canonical semantic coverage.

The guides distinguish actual packages/processor builds from framework
possibilities or an OS support matrix. They explain installation/runtime
failures versus audio-file errors, literal paths and encodings, system
file-manager launches versus interactive opening, physical Control versus
Command/Option/AltGraph, tray/window behavior, optional credential stores,
per-user settings and separate save/write boundaries. Source-specific limits
are kept visible instead of transferring another editor's platform behavior.

Read-only downloads of three actual v0.2.13 assets match the verified manifest's
SHA256 and sizes: Apple-silicon app archive, Windows installer and x64 DEB.
The Mac bundle displays an ad-hoc signature/version and an arm64 Mach-O; its
packaged document associations are Alternate. Its declared minimum-version
metadata is not promoted to a tested OS guarantee. The installer has no
embedded Authenticode certificate table; the NSIS stub's 32-bit PE is not
misrepresented as the application's processor target. The DEB has an
x86-64 ELF64 payload, actual GTK/WebKitGTK/Ayatana dependencies and an OpenTag
desktop entry. These are archive/header declarations, not installation,
Gatekeeper assessment, runtime download or cross-OS launch tests. No app,
installer, keychain, mount or user security setting was invoked/changed.

Three focused actual-core tests execute the written path format,
known-codepage hex examples and volume-label display fallback under LF/CRLF.
Thirty-four Node checks pass: four new guide-contract checks plus 30 existing
keyboard/file-opening/association models. The new checks run 20 written
shortcut rows in both newline forms through actual source routing, verify
all 11 desktop filenames against the published manifest, bind Linux declarations
to the inspected DEB and exercise frontend replacement/pending-edit handling
for the written folder paths. They do not pretend to run the OS commands.
Eight publication tests, strict build, focused Clippy, formatting/whitespace
checks pass;178 recorded production/dependency/workflow/config files match main325.

Nine isolated generated-doc browser checks verify new Platforms navigation,
real procedure lists,local search opening, palettes,390px tables/document
bounds, no-wrap package names, actual horizontal scrolling and copyable
checksum code. Owner viewed the complete mobile Mac filenames and Windows
dark teaching. A stale search-result handle was replaced with a locator;
a mixed-case architecture keyword observation remained broad/unhelpful,
so that failed scope is retained and search quality is not claimed generally.
The final mounted-path keyword opened Linux guidance. No existing search
implementation was changed. All owned profiles and the HTTP server are closed.

The ledger `desktop-platforms-2026-10-16.json` contains exact source/page/test,
fixture/binary/log hashes, the whole-page aggregate and limits. Package
manifest/inspection metadata are retained in `desktop-v0.2.13-*.json` and the
preceding finite path-recipe peer is preserved unchanged under reviews/.
Official Apple/Tauri/AppImage guidance was checked for the scoped app-opening
and runtime procedures. This trio awaits peer and later actual-main
reconciliation, exact final-head CODE, trusted CI and publication. Original
P2/REA work and source17–20's feature behavior remain untouched.


## Publish the 164-page reviewed candidate

The user's request to publish the detailed guide lifts the earlier local
source-queue hold. This publication candidate starts from actual main325 and
integrates exactly the 27 independently peer-clean chapters through e297.
The next automation/HTML/Cookbook drafts are separately checkpointed at07af
with draft status and are excluded. Historical author/peer receipts retain
their original hold language; this entry records the later authorization.

All nine finite clean peer receipts are preserved, including the corrected
25bd learning footer followup and the desktop-platform peer. Each of the27
current chapter bytes matches its exact peer-reviewed commit. No other
reader-facing page is changed. Source/config/dependency/workflow identity
is checked against325 rather than inferred from the old writing branch.

Fresh combined checks pass34 Rust example tests across all five new guide
targets and the required existing user_guide target,10 new guide-contract
Node tests, eight publication tests, strict build and focused Clippy.
Navigation/search have164ready routes and1378sections;23drafts remain out of
HTML/navigation/search. All prior137navigation entries retain their order.
Every one of the164built HTML pages and search JSON is byte-identical to the
reviewed e297 build, so its scoped browser evidence is reused with its
existing qualifications. All164completed pages have no original draft-brief
markers. No native app/OS/browser run or public164 deployment is invented.

`publication-candidate-164-2026-10-16.json` binds the peer chain, exact page
hashes, output/source aggregates, fresh commands/logs and remaining gates.
Open a ready PR only after this final local checkpoint. GitHub must still
complete a clean CODE review of that exact final head before the trusted
controller authorizes current-main/testmerge CI. Root owns the protected
merge and actual release/Pages verification. Full canonical semantic
completion, the remaining23drafts and original deferred P2/REA work are not
claimed complete by this publication step.


## PR18 provider features on actual main

The `pr18-feature-only-2026-10-16.json` ledger records the provider-only
preparation on main `1fe6a800ac1096494b33d3b461286ad2d91e1a13`. It retains
the 164 published chapter entries and adds six completed source chapters.
This checkpoint excludes the independently reviewed, unmerged PR17 Language
implementation. Existing guide receipts continue to describe their exact
source baselines; they are not new native, live-provider or publication proof.

## PR #20 source reconciliation

`docs/verification/src-compat-direct.md` records the direct-main compatibility
change and fresh qualification. The five affected format, regex and web-source
pages describe the revised grammar, cursor, JSON and output behavior; their
written native examples pass under LF and CRLF. This reconciliation does not
claim a complete downloaded-script corpus or full semantic Mp3tag parity.

## Local combined provider and compatibility preparation

The local integration preparation combines the independently prepared PR18
providers, PR19 gnudb workflow and PR20 web-source compatibility changes. It
preserves the combined 172 completed navigation entries and 15 drafts; PR17
Language behavior remains separate. This is preparation for reconciliation
against the actual feature merges, not evidence that all three are merged or
published. Earlier per-PR receipts retain their pinned source and limitations.

## Language integration on the reviewed 164-page guide

PR17 also integrates actual reviewed main `1fe6a800` once, preserving all 27
PR33 chapters, their five native example targets, four frontend guide-contract
files and the complete 164-page navigation order. Its two source-backed
Language pages bring the combined candidate to 166 current pages and 21 drafts.
Application source, catalogs and existing tests are unchanged from the locally
qualified `b7ea3ca` checkpoint; the incoming guide fixtures are qualified on this
combined source using PR17's own target. Historical guide author/peer JSON
receipts retain their original source identities, counts and publication limits.
This integration requires a fresh exact-head CODE review and trusted validation;
local guide counts do not establish publication or full semantic completeness.

## Local Language preparation on all three source features

The background integration now retains the 172 feature-guide navigation
entries plus the two Language chapters: 174 completed pages and 13 drafts.
It combines local feature checkpoint `92268df` with translation checkpoint
`6fe1eed`. These are local preparations, not actual merged-main or public
availability claims; historical receipts retain their original sources.
