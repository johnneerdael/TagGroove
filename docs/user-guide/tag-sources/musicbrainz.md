# MusicBrainz

Use MusicBrainz to find release metadata and artwork for an album. A
**release** is a particular edition, which can differ from other editions in
country, date, format or tracklist. TagGroove needs no API key for this source.

Select local practice files, save intended pending edits and choose **Tag
Sources → MusicBrainz…**. For a first guided import, follow
[Tag an album from an online database](tag-from-online-sources.md).

## Find one track

Select the intended file and choose **Artist and track title** in **MusicBrainz…**.
Keep the full mix/version in the title, search, and check the exact track before
saving. Recording searches keep the selected recording and release identities. A recording without a release can supply its track fields without inventing an album.

Use [Compare metadata…](index.md#compare-fields-from-several-sources) to choose
individual fields from different providers for one track or an entire album.

## Choose a search mode

Use **Search by** to choose the information you know. **Artist** and **Album**
are prefilled from the selected files' most common album artist/artist and
album; correct them before searching when the existing tags are unreliable.

| Search mode | Enter | When it helps |
| --- | --- | --- |
| **Artist and album** | Artist and/or album text in the two boxes | Start with a recognizable album; either box can be used on its own. |
| **Free text** | Search words or a MusicBrainz release-search query | Refine a search when the basic pair is insufficient. |
| **Release ID (MBID) or address** | The release's MBID or a release-page address | Open a known edition directly without choosing from search results. |
| **Barcode** | The edition's barcode, retaining any leading zeros | Narrow candidates using packaging information; whitespace is removed. |
| **Catalog number** | The label's catalog number | Find editions sharing a particular catalog reference. |

Artist/album, barcode and catalog-number modes treat their entries as search
values. **Free text** passes a query to the provider's release search. For
example, `release:"Harbour Lights" AND artist:"Juniper Vale"` demonstrates
the query structure with invented values; it is not a promise of a real
database entry. Use your album's actual details. Start with the dedicated
search modes before learning more elaborate query syntax.

For a direct lookup, copy the address of a MusicBrainz **release**, whose
path contains `/release/`, or its 36-character MBID. A **release group**
collects editions and has `/release-group/` in its address. TagGroove refuses
a release-group address here: open one of its specific releases first.

Choose **Search**, or **Look up** when shown for a direct ID lookup. Search
results can be extended with **More results**. An ID lookup goes straight
to the review when the release is found.

## Identify the right edition

Compare artist, title, year, country, label, catalog number, format and track
count. The MusicBrainz score ranks a search match; it is not a verification of
your files. A deluxe edition can score highly while adding tracks you do not
have.

Select a result and choose **Preview** to inspect its source page in your
browser, or **Next** to open the import review. If the reviewed tracklist
does not fit, use **Back**. Do not use matching controls to disguise the
wrong release.

## Review fields and file assignments

In **Fields**, clear values you want to retain and check the fields you want
from the source. **All** and **None** change the set quickly. Check it on
each run: unchecked fields are remembered for the provider.

The source can provide these kinds of information when its record includes
them:

| Scope | Fields |
| --- | --- |
| Album description | `ALBUM`, `ALBUMARTIST`, `YEAR`, `GENRE`, `PUBLISHER`, `CATALOGNUMBER`, `BARCODE` |
| Release identity | `MUSICBRAINZ_ALBUMID`, `MUSICBRAINZ_RELEASEGROUPID`, `MUSICBRAINZ_ALBUMARTISTID` |
| Release classification | `MUSICBRAINZ_ALBUMRELEASECOUNTRY`, `MUSICBRAINZ_ALBUMSTATUS`, `MUSICBRAINZ_ALBUMTYPE` |
| Track information | `TITLE`, `ARTIST`, track/disc numbering, `COMPOSER`, `ISRC` |
| Track and work identity | `MUSICBRAINZ_TRACKID`, `MUSICBRAINZ_RELEASETRACKID`, `MUSICBRAINZ_ARTISTID`, `MUSICBRAINZ_WORKID` |

Availability depends on the release, recording and work data. The imported
`YEAR` is the year part of the selected release's date. A recording ID and a
release-track ID identify different things: the former identifies the
recording; the latter identifies its entry on this release. Do not swap
them merely because both are called track-related IDs.

Edit proposed album values directly. Choose **Edit per track**, or change
**Track column**, to inspect/edit differing track values in **Tracks and
files**. Separate multiple values with `\\` when using these editors.
An empty/missing proposed value is skipped; it does not request removal of
that existing field.

| Matching control | Use it when… | Still check… |
| --- | --- | --- |
| **Auto-match** | Existing disc/track numbers or titles are useful clues. | Ambiguous numbering and similar titles. The fallback can use selection order. |
| **Selection order** | You deliberately arranged the local files in release order. | Every row; positional pairing does not identify the recording. |
| **Suggest sorting → Title** | Names are more reliable than numbers. | Repeated or nearly identical titles. |
| **Suggest sorting → Length** | Durations help distinguish the files. | Different edits can have similar lengths. |
| Dragging or row arrows | A particular assignment needs correction. | Neighboring assignments after moving a file. |

A duration difference of more than 10 seconds is highlighted. It is a
clue to review, not an automatic rejection or proof of the right/wrong track.
**No file** means that source track has no assigned local file; **Not
written** means a selected file is outside the imported matches.

## Choose Cover Art Archive artwork

MusicBrainz artwork comes from the Cover Art Archive. The front cover can
appear first while the remaining image list loads. Choose the intended image
under **Artwork** and inspect it, rather than relying only on a thumbnail or
the order of the list.

Under **Tag Sources → Options…**, **MusicBrainz cover size** requests **250
pixels**, **500 pixels (default)**, **1200 pixels** or **Original size**.
The setting chooses the requested source variant; it does not resize images
already embedded in your files. Larger originals can require more transfer
and storage, and not every variant is guaranteed available.

| Choice | Result |
| --- | --- |
| **Write cover art** | Include the selected image in this import; clear to keep the import to text fields. |
| **Replace existing artwork** | Replace embedded pictures when writing the chosen cover; clear to add it alongside existing embedded artwork. |
| **Save image to file** | Also write an image beside the written files, using the configured cover filename and the actual image extension. |
| **Save cover as…** | Save the displayed image separately to a destination you choose. |

The default external basename is `folder`. An existing image at the resulting
name and extension can be replaced. **Replace existing artwork** protects the
embedded-cover choice only; it does not protect that separate external file.
**Save cover as…** performs its own save, so canceling the tag import does
not undo it.

For artwork without text changes, choose **Tag Sources → Covers:
MusicBrainz…**. After the same search, the review contains artwork controls
without the text-field and track-matching sections. The chosen image applies
to the selected files; confirm that all belong to the intended album.

## Save deliberately

Read the count on **Save tags to … files**, or **Save cover to … files** in
cover-only mode. Choose the button only after the fields, matches and cover
choices are right. Saving writes immediately. Unmatched files and files with
no chosen values or cover are skipped.

Read the operation result and verify saved values by reading the files back.
A source import can be one desktop Undo step in the current session; inspect
the available operation before relying on it. Keep backups separately.

## Gotchas

| Symptom | What to check |
| --- | --- |
| No suitable result | Search spelling, edition details and alternative identifiers. A missing database edition cannot be fixed by forcing the nearest result onto your files. |
| ID refused | Use a release MBID/address, not a release-group address. |
| Save count is smaller than the selection | Inspect unmatched rows and whether checked source fields actually have values. |
| Missing or slow artwork | The archive is a separate network source and can lack the image or respond slowly. Do not repeatedly import text merely to retry a cover. |
| Unexpected changed spelling | Review checked fields and edited per-track values; imports can replace existing values even when those values were already meaningful. |
| Command unavailable | Check selection, pending edits, busy state and source/platform restrictions. |

Searches send your entered terms and identifiers to the provider. **Preview**
also contacts its website in a browser. MusicBrainz lookup does not upload
the audio or identify it by listening; [AcoustID](acoustid.md) is the separate
fingerprinting source. Respect the provider's data/artwork terms and wait
before retrying rate-limit or service errors.

## Related

- [Online-tagging lesson](tag-from-online-sources.md)
- [Compare metadata from several sources](compare.md)
- [Shared tag-source workflow](index.md)
- [Tag Sources options](../reference/options/tag-sources.md)
- [Working with covers](../tagging/covers.md)
- [AcoustID fingerprints](acoustid.md)

<!-- Candidate evidence: src-tauri/src/sources/musicbrainz.rs search_query, release_id, parse_release, front_cover and parse_covers executed in a private standalone harness against invented JSON; actual src/tag-sources.ts searchModes, prefill, matching and buildApplyItems executed on that parsed output. Logs and source hashes are in private validation/. Illustrative query is not a live search. No provider requests, downloaded artwork, native save/UI, physical-device behavior or original-Mp3tag comparison were performed. Editorial/source review completed; provider/native-workflow qualification remains separate. -->
