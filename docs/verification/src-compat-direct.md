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
