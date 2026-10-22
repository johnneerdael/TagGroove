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

If a release search finds nothing, clear the **Artist** box and search by the
release title alone. A compilation can be credited to *Various Artists* on one
store and to its full list of artists on another, so the artist of your files may
not match.

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

## When Cloudflare blocks a lookup

Traxsource in particular can answer with a Cloudflare challenge instead of the
page. When any tag source does, TagGroove repeats the request through an
optional FlareSolverr-compatible service (FlareSolverr, Byparr or trawl) that you
run, and stops contacting that site directly until you choose **Retry direct
access**. Without a solver the lookup stops with a message that says where to
add one. Set it up once under **Options → Tag Sources → Cloudflare solver**; it
serves every tag source. See [Cloudflare solver](cloudflare-solver.md).

## Network access and privacy

Searches send artist/title terms and provider identifiers to the selected
catalog. Metadata and artwork retrieval contact that catalog and its image
hosts. Audio files are not uploaded by these catalog lookups.

A configured solver receives the page addresses of the requests it handles. A
request that carries a credential, such as Beatport's authenticated catalog API,
never goes to a solver. Choose a solver whose operator and network are trusted;
the endpoint is saved in ordinary Options, separately from private source
credentials, so do not put passwords or tokens in its address.

## Related

- [Search, review and save](index.md)
- [Compare metadata from several sources](compare.md)
- [Discogs artist and title search](discogs.md#find-a-track-by-artist-and-title)
- [Cloudflare solver](cloudflare-solver.md)
- [Tag Sources options](../reference/options/tag-sources.md)
- [Undo and history](../tagging/undo-history.md)
