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
- macOS: all six failing native scripts reproduced locally on the repaired app
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
