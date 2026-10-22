# Tag sources

Tag sources look up metadata, lyrics and artwork for the files you select.
You choose a result, check its fields and the file-to-track matches, and only
then save the changes.

<div class="grid cards" markdown>

-   **Tag one album from one source**

    ---

    Search, pick a release, match the tracks and save.

    [:octicons-arrow-right-24: Tag an album from an online database](tag-from-online-sources.md)

-   **Compare several sources, field by field**

    ---

    See what each provider returns for the same files and pick the best value
    for every field.

    [:octicons-arrow-right-24: Compare metadata from several sources](compare.md)

-   **Set up keys and sign-ins**

    ---

    TagGroove brings its own keys for Discogs, AcoustID and Last.fm, so you
    enter nothing; a Discogs token and a Last.fm sign-in are optional extras.
    Only fanart.tv takes a key from you. If a site blocks TagGroove with a
    Cloudflare check, an optional solver gets you through.

    [:octicons-arrow-right-24: API keys and sign-ins](api-keys.md)  
    [:octicons-arrow-right-24: Cloudflare solver](cloudflare-solver.md)

-   **Add or write your own sources**

    ---

    Install community `.src` web sources, or write one for a site you use.

    [:octicons-arrow-right-24: Web sources](web-sources.md)

</div>

## Choose a source

<!-- figure:menu-tag-sources -->
<figure markdown>
![The Tag Sources menu open in TagGroove, listing MusicBrainz, Discogs, Beatport, Traxsource, Compare metadata, iTunes, Deezer, AcoustID, freedb, LRCLIB, the cover commands and API keys](../assets/screens/menu-tag-sources-light.png#only-light)
![The Tag Sources menu open in TagGroove, listing MusicBrainz, Discogs, Beatport, Traxsource, Compare metadata, iTunes, Deezer, AcoustID, freedb, LRCLIB, the cover commands and API keys](../assets/screens/menu-tag-sources-dark.png#only-dark)
<figcaption>Every provider has its own command; Compare metadata… puts several side by side.</figcaption>
</figure>
<!-- /figure -->

| Source | Use it for | Setup |
|---|---|---|
| [MusicBrainz](musicbrainz.md) | Track and release metadata and Cover Art Archive artwork | No key |
| [Discogs](discogs.md) | Releases, artist/track-title search, catalog numbers and artwork | TagGroove application credentials or a personal token |
| [Beatport / Traxsource](catalog.md) | Individual tracks, releases, mix/version metadata and artwork | No personal key; a site that blocks TagGroove can use an optional [Cloudflare solver](cloudflare-solver.md) |
| [iTunes](itunes.md) | Store track/album metadata and artwork | No key; choose the store country in Options |
| [Deezer](deezer.md) | Track/album metadata, ISRCs and artwork | No key |
| [LRCLIB](lrclib.md) | Lyrics matched to each selected file | No key |
| [AcoustID](acoustid.md) | Recordings identified by audio fingerprint | None; fingerprinting libraries are included |
| [Last.fm](lastfm.md) | Track/album metadata and listener tags for `GENRE` | None; optional [account sign-in](api-keys.md#lastfm-account) |
| [fanart.tv](fanart-tv.md) | Album covers and disc artwork | [API key](api-keys.md); covers only |
| [freedb and gnudb](freedb.md) / [local freedb](local-freedb.md) | Disc metadata from a CDDB server or local database folder | Configure the server or local folder in Options |

**Covers: MusicBrainz…**, **Covers: Discogs…**, **Covers: iTunes…**,
**Covers: Deezer…** and **Covers: fanart.tv…** write artwork without changing
text fields. A provider may have no result for your edition.

## Search, review and save

1. Select one track or the files for one album. Check their order and existing tags.
2. Choose a source from **Tag Sources**, choose a search type, enter the
   identifying information and select **Search** or **Look up** for an ID.
3. Select a search result and choose **Next**. An ID lookup opens the review
   directly. **Preview** opens the source's page
   in your browser without writing tags.
4. In **Fields**, uncheck values to keep unchanged and edit proposed values
   where needed. Missing source values do not clear existing fields.
5. Check **Tracks and files**. Use **Auto-match**, **Selection order** or
   **Suggest sorting**, then drag files or use row arrows to correct matches.
   AcoustID retains fingerprint pairs; LRCLIB's per-file result keeps file order.
6. Choose whether to write artwork and replace existing covers. **Save image
   to file** also saves it beside the written files, using the configured
   [cover file name](../reference/options/tag-sources.md).
7. Select **Save tags to … files** or **Save cover to … files**. Its count
   shows how many files will be written.

Saving writes immediately. Unmatched files and files with no checked source
values or selected cover are skipped. An eligible successful desktop local-file import is one undo step;
use **Edit → Undo** and read the history notice. Snapshot/storage failures can
leave a successful write without Undo. Provider documents are outside the
desktop local-file journal. See [Undo and history](../tagging/undo-history.md). **Cancel** leaves proposed tag changes unwritten.
**Save cover as…** saves the displayed image separately when requested;
canceling the tagging dialog does not cancel that separate save.

**Save image to file** replaces an existing image with the same name and
extension. **Replace existing artwork** controls embedded covers; it does
not protect an existing external image from that replacement.

Repeat the last source with ++cmd+i++ / ++ctrl+i++. Add Shift
(++cmd+shift+i++ / ++ctrl+shift+i++) to start its search immediately when
the required input is available.

## Compare fields from several sources

**Tag Sources → Compare metadata…** puts several providers side by side for
one track or one album. For every field you choose **Inherit** (keep what the
files have), **None** (remove it) or a provider, then review the combined
result before anything is written. A failed source leaves the others
available.

<!-- figure:compare-results -->
<figure markdown>
![The Compare metadata window after searching: the field table shows how many sources returned each field](../assets/screens/compare-results-light.png#only-light)
![The Compare metadata window after searching: the field table shows how many sources returned each field](../assets/screens/compare-results-dark.png#only-dark)
</figure>
<!-- /figure -->

The full walkthrough, including track matching, per-field choices and partial
save recovery, is on [Compare metadata from several sources](compare.md).

## Set up API keys { #api-keys }

**Tag Sources → API keys…** stores your fanart.tv key; without a built-in or
saved key, fanart.tv stays unavailable. The same dialog signs you in to Last.fm.
AcoustID and Last.fm need no key from you, and Discogs uses its own sign-in. Details and precedence rules are on
[API keys and sign-ins for tag sources](api-keys.md). If a site blocks TagGroove
with a Cloudflare check, see [Cloudflare solver](cloudflare-solver.md).

## Network access and privacy

Online searches send search terms and identifiers to the provider you choose;
AcoustID sends a computed fingerprint, never the audio file. Requests are
spaced per service, so large per-file lookups take time. What is sent, where
keys are stored and how rate limits behave are covered on
[Network access, privacy and rate limits](privacy-and-network.md).

## Related

- [Tag an album from an online database](tag-from-online-sources.md) (tutorial)
- [Compare metadata from several sources](compare.md)
- [MusicBrainz](musicbrainz.md), [Discogs](discogs.md), [Beatport and Traxsource](catalog.md),
  [iTunes / Apple Music](itunes.md), [Deezer](deezer.md), [LRCLIB lyrics](lrclib.md),
  [AcoustID fingerprints](acoustid.md), [Last.fm](lastfm.md), [fanart.tv](fanart-tv.md)
- [freedb and gnudb](freedb.md) and [local freedb](local-freedb.md)
- [Web sources](web-sources.md), [writing a web source](writing-web-sources.md) and the
  [parser reference](web-source-reference.md)
- [Tag Sources options](../reference/options/tag-sources.md) and the
  [Tag Sources menu](../reference/menus/tag-sources.md)
