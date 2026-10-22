# Tag Sources menu

Tag sources propose release metadata and artwork for selected audio files.
Select the files of one album, save pending edits, then choose a provider.
A search result is a candidate: review the release, enabled fields and track
matching before importing it.

## Built-in source commands

| Item | Purpose |
| --- | --- |
| **MusicBrainz…** | Search or look up a MusicBrainz release and review its fields/tracks ([MusicBrainz](../../tag-sources/musicbrainz.md)) |
| **Discogs…** | Search or look up a Discogs release ([Discogs](../../tag-sources/discogs.md)) |
| **Discogs Artist + Title…** | Find releases by the performing artist and track title; choose the exact track before saving |
| **Beatport…** / **Traxsource…** | Search tracks and complete releases, or open a track’s exact release ([Beatport and Traxsource](../../tag-sources/catalog.md)) |
| **Compare metadata…** | Compare a track or album across native and installed sources; choose Inherit, None or a provider per field ([Compare metadata](../../tag-sources/compare.md)) |
| **Discogs Release ID…** | Open a specific release using its ID/address rather than a broad search |
| **iTunes…** | Search the iTunes / Apple Music catalogue for track or album metadata and artwork ([iTunes](../../tag-sources/itunes.md)) |
| **Deezer…** | Search Deezer for track or album metadata, ISRCs and artwork ([Deezer](../../tag-sources/deezer.md)) |
| **Last.fm…** | Look up album or track metadata, cover art and listener tags for `GENRE`; needs an API key ([Last.fm](../../tag-sources/lastfm.md)) |
| **AcoustID (fingerprints)…** | Identify recordings from an audio fingerprint computed in the app; needs an API key ([AcoustID](../../tag-sources/acoustid.md)) |
| **freedb…** | Query the configured server by selected-file track layout, or search gnudb by artist and album |
| **freedb (local)…** | Search a configured local freedb database/index; Primary+L |
| **Lyrics: LRCLIB…** | Fetch lyrics matched to each selected file ([LRCLIB](../../tag-sources/lrclib.md)) |
| **Covers: freedb (gnudb)…** | Search gnudb artwork without importing tag fields |
| **Covers: MusicBrainz…** | Look up artwork without importing tag fields |
| **Covers: Discogs…** | Look up artwork without importing tag fields |
| **Covers: iTunes…** / **Covers: Deezer…** | Look up store artwork without importing tag fields |
| **Covers: fanart.tv…** | Choose album covers or disc artwork; needs an API key ([fanart.tv](../../tag-sources/fanart-tv.md)) |
| **Install tag sources…** / **Open sources folder** | Add or reveal installed web sources on desktop platforms with folder access (see below) |
| **Discogs access token…** | Configure the token used by the Discogs connection |
| **API keys…** | Save or remove your AcoustID, Last.fm and fanart.tv keys ([API keys](../../tag-sources/api-keys.md)) |
| **Options…** | Open the Tag Sources preferences |

[All tag sources](../../tag-sources/index.md) explains how to choose between
these and how the shared search, review and save dialog works.

The remote [freedb dialog](../../tag-sources/freedb.md) offers **Track layout
of the selected files** and **Artist and album**. Text and artwork searches
require a recognized gnudb host. TagGroove asks for matches before reading the
chosen entry; it does not read a remote entry from a typed disc ID.

The [local database](../../tag-sources/local-freedb.md) supports a disc ID
derived from the selected files or typed directly, plus **Artist and album
words** through its text index. These are lookups from files/identifiers;
the dialog does not expose a physical-CD-drive selector.

Configure provider/server and local-index settings under
[Options → Tag Sources](../options/tag-sources.md),
[freedb](../options/tag-sources-freedb.md) and
[Local freedb](../options/tag-sources-local-freedb.md). Server availability,
authentication and network policy still affect whether a live query succeeds.
These menu entries do not certify that a remote service accepts every saved
configuration.

## Last used source and quick search

Primary+I reopens the last used source, including an installed web source.
The corresponding menu entry can show that shortcut. Primary+Shift+I requests
a quick search for built-in sources using the selected album/artist or disc
information. If the search returns one result, it can open that release
without the result-list step. Review/import remains a separate decision;
quick search is not a command to trust an arbitrary match.

An installed web script has its own search fields and is not the same quick
built-in-provider route. Use its visible dialog when search details need
editing. Primary means Ctrl on Windows/Linux and Command on macOS.

## Installed web sources

On desktop platforms with folder access, **Install tag sources…** copies
supported source files into the configuration's sources folder. **Open sources
folder** reveals it. Installed scripts and settings can create submenus;
`#` in a source filename separates menu levels, and `&` marks an access key.

A source normally searches a page/API, lists releases, then parses the chosen
release for review. It does not run page JavaScript as a browser would.
Companion include/settings files can be required. Inspect the source's origin,
configuration and output before using it on important tags. A script created
for another application can need adaptation.

The original documentation's [web-source archive](https://docs.mp3tag.de/tag-sources/archive/)
points to further resources. A listed script or provider is not a
compatibility or availability guarantee for TagGroove.

## Match files before applying

For example, if two selected files are `Paper Boats` and `Lanterns on the
Water`, choosing a release with the same album title is not enough. Check its
edition, track count and order, and match those two file rows to the intended
source tracks. Moving rows in source review changes matching, not file paths.

The enabled field set determines which proposed values are imported. Field
selection and enabled state are separate. A covers-only command deliberately
leaves tag fields alone. Read the Apply result and any per-file errors before
assuming the entire selected album was updated.

Source review supports F2 to edit a field, Space to toggle enabled fields,
Primary+A/Primary+Shift+A for selection and Primary+E/D to enable/disable all.
Alt+Up/Down or Alt+Page Up/Down moves file rows for matching. See
[keyboard shortcuts](../keyboard-shortcuts.md) for the complete focus rules.

<!-- Source: integrated PR19 src/main.ts Tag Sources menu; src/tag-sources.ts freedbModes and sourceOperation. No remote-service or native-runtime validation is claimed here. -->
