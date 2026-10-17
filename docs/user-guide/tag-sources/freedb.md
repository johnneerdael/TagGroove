# Find album tags with freedb and gnudb

Use **Tag Sources → freedb…** to find album metadata by artist and title or
by the track layout of selected files. The original freedb service ended;
OpenTag's default server is gnudb. Treat a result as a suggestion: check its
edition, track order and values before writing tags.

## Search by artist and album

1. Save pending edits and select copied files from one disc.
2. Choose **Tag Sources → freedb…**, then **SEARCH BY → Artist and album**.
3. Enter the artist and album, such as `Juniper Vale` and `Harbour Lights`.
   Choose **Search**. The selected-file count helps narrow the request when
   it is between 1 and 99.
4. Compare the artist, album title, category and **Entry ID** shown in the
   results. Select a result and choose **Next**, or double-click its row, to
   load that entry into the import review. Year and track count are not
   supplied in this artist/album result list.

OpenTag provides artist/album and artwork search only for recognized gnudb
servers (`gnudb.org` or its subdomains). Use track-layout lookup for other
configured CDDB servers, even if they implement similar search commands. Choose the
server under [Options → Tag Sources → freedb](../reference/options/tag-sources-freedb.md).

## Search from the selected tracks

Choose **Track layout of the selected files**. OpenTag derives a disc layout
from file-list order and durations. Select one complete disc in track order,
with known durations for every file and at most 99 files. Exclude tracks from
other discs and unrelated bonus files.

This does not listen to the music or read a physical CD. Missing tracks,
different durations, another order or a different pressing can prevent a
match. Try artist/album search when layout lookup finds nothing.

Remote results show **Entry ID**, gnudb's identifier for a listed entry.
OpenTag asks for matches first and reads the entry you choose. Typed disc-ID
lookup is available only in a [local freedb database](local-freedb.md).

## Review before writing

After the entry loads, check its year when provided and compare its source
track list with the selected files. The review pairs those source tracks
with your files. Check the first and last track, as well as tracks with
similar titles.

- Use **Auto-match**, **Selection order**, or **Suggest sorting → Title** or
  **Length** to help arrange files. A suggested match still needs inspection.
- Drag a file to another row, use its arrow buttons, or press ++alt+up++ or
  ++alt+down++. A duration difference over ten seconds is highlighted.
- Enable only the fields you want to import. Edit album values or choose
  **Edit per track** to correct a track field before saving.
- Check **Write cover art** independently. **Replace existing artwork** affects
  embedded covers; **Save image to file** writes an external image beside
  the affected audio files.

Choose **Save tags to … files** after checking the plan. Inspect the saved
files and any reported per-file errors before continuing. OpenTag does not
submit corrections to the database.

## Find cover art only

Choose **Tag Sources → Covers: freedb (gnudb)…** to search by artist/album or
selected track layout. Inspect the artwork, then choose **Save cover to …
files**. This workflow selects artwork rather than album tag fields.

gnudb can list Cover Art Archive images in an entry or through a separate
artwork search. Album metadata does not guarantee an image. **MusicBrainz
cover size** under **Options → Tag Sources** also controls the image variant
requested here; it does not resize covers already embedded in files.

## Gotchas

| Result | Next step |
|---|---|
| No album match | Check the selected disc; try artist/album search or another source |
| `500 Unknown application` | The server may not have registered OpenTag; use a compatible server or a local snapshot |
| `429 Too Many Requests` | Wait before searching again; OpenTag does not automatically repeat the rejected request |
| Text/artwork search unavailable on a custom server | Select a recognized gnudb server for those searches, or use track-layout lookup |
| Incorrect database values | Choose another entry or edit the review; saving tags does not update gnudb |

Each server request includes OpenTag's application identity/version and a
handshake email identity. Configure your own address or use the documented
developer-contact fallback in the [server options](../reference/options/tag-sources-freedb.md).
Saving valid settings does not establish live service registration.

## Related

- [Server settings and disc-ID fields](../reference/options/tag-sources-freedb.md)
- [Search a local freedb snapshot](local-freedb.md)

<!-- Source: PR19 integrated source src/tag-sources.ts freedbModes/renderSearch/showReview/
renderCover; src-tauri/src/sources/freedb.rs and sources/mod.rs. Source and
existing fixtures; no live service or original-app runtime claim. -->
