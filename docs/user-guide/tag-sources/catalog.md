# Beatport and Traxsource

Use **Tag Sources → Beatport…** or **Traxsource…** to find metadata for an
individual track or a complete release. Check the artist, mix/version, label
and catalog number before importing a candidate. Catalog availability and
access restrictions can change independently of TagGroove.

<!-- figure:source-beatport -->
<figure markdown>
![The Beatport dialog with a search type, artist and title](../assets/screens/source-beatport-light.png#only-light)
![The Beatport dialog with a search type, artist and title](../assets/screens/source-beatport-dark.png#only-dark)
</figure>
<!-- /figure -->

## Search a track or release

<!-- figure:source-traxsource -->
<figure markdown>
![The Traxsource dialog with a search type, artist and title](../assets/screens/source-traxsource-light.png#only-light)
![The Traxsource dialog with a search type, artist and title](../assets/screens/source-traxsource-dark.png#only-dark)
</figure>
<!-- /figure -->

<!-- figure:beatport-release-form -->
<figure markdown>
![The Beatport dialog set to search by release, with the artist and release title filled in from the selected files](../assets/screens/beatport-release-form-light.png#only-light)
![The Beatport dialog set to search by release, with the artist and release title filled in from the selected files](../assets/screens/beatport-release-form-dark.png#only-dark)
<figcaption>Search by release to find a whole release; search by track for a single file.</figcaption>
</figure>
<!-- /figure -->

1. Select one track, or the files of one release, and open its source.
2. Use **Track: artist and title** to search a track. The search starts with the
   performing artist and complete title of the first selected file, preserving
   the mix/version. Adjust those terms if necessary.
3. Use **Release: artist and title** to search a release by artist and its title.
   Its initial values come from the selected files' album artist and album.
4. Alternatively, choose **Track or release ID / address** for direct lookup.
   Use `track:123` or `release:456` to distinguish a track from a release, or
   paste its provider address.
5. Choose the intended result and select **Next**. Review its fields, artwork
   and track-to-file matches before saving through the ordinary
   [source review](index.md#search-review-and-save).

Fields depend on the provider and result. Traxsource release rows do not always
include BPM or key. An individual track lookup also reads its detail page for
those values when available; missing fields are not invented.

A track result retains its actual release position. For example, a match for
track 7 stays track 7 when loaded by itself; it is not renumbered to 1.

## Open a matched track's full release

In a track review, select **Open full release**. TagGroove uses that track's
exact release identifier to load the release and its complete tracklist; it
does not perform another title search for a similarly named release.

<!-- figure:beatport-release-results -->
<figure markdown>
![Beatport release results: the first hit is Digital Renaissance Vol. 1 on ÆDEN RECORDS, catalog number Æ001D, 15 tracks](../assets/screens/beatport-release-results-light.png#only-light)
![Beatport release results: the first hit is Digital Renaissance Vol. 1 on ÆDEN RECORDS, catalog number Æ001D, 15 tracks](../assets/screens/beatport-release-results-dark.png#only-dark)
<figcaption>The release search lists label, catalog number, year and track count.</figcaption>
</figure>
<!-- /figure -->

For one selected file, its matched catalog track remains paired by track
identifier. **Use track for this file** can choose another exact track row.
With several files, inspect and adjust all file-to-track matches before
saving. Opening a release alone writes nothing.

To choose individual fields from several providers, use
[Compare metadata…](index.md#compare-fields-from-several-sources).

## Optional challenge solver

Traxsource may require a FlareSolverr-compatible proxy when direct catalog
requests are restricted. TagGroove does not install or start a solver.

<!-- figure:options-tag-sources-catalog -->
<figure markdown>
![The Traxsource page of the Options dialog with the solver endpoint, timeout and the button that clears remembered restrictions](../assets/screens/options-tag-sources-catalog-light.png#only-light)
![The Traxsource page of the Options dialog with the solver endpoint, timeout and the button that clears remembered restrictions](../assets/screens/options-tag-sources-catalog-dark.png#only-dark)
<figcaption>Options → Tag Sources → Traxsource holds the optional solver endpoint.</figcaption>
</figure>
<!-- /figure -->


Open **Options → Tag Sources → Traxsource**:

- **Traxsource solver endpoint:** leave empty for direct access, or
  enter the HTTP/HTTPS endpoint of a trusted solver, for example
  `http://localhost:8191/v1`. Include a host and omit credentials, query
  parameters and fragments.
- **Solver timeout (seconds):** use 1–300 seconds; the default is 120.
- **Retry restricted catalog services:** clear restrictions remembered during this app session after
  updating the solver or fixing connectivity. This action does not edit tags.

Choose **OK** to save endpoint and timeout changes. Saving an endpoint permits
challenged Traxsource page requests to use that solver. An unavailable or rejected
solver produces a lookup error; a failed or challenged page is not a valid
empty release. Close or cancel an unwanted lookup before retrying.

## Network access and privacy

Searches send artist/title terms and provider identifiers to the selected
catalog. Metadata and artwork retrieval contact that catalog and its image
hosts. Audio files are not uploaded by these catalog lookups.

Beatport uses its direct catalog API and never uses this solver. A configured
solver receives Traxsource page addresses and fetches those pages.
Choose a solver whose operator and network are trusted. The endpoint is saved
in ordinary Options, separately from private source credentials; do not put
passwords or tokens in its address. Catalog solver configuration does not grant
it access to TagGroove's Discogs token.

## Related

- [Search, review and save](index.md)
- [Compare metadata from several sources](compare.md)
- [Discogs artist and title search](discogs.md#find-a-track-by-artist-and-title)
- [Tag Sources options](../reference/options/tag-sources.md)
- [Undo and history](../tagging/undo-history.md)
