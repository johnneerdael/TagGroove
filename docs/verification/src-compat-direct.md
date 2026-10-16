# Community web-source compatibility on main

This feature branch applies PR #20 directly to main
`1fe6a800ac1096494b33d3b461286ad2d91e1a13`. It preserves main’s English error
transport and existing providers; PR #17’s localization and PR #18/#19’s
provider additions are not dependencies.

## Resulting behavior

- Load source archives from bounded subfolders, resolve include/settings names
  without case sensitivity, and accept chained SearchBy triplets.
- Preserve literal parentheses in format-function arguments and support the
  documented command, cursor, current-line replacement and output conventions.
- Read the first JSON value with ordered object members, preserve the text
  cursor when leaving JSON mode, and tolerate unselecting at the root.
- Decode output character references, handle TRACKS/default OUTPUT buffers,
  substitute album URLs and interpret plain _LENGTH numbers as milliseconds.
- Update the existing tutorial, command reference, format and regex guidance
  to describe those behaviors. Existing copied-file/history/write safeguards
  remain the baseline.

## Local verification

A fresh isolated target rebuilt all five workspace packages after cleaning
copied workspace artifacts; dependency caches alone were retained. The full
workspace passed 911 Rust tests, with five explicit ignored tests, and all-target
Clippy passed with warnings denied. All 368 frontend tests, the production
TypeScript/Vite build, eight publication tests and strict guide build passed.
The guide still publishes 164 completed pages and retains 23 drafts.

The actual written unquoted `(Interlude)` example now succeeds, and JSON-root
unselection succeeds. These are intentional changes from main’s earlier
rejection behavior. Tests execute the written examples with LF and CRLF.
The optional downloaded-community-script compatibility test remains ignored;
no downloaded corpus, live-service, native UI or mobile run is claimed here.

An independent finite source review compared the extraction with its preserved
feature delta and found no lost intended behavior or localization dependencies.
Local review does not replace the clean exact-head GitHub Codex code review,
trusted CI, normal merge or subsequent release/Pages publication checks.

## Named character-reference review correction

Review of the first direct-main head found that the handwritten entity subset
left valid names such as `&Alpha;` untouched. The importer now uses the complete
`web_atoms` 0.2.6 table already present in the lockfile, with no dependency-version
updates. An independent comparison against the primary WHATWG
[entity data](https://html.spec.whatwg.org/entities.json) matched all 2,231 entries,
including all 2,125 semicolon-terminated names and 93 two-scalar expansions.
The importer requires the terminating semicolon, matches case exactly, and
performs one decoding pass; this is character-reference decoding, not an HTML
parser. Unknown references remain literal and existing numeric/Unicode escape
handling is retained.

The actual source-import regression first failed for Greek, non-BMP and
multi-scalar values. It now passes alongside the long-name, numeric, unknown,
case and single-pass examples. All 31 web-source tests passed (one optional
corpus test ignored), followed by the full 911-test workspace, all-target Clippy,
eight publication tests and the strict guide build. The frontend implementation
is unchanged from its 368-test production-build checkpoint. A finite independent
peer checked the decoder, dependency/table provenance and written contract.

## Local preparation with provider and gnudb changes

A separate local prep combines provider/gnudb checkpoint `4d8912e` with this
PR20 source at `661830e`. The only merge conflict is the guide coverage note;
both per-PR sections are retained. PR20 production code, dependency declarations
and tests remain byte-identical to their feature head, while the combined
provider routes, authentication and gnudb query-before-read behavior remain
unchanged from the checked PR18/PR19 prep. The `web_atoms` dependency uses the
already-locked 0.2.6 table; no dependency version is changed by this merge.
This preparation excludes PR17 localization and makes no combined GitHub
review, trusted CI, actual-main, release, live-provider or publication claim.


## Complete-address review correction

A subsequent review found that a direct source using one bare `%s` search
field and no album base encoded a complete stored URL as search words. The
actual-source regression failed with `?`, `&` and `#` changed into path escapes;
a separate exact-`%s` preview regression also failed by escaping an existing
`%2F` again.

The direct-address branch now preserves that one complete value. The same
branch applies to an exact `%s` preview, while nested preview templates,
ordinary searches, album-ID templates and chained fields retain their normal
word encoding. The existing caller still parses the address and applies the
HTTP(S)/no-embedded-credentials policy. Tests check query/fragment/existing
escapes, unsafe-address rejection and the unchanged encoded contexts.

The focused app web-source suite passes 34 tests with one explicit optional
corpus ignore. The native guide harness passes 27 tests, including its actual
source-definition and written-command examples. That harness imports the
production parser as source, so URL-policy assertions belong in app tests;
the pure source tests remain dependency-free. Its standalone JSON-order test
also exposed a missing test feature: tag-core's dev-only serde_json dependency
now explicitly enables preserve_order, matching the actual app and existing
full-workspace feature context. Standalone and workspace guide runs both pass.
No production core code, dependency version or lockfile entry changed.

The preceding complete-entity checkpoint's full 911-test workspace run is
historical evidence, not a newly executed full workspace run for this URL
correction. The frontend remains unchanged from its 368-test/build checkpoint.
Fresh all-target Clippy, the strict guide and eight publication tests qualify
the bounded correction; exact-head GitHub Codex review and trusted CI remain
required. No live provider/native desktop replay or general browser URL
recovery behavior is claimed by these parser tests.
