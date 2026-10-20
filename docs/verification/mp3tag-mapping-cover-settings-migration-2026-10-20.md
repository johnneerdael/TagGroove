# Native field mappings, cover directory and padding-off migration

This unmerged L57b slice adds four source-bound records. L57b remains open for
further confirmed adapters and individually named binding questions in the
[finite Options crosswalk](mp3tag-options-migration-crosswalk-2026-10-20.md).
A whitelist or preview-row count is not a completeness proof.

## Shipped source contract

Pinned Windows Mp3tag 3.36.1 x64 executable SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
Static inspection only; the original was not executed.

| Raw record ID | Contract and target |
| --- | --- |
| `41a00ac0f0ccf14f924a133d5bd6e0e7` | Ordered field mappings; u32 row count then five byte-length-prefixed strings per row |
| `a7a810227a5a8d4db511e77851eb10dd` | UTF-8 cover directory expression → `pages.directories.cover_art` |
| `81e9cd2aac92e14e9c71cf21ae4e0510` | Track leading-zero byte flag; false → remembered `track_digits=1` |
| `e4d56fdc8c9b154c99e8c0adf4f25b22` | Disc leading-zero byte flag; false → remembered `disc_digits=1` |

Mapping initializer `0x140005b00` constructs object `0x140ae8990` through
`0x140074414`; reader/writer `0x1400744a4` / `0x1400746a4` frame five fields,
not three. App load/save `0x14001a228` / `0x140019e84` bind the first three
(tag type/source/target) to the runtime list. Mapping Options Init/Update
`0x1401f12b4` / `0x1401f19e8` transfer the ordered vector `0x140ae9470`.
Native validator `0x1401f2170` walks the exact five type literals `APEv2`,
`ID3v2`, `MP4`, `VorbisComment`, `WMA`; no ASF or MP4iTunes alias is inferred.
The bounded validator disassembly SHA-256 is
`68f0669effa32808fc9a52424b6956f43bd8443473ddbdce76f53e5c379b5eed`.

Cover initializer `0x140001740`, object `0x140ae5a50`, uses the existing raw
UTF-8 reader/writer `0x1403037a8` / `0x140303748`. Directories DDX
`0x1401e12b0` binds member `0x328` / control `0x4ac`; Init `0x1401e1867` and
save `0x1401e1ff0` use that value. `_C_OPT_DIR_COVER` independently binds its
label. Native consumer `0x140133b38` evaluates it in selected-file context.
The adjacent startup-folder field remains separate.

The existing numbering audit establishes the two one-byte padding flags and
dynamic true widths 2/3/4. False is representable by the existing fixed-width
minimum of one; true is not guessed as two. Other flag/host-index/track-disabled
and special-field limitations remain explicit under the crosswalk and L66.

## Admission and preservation

The existing outer ID16/LE-u32-length envelope remains unchanged. Mapping lists
are bounded to 4096 rows and 4 MB per raw field. All five fields are framed;
only the three public fields are UTF-8 decoded, through first NUL. Reserved
fields remain opaque. Nonempty reserved payloads and trailing extension bytes
receive fixed diagnostics without copying unknown data. Malformed framing,
UTF-8 or declared bounds refuse that settings file before publication.

The last mapping-list record wins. A valid list replaces the whole current
mapping list in source order; explicit zero clears it. An absent record keeps
it. Existing target validation governs types/names/conflicts. An unsupported
or conflicting final list keeps the entire destination list; it never revives
an earlier valid record or silently publishes a partial subset. Preview shows
an owned translated count label and each escaped/verbatim ordered triple.

Cover expressions are compiled but never evaluated or probed by migration.
Accepted authored text is preserved, including explicit empty. Admission checks
trimmed leading text because the chooser trims its eventual result. Static
absolute, drive or URI prefixes and backslash syntax are rejected on every
platform by explicit portable-only policy. Schemes without `://`, such as
`file:/...`, are covered. Dynamic script branches are not evaluated to infer
their eventual paths. The existing Add cover fallback remains unchanged.

Explicit false padding overlays only the matching digit field on validated
existing numbering memory; all unmentioned counters remain. True/nonzero is
skipped with a fixed warning, preserving current fixed digits. No native
default is imported when a record is absent. Settings and browser memory stay
inside the existing staged configuration/receipt/restore engine. No audio write,
provider access, directory creation, export or numbering execution occurs.

These additions bring recognized native records to 72. Mapping preview has a
variable number of rows, so the older fixed preview maximum is historical:
there are at most 82 scalar rows plus one mapping-list summary and up to 4096
mapping rows. These bounds are admission limits, not parity percentages.

## Focused verification

RED reproduced unsupported mapping-only and cover/padding-only profiles. GREEN:
six pure native-record cases and an integration case with populated/explicit-
empty variants, unchanged source bytes and exact managed/window restoration.
Codec cases cover five-field framing, first NUL, Unicode, opaque reserved bytes,
truncation, declared limits, final-record overrides, explicit clears and whole-
list preservation for unsupported/conflicting data. Cover/padding cases cover
empty/relative/derived expressions, preserved starts, foreign paths and dynamic
padding refusal. Review found whitespace-prefixed host paths and URI schemes
without `://`; added regressions reproduced that hole before the correction.

Final focused checks: 19 native-settings tests and 42 profile-migration tests
passed (one controlled restart child is opt-in), as did 77 affected frontend
tests, the strict guide build and two-package library/test Clippy with warnings
denied. The final read-only source/docs/locale review had no findings. Cargo
reported an unrelated permission warning while cleaning its global registry
cache; compilation and lint completed successfully. Run bindings are retained
in the task checkpoint.
Original-generated fixtures, original runtime, native restart/restore, platform
and release acceptance are not inferred. No local application/native UI replay
or GitHub review/CI/merge is claimed. C12/C27/L78/L79 retain their qualification.
