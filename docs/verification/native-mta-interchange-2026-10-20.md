# Native action interchange and explicit older-port migration

Working-branch N05 implementation. Five confirmed codec gaps are repaired;
the approved local schema/matrix work and focused checks/review are complete.
Batch PR review/CI/merge and original/platform qualification remain
separate. This extends the historical [C57 Type 17/18/19 implementation](native-mta-implementation-2026-10-19.md)
without changing its geometry, quality, sentinel or unknown-option guards.

## Source contract

The pinned Windows Mp3tag 3.36.1 executable has SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
Read-only PE factory/vtable, complete reader/dialog and import-binding traces
establish the following decoded layouts. The target was not executed. Tests
are independently authored semantic predictions, not original-generated files.

| Type | Confirmed native representation | Repair |
|---|---|---|
| 1 | `F` field; `1` mode flags 1 mixed, 2 upper, 4 lower, 8 sentence; `2` separators | Replace the old array-index mapping; unsupported/compound flags stay opaque. |
| 11 | `F` field; `1` LCID passed to `GetLocaleInfoW` with `LOCALE_IDEFAULTANSICODEPAGE` | Retain the source locale and project its portable default ANSI encoding. Never interpret an LCID as a raw codepage. |
| 12 | `1` filename; `2` delete-existing flag; optional `3` picture override | Read/write the actual numbered filename/type slots; absent override retains front-cover semantics and empty override means picture type zero. |
| 13 | `1` filename; `2` duplicates flag; `3` enabled/type tuple | Disabled filters remain inactive, with their remembered type retained through unrelated edits. |
| 15 | `F` export configuration; `1` output filename; `2` append; `3` enabled/expression remainder | Disabled partitions stay inactive. Enabled-empty uses `%_folderpath%`; expression-internal pipes are retained. |

Reproduction anchors in that executable: Type 1 callback `0x14002c52c`
tests bits 1/2/4/8, with upper/lower import bindings; Type 11 callback
`0x14002cadc` loads key 1 and calls `GetLocaleInfoW` through IAT
`0x1408a6d38` with selector `0x1004`. Type 12 helper/run are
`0x14003285c`/`0x140032ab4`; Type 13 Run/dialog are
`0x14003437c`/`0x140033f30`; Type 15 dialog/worker are
`0x140035158`/`0x140035960`. The worker uses the token helper's remainder
mode for the partition expression. Type 12/13 do not read `F` as the filename.
The complete dialog option-key matrix covers all 18 native IDs; this is
bounded layout evidence, not qualification of every value or runtime default.

Microsoft documents the [LCID API](https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-getlocaleinfow)
and [default-codepage lookup](https://devblogs.microsoft.com/oldnewthing/20161007-00/?p=94475).
The portable table is derived from
[.NET runtime commit 60629d1](https://github.com/dotnet/runtime/blob/60629d14374c56f1cb51819049ad1fa529307f8d/src/libraries/System.Private.CoreLib/src/System/Globalization/IcuLocaleData.cs),
source SHA-256 `ab04bf41c61149233ad7bc04279637426e5a04c8f18f6d979d639ad1ff0f8bee`.
[The generator](../../scripts/generate-lcid-ansi.py) reproduces the upstream
LCID lookup's sort partitions and packed name indexes before reading each
numeric ANSI field. It admits all 354 nonzero mappings from 448 authoritative
LCIDs. The 94 zero-ANSI entries, unknown/custom/system sentinels remain
unresolved. It never inverts the 864 numeric rows, where custom IDs repeat.
The table is a frozen default; it does not reproduce Windows locale overrides.
The MIT license is retained in source, common installer and offline notices.

To reproduce, download the exact source URL in
[`actions_lcid.source.json`](../../crates/tag-core/src/actions_lcid.source.json),
then run `python3 scripts/generate-lcid-ansi.py /path/to/IcuLocaleData.cs --check`.
The extractor verifies the source hash, packed-array sizes, unique IDs,
nonzero/zero counts, independent example mappings and sentinel exclusions.

## Origin, saved settings and export

Old port encodings can be byte-identical to native records with different
meanings: Type 1 value 2 formerly meant sentence case; Type 15's former bare
expression can start `0|` or `1|`; Type 12/13's former `F`/type-in-1 layout
cannot safely be inferred from an extra `F`. Neither parse-failed fallback nor
an F-present heuristic selects the old meaning.

Import therefore accepts an explicit origin, independently from byte encoding:
unknown (default), Mp3tag/current corrected format, or older TagGroove. The
actual import dialog passes this choice through the registered single-file
command. The registered bulk/folder command also accepts it. Unknown-origin
collisions remain opaque/disabled; explicit old-origin parsing restores the
former five schemas. Native Type 12/13 strictly reject `F`. Original Mp3tag
profile migration supplies its known native origin and still stages imported
groups disabled. No audio action runs during import.

Existing JSON enum names, codepage strings and optional partition semantics
stay intact. New optional source metadata on four variants is absent by default.
It is bounded, type-checked and re-parsed before export. Changing an unrelated
field preserves inactive source values and absent/empty source distinctions.
Quick memory and the group editor retain this metadata. Explicit older-port
imports export using semantically equivalent corrected layouts, not the former
conflicting numeric meanings.

A newly authored raw codepage has no established reverse locale: many locales
share one encoding. It exports with the explicitly app-only Type 1004 and a
truthful export warning. An unchanged retained imported locale can export as
native Type 11, including after a field-name edit. Changing its codepage uses
1004. Saved JSON `Some("")` for export partition retains its distinct value
through app-only Type 1005, rather than becoming the native folder fallback.
Legacy 1001–1003 support is unchanged. No app-only ID is claimed to work in
Mp3tag, and the exporter report uses the same selector as the actual bytes.

Decoded opaque values/order, escaped controls/backslashes, UTF-8/UTF-16 size
budgets and atomic destination preservation retain C10's contracts. Re-import
with the same origin preserves unresolved records; deliberately choosing a
different origin can resolve an ambiguous record and requires inspection.
No original byte order, source encoding, escape spelling or byte-for-byte file
identity is promised.

## Focused verification and remaining qualification

RED reproduced all five codec mismatches, unsafe guessing of unmarked origin,
dropped Quick source metadata and missing production import-origin controls.
Focused GREEN covers semantic import/export, retained inactive settings after
edits, explicit old-origin migration, old JSON semantics, all 18 native plus
five app-only schemas, unknown numeric/text keys, disabled unresolved groups,
validation refusal and decoded-property equality through UTF-8/UTF-16 export.
The origin dialog is exercised in headless DOM tests; this is not native UI
acceptance. Generator regeneration and `--check` pass against the immutable
upstream source, separately from sampled codec examples.

Current results: ten new interchange cases, seven prior native-schema cases,
nine opaque-preservation cases and 46 existing core action cases pass. The
34 affected action UI/memory/installer-notice cases, 33 catalog/translation cases
and TypeScript pass. Registered single/folder command-origin, known-native
profile import and the existing populated-profile import/restart/restore cases
pass. Format/extraction, five-view ledger, diff, warnings-denied library/test
Clippy and strict guide checks pass. Final read-only source/generator/docs/locale/
ledger review has no actionable findings. Cargo reported an unrelated global
registry-cache cleanup permission warning. These cohorts are not broad UI sweeps.

Original-generated every-option files, original import/edit/re-export/readback,
malformed numeric-prefix/compound/sentinel grammar, Windows locale overrides,
native editor/export behavior, image quality/rounding and playlist global
settings remain qualification under C57/L29/L78/L79. No original Windows
execution, application/native UI replay or live-provider run was performed.
Passing source/codec checks does not replace the exact-final-head GitHub Codex
code review, required Linux CI, merge or release/native evidence.
