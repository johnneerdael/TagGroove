# Tag sources, round 4: more online services — 2026-10-15

Original development started from `feat/core-parity` at `11ba278`. The final feature-only preparation is based directly on reviewed main `1fe6a800ac1096494b33d3b461286ad2d91e1a13`; the unmerged Language/translation implementation is excluded. This round adds OpenTag's own application key for Discogs and six more services: iTunes, Deezer, LRCLIB lyrics, AcoustID, Last.fm and fanart.tv. Mp3tag ships none of these as built-in sources; its users get them as community web source scripts. OpenTag implements them as native sources that go through the same steps as MusicBrainz and Discogs: search, result list, review (field choice, editing, track ↔ file matching, cover choice, Save image to file), one undo step, Ctrl+I / Ctrl+Shift+I and the covers-only variant.

Code: `src-tauri/src/sources/{keys,itunes,deezer,lrclib,acoustid,lastfm,fanart}.rs`, the changes to `http.rs`, `credentials.rs`, `model.rs` and `mod.rs`, and `src/tag-sources.ts` (search types, result columns, API keys dialog, Options page) plus the menu in `src/main.ts`.

## Discogs: OpenTag's application key

- Order: (1) the user's personal access token (file or keychain, as before), (2) OpenTag's consumer key and secret using Discogs' key/secret request authentication (`Authorization: Discogs key=…, secret=…`), (3) anonymous requests with the existing advice to add a token. Key/secret authentication raises the limit to 60 requests a minute and returns image URLs, which covers search, release lookups and thumbnails. User-specific calls are not used, so OAuth is not needed.
- `keys.rs` optionally reads `OPENTAG_DISCOGS_CONSUMER_KEY` / `_SECRET` with `option_env!`. A build without them compiles and uses steps 1 and 3. This PR does not change the trusted build/release secret policy or supply provider secrets to PR tests. The deleted legacy `build.yml` / `release.yml` injection paths are historical, not evidence that a current downloaded binary includes a key.
- The values stay in Rust. They are sent only as a request header and never appear in addresses, logs, errors, settings or the webview. `load_tag_source_settings` returns only `discogs_app_key: true/false`. Network errors are built with `reqwest::Error::without_url()`, so even a key in an address could not reach an error message. A unit test checks the header and an error that contains no address.
- Discogs token dialog: with the application key it says "Discogs works without a token: this build of OpenTag signs its requests with its own Discogs application key (60 requests a minute, cover thumbnails included)". Without it, the old guidance is shown. A 401/403 while using the application key asks for a personal token.
- The other keyed services optionally read `OPENTAG_ACOUSTID_API_KEY`, `OPENTAG_LASTFM_API_KEY` and `OPENTAG_FANARTTV_API_KEY` at compile time. No new CI secret injection is included. Without a built-in or saved user key, the corresponding source remains disabled with an explanation.

## Per-source summary

### iTunes (Apple iTunes Search API)

| | |
|---|---|
| Endpoints | `GET https://itunes.apple.com/search?term=…&media=music&entity=album&limit=50&country=XX`; barcode: `lookup?upc=…`; release: `lookup?id=<collectionId>&entity=song&limit=200&country=XX` |
| Search types | Artist and album (one `term`), free text, album ID or Apple Music / iTunes address (`…/album/name/123`, `id123`, `?i=` ignored), barcode (UPC/EAN) |
| Album fields | ALBUM, ALBUMARTIST, YEAR (`releaseDate`), GENRE (`primaryGenreName`), COPYRIGHT, ITUNESALBUMID (`collectionId`), ITUNESARTISTID |
| Track fields | TITLE, ARTIST, TRACK `n/trackCount`, DISCNUMBER `d/discCount`, ITUNESCATALOGID (`trackId`), ITUNESADVISORY (explicit → 1, cleaned → 2). Songs only (`kind=song`), in disc and track order; lengths come from `trackTimeMillis` |
| Artwork | `artworkUrl100` with Apple's size segment replaced: 600, 1200 (default) or 3000 pixels (Options → Tag Sources → Online services); 250 px thumbnails |
| Limits | "approximately 20 calls per minute": requests are 3.1 s apart on their own gate |
| Terms | No key. Apple's promotional-content terms allow artwork for promoting store content and require "provided courtesy of iTunes" only for previews; OpenTag does not use previews. Each result links to its Apple Music page (Preview). Whether embedding store artwork in personal files is "promotion" is not settled by the page, and is listed under remaining gaps |
| Covers only | Covers: iTunes… |

### Deezer (public API)

| | |
|---|---|
| Endpoints | `GET https://api.deezer.com/search/album?q=…&limit=50&index=…` (paged); barcode: `album/upc:<code>`; release: `album/<id>` plus `album/<id>/tracks?limit=500`, collecting subsequent pages before review (the album object's track list has no ISRC, disc or position) |
| Search types | Artist and album, free text, album ID or deezer.com address, barcode. Deezer's `artist:"…" album:"…"` syntax returned nothing for album searches in a live check, so plain words are sent |
| Album fields | ALBUM, ALBUMARTIST, YEAR (`release_date`), GENRE (`genres`), PUBLISHER (`label`), BARCODE (`upc`) |
| Track fields | TITLE, ARTIST, ISRC, TRACK `n/total on that disc`, DISCNUMBER `d/discs` (from `disk_number`, `track_position`) |
| Artwork | `cover_xl` (1000 px, default) or `cover_big` (500 px); `cover_medium` thumbnails |
| Errors | Deezer answers errors with HTTP 200 and `{"error":{…}}`: code 4 (quota) → rate-limit message, 800 → not found, otherwise its message |
| Limits | 50 requests per 5 s: requests are 120 ms apart |
| Terms | No key for the public API. Deezer's developer terms are behind a login (see registrations below). OpenTag uses no audio, previews or streams |
| Covers only | Covers: Deezer… |

### LRCLIB lyrics

| | |
|---|---|
| Endpoints | `GET https://lrclib.net/api/get?track_name&artist_name&album_name&duration` (exact signature; LRCLIB matches lengths within about two seconds); fallback `api/search?track_name&artist_name`, where the hit closest in length (at most 3 s away) is taken; free text `api/search?q=`; record `api/get/<id>` |
| Search types | Each selected file (default: one result for all files, which opens the review at once), free text (lists lyrics records), lyrics record ID |
| Mapping | Plain lyrics → UNSYNCEDLYRICS (Mp3tag's field: ID3 USLT, MP4 ©lyr, Matroska LYRICS, ASF WM/Lyrics, Vorbis/APE by name). Options → Online services → "Write synchronized lyrics (LRC time stamps) when available" writes the LRC text to the same field instead. Mp3tag maps no field to ID3's SYLT frame, so synchronized lyrics stay text. ID3v2 needs a language: LRCLIB gives none, so the import prefixes raw lyrics with `XXX||` before ID3 parsing; literal `||` in the lyrics stays text. Other tag kinds, including APE when written alongside ID3 on MPEG, get the original text. Web sources retain their explicit language-prefix syntax. When LRCLIB has only synchronized lyrics and the option is off, the LRC lines are written without their time stamps (line and word tags removed, ID tags such as `[ar:]` dropped). Instrumental records write nothing |
| Matching | One track per selected file, in file list order, numbered like the file. Files without lyrics are not written |
| Limits | No published limit; requests are 200 ms apart, a 503 is retried once, and a file whose lookup fails is reported in the result line ("2 of 3 files, 1 lookup failed") without stopping the others. Searches already return the lyrics, so opening the result usually needs no second request |
| Terms | No key; LRCLIB asks for an identifying user agent (OpenTag's: `OpenTag/<version> ( https://github.com/johnneerdael/opentag )`) |

### AcoustID (key required, fpcalc required)

| | |
|---|---|
| Endpoints | `POST https://api.acoustid.org/v2/lookup` (form: `client`, `clientversion` (OpenTag's `CARGO_PKG_VERSION`), `duration`, `fingerprint`, `meta=recordings releases compress`, `format=json`; the documentation prefers POST for long fingerprints); the chosen release is read from MusicBrainz |
| Flow | Each selected file is fingerprinted with `fpcalc -json <file>` (no shell, stopped after 2 minutes). The releases of the matched recordings are ranked by how many selected files they hold, then by score. Columns: Score, Artist, Title, Year, Country, Files ("2 of 3 files"), Tracks. Choosing one runs the MusicBrainz release lookup, so fields and covers (Cover Art Archive) are the MusicBrainz ones. In the review each file goes to the track of the recording its fingerprint was identified as (`acoustid_file_matches`, by MUSICBRAINZ_TRACKID, any disc); only files without a match, and the tracks left over, are paired by tags or selection order. Auto-match, Selection order and the Suggest sorting checkboxes keep the fingerprint pairs and only arrange the remaining files. ACOUSTID_ID is offered for the identified tracks and written per file: a file gets the AcoustID of its own match for the recording of the track it is saved on, and none when it was not identified as that recording (for example a file moved there by hand). Each completed search replaces the remembered matches |
| fpcalc | Chromaprint's `fpcalc` is not bundled (see decisions). OpenTag uses the path from Options → Tag Sources → Online services → fpcalc program, or finds `fpcalc` on PATH and in `/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin`, `/opt/local/bin`. Without it the search explains how to install it. Not offered on Android/iOS |
| Limits | "Do not make more than 3 requests per second": 350 ms apart. MusicBrainz lookups keep their 1.1 s gate |
| Terms | Free for non-commercial use with a registered application key; commercial use goes through acoustid.biz. OpenTag only looks up and never submits fingerprints, so no per-user key is needed |
| Key | Build key `OPENTAG_ACOUSTID_API_KEY` or the user's key (Tag Sources → API keys…). Without a key the menu entry is disabled and its tooltip says why |

### Last.fm (key required)

| | |
|---|---|
| Endpoints | `GET https://ws.audioscrobbler.com/2.0/?method=album.getinfo&artist&album&autocorrect=1` (Artist and album: a direct lookup, shown as one result) or `&mbid=`; `method=album.search&album=…&limit=50&page=…` (album title words); `api_key`, `format=json` |
| Fields | ALBUM, ALBUMARTIST, GENRE (the album's top tags, title-cased; listener tags such as "seen live", "favorite", "albums I own" and bare numbers are skipped; 1 by default, 2/3/5 in Options), tracks with TITLE, ARTIST, TRACK `rank/total` and lengths. Cover: the largest image Last.fm offers (300 px) |
| Errors | `{"error": n}`: 6 not found, 10/26 key rejected, 29 rate limit. Single entries that Last.fm sends as objects instead of arrays are handled |
| Limits | Last.fm publishes no number; requests are 250 ms apart |
| Terms | Non-commercial use, credit and link back to Last.fm: the album's Last.fm page is the release address and Preview. The key goes in the address, as the API requires, and errors never show it |
| Key | Build key `OPENTAG_LASTFM_API_KEY` or the user's key. Only unsigned read methods (`album.search`, `album.getinfo`) are used, so no shared secret is needed |

### fanart.tv (key required, covers only)

| | |
|---|---|
| Endpoints | Albums are found with MusicBrainz's release-group search (`musicbrainz.org/ws/2/release-group/?query=…`, MusicBrainz gate). Artwork: `GET https://webservice.fanart.tv/v3/music/albums/<release group MBID>` with headers `api-key` (project key) and `client-key` (the user's personal key), so keys never go in addresses |
| Search types | Artist and album, free text, release group ID or address (release addresses are refused with an explanation) |
| Artwork | `albumcover` by likes ("Front (5 likes)"), then `cdart` ("Media (disc 1)"); `http://assets.fanart.tv` addresses are fetched over HTTPS; `…/preview/…` thumbnails |
| Key | Build project key `OPENTAG_FANARTTV_API_KEY`. The user's optional personal API key is sent with it as `client-key`. A build without a project key uses the user's key as the project key |
| Covers only | Covers: fanart.tv… (the only entry; fanart.tv has no tag data) |

### Cover Art Archive

It was already a source: Covers: MusicBrainz… lists every Cover Art Archive image of the release. No second entry was added.

## Common changes

- Trusted HTTPS hosts (also enforced on redirects): `itunes.apple.com`, `*.mzstatic.com`, `api.deezer.com`, `cdn-images.dzcdn.net`, `e-cdns-images.dzcdn.net`, `lrclib.net`, `api.acoustid.org`, `ws.audioscrobbler.com`, `lastfm.freetls.fastly.net`, `webservice.fanart.tv`, `assets.fanart.tv`. Preview opens `music.apple.com`, `www.deezer.com`, `www.last.fm` and `fanart.tv` pages as well.
- `http::Service` carries an `Auth` (none, Discogs token, Discogs application key, header keys, key in the request) instead of the Discogs token alone. It also gained `json_optional` (404 is an answer) and `json_post` (form POST). Every service has its own gate and spacing.
- Tag Sources menu: iTunes…, Deezer…, Last.fm…, AcoustID (fingerprints)…, Lyrics: LRCLIB…, Covers: iTunes…, Covers: Deezer…, Covers: fanart.tv…, API keys…. Ctrl+I remembers any of them, and Ctrl+Shift+I searches at once (the per-file sources always can).
- Tag Sources → API keys… (also a button on Options → Tag Sources → Online services) holds the user's AcoustID, Last.fm and fanart.tv keys in the private `tag-sources.json` (0600, written atomically). The keys are never in the shared settings or returned to the window. The dialog says whether the build has its own key and where to register.
- Options → Tag Sources → Online services: iTunes store country and cover size, Deezer cover size, LRCLIB synchronized lyrics, Last.fm genre count, fpcalc program.
- Scripted offline test runs (`OPENTAG_E2E_SCRIPT` + `OPENTAG_E2E_OFFLINE`, debug builds) send every built-in source to `http://127.0.0.1:18766/<host>/<path>` (`http.rs`, `route`). Before this round those runs could reach the real services.

## How it was verified

| Item | Unit tests (recorded or hand-written fixtures, no network) | Native suite `tests/e2e/sources-4.js` (CI, headless, all three OSes) |
|---|---|---|
| Discogs auth order | `keys::discogs_prefers_the_personal_token_then_the_app_key`, `http::status_messages_explain_tokens_limits_and_outages` | Token dialog text of a build without the key |
| Credentials in headers, not errors | `http::credentials_travel_in_headers_and_never_in_errors` (local server: Discogs key/secret header, fanart headers, form POST; refused connection error without the address's key) | `load_tag_source_settings` never returns the saved keys |
| iTunes | `itunes::*` (queries, IDs/addresses, search, songs only, order, advisory, 3000 px artwork) with `fixtures/itunes-lookup.json` | Search, columns, thumbnail, Preview address, review tracks, 1200 px cover, fields, IDs, cover on disk, Ctrl+I |
| Deezer | `deezer::*` (paging, barcode, errors in HTTP 200, multi-disc numbering, ISRC) with `fixtures/deezer-*.json` | Import: ISRC per track, label, barcode, genre |
| LRCLIB | `lrclib::*` (signature URLs, closest-length fallback, `files:` IDs, plain/synced choice) with `fixtures/lrclib-record.json`; `lyrics_without_a_language_are_written_to_id3_and_other_tags`, `raw_online_lyrics_keep_literal_language_delimiters`, native MPEG ID3+APE readback and unchanged explicit-language regression tests | Per-file lookup, auto-open review, one track per file, plain lyrics on MP3 (ID3 `XXX`), file without lyrics untouched, undo step |
| AcoustID | `acoustid::*` (fpcalc JSON, a stand-in fpcalc script run without a shell, ranking by files then score, each match's own ACOUSTID_ID, per-file recordings, a second search replacing the first); `AcoustID pairs each file with the track of its fingerprinted recording` (shuffled selection, missing tags, two discs, duplicate matches) and `ACOUSTID_ID is written per file…` (manual reassignment, another match's ID for the same recording, unchecked field) | Disabled without a key, enabled after one, missing fpcalc explained; with a stand-in fpcalc (sh / .cmd) the release holding both files is listed, files are paired by fingerprint against their tags and order, titles and ACOUSTID_ID written |
| Last.fm | `lastfm::*` (key in requests, name/MBID lookups, single-object lists, tag filtering, errors) with `fixtures/lastfm-album.json` | Enabled after saving a key; GENRE "Electronic" ("seen live" skipped) and titles on disk |
| fanart.tv | `fanart::*` (release-group search and IDs, likes order, disc art, HTTPS, previews) | Release-group search, covers by likes, covers-only write |
| Options and keys | `credentials::api_keys_are_saved_privately_…`, `mod::online_service_options_have_safe_defaults`, `mod::keyed_services_need_a_key_…`; `tests/tag-sources.test.mjs` (commands and Ctrl+I, search types, columns, key gating, empty tracks, sparse selected fields, cover-only writes, raw-lyrics provenance, Options page) | API keys dialog, Online services page |
| Offline routing | `http::offline_test_runs_send_built_in_sources_to_the_fixture_server` | Every answer comes from `tests/e2e/sources-4-fixtures.sh` |

Historical original checks (not the current feature-only head): `AR=/usr/bin/ar npm test` (301 UI tests, all Rust tests), `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all -- --check`, `npx tsc --noEmit`. The ignored `live_round_four_itunes_deezer_and_lrclib` passed against the live services once (iTunes lookup of Discovery, Deezer album 302127 with ISRC, LRCLIB record 250327). During development a few live answers were sampled with curl to confirm the shapes the fixtures use. No app window was opened on a workstation. The native suite runs only in the `native-e2e` CI job.

## Decisions

- **fpcalc is not bundled.** Chromaprint is LGPL 2.1+. The `fpcalc` binaries are built with FFmpeg (LGPL, or GPL depending on the build), so shipping them would mean a binary per platform (five desktop targets, none for mobile), license texts and source offers, and keeping FFmpeg patched. A pure-Rust port (`rusty-chromaprint`, MIT) would also need a decoder for every format (for example Symphonia, without Opus). For now OpenTag uses a `fpcalc` the user installs, found automatically or set in Options. In-process fingerprinting is a possible later step.
- LRCLIB synchronized lyrics are text in UNSYNCEDLYRICS (optional), because Mp3tag defines no synchronized-lyrics field. Their language is ID3's `XXX`.
- AcoustID only looks up and never submits. The chosen release is MusicBrainz data, so the review matches the MusicBrainz source.
- Last.fm's Artist and album search is a direct `album.getinfo` because `album.search` takes only the album title.
- fanart.tv is covers-only, and its albums are found through MusicBrainz release groups, which are fanart.tv's album keys.
- iTunes album search requests at most 50 results. The album lookup requests up to 200 rows and has no documented continuation route in the checked Apple documentation. Incomplete or capped unverifiable album listings are refused before review, as detailed below.
- API keys are kept like the Discogs token: in the private file only, with no keychain option for now, because they are application keys rather than account credentials.

## Remaining gaps

- AcoustID needs a user-installed fpcalc and does not run on Android/iOS.
- Apple's terms describe artwork use for promoting store content. Embedding iTunes artwork in one's own files is common practice, but the terms page does not address it directly. Deezer's API terms could not be read without a developer login. The owner should review both (below).
- Last.fm covers are at most 300 px. Deezer covers stop at 1000 px, the largest documented size.
- No Discogs OAuth (user-specific calls are not needed for tagging).
- Deezer's advanced search syntax is not used (see Deezer), so artist and album words may also match other fields.
- Historical key-gated end-to-end evidence used fixture answers and a user-entered test key. Current built-in key availability is a property of each downloaded binary, not established by these tests.

## Registrations the owner still needs

- **Discogs, AcoustID, Last.fm, fanart.tv**: done. The repository secrets `DISCOGS_CONSUMER_KEY`/`_SECRET`, `ACOUSTID_API_KEY`, `LASTFM_API_KEY` (no shared secret, by design) and `FANARTTV_API_KEY` were configured during the original verification. The legacy `build.yml` and `release.yml` workflows were disabled and removed during the 2026-10-15 review-gate transition; their earlier key-injection paths are not evidence for current builds. AcoustID and Last.fm allow non-commercial use only; Last.fm asks for attribution and may need to approve "powered by" placements. The AcoustID application's version should follow OpenTag's version (it is sent as `clientversion`).
- **Deezer**: sign in at <https://developers.deezer.com> and accept the API terms of use, which are only shown after login. Registering an application is optional for the public endpoints used here.
- **Apple**: no registration. Review the iTunes Search API promotional-content terms for embedding artwork.
- **LRCLIB**: no registration.

## Review follow-up (2026-10-15)

- Skip matched files that have no checked values after constructing their updates, including sparse iTunes advisory fields. Keep empty updates only when a cover is selected.
- Mark LRCLIB apply items as raw lyrics. The source-specific core write path adds the unknown-language prefix per ID3 target before parsing, preserving literal delimiters and keeping APE text unchanged during simultaneous MPEG ID3+APE writes. Ordinary and web-source writes retain explicit language-prefix semantics.
- Focused local checks passed: 25 JavaScript tag-source tests, 13 application source tests (three live-network tests ignored), and two native MPEG source-write regressions. These checks do not establish live service behavior or current CI success.

## Feature-only main integration and iTunes completeness

The provider features are extracted onto actual main without PR17 translation
modules, catalogs, typed-error companions or changes to existing main language
behavior. Main's cover filename custom-column evaluation, native tag handling,
Undo/Redo and current guide chapters are retained. The existing trusted native
suite list gains `sources-4`; no automatic workload trigger, secret, cache,
version or review-gate policy changes.

The iTunes release parser counts distinct positive song IDs belonging to the
selected collection. A positive album track count larger than that eligible
listing produces an explicit incomplete-album error before a `SourceRelease`
is returned. A response at the requested 200-row cap without a positive album
count is refused because completeness cannot be proved. A smaller response
without a usable total retains the existing best-available fallback. Complete
200-song and multi-disc responses retain disc/track ordering and metadata.

Apple's checked Search API documentation describes a maximum search `limit` of
200. The archived lookup examples use `limit` but do not document continuation;
no unsupported lookup offset pagination is invented here. Primary references:
[Apple Search API](https://performance-partners.apple.com/resources/documentation/itunes-store-web-service-search-api/)
and [archived Lookup Examples](https://developer-mdn.apple.com/library/archive/documentation/AudioVideo/Conceptual/iTuneSearchAPI/LookupExamples.html).
This is documentation and fixture evidence, not a fresh live Apple guarantee.

Actual parser regressions cover partial albums above and below the request cap,
foreign/duplicate rows, missing/zero totals, complete 200-song multi-disc output,
raw titles/metadata and the actual native two-song lookup fixture. The older
three-song unit fixture now advertises three songs consistently.

Current-head local validation is recorded in the feature-only coverage ledger;
older aggregate counts above describe their historical source. Native UI suites
are configured for trusted reviewed CI; no local app window or current remote
CI success is claimed by this integration.

A finite independent integration review caught two historical fpcalc error
guidance strings naming a nonexistent AcoustID Options subpage. Both now name
**Tag Sources → Online services**. The real missing-program producer first
failed the corrected guidance assertion, then all seven AcoustID module tests
passed. This message/assertion-only followup uses the 951-test full-workspace
feature checkpoint as its broader baseline; final focused tests and full
all-target Clippy qualify the correction without claiming another full run.

## Native provider-suite shortcut qualification

The new `sources-4` suite was run locally against the frozen `8d0c87f`
production source in a dedicated ad-hoc debug bundle and app-data identity,
`org.opentag.editor.e2e.pr18direct`, with generated audio and loopback-only
provider answers. Its first run passed 19 checks before timing out at the
repeat-source shortcut: the test sent physical Control on macOS, while the
application correctly requires Command. The test now sends `metaKey`, as the
existing `sources-3` suite does; the existing debug prelude converts it to
Control on Windows/Linux. The dialog assertion and every later assertion are
unchanged. Production code is unchanged.

Replaying only this suite with the same executable and `--no-build` passed
**40 checks, zero failures**, including LRCLIB writes and Undo, iTunes metadata
and covers, repeating iTunes, Deezer fields, key storage, Last.fm genres,
fanart.tv artwork and AcoustID fingerprint pairing and per-file writes. The
fixture server and app stopped; only the dedicated test identity's state was
removed. This is actual local macOS native UI evidence, not live-provider,
Windows/Linux source-suite, mobile or release validation. The extraction ledger
records the preceding feature/message baseline; this test-only followup retains
its full Rust/UI/build results because production, native include fixtures and
dependencies are unchanged. The prior trusted-main premerge workflow did not yet
list this new suite, so its common native jobs are separate evidence.

## Final integration with reviewed gnudb main

PR19 merged as main `dc95b1662a4020e8bdf54e3830e285f0df6b610e`. The final
PR18 integration preserves both providers' authentication, request limits and
review routes, and supersedes the obsolete gnudb registration/disc-ID-only
claims in the parity records. These are inherited gnudb features, not new PR18
release-note claims.

All tracked files match the independently reviewed combined preparation
`4d8912e4cf082ea9882f1eb98b5e5fbfa594b6c0` except this verification note and
the portable shortcut test above. Therefore its fresh full-workspace **960
passing Rust tests, five explicit ignored cases, and all-target Clippy** apply
to the identical production/native/Cargo/include inputs; no additional full
native run is claimed. Fresh final integration checks passed **380 UI tests**,
the production build, eight publication tests, formatting, review tooling and
the strict guide build with **172 completed routes**. The earlier 40-check
provider UI run remains explicitly scoped to its recorded provider-only binary.
Final-head GitHub review, current-main trusted CI and protected merge remain
separate gates.
