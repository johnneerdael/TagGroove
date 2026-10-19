# Daily workflows: C58–C60 — 2026-10-19

This receipt represents the locally implemented toolbar-band, ordinary restart
and loaded-row date-refresh scopes in PR64. It removes those
implementation tasks from [active intake](../PARITY.md), retaining their
historical L02/L47b/L60 anchors on [C58–C60](../research/remaining-gaps.md#c58).
C57 is reserved for the separately coordinated native action-file repair.
The implementation checks below retain their original source checkpoints.
Later named native journeys are recorded in the
[native workspace receipt](native-workspace-acceptance-2026-10-19.md). Neither
receipt supplies release or original Windows-runtime proof.

## C58: toolbar bands

The main customization dialog now provides a native select control for four
arrangements: main/custom first on a shared row, or main/custom above the other
on separate rows. Arrangement persists with main-toolbar settings; Cancel
preserves the saved state. Command identities, saved labels and item visibility
remain their existing independent settings. The filter keeps its own dock.

This explicitly controlled arrangement is an independently justified accessible
equivalent for band order and row placement. It does not implement arbitrary
floating coordinates or free dragging. Vendor toolbar layout/symbol import
remains L50. The real production customization dialog and band renderer are
exercised by [the focused tests](../../tests/daily-layout.test.mjs), including
normalization, save/reopen, Cancel and left/right/list filter placement.

## C59: ordinary list restart

The desktop app saves a typed local workspace reference snapshot to its browser
store, independently of migration rollback and portable configuration export.
It retains exact physical/CUE/chapter row identities, manual insertion order,
source rank and the selected manual/source/column display mode. Startup reads
those rows again through the existing exact-row scanner. Explicit launches win
before restoration; requests queued during restoration are applied afterward.

No audio, tag/artwork drafts or provider grants are serialized. Structural row
revision/signature checks skip capture and serialization on unrelated renders,
field changes and locale refresh. Normal close synchronously captures final
selection/filter state. Payload parsing/writes have a 16 MiB guard and browser
storage quotas may be smaller. Schema validation checks absolute local paths,
unique rows/source rank and selected/anchor membership without imposing the
portable backup's distinct 10,000-row migration cap.

Corrupt or unavailable saved-row restoration preserves the prior snapshot
through automatic render/focus/refresh and forced close until a real
membership/order/root change. A failed storage publication does not advance
the successful-save identity; normal close can retry the unchanged snapshot
and a successful retry clears the warning. Both defects were independently
reproduced against the real production handlers before correction.

See [capture/startup production tests](../../tests/configuration-workspace.test.mjs),
[startup queue tests](../../tests/file-ops.test.mjs),
[validation/revision code](../../src/workspace-session.ts) and
[orchestration](../../src/main.ts).

## C60: loaded-row date refresh

The [observer](../../src/locale-refresh.ts) checks native formatter state after
language/focus/visibility notifications and once per second while visible and
idle. A changed fingerprint triggers RAW-backed reformatting of loaded
technical fields; current-entry identity guards reject stale replacements and
evaluated column/order/filter caches are invalidated. Audio is not rescanned,
tag/artwork drafts stay staged and no save is performed.

The backend contract is `locale_format_state() -> string` and
`reformat_loaded_timestamps({items:[{path,technical}]}) -> same items shape`.
Known file/MP4 RAW timestamps feed the reader formatter. Invalid raw values
clear only their stale derived date fields while preserving raw/unrelated
fields. macOS formatting uses fresh CoreFoundation region, date/time pattern
and timezone state. Windows/Linux retain their existing formatter paths and
need their own native regional-format mapping qualification.

The first backend receipt is `/tmp/taggroove-locale-backend-receipt.json`:
three command tests plus three real CoreFoundation tests passed. They cover
explicit en_US/en_GB regional formatting, a +14-hour date crossing, custom-pattern
fingerprints and equality with actual reader output. Those tests changed no OS
preferences and opened no native window. The receipt binds source, binary and
logs. Initial 1,000/10,000-item command timings were 381.10/3957.28 ms in that
local baseline, prompting a separately coordinated one-snapshot batch refactor.
That refactor was initially still undergoing its own compilation/rerun; its
subsequent frozen receipt is `/tmp/taggroove-locale-batch-receipt.json`.
It records three command tests passing (the manual benchmark is intentionally
ignored in the normal run) and four real CoreFoundation tests passing, including
an independently reproduced RED whole-batch region/timezone/mixed-invalid-RAW
control. Source hashes match its GREEN compile.

The same manual production-command benchmark ran once per size/version on this
host/debug build: the single-snapshot version measured 22.20/117.48 ms for
1,000/10,000 items versus the 381.10/3957.28 ms baseline. These in-memory timings
exclude request construction/assertions, WebView serialization/rendering,
disk/media I/O and native preference transitions, and enforce no timing deadline.
The first six-test receipt remains historical evidence for its earlier source;
the later batch receipt supplies the changed-source rerun. Neither receipt
certifies final integrated-head checks or a live OS-setting transition.

## Executed frontend evidence

All logs below are retained under `/tmp/taggroove-daily.8zdRpZ/`. Each matching
JSON records the exact command, isolated environment, exit code and source
SHA-256 bindings; logs are unfiltered test output. The Node preload denies
network access. No Cargo, app, browser, GUI, provider or device run was part of
this frontend execution.

| Cohort | Result and retained logs |
| --- | --- |
| Band placement | `toolbar-red.log`: two expected production-dialog/renderer failures; `toolbar-green.log`: 20/20 pass. |
| Ordinary persistence | `workspace-red.log`: two expected missing persistence/startup-handler failures; `workspace-green.log`: 55/55 pass. |
| Quota reporting | `workspace-quota-red.log`: warning assertion fails; `workspace-quota-green.log`: 42/42 pass. |
| Loaded-row notification | `locale-red.log`: two expected missing observer/publication failures; `locale-green.log`: 19/19 pass. |
| Structural persistence cache | `workspace-efficiency-red.log`: missing structural revision behavior; `workspace-efficiency-green.log`: 69/69 pass. |
| Failed-restore and quota-retry review findings | `persistence-review-both-red.log`: 7 pass, two expected production-handler failures. `persistence-review-green-final.log`: 102/102 pass across eight focused files. |
| TypeScript/whitespace | `tsc-final.log`/`tsc-final.json`: exit 0 for their bound source snapshot; TypeScript and `git diff --check` also passed after the persistence review fixes. Final integrated-head checks remain separately required. |

The 102-test command was:

```sh
node --import /tmp/taggroove-daily.8zdRpZ/deny-network.mjs --test --test-concurrency=1 tests/daily-layout.test.mjs tests/configuration-workspace.test.mjs tests/file-ops.test.mjs tests/playlist-refresh.test.mjs tests/toolbar-catalog.test.mjs tests/toolbar-followups.test.mjs tests/columns.test.mjs tests/filter-sync.test.mjs
```

One intermediate post-review run passed 99/102 because three extraction
harnesses omitted the newly accessed error-state fixture. That output remains
`persistence-review-green.log`; the fixtures were corrected and the full named
cohort rerun to the 102/102 result above. Neither that intermediate failure nor
earlier source-bound counts are represented as final-head integration proof.

## Native journeys: historical cases and current acceptance

The original preparation at `/tmp/taggroove-native-daily.y4p1t7ee/` supplied
fixtures and a procedure; it supplied no runtime result at that checkpoint.
Subsequent native runs are recorded with their separate source/executable pins
in the [native workspace receipt](native-workspace-acceptance-2026-10-19.md).
They precede final PR64 head `550a1eed241fa05f711f211dabad65ef3a4913e3`
and its root-classification changes to loading, workspace serialization and
restoration. Preserve their exact source/executable pins. A [focused merged-source
replay](native-workspace-acceptance-2026-10-19.md#focused-merged-source-replay)
records suffixed-directory Refresh and manual/playlist-source-order normal
restarts only. Other C59/C47 cases remain pending. C58 controls and geometry
remain historical evidence for the older builds.

| Scope | Native macOS evidence and source limits | Still pending on this Mac |
| --- | --- | --- |
| C58 | All four arrangements through native controls; save/reopen/Cancel, ordinary restart and list/left/right filter docks with typing/Enter/F3. Normal/compact geometry and a separate fresh compact Files/Tags/select/restart control are observed. | VoiceOver, broader keyboard/IME/accessibility and the retained AX/AppKit menu-tracking observation. The geometry case does not certify all controls at every size. |
| C59 / C47 | Earlier builds observed manual reorder and normal restarts, independent source/manual/column modes, exact CUE and embedded chapter identities, missing-CUE recovery, corrupt-JSON retention and native folder-picker recovery. Running-app explicit replace/add and relative paths with spaces plus pending-edit No/Yes retain their original pins. The focused current-source replay observes only suffixed-directory Refresh and manual/playlist-source-order normal restarts. | Current running-instance CLI including relative paths/pending edits, column mode, CUE/embedded chapters, missing/corrupt recovery, actual storage-quota refusal/retry and queued arrivals during restoration; the separately documented forced-new-process route and broader native input/accessibility. |
| C60 | Source-bound formatter/handler controls above; no actual OS-preference-transition result. | Actual live region, custom date-pattern and timezone transitions while tags/artwork remain staged. Observe loaded-row changes without a rescan/save and preserve RAW values and whole-media hashes. |

The historical cases and focused replay establish only their named source-bound observations;
they do not close current merged C58, C59, C60 or the cross-cutting L78/L79
qualifications. Retained earlier failures
and limits remain visible. Use [the environment register](../handover/PARITY-OTHER-MACHINES.md)
and [engineer procedures](../handover/PARITY-VERIFICATION-PROCEDURES.md) for native
Windows/Linux and original Windows 3.36.1 work. Synthetic DOM events, direct
backend calls and CoreFoundation controls keep their separate evidence limits.
