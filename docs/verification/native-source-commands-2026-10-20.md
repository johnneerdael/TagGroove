# Native source-language command snapshot: L32 working branch

This records unmerged work on `codex/refresh-file-identity`. L32 remains in the
intake until a covering PR exists. Final GitHub Codex review, Linux validation,
merge and original/native/provider acceptance are separate requirements.

## Evidence and admission policy

The pinned original is Windows Mp3tag 3.36.1 x64, executable SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
Only static PE inspection was used. The runtime-function extent
`0x140322524`–`0x140325770` constructs the actual command registry;
`0x140323b56`–`0x140323c03` validates its argument signatures.
[The retained snapshot](../../tests/fixtures/web-source-native-3361-commands.json)
binds all **67** registered command names, signatures, arities and string
addresses to that subject. It is not a loose strings inventory or a historical
command-introduction database.

[The engine table](../../src-tauri/src/sources/web/engine.rs) matches all 67
names and their minimum/maximum argument counts. The existing C20 policy is
preserved: compile actual capabilities before installation, keep original
minimum-version declarations informational, and do not compare Mp3tag and
TagGroove version numbers. A source declaring minimum 99.0 with known commands
passes preflight; an unknown command still refuses. Compilation emits no file
effects. Matching names/arity alone does not prove every original behavior.

## Added commands and forms

| Change | Native static anchor | Resulting behavior |
| --- | --- | --- |
| IncOutput / DecOutput | `0x140320337`–`0x1403203bd` | Change the raw current output name, preserving buffer values; normalize only map access. BMP case-expanding characters do not leave extra letters after removal. |
| SayAll | `0x14031b8ce`–`0x14031b9bc` | Consume a maximal prefix from a character set, trim its emitted edges according to Trim, advance by raw consumed text. |
| Exit | `0x14031ff90` | Finish normally with accumulated outputs; never execute later commands. |
| Extern | `0x14031fba0`–`0x14031fe69` | Recognize the two-string callback request and report `NO EXTERN HANDLER`. The three inspected native web runners also supply no callback; no shell/process execution is invented. |
| SaveData | `0x14031fe87`–`0x14031ff1a` | Emit transformed retained input lines with CRLF for real host file output. A failed filesystem write continues. |
| OnError | `0x14031ff3c`–`0x14031ff86`, `0x140319d50`–`0x140319e66` | Stop on runtime failure; exact `<FAIL>` keeps failure, literal non-angle fallback replaces only current output and succeeds, other/empty handlers keep failure. Optional retained dump captures pre-substitution output; default `<FAIL>` skips the dump. Compile errors remain errors. |
| json_foreach_continue | `0x14031d8d8`–`0x14031d904` → `0x14032043c` | Advance the innermost iteration, restore context and skip its remaining body, including at exhaustion/reverse/nested loops. JSON-on/no-active-loop continues normally; JSON-off still errors. |
| OutputTo | `0x14031c8f3`–`0x14031c923` | A nonempty second argument replaces the buffer; absent/empty preserves it. The empty name is separate from the default Output buffer. |
| While | `0x140320204`–`0x14032031d` | Omitted/empty condition loops while a character remains and bypasses the optional limit; nonempty condition retains prefix/limit behavior. |
| Unspace | `0x14031aea5`–`0x14031aeb9` | Accept the ignored optional string. |
| JSON collection paths / array indices | `0x140321abc`, `0x14031e4c9`–`0x14031e656` | Omitted/empty loop path uses the current collection; omitted/empty and all nonpositive array indices join every element; positive indices remain one-based. |

The initial output name is `Output` (string `0x1409574d8`, assignment
`0x140318ddc`–`0x140318deb`). The index runner reads that named buffer at
`0x1401acae6`–`0x1401acaf5`. Empty OutputTo assigns the raw empty name instead.
Three existing unit examples and the authoring lesson now return explicitly
with `OutputTo "Output"`; the guide calls out the migration from the former
TagGroove empty-name shortcut.

## Host constraints and limits

SaveData/error-dump requests become real files through
[the existing debug writer](../../src-tauri/src/sources/web/mod.rs), with
sanitized plain filenames under `sources/debug`, UTF-8 bytes and atomic
replacement. Pre-existing symlinked sources/debug directory positions refuse
writes. Tests cover those existing links; no claim of adversarial concurrent
directory-swap resistance or Windows reparse qualification is made.

At most 64 data-file requests share a 16 MiB filename/content allowance.
Output names and values are reserved against the existing cumulative 16 MiB
budget before mutation. Default trace limits and the 2,000,000-step guard
remain. File-request bounds do not silently turn a requested artifact into a
successful write. Filesystem write failures remain best effort, as in the
inspected native command; parser budget refusal remains an error.

DecOutput refuses removal that would leave half a UTF-16 surrogate in an
output name. Native non-ASCII file encoding, broader UTF-16/case behavior,
historical introduction versions, original runtime and live-provider/native
acceptance remain unqualified. These constraints are explicit in the
[command guide](../user-guide/tag-sources/web-source-reference.md), which now
teaches all 67 names and the new argument forms. No unconstrained native
execution-equivalence claim is made.

## Focused validation

- Nine initial command/arity tests failed before implementation and passed
  after it. The symlink diagnostic-path regression failed before the host fix.
- Read-only review identified Continue outside an active loop and uppercase
  expansion before DecOutput. Each received a failing regression, then a fix.
  Resource tests cover Set/OutputTo refusal before mutation, 64 file requests,
  byte preflight, actual oversized retained input, error dumps and name guards.
- The final engine cohort passed **38 tests**; source conversion/admission/file
  checks passed **27 tests**, with the isolated N03 acquisition test left
  opt-in. Frontend/source/catalog/guide checks passed **69 tests**.
- An earlier wider source cohort passed 111 tests but exposed one old golden
  fixture whose empty While loop did not advance. The fixture now moves its
  cursor, preserving its empty-output diagnostic purpose under the native
  loop rule; it passes in the final engine cohort. That wider cohort also ran
  existing headless filesystem/audio-fixture workflows, not native UI.

Clippy, formatting, extraction, strict guide and ledger checks are recorded
with the committed checkpoint. No original application, native UI, live
provider or production audio library was used for these checks.

## Cross-crate fixture path correction

The later L22a test-target lint check also compiles this engine through the
core guide harness. It reproduced a compile failure because the command snapshot
path used `CARGO_MANIFEST_DIR`, which then referred to `crates/tag-core` instead
of `src-tauri`. The snapshot is now included relative to this source file, so
both consumers resolve the same committed fixture independently of the crate
manifest. This changes test inclusion only, not parser behavior or the 67-command
contract. Focused cross-crate compilation and the 67-command registry test passed;
library/test-target Clippy for both affected packages also passed. Exact run
identities are retained in the task checkpoint.

## PR73 executable guide repair

The first [protected Linux run](https://github.com/johnneerdael/TagGroove/actions/runs/39472886325)
exposed two stale guide-test assumptions: the fixed count still expected 59,
and eight added command sections lacked isolated executable-example markers.
The parser registry and written headings already contained 67 commands; this
was not a missing engine implementation.

The guide now supplies a complete primary input/script/result case for all 67
commands. Explicit case boundaries keep supplementary teaching snippets from
being attributed to the preceding command. The test requires the exact example
name set to equal the registry, runs both LF and CRLF input, and checks SaveData's
actual captured file with CRLF terminators. No parser behavior was changed.
All 52 focused web-source guide/engine tests pass locally; strict guide rendering
and Rust formatting are checked separately. Final-head hosted validation remains
required after this repair.
