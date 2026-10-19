# Private source search through public apply — 2026-10-19

This extends the earlier [private album-parser replay](private-source-joined-replay-2026-10-19.md)
with the normal logical search, release, image and public apply handlers. It uses
the actual user-owned Traxsource and SoundCloud bundles, isolated settings and
independently invented external pages and expected results. Private inputs stay
outside Git. No recorded provider response or live request is involved.

## Executed path

The opt-in test in [workflow_replay.rs](../../src-tauri/src/sources/web/workflow_replay.rs)
installs each complete bundle in a disposable folder. The production search and
release command bodies share deferred preparation workers with the harness:
search claims its ticket before preparation, and release validates its registered
address before polling preparation. Command signatures and the installer lock's
scope remain unchanged.

A `cfg(test)` response scope supplies exact GET bodies at the existing HTTP send
points, after production client, URL, header, admission and request-gate setup.
The bodies become real `reqwest::Response` values and pass through existing
status, size, decoding and image checks. An active scope refuses an unexpected
method or address; it never falls back to networking. Production builds have no
fixture hook. Each private case runs in a fresh test process because request
tickets and the URL registry are process-global.

The Traxsource direct search registers its release address. The SoundCloud case
executes the actual index parser on an independently invented search wrapper,
then its album parser. Both register release/artwork addresses through the normal
flow; neither manually grants them. The versioned, sequenced Node bridge drives
production `TagSourcesUI` and dispatches its search, release, image and apply
calls to those handlers. The public apply handler then uses the normal local
writer and history seam.

Independent expectations check source IDs, query/search rows, release fields,
ApplyItems, checked fields and artwork before writing. The original scalar cases
retain one selected Traxsource WAV and two selected SoundCloud WAVs, controlled
PNG embedding/sidecar replacement, unchanged unchecked fields and an unselected
sentinel. Every file's PCM is compared independently across save, Undo and Redo;
history restores the complete original and saved WAV/sidecar bytes exactly.

## Refusal and process controls

Both positive cases pass. Five deliberate field, artwork or checked-field
changes fail the exact pre-write oracle, with no writer invocation, success
receipt or WAV/sidecar/history change. Four earlier internal-replay corruption
controls retain their intended failures. Status, authorization, body size, image,
stale-ticket and unexpected-GET/POST controls require their named errors.

The subprocess bridge continuously drains stderr while retaining at most 64 KiB.
For each source, a 256 KiB stderr control completes the full flow. A stalled
child hits the parent deadline; closing its stdout while it remains alive hits
the distinct early-close guard. Both controls kill and reap the child and leave
transport, files and history unchanged before any command. The independent
watchdog and parent cleanup also cover protocol errors and dropped bridges.
This bounds the child lifecycle; it does not interrupt arbitrary Rust filesystem
work.

## Frozen-source verification

The root independently repeated both positives, all five pre-write refusals and
all six bridge controls from a separate restricted artifact folder and immutable
backend copy. The whole process had OS network denial. All 27 inputs bound by
the two acquisition manifests remained unchanged, including the original 24
private replay inputs. Author receipts and earlier binaries/logs were retained.

The repeated executable SHA-256 is
`49c7f50562db29469a2d6829f2364bbe3fd9b8e65dd076f3c482ab8ba5e3331d`.
The combined Rust workspace passes **1,335 tests, zero failures and 16 ignored**;
strict workspace/all-target Clippy and formatting pass. One additional ignored
entry is this explicit private-input acquisition test. Independent scoped review
found no remaining confident issue after the stderr/deadline repair.

The catalog extractor now recognizes an explicitly test-only Rust file without
adding test protocol/oracle messages to application locales. Its controls also
cover quoted/nested-module attributes and nested Rust block comments, which must
not hide production diagnostics. The backend key snapshot remains byte-identical
at 1,178 keys and the English catalog remains at 3,392 keys.

## Remaining acceptance

This is logical handler and in-memory response qualification. It does not cover
sockets, DNS, TLS, redirects, proxies, wire timeouts/chunking or real provider
responses. Native Tauri dispatch, AppHandle configuration-directory lookup and
the main-window confirmation host are outside the bridge. The URL registry is
session-wide; this is not a new source-bound grant or server-side field allowlist.
The private pages have zero explicit tracks, so private multitrack matching and
SoundCloud duration remain unqualified. Native UI, original Mp3tag, Windows,
devices, hosted final-head CI and GitHub Codex approval remain separate work.
