# PR64 CI repairs — 2026-10-19

The [reviewed validation run](https://github.com/johnneerdael/TagGroove/actions/runs/39291679239)
for `355484a27f6fef506bc76196ac46ff413490b8eb` failed on Linux, Windows
and macOS. Tooling/UI, strict documentation and Android passed on that old head.
The result below is local repair evidence; the failed CI run remains failed.

## Changes and reproduced failures

- Linux: an APE edit can create a new `TAG` lookalike at EOF-128 before the
  ID3v1 writer runs. A guard immediately before that writer preserves APE and
  the requested values. The independently authored carrier fails before repair;
  its no-V1 control passes. All 11 raw-stream tests pass after repair, including
  real FFmpeg decoding controls. See the [APE transition receipt](ape-id3v1-transition-2026-10-19.md).
- Windows: six configuration-transfer tests assumed successful POSIX workspace
  restoration despite the existing `unsupported-local` boundary. Three POSIX
  success cases now require Unix; three asset/preferences/journal cases explicitly
  select settings only. Both unsupported and foreign styles retain inventory,
  refuse selected workspace Apply and allow reviewed settings-only restoration.
  Production policy and shared selection defaults are unchanged. See the
  [configuration receipt](portable-configuration-2026-10-18.md#windows-ci-test-domain-correction-2026-10-19).
- macOS: the six native scripts treated below reproduced their failures locally on the repaired app
  before the test updates. The table records their final runs on that same app.

| Suite | Corrected test contract | Final phase | Passed |
| --- | --- | --- | --- |
| keyboard-extended | Multiline completion keeps the typed prefix until an explicit choice; unselected Enter is not consumed, and typing does not save tags. | green-keyboard-2 | 14/14 |
| script-export | Compare semantic action definitions, require retained native metadata for types 17–19 when present, and compare raw records after a second export/import. | green-script-2 | 7/7 |
| panel-dock | Measure the current toolbar-bands wrapper; retain filtering and F3 checks. | green-panel | 12/12 |
| chapters | Observe exact restored physical rows, then explicitly Refresh after changing chapter settings. | green-chapters | 6/6 |
| chapter-fields | Use the same explicit Refresh before chapter-field operations. | green-chapter-fields | 8/8 |
| formats-edge | Exercise explicit MPEG APE read/write choices, retain enabled APE on Save and select APE for removal; close a failed dialog to prevent cascades. | green-formats | 5/5 |

The total is **52/52**. These are native debug-app/WebKit/backend/file operations
with scripted DOM events. The synthetic Enter check proves non-consumption; it
does not prove physical keyboard newline insertion, IME, accessibility, installed
release behavior or original Mp3tag equivalence. No production frontend behavior
was changed to satisfy these scripts.

## Retained evidence and source binding

All roots below are under the authoring Mac's temporary directory:
`/var/folders/cm/m92k6_qs115046xzbll7hfkh0000gn/T/`.

| Check | Root | Result |
| --- | --- | --- |
| Complete Rust workspace | taggroove-native-daily-workspace-tests-n1b4wdiy | 1,513 passed, 0 failed, 21 intentional ignores; 84 executables, all exit 0 |
| Strict workspace/all-target Clippy | taggroove-ci-platform-repair-strict-clippy-vgy0rs7b | `cargo clippy --workspace --all-targets --locked -- -D warnings`, exit 0 |
| Strict guide | taggroove-ci-platform-repair-final-guide-g6zar8lm | Exit 0 |
| Native build | taggroove-ci-native-repair-app-build-6m6lbjss | TypeScript/Vite and isolated ad-hoc-signed debug app, exit 0 |
| Six native suites | taggroove-ci-native-repair-tndlblgz | Per-phase identity, executed script, inputs, outputs, profile archives and results retained, including all RED and intermediate failures |
| Full frontend suite | taggroove-ci-repair-final-ui-i0jelq_i | `npm run test:ui`: 851 passed, 0 failed/skipped |
| Formatting | taggroove-taggroove-ci-repair-final-fmt-n7dmnlm2 | `cargo fmt --all -- --check`, exit 0 |

The native executable SHA-256 is
`9a8a79d0adaf137d40d63c9b1426cc130c2b9fd43e939f3993d174816c9823b3`.
Its unique app identifier is `org.taggroove.editor.e2e.cirepairbe506642`.
Build context and each phase bind production source hashes; the Rust executable
set has compiled-source bindings. Rechecking those bindings found no drift.
The production app was built from `355484a` plus the uncommitted Rust repair;
per-phase driver hashes distinguish the later test changes.

Final native test SHA-256 values:

- `tests/e2e/keyboard-extended.js`: `52f931f8aa9353eecab0fed9c48cc4881cdd696f3c6806666fff3a7366c22b3b`
- `tests/e2e/script-export.js`: `cc0d2701c3bfe4d4db48c3bb0111e82de689d5f2e5fb4525f195f3f6860fc571`
- `tests/e2e/panel-dock.js`: `33c5370364904ec44640d7de39bb90184e585425bd439eafd0ffea147657f8bf`
- `tests/e2e/chapters.js`: `85e493fdcba48cbe3485edf0fc2a5908a8e0255b31e98e12dc16e7034d7e12c1`
- `tests/e2e/chapter-fields.js`: `0f9d96934433db46e4585e018d107f5580d11d5d9ffdd8cb8d7d1b478a63051f`
- `tests/e2e/formats-edge.js`: `4aa381108fdd1cb445ed83f090e94ed21ff8afaa40f7dc7d56f87de1d9372e2f`

A scoped local review found no blocking issue in the tag guard, Windows test
domain or six native-script repairs. It is separate from GitHub Codex approval.
A new exact-final-head GitHub CODE review and trusted current-main validation
are required before merge. No local result proves native Windows/Linux reruns,
release signing, notarization or publication.

## Subsequent complete-run audit

Run 39300072099 passed Android, tooling/save acceptance and documentation, but
failed directory-history replay on Linux, an open-destination identity fixture
on Windows, and three other Mac suites. Those three suites—options-utils,
general-options and list-behaviour—were also failing in the first Mac job and
were initially missed; the complete failure inventory is now audited. The six
suites in this receipt passed on the second run.

The [directory identity repair](history-directory-identity-2026-10-19.md)
records deterministic reuse and real rollback reproductions, four passing new
controls, complete current-source Rust validation, and the Windows fixture
correction. The three native-script repairs now pass locally as recorded below. A fresh
exact-head review and immutable CI remain required. Neither earlier run is overall green.

## Remaining native contracts: second repair

The unchanged options-utils, general-options and list-behaviour scripts reproduced
their CI failures on the same isolated current-source debug app before correction.

| Suite | Correction | Final result |
| --- | --- | --- |
| options-utils | Exercise current configuration export/preview/apply, explicit settings-only selection and production reload/acknowledgement. Reject invalid input before Apply can cause a pending-state refusal. | 13/13 |
| general-options | Start the previous-directory case without the disposable ordinary-workspace snapshot, whose restored selection takes precedence. Keep inline completion and require its matching filtered list with aria-autocomplete both. | 15/15 |
| list-behaviour | Require persisted manual mode while preserving a nonempty saved column sort and independent source rank. Retain keyboard/drag order, numbering and all save-on-move assertions. | 10/10 |

Completing the manual-order test restores its intended starting order for later
steps; the drag and Ctrl+N failures then disappear without changing production
selection or save behavior. The scoped draft review found no blocking issue.

A subsequent inventory covered all 27 suites from the actual CI workflow on the
same repaired executable, and all 27 passed. The retained local runner initially
omitted OPENTAG_E2E_BINARY for the launch-forwarding fixture, so that case did not
launch; the corrected targeted run supplied the documented environment and
passed. Those preparation failures and every original/intermediate failure are
retained. The other passing suites were not repeated for that harness repair.

Root: `taggroove-ci-native-round2-6x43lsie` under the authoring Mac temporary
directory. `final-native-inventory.json` maps each suite to its executed driver,
result and SHA-256; every production and final driver hash matches the current
files. The app build/log root is `taggroove-ci-native-round2-build-kc2r4wuu`.
The native executable SHA-256 is
`2bf9d6655aa562b8957bc0a5719a3e385e1ceff6aede9ff3afacf2f1faabade0`;
identifier `org.taggroove.editor.e2e.ciround2e2373403`, 1024×800 test window.

This is scripted native WebKit/backend/file evidence. Physical input, original
Mp3tag comparisons and other OS/device acceptance retain their separate limits.
The full Rust workspace passes 1,517 tests/84 executables with 21 intentional
ignores; strict workspace/all-target Clippy, formatting, TypeScript/Vite and the
strict guide build pass for their bound current snapshots. Frontend production
code is unchanged; its earlier 851-test local checkpoint belongs to c169e9ee.
Full frontend/platform validation of the new head belongs to reviewed CI.
