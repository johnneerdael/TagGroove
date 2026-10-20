# Action and Tag → Tag companion renames

Unmerged L22a working-branch implementation, following the separately recorded
[Copy](associated-copy-2026-10-20.md) and [Filename → Filename](associated-filename-converter-2026-10-20.md)
extensions. Together these close the approved local implementation scope.
They do not establish original Mp3tag, native or cross-platform equivalence.
The batch still requires its covering PR, exact-final-head GitHub Codex code
review, required Linux CI and merge.

## Execution and history contract

The saved companion option remains off by default. Public action-group, Quick
action and Tag → Tag commands prepare exact-basename .lrc/.cdg membership
before history capture, then pass the same preparation into execution. Only
enabled actions capable of producing a filename effect enumerate companions;
metadata-only and directory-only groups do not scan unrelated sidecars. The
existing action classifier covers Format Value, case/codepage conversion,
replacement/regular-expression replacement and Guess Values filename targets.
Ordinary per-input audio read errors remain per-input errors.

Actions are immediate and order-dependent. Unlike the reviewed converters and
transfers, they have no human-reviewed companion destination receipt. Each
effect uses its actual computed path once; no action simulator or second
evaluation of stateful/random expressions was introduced. Membership remains
frozen across own metadata writes and repeated filename changes. Whole-directory
effects carry directory contents naturally; the tracker rebases companions
instead of moving them a second time. Physical paths are used for membership
rebasing, including mixed-case directory names; Windows-folded comparison keys
remain separate from physical path spelling.

Companion original→final changes are returned separately as `associated_renamed`
and included in the existing history step. They never become audio selection
rows. The initial capture includes frozen originals before any move, and cover
output capture retains the existing directory-epoch behavior. Undo/Redo may
refuse an externally changed member while reversing eligible others. Empty
destination folders can remain; no new Undo directory-cleanup guarantee exists.
Copy still has no transfer Undo.

## Failure ownership

Each filename effect moves companions before audio and includes audio readback
in its transaction. Member/readback failure attempts to reverse that effect's
completed moves, with original hashes, produced native file identities and both
parent identities checked before a no-clobber reversal. Occupied originals,
outside edits/replacements and replaced parents are retained with typed paths
and nested rename diagnostics. Successful own reversal refreshes the recorded
native identity, including when the low-level move used copy/delete fallback;
an external replacement never supplies a trusted refreshed identity.

Only successfully created empty parent folders with unchanged identities are
removed after a handled failure. Pre-existing/nonempty/replaced folders stay.
This is observed ownership checking, not a hostile-filesystem atomic guarantee.
Earlier successful effects and metadata changes retain the existing action
warning/continuation semantics; this is not a whole-group transaction.

A native move can fail after an unreported partial operation. Unknown staging
paths are not guessed as successful mappings. Unproved companion state blocks
later filename/directory participation. Unproved audio location/identity blocks
all later host I/O for that input before synchronization or writing. The same
guard protects directory moves when companions are disabled or the group only
renames directories: it freezes the affected selected audio identities/bytes,
checks the original directory after failure and blocks unsafe later writes.
Known retained moves remain reported separately from unknown locations.

## Scope disposition

[C06/PR54](../research/remaining-gaps.md#c06) retains its original Tag → Filename
and desktop Move delivery scope. The three new receipts extend the working
branch; they do not rewrite PR54's history. Inline filename editing remains the
pre-existing explicit audio-only policy recorded in
[inline filename paths](inline-filename-paths-2026-10-20.md) and the
[file-list guide](../user-guide/guides/file-list.md#edit-a-cell).
Original inline companion behavior remains unobserved, rather than an open
local implementation decision. Provider/mobile action execution is separate L18
work. L22b/L78/L79 retain original/native picker and input behavior,
Windows/cross-volume/network and broader fault qualification.

## Focused evidence

Real WAV, lyric/karaoke and PNG fixtures exercise policy off/on, repeated
filename/directory effects, multiple selected audio files sharing a directory,
excluded suffixes, collisions followed by metadata actions, Guess Values/regex,
new/deleted/edited/replaced frozen members, mixed-case physical paths and
metadata-only avoidance of unrelated sidecars. Transaction regressions cover
typed partial failure, external edits/equal-byte replacements/occupied originals,
readback rollback, own copy/delete fallback identities and unknown partial
native operations. A RED partial-directory regression reproduced overwriting
an outside replacement; its GREEN form proves both selected inputs are blocked
with the option off, directory-only on and filename-capable on.

Current focused results: 73 action-host tests, five transaction tests, ten
shared association tests, 32 converter regression tests, one combined actual
journal test and one existing cover-directory-epoch regression pass. Cohorts
are not added as independent coverage totals. The actual journal test performs
filename → cover export → directory → filename → cover export in one history
step and byte-checks audio/lyric/cover outputs through Undo/Redo. The 45 affected
Options/catalog tests, TypeScript and strict guide build pass. Format,
extraction, five-view ledger checks, diff checks and warnings-denied library/test
Clippy pass. Final local read-only source and documentation/locale/ledger reviews
found no actionable findings after the partial-operation and physical-path fixes.
These reviews are separate from the GitHub code-review gate.

No application/native UI replay, Windows-original execution or live-provider
comparison was run. Native Windows identity support and physical-path handling
are source-level coverage here, not Windows runtime proof. Cargo reported an
unrelated global registry-cache cleanup permission warning.
