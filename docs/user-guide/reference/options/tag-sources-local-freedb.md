# Tag Sources → Local freedb

Open **File → Options… → Tag Sources → Local freedb** to use an extracted historical freedb database on disk. This is a source of album metadata, separate from OpenTag's own [Library cache](library.md).

!!! tip "New to this?"
    Select the database root containing category subfolders, not an individual album file. A local snapshot reflects the date it was created; it does not receive current server updates.

## Local freedb folder

The folder defaults to empty. Type its full path or use **Choose folder…**. The reader supports category folders such as `blues`, `classical`, and `rock`, with either one xmcd file per disc or the Windows `xxtoyy` range-file layout.

Extract the snapshot before selecting it; a downloaded compressed archive is not the database folder. A nonexistent path or unreadable category data cannot provide matches. Store it somewhere the app can continue to access.

Local lookup by disc ID reads that snapshot. The historical Mp3tag documentation says audio-file-based lookup is unavailable locally. Do not transfer that restriction to OpenTag unchanged: its local reader supports disc-ID lookup, and its tag-source workflow can derive a disc ID from a suitable ordered track layout. That is metadata matching, not an acoustic fingerprint of the music.

## Text index

To search by artist/album words, choose **Build index**. The page reports whether an index exists, its disc count, and whether it belongs to the currently entered folder. Building has progress reporting; **Cancel** requests cancellation.

An index for another folder is not a valid index for this folder. Rebuild after replacing the snapshot or changing the configured root. An empty root must be set before building can start. In environments without the index backend, the button is unavailable and the page states that text indexing is not available there.

Building an index performs work immediately using the entered folder. Saving Options chooses which folder later searches use; canceling the outer dialog does not undo a completed index build. No local index is needed for a direct disc-ID lookup.

## No matching album

Confirm the selected files belong to one complete disc, their list order and durations are appropriate, and the snapshot actually contains the release. A title search needs an index; a missing disc cannot be recovered by rebuilding the same snapshot's index. A newer edition may require another source.

## Related

- [Remote freedb server](tag-sources-freedb.md)
- [Tag Sources](tag-sources.md)
- [Library cache](library.md)

<!-- verified: baseline007a806 src/tag-sources.ts local-freedb options, bindIndex, disc-ID derivation; src-tauri/src/sources/freedb.rs local_search/local entry reader; src-tauri/src/sources local index implementation. Original local audio-lookup restriction explicitly distinguished. -->
