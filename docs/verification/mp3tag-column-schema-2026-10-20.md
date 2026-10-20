# Mp3tag column schema and numeric alignment

Working-branch evidence for L07, dated 2026-10-20 and based on main `7544625`.
The scope is the shipped Windows 3.36.1 reader/writer and the local importer.
No original application, populated private profile or Windows UI was executed.
This is not a merged-PR or release receipt.

## Reference schema

The previously extracted official `Mp3tag.exe` has SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
A bounded leanrea inspection used PE section/runtime-function tables,
RIP-relative references and LLVM disassembly. The debug string
`CMTFileListCtrl::GLoadColumnsFromFile` identifies the reader extent
`0x1400c1878–0x1400c1d2c`; `GSaveColumnsToFile` identifies the writer extent
`0x1400c1d54–0x1400c2110`. These bounds are static observations, not execution.

Both inspected routes use the following seven keys. The table records
reader/writer reference instructions, not original source code.

| Native key | Reader reference | Writer reference | TagGroove mapping |
| --- | --- | --- | --- |
| `name` | `0x1400c1b39` | `0x1400c1ee3` | Column heading |
| `value` | `0x1400c1b56` | `0x1400c1f3d` | Display expression |
| `field` | `0x1400c1b90` | `0x1400c1ff1` | Editable field placeholder |
| `sort` | `0x1400c1b73` | `0x1400c1f97` | Sort expression |
| `width` | `0x1400c1af4` | `0x1400c1e30` | Width subject to local limits |
| `numeric` | `0x1400c1ad7` | `0x1400c1e85` | Numeric sorting and default right alignment |
| `enabled` | `0x1400c1bc2` | `0x1400c2042` | Visibility, subject to one-visible-column safeguard |

All seven keys were already recognized by
[the importer](../../src-tauri/src/mp3tag_columns.rs). No additional key mapping
was established in these two inspected routes. That finding does not certify
other versions, adjacent global/binary settings, malformed files or a complete
profile round trip. Those should not be invented from undocumented key names.

The official [File List documentation](https://docs.mp3tag.de/customization/file-list/)
also states that Numeric affects sorting and alignment. The local importer
previously set numeric sorting but defaulted alignment to Left. It now uses
Right for numeric columns when no explicit alignment is present, and preserves
explicit `align`/`alignment` entries. Ordinary nonnumeric columns remain Left
by default. Existing native JSON layouts are unaffected by this import mapping.

## Focused checks

An independently authored fixture uses all seven observed keys, with no
copied private/vendor profile. The regression matrix covers native numeric
and nonnumeric defaults, explicit Left and Center overrides, and the remaining
name/value/field/sort/width/visibility mappings. Before the change, the native
numeric case failed with Left instead of Right.

RED run: `taggroove-l07-numeric-alignment-red-_buy6bde`.
GREEN run: `taggroove-l07-numeric-alignment-green-62y7c0v3`, with all five
importer tests passing and 614 unrelated cases filtered out. A focused
read-only code review found no remaining issue in the mapping change.
The local probe retains target digest, function extents,
instruction locations and assembly hashes under
`/tmp/taggroove-p1-policy-functions.json` and
`/tmp/taggroove-p1-column-key-static.json`.

Original generated-file/re-import and rendered width/alignment checks remain
platform/reference qualifications. This evidence does not close L57b's distinct
global/binary or per-field suggestion settings.
