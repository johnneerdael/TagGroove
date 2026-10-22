# Tag an album from an online database

In this lesson you will review one MusicBrainz release, match its tracks to
copied audio files and import a small, deliberate set of fields. Allow about
15 minutes, plus time for a search or artwork download. The aim is to learn
how to judge a match, rather than accept the first result.

You need:

- [ ] A desktop TagGroove build and a working network connection.
- [ ] A copied album you own, with one local audio file per track.
- [ ] The album's artist/title and enough information to recognize its edition,
      such as track count, release country or catalog number.
- [ ] A backup kept outside the practice folder.

Use a real album from your collection for this lesson. The guide's invented
Juniper Vale album teaches text editing, but cannot promise a result in an
online catalog. If MusicBrainz does not have a matching edition, stop without
saving and choose another practice album; a near match is not a successful
import.

## 1. Open and inspect the copied album

Open the copied album in TagGroove and select only its audio files. Save or
deliberately discard earlier pending edits before opening a tag source.
Check the file count, titles, disc numbers and track order.

You should be able to say how many tracks the album contains and which local
file belongs to each track. Keep bonus tracks and different editions in mind:
two releases can have the same album title but different tracklists.

For the first lesson, use an ordinary single-disc album with a known order.
This keeps the matching exercise small enough to inspect row by row.

## 2. Search by artist and album

Choose **Tag Sources → MusicBrainz…**. Under **Search by**, choose **Artist
and album**. Check **Artist** and **Album** before choosing **Search**.
TagGroove prefills them from the selection; a prefilled value can still be
wrong or inherited from an earlier tagging mistake.

The results include artist, title, year, country, label, catalog number,
format and track count. MusicBrainz also supplies a match score. That score
helps find candidates; it does not prove that a candidate is your edition.

If no result fits, correct the spelling and edition information rather than
removing details until an unrelated result appears. The
[MusicBrainz guide](musicbrainz.md) explains other search modes.

## 3. Choose the release that matches your files

Compare the result's artist, track count and edition details with your album.
Choose **Preview** when you need to inspect its source page; this opens a
browser page without importing tags. Select the intended result and choose
**Next**.

<!-- figure:musicbrainz-results -->
<figure markdown>
![MusicBrainz search results for R.E.M. Automatic for the People, with label, country, year and track count for each edition](../assets/screens/musicbrainz-results-light.png#only-light)
![MusicBrainz search results for R.E.M. Automatic for the People, with label, country, year and track count for each edition](../assets/screens/musicbrainz-results-dark.png#only-dark)
</figure>
<!-- /figure -->

The review now shows **Fields**, **Artwork** when available, and **Tracks
and files**. This is a proposal. Nothing in the proposed tag import is written
until you choose its save button.

If the tracklist is the wrong edition, choose **Back** and select another
release. Do not try to turn a different edition into your album by moving
unrelated tracks around.

## 4. Choose only the fields for this lesson

In **Fields**, choose **None**, then check `ALBUM`, `ALBUMARTIST`, `ARTIST`
and `TITLE`. Inspect their proposed values. If a field is absent from the
source, it will not appear as a value to import.

<!-- figure:musicbrainz-review -->
<figure markdown>
![The MusicBrainz review step: fields to write with check boxes, the release cover and a tracks-and-files table](../assets/screens/musicbrainz-review-light.png#only-light)
![The MusicBrainz review step: fields to write with check boxes, the release cover and a tracks-and-files table](../assets/screens/musicbrainz-review-dark.png#only-dark)
</figure>
<!-- /figure -->

This narrow selection makes the result easier to verify. Other fields,
such as your own genre, comments or numbering, stay outside this exercise.
Unchecked fields are not imported. Missing source values do not erase
existing file values.

Album-level values can be edited in the field list. If a per-track field
shows a summary, choose **Edit per track** to inspect it in the matching
table. Keep this lesson's source spellings unless a specific correction is
needed; a review is a chance to make a decision, not a requirement to alter
every field.

## 5. Check every file-to-track pair

In **Tracks and files**, inspect each track beside its assigned local file.
Choose **Selection order** for this practice album, whose order you checked
in step 1, then verify that every assignment is right. This restores a
positional pairing; it does not determine which recording the file contains.

<!-- figure:musicbrainz-tracks -->
<figure markdown>
![The Tracks and files table pairing each of the twelve files with a track from the chosen release](../assets/screens/musicbrainz-tracks-light.png#only-light)
![The Tracks and files table pairing each of the twelve files with a track from the chosen release](../assets/screens/musicbrainz-tracks-dark.png#only-dark)
</figure>
<!-- /figure -->

For example, if source track 1 is Paper Boats and track 2 is Lanterns on the
Water in a practice comparison, the local file containing Paper Boats must
be beside track 1 even if its old filename begins `02`. The names here are
illustrative; compare your real album's actual titles.

If a file is on the wrong row, drag it to the correct track or use its row
arrows. Moving a file shifts its position in the pairing; it does not rename
the audio file. A large duration difference is another reason to inspect
the assignment. Do not use a similar duration as the only evidence.

Check that there are no unexpected **No file** or **Not written** rows.
If the source has extra bonus tracks, go back and confirm the edition before
continuing. The save button's count is the number of files that will actually
be written, which can be smaller than your selection.

## 6. Make an explicit cover choice

If artwork is available, inspect the displayed image. For this first import,
leave **Write cover art** on only when the image is the intended front cover.
Clear **Replace existing artwork** so existing embedded artwork is retained.
The chosen image is added alongside those existing pictures.
Leave **Save image to file** off: the first exercise is about the files'
metadata and embedded cover choice, without an additional folder-image write.

If there is no suitable image, turn **Write cover art** off, or continue with
the no-artwork state if none is available. The metadata lesson can still
succeed. A source result does not guarantee that an image exists or downloads.

**Save cover as…** is a separate image-save operation. Do not use it for this
lesson. Canceling the tag import would not undo a separate image save.

## 7. Save and read the result back

Read the file count on **Save tags to … files**. If it is the intended count,
choose the button and read the operation result. This writes the checked
source fields immediately; it does not create tag-panel edits waiting for a
later **Save tags**.

After a successful import, inspect a first, middle and last track. Reload the
tags when there are no pending edits to discard, and compare the saved
`ALBUM`, `ALBUMARTIST`, `ARTIST` and `TITLE` values with your chosen release.
Check the embedded cover where you elected to write one.

The import can be recorded as one desktop Undo step. If a wrong match was
saved, inspect **Edit → Undo** and its operation label immediately, then
check the restored files. Keep the original backup; session history is not
a permanent recovery copy. If an error is reported, inspect which files
were changed before retrying the import.

## What you learned

- Search fields are clues, and a high score is not proof of an edition.
- Choosing a release and matching its tracks are separate decisions.
- Checked fields control the imported metadata; missing values do not clear it.
- Embedded artwork and a separate folder image are different outputs.
- Confirming the source import writes immediately, so the save count matters.

## Next steps

- [MusicBrainz search modes, identifiers and artwork](musicbrainz.md)
- [Tag-source review, API keys and other sources](index.md)
- [Working with covers](../tagging/covers.md)
- [Undo and history](../tagging/undo-history.md)

<!-- Candidate validation: private validation/harness/guide_backend.rs compiled the actual MusicBrainz parser/model and parsed invented release/artwork JSON without requests. validation/source-lesson-fixture.mjs consumed that parser output using the actual current src/tag-sources.ts prefill, searchModes, selection-order pairing and buildApplyItems functions; checked four-field updates, unmatched files and empty-value skipping. The full native lesson, immediate native save, artwork download and optional artwork branch remain unexecuted. Current immediate-save wording follows src/main.ts tag-source apply and src/tag-sources.ts syncReview. Editorial/source review supports the written instructions; native UI and provider-journey qualification remains separate. -->
