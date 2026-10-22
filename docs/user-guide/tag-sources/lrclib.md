# LRCLIB lyrics

Use **Tag Sources → Lyrics: LRCLIB…** to find lyrics for selected files.
No API key is needed. Lyrics are proposed in `UNSYNCEDLYRICS` for review
before writing.

<!-- figure:lrclib-form -->
<figure markdown>
![The LRCLIB lyrics dialog: each selected file is looked up by title, artist, album and length](../assets/screens/lrclib-form-light.png#only-light)
![The LRCLIB lyrics dialog: each selected file is looked up by title, artist, album and length](../assets/screens/lrclib-form-dark.png#only-dark)
</figure>
<!-- /figure -->

Use [Compare metadata…](index.md#compare-fields-from-several-sources) to mix this
source's fields with other providers for one track or an album. In Album mode,
one provider choice per field uses each track's own matched values. Inherit keeps
existing values; None explicitly removes them.

## Look up the selected files

1. Check the files' title, artist, album and duration. Correct misleading
   tags before searching.
2. Open **Lyrics: LRCLIB…** and keep **Each selected file (title, artist,
   album, length)** selected.
3. Select **Search**. The combined result opens the review for the files.
4. Inspect the proposed lyrics and save the checked field. Files without
   lyrics are left unchanged.

The lookup first uses metadata and duration. If it returns no usable lyrics,
TagGroove searches by title and artist. With a known, positive file duration,
the fallback accepts only a result within 3 seconds. Without a usable duration,
it takes the first result containing lyrics, so review that match especially
carefully. Similar metadata can still identify the wrong version; check live,
instrumental and edited recordings.

A lookup failure is reported without discarding the other files' results.
The combined result keeps one track per file in file list order. Instrumental
records supply no lyrics to write.

## Search for one record

Choose **Free text** to browse lyrics records, or **Lyrics record ID** to
open a numeric LRCLIB record directly. Check the file-to-track pairing before
saving an individually selected record.

## Plain or synchronized lyrics

The default is plain text. Enable **Write synchronized lyrics (LRC time
stamps) when available** in **Options → Tag Sources → Online services** to
keep LRC timestamps. Both choices write text to `UNSYNCEDLYRICS`; this option
does not create an ID3 synchronized-lyrics frame. Player support varies.

<!-- figure:lrclib-review -->
<figure markdown>
![The LRCLIB review step showing the lyrics found for the selected file](../assets/screens/lrclib-review-light.png#only-light)
![The LRCLIB review step showing the lyrics found for the selected file](../assets/screens/lrclib-review-dark.png#only-dark)
</figure>
<!-- /figure -->

When only synchronized lyrics are available and the option is off, TagGroove
removes timestamps and LRC metadata lines. LRCLIB supplies no language;
ID3 lyrics use the unknown-language code `XXX`. Literal text, including `||`,
is retained rather than interpreted as a language prefix.

Requests are spaced about 200 milliseconds apart; a busy-server response is
retried once. Each selected file needs a lookup, and a fallback can add another.

## Related

- [Search, review and save](index.md#search-review-and-save)
- [Compare metadata from several sources](compare.md)
- [Network access, privacy and rate limits](privacy-and-network.md)
- [Tag Sources options](../reference/options/tag-sources.md)
