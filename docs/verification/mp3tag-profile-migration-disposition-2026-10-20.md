# Bounded profile-migration P1 disposition

Implementation head: `4baef8b`. This is a **local implementation** disposition;
the batch PR, exact-final-head GitHub Codex code review, required Linux CI and
merge have not happened. Original/native/runtime acceptance stays separate.

## Scope and evidence

The existing [priority scope](../handover/PARITY-PRIORITIES.md)
says: “Ship a bounded confirmed mapping rather than all undocumented settings;
preserve existing import/recovery and opaque data.” This audit follows that
scope. It does not substitute a native record count or target-control count
for a requirement review, and it does not claim universal profile parity.

The source review covered every row of the finite Options crosswalk and every
named remaining original choice below. It found two concrete implementation
defects: the effective language page was not updated, and the freedb relative
path preview disagreed with the provider. Both were reproduced before repair.
The corrected source updates both language stores and normalizes preview only;
authored server fields remain unchanged. Final local review has no findings.

Final evidence: 29 settings tests, the combined twenty-binding exact-restore
case, application library/test Clippy with warnings denied, format/extraction
and ledger checks pass. Earlier affected checks in this same slice: 43 profile
cases plus one opt-in controlled child, 75 frontend cases, TypeScript and the
strict guide build. These are headless/filesystem checks, not application or
original Windows UI replay. The [implementation receipt](mp3tag-options-bindings-migration-2026-10-20.md)
contains the source bindings and qualification boundaries.

The row dispositions below are 110 implemented supported counterparts,
21 host/private/derived/non-equivalent controls and five independently owned
rows. Multiple rows can share one native record. Those numbers are inventories,
not a feature-completion percentage. The two formerly pending repairs account
for the language row and six aliases of one freedb dataset.

## Target-row disposition

| Target row | Disposition | Reason |
| --- | --- | --- |
| `appearance / window.opentag.appearance` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `button:check-updates` | HOST_PRIVATE_OR_NON_EQUIVALENT | Runs product-specific update check, not saved setting. |
| `button:customize-tag-panel` | HOST_PRIVATE_OR_NON_EQUIVALENT | Editor entrypoint; fields/layout covered by file adapter rows. |
| `button:edit-field-values` | HOST_PRIVATE_OR_NON_EQUIVALENT | Editor entrypoint; lists/preferences covered by file adapter rows. |
| `button:open-languages-folder` | HOST_PRIVATE_OR_NON_EQUIVALENT | Opens current app catalog folder, not portable setting. |
| `button:tag-source-keys` | HOST_PRIVATE_OR_NON_EQUIVALENT | Private credential manager; explicitly excluded, never read/import credentials. |
| `custom:language-warnings` | HOST_PRIVATE_OR_NON_EQUIVALENT | Derived local catalog diagnostics, not saved setting. |
| `custom:library-maintenance` | HOST_PRIVATE_OR_NON_EQUIVALENT | Target-local cache cleanup/clear jobs; no source cache transfer. |
| `custom:local-freedb-index` | HOST_PRIVATE_OR_NON_EQUIVALENT | Build/cancel target-local text index, not portable source state. |
| `directories.subfolders` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `exports.active_configuration` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `exports.preferences.append` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `exports.preferences.filename` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `exports.preferences.partition enabled` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `exports.preferences.partition_memory` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `files.folder_separator` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `files.restrict_extensions` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `general.natural_sort` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `general.show_directory_in_title` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.appearance.grid` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.appearance.toolbar_scale` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.appearance.ui_scale` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.directories-startup.startup` | IMPLEMENTED_SUPPORTED_SUBSET | Native empty/previous modes implemented, with previous explicitly using target-local history. Source specific-folder mode2 is not target first-favorite; its host-path identity is deliberately excluded. |
| `pages.directories.cover_art` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.export.dont_create_directories` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.export.show_dialog` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.export.write_bom` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.files.associated_files` | HOST_PRIVATE_OR_NON_EQUIVALENT | Target is a narrow opt-in for same-basename .lrc/.cdg companions with specified operations, not a source-bound generic original file-association preference. Parent Copy work is separately owned L22; no guessed CFG alias. |
| `pages.general.auto_select_next` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.general.autocomplete` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.general.keep_moved` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.general.minimize_to_tray` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.general.select_added` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.general.tray_icon` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.language.language` | IMPLEMENTED_SUPPORTED_SUBSET | Imported language updates both the effective Options page and window preference; all four supported choices and exact restore pass. |
| `pages.library.directories` | HOST_PRIVATE_OR_NON_EQUIVALENT | These controls require actual local filesystem directories. Native foreign Windows paths/cache contents cannot be adopted as current local directories; preserve current values. Native library restriction true must remain coupled to directory admission. |
| `pages.library.enabled` | IMPLEMENTED_SUPPORTED_SUBSET | Implemented explicit disable, and enable only with an explicit admitted unrestricted source policy. An unknown/host-folder restriction cannot be repurposed against an unrelated target folder list. |
| `pages.library.restrict` | IMPLEMENTED_SUPPORTED_SUBSET | Explicit false is implemented. Source true requires the original included-host-directory identity; current migration excludes foreign host folders rather than treating the target retained list as equivalent. This is a documented host-data boundary, not an unknown portable bool codec. |
| `pages.messages.action_warnings` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.messages.after_change` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.messages.after_delete` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.messages.after_save` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.messages.confirm_cut_tags` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.messages.confirm_delete_files` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.messages.confirm_export` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.messages.confirm_import_covers` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.messages.confirm_paste_tags` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.messages.confirm_remove_tags` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.messages.import_covers_choice` | IMPLEMENTED_SUPPORTED_SUBSET | Native remembered answer bool now maps preserve/replace independently of ask flag. |
| `pages.messages.unsaved_changes` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.networking.proxy` | INDEPENDENT_LEDGER_ITEM | Network/proxy semantics remain independently owned. A provider-specific original CDDB timeout is not an equivalent global target timeout; no blind alias is justified. |
| `pages.networking.proxy_url` | INDEPENDENT_LEDGER_ITEM | Network/proxy semantics remain independently owned. A provider-specific original CDDB timeout is not an equivalent global target timeout; no blind alias is justified. |
| `pages.networking.timeout` | INDEPENDENT_LEDGER_ITEM | Network/proxy semantics remain independently owned. A provider-specific original CDDB timeout is not an equivalent global target timeout; no blind alias is justified. |
| `pages.playlist.auto_create` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.playlist.confirm_filename` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.playlist.dont_create_directories` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.playlist.extended` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.playlist.extinf_format` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.playlist.filename` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.playlist.generator_filename` | IMPLEMENTED_SUPPORTED_SUBSET | Independent native UTF8 memory imports now exist, including admitted source ordinary-filename fallback for explicit empty generator filename and preserved empty partition. |
| `pages.playlist.generator_partition` | IMPLEMENTED_SUPPORTED_SUBSET | Independent native UTF8 memory imports now exist, including admitted source ordinary-filename fallback for explicit empty generator filename and preserved empty partition. |
| `pages.playlist.relative` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.playlist.write_bom` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.tag-sources-freedb.address` | IMPLEMENTED_SUPPORTED_SUBSET | Atomic dataset and all six aliases transfer together; reviewed preview now trims address/path and adds the provider leading slash without changing saved fields. The relative-path regression and exact restore pass. |
| `pages.tag-sources-freedb.comment_disc_id` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.tag-sources-freedb.disc_id_field` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.tag-sources-freedb.email` | HOST_PRIVATE_OR_NON_EQUIVALENT | Provider handshake identity/personal contact, not public output behavior. Preserve destination identity; no actual user email/credential values read. |
| `pages.tag-sources-freedb.path` | IMPLEMENTED_SUPPORTED_SUBSET | Atomic dataset and all six aliases transfer together; reviewed preview now trims address/path and adds the provider leading slash without changing saved fields. The relative-path regression and exact restore pass. |
| `pages.tag-sources-freedb.port` | IMPLEMENTED_SUPPORTED_SUBSET | Atomic dataset and all six aliases transfer together; reviewed preview now trims address/path and adds the provider leading slash without changing saved fields. The relative-path regression and exact restore pass. |
| `pages.tag-sources-freedb.protocol` | IMPLEMENTED_SUPPORTED_SUBSET | Atomic dataset and all six aliases transfer together; reviewed preview now trims address/path and adds the provider leading slash without changing saved fields. The relative-path regression and exact restore pass. |
| `pages.tag-sources-freedb.server` | IMPLEMENTED_SUPPORTED_SUBSET | Atomic dataset and all six aliases transfer together; reviewed preview now trims address/path and adds the provider leading slash without changing saved fields. The relative-path regression and exact restore pass. |
| `pages.tag-sources-freedb.servers` | IMPLEMENTED_SUPPORTED_SUBSET | Atomic dataset and all six aliases transfer together; reviewed preview now trims address/path and adds the provider leading slash without changing saved fields. The relative-path regression and exact restore pass. |
| `pages.tag-sources-local-freedb.directory` | HOST_PRIVATE_OR_NON_EQUIVALENT | These controls require actual local filesystem directories. Native foreign Windows paths/cache contents cannot be adopted as current local directories; preserve current values. Native library restriction true must remain coupled to directory admission. |
| `pages.tag-sources-online.deezer_cover_size` | HOST_PRIVATE_OR_NON_EQUIVALENT | Target direct-API provider option. Original native Online Options bound here exposes cover filename/history and MusicBrainz size; no corresponding product-wide original setting established. A similarly named web-source script field/settings file is not this target direct-API preference. |
| `pages.tag-sources-online.fpcalc` | HOST_PRIVATE_OR_NON_EQUIVALENT | Optional local executable override. Do not import foreign executable path or execute anything; built-in target fingerprinting remains current. |
| `pages.tag-sources-online.itunes_country` | HOST_PRIVATE_OR_NON_EQUIVALENT | Target direct-API provider option. Original native Online Options bound here exposes cover filename/history and MusicBrainz size; no corresponding product-wide original setting established. A similarly named web-source script field/settings file is not this target direct-API preference. |
| `pages.tag-sources-online.itunes_cover_size` | HOST_PRIVATE_OR_NON_EQUIVALENT | Target direct-API provider option. Original native Online Options bound here exposes cover filename/history and MusicBrainz size; no corresponding product-wide original setting established. A similarly named web-source script field/settings file is not this target direct-API preference. |
| `pages.tag-sources-online.lastfm_genres` | HOST_PRIVATE_OR_NON_EQUIVALENT | Target direct-API provider option. Original native Online Options bound here exposes cover filename/history and MusicBrainz size; no corresponding product-wide original setting established. A similarly named web-source script field/settings file is not this target direct-API preference. |
| `pages.tag-sources-online.lrclib_synced` | HOST_PRIVATE_OR_NON_EQUIVALENT | Target direct-API provider option. Original native Online Options bound here exposes cover filename/history and MusicBrainz size; no corresponding product-wide original setting established. A similarly named web-source script field/settings file is not this target direct-API preference. |
| `pages.tag-sources.cover_filename` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.tag-sources.musicbrainz_cover_size` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.tag-sources.web_debug` | HOST_PRIVATE_OR_NON_EQUIVALENT | Target forced global trace is a port extension. Inspected native Debug/DebugWriteInput are per-script commands, and FLOG1 is command-line logging. Preserve target global preference; script-authored commands remain source-script scope. |
| `pages.tags.extended_save_mode` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.tags.no_folder_image` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.tags.save_on_move` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `pages.tools.default_tool` | INDEPENDENT_LEDGER_ITEM | Original tool grammar, executable paths, ordered import and default identity remain independent L51 scope. |
| `pages.tools.tools` | INDEPENDENT_LEDGER_ITEM | Original tool grammar, executable paths, ordered import and default identity remain independent L51 scope. |
| `pages.updates.check_on_startup` | IMPLEMENTED_SUPPORTED_SUBSET | Parent explicitly accepted generic preferences: source mode0/1/2 and notes bool now map actual target controls; target endpoint and product version state remain unchanged. |
| `pages.updates.include_beta` | IMPLEMENTED_SUPPORTED_SUBSET | Parent explicitly accepted generic preferences: source mode0/1/2 and notes bool now map actual target controls; target endpoint and product version state remain unchanged. |
| `pages.updates.last_version` | HOST_PRIVATE_OR_NON_EQUIVALENT | Preserve target updater consent/channel/version context. Native preference refers to Mp3tag updates; target contacts TagGroove GitHub releases and its own notes. A source product opt-in is not automatically target product opt-in. This is an explicit proposed scope disposition, not absence of an original counterpart. |
| `pages.updates.release_notes` | IMPLEMENTED_SUPPORTED_SUBSET | Parent explicitly accepted generic preferences: source mode0/1/2 and notes bool now map actual target controls; target endpoint and product version state remain unchanged. |
| `tag_panel.field_values.lists` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tag_panel.field_values.only_user_genres` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tag_panel.field_values.prefer_tag_fields` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tag_panel.field_values.prefer_tag_fields.GENRE` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tag_panel.field_values.prefer_tag_values` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tag_panel.panel.fields` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tag_panel.panel.show_panel` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.advanced.id3v2_comment_language` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.advanced.list_chapters` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.advanced.matroska_video_mapping` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.advanced.mp4_single_atom` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.advanced.popularimeter_ratings` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.advanced.reuse_id3v2_frames` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.advanced.reuse_mp4_atoms` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.ape.read_ape` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.ape.read_id3v1` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.ape.remove_ape` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.ape.remove_id3v1` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.ape.remove_id3v2` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.ape.write_ape` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.ape.write_id3v1` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.mappings` | IMPLEMENTED_SUPPORTED_SUBSET | Native ordered list adapter exists, all five exact native tag types match target, source three fields are applied and reserved/extension payloads reported. No inferred ASF alias or generic list framing shortcut. |
| `tags.mpeg.id3v2_format` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.mpeg.id3v2_only_if_needed` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.mpeg.read_ape` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.mpeg.read_id3v1` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.mpeg.read_id3v2` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.mpeg.remove_ape` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.mpeg.remove_id3v1` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.mpeg.remove_id3v2` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.mpeg.write_ape` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.mpeg.write_id3v1` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.mpeg.write_id3v2` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `tags.preserve_modified_time` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `version` | HOST_PRIVATE_OR_NON_EQUIVALENT | Target JSON schema version, not a migrated original preference. Never import foreign schema version. |
| `window.opentag.hideFilter` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `window.opentag.numbering.disc_digits` | IMPLEMENTED_SUPPORTED_SUBSET | Native false maps to digits1. Native true uses dynamic widths and is explicitly diagnosed; no fixed target width is equivalent. See named original-choice disposition for why this does not require another exact migration adapter in the bounded scope. |
| `window.opentag.numbering.disc_enabled` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `window.opentag.numbering.disc_total` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `window.opentag.numbering.next_disc_on_directory` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `window.opentag.numbering.next_disc_on_field` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `window.opentag.numbering.reset_disc_on_field` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `window.opentag.numbering.reset_track_on_next_disc` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |
| `window.opentag.numbering.track_digits` | IMPLEMENTED_SUPPORTED_SUBSET | Native false maps to digits1. Native true uses dynamic widths and is explicitly diagnosed; no fixed target width is equivalent. See named original-choice disposition for why this does not require another exact migration adapter in the bounded scope. |
| `window.opentag.numbering.track_total` | IMPLEMENTED_SUPPORTED_SUBSET | Current source has the confirmed adapter and target application path. Admission, missing-record preservation and stated native qualification remain part of the contract. |

## Original choices that remain explicitly unsupported or qualified

### Native track/disc leading-zero true

Supported false subset is sufficient for this bounded migration adapter because current target fields only encode a fixed minimum width. True requires changing the numbering planner/runtime policy, not adding an exact preference mapping. Do not call it supported or move the policy gap to a completed row.

### Native track-enabled false

No exact existing target toggle exists: planner always emits TRACK. This remains an explicit source capability limitation; adding track-disable would expand operation functionality beyond bounded confirmed adapters. Do not pretend disc_enabled or another bool preserves it. No unrelated ledger owner is invented.

### Enabled native underscore/special field pair, including _DIRECTORY

Ordinary paired fields and explicit disable are implemented. The planner compares tag fields only; it does not supply native path pseudo-field semantics. Rejecting all underscore names is a conservative documented admission boundary, not proof that every underscore spelling is a path expression. It is sufficient for the approved bounded ordinary-field subset, but not complete original field-wizard parity.

### LastCodepage chooser row

The source stores a Windows locale chooser row index, not codepage/LCID. Existing target codepage memory cannot represent that foreign identity from this record alone. Excluding it is a justified host-state boundary; no P1 mapping can be created by numeric casting. Original chooser/runtime context is deferred, and broader encoding support remains L56b.

### Restricted library included folders / source specific startup directory

Portable explicit false restriction and empty/previous startup choices are implemented. Original host-directory lists and startup paths lack target identity translation; current target folders/favorite ordering cannot stand in for source identities. Deliberately preserved with diagnostics, not remaining portable-codec work.

### Original unavailable/custom language catalogs

Empty/system and three pinned stock identities are implemented with the effective-store correction verified. Unsupported native catalog contents cannot be installed as target JSON catalogs or treated as available solely by stem. Extra independently authored locales remain L63 rather than a missing exact L57b scalar conversion.

### Empty/incomplete/invalid freedb list/index pair

Valid nonempty source datasets with explicit in-range index are implemented. Target empty list restores built-ins, so an exact empty native clear has no current equivalent. A row index without its source list does not identify a target provider. Keep whole current dataset with warning rather than silently map defaults.

### Host-specific file/directory expressions and provider credentials/identity

Admitted portable expressions and generic settings are implemented. Original foreign absolute paths, provider grants, email identity and credentials are outside settings transfer and are not silently copied.

### Toolbar/column/tool/network/shell settings outside admitted adapters

Keep each named existing ledger owner. Do not claim L57b delivers their parsing, identities, host launch behavior, proxy authentication/PAC/SOCKS or unbound original settings.

### format.ini / format2.ini / mp3tag.dat and undiscovered native records

Only filename literals/legacy conversion paths were seen; no new live modern portable counterpart was proved. This audit cannot assert absence or complete native-universe coverage. Under the approved bounded-confirmed-mapping scope, lack of a newly proved counterpart is not itself a concrete P1 adapter defect. Retain the limitation and reopen if populated source evidence establishes one; do not relabel an actual proved missing setting as P3 to close it.

## Completion boundary

No further concrete portable adapter remains proved missing within the approved
bounded P1 mapping scope after these repairs. L57b joins the locally implemented
batch; its intake identity stays until a covering PR exists. Preserve every
unsupported choice above and the independent L07/L50/L51/L64/L65 scopes. Adding
a new original operation is distinct from mapping an existing counterpart.
Do not call these unsupported choices implemented, invent another ledger owner,
or demote a newly proved portable omission to obtain closure.

The original object registry is not exhaustively inventoried. Static PE caller/
serializer evidence and stock language metadata do not establish populated
original-profile differential results. `format.ini`, `format2.ini`,
`mp3tag.dat`, old filename literals and undiscovered original records do not
become certified by this audit. Reopen a concrete mapping if source evidence
establishes one. C12/C27/L78/L79 retain original/native import, restart, restore
and platform qualification. This does not complete the whole P1 goal.

Frozen read-only audit JSON SHA-256: `755f59debbfe962ddc019a91490e1abc9ea2f5e326cde78249dabfcb29da1920`.
