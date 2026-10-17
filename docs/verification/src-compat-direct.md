# Web-source compatibility qualification

The PR20-only results below describe their stated source checkpoints. This
local PR17 preparation also includes localization and typed diagnostics; its
current integration status is recorded separately at the end of this note.

The PR20 candidate summarized here integrates with actual main
`21de24598ac0170faa5be7f4e408389c69af30ac`. GNUdb, the additional online
providers and neutral guide publishing are already part of that baseline.
The remaining feature delta is scripting/source compatibility and its guide
examples; PR17 localization and typed diagnostic integration are not included.

## Resulting behavior

- Load source archives from bounded subfolders, resolve include/settings names
  without case sensitivity, and accept consecutive SearchBy triplets.
- Preserve balanced literal parentheses, dollar anchors and numeric captures in
  format-function arguments.
- Read the first JSON value with ordered object members, preserve the text
  cursor when leaving JSON mode, and tolerate unselecting at the root.
- Correct current-line replacement, search and output-buffer behavior, including
  TRACKS/default OUTPUT conventions and plain _LENGTH milliseconds.
- Decode the complete semicolon-terminated named HTML reference set and Unicode
  escapes. Preserve complete direct lookup and preview URLs in their narrowly
  identified complete-address contexts.
- Teach those behaviors in the format/regex reference and three web-source
  guides. Existing file-write and history safeguards remain the baseline.

## Local qualification and final integration

The fresh native qualification at `3aab350a45ce6ff3966857ebf94d3ae15d634758`
rebuilt all five workspace packages in its isolated target. It passed 973 Rust
tests with six explicit ignores, full-workspace/all-target Clippy with warnings
denied, the production TypeScript/Vite build, publication tests and the strict
172-page guide build. The original low-disk Undo failure and successful rerun
are retained separately; no application change was made for that environment
failure. The later fixture/cleanup and provider-exit changes affect the test
harness; the 606 recorded native/build input hashes remain unchanged.

The subsequent `dae5e69ae84fd646f239a15e6b8a597672b6c20f` checkpoint freshly
passed all 384 UI/model tests, including the actual provider-script exit
regression. Its unchanged review-policy/harness inputs retain 84 passing
Python checks. The final merge of actual main adds only the corrected publishing
recovery instructions; this verification note is updated to identify that base.
Production code, dependency files, native/build inputs and all 172 reader pages
remain byte-identical to those qualified checkpoints. Counts above retain their
stated source/run scopes; they are not new full native executions on this head.

The ignores are the system-trash probe, benchmark, three live-service checks
and optional downloaded-script corpus. These local results do not establish
live API availability, native desktop replay, mobile source behavior or
compatibility with every downloaded community script. Exact final-head GitHub
Codex code review and trusted validation against current main are still required
at this local checkpoint. Normal merge, release and live Pages verification are
separate outcomes.

## Review corrections retained in the feature

The initial handwritten character-reference subset left valid names such as
`&Alpha;` untouched. The importer now uses the complete `web_atoms` 0.2.6 table
already present in the lockfile, with no dependency-version changes. An
independent comparison with the primary WHATWG
[entity data](https://html.spec.whatwg.org/entities.json) matched all 2,231
entries, including 2,125 semicolon-terminated names and 93 two-scalar
expansions. Decoding requires the terminating semicolon, is case-sensitive and
performs one pass; unknown references remain literal. Actual regressions cover
Greek, non-BMP, multi-scalar, long-name, numeric, unknown, case and single-pass
values.

The complete-address correction is limited to a direct source with one bare
`%s` field and no album base, and its exact-`%s` preview. It preserves queries,
fragments and existing escapes. Nested preview templates, ordinary search words,
album-ID templates and chained fields retain normal word encoding. The caller
still parses addresses and enforces HTTP(S) without embedded credentials.
Actual regressions cover both preservation and rejection/unchanged contexts.

The source-definition guide tests use the production parser as source.
Tag-core's dev-only serde_json dependency explicitly enables preserve_order,
matching the application and full-workspace feature context. No production
dependency version or lockfile entry was changed for that test feature.

## Bounded work for long literal inputs

Two further review findings concerned repeated work, not a new script feature.
The argument parser rescanned a literal's trailing backslash run before every
character. It now maintains that run's odd/even parity incrementally and resets
it when the literal ends. Quotes, fields, optional sections, nested calls and
literal dollar references keep their existing parsing boundaries. The value
decoder previously lowercased and copied the remaining buffer for every
`$verticalBar()` marker. It now scans once, matches that fixed marker using
ASCII case rules, and preserves UTF-8 text, nonmarkers, the private pipe marker
and final trimming. No dependency or grammar change is involved.

Bounded old-code probes retained their small semantic results but exceeded a
two-second child-process deadline for each near-limit input; the child was
terminated. The core probe used the previously qualified core library; the
second probe compiled the unchanged private decoder body and its marker
constant. This is local source/core evidence, not native UI or platform timing
proof. The committed regressions assert exact values, without timing thresholds:
a backslash argument just below the 1 MiB format limit, a marker-filled value
just below the 16 MiB parser-output limit, odd/even literal runs across node
boundaries, and Unicode/case/nonmarker preservation.

Fresh qualification of this correction passed **977 Rust tests**, zero failures
and six explicit ignores across 67 result summaries, full-workspace/all-target
Clippy with warnings denied, **384 UI/model tests**, and the production build.
The existing PR20 target and task-specific temporary directory were used; no
workspace cache was borrowed or cleaned. Available space was checked before
compilation, after compilation before history tests, after the full test run
and after Clippy, remaining above 2.3 GiB at each point. The benign existing
Cargo global-cache cleanup permission warning did not fail either command; no
permissions or global caches were changed to suppress it. Full-platform trusted
CI and a fresh exact-head Codex code review remain required after this source
change. Earlier 973-test results above describe the earlier source only.

## HTML numeric-reference normalization

The decoder now applies the [HTML numeric-character-reference end-state
rules](https://html.spec.whatwg.org/multipage/parsing.html#numeric-character-reference-end-state)
to valid semicolon-terminated decimal/hexadecimal references. It reuses the
`web_atoms` 0.2.6 C1 replacement table already in the dependency: `&#128;`
becomes `€`, and null, surrogate or out-of-range code points become `�`.
Digit validation distinguishes malformed payloads from overflow; a valid digit
sequence that overflows the integer parser is out of Unicode range. The five
unmapped C1 controls and other valid scalars, including noncharacters, are
retained. All 27 replacement pairs in the locked library were independently
compared with the current WHATWG table.

This remains a semicolon-required metadata decoder, rather than a full HTML
tokenizer with missing-semicolon or partial-reference recovery. Malformed or
unknown references remain literal, named-reference case and single-pass
handling stay unchanged, and the separate backslash-u escape path is unchanged.
No dependency, parser grammar, URL policy or field-write behavior was added.

Three regressions cover all 32 C1 entries in decimal and hexadecimal forms,
invalid/overflow scalar references, ordinary and non-BMP values, controls and
noncharacters, malformed/missing-semicolon input, and single-pass/backslash-u
boundaries. The production release-conversion fixture checks album, track and
comment values. Before the correction, the unchanged extracted decoder failed
the euro and null-reference assertions; the corrected helper and actual
workspace fixtures pass.

Fresh qualification passed **980 Rust tests**, zero failures and six explicit
ignores across 67 result summaries, plus full-workspace/all-target Clippy with
warnings denied, the strict 172-page guide and eight publication tests. Native
artifacts were produced from this worktree in its existing exclusive target,
with reserve checks before compilation and before/after history tests. The
unchanged frontend retains the 384 UI/model tests and production build qualified
at `80cad18d21a43c456fe9ec3c24c41ff2b4c11223`; those are not claimed as new runs
for this decoder-only correction. A new exact-head Codex code review and trusted
platform validation are still required before merging.

## Retained-line join growth

`JoinLines` and `JoinUntil` now share a cumulative 16 MiB append budget for one
script run. Repeated and overlapping copies count toward that budget; replacing
input does not reset it. Checks precede allocation and mutation, and successful
joins append directly while preserving retained lines and cursor positions.
Six regressions cover bounded repeated loops, the exact byte limit, UTF-8,
refusal state, navigation, overlap and input switches. Two bounded old-code
probes failed the expected-limit assertions; all 21 updated engine tests pass.

Fresh workspace qualification passed **992 Rust tests**, zero failures and six
explicit ignores across 67 result summaries, plus all-target Clippy with warnings
denied, the strict 172-page guide and eight publication tests. The unchanged
frontend retains the 384 UI/model tests and production build qualified at
`80cad18`. Stale dependency build metadata first prevented compilation; scoped
regeneration in this worktree's own target recovered it without source changes.
Disk guards held testing until sufficient reserve was available. Both tests and
Clippy then exited successfully; the final reserve check fell below the
conservative 2.3 GiB threshold while remaining above the journal's 2 GiB minimum.
Root subsequently archived only superseded test executables to restore headroom.
A fresh final-head Codex code review and trusted platform validation remain
required.

## Earlier local provider and gnudb preparation

The following describes an earlier preparation, before the current PR17
integration. It is retained as historical provenance.

A separate local prep combines provider/gnudb checkpoint `4d8912e` with this
PR20 source at `661830e`. The only merge conflict is the guide coverage note;
both per-PR sections are retained. PR20 production code, dependency declarations
and tests remain byte-identical to their feature head, while the combined
provider routes, authentication and gnudb query-before-read behavior remain
unchanged from the checked PR18/PR19 prep. The `web_atoms` dependency uses the
already-locked 0.2.6 table; no dependency version is changed by this merge.
This preparation excludes PR17 localization and makes no combined GitHub
review, trusted CI, actual-main, release, live-provider or publication claim.

## Local PR17 integration awaiting qualification

The local translation preparation at `efa7e868aea2365b66848da78dd9866cbf6f1d08`
now incorporates PR20 checkpoint `403bf9a2275075d29f5685947c604c9928d554cb`,
which contains actual main `21de24598ac0170faa5be7f4e408389c69af30ac`.
The literal-parser, numeric-reference and retained-line growth corrections retain PR17's typed
errors, raw diagnostic data and existing localization boundaries. The sole
merge conflict joined historical verification notes. All 2,926 existing catalog
entries, Windows fallback controls and hosted-only cleanup are retained. The
new owned join-limit diagnostic has Dutch and German translations; catalogs
now contain 2,927 entries. The six incoming join regressions remain unchanged.

At the source-only preparation checkpoint, no new native or history tests had
run for that merge. Earlier PR17 and PR20 counts retained their original source
scopes; further PR20 review corrections and actual-main adaptation were still
pending. That checkpoint made no remote-review, merge, release or publication
claim.

The local translation merge additionally passed 537 UI/model tests and the
production build, including the existing search/release dialog regression
extended with both join-limit errors under Dutch, German and custom English.
Its raw command and filename roles and single-pass translation checks remain
active. Catalog extraction is stable at 2,927 entries with all previous entries
preserved. The strict local 174-page guide and eight publication tests pass.
These lightweight checks do not requalify the native interpreter or history
paths; the retired earlier PR17 targets are not live caches for this source.

## PR17 qualification after the actual PR20 merge

Actual main `5c49582da4d4e6ba1e63e5fb2f7ce66c1918b0bf` merged PR20 head
`403bf9a2275075d29f5685947c604c9928d554cb`. The local PR17 preparation already
contained that source. Merging actual main left all 1,385 tracked qualification
inputs byte-identical to `fc6cf5e08b1dca6b1ce88594ad0877177e3a19bc`; this
verification note is the only subsequent documentation change.

Fresh local qualification at `fc6cf5e` passed 1,061 Rust tests, zero failures
and six explicit ignores across 68 summaries, plus workspace/all-target Clippy
with warnings denied. All five workspace packages were freshly compiled from
PR17 source using the stable shared target path after cleaning only the old
PR20 workspace packages; dependency paths were retained and incremental
compilation was disabled. Reserve checks before native/history tests and
before/after Clippy exceeded 2.3 GiB.

Runtime, frontend, tests, catalogs, reader pages and build configuration remain
unchanged. The same-source 537 UI/model tests, production build, 2,927-entry
catalog checks, strict 174-page guide and eight publication tests therefore
retain their qualification. These are local results; exact-final-head GitHub
Codex review and trusted current-main platform CI remain required. Normal merge,
release and public guide deployment are separate outcomes.
