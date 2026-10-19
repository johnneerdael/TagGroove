# Native action-file compatibility (C57)

PR64 implements the canonical native Type 17 (Adjust Cover), Type 18 (Set Cover
Properties) and Type 19 (Generate Playlists) records. This closes the known
schema implementation gap formerly tracked as L17. Original Windows execution,
original-generated files and original GUI re-import remain qualification.

The independently authored semantic fixture is
`crates/tag-core/tests/fixtures/parity-native/mta-schemas.json`, SHA-256
`539270d4780851abdf606369e3112a053db1345316ba2c0f070d6f56191daccf`.
It retains the frozen static predictions rather than replacing them with
expectations copied from the implementation. All 13 native records were
unresolved in the retained RED execution and are executable in the focused
GREEN execution. Three additional records contrast the older application-only
schemas.

## Resulting behavior

- Parse native format, packed size/shape/policy/state, quality, picture role,
  description, filter and playlist settings with their distinct meanings.
- Preserve native source settings on the three recognized variants, including
  the proven inactive Type 18 option 3 and empty or default sentinel spellings.
  Reconcile edited semantics before reusing retained source spellings.
- Keep legacy 1001–1003 readers and old saved action JSON defaults. Use native
  export only where it represents the same behavior. Legacy-only quality,
  dimensions, shape behavior or literal sentinel-like descriptions retain the
  legacy representation. A conflicting native/legacy option 3 refuses before
  replacing the output.
- Apply square/crop geometry independently of the resize-enabled setting.
  Invalid enabled dimensions skip the adjustment, including format conversion.
  Preserve the distinct maximum/minimum sizing rules and disabled-size shape
  behavior; do not reinterpret older saved shape geometry.
- Ask the backend which actions actually require legacy export and show the
  resulting warning. A hidden empty warning no longer retains a language-change
  listener after dialog errors are cleared.

## Executed evidence and limits

The initial focused GREEN run passes five native schema/source-reconciliation
tests (including every native fixture record), nine opaque-record tests, 46
action unit tests and the independent five-vector geometry regression. The RED
executables, source bindings and complete logs remain under
`taggroove-native-mta-red-build-g3emr5hy` and
`taggroove-native-geometry-red-ywe32pic`; the focused GREEN receipt is
`taggroove-mta-green-rebuild-v60f6uxx`.

The focused UI cohort passes 21 tests. The full UI run subsequently exposed two
real listener-lifecycle regressions; both were reproduced before correction,
and the combined lifecycle/MTA-language cohort passes ten tests. Those logs
remain under `taggroove-mta-ui-listener-red-km6hjhem` and
`taggroove-mta-ui-listener-green-j17oxrh0`. Final integrated branch checks are
recorded separately; these focused results do not replace them.

## Resize working-allocation guard

Static inspection of the pinned image library found that decoding's existing
512 MiB allocation budget does not propagate into resampling. For example, a
narrow 20,000 × 1 input can request a 20,000-square result and a multi-gigabyte
RGBA32F intermediate. Existing legacy transforms also reached amplified
allocations. No oversized image allocation was executed to reproduce this.

A dimension-only regression first failed with the guard absent. A shared checked
preflight now accounts for source/crop buffers, RGBA32F intermediates, complete
resized and final-crop output, and conservative sampling-weight vector capacity.
An additional dimension-only 1 × 130,000,000 grayscale case exposed the missing
weight-vector allowance before that allowance was added. Both RED executables
and source bindings remain retained. The 512 MiB processing estimate is a new
shared safeguard, separate from the decoder, input, target-size and journal
limits; it is not a whole-process resident-memory guarantee.

After the buffer preflight passed its arithmetic controls, small encoded PNGs
exercised real native and legacy refusal paths, and an actual audio-file case
verified exact unchanged bytes. Those focused controls pass without attempting
the oversized transformations. Final whole-workspace results are recorded in
the integrated checkpoint.

Unknown native settings and unqualified numeric-prefix/sentinel spellings remain
opaque rather than being guessed. Native format 3 is also deliberately opaque.
Encoded-image quality/rounding, complete original-generated matrices, global
playlist options and original Windows import/export/readback require further
qualification. The local Mac native editor journeys remain local work in the
[handover register](../handover/PARITY-OTHER-MACHINES.md); Windows reference
execution requires the pinned Windows 3.36.1 artifact on Windows.

Final integrated source is covered by the [1,450-test workspace and 842-test frontend checkpoint](local-parity-closeout-2026-10-19.md#integrated-native-action-and-daily-workflow-checkpoint). Original/native qualification remains separate.
