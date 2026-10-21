# Unified Source Comparison Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the approved single-track/album comparison editor across all supported sources, with visible provider values, Inherit, None, and accurate per-track album application.

**Architecture:** Keep acquisition in existing source adapters, introduce a small pure comparison model and a source-session coordinator, and replace the existing comparison dialog with the approved editor. Reuse the existing native import writer, history and document recovery rather than introducing another write pipeline. An album has one provider choice per field and a separate mapping to that provider's corresponding track for each local file.

**Tech Stack:** Existing TypeScript, DOM/CSS, Rust/Tauri, reqwest, Node test runner/happy-dom, Rust unit tests, and MkDocs. Add no product dependency.

**Spec:** `docs/superpowers/specs/2026-10-22-unified-source-comparison-design.md`. The revised mockup was approved on 2026-10-22; its provider selector has an explicit read-only value field beside each provider, using track 1 as the album example.

## Global Constraints

- Support both single-track and album searches.
- Show existing values separately from choices offered by providers.
- Offer **Inherit**, **None**, and the available providers for every field.
- Default every field to **Inherit**.
- Inherit always means existing file metadata; it never means another provider, an album default, or an inferred value.
- In album mode, choosing a provider for a field uses that provider for every track in the album. Resolve each track's own value from its matched source track; do not copy one track's value to every file.
- Keep **Artist** (`ARTIST`) separate from **Album artist** (`ALBUMARTIST`).
- Include standard and extended tags, including fields that only some sources supply. Do not constrain comparison to Discogs, Beatport, and Traxsource.
- Opening the comparison makes no provider requests and changes no files.
- Do not edit unselected or unloaded files, or silently combine different albums.
- Cancellation before save writes nothing.
- Use invented offline fixtures; do not claim live availability from offline checks.
- Preserve current application versions, credential policies, rate gates, tag mappings, Undo limitations and document recovery behavior.
- Finish with a PR to main, resolve review findings until the final head is merge ready, then stop before merging.

## Review Focus

- A compilation with different performing artists must never receive VA as every track's ARTIST (Tasks 1, 2 and 6).
- A provider returning some missing values must not clear those fields or borrow another provider's values (Tasks 1 and 6).
- Concurrent sources, installed scripts and retried lookups must not cancel peers or replace choices with stale candidate data (Tasks 3 and 6).
- A changed local file, changed selection or changed candidate must invalidate the affected preview before writing (Tasks 3, 4 and 6).
- A partially failed album save must keep failures visible and retry only failed files, without writing successful files again (Tasks 4 and 6).

---

## File responsibilities and shared contracts

- `src/metadata-comparison.ts`: pure field choices, effective source values, conservative track mapping, and immutable per-file preview.
- `src/metadata-comparison-sources.ts`: native/installed-source capabilities, query construction and the cancellable multi-source session.
- `src/metadata-comparison-ui.ts`: the production dialog, selector, matching controls, review and partial-save states.
- `src/tag-sources.ts`: existing single-source dialogs and the comparison entry point; share types through type-only imports to avoid a runtime import cycle.
- `src-tauri/src/sources/comparison.rs`: comparison snapshot lifecycle using existing native file identities.
- Existing provider modules: their own track discovery/parsing only.
- Existing `sources/mod.rs` import writer: explicit cover removal and comparison snapshot checks inside the existing history/document path.

Export these TypeScript contracts from `src/metadata-comparison.ts`:

```ts
type ComparisonScope = 'track' | 'album';
type ComparisonChoice = {kind:'inherit'} | {kind:'none'} |
  {kind:'source';sourceId:string;revision:number};
type ComparisonCandidate = {
  sourceId:string;revision:number;release:SourceRelease;
  tracks:Record<string,number|null>; // local file path -> source track index
};
type ComparisonPreview = {
  items:ApplyItem[];cover:ApplyCover|null;
  missing:{path:string;field:string;sourceId:string}[];
  blocked:{path:string;field:string;reason:string}[];
};
```

Use `native:<provider>` and `web:<installed-script-id>` source IDs. Keep candidate
revision and native snapshot token separate. Extend existing `ApplyItem` with
optional `remove_covers:boolean` and `comparison_snapshot:string`; old callers
omit both. Use one existing global `ApplyCover` for a chosen album/release image;
do not introduce per-file image selection or external image-file saves.

## Task 1: Pure choice and album application model

**Files:** create `src/metadata-comparison.ts`, `tests/metadata-comparison.test.mjs`; reuse `tests/helpers/load-production-module.mjs`. Export needed existing types from `src/tag-sources.ts` without changing old behavior.

**Interfaces:** produce `comparisonFields(files,candidates):string[]`, `comparisonExample(files,candidate,field):string[]|null`, `comparisonTrackMatches(files,release,trackId?):Record<string,number|null>`, `buildComparisonPreview(files,candidates,choices):ComparisonPreview`, and `comparisonSelectionKey(files):string`.

- [ ] Write failing tests using two compilation files: ALBUMARTIST=`VA` on both, ARTIST=`Artist A`/`Artist B`, and a source with ALBUMARTIST=`Various Artists`, ARTIST=`Artist A feat. Guest`/`Artist B`.
- [ ] Assert Inherit emits no update; None emits `FIELD:[]`; a source-selected ARTIST yields the two different source artists, while source-selected ALBUMARTIST yields `Various Artists` on both. Assert arbitrary multi-value `DJ_COLLECTION` retains its full array.
- [ ] Assert track-1 examples read only the first local file's mapped source track, including when the source release is reordered. Assert missing first-track data remains missing rather than showing a later track's value.
- [ ] Assert union-based field discovery, missing-value preservation, unmapped-track blockers, candidate-revision blockers, and wrong-mix rejection. Require identity or an unambiguous artist/title/version match; treat positional guesses as suggestions requiring confirmation.
- [ ] Run `node --test tests/metadata-comparison.test.mjs` and observe the missing-module/API failures.
- [ ] Implement the pure functions. Do not infer ALBUMARTIST from a compilation's first performing artist. Use existing provider identity tags and supplied search track identity; never map by track number alone.
- [ ] Run the new tests and existing `tests/tag-sources.test.mjs`; require zero failures. Commit the model and tests.

## Task 2: Genuine track discovery in the remaining native adapters

**Files:** modify `src-tauri/src/sources/{model,musicbrainz,itunes,deezer,lastfm,mod}.rs`, `src/tag-sources.ts`; update fixture producers/constructors affected by an optional search-result field. Keep existing Beatport/Traxsource and Discogs behavior.

**Interfaces:** add optional serialized `SearchResult.track_id:Option<String>` and its TypeScript counterpart. Keep `SearchResult.id` valid for its existing release/detail loader and `release_id` as the parent release identity where known. Add `ArtistTitle` support to each provider's existing query builder and parse track results through `parse_track_search(body:&Value)->Result<SearchPage,String>`. Last.fm also produces `track_info_url(id:&str,key:&str)->Result<String,String>` and `parse_track(body:&Value,id:&str,genres:usize)->Result<SourceRelease,String>` for an opaque artist/title track ID.

- [ ] Write failing query/parser tests for MusicBrainz recording searches, iTunes song searches, Deezer track searches and Last.fm artist/title searches. Include encoded punctuation, full mix names and a VA compilation whose performing artist differs from album artist.
- [ ] Preserve exact track identity and parent release identity. MusicBrainz recording results must retain the selected recording and release; iTunes song results retain trackId and collectionId; Deezer results retain track ID and album ID. Add their native track IDs to parsed track fields when needed to resolve a loaded release.
- [ ] Add `musicbrainz::recording_url(id:&str)->String` and `parse_recording(body:&Value,size:CoverSize)->Result<SourceRelease,String>` for a recording without an available release. Route an opaque `recording:<MBID>` through the existing detail command and return its actual track metadata without inventing an album. For Deezer, retain an opaque `track:<numeric-id>` detail path that reads the selected track's metadata and its exact parent album; do not lose track-only BPM/ISRC data by substituting an album-list row.
- [ ] For Last.fm, route the track ID to track.getInfo while keeping ordinary album IDs on album.getInfo. Do not expose the API key in IDs or tag fields. Keep track artist separate from available album metadata.
- [ ] Run `cargo test -p opentag --lib sources:: --locked` and observe the new assertions failing for unsupported track modes or missing identity.
- [ ] Implement the new branches behind the same native transport, credential and request gates. Keep album pagination/completeness checks. Do not replace Deezer's album-only search with an album-title approximation for track mode; use its track-search/detail contract and distinguish provider errors from no matches. Verify primary documentation if accessible; record any remaining live-compatibility qualification rather than inventing evidence.
- [ ] Expose the new track/release search choices in ordinary source dialogs and preserve the selected search track when its full release loads. Use explicit matching if an exact recording occurs more than once on the release.
- [ ] Run focused native source tests and `node --test tests/catalog-sources-ui.test.mjs tests/tag-sources.test.mjs`; require zero failures. Commit this provider change independently.

## Task 3: All-source discovery and multi-source session

**Files:** create `src/metadata-comparison-sources.ts`, `tests/metadata-comparison-sources.test.mjs`; modify host wiring in `src/tag-sources.ts` as needed.

**Interfaces:** produce `comparisonSources(credentials,webListing):ComparisonSource[]`, `comparisonQuery(source,scope,files,terms):SearchQuery|NeedsInput`, and `ComparisonSearchSession`. Its constructor takes `{invoke,evaluate,nextTicket,onSourceState}`; `search(sources,scope,files,terms):Promise<void>`, `load(sourceId,result):Promise<void>`, `retry(sourceId):Promise<void>`, `cancel():void` own one session ticket and per-source request revisions. Source states distinguish setup/input requirements, searching, candidates, needs-match, ready, empty, failed and cancelled.

- [ ] Write table-driven tests covering every native Provider and two installed scripts with different IDs. Assert their capability/setup reasons, real query modes, and distinct candidate storage. Compare the covered provider set with the actual Provider type so adding a provider requires a deliberate capability decision.
- [ ] Assert album queries use ALBUMARTIST/ALBUM and never a compilation's first ARTIST; track queries use ARTIST/full TITLE. LRCLIB gets file signatures, AcoustID gets fingerprint mode, freedb uses valid album/disc context, and fanart remains artwork-only.
- [ ] Resolve installed scripts' declared input formats through the existing host evaluator. Use `web_sources_list`, `web_source_search({source,values,ticket})` and `web_source_release({source,url,previewUrl,ticket})`; preserve direct scripts, authored names, structured failures and registered-result URL checks. Expose unresolved extra inputs rather than silently omitting the source.
- [ ] Assert three active source requests at most, one shared session ticket for Search/Release streams, independent source failures, failed-source-only retry, cancellation, and late responses after a retry/candidate replacement. A new source request must not cancel another source's work.
- [ ] Run `node --test tests/metadata-comparison-sources.test.mjs` and observe failing assertions, then implement the coordinator using existing backend IPC. Reuse the same session ticket for parallel provider claims; do not redesign HTTP rate gates.
- [ ] Load only bounded suggested candidate detail, not every returned release. Keep alternatives accessible. Use Task 1 mapping for unique suggestions and explicit needs-match state for ambiguity; retain AcoustID's per-file recording associations and LRCLIB's original file order.
- [ ] Run both comparison-model and source-session suites; require zero failures. Commit the coordinator and tests.

## Task 4: Snapshot checks, None and artwork through the existing writer

**Files:** create `src-tauri/src/sources/comparison.rs`; modify `src-tauri/src/sources/mod.rs`, `src-tauri/src/lib.rs`, `src/tag-sources.ts`, `src/main.ts`, and `tests/tag-source-apply.test.mjs`.

**Interfaces:** add `tag_source_comparison_snapshot(paths:Vec<String>,ticket:u64)->Result<String,String>`. Store one active bounded comparison snapshot with opaque token, session ticket, captured `tag_core::file_copy::FileIdentity`, and metadata/cover state for each requested path. Add serde-defaulted `ApplyItem.remove_covers:bool` and `ApplyItem.comparison_snapshot:Option<String>`. Existing `tag_source_apply` IPC stays backward compatible. Export `ComparisonSaveOutcome={succeeded:string[];failed:{path:string;error:string}[]}` to the UI through `applyComparison(items,cover):Promise<ComparisonSaveOutcome>`.

- [ ] Add failing native tests for a populated text field cleared with `[]`, a cover-only None removal, unchanged covers with Inherit, and explicit source artwork replacement. Reject contradictory remove/import artwork requests. No-op inherited files must not be written or journaled.
- [ ] Add failing tests for changed local metadata/file identity between snapshot and save, an unapproved path/token, cancellation/obsolete session, and changed provider-document metadata through its existing working-document path. Refuse stale writes before mutation.
- [ ] Add a partial failure test: one file succeeds, one fails, and retry submits only the failed path under the same still-valid snapshot. Consume successful snapshot targets; never authorize duplicate application to completed targets.
- [ ] Run `cargo test -p opentag --lib sources:: --locked` and `node --test tests/tag-source-apply.test.mjs` to observe the new failures.
- [ ] Implement snapshot checks inside the existing import/history/document mutation path, immediately before applying chosen updates. Release snapshots on session cancellation/close. Keep snapshots in memory only and serialize no native file identities to the frontend.
- [ ] Extend `apply_one`/item validation for explicit cover removal. Reuse existing image download/validation and tag write preferences. Comparison imports set no external `save_as`; do not create a second writer or remove existing ordinary-source options.
- [ ] Factor main's import callback so normal source dialogs retain their existing cover-confirmation behavior, while comparison uses the explicit reviewed cover action. Return structured successes/failures to comparison instead of hiding partial successes behind a generic exception. Preserve diagnostics and refresh affected entries.
- [ ] Run the native source, tag-source-apply, history and save-acceptance tests affected by this change; require zero failures. Commit the write-path change.

## Task 5: Approved production editor and visible provider value fields

**Files:** create `src/metadata-comparison-ui.ts`, `tests/metadata-comparison-ui.test.mjs`; modify `src/tag-sources.ts`, `src/style.css`, `src/main.ts`, `src/toolbar.ts`, and affected source/i18n test loaders.

**Interfaces:** `MetadataComparisonUI` consumes `{files,reason,invoke,evaluate,loadExistingCovers,applyComparison,nextTicket}` plus Task 1/3 contracts; `open():Promise<void>` owns and closes the dialog. `loadExistingCovers(path):Promise<{digest:string;thumbnail:string;description:string}[]>` adapts existing cover hydration rather than retaining large images in tag snapshots. `TagSourcesUI.compare()` delegates to this editor and retains its existing single-dialog guard.

- [ ] Add failing DOM tests for both scopes, separate existing values, and Inherit defaults. Assert selecting more than one file enables Album and does not preserve the obsolete exact-one-file comparison guard.
- [ ] Assert the open field selector contains one visible read-only input/textarea for every offered provider. In Album mode it shows the first local track's mapped value and explicitly labels it as the track-1 example. A missing first-track value must show no value, even when later tracks have data. Test lyrics and multi-value extended tags without truncation.
- [ ] Assert Inherit, None and provider choices stay distinct; Artist and Album artist use distinct rows and queries. Use a compilation fixture where every track ARTIST differs. The choice is once per field for the album, not a per-track choice editor.
- [ ] Run `node --test tests/metadata-comparison-ui.test.mjs` and observe the new DOM failures before implementing the editor.
- [ ] Add the field list, source statuses/inputs, candidate alternatives, album track-matching view, searchable full field union and per-track result inspection. Use existing tokens and dialog behavior, including theme, keyboard navigation, translated labels and narrow-window scrolling.
- [ ] Render artwork choices with existing/provider thumbnails; Inherit preserves, None removes, and a provider imports the reviewed image. Keep normal source menus separate and rename this action to **Compare metadata…**.
- [ ] Reuse `tests/helpers/load-production-module.mjs` for new runtime imports; update existing data-URL test loading where the delegated comparison import requires it. Run `node --test tests/metadata-comparison-ui.test.mjs` and pass the suite.
- [ ] Commit the editor, styles and integration tests.

## Task 6: Review, cancellation and partial-save integration

**Files:** extend `tests/metadata-comparison-ui.test.mjs`, `tests/metadata-comparison-sources.test.mjs`, `tests/catalog-sources-ui.test.mjs`, `tests/tag-source-apply.test.mjs`; finish production callbacks in the preceding tasks' modules.

- [ ] Add failing end-to-end DOM/IPC-host tests for explicit Search only, no request on open, one source failure beside successful sources, candidate replacement, remapping, Cancel, and late response disposal.
- [ ] Change the host file selection/fields during a pending search and just before Save; assert no stale write. Verify native snapshot rejection remains visible without discarding the user's choices.
- [ ] Select MusicBrainz ARTIST, Discogs ALBUMARTIST and None COMMENT for a reordered, multi-disc compilation. Assert exact per-file requests for every selected file and no request for unselected files. Include a sparse extended field and preserve its existing value where the chosen provider lacks data.
- [ ] Exercise final review: display removals, missing-value preservation, affected paths and the exact immutable preview passed to the writer. A double click must not start a second save.
- [ ] Simulate one successful and one failed write. Freeze the reviewed batch, show completed/failed targets separately, and make **Retry failed files** submit only failures. Do not close the dialog or rebuild successful targets into retry requests.
- [ ] Run the four focused comparison/source-apply suites before and after fixes; require zero failures. Commit only meaningful integration fixes/tests.

## Task 7: Documentation, translations and visual verification

**Files:** update `docs/user-guide/tag-sources/{index,catalog,discogs,itunes,deezer,lrclib,acoustid}.md`, `docs/user-guide/reference/{menus,options}/tag-sources.md`, relevant overview/Last.fm prose, `scripts/i18n-extract.mjs` and `src/i18n/catalogs/{en,nl,de}.json` when affected. Update `tests/guide-online-source-lesson.test.mjs` only for actual changed instructions.

- [ ] Teach the approved model in both scopes, with VA/Artist separation, track-1 example fields, partial coverage, matches, None removal, review, retries and platform-specific save/Undo limits. Describe setup requirements without claiming every provider supplies every field.
- [ ] Include the new production modules in extraction and update English/Dutch/German source text. Do not translate returned metadata, source-authored script names, raw tag keys or file values.
- [ ] Run `npm run i18n:extract`, relevant i18n/guide tests, `python3 scripts/docs-status.py`, `python3 -m unittest discover -s docs/tests -v`, and `python3 -m mkdocs build --strict`; require no missing extraction keys, writing-plan omissions or strict guide errors.
- [ ] Open the actual production editor through an offline fake host. Verify the approved provider/value selector, both scopes, long fields, many sources, sample-track mapping, themes, keyboard use and narrow/desktop widths. Capture screenshots of those states. A fake-host preview proves layout/interaction, not packaged native or live-provider operation.
- [ ] Commit docs/translations and any visual corrections; do not bump an application version or add an unsolicited release section.

## Task 8: Final branch validation and reviewed PR

**Files:** final implementation diff and PR metadata only; no release/workflow changes are planned.

- [ ] Self-review the final diff against every spec requirement and the approved mockup. Run a fresh code review of the whole final change, with special attention to the five Review Focus items. Fix findings and run checks appropriate to each fix.
- [ ] Run `npm run test:ui`, `npm run build`, `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, the required documentation checks, and `git diff --check`. Record what actually passed; do not substitute fixture counts for requirement coverage.
- [ ] Commit and push the feature branch; create one PR to main with substantive user-facing release notes, validation, documentation impact and links to issues #70/#72. Keep versions and unrelated local `docs/community/` files untouched.
- [ ] Request GitHub Codex code review. Address findings, resolve threads and request a new review after every final-head change. Require a clean authenticated code-review completion on the exact final SHA.
- [ ] Use `review-controller.yml` as documented to authorize required Linux checks on the reviewed head/current base/test merge. Investigate failures and rerun only failed jobs; do not bypass the gate or rebuild successful native packages.
- [ ] Inspect live PR review state, unresolved threads and required checks for that final head. Stop with the PR merge ready. Do not merge, release, deploy or close the feature issues during this task.

## Self-review and execution handoff

Coverage: Tasks 1/5 cover fields, current values and the approved selector;
Task 2 covers genuine native track discovery; Task 3 covers every native/installed
source and session behavior; Tasks 1/3/6 cover album matching and consistent
field choices; Task 4 covers saving/removal/history/recovery; Tasks 6/7 cover
failure states and the rendered product; Task 8 covers the requested PR gate.
All five Review Focus cases have explicit tests above. No product dependency or
new release machinery is needed.

Recommend **Native** execution in this session because the tasks share existing
adapter and import interfaces. Execute sequentially with focused tests, then a
fresh whole-branch review and the mandatory GitHub Codex review. The alternative
is subagent-driven execution with a fresh implementer/reviewer per task. User
review of this plan and execution-method selection are the remaining handoff.
