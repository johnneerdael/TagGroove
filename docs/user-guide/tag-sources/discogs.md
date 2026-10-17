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

## Authentication

Open **Tag Sources → Discogs access token…** to see this build's setup.
Requests use your personal token first, then a built-in application key if
available, or otherwise anonymous access. Not every build includes an
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

Anonymous requests are spaced about 2.5 seconds apart; authenticated requests
about 1.1 seconds apart and can supply search thumbnails. Check or replace a
rejected token. A quota error may still require waiting. TagGroove uses release
lookups, not account collection management or an OAuth sign-in flow.

## Related

- [Search, review and save](index.md#search-review-and-save)
- [Network access and privacy](index.md#network-access-and-privacy)
- [Tag Sources options](../reference/options/tag-sources.md)
