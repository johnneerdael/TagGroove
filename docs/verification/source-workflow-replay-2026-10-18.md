# Joined source workflow replay — 2026-10-18

Consolidated draft PR64 adds an independent filesystem replay joining previously
separate parser, review and save/history controls. No production API changed;
the only existing-source change registers a test module.

[Rust replay](../../src-tauri/src/sources/web/workflow_replay.rs) installs an
independently authored `.src`/`.inc`/`.settings` bundle, saves a nondefault setting,
and executes the actual loaded index/album parsers against saved local pages.
The resulting search/release objects and physically read WAV summaries cross a
versioned JSON bridge to the [production review helper](../../tests/helpers/source-workflow-review.mjs).
It executes the current `TagSourcesUI` in a controlled DOM, performs sparse field
and artwork selection and returns the actual ApplyItems/ApplyCover payload.

| Scenario | Required outcome |
| --- | --- |
| Search and multi-track album | Reversed selected-file order is matched to the proper two tracks; the unmatched third WAV is byte-identical. |
| Direct release with zero track rows | Actual album-wide scalar fallback updates the two selected WAVs without inventing track rows. |

Both scenarios select TITLE/ALBUM/COMMENT while retaining original ARTIST/TRACK.
The saved source setting reaches COMMENT. Parsed artwork provenance is checked
before the existing image validator and journaled writer embed the independently
created PNG and replace an existing sidecar. Physical reload checks fields and
actual image bytes. One two-file history step records the import; actual Undo
restores every WAV/sidecar byte, and Redo restores the saved bytes. Independent
before/saved/Undo/Redo SHA-256 values are emitted by the test.

The two new tests pass, including malformed protocol refusal before an output
manifest. Deliberate test-payload corruption for matching, an unchecked field
and an unreturned artwork URL each fails its intended pre-write oracle. These
mutation controls demonstrate oracle sensitivity, not a production defect.
Existing generic writer/Undo and scalar-review controls were also rerun. Strict
package Clippy, formatting and whitespace checks passed; source review found no
remaining issue in the bounded evidence.

The local execution denied network access at the OS boundary. The helper also
rejects Node/DOM network operations and reports zero requests. Portable test
bodies start no server and require the existing Node/TypeScript/Happy DOM setup;
reviewed-build runs `npm ci` before workspace Rust tests. No dependencies were
added. All 16 original private runtime files remain hash-identical to the named
earlier inventory; private script or provider response contents are not copied.

This closes only the independent parser→production review→filesystem/history
join. HTTP acquisition, public apply ingress, private SoundCloud/Traxsource
search/direct response replay, native interaction, released builds and provider
journeys remain N02/L30/L78/L79. The earlier private receipt contained no saved
provider response bodies. No original program, native GUI, Windows, picker,
clipboard, provider service or device was used.


## Private-source follow-up — 2026-10-19

The [joined private replay](private-source-joined-replay-2026-10-19.md) reuses this production review/writer/history path with the actual private Traxsource and SoundCloud album parsers and invented saved input. It adds independent per-file PCM checks. Private index/HTTP/public ingress and provider/native qualification remain separate.
