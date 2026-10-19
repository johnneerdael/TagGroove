# Remaining toolbar routes — 2026-10-18

The [catalog](../../src/toolbar.ts) appends 29 hidden choices to the existing
69 commands. Existing order, authored labels, visibility and eight visible
defaults remain intact. Custom toolbars can assign the same catalog without
automatically receiving new buttons.

The additions expose existing Copy as Text, utility, Help, list-order,
column/toolbar configuration, configuration save/load/folder and seven
cover-operation routes. They do not create new utility, restore, browser,
clipboard or artwork-write behavior. **Random sort** exposes the existing
shuffle route; **Unsorted** restores load order without rereading a playlist.

Cover aliases share the actual cover menu's eligibility through
`CoverUI.canRun`, including pending-read and busy checks. Dispatch rechecks
availability before calling existing handlers. Existing dirty/provider/CUE,
clipboard, overwrite/confirmation and configuration-restore policies remain.

The focused 88-test run passed toolbar render/guard/dispatch and existing
cover/document/XML/audio-notice controls. New cases exercise missing, varied
and shared pictures, pending artwork, stale calls, clipboard arguments and
cover plans, description cancellation, extract overwrite choices and accepted
or canceled configuration restore. Host effects are recorded by spies: no
native clipboard, picker, browser, provider or filesystem writes are inferred
from those UI tests. Production TypeScript/Vite and whitespace checks passed.

New labels/hints are available in English, Dutch and German. Catalog length
98 is not a one-to-one mapping to the original 75 identifiers or an acceptance
score. Original-ID reconciliation, imported toolbar layouts/custom symbols,
native pointer/keyboard/accessibility and installed-platform qualification
remain open. See [toolbar usage](../user-guide/customize/toolbars.md) and the
[uncovered ledger](../PARITY.md).
