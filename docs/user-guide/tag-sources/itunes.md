# iTunes / Apple Music

Use **Tag Sources → iTunes…** for album metadata and store artwork. No Apple
account or API key is needed for the lookup.

<!-- figure:source-itunes -->
<figure markdown>
![The iTunes search dialog prefilled with the artist and album of the selected song](../assets/screens/source-itunes-light.png#only-light)
![The iTunes search dialog prefilled with the artist and album of the selected song](../assets/screens/source-itunes-dark.png#only-dark)
</figure>
<!-- /figure -->

## Find one track

Select the intended file and choose **Artist and track title** in **iTunes…**.
Keep the full mix/version in the title, search, and check the exact track before
saving. The selected song retains its store track identity when the parent album loads. Album completeness checks still apply.

Use [Compare metadata…](index.md#compare-fields-from-several-sources) to choose
individual fields from different providers for one track or an entire album.

## Find an album

<!-- figure:itunes-results -->
<figure markdown>
![iTunes search results for Taylor Swift The Life of a Showgirl](../assets/screens/itunes-results-light.png#only-light)
![iTunes search results for Taylor Swift The Life of a Showgirl](../assets/screens/itunes-results-dark.png#only-dark)
</figure>
<!-- /figure -->

1. Select the album's files and open **iTunes…**.
2. Choose **Artist and album**, **Free text**, **Album ID or Apple Music
   address**, or **Barcode (UPC)**. For an ID lookup, use the album's numeric
   collection ID or its Apple Music / iTunes album address.
3. Select **Search**, choose the edition and select **Next**. In album ID
   mode, **Look up** opens the review directly.
4. Check the country, fields, artwork and file-to-track matches before
   [saving](index.md#search-review-and-save).

iTunes returns up to 50 album search results; there is no further result-page
lookup. Narrow the words or use an album ID if the edition is missing.

## Country and artwork

Set **Store country (two letters)** in **Options → Tag Sources → Online
services**, for example `US` (the default) or `NL`. Store catalogs can differ,
so changing country can change which editions are found.

The same page selects 600, 1200 or 3000 pixel artwork; 1200 is the default.
**Covers: iTunes…** uses artwork without changing text fields. The selected
size requests Apple's image variant; availability and quality depend on the store.

## Fields and limits

The review can offer album, album artist, year, genre and copyright; song
titles, artists, track and disc numbers; and iTunes album, artist and catalog
identifiers. Advisory information appears only when supplied. The release
lookup requests a limited response (`limit=200`). If Apple returns fewer album
songs than its stated positive track count, TagGroove reports an incomplete album
before opening the review or saving. A response at the limit without a usable
album total is also refused. Smaller responses without a total show the
available songs, whose completeness cannot be confirmed.

<!-- figure:itunes-review -->
<figure markdown>
![The iTunes review step: fields to write, the album cover at 1200 × 1200 and a tracks-and-files table](../assets/screens/itunes-review-light.png#only-light)
![The iTunes review step: fields to write, the album cover at 1200 × 1200 and a tracks-and-files table](../assets/screens/itunes-review-dark.png#only-dark)
<figcaption>The review shows which fields iTunes supplies and the cover it will write.</figcaption>
</figure>
<!-- /figure -->

TagGroove makes no continuation request for this album lookup. Large or
incomplete albums can therefore be unavailable, including in **Covers:
iTunes…**, which uses the same lookup. Try another edition, store country or
source rather than saving a partial listing as the complete album.

TagGroove spaces iTunes requests about 3.1 seconds apart, so a search followed
by a release lookup can pause. It retrieves metadata and artwork, not music,
audio previews or an Apple Music subscription library.

## Related

- [Search, review and save](index.md#search-review-and-save)
- [Compare metadata from several sources](compare.md)
- [Tag Sources options](../reference/options/tag-sources.md)
- [Deezer](deezer.md)
