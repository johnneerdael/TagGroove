# Ordinary ID3v2.4 frame unsynchronisation — 2026-10-18

Consolidated draft PR64 fixes a reproduced accepted-input metadata corruption.
An ordinary top-level PRIV or experimental X000 frame carries a logical literal
`FF 00`, stored correctly as `FF 00 00` under frame-unsynchronisation flag `0x02`.
An unrelated ARTIST save previously succeeded with the decoded body and retained
flag, so reopening removed the payload's literal zero. Unflagged controls kept
the same payload. This was opaque metadata loss; encoded media stayed unchanged.

The existing [raw-frame scanner and passthrough](../../crates/tag-core/src/id3_frames.rs)
now include ordinary frame-unsynchronised records. Unchanged same-revision
records retain their complete stored bytes. A decoded frame that cannot be
copied, including the tested v2.4→v2.3 conversion, clears the unsynchronisation
declaration rather than describing already-decoded bytes as escaped again.
Invalid synchsafe sizes and frames beyond the declared tag range refuse before
raw-copy allocation/write. The guard does not claim strict rejection by the
existing read-only parser.

[Actual-file regressions](../../crates/tag-core/tests/id3_opaque_preservation.rs)
require successful saves on MPEG, DSF, DSDIFF, WAV and AIFF, plus one MPEG
v2.4→v2.3 conversion. Each checks untouched PRIV/X000 records, an ordinary X001
control, logical bytes, TITLE retention, ARTIST update/reopen, encoded media and
exact Undo/Redo. Two malformed size variants across all five carriers refuse
with original bytes, prior Undo label and retained budget unchanged; those
malformed cases do not execute the prior Undo themselves.

The focused run passed 48 adjacent integrations and five raw-frame unit tests,
strict tag-core Clippy, formatting and whitespace checks. Independent stdlib
byte inspection of retained backend stages confirms six required successes,
ten unchanged refusals and two pre-fix accepted corruptions. Local source review
found no remaining issue in this bounded diff; final GitHub CODE review is
separate.

This does not qualify global/nested/combined transformations, arbitrary opaque
frames or original/native/platform outcomes. Existing optional-envelope support
is documented [separately](id3-extended-preservation-2026-10-18.md).

## Confirmed remaining placeholder collision

Ordinary unflagged experimental Y000/YABC binary frames can still be mistaken
for synthetic encrypted-frame placeholders, producing an unrelated-edit refusal.
X000/Z000 controls save. A legitimate Y000 beside an actual encrypted record
also refuses in the retained controls; no accepted loss was observed there.
Both the prefix classifier and overlay ID allocation require actual generated
placeholder provenance, including through revision conversion. This remains
explicit L26 implementation work; another prefix/body heuristic is insufficient.

No original Mp3tag, Windows, native GUI, provider, network or device was used.
