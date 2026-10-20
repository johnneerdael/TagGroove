# Native Options and remaining named bindings

This working-branch L57b extension implements twenty further source-bound
records. A source-surface disposition is still required before claiming L57b
local implementation complete; native/original acceptance remains separate.
The [earlier crosswalk](mp3tag-options-migration-crosswalk-2026-10-20.md) is a
frozen audit, with the superseding decisions recorded here.

## Source contracts

Pinned Windows Mp3tag 3.36.1 x64 executable SHA-256:
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
Static inspection only; the original was never run. The existing raw-ID16 /
LE-u32-length outer framing, first-NUL UTF-8 strings and one-byte nonzero boolean
semantics remain. All dword choices consume four bytes, not only the first.
Later records win, including unsupported final values, so earlier valid
records cannot silently revive.

| Raw ID | Target | Native codec |
| --- | --- | --- |
| `345420b668107b40b253c362f84c07ae` | `appearance / window.opentag.appearance` | u32LE reader0756f4/writer075724 |
| `2fc8d3d38c9a594394745f4bf5373c52` | `pages.appearance.toolbar_scale` | u32LE reader0756f4/writer075724 |
| `35de0f55b39ed3468af928dbf3a478f7` | `pages.appearance.ui_scale` | u32LE reader0756f4/writer075724 |
| `7bbbf22d6046a94d849c82eb792d0910` | `pages.library.enabled` | bytebool reader075640/writer075670 |
| `65378755075c9046b6d4ac4cd6346af9` | `pages.library.restrict` | bytebool reader075640/writer075670 |
| `eada7c7f01da5f4a849392b72b37ef08` | `pages.playlist.write_bom` | bytebool reader075640/writer075670 |
| `e60558750d35d148b3f22c2c229005dd` | `pages.export.dont_create_directories` | bytebool reader075640/writer075670 |
| `73e3c9e1559ea5449bad26fadc7ea56d` | `pages.directories-startup.startup` | u32LE reader0756f4/writer075724 |
| `f64318074ac91b4db1b909e2ce5a695c` | `pages.tag-sources.cover_filename` | UTF8 native constructor072f70/reader3037a8/writer303748 |
| `a29cd34be23c764e89f5976ccaa2da25` | `pages.tag-sources.musicbrainz_cover_size` | u32LE reader0756f4/writer075724 |
| `a4b6e530061c5642a1adb455bf362784` | `pages.tag-sources-freedb.comment_disc_id` | u32LE reader0756f4/writer075724 |
| `192fa431f5594c4c9c17940c895f2eb9` | `pages.tag-sources-freedb.disc_id_field` | u32LE reader0756f4/writer075724 |
| `7b29e93f08c57b4abdec988fa4ab9583` | `Startup update mode` | LE u32: off/stable/beta |
| `7c7d406cdfd18b48a914db716854fc2f` | `Release-note display` | byte bool |
| `f0bb1392e33e1041801e42ce3b64f1b7` | `Remembered cover-import answer` | byte bool: preserve/replace |
| `1a0bfd2a930e704dac9262461ddcd9c6` | `Language` | UTF-8 stock catalog stem |
| `312fa18b404068489ebd20ea4183cc24` | `Generator filename` | UTF-8 expression; source fallback |
| `0e3b78b6452b224789abc77122e2d677` | `Generator partition` | UTF-8 grouping expression |
| `1f60b2ade49d454cb0eeeaf43c19359e` | `freedb ordered server list` | LE u32 count; four length-prefixed UTF-8 fields per row |
| `2345b1c1c149ad4fbaeed1d837e96668` | `freedb active row` | LE u32 index into the same source list |

Appearance DDX/Init/save `0x14013eb88` / `0x14013ee1f` / `0x14013f82a`
bind system/dark/light. Toolbar `0x14013ef9f..0x14013f0f6` and save
`0x14013f941` encode 0..4 as 100/125/150/175/200. UI scale
`0x14013ef15..0x14013ef99` and save `0x14013f876..0x14013f935`
use percentages in 10-point steps; unsupported values normalize to 100.
The target UI dropdown now includes those percentages while retaining its
existing 125/175 choices. Theme uses the existing staged window receipt. Language
updates both the saved Options page and the window preference: startup reads
the page after loading Options, so a window-only update would be overridden.

Library enable/restrict bind through `0x1401efc01` / `0x1401efc15` and save
`0x1401f05e2` / `0x1401f0598`. No foreign included-folder list is adopted.
An explicit false restriction can transfer; true preserves the current
restriction with a warning. Enabling requires an explicit admitted false
restriction. An explicit disable can still transfer. Existing target folders
are never replaced with source host paths or reused as source identities.

Startup Init `0x1401e1905..0x1401e19cf` admits empty/previous; the specific-folder
choice keeps the current mode. Previous is explicitly target-local history.
Source-cover filename `0x1401f3427..0x1401f3438` and cover-size
`0x1401f2fae..0x1401f30d5` bind the public expressions and 1/2/3/4 sizes.
Filename expressions use portable-prefix admission and compile without
execution; explicit blank cover filename retains the target's existing runtime
`folder` fallback. Partition expressions are grouping keys, not folder paths.

The freedb output fields are separately bound dwords at DDX `0x1401e803d` /
`0x1401e8051`; only 0/1 transfer. Playlist BOM and export no-directories remain
independent. The latter overlays the final in-memory export library as well as
the effective Options page, avoiding a later stale library save.

Generic updater mode `0x1402050c4` transfers off/stable/beta plus release-note
preference. Off preserves current beta selection; no native defaults are
inferred from absence. TagGroove's endpoint, current version and saved
`last_version` remain unchanged. This supersedes the audit's proposed updater
exclusion; it does not transfer product-specific updater identity.

Cover consumer `0x1402296f4` reads ask and remembered answer independently.
TaskDialog Yes (button 6) at `0x140229e99` keeps covers; the false branch at
`0x140229ec6` removes them before import. The target stores preserve/replace;
no cover operation runs during setup import.

Language Update `0x1401ef3b0` strips extension then directory at `0x1401ef47a`
and `0x1401ef48a`, storing the bare stem at `0x1401ef554`. Pinned stock English,
Dutch and German metadata identify en/nl/de. Empty maps to system intent;
unknown/custom stems preserve the current catalog. No catalog contents transfer.
Windows locale choice versus target system language selection stays qualified.

Generator Init `0x140140260` and OnOK `0x140140cbc` bind independent filename
and partition. Empty filename falls back at `0x140140af0..0x140140b62` to the
final admitted source ordinary filename, validated again for generator use.
No source fallback means keep current with warning. Empty partition stays empty.

freedb reader/writer `0x1400748d4` / `0x140074ad4` frame **four** fields
(name, address, path, port). This differs from the mapping list's five fields.
Update `0x1401e9824` calls `0x1401e8298`; selection resolution through
`0x1401e862c` saves the exact list index at `0x1401e82dc`.
Native request consumers use literal HTTP (`0x140938b10`, `0x140938c00`,
`0x140938ca0`). The importer preserves that protocol and row order/names.

Lists are bounded to 4096 rows and 4 MB per raw field inside the profile's
existing total bounds. Every row must decode and pass target provider URL
validation without network access. Empty lists, missing/invalid index, bad
hosts, credentials/query/fragment, invalid ports or unsupported paths preserve
the whole current dataset. No index clamp, inferred zero, unrelated target-list
selection, default restoration or credential-bearing partial list is published.
All six target list/selection/URL aliases change together in the unpublished
stage. Preview trims address/path and supplies the provider's leading slash
without rewriting the saved authored fields. Email identity is kept. Extension bytes warn; malformed framing refuses
the source settings file. Preview includes escaped, verbatim admitted rows and
the selected row. Unknown/invalid payload values do not leak into diagnostics.

## Qualification and verification

These twenty additions bring recognized records to 92. This is an inventory,
not an original-settings completeness percentage. Global forced source debugging
is a target extension; per-script Debug/DebugWriteInput is not its serialized
global counterpart. Source-authored scripts retain their own commands.
Independent toolbar/tool/networking work and host-specific paths remain owned
by L50/L51/L64 and explicit migration policy. Original/native import, restart,
restore and platform acceptance remain C12/C27/L78/L79.

Focused RED reproduced missing settings and the UI dropdown's missing 120%
choice. Subsequent RED reproduced the missing freedb dataset and invalid
source generator fallback. Ten pure cases now pass, including real target
provider URL validation, all named enum choices, independent flags, final
unknown choices, malformed framing and whole-dataset preservation. No local
application/native UI replay or original/provider network request was run.
The combined import/restore case passes for all twenty bindings plus their
ordinary-playlist fallback, preserving exact previous managed bytes and window
state. The full profile-migration module passes 43 cases with one controlled
restart child ignored; 29 settings and 75 affected frontend cases, TypeScript,
the strict guide build and two-package library/test Clippy pass. Review exposed
a relative-path preview mismatch and the second startup language consumer;
both regressions reproduced the defects before correction. The final settings
module, combined exact-restore case and application library/test Clippy with
warnings denied pass after those corrections; final read-only review is clear.
Cargo emitted an unrelated global-registry cache-clean permission warning;
compilation/lint itself completed successfully.

Static audit `taggroove-l57b-target-options-crosswalk.json` SHA-256: `79b59afa3727bf6e0707a0bc8b0573d2c1817355e24785d5be7b0e7b81a0f4c8`.

Static audit `taggroove-l57b-binding-remainders.json` SHA-256: `07d37b906883b9dd381aaf1c27a66e6dd7d9d9edac04bd25bef7fa6608f3f818`.
