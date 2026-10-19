# Four approved parity repairs

These repairs follow the pinned static analysis and independently prepared
local oracles recorded in this PR. Native findings remain static evidence;
owned Rust/backend tests establish the repaired TagGroove behavior. They do
not establish original-application runtime or Windows filesystem parity.

## Scripting and export

<a id="c51"></a>

The evaluator now tracks sequential presence separately from visible text.
Arguments evaluate eagerly from left to right, including unused selector
branches. Predicates publish empty text and replace the enclosing found state;
optional children suppress absent output. Genuine unknown functions preserve
their authored spelling, evaluate and discard arguments once, then replay the
original syntax in the same enclosing frame. Returned field or host text is
never parsed as new syntax.

Public extensions retain independent presence/truth flags and external-error
provenance. The private export variable adapter stores strings, uses ASCII
case-insensitive names and derives lookup presence from the stored text. It
does not change public extensions named `get`, `put` or `puts`. Existing input,
output, depth and arity limits remain; a shared 1,048,576 AST-visit limit bounds
eager replay. Malformed identifiers and reserved native-known functions still
fail explicitly. Context helpers `getenv`, `columnnames` and `columnvalues`
retain their qualified independent flag contract.

The focused suite passed **56 tests**: 12 presence, six public-extension,
34 export and four diagnostic/budget tests. Additional review regressions
first failed for context-argument truth leakage and malformed export names;
both were fixed and passed independent scoped re-review. Evidence is retained
under `taggroove-scripting-green-final-aen4zery`, with source bindings, commands,
executable hashes and raw logs. The original failing probes remain unchanged.

Numeric, Unicode, direct callback, re-entry, column self-reference and other
unproved native semantics retain their separate qualifications. See the
[presence baseline](script-presence-static-2026-10-19.md) and
[unknown-function analysis](unknown-functions-static-2026-10-19.md).

## Playlist location identity

<a id="c53"></a>

M3U/M3U8, PLS and quoted CUE locations preserve extracted authored filename
data. The complete authored candidate family precedes compatibility trimming;
literal candidates precede percent-decoded candidates. URI classification and
refusal occur before local existence probes. The writer escapes relative
comment-looking or whitespace-only members with `./` where playlist syntax
requires it. Per-line M3U BOM handling remains grammar; a BOM inside a PLS
value remains filename data. XML retains its separate location policy.

The focused suite passed **71 tests**, including identity, source order, XML,
chapters, CUE, subsongs and published guide controls. The immutable expanded
68-case probe passed unchanged against the current Cargo-captured library.
All original 42 member/order/byte cases and six line-break refusal sentinels
also passed. That original diagnostic then exited 101 at its historical
aggregate assertion because it expects eight failures and observed zero.
This counter-only harness failure is retained and is not reported as an
exit-zero replay. URI-before-probe controls passed separately.

Evidence is under `/tmp/taggroove-playlist-green-053tfiv5`, including the actual
linked library hash, original sources/executables, raw logs and result JSON.
Physical filename identity was tested on the local Unix filesystem. Windows
aliases, external players and original Mp3tag execution remain unqualified.
See the [preserved diagnosis](playlist-location-identity-gap-2026-10-19.md).

## Lyrics3v2 and dependent tag removal

<a id="c52"></a>

A bounded Lyrics3v2 envelope is recognized at EOF or before a real ID3v1 tag.
The body stays opaque. Logical reads and inventory can now see APE data behind
that envelope without changing physical ID3 offsets. Explicit removal follows
the proved dependency rules, including orphan Lyrics3v2 cases. APE-only Remove8
can remove dependent ID3v2 and regenerate semantic ID3v1; ordinary nonempty
APE-only Write8 preserves the captured suffix. Legacy Lyrics3v1, malformed
lookalikes and unselected bytes stay intact.

The focused suite passed **71 tests** across the Lyrics3 descriptor/Skip,
APE guard, removal, MPEG Save policy, tag policy, raw streams and opaque ID3
preservation. Public regressions first reproduced stale ID3v1 restoration and
an extra blank ID3v1 after an owned placeholder; both repairs passed independent
scoped re-review. Tests assert exact carrier/tail bytes, current inventory,
journal admission and exact Undo/Redo. They establish encoded-byte preservation,
not decoder or original-application execution.

The source-bound receipt is retained under
`taggroove-lyrics3-green-z7aeed2t/receipt.json`, SHA-256
`6b08ec0f9caf074edc87323144043cbe61786fcea96b13b11530036d1545d0f9`.
The distinct Ape/Mpc ID3v2 display question remains a qualification.

## Nonfatal history-startup notice

<a id="c54"></a>

Failed lazy journal initialization preserves the original operation's result
and exposes the existing nonfatal cannot-Undo notice through history state.
Opening history or completing a no-op does not clear it. Failed groups remain
unrecorded; later genuinely recorded work can clear the notice and provide
normal Undo/Redo. Provider-managed and excluded paths retain their existing
behavior. The retained-byte cap and free-space reserve remain **2 GiB each**.

Real backend Save tests inspect nonzero PCM, an opaque RIFF chunk, selected
and unselected files, obstruction bytes, notice persistence, grouped writes
and exact recovery Undo/Redo. All **15 history tests** pass within the **563-test backend suite**. Two
no-op/group edge cases were first reproduced against failing implementations.
The final adapter retains the original notice and every failed-group ID across
repeated startup failures; independent scoped rereview is clean. Source-bound
logs, the frozen backend and `history-final-result.json` are retained under
`taggroove-repaired-workspace-tests-eyixgw5a`. Cross-session retention and stronger stale-file
baseline checks are separate work and are not supplied by this adapter.

## Combined verification

Before the additional cover correction, the repaired workspace passed **1,422 Rust tests**, with **19 intentional ignores**
and no failures. Logs and source bindings are retained under
`taggroove-repaired-workspace-tests-eyixgw5a`; the first failing integration run
is preserved separately. Older unknown-name-as-error test cases were migrated
with exact positive fallback assertions and separate reserved-name, syntax,
arity, numbered-reference and raw-diagnostic rejection controls. Original raw
migration and diagnostic fixtures remain unchanged.

Whole-branch review identified a cover verification interaction: a normal APE
read view could reject a valid ID3 cover edit after unwritten APE retention.
All three real cover-plan cases first failed with that refusal. The correction
checks exact image digest, role and description in the written tag kinds and
retains the non-policy verification path. Its **67 focused tests pass**, including
three new replacement/description/removal cases, retained raw APE/binary data,
encoded MPEG bytes, normal APE read priority and exact Undo/Redo. Proof is under
`/tmp/taggroove-cover-read-masks-o855amwb`. A test-only summary-reader assertion
mistake was corrected to inspect the full artwork; that failing executable and
log are preserved alongside the original semantic RED. Independent scoped
rereview resolved the finding without new defects.

The final frozen source passed these local checks:

| Check | Result | Retained run |
| --- | --- | --- |
| Rust workspace, including doc tests | 1,425 passed, 19 intentional ignores, zero failures | `taggroove-final-workspace-tests-jj2v8_77` |
| Frontend suite | 828 passed | `taggroove-final-ui-_alv1gi5` |
| Workspace/all-target strict Clippy | Passed with `-D warnings` | `taggroove-final-clippy-5-7k8ncokp` |
| Workspace formatting | Passed | `taggroove-final-format-2iqwsp8e` |
| TypeScript and production frontend build | Passed | `taggroove-final-frontend-build-w5y2kemw` |
| Translation extraction consistency | Passed; 3,394 canonical keys | `taggroove-final-i18n-d2277urn` |
| Documentation publication controls | Eight tests passed | `taggroove-final-docs-tests-p7m0kfgt` |
| Guide writing plan | 189/189 pages | `taggroove-final-docs-status-ijnnmn2y` |
| Strict guide build with pinned toolchain | Passed | `taggroove-final-docs-build-do0oo1qa` |

Commands, source bindings and raw stdout/stderr remain in those owned temporary
run directories. The final executable list comes from Cargo's JSON artifact
stream under `taggroove-final-workspace-build-vpenewif`. Compiler-cache
maintenance preserved all proof media, failing executables and archives;
production history limits stayed unchanged. The 58 active intake identities
match 58 active evidence rows, with 54 represented scopes and 14 separate
qualification records.
Focused checks do not substitute for a final-head GitHub Codex code review,
trusted current-main CI, platform checks or a merge.
