<!-- Static audit snapshot; implementation status is frozen at the stated baseline. -->
# L57b target Options crosswalk

Pinned original: Windows Mp3tag 3.36.1 x64, SHA-256 `484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`. Target baseline `422bc30b320b05807bb081e2ec9e019eae98b5d9`; parent in-progress four-record additions were visible. Read-only static audit: no app/native replay, tests, builds, network/GitHub, credentials or repository edits. All new evidence is under `/tmp/taggroove-l57b-final-inventory-*`.

This is a finite crosswalk of the target Options surface: registered page controls, all fields of backend Settings/TagOptions, custom editor schemas, buttons, and the named adjacent saved page defaults. It does **not** claim to inventory every original serialized object or original native profile feature.

## Later implementation disposition

This audit is frozen at its stated baseline. Its proposed exclusion of generic
update preferences is superseded: the importer transfers startup-check,
stable/beta and release-note choices while retaining TagGroove's endpoint and
version history. The eleven named binding rows have also been resolved into
six records plus the target-only global debug disposition. See the
[twenty-record implementation receipt](mp3tag-options-bindings-migration-2026-10-20.md)
for current behavior. The tables below retain the audit's original statuses;
they must not be read as a live to-do list.

## Result

The target surface contains twelve further source-bound native preferences beyond the existing 68 records and parent current four additions. They are listed below with exact IDs/codecs/bindings. Some support only a bounded source subset; the remainder must stay explicit. Twelve is an actionable record inventory, not a closure criterion.

Updates have source bindings but are proposed product-specific exclusions because the target checks a different product and endpoint. Foreign host paths, credential/provider identities, non-equivalent target-only controls and independently owned work are classified separately. Eleven unknown rows name actual target fields; none is an unbounded modern-others bucket. Six of those rows share one unresolved freedb server collection/active-identity problem.

## Confirmed additional native contracts

### `appearance / window.opentag.appearance`

`345420b668107b40b253c362f84c07ae`; object `0x140ac5790`; u32LE reader0756f4/writer075724.

0system,1dark,2light. Target portable window preference validated in configuration_transfer.rs181; apply via staged window receipt, not a new settings field.

Bindings: `Appearance resource25150 control86c`; `DDX13eb88 member150`; `Init13ee1f/ee6c/eeb2 itemdata0/1/2`; `save13f82a valueac57d0`.

### `pages.appearance.toolbar_scale`

`2fc8d3d38c9a594394745f4bf5373c52`; object `0x140ac3bc0`; u32LE reader0756f4/writer075724.

Admit source enum0/1/2/3/4 to target strings100/125/150/175/200; diagnose unknown enum. Do not mistake UI scale numeric record for toolbar.

Bindings: `resource25150 Toolbar Scaling control6f7`; `DDX13eb88 member780`; `Init13ef9f..f0f6 adds0/1/2/3/4`; `save13f941 valueac3c00`.

### `pages.appearance.ui_scale`

`35de0f55b39ed3468af928dbf3a478f7`; object `0x140ac5510`; u32LE reader0756f4/writer075724.

Original percent100..200 in steps10; native Init/save normalize other values to100. Target scalar appearanceOptions admits100..200, but dropdown lists only100/110/125/150/175/200. Decide admission/display for120/130/140/160/180/190; do not silently display100 while applying an unavailable percent.

Bindings: `resource25150 UI Scaling control969`; `DDX13eb88 member360`; `Init13ef15..ef99`; `save13f876..f935 valueac5550`.

### `pages.library.enabled`

`7bbbf22d6046a94d849c82eb792d0910`; object `0x140ac5560`; bytebool reader075640/writer075670.

Actual cache-enable preference exists in both products. Couple with native restriction disposition so skipping foreign directories does not turn a restricted source into an unrestricted target silently.

Bindings: `DDX1efc01 member8a8/control72b`; `Init1efc95`; `save1f05e2 valueac55a0; native confirmation can veto change`.

### `pages.library.restrict`

`65378755075c9046b6d4ac4cd6346af9`; object `0x140ac2090`; bytebool reader075640/writer075670.

False is directly representable. True requires admitted source directory list/identity; never apply against unrelated existing target list. Source host paths remain untransferred without mapping.

Bindings: `DDX1efc15 member8ac/control72c`; `Init1efca2`; `save1f0598 valueac20d0`.

### `pages.playlist.write_bom`

`eada7c7f01da5f4a849392b72b37ef08`; object `0x140ac2f90`; bytebool reader075640/writer075670.

Direct BOM preference for UTF8 playlist outputs.

Bindings: `DDX1f3b39 memberab8/control97c`; `Init1f3cab`; `save1f476a valueac2fd0`; `language15f321 slotca0 WRITE_BOM; Init1f3ed9 applies to control97c`.

### `pages.export.dont_create_directories`

`e60558750d35d148b3f22c2c229005dd`; object `0x140ac51a0`; bytebool reader075640/writer075670.

Direct export no-directory-creation preference, independent of playlist no-directories.

Bindings: `DDX1e4911 member154/control570`; `save1e4f91 valueac51e0`; `Options export resource1008 control570`.

### `pages.directories-startup.startup`

`73e3c9e1559ea5449bad26fadc7ea56d`; object `0x140ac5740`; u32LE reader0756f4/writer075724.

Source0empty and1previous are target-representable; source2specific-startup-directory is NOT target favorite. Retain/diagnose2 unless original path is explicitly translated/bound. Previous uses current target workspace, so disclose no transfer of foreign previous-directory path.

Bindings: `Init1e1905..19cf itemdata0empty/1previous/2standard; language slotsa00/a08/a10`; `save1e1f5d valueac5780`.

### `pages.tag-sources.cover_filename`

`f64318074ac91b4db1b909e2ce5a695c`; object `0x140ae7030`; UTF8 native constructor072f70/reader3037a8/writer303748.

Direct public authored cover filename format; preserve empty/normalization policy deliberately and validate target filename expression; foreign path text diagnosed.

Bindings: `initializer002734 ID920120 defaultfolder`; `Init1f2db1 valueae7070`; `save1f3427..3438 frommember360`; `targettag-sources.ts293`.

### `pages.tag-sources.musicbrainz_cover_size`

`a29cd34be23c764e89f5976ccaa2da25`; object `0x140ac2130`; u32LE reader0756f4/writer075724.

Admit1original,2=250,3=500,4=1200. Unknown values diagnose rather than infer default. Original default4=1200 differs target500; absent source must not import default.

Bindings: `Init1f2fae..30d5 itemdata1/2/3/4, literal250/500/1200`; `save1f3539 valueac2170`; `label15f39e slotcc8, Init1f2f9f control86e`.

### `pages.tag-sources-freedb.comment_disc_id`

`a4b6e530061c5642a1adb455bf362784`; object `0x140ac26d0`; u32LE reader0756f4/writer075724.

Admit ordinary checkbox0/1; unsupported u32 values need explicit handling. Public output-field switch, not credentials or network routing.

Bindings: `DDX1e803d control676/member750`; `Init1e8fff valueac2710`; `save1e9911`; `labelslot158 loaded15cfe0 literal94dc20; English.lng43`.

### `pages.tag-sources-freedb.disc_id_field`

`192fa431f5594c4c9c17940c895f2eb9`; object `0x140ac2720`; u32LE reader0756f4/writer075724.

Admit ordinary checkbox0/1; unsupported u32 values need explicit handling.

Bindings: `DDX1e8051 control677/member838`; `Init1e9026 valueac2760`; `save1e992c`; `labelslot160 loaded15cff9 literal94dc40; English.lng44`.

## Mapping type correction / proof

Native validator `0x1401f2170` walks the exact five-string table `0x140ac6970..0x140ac6998`: `APEv2`, `ID3v2`, `MP4`, `VorbisComment`, `WMA`. They match target canonical types. WMA consumers pass literal `0x14095d630` through `0x14035d098`/`0x14035d4b1`; MP4 consumers use `0x14095d6c8` at `0x140342bce` and `0x140345767`. No `ASF` or `MP4iTunes` alias is needed or proved. The full list codec and 2 unused serialized columns remain in the earlier final-inventory report.

## Appearance correction and admission

Use DDX/resource identities, not the order of combo retrieval in save. Resource 25150 and DDX `0x14013eb88` bind **UI scaling** to member360/control969 (numeric source ID `35de...`), **toolbar scaling** to member780/control6f7 (enum source ID `2fc8...`), and **Icons** to member570/control980 (source ID `2d266dca0f66a84d9c2c15d782f0e55e`, no equivalent target icon-family selector). Preliminary naming was corrected before parent implementation. Color is member150/control86c and source enum0system/1dark/2light.

The native toolbar enum maps directly to target100/125/150/175/200. Native UI scale uses integer percentages100..200 in steps10; native malformed/nonstep values normalize100. Target runtime accepts all percentages in that range, but its select lists100/110/125/150/175/200. For example native120 is currently representable by the runtime but may be shown as100 by the select. Require shared-value admission or a visible unavailable-percent representation; do not silently apply120 while showing100.

## Full target crosswalk

`MAPPED_68` means an existing adapter was seen; `CURRENT_NEW_4` means parent in-progress implementation, not passing tests or a committed result. File adapters preserve their existing stated subset/qualifications. Rows are target fields and can share one native record.

| Target control / stored field | Disposition | Original ID / reason |
|---|---|---|
| `appearance / window.opentag.appearance` | MISSING_CONFIRMED | `345420b668107b40b253c362f84c07ae` — 0system,1dark,2light. Target portable window preference validated in configuration_transfer.rs181; apply via staged window receipt, not a new settings field. |
| `button:check-updates` | ACTION_OR_DERIVED | Runs product-specific update check, not saved setting. |
| `button:customize-tag-panel` | ACTION_OR_DERIVED | Editor entrypoint; fields/layout covered by file adapter rows. |
| `button:edit-field-values` | ACTION_OR_DERIVED | Editor entrypoint; lists/preferences covered by file adapter rows. |
| `button:open-languages-folder` | ACTION_OR_DERIVED | Opens current app catalog folder, not portable setting. |
| `button:tag-source-keys` | ACTION_OR_DERIVED | Private credential manager; explicitly excluded, never read/import credentials. |
| `custom:language-warnings` | ACTION_OR_DERIVED | Derived local catalog diagnostics, not saved setting. |
| `custom:library-maintenance` | ACTION_OR_DERIVED | Target-local cache cleanup/clear jobs; no source cache transfer. |
| `custom:local-freedb-index` | ACTION_OR_DERIVED | Build/cancel target-local text index, not portable source state. |
| `directories.subfolders` | MAPPED_68 | `fbb012dd766b664493dff2ac5f006238` — Native import present; static implementation inventory, not original runtime acceptance. |
| `exports.active_configuration` | MAPPED_68 | `536d231fc389234bafa84663f0986a0c` — Existing bounded UTF8 native-text import; pairing/identity rules apply for numbering/export. |
| `exports.preferences.append` | MAPPED_68 | `a91684289cba6f498fd857faeaa3749c` — Native import present; static implementation inventory, not original runtime acceptance. |
| `exports.preferences.filename` | MAPPED_68 | `d1c3c5a4c95780489c008de588e2eeea` — Existing bounded UTF8 native-text import; pairing/identity rules apply for numbering/export. |
| `exports.preferences.partition enabled` | MAPPED_68 | `9e917d2becf8af4793061746fdf7bad5` — Native import present; static implementation inventory, not original runtime acceptance. |
| `exports.preferences.partition_memory` | MAPPED_68 | `e31f66267bd2a34baffa13f42d34fd95` — Existing bounded UTF8 native-text import; pairing/identity rules apply for numbering/export. |
| `files.folder_separator` | MAPPED_68 | `375e4b6e9c45af49a77caac696ced2b6` — Existing bounded UTF8 native-text import; pairing/identity rules apply for numbering/export. |
| `files.restrict_extensions` | MAPPED_68 | `0c7f0a198af644488be02a0d19631585` — Existing bounded UTF8 native-text import; pairing/identity rules apply for numbering/export. |
| `general.natural_sort` | MAPPED_68 | `3ec5faaa30c28a4b8ea2c470e1f7a276` — Native import present; static implementation inventory, not original runtime acceptance. |
| `general.show_directory_in_title` | MAPPED_68 | `6317969e3cb5424781aab010efedb4a2` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.appearance.grid` | MAPPED_68 | `10ad9a5de1a201419efb4f510bd262f4` — Native u32 0off/1alternate/2lines |
| `pages.appearance.toolbar_scale` | MISSING_CONFIRMED | `2fc8d3d38c9a594394745f4bf5373c52` — Admit source enum0/1/2/3/4 to target strings100/125/150/175/200; diagnose unknown enum. Do not mistake UI scale numeric record for toolbar. |
| `pages.appearance.ui_scale` | MISSING_CONFIRMED | `35de0f55b39ed3468af928dbf3a478f7` — Original percent100..200 in steps10; native Init/save normalize other values to100. Target scalar appearanceOptions admits100..200, but dropdown lists only100/110/125/150/175/200. Decide admission/display for120/130/140/160/180/190; do not silently display100 while applying an unavailable percent. |
| `pages.directories-startup.startup` | MISSING_CONFIRMED | `73e3c9e1559ea5449bad26fadc7ea56d` — Source0empty and1previous are target-representable; source2specific-startup-directory is NOT target favorite. Retain/diagnose2 unless original path is explicitly translated/bound. Previous uses current target workspace, so disclose no transfer of foreign previous-directory path. |
| `pages.directories.cover_art` | CURRENT_NEW_4 | `a7a810227a5a8d4db511e77851eb10dd` — Parent implementing native UTF8 directory-expression import; foreign host paths remain qualified. |
| `pages.export.dont_create_directories` | MISSING_CONFIRMED | `e60558750d35d148b3f22c2c229005dd` — Direct export no-directory-creation preference, independent of playlist no-directories. |
| `pages.export.show_dialog` | MAPPED_68 | `97a81ca5caca824ab21be214338c735e` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.export.write_bom` | MAPPED_68 | `759de112c0072d48b5650c2fbec586a8` — Whole-u32 nonzero export-consumer contract |
| `pages.files.associated_files` | PRODUCT_NON_EQUIVALENT | Target is a narrow opt-in for same-basename .lrc/.cdg companions with specified operations, not a source-bound generic original file-association preference. Parent Copy work is separately owned L22; no guessed CFG alias. |
| `pages.general.auto_select_next` | MAPPED_68 | `53fe21e50af824458cf74de7edade298` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.general.autocomplete` | MAPPED_68 | `a17772f7656a2b4fb710657ed41cb01d` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.general.keep_moved` | MAPPED_68 | `8d7feb69eab69f4883076efb11f4b20c` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.general.minimize_to_tray` | MAPPED_68 | `d5c6e9f135cd614aa8f3d9f513f774f1` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.general.select_added` | MAPPED_68 | `de10ea102047ae448c2c53c4ba58d55e` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.general.tray_icon` | MAPPED_68 | `07a26324a1f7d84782538bb89e2a57e8` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.language.language` | SOURCE_BINDING_UNKNOWN | `1a0bfd2a930e704dac9262461ddcd9c6` — NativeUTF8 language record1a0bfd2a930e704dac9262461ddcd9c6 uses source language filename/catalog identity (objectae5970/valueae59b0; Update1ef3b0). Target uses JSON catalog locale or system. Common locale identity conversion/installed catalog availability not established; keep current rather than infer from arbitrary filename. Named target control, not open native universe. |
| `pages.library.directories` | FOREIGN_HOST_WORKSPACE | These controls require actual local filesystem directories. Native foreign Windows paths/cache contents cannot be adopted as current local directories; preserve current values. Native library restriction true must remain coupled to directory admission. |
| `pages.library.enabled` | MISSING_CONFIRMED | `7bbbf22d6046a94d849c82eb792d0910` — Actual cache-enable preference exists in both products. Couple with native restriction disposition so skipping foreign directories does not turn a restricted source into an unrestricted target silently. |
| `pages.library.restrict` | MISSING_CONFIRMED | `65378755075c9046b6d4ac4cd6346af9` — False is directly representable. True requires admitted source directory list/identity; never apply against unrelated existing target list. Source host paths remain untransferred without mapping. |
| `pages.messages.action_warnings` | MAPPED_68 | `215db77b5ec4934c9d28593f17c07afd` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.messages.after_change` | MAPPED_68 | `c926caafa65b39429d863d45454e9457` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.messages.after_delete` | MAPPED_68 | `50ee520115df554ca05d5a2318e7dbb1` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.messages.after_save` | MAPPED_68 | `a0d12c9bfeca104a9e28c4d677eeed5b` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.messages.confirm_cut_tags` | MAPPED_68 | `3e5445fa1d3cc94ea144ac8bcc08cbe6` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.messages.confirm_delete_files` | MAPPED_68 | `c2b1b77b33dd5e42b8e2bc0a90d00586` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.messages.confirm_export` | MAPPED_68 | `f943c466dfced441ab7d90819f99baaa` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.messages.confirm_import_covers` | MAPPED_68 | `fc0a600b58c13744ab155cb882f9d901` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.messages.confirm_paste_tags` | MAPPED_68 | `a5fea160d8e1d543b742a86a86a8f2d5` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.messages.confirm_remove_tags` | MAPPED_68 | `6ffacd136f47034d95660ba5e0469de8` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.messages.import_covers_choice` | SOURCE_BINDING_UNKNOWN | Original ask-before-cover record is already imported, but persistent remembered preserve/replace answer and its codec/choice binding were not established. Do not infer it from the ask bool or no-folder-image bool. |
| `pages.messages.unsaved_changes` | MAPPED_68 | `0175173ea37ff746bf47382a1c7c3f76` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.networking.proxy` | INDEPENDENT_L64 | Original network/proxy semantics are independently owned L64. Provider-specific CDDBTimeout cannot silently become target global timeout. No transfer of credential-bearing proxy URL.  |
| `pages.networking.proxy_url` | INDEPENDENT_L64 | Original network/proxy semantics are independently owned L64. Provider-specific CDDBTimeout cannot silently become target global timeout. No transfer of credential-bearing proxy URL.  |
| `pages.networking.timeout` | INDEPENDENT_L64 | Original network/proxy semantics are independently owned L64. Provider-specific CDDBTimeout cannot silently become target global timeout. No transfer of credential-bearing proxy URL.  |
| `pages.playlist.auto_create` | MAPPED_68 | `b2efeb13fa6c2c44b8386c2182ac5965` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.playlist.confirm_filename` | MAPPED_68 | `f25e55356aa23840bb4e104ab58ce1f3` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.playlist.dont_create_directories` | MAPPED_68 | `b0b41d103f16944cb184a331ae6136a9` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.playlist.extended` | MAPPED_68 | `04d7ba2f4f3bfa4d8b0c1ccf2ab1c361` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.playlist.extinf_format` | MAPPED_68 | `aa0ec86bea2e7a40a7350a520bcf4a46` — Existing bounded UTF8 native-text import; pairing/identity rules apply for numbering/export. |
| `pages.playlist.filename` | MAPPED_68 | `7eaf6b64ce85a54196043a3c8cfb1d2e` — Existing bounded UTF8 native-text import; pairing/identity rules apply for numbering/export. |
| `pages.playlist.generator_filename` | SOURCE_BINDING_UNKNOWN | Saved target Generate Playlists dialog memory, not Options page controls. Native corresponding generator-memory record/default/semantics not bound in this audit; do not reuse ordinary playlist filename or export partition record. |
| `pages.playlist.generator_partition` | SOURCE_BINDING_UNKNOWN | Saved target Generate Playlists dialog memory, not Options page controls. Native corresponding generator-memory record/default/semantics not bound in this audit; do not reuse ordinary playlist filename or export partition record. |
| `pages.playlist.relative` | MAPPED_68 | `59fc84e13a7c8044b3192709f11d754d` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.playlist.write_bom` | MISSING_CONFIRMED | `eada7c7f01da5f4a849392b72b37ef08` — Direct BOM preference for UTF8 playlist outputs. |
| `pages.tag-sources-freedb.address` | SOURCE_BINDING_UNKNOWN | Original freedb server controls exist (Update1e9824, native server fields and possible list identities). Target owns ordered server definitions plus active server aliases. Complete serialized server codec/order/active identity binding not established; do not import individual aliases into unrelated server list or mark them credential exclusions. |
| `pages.tag-sources-freedb.comment_disc_id` | MISSING_CONFIRMED | `a4b6e530061c5642a1adb455bf362784` — Admit ordinary checkbox0/1; unsupported u32 values need explicit handling. Public output-field switch, not credentials or network routing. |
| `pages.tag-sources-freedb.disc_id_field` | MISSING_CONFIRMED | `192fa431f5594c4c9c17940c895f2eb9` — Admit ordinary checkbox0/1; unsupported u32 values need explicit handling. |
| `pages.tag-sources-freedb.email` | PRIVATE_PROVIDER_IDENTITY | Provider handshake identity/personal contact, not public output behavior. Preserve destination identity; no actual user email/credential values read. |
| `pages.tag-sources-freedb.path` | SOURCE_BINDING_UNKNOWN | Original freedb server controls exist (Update1e9824, native server fields and possible list identities). Target owns ordered server definitions plus active server aliases. Complete serialized server codec/order/active identity binding not established; do not import individual aliases into unrelated server list or mark them credential exclusions. |
| `pages.tag-sources-freedb.port` | SOURCE_BINDING_UNKNOWN | Original freedb server controls exist (Update1e9824, native server fields and possible list identities). Target owns ordered server definitions plus active server aliases. Complete serialized server codec/order/active identity binding not established; do not import individual aliases into unrelated server list or mark them credential exclusions. |
| `pages.tag-sources-freedb.protocol` | SOURCE_BINDING_UNKNOWN | Original freedb server controls exist (Update1e9824, native server fields and possible list identities). Target owns ordered server definitions plus active server aliases. Complete serialized server codec/order/active identity binding not established; do not import individual aliases into unrelated server list or mark them credential exclusions. |
| `pages.tag-sources-freedb.server` | SOURCE_BINDING_UNKNOWN | Original freedb server controls exist (Update1e9824, native server fields and possible list identities). Target owns ordered server definitions plus active server aliases. Complete serialized server codec/order/active identity binding not established; do not import individual aliases into unrelated server list or mark them credential exclusions. |
| `pages.tag-sources-freedb.servers` | SOURCE_BINDING_UNKNOWN | Original freedb server controls exist (Update1e9824, native server fields and possible list identities). Target owns ordered server definitions plus active server aliases. Complete serialized server codec/order/active identity binding not established; do not import individual aliases into unrelated server list or mark them credential exclusions. |
| `pages.tag-sources-local-freedb.directory` | FOREIGN_HOST_WORKSPACE | These controls require actual local filesystem directories. Native foreign Windows paths/cache contents cannot be adopted as current local directories; preserve current values. Native library restriction true must remain coupled to directory admission. |
| `pages.tag-sources-online.deezer_cover_size` | PRODUCT_SPECIFIC | Target direct-API provider option. Original native Online Options bound here exposes cover filename/history and MusicBrainz size; no corresponding product-wide original setting established. A similarly named web-source script field/settings file is not this target direct-API preference. |
| `pages.tag-sources-online.fpcalc` | FOREIGN_HOST_WORKSPACE | Optional local executable override. Do not import foreign executable path or execute anything; built-in target fingerprinting remains current. |
| `pages.tag-sources-online.itunes_country` | PRODUCT_SPECIFIC | Target direct-API provider option. Original native Online Options bound here exposes cover filename/history and MusicBrainz size; no corresponding product-wide original setting established. A similarly named web-source script field/settings file is not this target direct-API preference. |
| `pages.tag-sources-online.itunes_cover_size` | PRODUCT_SPECIFIC | Target direct-API provider option. Original native Online Options bound here exposes cover filename/history and MusicBrainz size; no corresponding product-wide original setting established. A similarly named web-source script field/settings file is not this target direct-API preference. |
| `pages.tag-sources-online.lastfm_genres` | PRODUCT_SPECIFIC | Target direct-API provider option. Original native Online Options bound here exposes cover filename/history and MusicBrainz size; no corresponding product-wide original setting established. A similarly named web-source script field/settings file is not this target direct-API preference. |
| `pages.tag-sources-online.lrclib_synced` | PRODUCT_SPECIFIC | Target direct-API provider option. Original native Online Options bound here exposes cover filename/history and MusicBrainz size; no corresponding product-wide original setting established. A similarly named web-source script field/settings file is not this target direct-API preference. |
| `pages.tag-sources.cover_filename` | MISSING_CONFIRMED | `f64318074ac91b4db1b909e2ce5a695c` — Direct public authored cover filename format; preserve empty/normalization policy deliberately and validate target filename expression; foreign path text diagnosed. |
| `pages.tag-sources.musicbrainz_cover_size` | MISSING_CONFIRMED | `a29cd34be23c764e89f5976ccaa2da25` — Admit1original,2=250,3=500,4=1200. Unknown values diagnose rather than infer default. Original default4=1200 differs target500; absent source must not import default. |
| `pages.tag-sources.web_debug` | SOURCE_BINDING_UNKNOWN | Named target writes network pages and parser traces for web sources. Original native Online Options has no matching control in inspected resource path; original command-line/script debug is not proven persistent counterpart. Keep current; no arbitrary Debug key alias. |
| `pages.tags.extended_save_mode` | MAPPED_68 | `16ba6ee64495f544a1f46ad5ed57b876` — Native u32 admitted modes |
| `pages.tags.no_folder_image` | MAPPED_68 | `ee0051ba892b9941bce4471869a8f2d0` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.tags.save_on_move` | MAPPED_68 | `8840da8e2956d74da1853b63f6df9037` — Native import present; static implementation inventory, not original runtime acceptance. |
| `pages.tools.default_tool` | INDEPENDENT_L51 | Original tools.ini ordered identity, argument grammar and host executable paths remain L51; supported target editor does not prove native source import. |
| `pages.tools.tools` | INDEPENDENT_L51 | Original tools.ini ordered identity, argument grammar and host executable paths remain L51; supported target editor does not prove native source import. |
| `pages.updates.check_on_startup` | PRODUCT_SPECIFIC | `7b29e93f08c57b4abdec988fa4ab9583` — Preserve target updater consent/channel/version context. Native preference refers to Mp3tag updates; target contacts TagGroove GitHub releases and its own notes. A source product opt-in is not automatically target product opt-in. This is an explicit proposed scope disposition, not absence of an original counterpart. |
| `pages.updates.include_beta` | PRODUCT_SPECIFIC | `7b29e93f08c57b4abdec988fa4ab9583` — Preserve target updater consent/channel/version context. Native preference refers to Mp3tag updates; target contacts TagGroove GitHub releases and its own notes. A source product opt-in is not automatically target product opt-in. This is an explicit proposed scope disposition, not absence of an original counterpart. |
| `pages.updates.last_version` | PRODUCT_SPECIFIC | Preserve target updater consent/channel/version context. Native preference refers to Mp3tag updates; target contacts TagGroove GitHub releases and its own notes. A source product opt-in is not automatically target product opt-in. This is an explicit proposed scope disposition, not absence of an original counterpart. |
| `pages.updates.release_notes` | PRODUCT_SPECIFIC | `7c7d406cdfd18b48a914db716854fc2f` — Preserve target updater consent/channel/version context. Native preference refers to Mp3tag updates; target contacts TagGroove GitHub releases and its own notes. A source product opt-in is not automatically target product opt-in. This is an explicit proposed scope disposition, not absence of an original counterpart. |
| `tag_panel.field_values.lists` | MAPPED_FILE_ADAPTER | Native usrfields.ini/usrfieldvalues.ini/genres.ini adapters import supported layouts/values/preferences under bounded validation and fixed precedence. Original-generated populated native acceptance remains qualified. |
| `tag_panel.field_values.only_user_genres` | MAPPED_68 | `a635489304121948aceaf4ad079f3fa1` — Native import present; static implementation inventory, not original runtime acceptance. |
| `tag_panel.field_values.prefer_tag_fields` | MAPPED_FILE_ADAPTER | Native usrfields.ini/usrfieldvalues.ini/genres.ini adapters import supported layouts/values/preferences under bounded validation and fixed precedence. Original-generated populated native acceptance remains qualified. |
| `tag_panel.field_values.prefer_tag_fields.GENRE` | MAPPED_68 | `a5a066496e5a1d409b6a874bb814b65b` — Native import present; static implementation inventory, not original runtime acceptance. |
| `tag_panel.field_values.prefer_tag_values` | MAPPED_FILE_ADAPTER | Native usrfields.ini/usrfieldvalues.ini/genres.ini adapters import supported layouts/values/preferences under bounded validation and fixed precedence. Original-generated populated native acceptance remains qualified. |
| `tag_panel.panel.fields` | MAPPED_FILE_ADAPTER | Native usrfields.ini/usrfieldvalues.ini/genres.ini adapters import supported layouts/values/preferences under bounded validation and fixed precedence. Original-generated populated native acceptance remains qualified. |
| `tag_panel.panel.show_panel` | MAPPED_68 | `a0f6c4c7dcd9b44fb7d973852061f2cb` — Native import present; static implementation inventory, not original runtime acceptance. |
| `tags.advanced.id3v2_comment_language` | MAPPED_68 | `8eec759f9a170443abf0e47af87c1079` — Existing bounded UTF8 native-text import; pairing/identity rules apply for numbering/export. |
| `tags.advanced.list_chapters` | MAPPED_68 | `ac740b3914557e479e4b32dbbbacaab0` — Native import present; static implementation inventory, not original runtime acceptance. |
| `tags.advanced.matroska_video_mapping` | MAPPED_68 | `e4f596ec68aba54da0bf8bff3a489238` — Native import present; static implementation inventory, not original runtime acceptance. |
| `tags.advanced.mp4_single_atom` | MAPPED_68 | `40b8c799a9b6ac4080064606a8c10200` — Native import present; static implementation inventory, not original runtime acceptance. |
| `tags.advanced.popularimeter_ratings` | MAPPED_68 | `243cae0bb6d7804b932f5ea2f2d2a55d` — Native import present; static implementation inventory, not original runtime acceptance. |
| `tags.advanced.reuse_id3v2_frames` | MAPPED_68 | `8b86f06a405be14991918e4fc1cf13d8` — Native import present; static implementation inventory, not original runtime acceptance. |
| `tags.advanced.reuse_mp4_atoms` | MAPPED_68 | `a8c68011b761684596bd8f68db51d329` — Native import present; static implementation inventory, not original runtime acceptance. |
| `tags.ape.read_ape` | MAPPED_68 | `ac7785e8c51b644fa377abc80a954b8b` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.ape.read_id3v1` | MAPPED_68 | `ac7785e8c51b644fa377abc80a954b8b` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.ape.remove_ape` | MAPPED_68 | `9e2950e85b1dbd45b4009b2ba739a744` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.ape.remove_id3v1` | MAPPED_68 | `9e2950e85b1dbd45b4009b2ba739a744` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.ape.remove_id3v2` | MAPPED_68 | `9e2950e85b1dbd45b4009b2ba739a744` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.ape.write_ape` | MAPPED_68 | `1d46a2fc85e7654794a5490bdc3e9eec` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.ape.write_id3v1` | MAPPED_68 | `1d46a2fc85e7654794a5490bdc3e9eec` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.mappings` | CURRENT_NEW_4 | `41a00ac0f0ccf14f924a133d5bd6e0e7` — Parent implementing current native ordered five-field list adapter; first3 tag_type/source/target. Validator permits exactly APEv2,ID3v2,MP4,VorbisComment,WMA. No ASF alias. |
| `tags.mpeg.id3v2_format` | MAPPED_68 | `025280d33d85d942b0a8699eb8a0b3d2` — Native ID3v2 version/encoding selector |
| `tags.mpeg.id3v2_only_if_needed` | MAPPED_68 | `f4a149060db0964aa32d016776713477` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.mpeg.read_ape` | MAPPED_68 | `10d6c497feac8340a1d9b78566f9aede` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.mpeg.read_id3v1` | MAPPED_68 | `10d6c497feac8340a1d9b78566f9aede` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.mpeg.read_id3v2` | MAPPED_68 | `10d6c497feac8340a1d9b78566f9aede` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.mpeg.remove_ape` | MAPPED_68 | `33172482169bb947865a0d6d2f644446` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.mpeg.remove_id3v1` | MAPPED_68 | `33172482169bb947865a0d6d2f644446` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.mpeg.remove_id3v2` | MAPPED_68 | `33172482169bb947865a0d6d2f644446` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.mpeg.write_ape` | MAPPED_68 | `f4a149060db0964aa32d016776713477` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.mpeg.write_id3v1` | MAPPED_68 | `f4a149060db0964aa32d016776713477` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.mpeg.write_id3v2` | MAPPED_68 | `f4a149060db0964aa32d016776713477` — Packed native tag-mask import; writable-mask dependency validation applies. |
| `tags.preserve_modified_time` | MAPPED_68 | `c43475eedd175e42b0d89661bd66f42d` — Native import present; static implementation inventory, not original runtime acceptance. |
| `version` | PRODUCT_METADATA | Target JSON schema version, not a migrated original preference. Never import foreign schema version. |
| `window.opentag.hideFilter` | MAPPED_68 | `86fd6bff815ed0488b445b271955c35c` — Native import present; static implementation inventory, not original runtime acceptance. |
| `window.opentag.numbering.disc_digits` | CURRENT_NEW_4 | `e4d56fdc8c9b154c99e8c0adf4f25b22` — Parent implementing explicit false => digits1 only. True dynamic2/3/4 policy remains skipped/diagnosed under L66; no fixed-width equivalence claimed. |
| `window.opentag.numbering.disc_enabled` | MAPPED_68 | `c90c8b8c433f5f4e8c86ba63e902fb8b` — Native import present; static implementation inventory, not original runtime acceptance. |
| `window.opentag.numbering.disc_total` | MAPPED_68 | `5a98eb5f2200764a94daf6d4d9300c0b` — Native import present; static implementation inventory, not original runtime acceptance. |
| `window.opentag.numbering.next_disc_on_directory` | MAPPED_68 | `aaeda90a7158644ab6371c40786599a8` — Native import present; static implementation inventory, not original runtime acceptance. |
| `window.opentag.numbering.next_disc_on_field` | MAPPED_68 | `36d2f91d9589434ea5669328ab6b9486` — Existing bounded UTF8 native-text import; pairing/identity rules apply for numbering/export. |
| `window.opentag.numbering.reset_disc_on_field` | MAPPED_68 | `89d6949ac805e64b9eab9c7fa533a0db` — Existing bounded UTF8 native-text import; pairing/identity rules apply for numbering/export. |
| `window.opentag.numbering.reset_track_on_next_disc` | MAPPED_68 | `e242331383e5a2448bec645e3eabdd87` — Native import present; static implementation inventory, not original runtime acceptance. |
| `window.opentag.numbering.track_digits` | CURRENT_NEW_4 | `81e9cd2aac92e14e9c71cf21ae4e0510` — Parent implementing explicit false => digits1 only. True dynamic2/3/4 policy remains skipped/diagnosed under L66; no fixed-width equivalence claimed. |
| `window.opentag.numbering.track_total` | MAPPED_68 | `f4a314340ea7c044a6de766e7a2adc74` — Native import present; static implementation inventory, not original runtime acceptance. |

## Custom controls and action coverage

- `tags.mappings`: Ordered rows of tag_type/source/target; CURRENT_NEW_4 applies to the collection and all three editor inputs.
- `pages.tools.tools`: Ordered rows of name/path/parameters/for_each_file; all INDEPENDENT_L51. Default tool selects an identity in that list.
- `pages.tag-sources-freedb.servers`: Ordered rows of name/protocol/address/path/port, plus active index and current protocol/address/path/port aliases; all share the one named unresolved server collection/active identity binding.
- `pages.library.directories`: Ordered local directory paths; FOREIGN_HOST_WORKSPACE and requires source mapping before restriction true can transfer.
- `tag_panel.panel.fields`: Field layout editor reached by button; supported native usrfields.ini subset handled by existing adapter.
- `tag_panel.field_values`: lists + per-field prefer + global prefer + only_user_genres; existing usrfieldvalues.ini/genres.ini/native checkbox mappings.

Custom list buttons (new/remove/reorder/browse) mutate the listed collection; they introduce no separate portable setting. Maintenance/check/folder/credential buttons have explicit ACTION_OR_DERIVED rows. Literal declarative paths were checked against the table; no unclassified control path remains. The `/~cddb/cddb.cgi` server seed value is an endpoint literal, not a control path.

## Named unknowns and precise next evidence

- `pages.language.language`: NativeUTF8 language record1a0bfd2a930e704dac9262461ddcd9c6 uses source language filename/catalog identity (objectae5970/valueae59b0; Update1ef3b0). Target uses JSON catalog locale or system. Common locale identity conversion/installed catalog availability not established; keep current rather than infer from arbitrary filename. Named target control, not open native universe.
- `pages.messages.import_covers_choice`: Original ask-before-cover record is already imported, but persistent remembered preserve/replace answer and its codec/choice binding were not established. Do not infer it from the ask bool or no-folder-image bool.
- `pages.playlist.generator_filename`: Saved target Generate Playlists dialog memory, not Options page controls. Native corresponding generator-memory record/default/semantics not bound in this audit; do not reuse ordinary playlist filename or export partition record.
- `pages.playlist.generator_partition`: Saved target Generate Playlists dialog memory, not Options page controls. Native corresponding generator-memory record/default/semantics not bound in this audit; do not reuse ordinary playlist filename or export partition record.
- `pages.tag-sources-freedb.address`: Original freedb server controls exist (Update1e9824, native server fields and possible list identities). Target owns ordered server definitions plus active server aliases. Complete serialized server codec/order/active identity binding not established; do not import individual aliases into unrelated server list or mark them credential exclusions.
- `pages.tag-sources-freedb.path`: Original freedb server controls exist (Update1e9824, native server fields and possible list identities). Target owns ordered server definitions plus active server aliases. Complete serialized server codec/order/active identity binding not established; do not import individual aliases into unrelated server list or mark them credential exclusions.
- `pages.tag-sources-freedb.port`: Original freedb server controls exist (Update1e9824, native server fields and possible list identities). Target owns ordered server definitions plus active server aliases. Complete serialized server codec/order/active identity binding not established; do not import individual aliases into unrelated server list or mark them credential exclusions.
- `pages.tag-sources-freedb.protocol`: Original freedb server controls exist (Update1e9824, native server fields and possible list identities). Target owns ordered server definitions plus active server aliases. Complete serialized server codec/order/active identity binding not established; do not import individual aliases into unrelated server list or mark them credential exclusions.
- `pages.tag-sources-freedb.server`: Original freedb server controls exist (Update1e9824, native server fields and possible list identities). Target owns ordered server definitions plus active server aliases. Complete serialized server codec/order/active identity binding not established; do not import individual aliases into unrelated server list or mark them credential exclusions.
- `pages.tag-sources-freedb.servers`: Original freedb server controls exist (Update1e9824, native server fields and possible list identities). Target owns ordered server definitions plus active server aliases. Complete serialized server codec/order/active identity binding not established; do not import individual aliases into unrelated server list or mark them credential exclusions.
- `pages.tag-sources.web_debug`: Named target writes network pages and parser traces for web sources. Original native Online Options has no matching control in inspected resource path; original command-line/script debug is not proven persistent counterpart. Keep current; no arbitrary Debug key alias.

These unknowns are still work/dispositions to resolve before claiming the selected migration scope complete. They are not automatically exclusions. In particular, freedb server address/port/path are public portable settings, so unresolved collection identity cannot be described as a credential exclusion.

## Retained numbering and independent ownership

Track/disc false-leading-zero choices are parent current digits1 adapters. True dynamic widths remain L66; track-disabled has no target toggle; enabled underscore/special field pairs remain explicitly skipped; LastCodepage is a host chooser row index, not codepage/LCID. No omission is silently promoted to parity.

L07 owns original list/column/sort/workspace identity; L50 owns native toolbar layout/symbol/icon-family selection (not the separately confirmed portable scaling controls); L51 owns tool lists/default identities/forms; L64 owns proxy/PAC/SOCKS/provider timeout behavior; L65 owns Windows shell/context-menu behavior. OS paths/window coordinates/cache contents and private identities/credentials remain excluded without rewriting their original ledger status.

Original-generated populated fixtures, runtime original comparison, native import/restart/restore and C12/C27/L78/L79 acceptance remain deferred. This audit performed no tests and closes no ledger item. Machine-readable contracts, every row, source hashes and fresh bounded disassembly hashes: `/tmp/taggroove-l57b-target-options-crosswalk.json`.

Machine-readable local audit SHA-256: `79b59afa3727bf6e0707a0bc8b0573d2c1817355e24785d5be7b0e7b81a0f4c8`. Later implementation receipts take precedence for working-branch status.
