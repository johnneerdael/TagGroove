# Reviewed Filename → Filename companion extension

Unmerged L22a implementation. The default remains off. Tag → Filename, Move
and Copy retain their separate execution contracts. Action-driven renames,
Tag → Tag and inline filename editing are not enrolled by this slice; L22a
remains active. Original/native/cross-platform qualification remains separate.

## Frozen review and execution

The existing converter token still binds the full serialized request, ordered
selection, source snapshots/full hashes, exact rendered targets, raw-target
normalization, destination ancestors, expiry and single-use admission. It now
also freezes the saved companion policy and the common association Plan for
matched audio only. Every matched preview item exposes the exact companion
pairs. Unmatched audio and its companions remain untouched. The shared rules
still admit only exact basenames with .lrc/.cdg case-insensitive extensions,
preserve extension spelling and refuse nonregular entries, ambiguous/shared
members and portable destination clashes. No low-level rename primitive gained
preference awareness. A failed history-admission request consumes its token;
restoring an earlier policy cannot resurrect it.

The command extends initial history capture with original frozen companion
sources before any movement. They are returned separately as
`associated_renamed` and included in history's original→final mappings. They
never become audio rows or get parsed as audio during converter/readback/replay.
The production frontend validates companion shapes and duplicate members,
snapshots them in the existing review receipt, and renders escaped/verbatim
source/destination paths in the existing dialog.

Unlike Tag → Filename and desktop Move's per-group runtime behavior, this
converter retains **whole-batch rollback**. Companions move before their audio;
a later member, audio group or readback failure reverses all completed members
in reverse order. Each ledger entry carries the original content hash,
produced file metadata/native identity and both source/destination parent identities.
Source snapshots also bind the core native file ID (including Windows 128-bit
or legacy handle IDs), so timestamp equality cannot substitute for an identity. Rollback
requires the same regular file, unchanged bytes/metadata and original parent
identities before using the existing no-clobber diagnostic rename. Reoccupied
sources, externally edited/replaced outputs and replaced parent folders are
retained with structured `ConverterError::Rollback` paths and the existing
typed rename provenance. Uncapturable produced identity is retained rather
than guessed.

Full original destination-ancestor metadata, normalized targets and conflicts
are admitted before history capture. Capture probes clone capability with
temporary files in source folders; that legitimately changes directory mtime.
The opaque token records that completed admission. After capture, Apply checks
ancestor native identity, normalized targets/conflicts and every source hash
again; it does not adopt a refreshed directory identity or compare the old
mtime against its own probe. Non-history application retains the full original
ancestor metadata check.

Destination ancestor identities and raw/normalized targets are checked again
immediately before member movement. Newly required parents are created one at
a time; only a successful create is recorded as owned. An AlreadyExists result
is validated but never adopted as owned. After a handled failure, cleanup visits
created parents deepest-first and calls remove_dir only with the same identity;
nonempty, replaced and unidentifiable folders remain with diagnostic paths.
It never recursively deletes user contents. This is observed identity checking,
not a hostile-filesystem compare-and-swap guarantee.

The existing successful Undo/Redo contract remains: file replay can refuse an
externally changed member while reversing eligible others, and empty destination
folders can remain. No new Undo directory-ownership guarantee is claimed.
Storage failure can still make an operation unavailable to Undo with its normal
notice. No Copy Undo or action-preview simulator was introduced.

## Focused verification

RED reproduced missing preview members, unchanged companion paths, failure to
reject stale membership/content/policy and failure to remove created empty
parents after handled rollback. Separate RED reproduced rollback adopting a
replacement source/destination parent and reuse after failed history admission.
Frontend RED reproduced absent member validation and absent path rendering.

GREEN cases use real valid WAVs and byte-compared .lrc/.CDG files. They cover
policy off/on; matched/unmatched selections; excluded suffix/text files;
stale/new/deleted companions; single-use/policy refusal; full-batch rollback;
external sidecar edits; replaced parent identities; unrelated contents in
created folders; pre-existing folders; and a later companion collision.
The actual journal test captures original companions, applies one history step,
replays Undo/Redo without companion audio parsing, and preserves a later external
lyric edit while eligible audio/karaoke replay follows the existing contract.
The isolated journal lives outside the reviewed music ancestor, matching the
ordinary application-cache boundary. The history integration also reproduced
the production clone probe changing source-parent mtime; the two-stage admission
above handles that intentional change without replacing source/parent identities.

Headless frontend tests capture actual production review state and preview
markup. They verify receipt snapshots, malformed/duplicate member rejection,
escaping, untranslated authored paths even with matching catalog entries, and
structured rollback presentation. Native application/UI replay and original
Windows/provider execution were not run. Final focused results: 32 converter cases (including eight companion cases),
one actual journal Undo/Redo case, four core copy/identity cases, 60 affected
frontend/options/catalog cases and two production DOM cases pass. TypeScript,
format/extraction, strict guide and ledger checks pass. Two-package library/test Clippy with warnings denied and final read-only
source/docs/locale/ledger review pass. Cargo reported an unrelated global
registry-cache cleanup permission warning; compilation and lint completed
successfully. No suite count is added
to another overlapping cohort as an independent coverage total. Genuine exact-final-head GitHub
Codex code review, required Linux CI, merge and release/native acceptance remain
unperformed for this batch. L22b/L78/L79 retain their qualification boundaries.
