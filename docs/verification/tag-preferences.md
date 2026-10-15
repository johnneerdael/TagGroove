# Options dialog and tag preferences

Scope: File → Options… and the Tags → Mpeg, Ape/Mpc and Advanced pages, plus the General, Files, Directories and Appearance settings that change behaviour. References: `customization/options/*`, `customization/options/tags/{mpeg,ape,advanced}`, `keyboard`.

## Implemented

**Settings store** (`src-tauri/src/settings.rs`). Settings are stored as `settings.json` in the app config directory with `version: 1`. Missing files load the defaults. A partial file is filled in from the defaults. A corrupt file, a file from a newer version, or a file with invalid values also loads the defaults, and the dialog explains why; the next save replaces the file. `save_settings` validates the settings, writes atomically, and runs under the file-operation lock, so a batch never sees the options change mid-run. Pages registered by other features store their values under `pages.<id>` as JSON objects, so no backend change is needed.

**Explicit options in tag-core** (`crates/tag-core/src/preferences.rs`, `tag_policy.rs`). `TagOptions` is passed to `*_with_options` variants of read, scan, write, remove-tags, add/remove cover and the cover actions. tag-core has no global state. The older functions use `TagOptions::default()`, which uses the Mp3tag defaults. In the Tauri layer, every desktop command routes tag I/O through the current settings: save, remove tags, covers, reload, previews, rename, export, actions and converters.

**Mpeg (MP1/MP2/MP3)**
- *Read:* only one tag kind is displayed. It is the highest-priority kind that is enabled and present: APE, then ID3v2, then ID3v1. Secondary tags are no longer merged in (this resolves the contradiction recorded in FEATURE-INVENTORY). `_TAG_READ` reports the displayed kind and `_TAG` still lists every kind present.
- *Write:* ID3v1, ID3v2 and APEv2 are independent switches.
  - ID3v2 can be written as ID3v2.3 UTF-16, ID3v2.3 ISO-8859-1 (characters outside Latin-1 become `?`) or ID3v2.4 UTF-8.
  - Converting between revisions re-expresses the documented version-specific frames. TDRC ↔ TYER/TDAT/TIME, TDOR ↔ TORY and TIPL ↔ IPLS are converted. Fields without a native v2.3 frame, such as MOOD/TMOO, are converted between their native frame and TXXX.
  - "ID3v2 only if ID3v1 too small" writes ID3v2 only for an existing ID3v2 tag, values that do not fit ID3v1 (length, Latin-1, extra fields, multiple values), a user-defined genre, or artwork.
  - Writing APEv2 upgrades an APEv1 tag to v2 and also updates an existing ID3v1 tag.
- *Remove:* File → Remove tag (Ctrl+R) strips the selected kinds ([remove-mapping](remove-mapping.md)). Saving also strips a selected kind that is not being written.

**Ape, Mpc (Monkey's Audio, Musepack, WavPack):** the same model with Read ID3v1/APE (APE has priority), Write ID3v1/APEv2, and Remove ID3v1/ID3v2/APE.

**Saving model.** The displayed tag is patched in place, so unknown native items survive. Every other kind being written receives the full displayed field set, and its fields that are absent from that set are removed, so all written kinds agree. A newly created kind starts from all displayed values and artwork. Covers follow the same write and remove sets.

**Advanced:** the ID3v2 comment language (ISO-639-2) is used for new COMM frames. "Always display POPULARIMETER values" shows POPM as `POPULARIMETER` instead of `RATING MM`/`RATING WMP`.

**Other pages**
- General: natural sorting, and the current directory in the window title.
- Files: "Restrict incoming files to" (`*.mp3;*.flac`, applies to folders), and "Preserve timestamps of modified files".
- Directories: read subfolders.
- Appearance: the existing System/Light/Dark choice, which is also still available in the title bar.

**Dialog** (`src/options-ui.ts`).
- Built from declarative controls. Other features call `registerOptionsPage({id, label, parent?, order?, controls, validate?})`.
- Uses a tree with arrow, Home and End keys and roving tabindex, OK/Cancel and per-page "Restore page defaults". Validation names the page that needs attention and switches to it.
- Dependent controls are disabled. For example, "only if too small" needs both ID3v1 and ID3v2, and the version needs ID3v2.
- In the compact layout (`max-width:1000px`) the tree becomes a page `<select>`.
- Checked in Chrome through a temporary harness page: Light at 1200×800, Dark at 400×780 with a warning, keyboard navigation, validation, and the save callback. When tag read options change, saving reloads loaded files that have no pending edits.

**Shortcut decision.** The keyboard reference assigns Ctrl+O to Options and Ctrl+D to Change directory. Ctrl/Cmd+O now opens Options. Open folder (or Add files on provider platforms) moved to Ctrl/Cmd+D, which is ignored while typing in a text field. The menu labels show the new shortcuts.

**Fixes found along the way**
- Lofty's strict parser rejected ID3v1 tags with an empty year (`\0\0\0\0`). Common taggers write that, and lofty writes it too. Only that error is now retried in relaxed mode.
- Removing the last frame of an ID3v2.3 tag failed with "Invalid ID3 serialization header". Such tags are now removed.

## Tests

- `crates/tag-core/tests/tag_policy.rs` (12 tests), using independently generated MPEG frames, with packet hashes when ffmpeg is present:
  - priority display
  - the default write: ID3v1 plus UTF-16 ID3v2.3, with APE stripped
  - v2.4 → v2.3 → v2.4 conversion, with dates and MOOD
  - Latin-1 mode
  - a 6×2 write/remove matrix that asserts tag kinds, readback values and byte-identical audio
  - "only if too small"
  - APEv1 upgrade plus the ID3v1 update
  - covers across written kinds
  - comment language and POPULARIMETER display
  - preserved mtime
  - empty ID3v1 year and restricted extensions
  - ffmpeg-encoded MP3 for every ID3v2 format, and WavPack Ape/Mpc policies, with unchanged decoder packets
- `preferences.rs` unit tests (2) and `settings.rs` tests (4): roundtrip, partial, corrupt and newer files, validation, extension syntax, and defaults equal to `tests/fixtures/default-settings.json`.
- `tests/options-ui.test.mjs` (8): the same defaults fixture, page tree and registration, path storage, validation-to-page, extension parsing, control rendering and enabling, collator and title.
- `metadata.rs` native-APE preservation now enables APEv2 writing explicitly, because the Mp3tag default removes APE from MP3s.
- Totals: 109 frontend tests pass (baseline 101) and 297 Rust tests pass (baseline 279). `tsc --noEmit`, `vite build` and `cargo clippy --workspace --all-targets -D warnings` are clean.

## Known gaps and risks

- **Defaults are not proven against the original.** The docs do not state default values. The defaults used are: write ID3v1 + ID3v2.3 UTF-16; read all kinds; remove every kind on MP3 and ID3v1 + APE on Ape/Mpc (changed 2026-10-15 so Remove tag removes everything by default; under the default write settings saving is unaffected); APEv2-only for Ape/Mpc; natural sort on; title shows the directory. These follow common Mp3tag configurations, not a captured original configuration. Removing APE on MP3 is kept as the default (decided 2026-10-14) because APE is read first: a stale APE tag would otherwise hide every ID3v2 edit on reload. To avoid losing gain data, `REPLAYGAIN_*` and `MP3GAIN_*` fields of a removed APE tag are carried into ID3v2 user frames (TXXX) unless ID3v2 already has them; a displayed APE tag's fields are mirrored into ID3v2 anyway. MP3Gain itself only reads its undo data from APE, so `MP3GAIN_UNDO` survives as text but is no longer usable by MP3Gain.
- **Mirroring is a deliberate approximation.** Other written kinds mirror the displayed text fields, but they keep their own artwork unless artwork is edited or the kind is new. Mp3tag's in-memory single-tag model may also replace that artwork.
- **ID3v2 on Ape/Mpc files** is not displayed: the docs list only ID3v1/APE reads. It is removable.
- **Not implemented (listed rather than shown as dead controls):**
  - Tags page: autosave on selection change, extended-tags save mode, folder image as cover.
  - Advanced: list chapters as files, Matroska mapping, single MP4 atom for multiple values, reuse unmodified MP4 atoms and ID3v2 frames (current writes always patch only edited fields, which matches "reuse" being on).
  - General: auto-select next file, auto-completion, select files automatically, keep moved files, notification area.
  - Files: folder separator replacement on rename.
  - Directories: favorite, startup and cover art directories.
  - Appearance: symbols, toolbar/UI scaling, grid lines.
  - Pages without behaviour here: Messages, Library, Tag Sources, Field Values, Tag Panel, Playlist, Export, Tools, Networking, Language, Updates.
- **Platform and command limits**
  - WAV/AIFF ID3v2 chunks keep their existing revision. The docs scope the Mpeg page to MP3/TTA, and TTA is not supported by the backend.
  - Android provider-document reloads still read with default options, because the mobile code was intentionally left untouched.
- **Original runtime.** No differential runs against Mp3tag 3.36.1 were done. Exact ID3v1 truncation and joining of multiple values (the first value is used), and the exact encoding choice for existing Latin-1 frames in UTF-16 mode (they are kept), are unverified.
