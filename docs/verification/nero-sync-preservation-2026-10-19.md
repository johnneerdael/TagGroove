# Chapterless Nero metadata synchronization — 2026-10-19

PR64 fixes an independently reproduced stale-copy save. In a valid AAC/M4A with
an existing Nero TITLE `X`, changing TITLE to `Changed café 日本` succeeded but
updated only iTunes. The physical Nero title stayed `X`; independently replacing
the iTunes title atom with equal-sized padding exposed it again. A value fitting
the existing Nero space updated both copies. Permanent controls also found short
shrink gaps of one to seven bytes that left the old value unchanged.

## One existing write path

The [Nero synchronization branch](../../crates/tag-core/src/mp4_chapters.rs)
keeps its in-place attempt first. A chapterless change needing a rewrite now uses
a metadata-only operation in the [existing layout engine](../../crates/tag-core/src/mp4_layout.rs).
It requires unique existing `moov/udta/tags` ownership and exact captured old-body
equality, changes that leaf only, and uses an immutable staging copy with the
existing writer's reordering disabled and no added chapter/media data. The
existing relocation/media checks and physical Nero reload verification remain.
No chapter is fabricated and no second save engine or public API is introduced.

For an existing mirrored field, both physical representations receive the
requested value on supported layouts. A required rewrite on an unsupported/ambiguous/changed layout refuses
the entire staged save, preserving the original. It no longer reports success
while publishing an iTunes value over a stale Nero copy. Existing fitting,
unrelated-field, absent-tag and removal cases retain their earlier paths.

## Executed evidence

The first two permanent regressions failed on the old growth and short-shrink
behavior. After repair, 58 focused tests pass: seven layout module tests,
19 format-gap tests, 17 MP4 raw-data tests, five chapter tests and ten corpus
tests. These are distinct suites; the layout tests include the two new helper
ownership/I/O controls. Strict tag-core all-target Clippy, formatting and
whitespace checks pass. Two independent frozen-source reviews are clean.

[Actual-file controls](../../crates/tag-core/tests/formats_gaps.rs) inspect physical
Nero values independently, not just the iTunes-preferred field view. They cover
front/tail movie placement, `stco`/`co64` sample targets, growth, short shrink,
fitting changes, mappings, absence/removal and zero-count chapter metadata.
They compare encoded media and unchanged `indx`, opaque siblings, Nero artwork,
unknown flags/payloads and raw iTunes data including a stored locale word. A
same-extent iTunes-title removal exposes the new Nero value. Journal Undo and
Redo compare the complete file bytes exactly.

Guarded rewrite fixtures cover fragments, top-level metadata, item locations,
bad offsets and duplicate targets. They require unchanged originals on refusal;
separate controls retain fitting/no-op/removal behavior where no relocation is
needed. Helper-level controls distinguish the owned location/drift guard from
real external I/O errors. The new guard has English/Dutch/German entries.

## Limits

This establishes named local backend behavior on independently authored metadata
and the retained real synthetic AAC carrier. It does not prove original
Nero/Mp3tag output, `indx` consumer semantics, arbitrary address-bearing metadata,
every layout, playback, native UI, Windows or provider behavior. Layouts the
shared rewrite engine cannot preserve remain unsupported for changes requiring
relocation; this is not completion of L24. Existing covers remain preserved in
the tested cases, without adding a new Nero artwork placement/editor feature.
L24/L28/L39/L77b/L78/L79 remain open for their separate contracts.

The ordinary staged-publication and history engines are reused. Final GitHub
Codex code review and current-main platform validation remain required before
merge; these local checks do not establish release or publication.
