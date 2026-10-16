# Tag Sources options

Open **File → Options… → Tag Sources** for cover-image output and source diagnostics. Server settings are under [freedb](tag-sources-freedb.md); historical offline databases are under [Local freedb](tag-sources-local-freedb.md).

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

This setting does not search for local images and does not choose the starting folder of **Add cover…**; that is [Directories → Cover art directory](directories.md).

## MusicBrainz cover size

Choose **250 pixels**, **500 pixels (default)**, **1200 pixels**, or **Original size**. The choice is used for MusicBrainz artwork from the Cover Art Archive, including its cover-source workflows.

Smaller images reduce transfer and storage requirements. Choose 1200 or Original when a high-resolution image is needed, accepting that the original can be much larger. This requests the source's image variant; it is not a local resizer for covers already embedded in files.

## Write debug output for web sources

Defaults to off. Enable to write downloaded pages and a step-by-step parser trace for `.src` web-source searches and album lookups. Find the `debug` folder through **Tag Sources → Open sources folder**.

Use the trace to identify whether a source's page structure or parsing steps have changed. Disable it when finished to avoid accumulating downloaded responses. A response or trace can contain search text and service data; inspect it before sharing it in an issue.

## Credentials and service-specific settings

The Discogs token is set through **Tag Sources → Discogs access token…**, independently of these ordinary settings. A correct cover-size choice does not provide authentication or bypass service restrictions.

Additional online-service controls belong to the builds that include those services. This page documents the baseline controls above; it does not imply that a future provider shown on a development branch is already present in a release.

## Related

- [freedb server settings](tag-sources-freedb.md)
- [Local freedb database](tag-sources-local-freedb.md)
- [Networking](networking.md)
- [Cover preservation messages](messages.md)

<!-- verified: baseline007a806 src/tag-sources.ts tagSourcesOptionsPages, coverTemplate; src-tauri/src/sources MusicBrainz cover-size and cover-output settings; web debug output. PR20 adds Online services, deliberately not described as baseline controls here. -->
