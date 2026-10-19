# Private parser to reviewed physical save — 2026-10-19

PR64 extends the [generic joined replay](source-workflow-replay-2026-10-18.md)
to the actual private Traxsource and SoundCloud album parsers. The source inputs
remain the independently invented saved pages from their separate
[Traxsource](private-traxsource-offline-2026-10-19.md) and
[SoundCloud](private-soundcloud-offline-2026-10-19.md) execution checks. No recorded
provider response or live service is used.

## Shared production path

The [ignored opt-in test](../../src-tauri/src/sources/web/workflow_replay.rs)
reads an external case manifest, installs the complete user-owned bundle into a
temporary folder, applies isolated settings and executes its actual album parser.
It checks independently supplied exact parser fields, release title/artist,
artwork and zero-track shape before the review step.

The existing [DOM helper](../../tests/helpers/source-workflow-review.mjs) invokes
production `TagSourcesUI`, including source-ID binding and actual checkbox changes.
A fixture-selected one-result search adapter supplies the saved release; the
private index parser and HTTP ingress do not run. Exact independently expected
ApplyItems and artwork choice must match before calling the existing internal
`write_import_diagnostic` and session history path.

| Case | Selected fields and files | Required physical result |
| --- | --- | --- |
| Traxsource | ALBUM and YEAR on one WAV | New album/year and controlled PNG; unchecked fields unchanged; one unselected WAV byte-identical. |
| SoundCloud | TITLE on two WAVs | Scalar title applied to both selected files with controlled PNG; their other fields unchanged; one unselected WAV byte-identical. |

Both private responses contain zero explicit tracks, so these are scalar-fallback
cases. Their sentinel is unselected, not evidence of private unmatched-track
handling. The generic multi-track replay retains its separate matching coverage.

The expected parsed artwork URL is checked before supplying locally generated PNG
bytes. Embedding and sidecar replacement use the normal writer. No image download
or private album/artwork session grant is used; registry checks require those
addresses to remain unregistered before and after the private case.

## Physical and negative oracles

Each saved file's complete field map is compared with its original plus only the
selected updates. Embedded PNG bytes, sidecar replacement and history file count
are checked. Undo restores every complete original WAV and old sidecar; Redo
restores every saved byte and new sidecar.

Full-file replay alone is insufficient to prove PCM preservation during a save.
The shared harness therefore seeds distinct deterministic samples per file and
uses an independent bounded RIFF chunk walker to compare every saved PCM payload
with its own original. Explicit PCM hashes match before/save/Undo/Redo. A changed
sample byte in an in-memory whole-WAV clone is detected by re-observation without
touching disk or history. This is a small mono 16-bit fixture oracle, not playback
or every-WAV-format qualification.

Changing the actual DOM checked-field choice or substituting an unreturned
artwork URL intentionally fails before the writer emits a success receipt. The
three existing generic payload corruption controls remain. Default generic
multi/scalar tests and the invalid-protocol control remain enabled; the new
private-input test is ignored unless explicitly supplied a case manifest.

## Current combined-source execution

The root repeated both private positive cases and both new pre-write negative
controls from an immutable copy of the final combined backend, after the Nero
and PCM changes were frozen. Both positive cases pass with distinct per-file PCM
unchanged, exact field/PNG/sentinel assertions and Undo/Redo. Both deliberate
corruptions fail their intended oracle with no success receipt. The whole backend
process has OS network denial; the DOM helper records zero network requests.

Private manifests bind the original 16 runtime files, two ZIPs, invented pages,
oracles and settings; their union has 24 inputs. Those inputs stay unchanged.
Private bodies, scripts and settings remain outside Git in restricted artifact
folders. The earlier author receipt and PCM-delta receipt are immutable historical
snapshots; the root's final executable/source hashes bind the combined rerun.
Two independent reviews found no confident issue in the test change or PCM delta.

The final combined executable SHA-256 is
`916e4c4f4f4c3d2be6770fdd094c543d81e3ede3f7c905412aea33c932eed377`.
The root's full workspace passes **1,332 Rust tests, zero failures and 15
ignored**; the new private opt-in entry is one of those ignores. The frontend
passes **760 tests, zero failures/skips**. Production build, strict all-target
workspace Clippy, formatting, the 3,391-key canonical catalog and strict MkDocs
also pass. The [Nero repair](nero-sync-preservation-2026-10-19.md) is included
in this combined source snapshot. These checks do not supply hosted platform
validation or final GitHub code-review approval.

## Boundaries

This proves invented saved pages through actual private album parsing,
production DOM selection and the internal local writer/history seam. It does
not establish real provider responses, private search/index execution, HTTP or
artwork acquisition, public apply ingress/session authorization, native GUI,
original Mp3tag behavior, Windows or physical devices. It does not establish
SoundCloud duration support. L30/N01/N02/L78/L79 remain active for their separate
contracts. Final GitHub Codex review/current-main validation and publication are
not supplied by these local tests.
