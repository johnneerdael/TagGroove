# External real-encoder lossless carriers — 2026-10-19

The [opt-in integration tests](../../crates/tag-core/tests/real_lossless_external.rs)
add real-carrier evidence beyond the constructed TAK/OptimFROG headers. Both
recordings stay outside Git because redistribution permission is not established.
Default test discovery skips both explicit opt-ins; supplying a missing or
invalid manifest fails rather than reporting success.

| Carrier | Primary archive and input SHA-256 | Qualified result |
| --- | --- | --- |
| TAK | [FFmpeg sample](https://samples.ffmpeg.org/A-codecs/tak/lostchord-tak_2_2_0.tak), `6696820264e05fd1dda2355a4ba92d2fc573716881d633553eaee448dd4b2eec` | Complete independent decoding and packet/PCM preservation on this carrier. |
| OptimFROG | [FFmpeg sample](https://samples.ffmpeg.org/A-codecs/lossless/luckynight.ofr), `f8a8740311c7aebe509a7a8bcd8dc528733f7a18fb3febc491ed0044b564b90c` | Header/readback, exact encoded bytes, physical tags and history; codec completeness/PCM remain unknown. |

The external versioned manifest binds each carrier, retrieval provenance,
expected header properties and the installed ffmpeg/ffprobe executable hashes.
The working file is a disposable copy. A separately implemented bounded APEv2
observer checks footer/header agreement, item extents, UTF-8 and multiple values,
and requires the entire original encoded stream immediately before the new tag.
It does not reuse the production payload helper or search audio for signatures.

Both cases require actual writer/readback success for Unicode TITLE, two ARTIST
values and a custom field. They require default journal admission, the real Undo
label/file count, complete original/saved bytes after Undo/Redo and unchanged
file/history on the existing NUL-value refusal. In-memory encoded-byte and tag
mutations must trip distinct independent guards without touching disk/history.
Production Undo limits, including the 2 GiB free-space reserve, are unchanged.

The complete TAK baseline decodes with ffmpeg `-xerror` and empty diagnostics:
5,589,504 mono 44.1 kHz/16-bit samples, 1,015 packets and 11,179,008 s16le bytes.
Before and after the save, every packet hash/timing tuple and the complete PCM
SHA-256 match:
`17c211550f3d49d521a370122a1dd4bdd77d0af85d1dbb1844b266130c33cf80`.
The local tool build has no OptimFROG decoder; its preserved encoded bytes are
not evidence of decoded PCM, playback or independently complete codec validity.

Both exact opt-ins pass, as do target-only strict Clippy and formatting. The
root independently repeats both from an immutable integration binary under OS
network denial. The executable SHA-256 is
`cecd6f910c1ce1e28a2440c9cb8e5df2d281586dd319ebf3740ac426342f4baf`.
Download inputs, baseline decodes, author receipts and raw logs stay immutable.

This qualifies these named carriers and installed tool builds. It does not
establish every TAK/OptimFROG revision, existing-tag layout, native/original
editor behavior, Windows/device/playback acceptance or released artifacts.
L23 retains the OptimFROG decoding oracle and broader real-encoder comparison.
