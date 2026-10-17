# Complete documentation acceptance inventory

This is an **open acceptance inventory**, not a parity certificate. The target remains Mp3tag 3.36.1 feature parity or an independently improved equivalent. [feature-inventory.json](feature-inventory.json) contains every crawled documentation page, its SHA-256, section/functional-entry requirements, all 97 keyboard-reference rows and additional key references from the other pages, per-format read/write/remove facts, source-framework commands, source-header/settings keys, evidence profiles and explicit contradictions.

All **94** official sitemap pages were fetched with verified HTTPS and recursively checked for further same-origin documentation links; none were missing and no additional pages or attachments were found. The private raw cache is `../reference-docs/full-site` outside the MIT application. It contains URL/status/TLS/SHA/byte manifests, the sitemap, raw HTML and private working extraction. No proprietary documentation body, screenshots, templates, provider scripts or vendor license text is redistributed in this inventory. Functional labels and API identifiers below are requirements; explanations are independently authored.

The JSON retains the inspected application HEAD and dirty source hashes from the documentation audit. Later implementation and verification must be assessed against their own source identities. The [parity ledger](PARITY.md) and the [remaining-gaps register](research/remaining-gaps.md) record subsequent progress, each linking its own verification note. Existing local contracts are linked separately from native commands, visible UI and runtime evidence. Every inventory item remains OPEN until its own acceptance evidence is recorded.

The catalog currently has **1533 section, functional-reference and authored acceptance entries**, plus **190 inline keyboard context references**. These counts include repeated references and historical headings; they are not a count of unique features or passed checks.

## Major acceptance gaps

Status as of 2026-10-17 (`main` `9d3764b`). Only open work is listed; the item-level list is the [remaining-gaps register](research/remaining-gaps.md).

| Area | Required acceptance work |
| --- | --- |
| Metadata and file formats | Real-encoder TAK/OptimFROG fixtures, global Nero fields and covers, original runtime comparison |
| File table and panel | Boost regex in Find, floating panel as a separate OS window, movable toolbar bands |
| Saved operations | Persistent history across sessions |
| Covers | Original resampler, quality and rounding equivalence |
| Converters/numbering | Original differential semantics |
| Actions/export/playlists | Original sort, locale and encoding proof |
| Sources | Bounded live-provider verification |
| General settings | Further bundled languages, setting-name audit against the binary, proxy credentials/SOCKS/PAC |
| Toolbars | The original registers 75 toolbar command ids; OpenTag offers 30, so about 45 are not assignable |
| File and OS operations | Associated `.lrc`/`.cdg` follow-along |
| Technical/expression behavior | Close the unverified items (`$caps` word starts, truth model, error degradation) with original runtime evidence |

## Open contradiction

The current [format reference](https://docs.mp3tag.de/file-formats/) lists DSF, OptimFROG, TAK, TTA and Matroska/WebM but not DFF or MIDI. TagLib support must not be taken as proof that the reference application supports DFF or MIDI. The [missing-format research](research/missing-formats.md) is a bounded recommendation, not implementation evidence.

## Every official page

The per-page scopes below are indexed by their official primary URL. Fine-grained section labels and independently authored requirements are in the JSON. Version-history pages are retained as historical references; removed providers and superseded features are not blindly reintroduced.

| Primary documentation page | Evidence profile | Acceptance entries |
| --- | --- | --- |
| [Mp3tag Documentation](https://docs.mp3tag.de/) | reference | 16 |
| [Actions](https://docs.mp3tag.de/actions/) | actions | 19 |
| [Adjust Cover](https://docs.mp3tag.de/actions/adjust-cover/) | actions | 5 |
| [Case Conversion](https://docs.mp3tag.de/actions/case-conversion/) | actions | 5 |
| [Convert Codepage](https://docs.mp3tag.de/actions/convert-codepage/) | actions | 2 |
| [Export Cover to File](https://docs.mp3tag.de/actions/export-cover/) | actions | 5 |
| [Export](https://docs.mp3tag.de/actions/export/) | actions | 1 |
| [Format Value](https://docs.mp3tag.de/actions/format/) | actions | 3 |
| [Generate Playlists](https://docs.mp3tag.de/actions/generate-playlists/) | actions | 1 |
| [Guess Values](https://docs.mp3tag.de/actions/guess/) | actions | 3 |
| [Import Cover from File](https://docs.mp3tag.de/actions/import-cover/) | actions | 2 |
| [Import Text File](https://docs.mp3tag.de/actions/import-text/) | actions | 2 |
| [Merge Duplicate Fields](https://docs.mp3tag.de/actions/merge-duplicate-fields/) | actions | 3 |
| [Remove Duplicate Fields](https://docs.mp3tag.de/actions/remove-duplicate-fields/) | actions | 3 |
| [Remove Fields Except](https://docs.mp3tag.de/actions/remove-fields-except/) | actions | 3 |
| [Remove Fields](https://docs.mp3tag.de/actions/remove-fields/) | actions | 3 |
| [Replace with Regular Expression](https://docs.mp3tag.de/actions/replace-regexp/) | actions | 5 |
| [Replace](https://docs.mp3tag.de/actions/replace/) | actions | 5 |
| [Set Cover Properties](https://docs.mp3tag.de/actions/set-cover-properties/) | actions | 4 |
| [Split Field by Separator](https://docs.mp3tag.de/actions/split-field/) | actions | 4 |
| [Command Line Interface](https://docs.mp3tag.de/command-line/) | missing-ui | 9 |
| [Converters](https://docs.mp3tag.de/converters/) | converter-basic | 2 |
| [Auto Track- and Disc-Numbering](https://docs.mp3tag.de/converters/auto-numbering/) | numbering | 21 |
| [Copy and Compose Contents from Tags](https://docs.mp3tag.de/converters/copy-compose-tags/) | converter-basic | 3 |
| [Import Tags from File and Folder Names](https://docs.mp3tag.de/converters/import-tags-from-filenames/) | converter-basic | 6 |
| [Import Tags from Text Files](https://docs.mp3tag.de/converters/import-tags-from-text-files/) | converter-additional | 6 |
| [Rearrange Parts of File Names](https://docs.mp3tag.de/converters/rearrange-filenames/) | converter-additional | 6 |
| [Rename Files](https://docs.mp3tag.de/converters/rename-files/) | converter-basic | 7 |
| [Credits](https://docs.mp3tag.de/credits/) | reference | 2 |
| [Customization](https://docs.mp3tag.de/customization/) | settings | 1 |
| [File List](https://docs.mp3tag.de/customization/file-list/) | settings | 9 |
| [Options](https://docs.mp3tag.de/customization/options/) | settings | 1 |
| [Appearance](https://docs.mp3tag.de/customization/options/appearance/) | settings | 12 |
| [Directories](https://docs.mp3tag.de/customization/options/directories/) | settings | 9 |
| [Export](https://docs.mp3tag.de/customization/options/export/) | export | 1 |
| [Field Values](https://docs.mp3tag.de/customization/options/field-values/) | settings | 6 |
| [Files](https://docs.mp3tag.de/customization/options/files/) | settings | 10 |
| [General](https://docs.mp3tag.de/customization/options/general/) | settings | 18 |
| [Language](https://docs.mp3tag.de/customization/options/language/) | settings | 3 |
| [Library](https://docs.mp3tag.de/customization/options/library/) | settings | 11 |
| [Messages](https://docs.mp3tag.de/customization/options/messages/) | settings | 14 |
| [Networking](https://docs.mp3tag.de/customization/options/networking/) | settings | 5 |
| [Playlist](https://docs.mp3tag.de/customization/options/playlist/) | export | 12 |
| [Tag Panel](https://docs.mp3tag.de/customization/options/tag-panel/) | settings | 1 |
| [Tag Sources](https://docs.mp3tag.de/customization/options/tag-sources/) | sources | 6 |
| [freedb](https://docs.mp3tag.de/customization/options/tag-sources/freedb/) | sources | 13 |
| [Local freedb](https://docs.mp3tag.de/customization/options/tag-sources/local-freedb/) | sources | 3 |
| [Tags](https://docs.mp3tag.de/customization/options/tags/) | tag-policy | 8 |
| [Advanced](https://docs.mp3tag.de/customization/options/tags/advanced/) | tag-policy | 15 |
| [Ape, Mpc](https://docs.mp3tag.de/customization/options/tags/ape/) | tag-policy | 7 |
| [Mapping](https://docs.mp3tag.de/customization/options/tags/mapping/) | tag-policy | 7 |
| [Mpeg](https://docs.mp3tag.de/customization/options/tags/mpeg/) | tag-policy | 10 |
| [Tools](https://docs.mp3tag.de/customization/options/tools/) | settings | 18 |
| [Updates](https://docs.mp3tag.de/customization/options/updates/) | settings | 7 |
| [Tag Panel](https://docs.mp3tag.de/customization/tag-panel/) | settings | 13 |
| [Exporting Tag and File Information](https://docs.mp3tag.de/export/) | export | 19 |
| [Extended Tags](https://docs.mp3tag.de/extended-tags/) | metadata | 9 |
| [Supported File Formats](https://docs.mp3tag.de/file-formats/) | metadata | 20 |
| [Filter and Search](https://docs.mp3tag.de/filter/) | filter | 21 |
| [Find](https://docs.mp3tag.de/find/) | missing-ui | 6 |
| [Format Strings and Placeholders](https://docs.mp3tag.de/format/) | technical | 95 |
| [Getting Started](https://docs.mp3tag.de/getting-started/) | reference | 1 |
| [First Steps](https://docs.mp3tag.de/getting-started/first-steps/) | reference | 25 |
| [Download and Installation](https://docs.mp3tag.de/getting-started/installation/) | reference | 15 |
| [Main Window](https://docs.mp3tag.de/getting-started/main-window/) | files | 26 |
| [Overview](https://docs.mp3tag.de/getting-started/overview/) | reference | 3 |
| [Release Notes Archive](https://docs.mp3tag.de/getting-started/release-notes-archive/) | reference | 241 |
| [Release Notes](https://docs.mp3tag.de/getting-started/release-notes/) | reference | 70 |
| [Keyboard Shortcuts](https://docs.mp3tag.de/keyboard/) | reference | 7 |
| [License Information](https://docs.mp3tag.de/license/) | reference | 1 |
| [Manage Covers](https://docs.mp3tag.de/manage-covers/) | cover | 9 |
| [Manage List](https://docs.mp3tag.de/manage-list/) | missing-ui | 5 |
| [Tag Field Mappings Table](https://docs.mp3tag.de/mapping-table/) | metadata | 95 |
| [Tag Field Mappings](https://docs.mp3tag.de/mapping/) | metadata | 102 |
| [Menu Commands](https://docs.mp3tag.de/menus/) | reference | 1 |
| [Menu Actions](https://docs.mp3tag.de/menus/actions/) | actions | 4 |
| [Menu Convert](https://docs.mp3tag.de/menus/convert/) | converter-basic | 7 |
| [Cover Window Context Menu](https://docs.mp3tag.de/menus/cover/) | cover | 19 |
| [Menu Edit](https://docs.mp3tag.de/menus/edit/) | files | 27 |
| [Fields Helper Menu](https://docs.mp3tag.de/menus/fields/) | missing-ui | 5 |
| [Menu File](https://docs.mp3tag.de/menus/file/) | files | 21 |
| [Format String Helper Menu](https://docs.mp3tag.de/menus/format-strings/) | missing-ui | 11 |
| [Menu Help](https://docs.mp3tag.de/menus/help/) | reference | 11 |
| [Menu Tag Sources](https://docs.mp3tag.de/menus/tag-sources/) | sources | 7 |
| [Menu View](https://docs.mp3tag.de/menus/view/) | files | 14 |
| [Playlists](https://docs.mp3tag.de/playlists/) | export | 11 |
| [Regular Expressions](https://docs.mp3tag.de/regex/) | scripting | 42 |
| [Scripting Functions](https://docs.mp3tag.de/scripting/) | scripting | 95 |
| [Tag Sources](https://docs.mp3tag.de/tag-sources/) | sources | 1 |
| [Using user-contributed Tag Sources](https://docs.mp3tag.de/tag-sources/archive/) | sources | 5 |
| [Developing Tag Sources](https://docs.mp3tag.de/tag-sources/development/) | sources | 77 |
| [Discogs Authorization](https://docs.mp3tag.de/tag-sources/discogs-auth/) | sources | 2 |
| [Importing Tags from Online Tag Sources](https://docs.mp3tag.de/tag-sources/import/) | sources | 12 |
| [Utils](https://docs.mp3tag.de/utils/) | missing-ui | 15 |

## User-test protocol

1. Select one acceptance item and record the exact source/build/platform identity and its primary documentation link.
2. Use disposable copies of actual audio, including custom fields, multivalues, opaque metadata and covers. Record pre/post values and media-range hashes when operations write files.
3. Exercise the real visible UI: keyboard/mouse on desktop and touch/document-provider paths on mobile. Include success, cancellation, invalid input, mixed selections, repeated execution and partial failure.
4. For theme behavior, exercise Light, Dark and System with menus, table selection/sorting/filtering, drafts, every dialog/control, cover preview, warnings, native pickers and persistence. Check system-theme changes while default mode is active.
5. Distinguish core contract, native execution, UI access, user-observed runtime and original Windows differential results. Store receipts/screenshots with their practical proof boundaries; none alone closes unrelated features.

No original runtime differential corpus or complete user-tested workflow matrix exists yet. Every inventory item remains open. This inventory is the starting acceptance contract for the active full-parity goal; it must evolve with implementation and measured behavior.
