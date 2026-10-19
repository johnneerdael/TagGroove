# Source workflow over real loopback HTTP — 2026-10-19

Commit `6f49dc4090046a7eda5c35bbf97a068f0c217576` adds a normal regression
parent that runs nine source-workflow cases in fresh worker processes. An
authored generic source joins real TCP HTTP acquisition to the existing public
search/release/image workers, production `TagSourcesUI` DOM review, public
apply, physical WAV/PNG reload and journaled Undo/Redo. This extends the earlier
saved-page and fixture-transport evidence with actual sockets and response
handling; it does not turn the supplied private scripts into wire-qualified
provider integrations.

The implementation is confined to the test-only
[workflow replay](../../src-tauri/src/sources/web/workflow_replay.rs) and
[loopback cohort](../../src-tauri/src/sources/web/workflow_loopback.rs). The
parent runs during ordinary backend tests; its ignored worker is invoked
explicitly by that parent to isolate global tickets, URL grants and fixture
state. Private inputs are not required for these nine cases.

## Joined acquisition and physical save

The source and persisted `addComment` setting are installed in a temporary
source directory. A parser preflight checks independently authored complete
expectations for two index rows and two release tracks before any HTTP request.
Actual search then fetches an index through a redirect. Production DOM review
selects the second result, whose release request redirects again. The release
body uses real Latin-1 bytes, a declared charset and multiple HTTP chunks.

The independently expected release has provider `Web`, an `id` equal to the
requested `/album/start/7` URL, and a `url` equal to the final
`/releases/7/page` URL. These are checked separately. Relative index links use
the final index URL; the relative cover uses the final release URL. The cover
itself redirects to a controlled generated PNG. The positive case records
eight exact GETs: two for search, two for release, two for preview artwork and
two for apply artwork. The server checks method, path/query, Host, page/image
User-Agent, image Accept and absence of authorization headers.

Two destination WAVs are selected in reversed order, with a third unselected
sentinel. TITLE, ALBUM and COMMENT are checked; the persisted setting supplies
`Saved setting reached the writer`. The real public writer embeds the PNG,
replaces the sidecar and records `Tag source import` for two files. Physical
reload checks the expected fields and artwork. An independent bounded RIFF
walker verifies distinct original PCM for every WAV and equal PCM after Save,
Undo and Redo. Its in-memory PCM corruption control is detected. Whole-file
Undo/Redo, sidecar restoration and the unselected file are also checked. The
existing 2 GiB Undo reserve is unchanged.

The Node helper's `network_requests: 0` records the absence of JavaScript
fetches. It does **not** mean HTTP was absent: the separate server ledger
records all eight real positive requests and their responses.

## Mandatory refusal controls

The ordinary parent passes only when the positive worker succeeds and every
deliberate refusal worker fails at its exact intended boundary. All nine
workers report joined server cleanup, reaped children and no server errors.
Each refusal checks no writer call and unchanged WAV, sidecar and history
state; no success receipt is accepted.

| Case | Exact GET count | Required boundary and cause |
| --- | ---: | --- |
| Positive joined save | 8 | Save, physical reload, PCM and exact Undo/Redo pass. |
| `file://` redirect | 3 | `release refused`; exact `Loopback Replay returned HTTP 302 for <requested URL>`, constructed from the independently known fixture URL. No additional request occurs. |
| Credentialed HTTP redirect to the same loopback host | 3 | `release refused`; `Loopback Replay redirected to an address TagGroove cannot open.` The literal nonsecret username `blocked` exercises the custom redirect policy. |
| Chunked body above 8 MiB | 4 | `release refused`; `Loopback Replay returned more data than expected.` |
| Invalid PNG during apply | 8 | `public apply refused`; `The downloaded cover is not a supported image.` Preview used the valid PNG. |
| Cancellation during the release body | 4 | `release refused`; `Cancelled: a newer tag source request replaced this one.` |
| Wrong independent index expectation | 2 | `independent index oracle`, before release acquisition. |
| Wrong independent release expectation | 4 | `independent album oracle`, before review/apply. |
| Choose the wrong search result | 2 | `release oracle`, before a release HTTP request. |

Cancellation calls the actual Rust public cancel function after at least one
response chunk has been written. It is not a concurrent GUI-click test. Only
BrokenPipe, ConnectionReset or ConnectionAborted are tolerated for the
oversize response, or for cancellation after that explicit barrier and a
written chunk. Timeouts, EAGAIN and other I/O errors are not accepted as
successful refusals. Initial unregistered release/image attempts also verify
that preparation and HTTP request counts stay at zero.

## Backward controls and frozen evidence

Fifteen fresh backward-control invocations use the same retained binary:
the default multi/scalar replay, unsupported protocol version, three actual
private public-acquisition routes (Traxsource album, SoundCloud second result
and Traxsource TRACK Search), three existing generic payload mutations, four
private payload/selection mutations, and three bridge lifecycle controls.
Those bridge controls cover 256 KiB of drained stderr with bounded retained
capture, the exact parent deadline, and a live child closing stdout before
apply. The deadline and closed-protocol cases require kill/reap, unchanged
transport and unchanged physical/history state. These private routes still
use the existing in-memory fixture transport; they are backward compatibility
evidence, not private HTTP evidence.

Compile-only, the nine-case parent, those fifteen invocations, scoped strict
backend library/tests Clippy, rustfmt and diff checks pass. All 258 bound
compile/runtime inputs and 40 external bindings remain unchanged. The latter
union includes manifests and later invented inputs as well as the original 27
inputs; it is not a claim of 40 original source files.

| Frozen artifact | SHA-256 |
| --- | --- |
| `final-receipt.json` | `54162697a8ee0c3a8a3c38cb94b73ea133e6376895efc0116af891bc6b190783` |
| `parent6.log` | `99e313753bf7d37fd7aeae684f86262663dc47fcd638dcca0dd0e7392db878ca` |
| Retained backend binary | `a805afda3cb4f7b4b7b766c17cb3c7586d0d413bd087c723d11c674d9c3a36a9` |
| `workflow_replay.rs` | `9ca37094547a730e03c1ceea58a373a5b95d582fbf0230c286c476c6802f1a71` |
| `workflow_loopback.rs` | `efb27b71d28c9f363ba9150251ff565c06a43b2090d6088fbf31fb83b72c6e55` |

The receipt, individual worker logs, earlier immutable failures, source/input
manifests and APFS-cloned binary are retained under
`/tmp/taggroove-loopback-implementation-96s6tt6t`. Earlier attempts exposed
authored fixture assumptions, accepted-stream mode handling in the test
server, and an incorrect friendly-error expectation for `file://` redirects.
They remain recorded as fixture/harness construction failures. No product fix
or production defect is inferred from them.

Local execution uses a sanitized credential/proxy-free environment and a
reviewed OS profile allowing loopback TCP while denying external networking.
A profile control observed local TCP success and external denial. This is
plain HTTP on `127.0.0.1`, not TLS, live-provider or private-script wire
qualification. Native Tauri/GUI ingress, original Mp3tag execution, Windows,
devices and original/provider acceptance remain unqualified. These scoped
passes do not claim a full workspace, CI, final CODE review, merge or release
result.
