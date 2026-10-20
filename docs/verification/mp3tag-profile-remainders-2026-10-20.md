# Advanced settings and custom genres: L57b extension

This is unmerged work on `codex/refresh-file-identity`. It extends the
[binary settings](mp3tag-binary-settings-migration-2026-10-20.md) and
[field-value](mp3tag-field-values-migration-2026-10-20.md) adapters.
The later [export settings slice](mp3tag-export-settings-migration-2026-10-20.md)
adds six more records. **L57b remains open:** numbering
memory still has concrete unimplemented mappings. A whitelist count is not
proof that all original profile settings are covered.

## Confirmed native records

Static evidence uses Windows Mp3tag 3.36.1 x64, executable SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
The original was not executed. The existing envelope remains 16 raw identifier
bytes, LE u32 payload length, then bounded payload; no UUID conversion.

| Added record / raw identifier | Mapping |
| --- | --- |
| `243cae0bb6d7804b932f5ea2f2d2a55d` | Advanced POPULARIMETER display |
| `ac740b3914557e479e4b32dbbbacaab0` | List chapters |
| `e4f596ec68aba54da0bf8bff3a489238` | Matroska video mapping |
| `40b8c799a9b6ac4080064606a8c10200` | Single MP4 atom |
| `a8c68011b761684596bd8f68db51d329` | Reuse MP4 atoms |
| `8b86f06a405be14991918e4fc1cf13d8` | Reuse ID3v2 frames |
| `a5a066496e5a1d409b6a874bb814b65b` | GENRE-specific prefer-existing choice; preserve the global fallback |
| `10ad9a5de1a201419efb4f510bd262f4` | Grid u32: 0 off, 1 alternate, 2 lines |
| `8eec759f9a170443abf0e47af87c1079` | Comment-language UTF-8 string |
| `0c7f0a198af644488be02a0d19631585` | Incoming-file restriction UTF-8 string |
| `375e4b6e9c45af49a77caac696ced2b6` | Folder-separator replacement UTF-8 string, including empty |

The six advanced checkboxes bind through DDX `0x1401df630`, Init
`0x1401df6fc` and Update `0x1401dff0c`; member/control pairs are
36c/868, 368/85d, 370/85e, 374/95e, 378/967 and 37c/95f.
The GENRE checkbox binds through DDX `0x1401eb275`, Init `0x1401eb34f`
and Update `0x1401ebd52`, member 8ac/control 5c6. These use the existing
one-byte boolean reader/writer `0x140075640`/`0x140075670`, nonzero true.
No unproved legacy text aliases were added.

Grid DDX `0x14013ebed` binds control 5bb; `0x14013f1b0`/`0x14013f63b`
read/save the u32. Consumers `0x1400c50ed` and `0x1400c8410` distinguish
native grid value 2 and alternating-row value 1. Comment-language Init/Update
`0x1401df731`/`0x1401e00be` bind control 726. Files Init/Update
`0x1401e247d`/`0x1401e2b8e` and `0x1401e2645`/`0x1401e2bed`
bind restriction 985 and separator 984. Their UTF-8 serializer pair is
`0x1403037a8`/`0x140303748`, also used by the already imported playlist text.

The importer validates the final value after duplicate collection. Unknown
grid values, non-three-letter comment languages and target-invalid restriction
or separator strings are reported and skipped, preserving the corresponding
current choice. An invalid final record cannot revive an earlier valid one.
Empty restriction/separator strings remain meaningful. Text previews escape
HTML and preserve authored values; intentional empty values display a
translated `(empty)` label.

Coverage at this slice is **53 native records** producing at most **64 preview
options**: 39 booleans, 17 packed tag switches, three choices and five text
values. This is an implementation inventory, not total native coverage.

## genres.ini codec and precedence

Genre-dialog Init `0x1401eb2ac` and Update `0x1401ebb30` load/save the named
file. The flat parser `0x14029dd28` removes CR, splits LF and takes key/value
tokens; the dialog uses the keys as names. It does not establish ordinary INI
section/comment semantics. Writer `0x14029e42c` escapes names and emits
`key=value` with CRLF; genre values are empty. Decoder `0x1402b8b90` handles
backslash, control escapes, four-digit `u`, two-digit `x`, four-digit braced
`x`, and unknown-escape passthrough characters. Its helper extents were
checked against PE unwind metadata:

| Helper | Disassembly SHA-256 |
| --- | --- |
| `0x1402b873c` | `afb75a5aa3070f1c304c9b9112f3dc405a32c315c80319130fa6ab2b0379b6a6` |
| `0x1402b889c` | `5b14ed3c526720897b6cfdae3db28ea3bb8c3e171eb7a6766fcc2b6052b6bd32` |
| `0x1402b7a00` | `0e3ec4ca7630dc9247c1fd4f3108a4839b9e65cfc88dfd1648aec5ca2d97d2a0` |

[The adapter](../../src-tauri/src/mp3tag_field_values.rs) decodes UTF-16 escape
units strictly, preserves valid Unicode/spaces and file order, and deduplicates
exact names. It rejects unsupported controls, malformed/unpaired escapes and
overlong names with line reasons, without copying raw invalid names into
diagnostics. Details are bounded to 50 plus an omitted-count summary. A list
over 5,000 unique names refuses instead of truncating. Invalid-only files keep
the current list; selecting an empty file explicitly clears it with a warning.

[Staged application](../../src-tauri/src/profile_migration.rs) uses fixed order:
legacy panel values, genre list, native settings, then modern per-field values.
Only matching lists/preferences are replaced; unrelated lists and the global
preference remain. The modern file wins only for entries it supplies. At most
one genres file is selectable, independently of one modern field-values file.
This deterministic TagGroove coexistence policy is disclosed in the preview
and [migration guide](../user-guide/getting-started/coming-from-mp3tag.md);
an original multi-file runtime precedence comparison is not claimed.

## Evidence and remaining work

The expanded boolean fixture isolates all 39 mappings at raw 0, 1 and 255 and
in one-hot profiles: 1,638 mapped-value checks. The binary cohort passed 13
tests; the profile cohort passed 32 with one opt-in worker ignored. The latter
includes a 17-option binary import with exact restore/source preservation,
genre escape/list import, explicit empty clearing, invalid-only refusal,
reversed selection order and retained unrelated fields/global preference.
Four pure field-value tests cover diagnostics, list refusal and bounded skips.
Frontend tests reproduce and verify empty-value preview and genre-file
selection. Remaining lint/catalog/type/guide checks are in the checkpoint.

Read-only audits additionally established export partition/default/identity
contracts and five direct numbering booleans. They also found required paired
numbering-field handling and a potential stale export-library overwrite during
combined settings/MTE import. The later export slice fixes that overwrite;
numbering mappings remain required. LastCodepage is
a Windows chooser row index, not an importable codepage or LCID; padding uses
dynamic widths, with L66 scope/qualification retained. Toolbar, tool, networking,
foreign workspace, credential and native acceptance boundaries remain explicit.
Original-generated populated profiles and original/native restart/restore
acceptance remain under C12/C27/L78/L79. No original application or local native
UI replay was run for this extension.
