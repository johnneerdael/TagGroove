# Tag sources (MusicBrainz, Discogs, freedb)

Branch `feat/tag-sources` added a top-level **Tag Sources** menu between Actions
and Help, matching Mp3tag's menu placement; `feat/sources-2` extended it. The
menu has **MusicBrainz…**, **Discogs…**, **freedb…**, **freedb (local)…**
(Ctrl/⌘+L, Mp3tag's shortcut), **Covers: MusicBrainz…**, **Covers: Discogs…**,
**Discogs access token…** and **Options…**. **Options…** opens the shared
Options dialog at its **Tag Sources** page, which corresponds to Mp3tag's
Options › Tag Sources, with **freedb** and **Local freedb** sub-pages. Ctrl/⌘+I
repeats the last used web source (MusicBrainz or Discogs), which is Mp3tag's
shortcut too. Every provider is a native Rust client; no Mp3tag `.src` web
source scripts are used or shipped.

## Workflow

1. **Search.** The dialog fills Artist and Album with the most common album
   artist (falling back to artist) and album among the selected files. You can
   search by artist and album, free text (a Lucene query on MusicBrainz, `q=` on
   Discogs), release ID or address, barcode, or catalog number. Results show
   artist, title, year, country, label, catalog number and format. MusicBrainz
   rows also show the match score and track count. The selected result shows a
   cover thumbnail, which is fetched only for that one row. A release ID or
   address skips the results step, as Mp3tag does. MusicBrainz takes an MBID or
   any `musicbrainz.org/release/…` address and rejects release-group addresses.
   Discogs takes `12345`, `r12345`, `[r12345]` or a `discogs.com/…/release/12345…`
   address. It rejects master addresses with an explanation. Each search
   fetches 50 results; **More results** appends the next page (MusicBrainz
   `offset`, Discogs `page`) while the source reports more, and shows "N of
   total". The chosen result has a **Preview** button that opens its release
   page in the default browser (see Preview below).
2. **Review.** The second step lists every field the release provides, with a
   checkbox for each. **All** and **None** set every box. Album-level values
   appear directly. Per-track fields show their shared value or the first value
   with a count. The cover section has its own **Write cover art** checkbox, a
   choice of artwork and **Replace existing artwork**. For MusicBrainz, the front
   cover is offered at once. The full Cover Art Archive listing loads in the
   background because archive.org can take 10 seconds or more. The track and file
   table pairs each release track with a file. Auto-match runs in three passes:
   disc and track number (ambiguous or duplicate numbers are skipped), then
   title similarity (normalized Levenshtein, plus whole-word containment in the
   filename), then selection order. Files beyond the last track are shown as
   "Not written". ↑ and ↓ swap neighbours. **Selection order** restores
   Mp3tag's plain order mapping. **By title** and **By length** are Mp3tag's
   Utils → "Suggest sorting — Title/Length": every file is paired with the
   track whose title is most similar, or whose length is closest, best pairs
   first, and the rest follow in selection order. File lengths that differ from
   the track length by more than 10 seconds are highlighted.
   **Editing before saving** (Mp3tag lets you adjust values in this window):
   album-level values, and per-track values that are the same on every track,
   are text boxes in the field list. Per-track values that differ show a
   summary and **Edit per track**, which switches the track column of the
   matching table to that field. The track column is always editable and its
   **Track column** selector picks any per-track field (TITLE by default). ` \\ `
   separates multiple values, as in the tag panel. Emptying a value means the
   field is not written for that file. Keyboard: Ctrl/⌘+E enables and
   Ctrl/⌘+D disables every field, F2 edits the value of the focused row, and
   Alt+↑/↓ moves the focused file (Mp3tag's Tag Sources shortcuts). The release
   address has a **Preview** button.
   **Cover utilities:** **Save image to file** (Mp3tag's Utils → "Save image to
   disc") also saves the chosen artwork next to the written files, named by
   Options → Tag Sources → "Default file name for cover art" (default `folder`).
   The name is a format string evaluated on the first written file of each
   folder, reduced to one safe file name; the image type adds the extension;
   an existing file of that name is replaced. **Save cover as…** (Utils →
   "Extract cover") downloads the shown artwork to a file chosen in a save
   dialog; the extension is corrected to the real image type.
3. **Apply.** Applying writes the checked fields and the chosen cover
   immediately. **Decision:** Mp3tag writes when the review window is confirmed,
   and OpenTag's other importers write immediately too: Filename → Tag through
   `filename_to_tags`, and Text File → Tag through `apply_text_import`. Cover art
   also cannot be staged as a draft in the tag panel, so tag sources do the same.
   Like the converters, the command is disabled while the selection has pending
   edits ("Save pending changes first."), when nothing is selected, or when the
   document-source policy blocks mutation. Unchecked fields are remembered per
   provider in local storage. The whole import is one undo step ("Tag source
   import") in Edit → Undo, through `history::record`; it was not journaled
   before this branch.

### Covers-only sources

Mp3tag's Tag Sources → Covers lists sources that import cover art only.
**Covers: MusicBrainz…** and **Covers: Discogs…** run the same search, then a
review with only the artwork section: the chosen cover (and optionally Save
image to file) is written to every selected file, and no fields change. There
is no track matching in this mode.

### freedb (gnudb) and freedb (local)

- **Disc ID.** As in Mp3tag, the disc ID comes from the selected files' lengths
  in file list order: tracks are laid out back to back after the two-second
  lead-in (150 frames), each length rounded to 1/75 s frames, and the CDDB ID is
  `(digit sum of start seconds mod 255) << 24 | playing seconds << 8 | tracks`.
  The search step shows the computed ID before searching. Both Rust
  (`freedb::Toc`) and TypeScript (`discToc`) implementations are tested against
  libdiscid's reference TOC (ID `830abf0a`). A typed disc ID is the other search
  mode.
- **freedb (gnudb).** CDDB over HTTP: `GET <server><path>?cmd=…&hello=<user>
  <host> OpenTag <version>&proto=6`. A file-based search sends `cddb query` with
  the TOC (status 200 is one exact match, 210/211 list exact or inexact matches,
  202 none). A typed disc ID has no TOC, so `cddb read` is tried in each of the
  eleven categories. Choosing a match sends `cddb read <category> <discid>`.
  `hello` uses the user and host of the configured email address, or
  `opentag localhost`. Requests are 500 ms apart on their own gate and are
  cancelled like the other sources.
- **Server settings.** Options → Tag Sources → freedb has protocol (HTTPS
  default, or HTTP), address (`gnudb.gnudb.org`), path (`/~cddb/cddb.cgi`), port
  (empty means the protocol default), email address, and Mp3tag's "Copy disc ID
  to COMMENT field" and "Copy disc ID to DISCID field". Restore page defaults
  returns to gnudb, which is what Mp3tag's "Current list of freedb servers"
  reset does. Because the server is user-configured, it is not on the trusted
  HTTPS host list: freedb requests use a separate client that never follows
  redirects, accept only http/https URLs built from the validated host name,
  read at most 1 MiB, and only parsed results reach the webview.
- **xmcd mapping.** `DTITLE` "Artist / Album" (continuation lines joined; no
  separator means artist and album are the same) → `ALBUM` and `ARTIST`;
  `DYEAR` → `YEAR`, falling back to `YEAR:` in `EXTD` (old rippers); `DGENRE` →
  `GENRE`, falling back to the category in title case except `misc` and `data`;
  `TTITLEn` → `TITLE` and `TRACK` n/total; `EXTD` (without `YEAR:`/`ID3G:`) →
  album `COMMENT` and `EXTTn` → track `COMMENT`, unless the disc ID goes to
  COMMENT; `DISCID` when that option is on. Various-artists discs (album artist
  "Various", "Various Artists", "VA", … or every title containing " / ") put
  the album artist in `ALBUMARTIST` and split each `TTITLEn` "Artist / Title"
  into the track's `ARTIST` and `TITLE`. `\n`, `\t` and `\\` escapes are
  decoded. Track lengths come from the `# Track frame offsets` and
  `# Disc length` comments, so the length warning works. Entries are UTF-8
  (protocol 6); local files that are not UTF-8 are read as Windows-1252.
- **Local freedb.** Options → Tag Sources → Local freedb holds the folder (text
  box plus **Choose folder…**). Lookups are by disc ID, typed or computed from
  the files, across every category subfolder: the one-file-per-disc layout
  (`rock/7a0a6b09`) and the Windows layout (`rock/78to7f` files holding many
  entries after `#FILENAME=` lines) are both read. Mp3tag's documentation notes
  that local databases are looked up by disc ID only; file-based IDs rarely
  match an exact local entry, because the local lookup is exact while gnudb's
  `cddb query` is fuzzy.
- **Preview.** gnudb results link to `https://gnudb.org/cd/<two-letter
  category><discid>`. That address pattern is taken from gnudb's site and was
  not verified this round, because gnudb.org serves a browser check page to
  non-browsers. Local entries have no page.

### Preview

`tag_source_open_page` opens a release page with the desktop opener plugin
(`tauri_plugin_opener::open_url`, the same Rust path Help uses), so the webview
needs no opener capability. Only `https` pages on musicbrainz.org,
discogs.com/www.discogs.com and gnudb.org, without credentials or ports, are
accepted. Mobile builds return the address in an error message instead.

## Field mapping

The mapping uses OpenTag/Mp3tag field names, so tag-core's existing ID3, MP4 and
Vorbis mappings apply.

- **MusicBrainz release fields:**
  - `ALBUM`, `ALBUMARTIST` (the artist credit with its join phrases) and `YEAR`.
  - `GENRE`: the top-voted release genre, falling back to the release group's,
    in title case.
  - `PUBLISHER` (labels), `CATALOGNUMBER` (`[none]` is dropped) and `BARCODE`.
  - IDs and release details: `MUSICBRAINZ_ALBUMID`, `MUSICBRAINZ_RELEASEGROUPID`,
    `MUSICBRAINZ_ALBUMARTISTID`, `MUSICBRAINZ_ALBUMRELEASECOUNTRY`,
    `MUSICBRAINZ_ALBUMSTATUS` and `MUSICBRAINZ_ALBUMTYPE`.
- **MusicBrainz track fields:**
  - `TITLE` and `ARTIST` (the track credit, falling back to the recording credit
    and then the release credit).
  - `TRACK` as `n/total` per disc, and `DISCNUMBER` as `d/total`.
  - `COMPOSER`, from composer relationships on the recording's works.
  - `ISRC`.
  - IDs: `MUSICBRAINZ_TRACKID` (the recording MBID, written to UFID in ID3, as
    Mp3tag and Picard do), `MUSICBRAINZ_RELEASETRACKID`, `MUSICBRAINZ_ARTISTID`
    and `MUSICBRAINZ_WORKID`.
- **Discogs release fields:**
  - `ALBUM`, `ALBUMARTIST` and `YEAR` (from `year`, falling back to `released`).
    Artist names honour Discogs join words and ANVs. A trailing `(2)`-style
    disambiguation suffix and `*` are removed.
  - `GENRE` (genres) and `STYLE` (styles).
  - `PUBLISHER`, `CATALOGNUMBER` (`none` is dropped) and `BARCODE` (the first
    Barcode identifier, without spaces).
  - `DISCOGS_RELEASE_ID`, plus `DISCOGS_MASTER_ID` when there is one.
- **Discogs track fields:**
  - `TITLE`, `ARTIST` (track artists, otherwise the release artists), `TRACK`
    and `DISCNUMBER`.
  - Discs come from positions such as `2-03`, `CD2-3` and `2.3`. Vinyl sides
    (`A1`) and plain numbers stay on the current disc. Tracks are numbered in
    order within each disc. Headings are skipped. An index track counts as one
    track and takes its position from its first sub-track.
  - `COMPOSER` comes from track credits and release credits with Composed By,
    Written-By, Music By or Songwriter roles. A release credit applies to a
    track when its `tracks` text names that position or a range containing it
    (`A2 to B1`), or when its `tracks` text is empty.

## Architecture

- `src-tauri/src/sources/model.rs` defines the provider-agnostic data model:
  `SourceRelease { fields, tracks: Vec<SourceTrack>, covers: Vec<SourceCover>, … }`,
  `SearchResult` and `SearchQuery`. Shared helpers handle de-duplication and
  per-disc numbering.
- `musicbrainz.rs` and `discogs.rs` each provide `search_query`,
  `parse_search`, `release_id`, `release_url` and `parse_release`. They are pure
  functions over `serde_json::Value` and are tested with hand-written fixtures.
  Another source, such as gnudb/freedb or local CUE and freedb files, can be
  added as a new module with the same functions. Matching, review, field
  selection and writing need no changes for a new source.
- `http.rs` holds the network layer:
  - One shared `reqwest` 0.13 client. It is the same reqwest version Tauri
    already pulls in, built with `rustls-no-provider`, with a preconfigured
    rustls 0.23 config that uses the **ring** provider and bundled
    **webpki-roots**. There is no OpenSSL and no platform verifier, which would
    need JNI initialisation on Android. HTTP/1.1 ALPN only.
  - The User-Agent is `OpenTag/<version> ( https://github.com/johnneerdael/opentag )`.
  - Timeouts: 10 s to connect and 30 s per request.
  - An HTTPS-only trusted host list (musicbrainz.org, coverartarchive.org,
    archive.org and its subdomains, api.discogs.com, i./img.discogs.com). The
    list is enforced on every request and every redirect, so the webview cannot
    use the image command as a general proxy.
  - Size limits: 8 MiB for JSON and 25 MiB for images.
  - Rate limits: requests through each service's gate run one at a time, at
    least 1.1 s apart for MusicBrainz and authenticated Discogs, and 2.5 s apart
    for anonymous Discogs (25 requests per minute). A MusicBrainz 503 ("server
    busy") is retried once.
  - Plain-language errors for timeouts, connection failures, 401/403 (token
    missing or rejected), 404, 429 and 5xx responses.
  - Cancellation: each request carries a monotonically increasing ticket on its
    channel (search, release or image). A newer ticket makes older work fail
    with "Cancelled…" before it is sent, after a rate-limit wait and before
    parsing. Closing the dialog cancels everything. The frontend also ignores
    stale responses.
  - `Image::validate` checks downloaded artwork with the `image` crate. It
    accepts JPEG, PNG, GIF, WebP or BMP that decodes to sane dimensions.
- The MusicBrainz cover size (250, 500, 1200 or original, as in Mp3tag's
  options) is set on the Options → Tag Sources page. That page is registered with
  `registerOptionsPage` and stored in the shared settings under
  `pages.tag-sources.musicbrainz_cover_size`. The backend reads it through
  `crate::settings::current()`.
- `credentials.rs` keeps the Discogs personal access token out of the shared
  settings. It lives in its own `tag-sources.json` in the app config directory,
  written atomically through a 0600 temporary file. The token never returns to
  the webview, which only receives `discogs_token_set`. The "Discogs access
  token…" dialog uses a masked input and a "Remove the saved token" checkbox.
- `freedb.rs` has the disc ID (`Toc`), CDDB URL building and response parsing,
  the xmcd parser and mapping, and the local folder lookup. It is pure apart
  from reading local files, and is tested with a hand-written xmcd fixture.
- `mod.rs` holds the Tauri commands: `tag_source_search` (returns a
  `SearchPage { results, total }`), `tag_source_release`, `tag_source_covers`,
  `tag_source_image` (a data URL, because the CSP blocks remote images),
  `tag_source_cancel`, `tag_source_apply`, `tag_source_open_page`,
  `tag_source_save_cover`, and `load_`/`save_tag_source_settings`.
  - Apply checks the items first: there must be at least one, no file may appear
    twice, and something must be selected to write.
  - It downloads and validates the cover **before touching any file**, then
    stages it as `cover.<ext>` in a temporary directory.
  - Inside the shared file-operation lock (`io`) and one `history::record`
    step, each file goes through `mutate_document`, so provider documents keep their recovery semantics. The
    steps use the settings-aware writers, so the user's MPEG/APE tag
    write/remove preferences apply to fields and covers: `settings::write_summary`,
    then `remove_all_covers_with_options` if replacing, then
    `add_cover_with_options`, all with `settings::tag_options()`. Results come back per file as a
    `ScanResult`. With `save_as`, the image is then written once per folder
    (errors are reported per folder); provider documents refuse this before
    anything is written.
- `src/tag-sources.ts` holds the UI module (`TagSourcesUI`) and its pure
  helpers: `searchPrefill`, `discToc`, `similarity`, `autoMatch`,
  `suggestByTitle`, `suggestByLength`, `moveSlot`, `fieldRows`, `editField`,
  `buildApplyItems` and the Options pages (`tagSourcesOptionsPages`). The
  `main.ts` changes are limited to the menu, operation names, enablement rules,
  dispatch, the Ctrl+I and Ctrl+L bindings, the Options page registration and
  the host callbacks.

## Verification

- `node --test tests/*.test.mjs`: 129 pass after merging `feat/core-parity`, 8 of them new in
  `tests/tag-sources.test.mjs`. They cover prefill, similarity, the three
  matching passes, ambiguous and multi-disc numbers, moves, field rows, apply
  items and tickets.
- `cargo test --workspace --locked`: all pass, including 30 new tests in
  `opentag::sources` (one more is ignored, described below). The new tests cover:
  - parsers and query builders for both providers, using JSON fixtures;
  - ID and address parsing, and the host allow-list;
  - ticket cancellation, status messages and image validation;
  - the credentials round trip (private file mode, token never viewed) and the
    cover size read from Options;
  - apply against real generated WAV files: fields written, covers replaced, an
    untrusted cover address rejected before any write, and several files written
    in one call.

  On this machine, Homebrew GNU `ar` is first on `PATH` and produces archives
  that Apple's linker rejects for `regex-compat`. The commands were run with
  `AR=/usr/bin/ar`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo fmt --all -- --check` and `npx tsc --noEmit` are clean.
- `cargo check -p opentag` succeeded for `aarch64-apple-ios` and for
  `aarch64-linux-android` (NDK 28 clang), so ring builds for mobile. The
  `x86_64-pc-windows-msvc` check could not be run from macOS because there are
  no Windows SDK headers for ring's C code. Windows relies on the native
  `windows-latest` CI job.
- The ignored test `live_musicbrainz_and_discogs_lookups` makes real requests:
  a MusicBrainz release lookup, a Discogs release lookup, a Cover Art Archive
  listing and a thumbnail download. It passed once during development
  (`cargo test -p opentag live_ -- --ignored`). It needs network access and is
  not part of the default suite. During development, real responses were
  sampled to confirm the shapes used by the fixtures. Anonymous Discogs search
  currently works but returns no thumbnails.
- The UI was exercised in a Vite harness with a mocked `invoke`, in light
  desktop (1280 px) and dark compact (500 px) layouts. The flow covered search,
  choosing a result, review, moving a file, the All/None and cover-only guards,
  Apply (checking the exact `items` and `cover` payload), and the Options save
  and token-clear path. The full Tauri app was **not** launched. The
  frontend → command → file path end to end was covered only by these separate
  layers.

### Round 2 (`feat/sources-2`)

- `node --test tests/*.test.mjs`: 200 pass (4 new: disc ID vector, suggested
  sorting by length and title, value editing and empty values, Options pages
  and validation; the field-row test now covers editability).
- `cargo test --workspace`: 441 pass, 3 ignored. New: 12 tests in
  `sources::freedb` (disc ID vector and file layout, ID validation, command
  URLs and handshake, Options parsing, query/read status codes including the
  gnudb refusal, xmcd mapping with continuation lines and escapes, various
  artists, Latin-1, local Unix and Windows layouts), paging parameters and
  totals for MusicBrainz and Discogs, Preview host checks, cover file naming
  per folder and extract-extension correction.
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --
  --check` and `npx tsc --noEmit` are clean.
- Native e2e `tests/e2e/freedb-local.js` (offline; `scripts/e2e-native.sh` now
  copies `tests/fixtures/freedb` next to the album): Options pages and saving
  through the real dialog, Ctrl+L, the computed disc ID, typed disc ID search,
  review layout fits, editing an album value (two values) and a track title,
  By length, Ctrl+D/Ctrl+E, apply, values on disk including DISCID, and the
  "Tag source import" undo step. All pass.
- Native e2e `tests/e2e/tag-sources-live.js` (live network, run once):
  MusicBrainz paging (50 of 552, then 98 after **More results**; MusicBrainz
  repeats a few releases across pages and duplicates are dropped), the Preview
  link, and Covers: MusicBrainz… by MBID with Save image to file: one cover
  written to both files, fields unchanged, one undo step. All pass. Preview was
  not clicked in e2e because it would open the desktop browser.
- Ignored `live_gnudb_query`: gnudb.gnudb.org currently answers every request
  from OpenTag with `500 Unknown application, developer email for OpenTag …`.
  gnudb only serves client applications its operator has registered, so
  **freedb (gnudb) does not return data for OpenTag until the project is
  registered with gnudb**. The dialog explains this and points to another CDDB
  server or a local folder. The protocol code is covered by fixtures only.

## Known gaps compared with Mp3tag

- There is no Discogs OAuth flow. A personal access token is used instead,
  which Discogs supports for single-user apps. Searches run without a token but
  at 25 requests per minute and without thumbnails.
- gnudb refuses unregistered applications (see Round 2), so the freedb source
  needs OpenTag to be registered with gnudb, or another CDDB server. freedb has
  no audio-CD lookup (OpenTag does not read CD drives) and no text search:
  gnudb's web search sits behind a browser check, and local text search needs
  an index that Mp3tag also requires users to build. Local lookups are exact;
  there is no fuzzy TOC matching against a local folder.
- The covers-only sources are MusicBrainz and Discogs. Mp3tag's Covers submenu
  lists whatever cover web-source scripts are installed; OpenTag has no script
  framework.
- Utils items are buttons rather than a menu, and the two "Suggest sorting"
  entries are one-shot actions rather than persistent checkboxes. Saved cover
  images are not part of the undo step. There is no drag and drop in the
  matching list (↑/↓, Alt+↑/↓ only), no Alt+Page Up/Down, and no field
  multi-selection: Space toggles a focused checkbox, but Mp3tag's Ctrl+A and
  Ctrl+Shift+A field selection and "apply checkbox state to all selected
  fields" are not implemented; All/None and Ctrl+E/Ctrl+D cover the common
  case.
- Discogs master releases must be resolved to a specific release by the user.
- A field that the user unchecks is left as it is. Mp3tag likewise only writes
  the visible fields. Writing is per file and not transactional across files: if
  the cover step fails after the fields were written, that file keeps the new
  fields, and the error is reported for it.
- Discogs vinyl sides all count as disc 1. Index tracks are not expanded into
  their sub-tracks.
