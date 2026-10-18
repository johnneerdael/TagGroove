# Convert Codepage group memory — 2026-10-18

Scope: the L16 implementation in consolidated draft PR64. This is local production-function/DOM evidence, not an original-program comparison or an installed native restart test.

New group actions reuse the last offered codepage confirmed with **Keep action**. The memory is the optional validated `groupCodepage` in the existing `opentag.actions.quick` envelope. Opening, cancelling or changing action type alone does not update it. Keeping an action updates this preference even if the outer group's unsaved definition is subsequently discarded; saving the group remains a separate operation.

Existing/imported actions keep their own settings. Imported aliases outside the offered dropdown do not become new-action defaults. Quick actions retain their independent last type and complete per-type settings through each write. Missing, invalid or unavailable storage falls back to the original default. The field remains TITLE for new actions; the change does not alter conversion behavior or save audio.

## Executed evidence

- [Production implementation](../../src/action-ui.ts).
- [Nine new focused regressions](../../tests/action-codepage-memory.test.mjs), plus the existing action tests: 27 passed locally.
- Four intended failing controls preceded the implementation; the other controls passed at that checkpoint.
- The TypeScript/Vite production build passed on the implementation batch.

The tests use actual production functions and isolated browser storage; they do not establish operating-system profile persistence, original choice/timing, native dialog behavior or full automation parity. Portable backup of this preference remains separate L57a work until its implementation is delivered.
