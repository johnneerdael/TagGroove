# Deezer

Use **Tag Sources → Deezer…** for album metadata, track ISRCs and artwork.
The public lookup needs no account or API key.

## Find one track

Select the intended file and choose **Artist and track title** in **Deezer…**.
Keep the full mix/version in the title, search, and check the exact track before
saving. The selected track retains its identity and its available BPM/ISRC values when the complete parent album loads.

Use [Compare metadata…](index.md#compare-fields-from-several-sources) to choose
individual fields from different providers for one track or an entire album.

## Find an album

1. Select the album's files and choose **Deezer…**.
2. Choose **Artist and album**, **Free text**, **Album ID or address**, or
   **Barcode (UPC)**. An album ID is numeric; a `deezer.com` album address
   also works.
3. Select **Search**, select the edition and choose **Next**. In album ID
   mode, **Look up** opens the review directly.
4. Check file-to-track matches and fields, then use the
   [shared review](index.md#search-review-and-save) to save.

Artist and album searches send plain words. Results can match other fields
too; a title match alone does not establish the correct edition.

## Fields and artwork

Album fields include `ALBUM`, `ALBUMARTIST`, `YEAR`, `GENRE`, `PUBLISHER`
and `BARCODE`. Track fields include `TITLE`, `ARTIST`, `ISRC`, `TRACK` and
`DISCNUMBER`, when supplied by Deezer.

Track pages are collected before the review opens. Track totals are calculated
per disc, and disc totals come from the returned tracks. A failed later page
produces an error instead of a partial album. Lookups exceeding 10,000 returned
tracks are refused. This does not guarantee the provider's catalog is complete.

Choose 1000 pixel artwork (the default) or 500 pixels in **Options → Tag
Sources → Online services**. Choose **Covers: Deezer…** for artwork only.

## Lookup errors

TagGroove spaces Deezer requests about 120 milliseconds apart. Deezer can
report an error inside a successful HTTP response; TagGroove displays quota
and missing-release messages rather than treating those as empty albums.
Wait after a quota error. Check the album ID, barcode or search words after
a missing result. TagGroove retrieves no Deezer streams or audio previews.

## Related

- [Search, review and save](index.md#search-review-and-save)
- [Tag Sources options](../reference/options/tag-sources.md)
- [iTunes / Apple Music](itunes.md)
