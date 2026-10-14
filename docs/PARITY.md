# Mp3tag 3.36.1 parity ledger

The goal remains active. The application is a partial reconstruction; passing its current tests does not establish exact feature or performance parity.

Reference target: `../Mp3tag.exe`, SHA-256 `484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`. Static resource evidence inventories 1,110 resources, including 88 dialogs and three menus. The release includes 43 language files. `evidence/feature-inventory.json` retains every English language key and raw menu command ID. Many menu labels are numeric placeholders populated at runtime; their runtime label mapping remains to be recovered.

| Required area | Current implementation | Missing proof or behavior |
| --- | --- | --- |
| Cross-platform shell and Rust backend | Tauri 2, TypeScript, portable Rust core; macOS debug app built and opened; frozen-source Android aarch64 debug APK and unsigned arm64 iOS simulator app built ([evidence](verification/mobile-final-artifacts.json)) | Windows/Linux native runs and packaging; mobile installation, document access/save-back and actual device workflows; release artifacts |
| Mobile interfaces | Responsive touch/phone/tablet layouts, 390×800 actual-interface preview, mobile Rust entry point and platform capability command; Android/iOS simulator build artifacts | Document URI storage adapter, save-back/export, sandbox permissions, rotation/lifecycle and actual device tests |
| Open source distribution | Independent code, user-supplied OpenTag artwork and icons, MIT application license, dependency locks; original vendor executable/assets outside app | Final dependency license audit, reproducible release artifacts |
| File loading and properties | Recursive scanning, canonical dedup, partial errors; Beatport 434-file folder loaded in native UI | Cancel/progress, very large libraries, complete format coverage and original properties equivalence |
| Tag edits | Patch writes, staged replacement, multi-values, custom native fields, read-only and malformed errors | Every format/tag version/mapping/encoding option and original differential results |
| Format coverage | Tests cover WAV, MP3, FLAC, M4A, Ogg, Opus, AIFF, WavPack and AAC | WMA/ASF, DSF/DFF, Matroska/WebM, RF64, MIDI and other original formats; video/chapter files |
| File table | Multi-selection in table order, cached sorting/filtering, virtualized rows, pending edits, Rust expression filter | Inline editing, configurable columns and expressions, find/select-similar, all shortcuts |
| Tag panel | Common fields, per-file drafts, unsaved-close guard | Fully configurable fields, defaults/history/custom values, docking/orientation/layout parity |
| Extended tags | Multi-values, name edits/swaps, mixed-value preservation, individual multiline value controls | Differential field ordering/mapping, complete tag-specific values |
| Covers | Lazy descriptors and bounded artwork cache; read/add/remove, action import/export, type/description edits and image transforms with staged verification | Manage-cover aggregation, full image-format support, duplicate APIC limitations, original resampler/quality/rounding equivalence and source previews |
| Converters | Filename to tag, tag to filename, format-value action; optional folder AST and absolute paths; single-use rename preview receipts preserve dynamic destinations | Filename to filename, text-file converter, complete original filename validation and Unicode behavior |
| Actions | All 18 documented types modeled and exposed; group editor/order/enabling/presets, guarded .mta imports, raw field transforms, _TAG/_ALL and native file/cover/text effects; deferred exports/playlists | Complete .mta schema recovery; native and original differential proof for each option, overwrite/duplicate policies and partial failures; saved-operation undo |
| Scripting | All 64 documented functions; nested/quoted/optional expressions, typed truth, table/locale/app context hooks; Boost.Regex bridge | Original runtime differential corpus, UTF16 indexing details, exact original Boost version/options and all technical placeholder producers |
| Filters | Compiled Boolean/query parser, documented operators, scripted predicates, Boost MATCHES and native UI integration | Original differential query corpus, edge-case ordering and locale behavior |
| Numbering | Basic ordered track numbers, padding and totals; backend compound track/disc support | Disc wizard, reset on folders, total offsets/counts and full original semantics |
| Export and playlists | Fixed CSV/M3U8 plus action-driven .mte rendering: nested loops, variables, partitions, totals, encoding/BOM core, append and actual CRC/MD5; playlist action generation | Export configuration manager/dialog and persistent settings, audio-only MD5, original sort/locale/encoding proof, cuesheets and playlist imports |
| Undo/redo and clipboard | Draft discard only | Saved-operation undo/redo, repeat, copy/cut/paste tags and Copy as Text |
| Sources | Shipped source-script directive inventory and native JSON-loop cross-references | MusicBrainz/Discogs/freedb/local CD database, authentication, query matching, full .src/.inc framework and source worker behavior |
| File operations | File renames without overwrite, exclusive copy fallback for cross-volume/no-hardlink filesystems; directory action rebasing including loaded siblings | Cross-volume and case-only native runtime proof; full directory move semantics, copy/delete/play, associated .lrc/.cdg updates, favorites/navigation, external tools |
| Settings | Build config only | Persistent user options, imported/exported config, tag format preferences, toolbars, themes and translations |
| CLI and OS integration | Desktop executable | Original CLI switches, shell/file associations, platform integration |
| Performance | Beatport 434-file debug baseline and lazy-artwork improvement measured in PERFORMANCE.md; cached/virtualized UI; metadata actions skip no-op writes | Same-corpus original comparison, release load/interaction/memory measurements, device-specific budgets |

Final completion requires every row to be implemented and verified with evidence appropriate to its scope. No row is currently certified as exact parity.
