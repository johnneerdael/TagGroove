# Remove tag, field mappings and WAV RIFF INFO

Branch `feat/remove-mapping`. It closes gaps 2, 3 and 24 of `docs/research/remaining-gaps.md`: File → Remove tag and Ctrl+R, Options → Tags → Mapping, and WAV RIFF INFO creation. Reference pages: `menus/file`, `customization/options/tags/mpeg`, `…/tags/ape`, `…/tags/mapping`, `mapping`, `file-formats`.

## File → Remove tag (Ctrl+R)

`menus/file` says Remove tag "removes the tags specified at Options → Tags".

- **MPEG and TTA** use Options → Tags → Mpeg → Remove. **APE, MPC, WavPack, TAK and OptimFROG** use Ape, Mpc → Remove (`tag_policy::removed_kinds`). Only the checked kinds that are present are stripped.
- **Other formats** have no Remove options, so every tag kind the `file-formats` page lists is removed. This is the earlier behaviour. FLAC loses its Vorbis comments and any ID3v2 tag. WAV loses RIFF INFO and ID3v2. MP4, ASF and Matroska keep the behaviour they already had.
- A file without any of the selected kinds is not rewritten. If no kind is checked, the file is reported as an error ("No tag types are selected under Options → Tags → Mpeg → Remove") rather than skipped silently.
- **Defaults** changed. MPEG now removes ID3v1, ID3v2 and APE. Ape, Mpc removes ID3v1 and APE. Before, the only default was APE on MP3, so Ctrl+R would have left most MP3 tags in place. With the default write settings, saving behaves exactly as before: the save-time rule removes only kinds that are not written, and the defaults write ID3v1 and ID3v2 on MPEG and update an existing ID3v1 on APE-family files. ID3v2 stays unchecked for the APE family because saving would otherwise strip the hidden ID3v2 tag that `raw_streams.rs` pins as preserved. A `settings.json` that already exists keeps its saved values. None of these defaults has been checked against the original program.
- The Options group is now labelled "Remove" (it was "Remove when saving"). A note under it explains both uses.
- **Menu and key.** The item moved from Edit to File as "Remove tag…" with ⌘/Ctrl R, and the row context menu uses the same label. A capture-phase `keydown` listener in `main.ts`, built on `remove-tag.ts`, consumes every Ctrl/⌘+R, including Ctrl+Shift+R, keys pressed while typing, and keys pressed while a dialog is open. WebView2 therefore never reloads the page. A plain ⌘/Ctrl+R outside text fields, with no dialog open and Remove tag enabled, runs the command. Tauri 2.12 does not expose wry's `with_browser_accelerator_keys`, so `preventDefault` is the only available guard. Chromium lets pages cancel Ctrl+R.
- The confirmation (Options → Messages) now names what will be removed, for example "MP3, TTA: ID3v1, ID3v2 and APE / APE, MPC, …: ID3v1 and APE / Other formats: all tags".
- The operation is journaled for undo as before (`history::record("Remove tags", …)`).

## Options → Tags → Mapping

- **Storage.** The rules are stored as `TagOptions.mappings`, which is `tags.mappings` in `settings.json`, in list order. Each rule is `{tag_type, source, target}`, and `tag_type` is one of APEv2, ID3v2, MP4, VorbisComment or WMA. `TagOptions::validate` runs the engine's `validate_user_mappings`, so `save_settings`, `load` and every staged write reject unknown tag types, empty or multi-line names, and a source mapped to two targets. Many sources may map to one target, as the docs allow.
- **Threading.** `settings::read_summary`, `read_full`, `write_summary` and the parallel `save_fields_batch` pass `options.mappings`. Every desktop read and write goes through them: scan, reload, save, actions, converters, Tag→Tag, tag sources, playlists, export, columns, the filter (which evaluates the loaded entries) and undo replays. Inside tag-core, the operations that have no `rules` parameter use `options.mappings` for the entries they return. These are scans, cover add/remove/manage/transform, Remove tag, Remove tag types, Optimize, Strip MP4 chapters, and the ASF and container cover paths. Policy writes for covers also mirror fields between tag kinds under the mappings.
- **Fixes.** MPEG and APE-family writes verified the result by reading it back with the user rules, then compared it against source-named values. Every mapped MP3 save therefore failed. Verification now reads native names, as the non-policy and container paths already did.
- **UI** (`src/field-mapping.ts`). The page is Tags → Mapping (order 30, between Ape, Mpc and Advanced). It shows the rule list and New, Remove, Move up and Move down, and edits the tag type (select), Source and Target. List labels update as you type. Validation mirrors the backend and reports "Mapping N: …". Restore page defaults clears the list. Saving changed tag options reloads files that have no pending changes (the existing `optionsApplied`), so renamed fields appear immediately.

## WAV RIFF INFO

- `file-formats` lists RIFF INFO and ID3v2 as written for WAV. A WAV write now creates RIFF INFO when it lacks one and at least one field with a RIFF id has a value. Those fields are ALBUM, ARTIST, COMMENT, COPYRIGHT, GENRE, LANGUAGE, TITLE, TRACK and YEAR, from the mapping table. The new chunk is seeded from every displayed value, so it matches ID3v2 and not just the edited field. Writes that touch only fields without a RIFF id do not create an empty chunk.
- Multiple values are joined with `"; "`. Before, saving a multi-valued ARTIST to a WAV that had RIFF INFO failed with "RIFF ARTIST supports one text value".
- Fields without a RIFF id are no longer lost. They are stored in the ID3v2 chunk, which every WAV write updates, and they read back through it; tests cover this. Unknown INFO ids read from a file (for example `IENG`) round-trip. Other four-letter names, such as `MOOD` or `SPOT`, are no longer written as bogus RIFF chunks: only `I` plus three uppercase letters or digits count as an INFO id.

## Tests

- `crates/tag-core/tests/remove_mapping.rs` (7 tests):
  - Remove tag with selected, absent and empty MPEG kinds, and the defaults.
  - WAV Remove tag strips both kinds.
  - RIFF INFO creation, seeding, joining, update and clearing.
  - Unmapped INFO ids round-trip and other names stay in ID3v2.
  - An ID3v2 rule through the MPEG policy, with a raw read showing COMPOSER.
  - A Vorbis rule applied to add-cover, scan and Remove tag results without a `rules` argument (ffmpeg FLAC).
  - `TagOptions` validation and serde.
- `in_place_tags.rs`: the WAV in-place test now edits COMPOSER, because a TITLE edit now also updates the new INFO chunk.
- `src-tauri/src/settings.rs`: a mappings roundtrip, and an invalid mapping falls back to defaults with a warning.
- `tests/remove-mapping.test.mjs` (5 tests): Ctrl/⌘+R decisions (run, suppress or ignore, including a non-Latin layout through `code`), the prompt text, the defaults, rule normalisation and validation, page placement, render and validation through `validateOptions`.
- `tests/fixtures/default-settings.json` is updated, and both default tests (Rust and frontend) use it.
- Native CI suite `tests/e2e/remove-mapping.js`, with `remove-mapping-fixtures.sh`, is added to the `native-e2e` job. It checks:
  - The File menu item.
  - Ctrl+R with only ID3v1 selected, confirming the key event is consumed and Undo restores the tags.
  - Ctrl+R with the defaults.
  - Ctrl+R in the filter field is consumed and removes nothing.
  - A mapping rule is added in the dialog, a conflicting rule is rejected, the FLAC reloads with YEAR, and the Tag Panel shows it.
  - Saving YEAR writes DATE.

  It has not been run locally (CI only).

Local results: `cargo test --workspace`, `cargo clippy --workspace --all-targets -D warnings` (1.95; CI uses a newer clippy), `cargo fmt`, `tsc --noEmit` and `node --test tests/*.test.mjs` all pass.

## Known gaps

- None of the Remove defaults, the RIFF separator `"; "` or the choice of which kinds are seeded has been checked against Mp3tag 3.36.1.
- An ID3v2 chunk created for a WAV that had only RIFF INFO receives only the edited fields, as before. The INFO values still appear because secondary tags add the fields the primary lacks.
- Android provider-document reloads (`documents.rs`) still read with `tag_core::read_entry`, so without options or mappings. This is unchanged and was left alone on purpose.
- AAC (ADTS) has no tag policy, so Remove tag strips all of its kinds (ID3v2; APE is not supported by lofty).
- WebView2 reload suppression relies on `preventDefault`, which is not verifiable on the Linux CI runner. Windows behaviour needs a manual check.
