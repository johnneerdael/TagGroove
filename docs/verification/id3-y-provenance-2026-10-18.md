# Experimental ID3 IDs and encrypted-placeholder provenance — 2026-10-18

Consolidated draft PR64 fixes a reproduced valid-input refusal. Ordinary
unflagged binary Y000/YABC frames read successfully but an unrelated ARTIST save
mistook them for generated encrypted-frame placeholders. Actual encrypted
fallback also allocated Y000 without excluding real occupied IDs. The observed
Y-specific failures preserved the original file; they were not accepted loss.

The [raw-frame pipeline](../../crates/tag-core/src/id3_frames.rs) now inventories
all occupied top-level IDs before allocating unused internal IDs. It carries the
actual generated-ID→stored-ID table through native/DSD/chapter/utility reads,
saves, technical names and revision conversion. Membership in that read's table
replaces prefix classification; an ordinary read has an empty table. No public
JSON model, schema, dependency or global registry was introduced.

An encrypted overlay is admitted only when physical/tag-chunk/header/frame bounds
and remaining padding establish a complete inventory. Incomplete framing or
namespace exhaustion returns no overlay and retains the original parser error.
Genuine encrypted revision conversion keeps its explicit protected refusal.

The [actual-file matrix](../../crates/tag-core/tests/id3_opaque_preservation.rs)
requires 30 successful same-revision cases: both ID3 revisions across MPEG, DSF,
DSDIFF, WAV and AIFF, with ordinary Y000/Y001/YABC and with two real encrypted
TXXX/TALB records in both orderings. Two ordinary-Y MPEG revision conversions and
one actual CHAP-row edit also require success. Controls assert real IDs, distinct
correct technical names, original ciphertext, payload/media bytes, reopen and
exact Undo/Redo. Thirty malformed cases and two genuine encrypted conversions
refuse unchanged; their prior Undo label and retained budget are checked, without
executing that prior Undo in the refusal cases.

The focused run passed 12 preservation plus 59 adjacent integrations and six
raw-frame unit tests. The namespace unit occupies all 46,656 candidate IDs and
requires no overlay or source-buffer mutation. Independent stdlib inspection of
retained backend stages confirms 33 required successes and 32 protected refusals.
Strict package Clippy, formatting and whitespace checks passed. Independent
source review found no remaining confident finding in the seven-file slice.

This supersedes the Y-ID follow-up in the earlier [frame-unsynchronisation
receipt](id3-frame-unsync-2026-10-18.md#confirmed-remaining-placeholder-collision).
L26/L38 still retain arbitrary/nested/global/combined transformations and
original/native/platform acceptance. No original Mp3tag, Windows, native GUI,
provider, device or network was used. Local review is separate from final
GitHub CODE review.
