# Tag sources

Tag sources look up metadata, lyrics and artwork for selected files. Choose a
result, check its fields and file-to-track matches, then save the changes.

## Choose a source

| Source | Use it for | Setup |
|---|---|---|
| MusicBrainz | Release metadata and Cover Art Archive artwork | No key |
| [Discogs](discogs.md) | Releases, artist/track-title search, catalog numbers and artwork | TagGroove application credentials or a personal token |
| [Beatport / Traxsource](catalog.md) | Individual tracks, releases, mix/version metadata and artwork | No personal key; Traxsource may need an optional challenge solver |
| [iTunes](itunes.md) | Store album metadata and artwork | No key; choose the store country in Options |
| [Deezer](deezer.md) | Album metadata, ISRCs and artwork | No key |
| [LRCLIB](lrclib.md) | Lyrics matched to each selected file | No key |
| [AcoustID](acoustid.md) | Recordings identified by audio fingerprint | API key; fingerprinting libraries are included |
| Last.fm | Album metadata and listener tags for `GENRE` | API key |
| fanart.tv | Album covers and disc artwork | API key; covers only |
| freedb / freedb (local) | Disc metadata from a CDDB server or local database folder | Configure the server or local folder in Options |

**Covers: MusicBrainz…**, **Covers: Discogs…**, **Covers: iTunes…**,
**Covers: Deezer…** and **Covers: fanart.tv…** write artwork without changing
text fields. A provider may have no result for your edition.

## Search, review and save

1. Select one track or the files for one album. Check their order and existing tags.
2. Choose a source from **Tag Sources**, choose a search type, enter the
   identifying information and select **Search** or **Look up** for an ID.
3. Select a search result and choose **Next**. An ID lookup opens the review
   directly. **Preview** opens the source's page
   in your browser without writing tags.
4. In **Fields**, uncheck values to keep unchanged and edit proposed values
   where needed. Missing source values do not clear existing fields.
5. Check **Tracks and files**. Use **Auto-match**, **Selection order** or
   **Suggest sorting**, then drag files or use row arrows to correct matches.
   AcoustID retains fingerprint pairs; LRCLIB's per-file result keeps file order.
6. Choose whether to write artwork and replace existing covers. **Save image
   to file** also saves it beside the written files, using the configured
   [cover file name](../reference/options/tag-sources.md).
7. Select **Save tags to … files** or **Save cover to … files**. Its count
   shows how many files will be written.

Saving writes immediately. Unmatched files and files with no checked source
values or selected cover are skipped. An eligible successful desktop local-file import is one undo step;
use **Edit → Undo** and read the history notice. Snapshot/storage failures can
leave a successful write without Undo. Provider documents are outside the
desktop local-file journal. See [Undo and history](../guides/undo-history.md). **Cancel** leaves proposed tag changes unwritten.
**Save cover as…** saves the displayed image separately when requested;
canceling the tagging dialog does not cancel that separate save.

**Save image to file** replaces an existing image with the same name and
extension. **Replace existing artwork** controls embedded covers; it does
not protect an existing external image from that replacement.

Repeat the last source with ++cmd+i++ / ++ctrl+i++. Add Shift
(++cmd+shift+i++ / ++ctrl+shift+i++) to start its search immediately when
the required input is available.

## Compare fields from several sources

Select exactly one audio file and choose **Tag Sources → Compare sources
for one file…**. This compares Discogs, Beatport and Traxsource candidates before
writing anything.

1. Choose a source, check the prefilled artist and complete track title, including
   the mix/version, and select **Search**.
2. Choose a matching result. A track result supplies its identified track; for
   a release, select the exact track from its list. No search result is accepted
   automatically. Repeat with another source to add a comparison column.
3. Compare each source's values with **Current file**. Every **Save from** choice
   starts at **Keep current**. Choose a source separately for each field to import;
   fields absent from that source cannot be selected. Choosing a different
   candidate for the same source resets that source's field choices.
4. Select **Save chosen fields** for one save through the existing tag writer.
   Only the chosen fields are written. Other fields, including personal notes,
   remain unchanged. This comparison does not import artwork.

**Cancel** performs no writes. A failed source lookup leaves other sources
available. Changing the selected file invalidates the comparison; close
it and start again for the intended file. Normal platform and provider save-back rules, write errors and the desktop
[Undo limits](../guides/undo-history.md) still apply. Writable provider documents
use their existing [save-back and recovery path](../platforms/android.md),
rather than the desktop Undo journal.

## Set up API keys { #api-keys }

Open **Tag Sources → API keys…** for AcoustID, Last.fm and fanart.tv. Each
section says whether this build includes a key. Without a built-in or saved
key, the corresponding source stays unavailable.

Enter your key and choose **Save**. An empty box keeps the saved key;
**Remove the saved key** deletes it. Saved keys are not displayed again.
Your AcoustID or Last.fm key takes precedence over a built-in key. For
fanart.tv, your personal key accompanies a built-in project key; without a
project key, your key is used as the project key.

The dialog provides registration addresses. Availability depends on the
installed build and provider; do not assume every downloaded or locally
compiled build includes working keys.

## Last.fm

Choose **Tag Sources → Last.fm…**. **Artist and album** looks up that album
directly; **Album title words** searches by album title. **MusicBrainz release
ID** accepts a release MBID for a direct lookup.

The review shows available album and track metadata, cover art and filtered,
title-cased listener tags as `GENRE`. Choose the genre count in **Options →
Tag Sources → Online services**. Personal collection tags and bare numbers
are filtered, but review the remaining genres. Artwork is limited to the
images Last.fm supplies, typically up to 300 pixels.

## fanart.tv

Choose **Tag Sources → Covers: fanart.tv…**. Search by artist and album or
free text, or enter a MusicBrainz **release group** ID or address. A release
group collects editions of an album; an individual release address is refused.

Search results come from MusicBrainz; artwork comes from fanart.tv. Front
covers are ordered by likes, followed by disc artwork. Choose the image to
write to all selected files. This source supplies no text metadata.

## Network access and privacy

Online searches need a network connection. They send search terms and
identifiers to providers; per-file lyrics lookups send track metadata and
duration. AcoustID sends a locally computed fingerprint and duration, not
the audio file. **Preview** also contacts the source in your browser.
There is no downloaded offline mirror of these online catalogs. A configured
local freedb folder can supply disc metadata without a server lookup.

Personal API keys live in `tag-sources.json` in TagGroove's configuration folder,
separately from ordinary Options. They are text with restricted file permissions
where supported, not encrypted by TagGroove. Keep this file out of shared
configuration copies. Discogs also has an optional system keychain location.
Built-in application keys are part of the installed binary; they cannot keep
an individual's secret confidential from someone inspecting that binary.

TagGroove avoids putting credentials in displayed errors. Authentication still
sends the required credential to its provider over HTTPS; Last.fm uses a
request URL parameter. Consult provider terms before redistributing data or
artwork.

Requests are spaced separately by service. iTunes pauses several seconds
between requests; many per-file lookups or album pages take longer. Correct
missing-key errors, and wait before retrying quota or server errors. Starting
a newer source request supersedes older work without resetting provider quotas.

## Related

- [Beatport and Traxsource](catalog.md)
- [iTunes / Apple Music](itunes.md)
- [Deezer](deezer.md)
- [LRCLIB lyrics](lrclib.md)
- [AcoustID fingerprints](acoustid.md)
- [Tag Sources options](../reference/options/tag-sources.md)
