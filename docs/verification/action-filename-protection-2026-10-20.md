# Edited Windows action filenames — 2026-10-20

This L54/C63 follow-up protects edited `_FILENAME` outputs on Windows. It keeps
Unix-valid names and the existing template/value-origin policies separate.
The implementation checkpoint is `b462ca2bc1b2922fead20671f60d587029495b1a`;
final PR review, CI, native execution and publication are separate requirements.

## Original shipped-artifact evidence

Target: Windows x64 Mp3tag 3.36.1, SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
REA 6.4.0 and Ghidra 12.1.4 inspected a copied executable on this Mac. The
target was never executed. The retained 44-record evidence bundle is
`/tmp/taggroove-l54-mp3tag-static-evidence-final-2026-10-20.json`, SHA-256
`ea83ba559848645d8f62481e6f66074b553bdd7a45f9109f69f40884e696adc1`.

The following are **seen static facts**, not original runtime observations:

| Anchor | Observation | Evidence ID |
| --- | --- | --- |
| `0x14002b318`, trace `MTAction::Run` | `_FILENAME`/`_ALL` route through the full-name or stem helpers and virtual Format slot `0x48`. | `ev_1a9c49e182165e5fbe86c015a6a71064b3e6248ad5d0187cc56848126e6ec6bc` |
| `0x140931de8` / `0x140931d98` | Replace/Regexp tables share Run at slot `0x40`; their Format entries at `0x48` are `0x14002f2c4` and `0x140030428`. | `ev_e5bc908b69dae79aeb932a65cdbaededa8150a9ac6bd04f452deb29059d78ecf`, `ev_ba05f14b8a8df1d4e83f43f37f8eb1724cfa50143f7c12b8fddc599b448a5273` |
| `0x14002b0a4` / `0x14002b194` | Both filename helpers call the safe-name route after formatting; the stem route appends the extension first. | `ev_4502c45bdded467f7127b6eef6cebd53a51696614d738a84e6a623627169f937`, `ev_7e485c4e5b3b889ba29b4e3be2a810ccf2889befdc82310d770336402210d496` |
| `0x1402a30f0` | Path/name splitting precedes `SFile::RemoveBadCharacters`, supplied with an empty replacement. | `ev_88a151fb172e495c39df7536ee9dc10633efbb9c993d54b4191e677745f0b4c7` |
| `0x140931240` | The replacement literal is NUL/empty, not underscore. | `ev_3e4c6b2cbfbae1ae2e21639823b0c7542d923c31992a964acf4d70eeb3e894b4` |
| `0x1402a2eb0` | The leaf regex removes colon/slash/star/question/angle/pipe/quote/LF/CR/TAB and has a `Reserved-Windows-Filename` branch. | `ev_3d13431b3405e3af49a49dbb5ffae9adb3bb317961dec8afdd86b987e562461c` |
| `0x1402a1b00` / `0x1402a1f80` | Inspected name/path helpers split on backslash `0x5c`. | `ev_b8a51d3d3591e18c27d4e0d761d6d755bf1411b536df8fb04c4e3a5c9ab406a8`, `ev_45889ace98eae141a7cfadfd2a7bd312104bc84386ada920c371dab52bc8196b` |

It is **inferred** from that path that the ordinary edited-filename route
removes the named characters with an empty substitution. A guessed underscore
policy would not match this trace. Exact original GUI results, runtime regex
flags and namespace/device-name corner cases remain **unknown**.

## Implemented policy and invariants

The [native action host](../../src-tauri/src/action_host.rs) applies cleanup
only to `NameOrigin::Edited` file effects on Windows. Literal Replace keeps
its original extension first; Regex retains its full transformed filename.
Invalid filename characters and ASCII control codes are removed. Ordinary
drive, UNC and extended prefixes stay separate from cleaned components;
namespace fields are not silently renamed. Authored intermediate `.`/`..`
navigation survives, while cleanup cannot create a new dot-navigation or empty
component. Reserved device names use the observed placeholder and keep their
extension. Windows trailing space/dot and superscript device-name protection
follow [Win32 naming rules](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file);
those broader guards are not claimed as exact original runtime equivalence.

Unix edited paths, `_DIRECTORY` effects, template/value-derived name rules,
sequential action/tag-flush timing, source synchronization, no-clobber rename
and journal/history handling remain unchanged. Cleanup-induced collisions must
still refuse the existing destination.

## Focused evidence

Three Rust policy tests first fail with actual unchanged illegal outputs and
missing refusal (`taggroove-l54-edited-filename-red-v4u4zzk8`). The implemented
helper then passes all three, covering 25 explicit vectors, with 592 unrelated
tests filtered out (`taggroove-l54-edited-filename-green-1ychd4i2`). These execute
the production helper on macOS; they are portable Windows-policy tests, not a
Windows filesystem or original-application run. The source binding retains
the exact compiled action-host file. Rebasing onto build-tool repair `f0bc8df`
does not change that Rust file.

Independent static source review has no concrete findings. The new owned error
is included in the backend snapshot and English/Dutch/German catalogs;
33 focused catalog/extraction tests pass (`taggroove-l54-error-catalogs-x_l39uci`).

The existing postmerge [native suite](../../tests/e2e/rename-fixes.js) gains a
managed-command case for literal Replace, Regex reserved names, Undo and a
cleanup-induced collision. On Unix it instead checks retained literal filename
characters. It checks actual file names and metadata through the instrumented
app's existing commands, not an Actions-dialog interaction. Syntax is checked;
this added case has **not been executed locally**. It supplies no native result
until a matching postmerge CI run executes and passes it.

Scoped backend Clippy with warnings denied passes
(`taggroove-l54-clippy-r0xefhas`); the strict guide passes
(`taggroove-l54-guide-0eq0_xwq`). Eight publication-boundary tests and writing-plan
validation pass. All 614 local links/anchors resolve, and the intake, active
evidence, priority Markdown/CSV and OS disposition tables agree on 48 active
IDs, 63 represented scopes and the unchanged 14 qualifications. Final source
and documentation/native-case reviews have no concrete findings.

The compiled `src-tauri/src/action_host.rs` SHA-256 remains
`fe1f4f10c89cc2f7fa72ef2759bac8273520887243c71d2577395a7477c786a1`.
The original GREEN binding matches the current file after the metadata-only
commit-message change and build-tool rebase; no application behavior was
re-pinned from an older executable.

## Engineer handover

Use a Windows desktop with the pinned original and a source/executable-bound
TagGroove build. Repeat the same owned fixtures through original Replace and
Regex actions: punctuation removal, extension case/doubling, device names,
empty results, paths, collisions and Undo. Include lowercase device names,
multiple extensions, drive-relative/UNC/extended prefixes and control codes.
Record actual preview/apply/readback and byte preservation; source-derived
vectors alone do not close this qualification. Original Windows execution
remains owner-deferred. Native keyboard/accessibility and installed filesystem
checks remain under L78/L79 and the
[OS procedures](../handover/PARITY-VERIFICATION-PROCEDURES.md).

No local app build or native UI replay was run for this follow-up. Hosted
premerge validation, exact-head GitHub Codex approval, merge, postmerge native
integration and publication remain distinct evidence layers.
