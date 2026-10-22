# Search a local freedb snapshot

A local freedb database is an extracted collection of historical album
entries. Use it when you already have a snapshot and want lookup without a
remote CDDB server. It reflects the snapshot's date; building a text index
does not add newer albums.

<!-- figure:source-local-freedb -->
<figure markdown>
![The local freedb dialog with a disc ID or artist and album words](../assets/screens/source-local-freedb-light.png#only-light)
![The local freedb dialog with a disc ID or artist and album words](../assets/screens/source-local-freedb-dark.png#only-dark)
</figure>
<!-- /figure -->

## Choose the database folder

1. Extract the snapshot into a folder the app can read.
2. Open **File → Options… → Tag Sources → Local freedb**.
3. Use **Choose folder…**, or type the full path, to select the root containing
   category folders such as `rock`, `classical` and `blues`.
4. Choose **OK** to save that folder for later searches.

TagGroove reads one-xmcd-file-per-disc snapshots and Windows `xxtoyy` range
files. Select their database root, not a category or individual entry. If an
entry is below `D:\Databases\freedb\rock`, choose `D:\Databases\freedb`.
A compressed download is not itself the database folder.

## Choose a lookup method

Select files from one disc and open **Tag Sources → freedb (local)…**.

| **SEARCH BY** | Input | Text index needed? |
|---|---|---|
| **Disc ID of the selected files** | ID derived from file-list order and durations | No |
| **freedb disc ID** | An existing freedb disc ID entered in **DISC ID** | No |
| **Artist and album words** | Words in indexed artist/album titles | Yes |

For selected-file lookup, select one complete disc in track order, with
known durations and at most 99 files. This is layout matching, not acoustic
fingerprinting. TagGroove supports deriving this local ID; the historical
Mp3tag restriction on local audio-based lookup does not describe its reader.

A freedb disc ID is different from gnudb's listed **Entry ID**, a barcode or
a MusicBrainz release identifier. If you do not know the ID, try the selected
files or build the text index.

## Build and use the text index

Open **Options → Tag Sources → Local freedb**, check **LOCAL FREEDB FOLDER**,
and choose **Build index**. Wait for the status to show a disc count for that
folder. Large snapshots take time; **Cancel** requests cancellation. An
unfinished build does not replace the previously completed index.

Building starts immediately with the entered folder, before Options **OK**.
Cancelling the outer Options dialog does not undo a completed build. Save
Options if later searches should use this same folder.

Now choose **Artist and album words** in **freedb (local)…**. For an indexed
entry `Juniper Vale / Harbour Lights`, try `juniper harbour`. All query words
must match artist/title text. Case and accents are normalized; a partial
word can match a longer word. This searches database entries, not your audio.

Rebuild after changing the root or replacing snapshot contents. An index
records its root, so one built for another folder is rejected. Changes to
database files are not indexed automatically. If the current environment
does not provide indexing, the page reports it and disables the control.

## Review and troubleshoot

Choose a result and **Next**. Compare source tracks with your files, correct
the pairing and select fields to write. The
[import review](freedb.md#review-before-writing) lets you edit values before
choosing **Save tags to … files**.

- No text results: check that the completed index belongs to this folder,
  then try fewer artist/title words.
- No disc-ID result: check order, durations and whether this snapshot contains
  the release. Rebuilding an unchanged snapshot cannot add a missing disc.
- Wrong folder: select the database root, with category subfolders inside it.

These lookups read the local database. They do not upload corrections or
update it from gnudb, and the database is separate from TagGroove's music cache.

## Related

- [Local freedb options](../reference/options/tag-sources-local-freedb.md)
- [Remote freedb and gnudb](freedb.md)

<!-- Source: PR19 integrated source src/tag-sources.ts local options/freedbModes;
src-tauri/src/sources/freedb.rs local readers and freedb_index.rs build/search/
status. Source inspection; no application launch. -->
