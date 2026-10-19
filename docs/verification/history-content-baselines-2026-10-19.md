# Content baselines for history and staged writes

This is the L61 repair in PR64, implemented in
`82a46df82a866c6da57e12b8c526ea9e87e61302`, based on
`9468d3ec9ee09d34923fe28a5ea72ef467ca3df0`.
It supersedes the unsafe behavior recorded in the
[concealed-edit diagnosis](undo-concealed-external-edit-2026-10-19.md).
Cross-session history (L67) is separate and remains unimplemented at this checkpoint.

## Implemented safety boundary

The shared [file baseline](../../crates/tag-core/src/file_baseline.rs) retains
length, modification time, available native identity and a streaming SHA-256.
It reads with a fixed 64 KiB buffer, bounds input to the initial length plus an
EOF probe, retries interrupted reads, and compares open-handle and leaf state
before/after reading. Native change time is checked during a read, but excluded
from retained equality because the journal's own hard links and renames change
it. Nonregular substituted leaf entries are refused.

Cheap size/free-space admission precedes content reads. The 50-operation,
2 GiB retained-data and Unix 2 GiB free-space defaults remain unchanged.
Snapshot copies/hard links, side-file copies and staged restores are checked
against their expected content. Side-file comparison no longer allocates two
whole-file vectors. Side-file symlink targets remain literal data.

Staged Save checks the captured epoch and the current source before publishing.
The watcher records the produced staging baseline so an outside edit between
publication and journal commit cannot become the application's recorded result.
Known copied moves must preserve content before adopting the destination's
identity. Replay verifies the expected current file and snapshot, then checks
the staged restoration and current target again before replacement.

These are observed-change safeguards. The final check and replacement remain
separate filesystem operations; this is not an atomic filesystem compare-and-swap
or exclusion of every concurrent writer. Direct trusted non-staged mutations
retain their existing caller contract. No universal external-writer guarantee
or automatic recovery of an interrupted operation is claimed.

## Reproductions and controls

The [public-core controls](../../crates/tag-core/tests/history_content_baseline.rs)
use generated nonzero PCM WAV files, real `write_fields`, and real `Journal`.
They independently retain input/saved/external/replay bytes on failure.

- Concealed same-object audio edits survive refused Undo and Redo, including
  edits beyond the first hashing buffer and recorded pure renames.
- Concealed audio/side-snapshot damage is refused before publication.
- Ordinary side-file outside edits and replaced leaf symlinks are refused.
- Source replacement or leaf substitution after capture cannot bypass Save.
- An outside change after the application's Save is not attributed to that Save.
- Refused admission does not read a write-only audio input.

Two [private staged-writer controls](../../crates/tag-core/src/tags_content_guard_tests.rs)
mutate the original during the mutation closure, using respectively an in-place
edit and an identical-byte replacement with restored length/mtime. The staged
copy is changed independently, so unsafe publication is observable.

The first public RED runtime had six failures and one passing ordinary control.
The expanded capture-interval RED runtime had nine failures and two passing
controls. Both staged-writer cases failed before the repair. An initial harness
compile error and missing-executable attempts are retained but are not semantic
RED evidence. The first repair exposed `/var` versus `/private/var` leaf matching;
normalizing only the parent fixed that failure without hiding substituted leaves.
The accelerated focused rerun passed all 12 public controls, both staged controls,
and the fixed empty/`abc` SHA-256 vectors. Existing core unit tests passed 210,
with the manual benchmark intentionally ignored; existing history integration
tests passed 26.

Retained local proof roots under the host's temporary directory include:

| Layer | Root basename |
| --- | --- |
| Initial public RED build/runtime | `taggroove-history-content-red-build-yxdks798` / `taggroove-history-content-red-runtime-d_ojxqy2` |
| Staged semantic RED build/runtime | `taggroove-history-staging-red-build-2-buu1a81u` / `taggroove-history-staging-red-runtime-2-atn89_gs` |
| Capture-interval RED runtime | `taggroove-history-capture-intervals-red-runtime-ulxnjw79` |
| Accelerated focused build | `taggroove-history-accelerated-green-build-ztpu0cq3` |
| Public content / core / existing history GREEN | `taggroove-history-accelerated-content-green-i659_t9g` / `taggroove-history-accelerated-lib-green-21ismgk9` / `taggroove-history-accelerated-integration-jcpc19np` |
| Formatted-source workspace producer/runtime | `taggroove-history-content-formatted-build-6capgled` / `taggroove-native-daily-workspace-tests-b8g700kx` |
| Final strict all-target workspace Clippy | `taggroove-history-content-final-clippy-y4ns2_jt` |

Producer logs, retained executables, manifests and source bindings accompany
those roots. The initial retained RED executable SHA-256 is
`3344a53b7dd57a3257160e325bcdc1651eacc57e84b33cd93f00b1d0d43bc5bc`.
These machine-local artifacts are not a downloadable release or hosted CI result.

## Cost and platform limits

Hashing is linear in file length; bounded memory does not imply constant latency.
The same 26 existing history tests took 2.34 s on the published metadata-only
baseline, 44.21 s with software SHA-256 in the debug build, and 8.00 s with the
Apple ARM64 accelerated backend. Each is a single local debug run, with system
load uncontrolled. These are not release-build or application-latency estimates.

The pinned `sha2` 0.10.9 dependency enables its `asm` feature only for Apple ARM64,
adding `sha2-asm` 0.6.4 through the existing `cc` build tool. Its runtime dispatch
retains the software fallback. Build profiles, debug information and production
limits were not lowered. A warm-cache read/hash/stability benchmark took
55,711,792 ns for 10 MiB and 307,557,750 ns for 64 MiB. Fixture creation is outside
that measurement; inputs and logs remain in `taggroove-history-hash-benchmark-keidzs5h`.

Windows history still requires a native file ID, preferring 128-bit IDs with the
legacy fallback. Ordinary staged Save keeps the prior unavailable-identity policy:
it can proceed with full-content checks while strict history capture refuses it
with the nonfatal unavailable-Undo notice. Native Windows identity, change-time,
unsupported-ID/network filesystem, staged last-write timestamp settlement after
writable-handle closure, and fallback behavior require actual Windows
execution; no Mac control certifies them.

A separate read-only review of the frozen working code returned no findings,
covering baselines, capture/produced epochs, snapshots, publication, copied moves,
pure renames, replay and budgets. It ran no tests and is not the final GitHub
Codex review. Actual Mac application entry/error presentation remains local native acceptance.
Original Windows behavior, Linux/native Windows runtime and final exact-head
GitHub Codex review/trusted CI are separate, still-required evidence.

## Integration checkpoint

Workspace compilation, final strict all-target Clippy, formatting and workspace
doc-test commands passed (no runnable doc examples). The updated user guide passed
its strict pinned MkDocs build. All 842 UI tests, TypeScript/Vite, extraction
consistency (3,424 canonical keys), the 189/189-page writing plan and 1,426 local
links across 150 changed documents pass. Two obsolete translated error keys
were removed after the first UI run identified them. Both the pre-format and final formatted-source Rust runs passed 1,465 tests
across 83 executables (21 intentional ignores). All compiled Rust source/test
bindings in the final run match the checked source. Its artifact-manifest SHA-256
is `177a690b78d7090ea841b796fac239b51d83924b389ee090043319809ff68d7e`.

The first workspace runtime stopped at a container Undo test after free disk
space fell below the production reserve. Its prior cue-mutation panic is an
intentional caught negative control; the actual failure was “nothing to undo.”
The failed runtime and logs are retained in
`taggroove-native-daily-workspace-tests-9whgpnlb`. Recovery removed only audited,
closed compiler object intermediates and compressed closed cache files with
unchanged hashes. The unchanged failed container test and the full workspace pass with the reserve
available. A final complete rerun also passes after the only formatting repair
(reordering private module declarations). Production limits and test expectations
remain unchanged.
