# Default tool: No action — 2026-10-19

The [original Tools documentation](https://docs.mp3tag.de/customization/options/tools/)
defines No action as doing nothing on a File List double-click. TagGroove's
option had that label but still fell back to Extended Tags when no usable tool
was selected. This was an identified R11/L51 daily-editing discrepancy.

The row-double-click handler now invokes a configured default tool only. No
action, missing configuration and an unavailable tool index no longer issue the
Extended Tags command. The existing slow-edit timer is still cancelled, and
non-row targets do not launch tools. Tool selection/scope, confirmation, error
handling, menus and shortcuts keep their existing controller paths. Extended
Tags remains available through its explicit menu and keyboard commands.

## Regression evidence

[The controlled-DOM test](../../tests/file-list-double-click.test.mjs) locates the
actual `wrap.addEventListener('dblclick', ...)` registration using the TypeScript
AST and binds that production callback to a Happy DOM table. It uses the real
`ToolsUI` controller; host effects are recorded, never actual program launches.
It does not reconstruct the double-click dispatch policy in a second helper.

The baseline fails the No action assertion because it records `extended`.
Configured-tool and timer/non-row controls already pass. The repaired three
controls pass for missing, explicit No action, invalid and valid tool settings,
the exact selected-file payload, no fallback operation and cancelled delayed
cell editing. The four affected options/keyboard/guide suites pass 23 tests.

This is local production-callback evidence, not native OS double-click,
external-process, original Mp3tag runtime or device acceptance. A normal click's
selection/save-on-move behavior is separate. Windows tool forms, non-path
placeholders and the per-file launch limit remain L51; the No action mismatch
must not remain as an implementation task.
