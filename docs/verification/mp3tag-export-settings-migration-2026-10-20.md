# Mp3tag export settings migration — 2026-10-20

This working-branch L57b slice imports six export records and preserves disabled
partition expressions. The later [numbering-memory slice](mp3tag-numbering-settings-migration-2026-10-20.md)
adds nine records; L57b remains open pending the remaining-choice disposition review. This is local
implementation evidence; final batch PR review, Linux CI, merge and original /
native acceptance are separate requirements.

## Source contract

Static inspection of Windows Mp3tag 3.36.1, SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
The executable was not run. These are raw identifier bytes, without UUID
byte-order conversion. The existing record envelope is 16 identifier bytes,
a little-endian u32 payload length and that many payload bytes. Later records
win; short known payloads or malformed known UTF-8 refuse that settings file.
Strings stop at the first NUL, with the existing 4 MB decoded-text bound.

| Native record | Identifier | Target |
| --- | --- | --- |
| ExpAppend | `a91684289cba6f498fd857faeaa3749c` | Saved export append switch |
| ExpMultiple | `9e917d2becf8af4793061746fdf7bad5` | Saved partition enabled state |
| Partition expression; no proven textual alias | `e31f66267bd2a34baffa13f42d34fd95` | Active or dormant partition expression |
| ExpWriteBOM | `759de112c0072d48b5650c2fbec586a8` | Effective Options → Export BOM switch and library fallback |
| ExpKonf | `536d231fc389234bafa84663f0986a0c` | Final selected template identity |
| ExpFilename | `d1c3c5a4c95780489c008de588e2eeea` | Filename bound to that source template |

Boolean records consume the first byte, nonzero meaning true; the envelope
skips extension bytes. ExpWriteBOM consumes the first four bytes as u32, with
the export consumer testing the whole value for nonzero. This does not invent
a BOM enum or claim identical Options-dialog normalization for unusual values.
Native export consumer `0x1400bd164`, emitter `0x1402bee04`; Options DDX
`0x1401e4884`. Export dialog DDX `0x1400aec94` / save `0x1400affec` bind append,
multiple and expression separately. Constructor `0x140002ec8` installs the
explicit `%_folderpath%` expression; save trims and replaces blank at
`0x1400b0205`. Import applies that documented normalization to authored incoming
expressions. This is not a runtime claim about manually edited original CFGs.

Native OnFileExport `0x140176de8..0x140176e7b` constructs `export/` + ExpKonf +
`.mte`. Target binding requires an exact source template stem from the explicitly
selected source files. Case-insensitive target name collisions do not establish
native case folding. Keep/replace resolves the final target UUID first, then
settings overlay that final in-memory library and save once. This avoids stale
library saves overwriting imported preferences. A kept matching source candidate
binds to its retained target identity; an unresolved/missing/unselected source
candidate preserves both current active configuration and filename with a warning.

## Target behavior and compatibility

Absent source settings keep current choices. An explicitly blank export filename
selects the template's own filename. Invalid filename expressions and foreign
Windows filenames on non-Windows platforms are skipped with fixed diagnostics;
source filename text is not echoed into those diagnostics. Import runs no export.

`exports.json.preferences.partition_memory` is optional and omitted when absent,
so old libraries remain readable. It retains the expression while `partition`
is null. The export dialog saves and restores it; only enabled partition text is
evaluated. Disabled invalid draft text must not prevent saving or ordinary export.
An absent expression uses the current active/dormant expression if available;
otherwise enabling uses the proven native `%_folderpath%` default. An explicitly
invalid incoming expression skips both its expression and switch, preserving
current partition settings. Numbering, LastCodepage, padding and unsupported
choices remain separate; no allowlist-size closure is claimed.

At this slice, the earlier 53-record implementation became 59 records and at most 70 preview
options: 41 byte booleans, 17 packed tag switches, three choices, eight text
values and one u32-derived BOM switch. The independently pinned 39-boolean
fixture remains its existing 1,638 checks; the six new export records have
separate integration and codec cases.

## Verification

RED evidence reproduced rejection of export-only settings and loss of a disabled
expression in the dialog. GREEN: six focused Rust export cases, including five
profile scenarios (each with applicable internal policy/ordering variants) and
one codec/bounds case; nine export frontend cases. Profile regressions: 37 passed,
one restart-worker entry ignored; frontend migration/configuration/catalog/export
cohort: 86 passed. Counts overlap and are not additive.

Review also found dormant invalid text was being compiled even when partitioning
was disabled. A focused real save/load regression was added before its correction.
The corrected export-library cohort passed 31 tests, including dormant draft
save/reopen and unchanged-disk refusal for an active invalid expression. The
updated frontend cohort passed 86 tests. TypeScript, scoped library Clippy,
test-target Clippy (after updating an earlier fixture to `as_chunks::<2>()`),
catalog extraction, strict guide build, format/diff checks and all five ledger
views passed. Read-only review found no other source issues; final re-review
is recorded in the task checkpoint. No GitHub review or CI result is claimed.

Coverage includes new/kept/replaced template identities, both selection orders,
partial imports, disabled expressions, whitespace default, duplicate last-wins,
first-NUL text, extension-byte skipping, whole-u32 BOM, effective-page precedence,
missing/unselected/case-mismatched template identities, Windows drive/UNC paths,
invalid expressions, exact managed-file restore and unchanged source profile.
No application/native UI replay, original Windows execution, provider request or
release delivery is inferred from these checks.
