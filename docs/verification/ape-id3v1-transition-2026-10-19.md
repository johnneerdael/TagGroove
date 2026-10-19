# APE growth before ID3v1 Save: CI repair, 2026-10-19

The [reviewed Linux validation job](https://github.com/johnneerdael/TagGroove/actions/runs/39291679239/job/118196739222)
on head `355484a` failed `ffmpeg_tta_decodes_identically_after_tagging` with
Ubuntu FFmpeg 6.1.1: Save refused readback for ALBUM. The final staged verification
preserved the original rather than publishing the inconsistent tag. The earlier
local Mac run used FFmpeg 9.0.2 and passed its differently sized carrier.

## Reproduction and repair

An independently encoded APEv2 header/items/footer reproduces the same ALBUM
refusal without depending on an old encoder executable. Original Title plus the
13-byte encoder value occupies 119 bytes. Setting Title to Tagged and Album to
Disc grows the APE tag to 131 bytes, placing TAG from its header at EOF-128. The
subsequent ID3v1 writer can then mistake APE data for an existing ID3v1 block.
The guard prepared on the input cannot predict that new intermediate layout.

The otherwise identical control with ID3v1 output disabled passes before repair,
isolating that later writer. The repair reuses the existing APE guard immediately
before ID3v1 output, combines phantom ownership for blank-V1 positioning and
finishes the inner guard before publication. The outer guard retains original
headerless-APE restoration and Lyrics3 ownership. Readback/content checks,
original-byte refusal, tag policy choices and history limits remain in force.

The final raw-stream cohort passes 11/11: populated, blank and omitted V1 output,
requested field values, physical APE/footer/V1 separation and unchanged stream
bytes, plus the existing FFmpeg/ffprobe/mutagen controls. A fresh scoped read-only
review found no concrete issue; it did not run tools and is not GitHub approval.

## Retained evidence

The following roots are under the authoring Mac temporary directory. Commands,
source bindings, executable hashes, full stdout/stderr and failing input/oracles
are retained, including the initial and corrected test harnesses.

| Evidence | Root / identity |
| --- | --- |
| Independent semantic RED | taggroove-tta-new-collision-retained-red-build-db1rshsy; executable 487b66eeef0a3de8415ed8be36c03793c815d79c5cd1069a0651efb2f5523237; runtime taggroove-tta-new-collision-red-runtime-8c8fau8k; failing fixture .tmps3o1qI |
| Pre-fix no-V1 control | taggroove-tta-collision-no-v1-control-72wyq4uq: PASS |
| Fixed-code intermediate harness | taggroove-tta-collision-green-runtime-a47ku1ay: 9 pass/2 fail because two footer assertions required equality of writer-owned flag bits. Requested fields/audio checks passed; footer revision/extent/itemcount/header/V1-separation assertions replace that unsupported exact-flags promise. |
| Retained intermediate produced file | taggroove-tta-footer-diagnostic-runtime-453x8oxd; fixture taggroove-ape-transition-1NfaQe; executable 6d1caed514ebdcab10d10e2b7d53d7776a478a5b7a94da48a3167e33dc4f5d2f |
| Final focused GREEN | taggroove-tta-collision-final-focused-build-49rqjilg; executable baccb9ca665f1c57094ba62a035b9d865d9f6fc2f32e56e987adbd859f4458c7; runtime taggroove-tta-collision-final-focused-runtime-iiamylcz: 11 pass/0 fail |

An independent byte inspection of the retained intermediate output found Title
Tagged and Album Disc in APE, Album Disc in the actual V1 tail, the original
encoder value and unchanged audio bytes. The output SHA-256 is
`5a36c0109f7d47caf589bcf929fe1fbbdeffbf4a4390382b04ebb770ccdeef8d`;
`independent-postwrite-inspection.json` is retained beside that fixture.

New controls retain their complete fixture and independent original bytes on any
panic, including post-write assertion failures. They do not claim the synthetic
TTA payload is a separately decodable reference recording; the existing actual
FFmpeg carrier has its own decoded-audio checks.

## Bound source and remaining validation

- `crates/tag-core/src/tag_policy.rs`: `7d9deeb0df0d0ec050cc2a64f0ab495553d008426905df4ef13cee8b3555c987`
- `crates/tag-core/tests/raw_streams.rs`: `7550983248b2b5214428016654438233bdf0449d858337fb8fc5b50e46e637f5`
- `docs/user-guide/reference/file-formats.md`: `e625c97df6700834907a23a1bf6ace437517e4ff7a410e42d4a7eaf0366b4125`

The complete current-source Rust workspace passed 1,513 tests across 84
executables, with 21 intentional ignores and no source-binding drift. Strict
workspace/all-target Clippy, formatting and the strict guide build passed. See
the [combined CI repair receipt](pr64-ci-repairs-2026-10-19.md) for retained roots.
A new exact-head GitHub Codex CODE review and trusted immutable CI remain required. The earlier
355484a clean review cannot approve this source change. No native Windows/Linux
success or old-run overall success is claimed by the local reproduction.
