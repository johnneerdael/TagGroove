# Confirmed portable binary settings migration

This working-branch slice advances L57b; it does not close the entire profile
migration scope. The companion [field-value migration receipt](mp3tag-field-values-migration-2026-10-20.md)
covers the independently implemented text files.

The later [advanced settings and genre extension](mp3tag-profile-remainders-2026-10-20.md)
raised that slice to 53 native records: 39 boolean records, packed tag
switches, three choices and five text options. Earlier test counts below retain
their original slice boundaries; neither count closes L57b.

## Static source and record contract

The subject is the shipped Windows Mp3tag 3.36.1 x64 executable, SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
The executable was inspected statically; it was not executed.

The `mp3tag.cfg` load/save callers at `0x140073300` and `0x1400733d8`
reach the record reader `0x14030339c` and writer `0x1403035c0`.
Each record contains 16 raw identifier bytes, a little-endian unsigned 32-bit
payload length and that many payload bytes. Identifiers undergo no UUID byte
order conversion. The boolean reader `0x140075640` consumes one byte; writer
`0x140075670` emits one byte. Nonzero means true. The envelope skips remaining
payload bytes. Objects contain the identifier at offsets `+8` and `+0x28`,
with the value at `+0x40`; the legacy setter `0x14003bf14` writes that value.
The legacy-key bindings and object addresses are recorded in the
[authored mapping fixture](../../tests/fixtures/mp3tag-settings-booleans.json).

| Native key | Raw identifier | TagGroove option |
| --- | --- | --- |
| AutoSave | `8840da8e2956d74da1853b63f6df9037` | Save on arrow-key/single-click selection |
| NoImageFromFolder | `ee0051ba892b9941bce4471869a8f2d0` | Do not show folder image |
| KeepFileTime | `c43475eedd175e42b0d89661bd66f42d` | Preserve modified time |
| JumpOnEdit | `53fe21e50af824458cf74de7edade298` | Select next after inline edit |
| UseAutoComplete | `a17772f7656a2b4fb710657ed41cb01d` | Selection-box completion |
| TitleShowFolderPath | `6317969e3cb5424781aab010efedb4a2` | Directory in title |
| Recurse | `fbb012dd766b664493dff2ac5f006238` | Read subfolders |
| HideStandardGenres | `a635489304121948aceaf4ad079f3fa1` | User genres only |
| HideID3Bar | `a0f6c4c7dcd9b44fb7d973852061f2cb` | Show panel, inverted |
| HideFilterBar | `86fd6bff815ed0488b445b271955c35c` | Show filter, inverted |
| TrayIconShow | `07a26324a1f7d84782538bb89e2a57e8` | Notification icon |
| TrayIconMinimize | `d5c6e9f135cd614aa8f3d9f513f774f1` | Minimize to icon |
| PlCreate | `b2efeb13fa6c2c44b8386c2182ac5965` | Automatic playlist creation |
| PlExtended | `04d7ba2f4f3bfa4d8b0c1ccf2ab1c361` | Extended playlist information |
| PlRel | `59fc84e13a7c8044b3192709f11d754d` | Relative playlist entries |
| PlShowDlg | `f25e55356aa23840bb4e104ab58ce1f3` | Confirm playlist filename |
| PlNoFolders | `b0b41d103f16944cb184a331ae6136a9` | Do not create playlist directories |
| ExpShowDlg | `97a81ca5caca824ab21be214338c735e` | Show export selection dialog |
| SORTNATURAL language key | `3ec5faaa30c28a4b8ea2c470e1f7a276` | Natural sorting |
| SELECTFILES language key | `de10ea102047ae448c2c53c4ba58d55e` | Select added files |
| KEEPMOVEDFILES language key | `8d7feb69eab69f4883076efb11f4b20c` | Keep moved files |

The three final General switches have bindings through the same shipped
English language file described below. `_C_OPT_GEN_CHK_SORTNATURAL`,
`_C_OPT_GEN_CHK_SELECTFILES` and `_C_OPT_GEN_CHK_KEEPMOVEDFILES` bind
language offsets `0xba0`, `0xb90` and `0xb18` through `0x14015cbbc`.
General Init `0x1401e3048` assigns these captions to controls `0x5bc`,
`0x5bd` and `0x5be` using control `0x63a` minus `0x7e`, `0x7d` and
`0x7c`. DDX `0x1401e2f6c` binds fields `0x164`, `0x160` and `0x15c`;
Update `0x1401e3ab4` writes them to the same boolean objects. The current
natural-sort choice has its own boolean record; the older `ListSortStyle`
u32 is not guessed to be that choice. Exact original/native collation and
selection-trigger outcomes remain separate qualification.

The five playlist flag values are loaded by
`CMTOptPlaylistDlg::Init` (`0x1401f3b90`, shipped debug string at
`0x14095df28`). `CMTMainFrame::CreatePlaylist` (`0x1401898c0`)
and `CMTMainFrame::GeneratePlaylistsThread` (`0x1401280d4`) read the
relative/extended flags. `CMTMainFrame::OnKonTTF` tests the automatic flag
at `0x14017f118` before entering the playlist command. The selected-only
scope flag is separate and remains unmapped. `CMTMainFrame::OnFileExport`
tests `ExpShowDlg` at `0x140176d64` before its selection-dialog route at
`0x140176d99`. The playlist filename/format mappings below supplement these
flags. Other companion templates and the last-used export configuration are
not inferred, and the guide calls out reviewing those values.

### Playlist text values

| Native key | Raw identifier | Serialized object / value | Destination |
| --- | --- | --- | --- |
| PlName | `7eaf6b64ce85a54196043a3c8cfb1d2e` | `0x140ae8370` / `0x140ae83b0` | Playlist filename |
| PlExtendedFormat | `aa0ec86bea2e7a40a7350a520bcf4a46` | `0x140ae83e0` / `0x140ae8420` | Extended-info format |

The static initializers at `0x140003934` and `0x140003ab8` pass
identifier bytes from `0x14091fc30` and `0x14091fbd0` to constructor
`0x140072f70`. That constructor registers both serializer interfaces and
stores the identifiers at `+8` and `+0x28`; the string value is at `+0x40`.
The legacy `PlName` and `PlExtendedFormat` key references at `0x140250d5c`
and `0x140250c18` reach these same objects. Defaults are `mp3tag.m3u`
and `%artist% - %title%`.

Reader `0x1403037a8` reads record-bounded bytes through `0x1402bb514`
and its string setter stops at the first NUL. Writer `0x140303748` obtains
the byte pointer and length from `0x140025f0c` and `0x140025f14`, omitting
the terminator. Legacy UTF-16 conversion uses `0x14028ac00` to count UTF-8
bytes, including surrogate pairs, and `0x14028a930` emits the UTF-8 leading
and continuation bytes. A bounded disassembly window through its final return
at `0x14028aacd` was used because the unwind table splits its prologue.
These identifiers and codec facts are in the
[authored text fixture](../../tests/fixtures/mp3tag-settings-text.json).

The importer preserves exact supported text, including whitespace and Unicode,
without evaluating expressions. It bounds decoded values to 4 MB, refuses
invalid UTF-8, and applies only the last occurrence. The preview marks authored
text as verbatim to prevent translation or HTML interpretation. Empty filenames
are skipped; drive-prefixed or backslash-containing filenames are also skipped
off Windows with a warning, keeping the current filename while allowing other
supported settings. The importer does not guess cross-platform path conversion.

### Extended Tags save mode

The `TagExtMode` object at `0x140ac35d0` has raw identifier
`16ba6ee64495f544a1f46ad5ed57b876` and a 32-bit value at `0x140ac3610`.
The legacy-key load at `0x140250243` reaches setter `0x14001d030`
through the object reference at `0x14025027f`. Reader `0x1400756f4`
and writer `0x140075724` consume/emit four little-endian bytes.

The consumer at `0x14026ac2c` identifies itself through the shipped debug
string `CMTViewTagsDlg::SaveMetaData`. Comparisons at `0x14026ac6d`
and `0x14026ac72` select Ignore for 2 and Save for 1. Other values
prompt; only answer 6 takes the Save branch. That branch calls
`0x14017acf8`, identified by its debug string as
`CMTMainFrame::HandleFileSaveWithWait`, at `0x14026ad7c`.
The importer therefore maps 1 to `save`, 2 to `ignore`, and every other
u32 to `ask`, independently of the general AutoSave flag. The preview
shows the named choice instead of treating this setting as an On/Off switch.

The focused mode regression covers 0, 1, 2, 3 and `u32::MAX`, short payload
refusals, extension bytes and last-record precedence. This establishes the
static mapping and local importer behavior; original UI execution is deferred.

### Message preferences

All 11 Messages switches have independent native bindings. The constructor
`0x1401e9fa4` loads their values into dialog fields; DDX `0x1401ea1a8`
connects those fields to the controls, Init `0x1401ea3b8` assigns captions,
and Update `0x1401eac48` writes the same booleans back. The language loader
`0x14015cbbc` connects `_C_OPT_GEN_CHK_MSG_*` keys to language-object offsets
`0xb20` through `0xb70`, which Init reads for those captions.

The English language file from the same extraction has SHA-256
`70c20d29a2d97021248420a6302b1880946997ee9a6a15a9eb33a81328ef5da8`.
Its named captions establish that the delete notification and removal
confirmation are separate choices. The fixture retains each language key,
offset, control, dialog field and raw identifier. Four rows use language keys
as their native identifier labels because no legacy registry alias is claimed.

| Native key or language suffix | Raw record identifier | TagGroove Messages choice |
| --- | --- | --- |
| MsgSave / SAVE | `a0d12c9bfeca104a9e28c4d677eeed5b` | after_save |
| MsgRemove / DEL | `50ee520115df554ca05d5a2318e7dbb1` | after_delete |
| MsgChange / CHANGE | `c926caafa65b39429d863d45454e9457` | after_change |
| MsgCut / CUT | `3e5445fa1d3cc94ea144ac8bcc08cbe6` | confirm_cut_tags |
| MsgPaste / PASTE | `a5fea160d8e1d543b742a86a86a8f2d5` | confirm_paste_tags |
| MsgExport / EXPORT | `f943c466dfced441ab7d90819f99baaa` | confirm_export |
| MsgDeleteFile / DELETEFILE | `c2b1b77b33dd5e42b8e2bc0a90d00586` | confirm_delete_files |
| ACTIONWARNINGS | `215db77b5ec4934c9d28593f17c07afd` | action_warnings |
| UNSAVED | `0175173ea37ff746bf47382a1c7c3f76` | unsaved_changes |
| TSIMPORTCOVER | `fc0a600b58c13744ab155cb882f9d901` | confirm_import_covers |
| REMOVETAGS | `6ffacd136f47034d95660ba5e0469de8` | confirm_remove_tags |

These mappings preserve the native option states, using the existing
TagGroove confirmation/notification routes. They do not establish every
original handler's timing. General save-on-selection and the artwork
replacement choice remain separate. The guide explains the effect of disabling
unsaved-change questions and retaining the current no-prompt artwork choice.

### Packed tag read/write/remove and encoding settings

Six native u32 records carry 17 switches; the version/encoding record carries
one named choice. The
[authored tag-setting fixture](../../tests/fixtures/mp3tag-settings-tags.json)
binds every bit to its native object, DDX field/control and destination.

| Key | Identifier | Mapped bit meanings |
| --- | --- | --- |
| ID3Read | `10d6c497feac8340a1d9b78566f9aede` | 1 ID3v1, 2 ID3v2, 8 APE |
| ID3Write | `f4a149060db0964aa32d016776713477` | 1 ID3v1, 2 ID3v2, 8 APEv2; only-if-needed checkbox requires `(value & 7) == 7` |
| ID3Remove | `33172482169bb947865a0d6d2f644446` | 1 ID3v1, 2 ID3v2, 8 APE |
| APERead | `ac7785e8c51b644fa377abc80a954b8b` | 16 ID3v1, 32 APE |
| APEWrite | `1d46a2fc85e7654794a5490bdc3e9eec` | 16 ID3v1, 32 APEv2 |
| APERemove | `9e2950e85b1dbd45b4009b2ba739a744` | 16 ID3v1, 32 APE, 64 ID3v2 |
| ID3ISO88591 | `025280d33d85d942b0a8699eb8a0b3d2` | 0 ID3v2.3 Latin-1, 1 ID3v2.3 UTF-16, 3 ID3v2.4 UTF-8 |

The MPEG Init/DDX/Update functions are `0x1401ec954`, `0x1401ec854`
and `0x1401ed5f4`; the APE equivalents are `0x1401e065c`,
`0x1401e0598` and `0x1401e0e44`. Their shipped debug names include
`CMTOptID3Dlg::Init` and `CMTOptAPEDlg::Init`. Native literal labels at
`0x140954208`, `0x140954210`, `0x140954368` and `0x140954378`
identify APE, APEv2, ID3v1 and ID3v2 controls. The extended PE dialog resources
1066 and 1137 (English language `0x409`) bind the controls' positions/types;
the encoding radio controls retain their actual captions. MPEG Init selects
control `0x6dd` for 0, `0x6df` for 1 and `0x6de` otherwise; Update writes
0, 1 or 3. Those captions establish Latin-1, UTF-16 and UTF-8 respectively.

All native values are decoded as little-endian u32. Unsupported packed bits
are reported while only the listed switches transfer. A write mask with no
supported write type is skipped rather than manufacturing a valid write type
or publishing settings rejected by TagGroove's existing validation. Unknown
encoding values are skipped; this does not assume they mean the UI fallback.
Later records replace earlier ones, including when the final value is skipped.
Other saved options and the source bytes remain unchanged.

These imported settings use the existing TagGroove save policy. MPEG Remove
flags affect the explicit removal command; Ape/Mpc Remove flags can also strip
ID3v2 and unwritten ID3v1/APE tags during ordinary saves
(`crates/tag-core/src/tag_policy.rs`). The guide explicitly discloses this
future-save effect; import itself never writes audio. Static mapping evidence
does not establish original runtime equivalence of those save policies.

## Implementation and focused evidence

`src-tauri/src/mp3tag_settings.rs` bounds-checks every header and payload,
skips unknown records without copying payload data, and applies the last
occurrence of a supported identifier. Only confirmed mappings enter the
unpublished setup stage. Existing settings are deserialized and validated
before modification. Panel values and browser filter visibility participate
in the existing durable setup publication and Restore mechanism.

`binary_settings_migration` initially passed five focused Rust cases: all 12 initial mappings
with raw values 0, 1 and 255; truncated/oversized records; unknown and duplicate
records with extension bytes; refusal of invalid current settings; and real
preview/apply/Restore filesystem transactions preserving unrelated values and
original source bytes. The focused frontend preview case also passed, showing
listed On/Off values and cancellation without applying the setup.

The profile transaction regression cohort passed 29 tests, with one subprocess
worker intentionally ignored; 52 focused profile-host and translation checks
passed. TypeScript, scoped Clippy, formatting and the strict user-guide build
also passed. These cohorts overlap and are not additive feature counts.

The expanded importer passed six focused `binary_settings` tests, including
all 18 boolean mappings at 0, 1 and 255 and the Extended Tags u32 mode case.
The additional six output flags also passed scoped Clippy, catalog validation,
formatting and the strict guide build; a read-only review found no concrete
issues with the mappings or documented dependencies.

With playlist text support, the immutable focused cohort passed nine tests,
including exact Unicode/space preservation, first-NUL/duplicate behavior,
malformed/oversized text refusal and foreign/empty filename handling. The
real profile filesystem case now carries both strings through apply and exact
Restore. The 29 profile regression tests, 52 profile-host/catalog tests,
TypeScript, scoped Clippy, formatting and strict guide build passed. A read-only
review found no concrete issues, including the default-skipped verbatim flag's
backward-compatible journal serialization.

The tag-policy slice passed 11 focused binary-settings tests. Its new cases
exercise 96 mask/field vectors against the independent native-control fixture,
five encoding values with last-record replacement, preservation of unrelated
preferences, skipped unsupported choices and short u32 refusals. Scoped Clippy,
catalog validation, formatting and the strict guide build passed. The read-only
review found no code issue; its guide correction about Ape/Mpc ordinary-save
removal was applied and rechecked.

The message slice passed the same 11 focused binary-settings tests with all
29 boolean bindings. The manifest case now includes 29 one-hot profiles in
addition to all-zero, all-one and noncanonical-true profiles, so swapping two
boolean destinations cannot hide behind equal input values. The expanded case
checks 928 mapped values through the actual saved JSON/browser preferences.
Catalog tests, scoped Clippy, formatting and the strict guide build passed;
the read-only review found no concrete issues with the message mappings or
their documentation.

The final three General bindings passed the same 11 focused binary tests,
including all 32 boolean records and their one-hot profiles (1,120 mapped-value
checks). Scoped Clippy, formatting and the strict guide build passed; existing
catalog entries cover all three labels. The read-only review found no issues
with the bindings, preservation or stated qualification limits.

The mapping fixture is independently authored from static identifiers and
codec evidence. It is not an exported private profile, a Windows runtime
comparison, or proof of the original application's acceptance of generated
files. Other binary setting types/mappings and cross-platform native acceptance
remain open. No original app or local native UI replay was run for this slice.
