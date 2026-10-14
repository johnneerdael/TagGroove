# Online tag sources (MusicBrainz and Discogs)

Branch `feat/tag-sources` adds a top-level **Tag Sources** menu between Actions
and Help, matching Mp3tag's menu placement. It has three entries:
**MusicBrainz…**, **Discogs…** and **Options…**. Ctrl/⌘+I repeats the last used
source, which is Mp3tag's shortcut too. Both providers are native Rust clients
for the public web APIs. No Mp3tag `.src` web source scripts are used or shipped.

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
   address. It rejects master addresses with an explanation.
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
   Mp3tag's plain order mapping. File lengths that differ from the track length
   by more than 10 seconds are highlighted.
3. **Apply.** Applying writes the checked fields and the chosen cover
   immediately. **Decision:** Mp3tag writes when the review window is confirmed,
   and OpenTag's other importers write immediately too: Filename → Tag through
   `filename_to_tags`, and Text File → Tag through `apply_text_import`. Cover art
   also cannot be staged as a draft in the tag panel, so tag sources do the same.
   Like the converters, the command is disabled while the selection has pending
   edits ("Save pending changes first."), when nothing is selected, or when the
   document-source policy blocks mutation. Unchecked fields are remembered per
   provider in local storage.

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
- `settings.rs` stores `tag-sources.json` in the app config directory. The
  Discogs personal access token and the MusicBrainz cover size (250, 500, 1200
  or original, as in Mp3tag's options) are kept there. The file is written
  atomically through a 0600 temporary file. The token never returns to the
  webview, which only receives `discogs_token_set`. The Options dialog uses a
  masked input and a "Remove the saved token" checkbox.
- `mod.rs` holds the Tauri commands: `tag_source_search`, `tag_source_release`,
  `tag_source_covers`, `tag_source_image` (a data URL, because the CSP blocks
  remote images), `tag_source_cancel`, `tag_source_apply`, and
  `load_`/`save_tag_source_settings`.
  - Apply checks the items first: there must be at least one, no file may appear
    twice, and something must be selected to write.
  - It downloads and validates the cover **before touching any file**, then
    stages it as `cover.<ext>` in a temporary directory.
  - Inside the shared file-operation lock (`io`), each file goes through
    `mutate_document`, so provider documents keep their recovery semantics. The
    steps are `write_fields_summary`, then `remove_all_covers_summary` if
    replacing, then `add_cover_summary`. Results come back per file as a
    `ScanResult`.
- `src/tag-sources.ts` holds the UI module (`TagSourcesUI`) and its pure
  helpers: `searchPrefill`, `similarity`, `autoMatch`, `moveSlot`, `fieldRows`
  and `buildApplyItems`. The `main.ts` changes are limited to the menu,
  operation names, enablement rules, dispatch, the Ctrl+I binding and the host
  callbacks.

## Verification

- `node --test tests/*.test.mjs`: 109 pass, 8 of them new in
  `tests/tag-sources.test.mjs`. They cover prefill, similarity, the three
  matching passes, ambiguous and multi-disc numbers, moves, field rows, apply
  items and tickets.
- `cargo test --workspace --locked`: all pass, including 29 new tests in
  `opentag::sources` (one more is ignored, described below). The new tests cover:
  - parsers and query builders for both providers, using JSON fixtures;
  - ID and address parsing, and the host allow-list;
  - ticket cancellation, status messages and image validation;
  - the settings round trip (private file mode, token never viewed);
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

## Known gaps compared with Mp3tag

- There is no Discogs OAuth flow. A personal access token is used instead,
  which Discogs supports for single-user apps. Searches run without a token but
  at 25 requests per minute and without thumbnails.
- No cover-only sources (Mp3tag's Tag Sources → Covers), and no freedb/gnudb or
  local freedb sources. The model is ready for them.
- Missing from the Utils menu: "Suggest sorting — Length", "Save image to disc"
  (and its file-name option) and "Extract cover". Length differences are only
  highlighted. Fields are not editable in the review dialog, and there is no
  drag and drop in the matching list (↑/↓ buttons only). Mp3tag's "apply
  checkbox state to all selected fields" behaviour is not implemented; All/None
  covers the common case.
- No "Preview" button to open the release page in a browser, because the app
  has no opener plugin. The release address is shown as selectable text instead.
- Only the first page of results is fetched (50 releases). Discogs master
  releases must be resolved to a specific release by the user.
- A field that the user unchecks is left as it is. Mp3tag likewise only writes
  the visible fields. Writing is per file and not transactional across files: if
  the cover step fails after the fields were written, that file keeps the new
  fields, and the error is reported for it.
- Discogs vinyl sides all count as disc 1. Index tracks are not expanded into
  their sub-tracks.
