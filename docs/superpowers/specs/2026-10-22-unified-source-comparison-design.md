# Unified metadata comparison for a track or album

Status: design for user review. Implementation has not started.

## Purpose and agreed behavior

Replace the narrow comparison shipped in PR #75 with an explicit workflow that
searches the available metadata sources and lets a person assemble the best
metadata for one track or one album before saving.

The user clarified these requirements on 2026-10-22:

- Support both single-track and album searches.
- Show existing values separately from choices offered by providers.
- Offer **Inherit**, **None**, and the available providers for every field.
- In album mode, choosing a provider for a field uses that provider for every
  track in the album. Resolve each track's own value from its matched source
  track; do not copy one track's value to every file.
- Keep **Artist** (`ARTIST`) separate from **Album artist** (`ALBUMARTIST`).
  Compilations can have VA/Various Artists as album artist and different
  performing artists on every track.
- Include standard and extended tags, including fields that only some sources
  supply. Do not constrain comparison to Discogs, Beatport, and Traxsource.
- Finish with a PR to main, resolve review findings until the final head is
  merge ready, then stop before merging.

The earlier catalog implementation plan is background evidence, not approval
to restrict this workflow to three providers or to declare this work complete.

## One choice model for both scopes

Use one field-choice map for the selected scope:

| Choice | Meaning for one track | Meaning for an album |
| --- | --- | --- |
| Inherit | Preserve that file's existing field, including absence | Preserve each file's own existing field independently |
| None | Remove that field; keep it absent if already absent | Remove that field from every selected album file |
| A provider | Use the field from that provider's matched track/release | Use that provider's corresponding value for each matched album track |

Default every field to **Inherit**. Inherit always means existing file metadata;
it never means another provider, an album default, or an inferred value.

Keep the existing value visible even after selecting a replacement. Distinguish
an empty field, a provider that supplies no value, and a failed lookup. A provider
value is its complete ordered string array, preserving multi-value tags.

Show a provider's coverage before choosing it. If it supplies a field for only
some matched tracks, preserve the existing values on the remaining tracks and
explicitly show those omissions in the preview. Never interpret missing data as
None or silently substitute a different provider. An unresolved track identity
is a matching problem, not an absent field: require it to be resolved before
using that source for track-level fields.

## Selection and searches

Expose **Compare metadata…** as a separate Tag Sources action. Opening it makes
no provider requests and changes no files. Offer Single track and Album scopes.
Single track requires exactly one selected file; Album operates on the explicitly
selected files of one album and displays the file count throughout. Do not edit
unselected or unloaded files, or silently combine different albums.

Single-track queries use performing artist and full title, including mix/version.
Album queries use album title and album artist, with album artist optional when
unknown. Do not replace a missing compilation album artist with the first or
most common performing artist. Treat VA aliases as matching hints only; never
rewrite stored artist names implicitly.

List the available sources with checkboxes and setup/status information. An
explicit **Search selected sources** action starts the chosen searches as one
session. Keep sources requiring credentials, configuration, disc context, or
extra search inputs visible with the reason. Offer fingerprint identification as
an explicit source choice and describe the extra per-file work for an album.

Search results populate independently. Show searching, found, needs match,
no result, needs setup, failed, and cancelled states separately. Retry only the
chosen failed source. Keep successful candidates and field choices intact.
Bound concurrent work and reuse existing shared provider request limits.

## Provider coverage

Use source capabilities rather than a hardcoded comparison-provider list.
Include every currently supported native source and every installed web source
as a distinct named source. Do not collapse different scripts into one `web`
candidate. Preserve service-specific authentication and country/options.

| Source | Track scope | Album scope |
| --- | --- | --- |
| MusicBrainz | Add recording artist/title discovery and preserve recording/release identity | Existing release search and full tracklist |
| Discogs | Existing artist/title discovery, followed by an exact release-track match | Existing release discovery and tracklist |
| Beatport / Traxsource | Existing track search, detail and exact parent release | Existing release search and tracklist |
| iTunes | Add song search and exact track/collection identity | Existing album search and tracklist |
| Deezer | Add track search/detail and exact album identity | Existing album search and tracklist |
| Last.fm | Add artist/title track search/detail; keep performing and album artists separate | Existing album discovery and tracklist |
| LRCLIB | Existing file signature/lyrics lookup | Resolve lyrics for each corresponding selected file |
| AcoustID | Existing fingerprint and recording matches | Identify selected files independently and preserve each file's recording association |
| freedb / local freedb | Use a known album/disc entry and explicitly matched track; explain required context | Existing disc/release discovery and tracklist; never invent a CD TOC |
| fanart.tv | Artwork for a matched release group; no invented text fields | Artwork for the matched release group |
| Installed `.src` sources | Resolve declared search inputs and retain the chosen track identity | Resolve declared inputs and match the selected release tracklist |

Show extra script input fields when its declared search cannot be derived from
the selected metadata. Do not make up an artist/title interface for every script.
Keep configured scripts available to this workflow without bypassing their
existing execution, credential, or HTTP policies.

## Matching and provenance

Present the best matching candidate per source with its artist, title, version,
release, year, catalog information, and track count where supplied. Explain
matching evidence and let the user inspect alternatives. Completeness is the
amount of supplied metadata, not proof of a correct match.

Do not accept a contradictory mix/version or an ambiguous identity merely
because it is first in the result list. Unique identity-backed suggestions can
be previewed directly; ambiguous candidates need a match choice before their
track fields become usable. Selecting a field remains explicit in both cases.

In album mode, retain one selected release per source and a mapping from local
file to that source's track. Show all selected local files against the proposed
source tracks, including disc positions, duration and performing artists.
Support reordered/missing tracks and multi-disc albums; do not map by row index
alone or reuse one remote track for multiple different files silently.

Choose a provider once per field for the whole album. The track list is for
matching and inspecting results, not a second per-track field-choice model.
Changing a source candidate or its mapping invalidates affected field choices
and clearly requests review; it must not silently change already chosen data.

## Editor layout

Extend TagGroove's current dialog styling, typography, colors, and keyboard
behavior. Use a resizable, scrollable workspace that also fits narrow windows.

1. Header: scope, selected track/album, file count, editable search terms.
2. Sources area: source selection, per-source progress, matched candidate and
   access to alternatives/track matching. Keep it collapsible after searching.
3. Fields area: a searchable list with **Field**, **Existing**, **Use**, and
   **Result** columns. Use is Inherit, None, or a provider; Result shows what
   that choice will save. Provider options show their value/coverage, not only
   their names. Expand long or multi-value alternatives without truncating data.
4. For albums, show a common existing/result value when equal, otherwise
   **Varies across N tracks** with an expandable per-track preview. Expand the
   same row to compare existing, provider, and resulting values per file.
5. Footer: changed-field/file counts, **Cancel**, and **Review and save**.

Discover fields from the union of existing metadata and all candidate metadata;
do not maintain an allowlist of importable text tags. Offer useful ordering and
filters for differences, changed fields, and incomplete provider coverage while
keeping every field reachable. Display the raw key alongside friendly labels
when needed to distinguish standard and extended fields. Treat physical audio
properties as read-only rather than writable tags.

Illustrative album choices (the preview contains each track's own values):

| Field | Existing | Use | Result |
| --- | --- | --- | --- |
| Album artist | VA | Discogs | Various Artists on all selected tracks |
| Artist | Varies across 12 tracks | MusicBrainz | Each matched track's performing artist |
| Title | Varies across 12 tracks | Beatport | Each matched track's own title and mix |
| BPM | Varies across 12 tracks | Inherit | Each file's existing BPM |
| Comment | Varies across 12 tracks | None | Empty on all selected tracks |

Support artwork through a visual row/section using existing thumbnails and
source candidates. Show existing artwork separately. Inherit preserves existing
embedded artwork, None removes it, and a provider choice previews the artwork
to import. Show the exact removal/replacement in final review; do not trigger
an external image-file save as a side effect of comparison.

## Architecture and save behavior

Separate provider acquisition, candidate/track mapping, field choices, and
rendering. Use stable source IDs, candidate revisions, and local file identities.
Represent choices as a tagged union (inherit, none, source), not overloaded
numeric dropdown indices. Build per-file write requests from the same immutable
preview used for final review.

Reuse the existing provider adapters, tag mapping, write preferences, diagnostic
reporting, desktop history, and mobile document save/recovery paths. Extend
adapters where true track discovery is missing. Keep existing single-source
workflows working. Keep comparison orchestration out of individual parsers.

Scope cancellation to a comparison session so parallel providers do not cancel
one another. Reject stale session/candidate/mapping responses and selection
changes. Recheck file/edit state before saving; never use a stale snapshot to
overwrite newer edits. Cancellation before save writes nothing.

Final review lists changed fields, removals, missing-value preservation, matches,
and affected files. Inherit emits no update; None emits an explicit removal;
a provider emits only that provider's mapped data. Use one user save action and
the existing batch history mechanism. Preserve actual per-file success/failure
semantics; do not claim an atomic all-or-nothing album transaction. Keep failed
files reviewable, avoid duplicate writes on retry, and report Undo limitations
accurately. Do not expose source keys, tokens, solver secrets, or raw credentials.

## Verification and completion gates

- Prove Inherit/None/provider behavior for absent, populated, and multi-value
  standard and arbitrary extended fields, with current data always visible.
- Exercise every listed source adapter and installed-source discovery; include
  credential failures and extra inputs instead of silently testing only three.
- Prove the same per-field provider choice resolves different values per album
  track, while shared release fields apply consistently.
- Test a VA compilation whose track artists all differ, including album artist
  queries, Artist imports, Album artist imports, Inherit, and None independently.
- Test multi-disc/reordered tracks, ambiguous versions, incomplete fields,
  missing matches, candidate changes, failed sources, cancellation, stale file
  edits/selection, partial save failures, and retry behavior.
- Visually inspect the actual editor with many providers, long lyrics/extended
  tags, differing album values, keyboard navigation, and narrow/desktop widths.
- Use invented offline fixtures for provider acquisition. Do not claim live
  provider availability from parser, fixture, or browser-host checks.
- Update affected user-guide pages and translations. Preserve application
  versions and run relevant local checks plus strict guide validation.
- Open the feature PR with substantive release notes. Resolve review findings
  and obtain a clean GitHub Codex code review on the exact final head plus all
  required Linux checks. Stop at merge readiness; do not merge or release.

## Evidence and research boundaries

Baseline: origin/main `43faff53fc710bb0610df59928519f68b2eeefea`.
`src/tag-sources.ts` currently hardcodes three comparison providers and uses a
single-file guard; `src-tauri/src/sources/model.rs` and provider modules show the
larger source set. Current MusicBrainz, iTunes and Deezer adapters reject track
search mode. Existing album parsers already distinguish release-level album
artist from track-level artist; preserve that separation during integration.

Official references consulted on 2026-10-22:

- [MusicBrainz search API](https://musicbrainz.org/doc/MusicBrainz_API/Search):
  recording discovery supports recording and artist criteria.
- [Apple iTunes Search API](https://performance-partners.apple.com/search-api):
  song searches and ID-based collection/track lookups are available; retain
  storefront selection and the shared request budget.
- [Last.fm track.search](https://www.last.fm/api/show/track.search) and
  [track.getInfo](https://www.last.fm/api/show/track.getInfo): artist/title
  discovery and track metadata are distinct from album discovery.

Deezer's primary developer documentation was inaccessible through the browsing
tool during this design pass. Verify its exact track query/detail contract
against primary evidence and existing source fixtures during planning; do not
substitute album-only search and call track support complete. No provider
metadata searches or user audio uploads were performed during this pass.
