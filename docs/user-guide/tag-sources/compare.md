# Compare metadata from several sources

Different providers describe the same release differently: one has the
catalog number, another the better artwork, a third the right mix names. The
comparison window lets you look at what each provider returned for your files,
field by field, and write exactly the combination you choose.

Use it when one source is not enough, or when you want to review several
editions before saving. For a single provider, the ordinary
[search, review and save](index.md#search-review-and-save) dialog is shorter.

## Open the window

Select one track or the files of one album and choose **Tag Sources → Compare
metadata…**. The same field choices work in both scopes. Opening the window
reads the current files; online searches start only when requested.

<!-- figure:compare-start -->
<figure markdown>
![The Compare metadata window for the album Automatic For The People: the album search terms, Single track and Album toggle, a field table with Existing values, a Use choice set to Inherit and the Result](../assets/screens/compare-start-light.png#only-light)
![The Compare metadata window for the album Automatic For The People: the album search terms, Single track and Album toggle, a field table with Existing values, a Use choice set to Inherit and the Result](../assets/screens/compare-start-dark.png#only-dark)
<figcaption>Every field starts on Inherit, so nothing changes until you choose.</figcaption>
</figure>
<!-- /figure -->

## 1. Choose the scope and search terms

Check **Single track** or **Album** and the search terms. Track searches use
the performing artist and complete track title, including the mix/version.
Album searches use album artist and album title. If local album titles differ,
explicitly confirm that the selected files belong to one album.

## 2. Pick sources and search

Open **Sources and matches** to choose native providers and installed `.src`
sources. Sources needing keys, a local database or extra search inputs explain
what is missing. AcoustID is optional: it reads each file to calculate its
fingerprint. Select **Search selected sources**, or search one source in its
details. A failed source leaves other results available; retry it separately.

<!-- figure:compare-sources -->
<figure markdown>
![The Sources and matches panel listing every provider, with the suggested release and match status for each](../assets/screens/compare-sources-light.png#only-light)
![The Sources and matches panel listing every provider, with the suggested release and match status for each](../assets/screens/compare-sources-dark.png#only-dark)
</figure>
<!-- /figure -->

## 3. Check the matches

Check the suggested release and **Tracks and files**. A source that found
several editions of the album shows an **Edition** list in **Sources and
matches**: choose the pressing or release you have, and that source loads it and
becomes ready. **Matches** lists the same candidates with more detail, and
**Match…** corrects a file's provider track.

<!-- figure:compare-editions -->
<figure markdown>
![Sources and matches after a search: a source with several editions has an Edition list, and sources that matched say Ready](../assets/screens/compare-editions-light.png#only-light)
![Sources and matches after a search: a source with several editions has an Edition list, and sources that matched say Ready](../assets/screens/compare-editions-dark.png#only-dark)
<figcaption>Choose the edition you have from each source's Edition list.</figcaption>
</figure>
<!-- /figure -->

A source is ready on its own when exactly one of its results has the artist and
title you searched for and every file has a matching track. Several editions of
the same album, or a track that does not line up, are ambiguous: that source
stays at **Choose the matching result** until you pick its edition, and its
values stay unavailable in **Use** until then. In **Use**, **Review match** takes
you to that source's list. Last.fm matches tracks by title and artist
only, because its track lengths are often inaccurate. A changed candidate resets
its field choices.

<!-- figure:compare-results -->
<figure markdown>
![The Compare metadata window after searching: the field table shows how many sources returned each field](../assets/screens/compare-results-light.png#only-light)
![The Compare metadata window after searching: the field table shows how many sources returned each field](../assets/screens/compare-results-dark.png#only-dark)
</figure>
<!-- /figure -->

## 4. See what each provider returned

Find any standard or extended field. **Existing values** stays separate from
the choice and result. Open **Use** to see each provider's returned value in
its own read-only box. Long text and multiple values remain available in full.
In Album mode the boxes show **track 1 as an example**; they never substitute
a later track when the first track has no value.

<!-- figure:compare-use-menu -->
<figure markdown>
![The Choose album panel: the existing value, Inherit and None, and each provider's returned album title](../assets/screens/compare-use-menu-light.png#only-light)
![The Choose album panel: the existing value, Inherit and None, and each provider's returned album title](../assets/screens/compare-use-menu-dark.png#only-dark)
<figcaption>Open Use to see what each provider returned before you choose.</figcaption>
</figure>
<!-- /figure -->

## 5. Choose a value for each field

Choose **Inherit**, **None**, or a provider for each field:

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

## 6. Check coverage and per-track previews

Check coverage counts and click an existing/result value for the per-track
preview. Missing provider values keep existing values; **None** is the explicit
removal action. An unmatched track must be matched before importing its track
fields. Artwork has the same Inherit/None/provider choices and a visual preview.

## 7. Review and save

Choose **Review and save**. Check affected paths, removals and missing-value
notices, then save the reviewed batch. Successful files appear separately if
another file fails. **Retry failed files** writes only the remaining failures.

<!-- figure:compare-review -->
<figure markdown>
![The review step of Compare metadata, listing the files and fields that would change before anything is saved](../assets/screens/compare-review-light.png#only-light)
![The review step of Compare metadata, listing the files and fields that would change before anything is saved](../assets/screens/compare-review-dark.png#only-dark)
</figure>
<!-- /figure -->

## Good to know

**Cancel** leaves unsaved choices unwritten; it cannot undo files already saved
in a partial batch. Changing the selection or editing a file outside the reviewed
state requires a new comparison. A provider may supply only some fields or no
usable result. This includes lyrics-only LRCLIB and artwork-only fanart.tv.
Normal platform save-back rules and the desktop [Undo
limits](../tagging/undo-history.md) apply. Writable provider documents use their
existing [save-back and recovery path](../platforms/android.md), outside the
desktop Undo journal. Ordinary single-source dialogs remain available.
