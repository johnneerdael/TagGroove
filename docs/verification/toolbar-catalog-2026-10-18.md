# R09 toolbar catalog qualification — 18 October 2026

This batch exposes useful commands that already have application routes. It
does not add a second toolbar, new file/source operations, or vendor website
buttons. The original catalog work started from verified branding commit
`c1375a6ccf24e728f3ac29dfd442d4bf1071d47b`. PR #55 now includes approved main
through `2841cdcf13f5d02161153dfb296b5413548d4027`, including the merged
associated-file delivery in PR #54.

## Behavior and compatibility

Retain the original 28 catalog command identities and every original default
item position. Retain the eight visible default commands. Append the additional
file, directory, selection, cover, playlist and built-in-source choices hidden
in main-toolbar configurations, including older saved configurations. Preserve
custom-toolbar order, visibility, authored labels, tool names and action-group
references. The custom toolbar gains choices but no automatically inserted items.

Use the existing main and custom renderers, button wiring, enabled/reason guards
and dispatcher. No separate file/source/cover implementation is introduced.
There is no new standalone cover extract/adjust route. Existing Tag → Filename
remains the rename converter; Edit in file list uses the existing editable
filename column when present. The application’s confirmations, pending-edit
requirements, provider/chapter restrictions and credential checks remain in effect.

Remember the last built-in source separately from the universal last-source
opener. Opening Deezer, then an installed web source, keeps Ctrl+I / **Last tag
source** on the web source and Ctrl+Shift+I / **Last tag source (quick)** on
Deezer. Preserve cover-only and Discogs Release ID variants. Quick commands use
the remembered built-in provider's eligibility and credential guards. A quick
command that opens a built-in source then becomes the universal last source too.

Persist only a validated built-in command ID in
`opentag.tagSources.lastBuiltIn`; keep `opentag.tagSources.lastSource` and its
`web:<id>` behavior unchanged. Seed the new preference before a web source can
overwrite an older profile's built-in `lastSource`. Retain the older
`opentag.tagSources.last` Discogs/MusicBrainz fallback when no valid command ID
is available. Existing configuration export/import collects seven native JSON
components and has no browser-storage preference allowlist; this local choice
is not added to the configuration bundle. This does not implement the separate
profile-migration work. No credentials enter the new preference.

## Local regression evidence

`tests/toolbar-catalog.test.mjs` exercises:

- Original catalog order, complete original default prefix and unchanged visible defaults.
- Hidden additions to older layouts without rewriting authored order/labels or input objects.
- New configuration persistence alongside named tools and action groups.
- Actual main/custom toolbar renderer and custom-choice construction.
- Actual main dispatcher routes for every added identity, with effects replaced by spies,
  and the production generic button binding reaching the dispatcher.
- Actual custom-toolbar dialog cancellation/save while retaining named tool/action state.
- Bundled German/Dutch translations for new hints and verbatim saved user labels.
- User-guide coverage of every added command label.
- Actual file, chapter, provider-document and credential eligibility policies for
  empty selections, pending edits, chapter rows, read-only provider sources,
  desktop-only folders, missing favorites, missing AcoustID keys, cover absence
  and busy state. A stale programmatic invocation remains refused.
- Actual source-dialog opens, main guards and dispatch for built-in Deezer →
  installed web → quick Deezer, while the ordinary last-source button still
  reopens web before quick runs. Reconstruct the source UI from persisted
  preferences to cover reload, cover-only and release-ID variants, legacy and
  invalid preferences, and opening web first after an upgrade. Refuse quick
  AcoustID without a key after a web source, preserving last-source state.

Before the fix, the three new state regressions failed: quick search invoked
MusicBrainz instead of Deezer, lost Deezer cover mode, and missed the remembered
AcoustID provider's key guard. The original eight catalog cases passed.

This is source, model and DOM/control-flow evidence. It is not native keyboard,
screen-reader, screenshot, original-app differential or live-provider qualification.
The earlier browser file-URL denial explicitly prohibited workarounds; no new
browser or screenshot attempt is part of this batch.

## Remembered-source correction validation

These local results cover the correction on the approved-main integration.
They do not replace native/platform CI or the final committed-head review.

| Check | Result |
| --- | --- |
| Full frontend suite (`npm test`) | 625 passed, zero failed |
| Native workspace suite (`npm test`) | 1139 passed, zero failed; 9 platform-specific tests ignored locally |
| Strict workspace Clippy and Rust formatting | Passed |
| Documentation publication tests and writing plan | 8 passed; 175 written / 188 planned pages |
| Focused toolbar/source/configuration-tools suites | 45 passed, including 11 toolbar cases |
| Production frontend build (`npm run build`) | Passed |
| Strict MkDocs guide build (existing docs virtual environment) | Passed |
| English catalog extraction check | Passed; no new catalog keys or catalog edits |
| Brand verification | 249 supplied files and 78 integrated copies verified; app/MSI/gnudb identities unchanged |
| Diff whitespace check | Passed |

## Historical catalog validation receipt

These results predate the remembered-built-in correction and approved-main
integration; they are retained as evidence for the original catalog batch,
not as final-head qualification.

| Check | Result |
| --- | --- |
| Full frontend suite (`npm run test:ui`) | 569 passed, zero failed |
| Focused toolbar catalog suite | 8 passed |
| Existing executable configuration/tools guide examples | 3 passed |
| Production frontend build (`npm run build`) | Passed |
| Strict MkDocs guide build | Passed |
| English catalog extraction check and bundled catalog validation | Passed |
| Brand verification | 249 supplied files and 78 integrated copies verified; app/MSI/gnudb identities unchanged |
| Diff whitespace check | Passed |

The final change includes the toolbar catalog/default list, its additive
English/German/Dutch keys, the separate built-in-source preference, and main
quick-source guard resolution. Existing source, protocol and metadata writer
identifiers are preserved; no Rust change is part of this correction.
Native/platform CI and exact-final-head GitHub Codex code review remain separate
gates owned by the coordinating owner.

## Work still open

R09 still needs a real migration-profile toolbar ordering and original toolbar
layout import format, native accessibility/keyboard/pointer qualification and any
additional cover commands justified by real operation routes. This batch makes
no vendor-button-count equivalence claim.

R10 remains a separate batch: playlist order/refresh qualification, XSPF/WPL/ASX
readers, externally added export-template preservation, and export encoding,
locale and sort behavior. Exposing existing playlist buttons does not implement
those readers or certify their output.

R07/R08/R11–R13 also remain open, including native save/edit journeys, native
qualification of the merged associated-file flow, remaining rename/copy flows, completion/tool details,
large-collection measurements, and desktop install/upgrade/signing outcomes.
The coordinating owner retains central parity-ledger and merge/CI ownership.
