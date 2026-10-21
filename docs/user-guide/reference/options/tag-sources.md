# Tag Sources options

Open **File → Options… → Tag Sources** or **Tag Sources → Options…** for source lookups, cover-image output and diagnostics. Server settings are under [freedb](tag-sources-freedb.md); historical offline databases are under [Local freedb](tag-sources-local-freedb.md).

!!! tip "New to this?"
    Saving an image beside an album and embedding it in audio files are separate actions. This page supplies the external image's name; the import review determines what is actually saved.

## Default file name for cover art

Defaults to `folder`. **Save image to file** in a tag-source dialog uses this name/format string, fills placeholders from the first written audio file of each destination folder, and adds an extension for the image type.

For example, `%albumartist% - %album%` can produce `Juniper Vale - Harbour Lights.jpg` when those fields are present and the chosen image is JPEG. The exact extension follows the downloaded image, so do not assume a JPEG result for every source. A shared album image is named once per folder, not once per track title.

The format also has the app’s working-folder context (`%_workingdir%`,
`%_workingpath%`) and active File List column functions. For example, a
`$columnValues` column based on `%ALBUM%` uses the newly imported album
when the external image is saved. Review the resulting path and any file
errors. This names a separate image; it does not rename audio or choose an
embedded picture type.

**Save image to file** replaces an existing image of the resulting name and
extension. The review's **Replace existing artwork** choice applies to embedded
covers, not this external image file.

This setting does not search for local images and does not choose the starting folder of **Add cover…**; that is [Directories → Cover art directory](directories.md).

## MusicBrainz cover size

Choose **250 pixels**, **500 pixels (default)**, **1200 pixels**, or **Original size**. The choice is used for MusicBrainz artwork from the Cover Art Archive, including its cover-source workflows, and for Cover Art Archive images listed by gnudb.

Smaller images reduce transfer and storage requirements. Choose 1200 or Original when a high-resolution image is needed, accepting that the original can be much larger. This requests the source's image variant; it is not a local resizer for covers already embedded in files.

## Write debug output for web sources

Defaults to off. This traces `.src` scripts, not built-in provider lookups. Enable to write downloaded pages and a step-by-step parser trace for `.src` web-source searches and album lookups. Find the `debug` folder through **Tag Sources → Open sources folder**.

Use the trace to identify whether a source's page structure or parsing steps have changed. Disable it when finished to avoid accumulating downloaded responses. A response or trace can contain search text and service data; inspect it before sharing it in an issue.

## Online services

| Setting | Choices and default |
|---|---|
| **API keys…** | Open the AcoustID, Last.fm and fanart.tv key dialog; storage is separate from ordinary Options. |
| iTunes **Store country (two letters)** | For example `US` (default) or `NL`; an empty value uses `US`. |
| iTunes **Cover size** | 600, 1200 (default) or 3000 pixels |
| Deezer **Cover size** | 500 or 1000 (default) pixels |
| **Write synchronized lyrics (LRC time stamps) when available** | Off by default; enable to prefer available LRC text in `UNSYNCEDLYRICS`. |
| Last.fm **Tags written to GENRE** | The top 1 (default), 2, 3 or 5 usable tags |
| **Optional fpcalc override (desktop)** | Empty uses built-in Chromaprint and audio decoding. An explicit executable path selects an optional desktop producer. Bare legacy `fpcalc` / `fpcalc.exe` settings use the built-in default; mobile always uses the built-in libraries. |

Cover sizes request provider image variants without guaranteeing availability
or detail. Last.fm has no cover size option; its supplied images are generally
smaller.

Choose **OK** to save Options. API keys have their own **Save** button and
do not wait for Options OK. Use **Tag Sources → Discogs access token…** for
its separate credential and optional keychain setting.

The **freedb** and **Local freedb** subpages configure a CDDB server and local
database folder. They do not choose servers for the other online services.

## Traxsource

The **Traxsource** subpage configures an optional
FlareSolverr-compatible challenge solver. Leave **Traxsource solver
endpoint** empty for direct access, or enter a trusted HTTP/HTTPS endpoint
without credentials in its address. Saving an endpoint allows challenged Traxsource
page requests to use it. Beatport uses its direct catalog API and never uses
this solver. The endpoint is stored in ordinary Options.

**Solver timeout (seconds)** defaults to 120 and accepts 1–300.
**Retry direct Traxsource access** clears remembered access restrictions;
use it after updating the solver or fixing connectivity. It does not change
tags. Read the [catalog source guide](../../tag-sources/catalog.md) for the
track/release workflow and solver privacy details.

## Related

- [freedb server settings](tag-sources-freedb.md)
- [Local freedb database](tag-sources-local-freedb.md)
- [Tag-source workflow and API keys](../../tag-sources/index.md)
- [LRCLIB lyrics](../../tag-sources/lrclib.md)
- [AcoustID setup](../../tag-sources/acoustid.md)
- [Networking](networking.md)
- [Cover preservation messages](messages.md)

<!-- verified: src/tag-sources.ts:tagSourcesOptionsPages,coverTemplate; src-tauri/src/sources online settings, credential commands and image output. Working-folder/column naming context retained from reviewed main. -->
