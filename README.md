<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/OpenTag-Brand-Kit/03-banners/opentag-readme-dark-1600x480.svg">
  <img src="assets/brand/OpenTag-Brand-Kit/03-banners/opentag-readme-light-1600x480.svg" alt="OpenTag — Tag, Organize, Enjoy" width="100%">
</picture>

# OpenTag

[![Build and test](https://github.com/johnneerdael/opentag/actions/workflows/build.yml/badge.svg)](https://github.com/johnneerdael/opentag/actions/workflows/build.yml)
[![Releases](https://img.shields.io/github/v/release/johnneerdael/opentag?include_prereleases&color=FF7800&label=Download)](https://github.com/johnneerdael/opentag/releases)
[![MIT License](https://img.shields.io/badge/License-MIT-101922)](LICENSE)

**Tag, Organize, Enjoy.** An open source music tag editor for Windows, macOS and Linux, with Android and iOS builds in progress.

OpenTag is an independently written reconstruction of Mp3tag 3.36.1, built with Tauri 2 and a Rust tagging engine. The goal is feature and performance parity with Mp3tag's documented behaviour. None of Mp3tag's code, text, templates or source scripts is included.

## Status

OpenTag is in **0.x prerelease**. The desktop editor covers most of Mp3tag's documented features, and every change runs through unit tests and headless UI suites on Linux, macOS and Windows. Nothing has been checked against the original program yet, so no area is certified as exact parity.

- [Parity ledger](docs/PARITY.md): what each area does today, with links to its evidence.
- [Remaining gaps](docs/research/remaining-gaps.md): what is still open, by area and impact.
- [Performance](docs/PERFORMANCE.md): measurements so far.

In progress on branches, not yet in `main`: UI translations, more metadata sources (iTunes, Deezer, LRCLIB, Cover Art Archive, AcoustID, Last.fm, fanart.tv), gnudb compliance with remote text search, and wider compatibility with community `.src` web sources.

## Features

**File list and tag panel**
- Loads folders, files, playlists and cue sheets in parallel with progress and Cancel; drop to replace the list, Ctrl+drop to add.
- Configurable columns from format strings, multi-key and natural sort, View → Sort by, random order, manual order (Alt+arrows, Alt+drag), auto-size, inline editing (F2 or slow click), Find, filter with history. Mp3tag `columns.ini` import.
- Tag panel with your own fields, mnemonics, drop-down lists, auto-completion, default values, `_DIRECTORY` and `_SEPARATOR`. Docks left, right, above or below the list, or floats. Mp3tag tag panel import.
- Extended Tags with multi-values, cover art, `<<`/`>>` navigation, a field-name drop-down and field cut/copy/paste.
- Undo and redo for every write, copy/cut/paste tags (with artwork), save on selection change, Repeat.

**Formats**
- MP3/MP2/MP1, AAC, FLAC, MP4/M4A/M4B/M4V (ALAC), Ogg Vorbis, Opus, Speex, WAV, AIFF, WavPack, Monkey's Audio, Musepack, WMA/ASF, DSF, DFF, Matroska/WebM, TTA, TAK and OptimFROG.
- ID3v1, ID3v2.3, ID3v2.4, APEv1/v2, Vorbis comments, MP4 atoms, ASF attributes, RIFF INFO and Matroska tags, mapped as in Mp3tag's mapping table, plus user field mappings (Options → Tags → Mapping).
- Read, write and remove choices per tag kind. Writes go to a verified staging copy; tags are rewritten in place inside existing padding where possible.
- Chapters: MP4 (Nero and QuickTime), Matroska and ID3v2 `CHAP`, optionally listed as separate rows with their own fields; cue sheet tracks.

**Scripting, converters and actions**
- All 64 documented scripting functions, the documented placeholders (a few only for some formats), filter syntax, and regular expressions through a vendored Boost.Regex.
- Converters: Tag → Filename, Filename → Tag, Filename → Filename, Text File → Tag, Tag → Tag, and auto-numbering, each with a preview.
- All 18 action types, action groups with submenus and mnemonics, Actions (Quick), `.mta` import and export.

**Export, playlists and covers**
- Export manager with editable MTE templates (loops, variables, totals, encodings, CRC/MD5), several bundled templates, per-folder export.
- M3U/M3U8 playlists, per-folder playlists, Generate Playlists, automatic playlists after save or rename.
- Cover window and Manage Covers: add, replace, reorder, adjust, extract, clipboard, drag in and out, folder image fallback.

**Tag sources**
- MusicBrainz and Discogs search, release review with field selection, track matching and covers.
- freedb: CDDB lookups by disc ID, with a server list, and local freedb directories with a text index. gnudb, the default server, refuses OpenTag until the application is registered there; that work is in progress.
- User-installed Mp3tag-compatible `.src` web sources through a clean-room interpreter (Tag Sources → Install tag sources…).

**Everything else**
- Options pages modelled on Mp3tag's, including Library (an SQLite cache that loads unchanged files without reading them), Tools, Messages, Networking and Updates; configuration backup and restore.
- Mp3tag's keyboard reference: all 97 rows are bound. ⌘ replaces Ctrl on macOS, and Option keys still type characters in text fields.
- File operations (copy, move, delete, rename with case-only support), favorite directories, command line switches (`/fp:`, `/fn:`, `/add`) with single-instance forwarding, macOS "Open With".
- Light, Dark and System appearance; notification area icon on Windows and Linux.

**Platforms**: Windows x64, macOS (Apple Silicon and Intel) and Linux (x64 and arm64) are the main targets. Android has document access and save-back for provider files; iOS builds for the simulator only. See the ledger for what each mobile build has been checked on.

## Download

Get installers from [GitHub Releases](https://github.com/johnneerdael/opentag/releases). Every release is marked **prerelease** while the [parity ledger](docs/PARITY.md) is open. Assets name their operating system and architecture and come with a checksum list. Windows builds are unsigned and macOS builds are ad-hoc signed, not notarized. See [release automation](docs/RELEASING.md) for targets, signing and publishing.

## Develop

Install the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS, current stable Rust, a C++17 compiler and Node.js 22. FFmpeg is needed for the encoded-format fixture tests. The regex bridge vendors Boost.Regex under the Boost Software License.

```sh
npm ci
npm run tauri dev        # run the app
npm run build            # type-check and bundle the frontend
npm test                 # node UI tests, then cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
npm run tauri build      # release packages
```

On macOS with Homebrew `binutils` ahead of the system tools in `PATH`, GNU `ar` builds a regex-bridge archive that Apple's linker rejects. Run cargo and npm commands with the system archiver, for example `AR=/usr/bin/ar npm test`.

Native UI suites (`tests/e2e/*.js`) open real app windows and drive them with scripted DOM events. The `native-e2e` job in [build.yml](.github/workflows/build.yml) runs them headless on Linux (under xvfb), macOS and Windows. `scripts/e2e-native.sh` refuses to run outside CI unless `OPENTAG_E2E_ALLOW_LOCAL=1` is set, because the windows would take focus and keystrokes on a working desktop.

### Metadata API keys

Some tag sources need an application key. Official builds embed OpenTag's keys at compile time from repository secrets; the values are never committed or shown in the UI. Builds from source and from forks compile without them: those sources then ask for your own key, or stay disabled. MusicBrainz, freedb and web sources need no key. In current `main` no key is embedded yet: Discogs uses your personal access token if you set one. Embedded keys arrive with the metadata sources work listed above. Secret names are in [RELEASING.md](docs/RELEASING.md#metadata-api-keys).

## Evidence

`docs/evidence/inventory.py ../` reproduces the resource-derived feature inventory against a separately supplied Mp3tag executable. The executable and vendor assets are not part of this repository. Per-feature verification notes are in [docs/verification](docs/verification). Tests of OpenTag are separate from comparisons with the original program, and no such comparison has been run yet.

## License

MIT for the independently authored application. Third-party dependencies retain their own licenses. The original Mp3tag software remains the property of its author.
