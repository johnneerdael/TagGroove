# Resolved input roots and known rename receipts — 2026-10-20

This is a bounded C25/C59 correctness repair after merged baseline
`8cbf29bc5a7afcb3e51236769d5a7f4d2aa601b7`. It does not close L47a's
inline-folder rename or externally renamed-file discovery requirements.

## Reproduction and behavior

A loaded root can use `/var/...` while the scanner returns `/private/var/...`,
or a symbolic link can refer to a differently spelled physical folder. Earlier
folder-membership checks compared these strings directly. With Keep moved files
off, an in-place rename could therefore remove its row. A known in-app rename
of an individually loaded aliased file could also leave Refresh using the old
input name.

The backend now returns the filesystem kind and optional canonical address of
each input. The frontend retains the original input for loading and ordinary
Refresh, and uses its resolved counterpart for containment, relative tool
context and known rename receipts. A known file/folder rename updates affected
roots; converging roots are deduplicated with aligned kind/address metadata.
Refresh and workspace restoration re-read resolution, so retargeted aliases do
not retain an old physical address. Resolved addresses are runtime metadata;
saved workspace format and logical root serialization remain unchanged.

Real outside-folder moves still follow Keep moved files; pending tag edits retain
their existing row-preservation policy. POSIX literal backslashes remain filename
characters in containment, parent calculation and receipt rebasing. Windows
drive/UNC separator controls remain covered by modeled tests. These are not
original Windows or installed Windows results.

## Focused local evidence

- Before the repair, four extracted production-function cases have two canonical
  controls passing and two alias cases failing. Retained root:
  `taggroove-source-root-alias-red-90o_lr8e`. The independent four-case probe
  also reproduces those two failures without executing filesystem renames.
- The real Unix symlink metadata regression fails before implementation because
  its resolved address is absent (`taggroove-source-root-metadata-red-8h01hk5e`).
  The separator regression separately has two failures before its fix
  (`taggroove-source-root-native-separators-red-fnovglkg`).
- The directly affected loading, ordering, playlist, workspace, startup and
  containment cohort passes **99 tests**, zero failures or skips
  (`taggroove-source-root-alias-final-wodkwph5`). A separate named bundled
  diagnostic passes (`taggroove-source-root-alias-diagnostics-dimv3q_4`).
- Independent review found missing resolved-state fixtures in ordinary-workspace
  tests and a newly reachable POSIX backslash receipt-rebasing error. Both were
  reproduced before correction (`taggroove-source-root-review-red-i7eqyq95`).
  The affected action, ordinary-workspace and loading cohort then passes **69
  tests**, zero failures or skips, in **2.08 seconds**
  (`taggroove-source-root-review-green-rxqan13_`). These cohorts overlap; their
  counts must not be added as unique coverage. Final static re-review reports
  no remaining concrete findings.
- TypeScript checking passes (`taggroove-source-root-alias-final-types-1as3l4wn`).
  Scoped backend Clippy with warnings denied passes
  (`taggroove-source-root-alias-final-clippy-jkw157tc`). Formatting and whitespace
  checks pass. Both focused backend metadata tests pass, with 590 unrelated
  tests filtered out (`taggroove-source-root-alias-final-metadata-lbey72z0`).
  Eight documentation publication tests and writing-plan validation pass;
  all **525 local links/anchors** across the seven changed Markdown files
  resolve. No broader backend or native UI suite was repeated.

The final production-source SHA-256 bindings are:

| Path | SHA-256 |
| --- | --- |
| `src-tauri/src/playlists.rs` | `eba6015f443cf8ac78dbaf6679582f57f306439b439de77bf57e2a236bbd67fd` |
| `src/action-ui.ts` | `a77d321d0af8db45081dc09055bd38b9dd712bb6b448419ec2c0d3e10be958b8` |
| `src/general-options.ts` | `013f93e1d2ecb01a2523bdd205e4d2d8b2148d8be5befc302dac8a43fc9218ef` |
| `src/main.ts` | `4b828673755f253740053227e2e491881f964c0af3e11581e972187403d33dc1` |
| `src/playlist-order.ts` | `75d6a0e1e65bc116c8c6b8724a3c32ecb824768b84a49efba4334df4e5c7f642` |
| `src/playlist-ui.ts` | `29eb7ae5d8490cb5ecf7b33e5e132b6f4c13349f03bfc7b3c9a924ca5f0ddc6e` |

## Native and parity boundaries

No local app build, native UI replay or broad frontend/Rust sweep was performed
for this repair. The [earlier four-process native receipt](native-workspace-acceptance-2026-10-19.md#focused-merged-source-replay)
retains its original `e4017801dd923b65e1e5e2fc0ece58706374b60d` production pin.
It does not qualify this changed alias-aware loading/restoration code. Native
integration belongs to postmerge CI; actual installed OS, original Mp3tag and
physical provider acceptance remain distinct evidence layers.

The active intake remains 49 IDs with 14 separate qualifications. This repair
adds no general external-rename search, inline folder-path editing or new save,
history or no-clobber contract. Follow the existing
[OS-specific procedures](../handover/PARITY-VERIFICATION-PROCEDURES.md) for the
remaining engineer handover.
