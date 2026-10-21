# Discogs

Use **Tag Sources → Discogs…** for release metadata and artwork. An exact
release helps distinguish editions with different tracks, catalog numbers
or covers.

## Find a release

Select the album's files and choose **Discogs…**. Search by **Artist and
album**, **Free text**, **Barcode** or **Catalog number**, or use **Release
ID or address**. **Tag Sources → Discogs Release ID…** opens the direct lookup.

Use a numeric release ID or an address containing `/release/`. A Discogs
master groups editions; choose the individual release rather than its master
ID. Select a search result and choose **Next**, or use **Look up** for a
direct release lookup. Then check the
[fields, artwork and file matches](index.md#search-review-and-save).

**Covers: Discogs…** writes artwork without changing text fields. Available
images and thumbnails depend on authentication and what Discogs supplies.

## Find a track by artist and title

Select one file and choose **Tag Sources → Discogs Artist + Title…**, or choose
**Artist and track title** in the ordinary Discogs dialog. The query starts
with the file's performing artist and complete title, including the mix/version.
Search returns releases that may contain that track, with edition information
such as label, catalog number and year.

Choose a release and select **Next**. For a release with several tracks, choose
the exact row using **Use track for this file**. The file remains unmatched
until a track is selected; the release's first track is not chosen automatically.
Check the proposed title, track position and fields before saving.

## Authentication

Open **Tag Sources → Discogs access token…** to see this build's setup.
Requests use your personal token first, then a built-in application key if
available. Without either credential, API lookups report a setup error; they
do not silently fall back to anonymous access. Not every build includes an
application key.

Create a personal access token in your Discogs account's **Settings →
Developers**, enter it in TagGroove and choose **Save**. An empty input keeps
the saved token; **Remove the saved token** deletes it. The stored token
is not displayed again.

When offered, **Keep the token in the system keychain** moves it to macOS
Keychain, Windows Credential Manager or the Linux Secret Service. Otherwise
it stays in the private configuration file. Mobile builds use the file.
The token moves when you save the dialog; keychain failures are reported
rather than silently deleting the token.

TagGroove application credentials allow at most **25 requests per minute**;
a saved personal token allows at most **60 requests per minute**. Requests are
spaced about 2.5 seconds apart for application access and 1.1 seconds apart for
user authentication. Search, pagination, release details and other Discogs API
requests share the same request gate, including Discogs API calls made by
installed `.src` web sources. Switching source commands does not start a new
budget. Search thumbnails depend on authentication and provider data. Check or
replace a rejected token. A quota error may still require waiting. TagGroove uses release
lookups, not account collection management or an OAuth sign-in flow.

## Related

- [Search, review and save](index.md#search-review-and-save)
- [Network access and privacy](index.md#network-access-and-privacy)
- [Tag Sources options](../reference/options/tag-sources.md)
