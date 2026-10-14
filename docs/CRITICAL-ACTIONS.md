# Critical acceptance gate: Actions and Action Groups

The requested end state includes every action type and the complete reusable group workflow. A catalog entry, an emitted effect, or a passing pure test does not establish native execution or original-application parity.

Authoritative surface: [Actions documentation](https://docs.mp3tag.de/actions/), supplied Mp3tag 3.36.1 `data/actions/*.mta`, and the executable's dialogs. Independently authored implementation: `crates/tag-core/src/actions.rs`. Proprietary action files remain outside the distributable application.

## Every documented action type

`Pure tested` means deterministic field transformations have regression tests. `Native pending` means a typed effect exists, but the adapter and real file/cover/export round trip are still required. Every row also requires differential verification against Mp3tag, configuration persistence, and a usable Quick Action/group editor.

| Action and authoritative source | Current implementation | Remaining acceptance evidence |
|---|---|---|
| [Adjust Cover](https://docs.mp3tag.de/actions/adjust-cover/) | Typed format/quality/resize effect | [ ] Original/JPEG/PNG; quality; min/max size; proportional/square/center crop; never upscale under maximum policy; image/native round trips |
| [Case Conversion](https://docs.mp3tag.de/actions/case-conversion/) | Pure tested: lower/mixed/sentence/upper and configured word separators; typed filename/directory effects | [ ] All modes/options; `_TAG`, `_ALL`, `_FILENAME`, `_DIRECTORY`, `_FIELDNAME`; APE/Vorbis field-name writes; Unicode comparison |
| [Convert Codepage](https://docs.mp3tag.de/actions/convert-codepage/) | Pure tested known Windows-1251/UTF-8 legacy bytes; numeric codepage aliases | [ ] Original codepage chooser inventory; every listed encoding; source-byte reconstruction and unchanged valid-Unicode behavior; missing OEM/UTF-7 and locale-specific codecs |
| [Export](https://docs.mp3tag.de/actions/export/) | Typed batch effect, delayed until group end | [ ] Export configuration resolution; filename/encoding/BOM; append; partitions; loops/variables; selected-set rendering; real output |
| [Export Cover to File](https://docs.mp3tag.de/actions/export-cover/) | Typed filename/type/duplicates effect | [ ] Absolute/relative paths; MIME-derived extension; only selected picture type; duplicate policy; multi-cover naming and collisions |
| [Format Value](https://docs.mp3tag.de/actions/format/) | Pure tested field evaluation; filename/directory effects | [ ] Native renames and later-action context; empty results; complete scripting; all tag formats |
| [Generate Playlists](https://docs.mp3tag.de/actions/generate-playlists/) | Typed batch partition/filename effect, delayed until group end | [ ] Multiple playlists; partition evaluation; table order; M3U/M3U8 and playlist settings; actual files |
| [Guess Values](https://docs.mp3tag.de/actions/guess/) | Pure tested complete raw-source matching, repeated fields, dummy capture, source/target overlap | [ ] Native filename/directory targets; all guessing pattern corner cases; exact empty-capture semantics |
| [Import Cover from File](https://docs.mp3tag.de/actions/import-cover/) | Typed evaluated filename effect | [ ] Relative/absolute image lookup; selected-file context; cover ordering/type; missing/invalid images; native round trip |
| [Import Text File](https://docs.mp3tag.de/actions/import-text/) | Typed field/filename effect; host sequencing tested | [ ] Actual text loading; encodings/BOM/newlines; relative paths; later actions see imported content; native writes |
| [Merge Duplicate Fields](https://docs.mp3tag.de/actions/merge-duplicate-fields/) | Pure tested ordered merge and literal separator | [ ] `_TAG`; format-specific duplicate preservation; separators including empty/newline; native round trip |
| [Remove Duplicate Fields](https://docs.mp3tag.de/actions/remove-duplicate-fields/) | Pure tested first-only and same-content deduplication | [ ] `_ALL`; case/empty-value identity rules; native multivalue round trip |
| [Remove Fields](https://docs.mp3tag.de/actions/remove-fields/) | Pure field removal; one cover-removal effect for COVER/PICTURE aliases | [ ] Semicolon editor; case-insensitive names; actual cover removal; unsupported/unknown field preservation |
| [Remove Fields Except](https://docs.mp3tag.de/actions/remove-fields-except/) | Pure tested keep-list and explicit cover-preservation policy | [ ] Complete format-specific removal; COVER/PICTURE; hidden/private metadata policy; native round trip |
| [Replace](https://docs.mp3tag.de/actions/replace/) | Pure tested literal replacement, case flag, documented whole-word punctuation | [ ] Full selector/native rename behavior; Unicode case/boundaries; empty replacement; original whole-word comparison |
| [Replace with Regular Expression](https://docs.mp3tag.de/actions/replace-regexp/) | Shared native Boost Perl engine; case flag; filename includes extension; lookaround/backreference/case-replacement tests | [ ] Original differential coverage; `_TAG`, `_ALL`, directory; native rename |
| [Set Cover Properties](https://docs.mp3tag.de/actions/set-cover-properties/) | Typed keep/set/remove-description and picture-type filter effect | [ ] All cover types; keep vs empty; descriptions; selected-type restriction; native round trips |
| [Split Field by Separator](https://docs.mp3tag.de/actions/split-field/) | Pure tested literal split with automatic trimming | [ ] `_TAG`; repeated/existing values; empty fragments; format-specific native multivalue round trips |

The codepage number/name association uses [Microsoft's identifier table](https://learn.microsoft.com/en-us/windows/win32/intl/code-page-identifiers). That table does not prove Mp3tag's chooser inventory or conversion behavior.

## Group workflow acceptance

The [official group workflow](https://docs.mp3tag.de/actions/) requires all items below. Library operations and runner behavior exist; the complete native editor/menu/persistence and original comparison remain pending.

- [ ] Quick Actions: every type, immediate write, Alt+Shift+6.
- [ ] Groups: Alt+6; apply checked groups to the actual selected files; immediate writes; preserve list order.
- [ ] Create, duplicate, delete, rename; F2/delayed rename and F4/double-click edit.
- [ ] Add, edit, duplicate, delete and reorder constituent actions.
- [ ] Group/action up/down movement; Alt movement to extremes.
- [ ] Checkboxes enable/disable groups; excluded groups perform no work.
- [ ] Selection presets save and restore checked states independently of action contents.
- [ ] Persistent group/preset configuration survives restart and import/export.
- [ ] `#` submenu organization; `&` accelerators through Alt+A and submenu/item characters.
- [ ] File-list order controls set-of-files output; Export/Generate Playlists execute after other actions in their group.
- [ ] Per-file failures remain visible; native effects cannot be silently ignored; no unknown imported action runs partially.
- [ ] Keyboard, mouse and touch workflows; mobile storage and all three desktop platforms.

## `.mta` evidence registry

Supplied files contain `[#N]` sections and settings `T`, `F`, `1`, `2`, `3`:

| Evidence | Interpretation supported in importer | Proof boundary |
|---|---|---|
| `Case conversion.mta`, `Schreibweise aendern.mta`: T=1, mode 1, separators `(-_` | Case Conversion, Mixed mode, explicit separators | Static sample supports schema; verify mode and character behavior in original runtime |
| `Standard.mta`, `CD-R.mta`: T=2, original/replacement strings, flags 0 | Replace; both checkbox options disabled | Original runtime comparison still required |
| `CD-R.mta` escaped characters such as `\xdf` | Decode observed hexadecimal character escapes | UTF-16/BOM and Unicode escape handling tested; other escape forms need artifact/runtime evidence |
| Numeric IDs and options absent from supplied files | Preserve `UnresolvedMta` records, report issues, disable imported group | Build authoritative registry from original dialogs/config output; do not infer IDs from catalog position |

Additional original-config evidence on the official forum extends the registry:

- [Explicit numeric registry and Replace checkbox packing](https://community.mp3tag.de/t/testing-mp3tag-behaviour/42469/22): IDs 1, 2, 4–16; `3=whole_word|case_sensitive` and legacy zero. The importer recognizes all four documented checkbox combinations.
- [Format/Guess/Remove Except records](https://community.mp3tag.de/t/remove-tag-as-an-action/11138?page=2), [Guess Values record](https://community.mp3tag.de/t/using-tag-in-actions-combining-actions-for-different-tags/42419/7), [merge/split/format/remove records](https://community.mp3tag.de/t/albumartistsort-and-artistsort-switch-first-and-last-names/55992): executable schemas for IDs 5, 7, 8, 9, 10 and 16.
- IDs with verified names but unresolved property/options schemas remain `UnresolvedMta`. Newer action IDs and Case mode IDs beyond the supplied mode 1 still require original-config/runtime evidence. Community originals support compatibility work; they do not replace a differential run against the supplied release.

The importer preserves setting values and numeric section order, rejects duplicate settings/sections, accepts UTF-8 and BOM-marked UTF-16, and requires an explicit encoding for ambiguous legacy bytes. Unknown modes/flags/IDs remain non-executable even if a group is checked. The existing five-type API has an explicit adapter; it is not the full catalog.

## Execution contract and proof

`ActionState` holds tag fields separately from file/technical context. `apply_action_spec` computes pure field changes plus typed file/image/text effects. `ActionHost` must execute every effect and update imported values or path context before following actions. Its evaluation hook binds actual `EvalContext` settings (columns, environment/app paths, locale and ANSI codepage). `execute_groups` processes enabled groups and routes deferred set-of-files effects with the final group states.

- [ ] Native adapter implements every `ActionEffect` and `BatchActionEffect` variant.
- [ ] No external-effect branch reports success without executing and verifying it.
- [ ] Disposable audio fixtures prove untouched fields, unknown metadata, artwork and audio payload preservation.
- [ ] Group persistence, settings import/export, action UI and shortcuts have actual runtime receipts.
- [ ] Original Windows/Wine differential runs cover all rows/options and imported groups.
- [ ] Read-only 434-FLAC benchmark and repeated grouped runs demonstrate acceptable performance; instrument parsing, transformation, I/O and output phases separately.

Pure regression tests cover transformations, same-field guessing, codepage byte recovery, cover policy, deferred exports, imported-text sequencing, unresolved-import preflight, presets, menu parsing and `.mta` parsing. These tests do not certify native effects or full parity.
