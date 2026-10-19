# Cached technical date formatting — 2026-10-18

Current scope update (2026-10-19): C60 now implements the already-loaded-list observer/reformatting route and macOS native regional formatter separately from this cache repair. The historical final paragraph remains the evidence boundary of this earlier run; real OS-setting transitions are still unqualified. See the [daily-workflow receipt](daily-workflows-2026-10-19.md). The observations below retain their original checkpoint scope.

Consolidated draft PR64 fixes stale formatted technical dates on Library hits.
File stamp, reader version and read options can remain unchanged when the
formatting context changes. Previously the cache returned the old date strings,
despite retaining the original raw timestamps needed to render them again.

The [Session adapter](../../crates/library/src/lib.rs) now regenerates only known
file-created/modified and MP4-created/modified date text through the existing
reader formatter. Valid rows remain cache hits; no audio parsing, metadata write
or database reset is needed. Missing, malformed, multivalued or unrepresentable
required raw timestamps become cache misses and are replaced by fresh reads.
The lower-level storage lookup remains unchanged. The [formatter](../../crates/tag-core/src/technical.rs)
queries its existing locale provider for each field set instead of freezing the
first locale in a process-wide OnceLock; locale parsing/fallback policy stays
the same.

[Actual cache tests](../../crates/library/tests/library.rs) use a disposable WAV
and the repository's real encoded M4A with independently patched movie-header
times. They seed a previous, explicitly generated locale/UTC formatting context
while preserving file bytes, raw times, stamps and options. Cache-backed entries
must exactly match a fresh read with two hits and zero rereads/stores. Additional
controls require repair of missing/invalid/out-of-range file/MP4 timestamps, then
ordinary hits again, and preserve a valid negative-epoch timestamp.

The two initial cache cases failed against the old adapter. All three focused
cases pass after the fix; the final combined run passes all eleven Library tests,
including the additional negative-epoch control. The earlier 3,000-file check recorded 753 ms plain,
568 ms initial and 234 ms cached loading in that local run. These timings are
not an original-app or cross-platform benchmark. Independent source review
found no remaining confident finding in the three-file slice.

No system preference, UI language, native window, provider, device or original
program was changed or run. The fixture explicitly controls stored formatting
context; it is not an observed native OS-locale transition. Already displayed
rows update when loaded/refreshed. Immediate OS-change notification, native
region/time-zone behavior and original/runtime qualification remain L60/L78/L79.
