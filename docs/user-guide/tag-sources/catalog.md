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

## Review a Beatport result

A release review lists the release's fields, each with a check box, and then its
tracks beside the files TagGroove matched them to, in the release's own track
order. A track review shows that one track's fields. Nothing is written until you
choose **Save tags**.

<!-- figure:beatport-release-review -->
<figure markdown>
![The Beatport review for Digital Renaissance Vol. 1 with its cover, the fields to write and the tracks matched to the selected files](../assets/screens/beatport-release-review-light.png#only-light)
![The Beatport review for Digital Renaissance Vol. 1 with its cover, the fields to write and the tracks matched to the selected files](../assets/screens/beatport-release-review-dark.png#only-dark)
<figcaption>The release's own cover is offered with the fields.</figcaption>
</figure>
<!-- /figure -->

<!-- figure:beatport-release-tracks -->
<figure markdown>
![The tracks of the Beatport release beside the files they are matched to, with arrows to move a file to another track](../assets/screens/beatport-release-tracks-light.png#only-light)
![The tracks of the Beatport release beside the files they are matched to, with arrows to move a file to another track](../assets/screens/beatport-release-tracks-dark.png#only-dark)
</figure>
<!-- /figure -->

<!-- figure:beatport-track-results -->
<figure markdown>
![Beatport track results for Adriatique's Palm Of Your Hands (Extended Mix), with the chosen result's cover, label and catalog number below the list](../assets/screens/beatport-track-results-light.png#only-light)
![Beatport track results for Adriatique's Palm Of Your Hands (Extended Mix), with the chosen result's cover, label and catalog number below the list](../assets/screens/beatport-track-results-dark.png#only-dark)
<figcaption>Search a track by artist and title, then choose the result.</figcaption>
</figure>
<!-- /figure -->

<!-- figure:beatport-track-review -->
<figure markdown>
![The review of one Beatport track: its fields and the file it is matched to](../assets/screens/beatport-track-review-light.png#only-light)
![The review of one Beatport track: its fields and the file it is matched to](../assets/screens/beatport-track-review-dark.png#only-dark)
</figure>
<!-- /figure -->

## Review a Traxsource result

Traxsource results and reviews work the same way. Traxsource often lists a
compilation under *Various Artists*, so search by the release title alone if the
artist of your files finds nothing.

<!-- figure:traxsource-release-review -->
<figure markdown>
![The Traxsource review for Digital Renaissance Vol. 1 with its cover, fields and tracks](../assets/screens/traxsource-release-review-light.png#only-light)
![The Traxsource review for Digital Renaissance Vol. 1 with its cover, fields and tracks](../assets/screens/traxsource-release-review-dark.png#only-dark)
</figure>
<!-- /figure -->

<!-- figure:traxsource-release-tracks -->
<figure markdown>
![The Traxsource release's tracks beside the matched files; one of the 15 tracks has no file](../assets/screens/traxsource-release-tracks-light.png#only-light)
![The Traxsource release's tracks beside the matched files; one of the 15 tracks has no file](../assets/screens/traxsource-release-tracks-dark.png#only-dark)
<figcaption>TagGroove says how many tracks have a file before you save.</figcaption>
</figure>
<!-- /figure -->

<!-- figure:traxsource-release-results -->
<figure markdown>
![Traxsource release search for Digital Renaissance with the artist box cleared: the compilation is listed under Various Artists](../assets/screens/traxsource-release-results-light.png#only-light)
![Traxsource release search for Digital Renaissance with the artist box cleared: the compilation is listed under Various Artists](../assets/screens/traxsource-release-results-dark.png#only-dark)
<figcaption>Searching by the release title alone finds a compilation credited to Various Artists.</figcaption>
</figure>
<!-- /figure -->

<!-- figure:traxsource-track-results -->
<figure markdown>
![Traxsource track search for Armin van Buuren's Here In My Arms, with its single result and cover below the list](../assets/screens/traxsource-track-results-light.png#only-light)
![Traxsource track search for Armin van Buuren's Here In My Arms, with its single result and cover below the list](../assets/screens/traxsource-track-results-dark.png#only-dark)
</figure>
<!-- /figure -->

<!-- figure:traxsource-track-review -->
<figure markdown>
![The review of one Traxsource track: its fields and the file it is matched to](../assets/screens/traxsource-track-review-light.png#only-light)
![The review of one Traxsource track: its fields and the file it is matched to](../assets/screens/traxsource-track-review-dark.png#only-dark)
</figure>
<!-- /figure -->

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
