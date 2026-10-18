# Consolidated preservation and recovery follow-ups — 2026-10-18

This PR64 checkpoint follows the [round-trip checkpoint](parity-roundtrip-2026-10-18.md).
The earlier counts remain historical. This batch adds the following bounded fixes
and their guide updates:

- [Exact ID3 placeholder provenance](id3-y-provenance-2026-10-18.md): legitimate
  experimental Y IDs coexist with generated encrypted-record placeholders.
- [Source-update recovery](source-update-recovery-2026-10-18.md): retain the previous
  snapshot through refused updates and recover the named publication/bootstrap
  process-interruption windows.
- [Cached date refresh](library-date-refresh-2026-10-18.md): reuse raw timestamps
  through the reader formatter, with invalid legacy entries repaired by rereading.
- [Manual Refresh order](refresh-manual-order-2026-10-18.md): keep surviving exact
  row order while accepting fresh values and independently retaining source rank.
- [Regex guidance](guide-reference-details-2026-10-18.md): explain global prefix
  context and execute the literal guide example with both LF and CRLF Markdown.

Implementation identities: metadata/cache `9930efe1a21984599a0e4521f5b5b33352f0defb`,
source recovery `9f22fc224bbe1d55a00b2e4b1d04d08ed48a2e06`, and manual
Refresh/regex guidance `fc3a2723a38458c89ee3d750358988dbd64f4c71`.

## Combined local checks

The final combined Rust workspace run passed **1,305 tests, zero failures and
14 ignored**. The ignored tests retain their existing opt-in, platform or worker
roles; the source-recovery receipt explains its six ignores. All eleven Library
tests and all guide-example consumers run in this workspace result.

The frontend run passed **760 tests, zero failures and zero skipped**. Production
TypeScript/Vite build, strict all-target workspace Clippy, Rust formatting,
canonical English extraction, seven documentation-workflow tests and strict
MkDocs also passed. The canonical catalog contains 3,389 keys; the new recovery
messages have Dutch and German translations.

The regex article gained one literal action example. Initial combined runs
correctly exposed older fixed-count consumers expecting five regex fences;
those consumers now accept six while still executing the five original scripting
examples. All 30 focused guide/checkout tests passed before the final workspace
run. Failed intermediate runs are not counted as passing validation.

## Evidence boundaries

These are local backend/filesystem and controlled production-DOM checks. Source
recovery and ID3 slices also received independent local reviews with their
reported findings resolved; these do not replace the final GitHub Codex code
review. The current four private folder/ZIP admission checks preserve all inputs
and do not qualify provider requests or native apply/save journeys.

Portable backup continues to contain saved settings and local workspace
references/order only. Unsaved edits, audio and credentials are excluded, and
configuration restore does not save audio. Ordinary cross-session order remains
L47b. Windows work is deferred by the owner. Native/original/provider/device
acceptance, immediate already-loaded OS-locale notifications, arbitrary concurrent
filesystem mutation and power-loss durability remain outside this checkpoint.

PR64 remains draft. No final GitHub CODE review, current-main immutable validation,
merge, release or guide deployment is established by these local checks.
