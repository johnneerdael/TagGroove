# Compare metadata from several sources

Different providers describe the same release differently: one has the
catalog number, another the better artwork, a third the right mix names. The
comparison window lets you look at what each provider returned for your files,
field by field, and write exactly the combination you choose.

Use it when one source is not enough, or when you want to review several
editions before saving. For a single provider, the ordinary
[search, review and save](index.md#search-review-and-save) dialog is shorter.

Select one track or the files of one album and choose **Tag Sources → Compare
metadata…**. The same field choices work in both scopes. Opening the window
reads the current files; online searches start only when requested.

1. Check **Single track** or **Album** and the search terms. Track searches use
   the performing artist and complete track title, including the mix/version.
   Album searches use album artist and album title. If local album titles differ,
   explicitly confirm that the selected files belong to one album.
2. Open **Sources and matches** to choose native providers and installed `.src`
   sources. Sources needing keys, a local database or extra search inputs explain
   what is missing. AcoustID is optional: it reads each file to calculate its
   fingerprint. Select **Search selected sources**, or search one source in its
   details. A failed source leaves other results available; retry it separately.
3. Check the suggested release and **Tracks and files**. Use **Matches** to
   choose another edition, or **Match…** to correct a file's provider track.
   Ambiguous matches require review. A changed candidate resets its field choices.
4. Find any standard or extended field. **Existing values** stays separate from
   the choice and result. Open **Use** to see each provider's returned value in
   its own read-only box. Long text and multiple values remain available in full.
   In Album mode the boxes show **track 1 as an example**; they never substitute
   a later track when the first track has no value.
5. Choose **Inherit**, **None**, or a provider for each field:

   | Choice | Result |
   |---|---|
   | **Inherit** | Keep each file's own existing values; this is the default. |
   | **None** | Explicitly remove that field from every selected file. |
   | **Provider** | Use that provider's own matched value for each file. |

   One field choice applies to the whole album. Choosing a provider for `ARTIST`
   writes each track's performing artist, not track 1's literal value to every
   track. `ALBUMARTIST` is independent: a compilation can use `Various Artists`
   while each track keeps its individual performer. An existing `VA` value is
   unchanged until a choice explicitly replaces it.
6. Check coverage counts and click an existing/result value for the per-track
   preview. Missing provider values keep existing values; **None** is the explicit
   removal action. An unmatched track must be matched before importing its track
   fields. Artwork has the same Inherit/None/provider choices and a visual preview.
7. Choose **Review and save**. Check affected paths, removals and missing-value
   notices, then save the reviewed batch. Successful files appear separately if
   another file fails. **Retry failed files** writes only the remaining failures.

**Cancel** leaves unsaved choices unwritten; it cannot undo files already saved
in a partial batch. Changing the selection or editing a file outside the reviewed
state requires a new comparison. A provider may supply only some fields or no
usable result. This includes lyrics-only LRCLIB and artwork-only fanart.tv.
Normal platform save-back rules and the desktop [Undo
limits](../tagging/undo-history.md) apply. Writable provider documents use their
existing [save-back and recovery path](../platforms/android.md), outside the
desktop Undo journal. Ordinary single-source dialogs remain available.
