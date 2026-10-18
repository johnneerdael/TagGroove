# TagGroove user guide: information architecture and coverage map

Not published (`_meta/` is excluded in `mkdocs.yml`). This file has three
jobs:

1. It explains how the guide is organised and why.
2. It **proves coverage**: every page of the Mp3tag documentation, used as a
   checklist of topics only, maps to TagGroove pages.
3. It tracks status and splits the remaining work into workstreams.

The full page plan lives in `_meta/planned-nav.yml` (`nav:`). Every page listed there
exists. Pages still to be written carry `status: draft` in their front
matter and show a brief of what they must cover. `python3 scripts/docs-status.py`
counts written and draft pages per section and checks the plan against
the files. `mkdocs.yml` contains only the public navigation. The publication
hook excludes pages marked `status: draft` from HTML and search; drafts
remain in the writing plan until ready. See `docs/PUBLISHING.md`.

## Structure

Eight tabs, each with one kind of page (see `style.md` § 3 for the page types):

| Tab | Pages | Sections | Page type |
|---|---:|---|---|
| Home | 1 | | Landing page |
| Get started | 12 | 7 top-level pages; Platform notes (5) | How-to guides for the first hour, plus platform notes |
| Learn | 19 | Tutorials (10), Concepts (8), plus an index | Tutorials and concept explainers. **Mp3tag's docs have neither.** |
| Guides | 23 | Everyday editing (11), TagGroove extras (3), Customize (8), plus an index | How-to guides for the main window and customization |
| Automate | 45 | Converters (7), Actions (19), Tag sources (12), Export and playlists (6), plus an index | How-to guides, one page per converter and action type |
| Reference | 50 | 8 top-level pages; Format strings (9), Menus (10), Options (23) | Reference: complete and exact |
| Cookbook | 35 | Rename (6), Fix and tidy (10), Import (5), Numbers (3), Covers (4), Find and report (6), plus an index | Recipes. **Mp3tag's docs have none.** |
| Help | 4 | | FAQ, troubleshooting, glossary, about |
| **Total** | **189** | | |

Design decisions:

- **Tasks before menus.** Readers arrive with a goal ("rename my files"),
  not a menu item. Learn, Guides, Automate and the Cookbook are organised by
  goal. The menu-by-menu and option-by-option pages sit under Reference for
  readers who look something up from the UI.
- **One page per converter, action type, menu and options page.** Each
  matches a single search, and the app's help links can point at a single URL.
- **Teaching layers on top of the reference.** Each hard topic has a
  tutorial (Learn), a complete reference and recipes:

  | Topic | Tutorial | Reference | Recipes |
  |---|---|---|---|
  | Format strings | `learn/format-strings-from-zero.md` | `reference/format-strings.md`, `reference/placeholders.md`, `reference/scripting/*` | rename-*, album-artist, year |
  | Regular expressions | `learn/regex-from-zero.md` | `reference/regular-expressions.md` | feat., brackets, capitalisation, track numbers |
  | Actions | `learn/first-action-group.md`, `learn/clean-a-messy-library.md` | `actions/*` | most "Fix and tidy" recipes |
  | Filters | (inside clean-up tutorial) | `reference/filter-syntax.md`, `guides/filter.md` | find-* |
  | Export | `learn/export-a-cd-catalogue.md` | `export/*` | html-album-catalogue, csv |
  | Tag sources | `learn/tag-from-online-sources.md` | `tag-sources/*` | |
  | Web sources | `tag-sources/writing-web-sources.md` | `tag-sources/web-source-reference.md` | |

- **Concept explainers** (`concepts/`) answer the "why" questions behind
  support requests: tag formats and versions, multi-value fields, track
  totals, covers, ReplayGain, encodings and sort fields.
- **TagGroove extras** that Mp3tag doesn't have: Library, chapters and cue
  sheets as rows, languages and translations, platform notes (macOS, Linux,
  Android, iOS), and the tag sources iTunes, Deezer, LRCLIB and AcoustID.
  Those four pages are stubs marked *waiting for merge*: the sources are
  being built on other branches. Write them once the sources are on main.

## Status: computed, never edited by hand

A page's status is its front matter. A page with `status: draft` is not
finished; removing that line marks it written. Nothing else records status,
so writers working in parallel never edit the same file for bookkeeping:
**don't edit this file to report progress**.

```bash
python3 scripts/docs-status.py              # written / total per tab
python3 scripts/docs-status.py --drafts     # every page still to write
python3 scripts/docs-status.py --coverage   # the coverage map below, with live status
```

`--coverage` reads the table below and reports each Mp3tag topic as
*written* (all mapped pages written), *partial* or *planned*. iTunes,
Deezer, LRCLIB and AcoustID are stubs that wait for those sources to reach
main. They stay drafts until then.

Edit this file only to change the plan itself (a page added, split or
renamed). Do that in a PR of its own, or tell the docs lead.

## Coverage map: Mp3tag documentation → TagGroove guide

Every page of the private Mp3tag documentation crawl
(`reference-docs/full-site/`, 94 pages) appears below with the TagGroove pages
that must cover it. The sub-topics column lists that page's section headings
as a **checklist**: each one must be covered somewhere in the mapped pages,
in our own words and from TagGroove's actual behaviour. Never copy prose,
examples or tables from the source. A sub-topic that TagGroove doesn't
implement is documented as such, or left out with a note in the PR.

| Mp3tag page | Topic | TagGroove page(s) | Sub-topics to cover |
|---|---|---|---|
| `actions` | Actions | [`actions/index.md`](../actions/index.md), [`learn/first-action-group.md`](../learn/first-action-group.md) | Action Types; Quick Actions; Action Groups; Adding, Duplicating, and Removing Action Groups; Rearranging Action Groups; Enabling and Disabling Action Groups; Loading and Saving of Selection Presets; Editing Action Groups; Organizing Action Groups; Action Types |
| `actions/adjust-cover` | Adjust Cover | [`actions/adjust-cover.md`](../actions/adjust-cover.md) | — |
| `actions/case-conversion` | Case Conversion | [`actions/case-conversion.md`](../actions/case-conversion.md) | Special Fields |
| `actions/convert-codepage` | Convert Codepage | [`actions/convert-codepage.md`](../actions/convert-codepage.md) | — |
| `actions/export` | Export | [`actions/export.md`](../actions/export.md) | — |
| `actions/export-cover` | Export Cover to File | [`actions/export-cover.md`](../actions/export-cover.md) | — |
| `actions/format` | Format Value | [`actions/format-value.md`](../actions/format-value.md) | — |
| `actions/generate-playlists` | Generate Playlists | [`actions/generate-playlists.md`](../actions/generate-playlists.md) | — |
| `actions/guess` | Guess Values | [`actions/guess-values.md`](../actions/guess-values.md) | — |
| `actions/import-cover` | Import Cover from File | [`actions/import-cover.md`](../actions/import-cover.md) | — |
| `actions/import-text` | Import Text File | [`actions/import-text-file.md`](../actions/import-text-file.md) | — |
| `actions/merge-duplicate-fields` | Merge Duplicate Fields | [`actions/merge-duplicate-fields.md`](../actions/merge-duplicate-fields.md) | Special Fields |
| `actions/remove-duplicate-fields` | Remove Duplicate Fields | [`actions/remove-duplicate-fields.md`](../actions/remove-duplicate-fields.md) | — |
| `actions/remove-fields` | Remove Fields | [`actions/remove-fields.md`](../actions/remove-fields.md) | Removing embedded cover art |
| `actions/remove-fields-except` | Remove Fields Except | [`actions/remove-fields-except.md`](../actions/remove-fields-except.md) | Preserving embedded cover art |
| `actions/replace` | Replace | [`actions/replace.md`](../actions/replace.md) | Special Fields |
| `actions/replace-regexp` | Replace with Regular Expression | [`actions/replace-regexp.md`](../actions/replace-regexp.md) | Special Fields |
| `actions/set-cover-properties` | Set Cover Properties | [`actions/set-cover-properties.md`](../actions/set-cover-properties.md) | — |
| `actions/split-field` | Split Field by Separator | [`actions/split-field.md`](../actions/split-field.md) | Special Fields |
| `command-line` | Command Line Interface | [`reference/command-line.md`](../reference/command-line.md) | Start with directory; Start with file; Add directory or file; Further Commands |
| `converters` | Converters | [`converters/index.md`](../converters/index.md) | Available Converters |
| `converters/auto-numbering` | Auto Track- and Disc-Numbering | [`converters/auto-numbering.md`](../converters/auto-numbering.md), [`concepts/track-and-disc-numbers.md`](../concepts/track-and-disc-numbers.md) | Tracknumbers; Begin at tracknumber; Save total count of tracks; Leading zeros for tracknumbers; Discnumbers; Begin at discnumber; Save total count of discs; Leading zeros for discnumbers; Resetting and Incrementing Counters; Next disc on change of directory; Next disc on change of field; Reset track counter on next disc; Reset disc counter on change of field |
| `converters/copy-compose-tags` | Copy and Compose Contents from Tags | [`converters/tag-to-tag.md`](../converters/tag-to-tag.md) | — |
| `converters/import-tags-from-filenames` | Import Tags from File and Folder Names | [`converters/filename-to-tag.md`](../converters/filename-to-tag.md), [`learn/tags-from-filenames.md`](../learn/tags-from-filenames.md) | Import Parts of the File Name to the Tag; Import Parts of the Folder Structure to the Tag |
| `converters/import-tags-from-text-files` | Import Tags from Text Files | [`converters/text-file-to-tag.md`](../converters/text-file-to-tag.md) | Matching files from the text file to Mp3tag’s File List |
| `converters/rearrange-filenames` | Rearrange Parts of File Names | [`converters/filename-to-filename.md`](../converters/filename-to-filename.md) | Simple Example; Complex Example |
| `converters/rename-files` | Rename Files | [`converters/tag-to-filename.md`](../converters/tag-to-filename.md), [`learn/rename-files-and-folders.md`](../learn/rename-files-and-folders.md) | Rename Files based on Tags; Create Folder Structures based on Tags |
| `credits` | Credits | [`help/about.md`](../help/about.md) | Special thanks to… |
| `customization` | Customization | [`customize/index.md`](../customize/index.md) | — |
| `customization/file-list` | File List | [`customize/columns.md`](../customize/columns.md), [`guides/file-list.md`](../guides/file-list.md) | Adding and Removing Columns; Rearranging Columns; Hiding and Showing Columns; Editing Columns; Loading and Saving of Configurations |
| `customization/options` | Options | [`reference/options/index.md`](../reference/options/index.md) | — |
| `customization/options/appearance` | Appearance | [`reference/options/appearance.md`](../reference/options/appearance.md) | Color Mode; Symbols; Toolbar Scaling; User Interface Scaling; Display grid lines in file view |
| `customization/options/directories` | Directories | [`reference/options/directories.md`](../reference/options/directories.md) | Subfolders; Favorite directory; Startup directory; Cover art directory |
| `customization/options/export` | Export | [`reference/options/export.md`](../reference/options/export.md) | — |
| `customization/options/field-values` | Field Values | [`reference/options/field-values.md`](../reference/options/field-values.md) | — |
| `customization/options/files` | Files | [`reference/options/files.md`](../reference/options/files.md) | Read Files; Restrict incoming files to; Rename Files from Tags; Replace folder separator in tag values with; Write Files; Preserve timestamps of modified files |
| `customization/options/general` | General | [`reference/options/general.md`](../reference/options/general.md) | General; Show current directory in window title; Automatically select next file when editing in file view; Auto-completion at selection boxes; Use natural sorting; Select files automatically; Keep moved files in the file list; Notification Area; Always show icon in notification area; Minimize to notification area |
| `customization/options/language` | Language | [`reference/options/language.md`](../reference/options/language.md) | — |
| `customization/options/library` | Library | [`reference/options/library.md`](../reference/options/library.md) | Enable Library; Only include these directories; Cleanup Library; Further Tips and Troubleshooting; Exclude the process mp3tag.exe from Windows Defender; Use Mp3tag and its settings on an SSD |
| `customization/options/messages` | Messages | [`reference/options/messages.md`](../reference/options/messages.md) | after save; after change; after delete; at removing tags; at cutting tags; at pasting tags; at exporting tags; at deleting files; at warnings from Actions; at unsaved changes; at importing covers |
| `customization/options/networking` | Networking | [`reference/options/networking.md`](../reference/options/networking.md) | Timeout; Use proxy |
| `customization/options/playlist` | Playlist | [`reference/options/playlist.md`](../reference/options/playlist.md) | Write extended info; Entries relative to working directory; Create Playlist automatically; Filename of playlist; Show confirm filename dialog; Don’t create directories |
| `customization/options/tag-panel` | Tag Panel | [`reference/options/tag-panel.md`](../reference/options/tag-panel.md) | — |
| `customization/options/tag-sources` | Tag Sources | [`reference/options/tag-sources.md`](../reference/options/tag-sources.md) | Default file name for cover art; MusicBrainz Cover Size; Further Configuration Options |
| `customization/options/tag-sources/freedb` | freedb | [`reference/options/tag-sources-freedb.md`](../reference/options/tag-sources-freedb.md) | freedb Server; Name; Address; Path; Port; Email address; General; Copy disc ID to comment field; Copy disc ID to DISCID field |
| `customization/options/tag-sources/local-freedb` | Local freedb | [`reference/options/tag-sources-local-freedb.md`](../reference/options/tag-sources-local-freedb.md) | — |
| `customization/options/tags` | Tags | [`reference/options/tags.md`](../reference/options/tags.md) | Save tags when using arrow-keys/single mouse click; Save mode at extended tag dialog; Don’t display first image from file directory as cover art; Further Configuration Options |
| `customization/options/tags/advanced` | Advanced | [`reference/options/tags-advanced.md`](../reference/options/tags-advanced.md) | ID3v2 comment language code (ISO-639-2); Always display POPULARIMETER values for ID3v2 ratings; List chapters as separate files; Use recommended mapping for Matroska video; Use single MP4 atom for multiple values; Reuse unmodified MP4 atoms; Reuse unmodified ID3v2 frames |
| `customization/options/tags/ape` | Ape, Mpc | [`reference/options/tags-ape-mpc.md`](../reference/options/tags-ape-mpc.md) | Read; Write; Remove |
| `customization/options/tags/mapping` | Mapping | [`reference/options/tags-mapping.md`](../reference/options/tags-mapping.md) | User-defined field mappings; Process when Reading mapped Fields; Process when Saving mapped Fields |
| `customization/options/tags/mpeg` | Mpeg | [`reference/options/tags-mpeg.md`](../reference/options/tags-mpeg.md) | Read; Write; Remove |
| `customization/options/tools` | Tools | [`reference/options/tools.md`](../reference/options/tools.md) | Name; Path; Parameter; for all selected files; Default tool; Further Reading |
| `customization/options/updates` | Updates | [`reference/options/updates.md`](../reference/options/updates.md) | Check for updates on startup; Also include Development Builds (Beta); Display release notes for new versions |
| `customization/tag-panel` | Tag Panel | [`customize/tag-panel.md`](../customize/tag-panel.md), [`guides/tag-panel.md`](../guides/tag-panel.md) | Adding and Removing Fields; Rearranging Fields; Hiding and Showing Fields; Editing Fields; Cover Art; Loading and Saving of Configurations |
| `export` | Exporting Tag and File Information | [`export/index.md`](../export/index.md), [`export/template-language.md`](../export/template-language.md), [`export/template-gallery.md`](../export/template-gallery.md), [`export/csv.md`](../export/csv.md), [`learn/export-a-cd-catalogue.md`](../learn/export-a-cd-catalogue.md) | Configuration Options; Adding, Duplicating, and Removing Export Configurations; Editing Export Configurations; Export Scripting Functions; Further Reading |
| `extended-tags` | Extended Tags | [`guides/extended-tags.md`](../guides/extended-tags.md) | Fields; Cover Art |
| `file-formats` | Supported File Formats | [`reference/file-formats.md`](../reference/file-formats.md), [`concepts/tag-formats.md`](../concepts/tag-formats.md) | AAC - Advanced Audio Coding; AIF / AIFC / AIFF - Audio Interchange File Format; APE - Monkey’s Audio; DSF - Direct Stream Digital; FLAC - Free Lossless Audio Codec; MKA / MKV - Matroska; MP3 - MPEG Audio Layer 3; MP4, M4A, M4B, M4V - MPEG-4; MPC - Musepack; OGG - Ogg Vorbis; OPUS - Opus Interactive Audio Codec; OFR/OFS - OptimFROG; SPX - Speex; TAK - Tom’s lossless Audio Kompressor |
| `filter` | Filter and Search | [`reference/filter-syntax.md`](../reference/filter-syntax.md), [`guides/filter.md`](../guides/filter.md) | Notes; Filter Syntax; <string>; <field> HAS <string>; <field> IS <string>; <expr1> AND <expr2>; <expr1> OR <expr2>; NOT <expr>; <field> GREATER <number>; <field> LESS <number>; <field> EQUAL <number>; <field> ABSENT; <field> MISSING; <field> PRESENT |
| `find` | Find | [`guides/find.md`](../guides/find.md) | — |
| `format` | Format Strings and Placeholders | [`reference/format-strings.md`](../reference/format-strings.md), [`reference/placeholders.md`](../reference/placeholders.md), [`learn/format-strings-from-zero.md`](../learn/format-strings-from-zero.md) | Characters with Special Functionality; Tag and Metadata Information; Technical Information; File Name and Path Information; File Properties; Dynamic Information; Export; Global placeholders for Export; Playlist |
| `getting-started` | Getting Started | [`getting-started/index.md`](../getting-started/index.md) | — |
| `getting-started/first-steps` | First Steps | [`learn/first-tagging-session.md`](../learn/first-tagging-session.md), [`getting-started/opening-files.md`](../getting-started/opening-files.md), [`getting-started/editing-and-saving.md`](../getting-started/editing-and-saving.md) | Saving Tags to Files; Step 1; Step 2; Step 3; Step 4; Renaming Files based on Tags; Step 1; Step 2; Step 3; Step 4; Importing Parts of the File Name to the Tag; Step 1; Step 2; Step 3 |
| `getting-started/installation` | Download and Installation | [`getting-started/install.md`](../getting-started/install.md), [`platforms/macos.md`](../platforms/macos.md), [`platforms/windows.md`](../platforms/windows.md), [`platforms/linux.md`](../platforms/linux.md), [`platforms/android.md`](../platforms/android.md), [`platforms/ios.md`](../platforms/ios.md) | Download; Installation; Standard; Portable; Store; Requirements; Check Installation Mode; Backup and Restore Configuration |
| `getting-started/main-window` | Main Window | [`getting-started/main-window.md`](../getting-started/main-window.md) | File List; Columns; Editing; Sorting; Customization; Tag Panel; Multiple Fields; Editing; Cover; Customization; Filter; Status Bar; Menus; Toolbar |
| `getting-started/overview` | Overview | [`getting-started/index.md`](../getting-started/index.md), [`index.md`](../index.md) | Documentation; Design Philosophy |
| `getting-started/release-notes` | Release Notes | [`getting-started/whats-new.md`](../getting-started/whats-new.md) | Mp3tag v3.36.1; Mp3tag v3.36; Improvements to Tag Sources; Custom Field Values Independent of Tag Panel Fields; Export to Multiple Output Files; Bugfixes and Internal Changes; Mp3tag v3.35.1; Bugfixes and Minor Changes; Mp3tag v3.35; File-related Configuration Options; Improvements to Toolbar Configuration; Improvements to Discogs and MusicBrainz Tag Sources; Support for RF64 WAV files; Bugfixes and Internal Changes |
| `getting-started/release-notes-archive` | Release Notes Archive | [`getting-started/whats-new.md`](../getting-started/whats-new.md) | Mp3tag v3.25; Mp3tag v3.24; Playlist Generator; Improvements for action Remove Duplicate Fields; Optimization of Padding for FLAC Files; Mp3tag v3.23; Modernized Dialogs and Larger Default Font; Improvements in Web Sources Framework; Mp3tag v3.22a; Updated to libwebp 1.3.2; Mp3tag v3.22; Configuration Settings for Tag Sources; Checkbox fields on Tag Panel; More JSON-related functions for Web Sources Framework |
| `index` | index | [`index.md`](../index.md) | Getting Started; Menu Commands; Using Mp3tag; Converters; Tag Sources; Extended Tags; Actions; Filter and Search; Export; Playlists; Customizing Mp3tag; File List; Tag Panel; Options |
| `keyboard` | Keyboard Shortcuts | [`reference/keyboard-shortcuts.md`](../reference/keyboard-shortcuts.md) | General; Convert; Actions; Tag Sources; Toolbar Customization; Useful helpers |
| `license` | License Information | [`help/about.md`](../help/about.md) | — |
| `manage-covers` | Manage Covers | [`guides/covers.md`](../guides/covers.md), [`concepts/cover-art.md`](../concepts/cover-art.md) | Apply to all; Adjust cover; Set cover type; Set cover description… |
| `manage-list` | Manage List | [`reference/menus/context-menus.md`](../reference/menus/context-menus.md) | History entries, names, descriptions, favorites, add/duplicate/edit/remove/reorder |
| `mapping` | Tag Field Mappings | [`reference/tag-mapping.md`](../reference/tag-mapping.md), [`reference/options/tags-mapping.md`](../reference/options/tags-mapping.md), [`concepts/tag-formats.md`](../concepts/tag-formats.md) | ACOUSTID_ID; ACOUSTID_FINGERPRINT; ALBUM; ALBUMSORT; ALBUMARTIST; ALBUMARTISTSORT; ARTIST; ARTISTSORT; BARCODE; BPM; CATALOGNUMBER; COMMENT; COMPILATION; COMPOSER |
| `mapping-table` | Tag Field Mappings Table | [`reference/tag-mapping.md`](../reference/tag-mapping.md), [`reference/field-names.md`](../reference/field-names.md) | — |
| `menus` | Menu Commands | [`reference/menus/index.md`](../reference/menus/index.md) | — |
| `menus/actions` | Menu Actions | [`reference/menus/actions.md`](../reference/menus/actions.md) | Actions; Actions (Quick); List of Action Groups |
| `menus/convert` | Menu Convert | [`reference/menus/convert.md`](../reference/menus/convert.md) | Tag - Filename; Filename - Tag; Filename - Filename; Text file - Tag (Import); Tag - Tag; Auto-numbering wizard… |
| `menus/cover` | Cover Window Context Menu | [`reference/menus/context-menus.md`](../reference/menus/context-menus.md), [`guides/covers.md`](../guides/covers.md) | Add cover…; Remove cover; Replace cover; Extract cover…; Adjust cover; Manage covers…; Set cover type; Set cover description…; Cut cover; Copy cover; Paste cover; Correct aspect ratio |
| `menus/edit` | Menu Edit | [`reference/menus/edit.md`](../reference/menus/edit.md) | Undo; Redo; Repeat; Copy; Copy as Text…; Cut; Paste; Find; Cover; Rename; Copy…; Move…; Remove; Delete… |
| `menus/fields` | Fields Helper Menu | [`reference/menus/context-menus.md`](../reference/menus/context-menus.md) | Remove field; Remove all fields…; Reset fields… |
| `menus/file` | Menu File | [`reference/menus/file.md`](../reference/menus/file.md) | Save tag; Remove tag; Read tag; Playlist (all files); Playlist (selected files); Generate Playlists…; Export; Change directory…; Add directory…; Load Playlist/Cuesheet…; Favorite directory; Open in Explorer; Options…; Save configuration… |
| `menus/format-strings` | Format String Helper Menu | [`reference/menus/context-menus.md`](../reference/menus/context-menus.md), [`reference/format-strings.md`](../reference/format-strings.md) | Field names in the first section; Extended fields; Information fields; Functions; Manage history…; Remove from history; Remove all from history… |
| `menus/help` | Menu Help | [`reference/menus/help.md`](../reference/menus/help.md) | Contents…; Keyboard Shortcuts; First Steps; Donation; Online Support…; Mp3tag Website…; Check for new version…; About Mp3tag… |
| `menus/tag-sources` | Menu Tag Sources | [`reference/menus/tag-sources.md`](../reference/menus/tag-sources.md) | freedb …; freedb (local)…; Discogs, MusicBrainz, … |
| `menus/view` | Menu View | [`reference/menus/view.md`](../reference/menus/view.md) | Refresh; Sort by; Tag Panel; Extended Tags…; Filter; Toolbars; Columns |
| `playlists` | Playlists | [`guides/playlists.md`](../guides/playlists.md), [`guides/xml-playlists.md`](../guides/xml-playlists.md), [`reference/options/playlist.md`](../reference/options/playlist.md) | Loading Playlists; Adding Files; Saving Playlists; Generating Multiple Playlists |
| `regex` | Regular Expressions | [`reference/regular-expressions.md`](../reference/regular-expressions.md), [`learn/regex-from-zero.md`](../learn/regex-from-zero.md) | Literals; Wildcard; Repeats; Non-greedy repeats; Parenthesis; Alternatives; Line anchors; Sets; Word boundaries |
| `scripting` | Scripting Functions | [`reference/scripting/index.md`](../reference/scripting/index.md), [`reference/scripting/text.md`](../reference/scripting/text.md), [`reference/scripting/search-compare.md`](../reference/scripting/search-compare.md), [`reference/scripting/conditions.md`](../reference/scripting/conditions.md), [`reference/scripting/numbers.md`](../reference/scripting/numbers.md), [`reference/scripting/dates-durations.md`](../reference/scripting/dates-durations.md), [`reference/scripting/lists-metadata.md`](../reference/scripting/lists-metadata.md), [`reference/regular-expressions.md`](../reference/regular-expressions.md) | String Functions; Boolean Functions; Arithmetic Functions; Variable Functions; Metadata Functions; Date and Time Formatting |
| `tag-sources` | Tag Sources | [`tag-sources/index.md`](../tag-sources/index.md) | — |
| `tag-sources/archive` | Using user-contributed Tag Sources | [`tag-sources/web-sources.md`](../tag-sources/web-sources.md) | Organizing Tag Sources |
| `tag-sources/development` | Developing Tag Sources | [`tag-sources/writing-web-sources.md`](../tag-sources/writing-web-sources.md), [`tag-sources/web-source-reference.md`](../tag-sources/web-source-reference.md) | Specification of the Web Source file format; Parser Scripts; How to Start; JSON; Debugging; List of Parser Commands; Configuration Settings; Configuration Schema Definition; Editing the Tag Source Configuration; Accessing Configuration Settings from a Tag Source |
| `tag-sources/discogs-auth` | Discogs Authorization | [`tag-sources/discogs.md`](../tag-sources/discogs.md) | — |
| `tag-sources/import` | Importing Tags from Online Tag Sources | [`tag-sources/index.md`](../tag-sources/index.md), [`learn/tag-from-online-sources.md`](../learn/tag-from-online-sources.md) | Search via Tag Source; Choose one of the search results; Confirm data to import; Utils menu |
| `utils` | Utils | [`guides/utilities.md`](../guides/utilities.md), [`guides/remove-tags.md`](../guides/remove-tags.md) | Remove Tag Types; MP3; Remove Unsupported Fields; MP3; Optimize; FLAC; MP4; Strip Chapters; MP4 |

All 94 checklist pages are mapped; none is unmapped.

## TagGroove pages beyond the checklist

These have no Mp3tag counterpart. They are TagGroove features or teaching
material:

| Page(s) | Why |
|---|---|
| `learn/*` (all tutorials), `concepts/*` | Teaching layer |
| `cookbook/*` (35 pages) | Recipe layer |
| `getting-started/editing-and-saving.md` | TagGroove's pending-edits model and session undo |
| `getting-started/coming-from-mp3tag.md` | Migration: compatibility and differences |
| `platforms/*` | macOS, Windows, Linux, Android, iOS specifics |
| `guides/library.md`, `reference/options/library.md` | TagGroove's library cache |
| `guides/chapters.md` | Chapters and cue sheets listed as rows |
| `guides/undo-history.md` | Session-wide undo journal |
| `customize/languages.md` | Translations and user language catalogs |
| `customize/configuration.md` | Save and load the whole configuration |
| `tag-sources/itunes.md`, `deezer.md`, `lrclib.md`, `acoustid.md` | New sources (waiting for merge) |
| `tag-sources/writing-web-sources.md` | A tutorial for writing web sources |
| `help/glossary.md`, `help/troubleshooting.md` | Support layer |

## Screenshots

Available scenes (`tests/docs-screens/screens.js`, each in `-light` and `-dark`):

| Scene | Shows | Used on |
|---|---|---|
| `main-window` | Album open, first file selected | `index.md`, first session |
| `tag-panel-keep` | All files selected, `< keep >` | first session |
| `menu-convert` | Convert menu open | first session |
| `tag-to-filename` | Tag → Filename with preview | first session |
| `extended-tags` | Extended tags dialog | first session |
| `filename-to-tag` | Filename → Tag with a pattern and preview (in `scenes/converters.js`) | for `converters/filename-to-tag.md` |

### Screens wanted

Writers add the scenes they need to their workstream's own scene file in
`tests/docs-screens/scenes/` (see the README there; `converters.js` is the
model). The table below lists screens the plan already expects, so writers
know what to script. If you can't script a scene, describe it in your PR and
the docs lead will add it.

| Scene name | State to show | For page |
|---|---|---|
| `actions-dialog` | Action groups dialog with one group expanded | `actions/index.md` |
| `filter-active` | Filter bar with an expression, list filtered | `guides/filter.md` |
| `options-tags-mpeg` | Options → Tags → Mpeg | `reference/options/tags-mpeg.md` |
| `manage-covers` | Manage covers dialog | `guides/covers.md` |
| `auto-numbering` | Auto-numbering wizard | `converters/auto-numbering.md` |
| `export-dialog` | Export dialog with a template | `export/index.md` |
| `customize-columns` | Customize columns dialog | `customize/columns.md` |

## Phase 2 workstreams

Eight workstreams that can run in parallel, each in its own branch from main
and its own PR. A writer owns exactly the listed pages, plus one scene file
`tests/docs-screens/scenes/<workstream>.js` and the screenshots it produces.
Links across workstreams point at stub pages that already exist, so no
writer waits for another.

Rules that keep parallel PRs conflict-free:

- Edit only your own pages. To fix something in another workstream's page,
  describe it in your PR instead.
- Don't edit `mkdocs.yml`, `_meta/*`, `stylesheets/` or
  `_snippets/abbreviations.md`. If you need a new page, a nav change, a style
  or a new abbreviation, ask the docs lead.
- Mark progress only by removing `status: draft` from finished pages.
- Commit only the screenshots your pages use, named after your scenes.
- Before pushing: `mkdocs build --strict` (style.md § 12) and
  `python3 scripts/docs-status.py` must both pass.

Scene files per workstream: W1 `onboarding.js`, W2 `learn.js`,
W3 `guides.js`, W4 `converters.js` (exists), W5 `sources-export.js`,
W6 `reference.js`, W7 `options.js`, W8 `cookbook.js`.

| # | Workstream | Pages | Files |
|---|---|---:|---|
| W1 | Onboarding and help | 16 | `getting-started/*` (7), `platforms/*` (5), `help/*` (4) |
| W2 | Tutorials and concepts | 18 | `learn/index.md`, the 9 remaining `learn/*.md` tutorials, `concepts/*` (8) |
| W3 | Everyday editing and customization | 22 | `guides/*` except `playlists.md` and `xml-playlists.md` (14), `customize/*` (8) |
| W4 | Converters and actions | 27 | `automate/index.md`, `converters/*` (7), `actions/*` (19) |
| W5 | Tag sources, export and playlists | 18 | `tag-sources/*` (12, 4 waiting for merge), `export/*` (4), `guides/playlists.md`, `guides/xml-playlists.md` |
| W6 | Scripting and data reference | 14 | `reference/index.md`, `reference/scripting/*` (7), `filter-syntax.md`, `field-names.md`, `tag-mapping.md`, `file-formats.md`, `keyboard-shortcuts.md`, `command-line.md` |
| W7 | Menus and options reference | 33 | `reference/menus/*` (10), `reference/options/*` (23) |
| W8 | Cookbook | 32 | `cookbook/index.md` and the 31 remaining recipes |

Notes per workstream:

- **W4**: action pages share a template: what it does, the settings table, 3+
  examples, gotchas, related. Write `format-value.md` and
  `replace-regexp.md` first, as models for the rest.
- **W6**: function pages follow `style.md` § 4 exactly. Generate
  `tag-mapping.md` from the registry in `crates/tag-core/src/mapping.rs`
  rather than by hand, and say so on the page.
- **W7**: one table row per control, with the exact English label, what it
  does, its default and a link to the guide page that explains the feature.
  Controls are in `src/options-ui.ts`, `src/general-options.ts`,
  `src/messages.ts`, `src/tag-sources.ts`, `src/playlist-ui.ts` and
  `src/tools.ts`.
- **W8**: follow the three exemplar recipes. Every format string and pattern
  must be checked against `crates/tag-core/src/script.rs` and its tests.
