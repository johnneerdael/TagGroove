# Top menu switching regression

Verified on 2026-10-14 with the macOS arm64 Tauri debug bundle in
`target/branding-verification/debug/bundle/macos/OpenTag.app`.

Before the change, native UI automation opened File and then Edit. Both
summaries reported `Value: on`, reproducing the user's overlapping-menu
screenshot. See `menu-switch-before.png` and `menu-switch-before-ax.txt`.

The document installs one delegated controller for the five top-level menus.
Opening one closes the other top-level menus and their nested disclosures.
Nested action-group disclosures are excluded from top-level exclusivity.
Delegation survives workspace re-renders without adding per-render listeners.

Native UI checks performed through CUA:

- File → Edit → Convert → Actions → Help: exactly one open summary after each
  click, recorded in `menu-switch-sequence.json`.
- Clicking the current Help summary closes it.
- Escape closes Help and returns focus to its summary.
- Return reopens Help; Left switches exclusively to Actions; Right switches
  exclusively back to Help.
- Clicking the Filter field closes File and focuses Filter.
- Option+A switches from Help to Actions and focuses Actions. macOS emits a
  modified character for Option+A, so the shortcut also checks physical KeyA.
- Space closes Actions without being consumed as an action-group mnemonic.

The final native build replay produced `menu-switch-after.png` and
`menu-switch-after-ax.txt`. The workspace was empty; no music was modified.

Additional pointer checks in Chrome against the production frontend served
locally: open File, move the mouse over Edit without clicking, then click Edit.
Read-only DOM observations confirmed that only Edit was open after the hover
and remained open after its first click. The native app also passed closing an
already-open menu by click. A later browser automation timeout prevented
recording the second Chrome click, so that case is not claimed as a separate
Chrome pass. Chrome's client extension blocks some brand images; the native
screenshot verifies the actual branded application.

`npm run build`, `npm run test:ui` (32 existing tests), and the native Tauri
debug app build passed. These are focused macOS/Chrome checks; Windows, Linux,
touch-device, and configured nested action-group runtime checks remain separate
acceptance work.
