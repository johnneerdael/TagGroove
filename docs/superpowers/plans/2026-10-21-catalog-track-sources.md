# Catalog Track Sources Implementation Plan

> **For agentic workers:** Use the ce:work execution workflow, with parallel independent implementation units and focused review before completion.

**Goal:** Support native Beatport/Traxsource track and release metadata in TagGroove, Discogs artist/title search, and explicit per-field source comparison; complete Traxsource release acquisition in CueRithm.

**Architecture:** Keep TagGroove's source review, native file writer and Undo. Adapt the GPLv3 OneTagger-derived catalog acquisition code from CueRithm into a small metadata-only crate; retain upstream notices and mark the combined application accordingly. Keep the separately hosted challenge solver optional. Track lookup must preserve the original track number and release identity; opening its release retrieves the full tracklist.

**Tech stack:** Rust, existing reqwest/rustls, TypeScript, existing Tauri UI and offline HTTP/HTML fixtures.

**Spec:** User-approved scope in this session: native track/release providers, direct track-to-release navigation from the supplied HAR, comparison before saving, shared Discogs credentials and maximum 25 requests/minute for application access or 60 for authenticated users. Consider GPL-compatible direct reuse. No live provider lookups.

## Global constraints

- Preserve existing versions and application file-writing safeguards.
- Never publish the HAR, cookies, captured headers, credentials, audio or local user paths; use minimal invented fixture data.
- Discogs artist/title search must reuse the existing application/user credential path and a single service gate for all API calls.
- Preserve existing MIT component notices; make GPL-derived source and combined application licensing explicit.
- Distinguish empty results, malformed data, challenge failures and cancellation.
- Keep UI consistent with existing Tag Sources dialogs and ensure Cancel performs no writes.

## Review focus

- A single track must retain its actual release position and mix/version.
- A track result must open its own release, never a similarly titled release.
- Comparison must preserve current fields by default and keep candidates tied to a specific local file.
- Concurrent Discogs search/detail requests must share the same request budget.
- A blocked/malformed Traxsource response must never be accepted as an empty album.

## Shared interface

- Extend TagGroove `Provider` with `beatport` and `traxsource`.
- Extend `SearchMode` with `artist_title` (track search) and `release` (release search). Add `title: String` to `SearchQuery` with a default.
- Catalog result IDs are `track:<numeric-id>` or `release:<numeric-id>`; ordinary providers retain their existing IDs.
- Add optional `release_id` and `release_title` to `SearchResult`; all old provider results default to absent.
- A loaded catalog track is a one-track `SourceRelease`, with `BEATPORT_RELEASE_ID`/`TRAXSOURCE_RELEASE_ID` in its fields. Opening a release passes `release:<id>` to the existing `tag_source_release` command.
- Catalog backend adapter exposes `search(provider, query, ticket)` and `release(provider, input, ticket)` returning existing source models. Transport must support cancellation and user's solver settings.

## Unit A: Catalog acquisition (independent backend)

**Files:** new `crates/catalog-sources/`, new `src-tauri/src/sources/catalog.rs`; Cargo manifests for this dependency only.
**Goal:** Beatport track/release search and detail, Traxsource track/release search and detail, shared challenge solver transport adapted from CueRithm without its tag writer/library.
**Approach:** Port minimal metadata structures, parsers and transport. Give the Tauri adapter source models; source dispatch remains Unit C's responsibility. Read local CueRithm providers and supplied HAR only for behavior/structure.
**Execution note:** Test first; prove meaningful failures before implementation.
**Tests:** invented JSON/HTML, pagination, multiple release tracks, exact track/release IDs, selected single-track numbering, empty vs malformed/challenge, bounded solver retry and cancellation.
**Verification:** focused Rust crate tests and formatting; no provider network.
- [x] Write and observe failing tests.
- [x] Implement and pass tests; report adapter interface to integrator.

## Unit B: TagGroove search and comparison UI

**Files:** `src/tag-sources.ts`, new comparison module if useful, source CSS, source-related frontend tests; relevant menu wiring in `src/menubar.ts`/`src/main.ts`/`src/toolbar.ts` as needed.
**Goal:** Expose both providers, artist/title and release search, track-to-release navigation, and single-file multi-source field comparison before one save.
**Approach:** Follow current dialogs; comparison holds candidate values in memory, chooses a source separately per field and defaults to Keep current. Save uses existing host.apply once. Reject cross-file reuse and preserve source candidate identity; no automatic cross-provider candidate choice. Handle unavailable providers independently.
**Execution note:** Test first, include DOM interaction tests, preserve old behavior tests.
**Tests:** query prefill for a selected track, provider routing, release navigation, selecting exact track from full release, field selection/current preservation, Cancel no write, one Save, errors and late responses.
**Verification:** focused node tests and TypeScript; screenshot review when integrated.
- [x] Write and observe failing tests.
- [x] Implement search and comparison; pass tests.

## Unit C: Integration and Discogs (coordinator)

**Files:** source model, source dispatch/keys/http, Discogs and affected other provider exhaustive matches, docs, license metadata, generated i18n as required.
**Goal:** Integrate catalog backend/frontend, native Discogs artist/title query, correct authentication-tier rate limiting, complete documentation and license provenance.
**Approach:** Keep all Discogs requests behind the existing shared gate. App auth uses 2500ms spacing, user token 1100ms. Add typed track search to Discogs release discovery and expose exact matching tracks for review; do not auto-save a release's first track. Support provider settings without leaking credentials into solver calls.
**Execution note:** Test first for query/limiter behavior; integration compile then fix exhaustive cases.
**Tests:** app credentials stay in slower tier, user token faster tier, query encodes track vs release separately, track identity survives adapter mapping.
**Verification:** frontend suites/build, affected Rust tests and Clippy, strict user-guide build, diff check; report native/CI limitations precisely.
- [x] Establish tests and integrate models/dispatch.
- [x] Correct Discogs behavior.
- [x] Finish docs, licensing, review and validation.

## Unit D: CueRithm Traxsource releases (separate repository)

**Files:** CueRithm `vendor/onetagger/providers/traxsource.rs`, `crates/cuerithm-autotag/`, album context/preparation where Beatport-only, relevant tests and user guide.
**Goal:** Retrieve complete Traxsource releases and use a matched track's exact release ID for album matching; expose release search through appropriate native API.
**Approach:** Existing CatalogHttp/solver routes; shared track parser on search/release HTML; capture-derived structure with invented fixtures. Preserve Beatport behavior and all analysis calculations.
**Execution note:** Test first; do not copy/publish captured responses.
**Tests:** full release track order/metadata, selected track's own release, malformed/empty/challenged pages, no unrelated track leakage, cancellable solver transport, album hint provider identity.
**Verification:** focused autotag/library tests and fmt/Clippy as practical using existing shared build cache; no live requests.
- [x] Write and observe failing tests.
- [x] Implement and pass tests; document behavior and limits.
