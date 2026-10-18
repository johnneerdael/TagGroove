# MP4 raw data preservation — 2026-10-19

Consolidated PR64 fixes an actual unrelated-edit loss: saving TITLE reset the
stored locale word of an untouched opaque `xQA!` item and legacy `plID` from
1033 to zero. The native dependency's model omits locale, leaving that loss
invisible to native-object equality and the earlier corpus snapshots. Further
independent fixtures exposed normalization of untouched UTF-16 data.

## Preservation boundary

The [save boundary](../../crates/tag-core/src/tags.rs) now captures native items
before mapping changes. The [MP4 writer](../../crates/tag-core/src/mp4_metadata.rs)
compares complete before/after groups by exact FourCC or freeform `(mean, name)`
identity. Unchanged groups reuse structurally parsed raw data; explicitly touched,
changed, remapped or removed groups use ordinary serialization. Artwork changes
also invalidate raw reuse for the cover identity. Deleted values do not reappear.

Grouping follows the measured existing option behavior: ordinary repeated items
can remain separate, multi-data items follow the single-atom setting, and the
legacy album-ID adapter retains its established aggregation. Regrouping preserves
each ordered data envelope rather than promising an obsolete item wrapper or
arbitrary whole-file ordering. Accepted terminal size-zero item headers receive
resolved lengths before relocation. `ilst` free/skip payloads remain opaque.

Both the padded/growing path and full fallback consume the same serialized plan.
Fallback uses the existing dependency's layout writer with an exact-size
surrogate, verifies its complete structural location/bytes/length, then substitutes
the equal-length final plan on disposable output before publication. No blind
marker search or unaccounted post-layout size change is used.

The pinned dependency also mishandles representable extended container headers.
A direct dependency-only control demonstrates that failure. Fallback adapts those
headers without moving descendant data before layout writing. A temporary first
child needed for non-full QuickTime metadata is structurally verified and replaced
with the original wide-header topology before publication. No synthetic child
reaches the result. ISO parent headers may normalize with legal padding, so this
is not whole-file byte preservation.

## Executed evidence

The first permanent locale test failed against the old writer while the grouping
and independent-observer controls passed. Expanded tests caught valid padding,
accepted size-zero and extended-header cases; those controls pass after repair.
Seventeen dedicated integration tests and ten strengthened corpus tests pass.
The 232 related library/format/chapter tests also pass. The four MP4 module
controls are included in that related count and must not be added again.

The [independent observer](../../crates/tag-core/tests/support/mp4_atoms.rs) parses
raw atom boundaries/types/locales/data without the production parser or native
metadata model. It detects locale loss even when visible fields and native-model
equality stay unchanged. The [common corpus](../../crates/tag-core/tests/support/save_corpus.rs)
now seeds a nonzero locale and records raw opaque data independently.

[Actual-file tests](../../crates/tag-core/tests/mp4_locale_preservation.rs) cover
ordered duplicates with different locales, UTF-8/UTF-16, freeform identities,
legacy integers, both grouping settings, explicit edits/removal/user mapping,
localized cover removal/re-add and exact journal Undo/Redo. They also cover absent
tags, growth, extended ISO/QuickTime parents, last-field removal, tiny/empty plans,
and owned structural refusals versus external I/O errors.

A retained 2,266-byte [fragmented carrier](../../crates/tag-core/tests/fixtures/save-corpus/tone-fragmented.m4a)
was codec-copy remuxed from the existing independently authored tone. Its
[provenance](../../crates/tag-core/tests/fixtures/save-corpus/tone-fragmented.provenance.json)
records the tool/command/hashes. Concatenated encoded audio stays byte-identical;
the regression checks the relative fragment sample address as well as metadata.
No runtime media CLI dependency is added.

Two independent frozen-source reviews found no remaining confident issue in the
named patch. Strict tag-core all-target Clippy, formatting and whitespace checks
pass. The owned refusal message has English/Dutch/German catalog entries.

The root's final combined workspace run passes **1,326 Rust tests, zero failures
and 14 ignored**. The frontend passes **760 tests, zero failures/skips**.
Production TypeScript/Vite build, strict all-target workspace Clippy, formatting,
the 3,390-key canonical catalog and strict MkDocs pass. These are local checks,
not final-head GitHub review, hosted platform validation or publication evidence.

## Limits

These are local backend/file and independently authored fixture results. They
do not establish original Mp3tag defaults/output, native GUI, Windows, playback,
files over 4 GiB or every fragmented/offset variant. The supplied fragment uses
default-base-is-moof with a relative sample offset. Inputs already rejected by
the reader, including the named zero-sized data/non-data-child fixtures, refuse
unchanged; the repair does not expand their acceptance. Ordinary staged
publication/history is reused; no dependency edit or second recovery engine is
introduced. L24/L28/L39/L77b/L79 remain separate requirements. Final GitHub Codex
code review and current-main validation are still required before merge.
