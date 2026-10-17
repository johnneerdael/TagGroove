# Tag Sources → freedb

Open **File → Options… → Tag Sources → freedb** to configure a CDDB server for album metadata. The historical freedb service ended; the saved defaults point to gnudb, and a compatible alternative server can be entered.

!!! tip "New to this?"
    A server address, application acceptance, and an album match are three separate conditions. If the server refuses TagGroove, changing the disc ID will not fix that refusal. A [local snapshot](tag-sources-local-freedb.md) avoids dependence on a remote CDDB server.

## Server list

Select the server to use from **FREEDB SERVER**. **New** adds a configuration; **Delete** removes the selected one while leaving at least one entry. Changing the selected entry changes which server is used after Options **OK**.

TagGroove ships HTTPS and HTTP gnudb entries, with HTTPS selected initially. **Current list of freedb servers** restores that shipped list and selects its first entry, replacing edits in the dialog. Unlike the remote-list retrieval described for Mp3tag, this button does not fetch a fresh list from a server. **Restore page defaults** also restores the page's saved-default choices for later **OK**.

## Server fields

| Field | Meaning and default |
|---|---|
| **NAME** | Label used in the server selector; the initial HTTPS entry is `gnudb (HTTPS)`. It is not the network address. |
| **PROTOCOL** | HTTPS or HTTP. Default HTTPS encrypts the request in transit when the server supports it. |
| **ADDRESS** | Hostname only, initially `gnudb.gnudb.org`. Do not paste a whole URL here. |
| **PATH** | Server endpoint, initially `/~cddb/cddb.cgi`. Keep it separate from the hostname. |
| **PORT** | Optional number from 1 to 65535. Empty uses the protocol's default port. |
| **Your email address (sent to the freedb server)** | Optional configured identity sent in the CDDB handshake, not a login password. Defaults to empty. |

The default endpoint is `https://gnudb.gnudb.org/~cddb/cddb.cgi`. HTTPS encrypts the connection; HTTP does not. Choose a protocol the selected server supports.

## Email and application identity

An address entered here must look like `name@example.org`. Every request sends its user/domain components with `OpenTagAudioTagEditor` and the application version. Empty email sends the developer contact `john@neerdael.nl`. Use an address intended to be shared with the selected server. This identifies the client; it is not a password or account sign-in.

gnudb may accept only applications registered by its operator and may reply `500 Unknown application` otherwise. Changing the email address does not register TagGroove, and saving valid settings does not establish live registration. A refusal message is a service/application registration issue, not evidence that the endpoint settings were saved incorrectly. Choose a compatible server or local database when appropriate. Ordinary timeout/proxy behavior is configured under [Networking](networking.md).

## Disc ID fields

Both switches default to off:

- **Copy disc ID to COMMENT field** includes the source disc ID in the imported comment data. Review the comment in the import before applying it if it already holds notes worth keeping.
- **Copy disc ID to DISCID field** imports the disc ID into its dedicated `DISCID` field, keeping that identity separate from human-readable comments.

The entry's own freedb `DISCID` is used when present, falling back to the listed ID. It can differ from the gnudb **Entry ID** shown in results.

These govern metadata imported from freedb; they do not change the audio or make a server accept the client. The disc ID is not a MusicBrainz release identifier, a barcode, or a filesystem path.

## Search methods

Use [remote freedb lookup](../../tag-sources/freedb.md) by track layout or artist and album. Typed disc-ID lookup is available only for a [local snapshot](../../tag-sources/local-freedb.md). TagGroove asks the server for matches before reading the chosen entry.

Artist/album and artwork search are restricted to recognized gnudb hosts (`gnudb.org` or its subdomains). Other configured CDDB servers use track-layout lookup, even if they implement similar text commands. If the server answers `429 Too Many Requests`, wait before searching again; TagGroove does not automatically repeat that rejected request.

## Related

- [Local freedb](tag-sources-local-freedb.md)
- [Tag Sources cover settings](tag-sources.md)
- [Networking](networking.md)

<!-- Source: PR19 integrated source src/tag-sources.ts FREEDB_SERVERS, server controls, freedbModes and validation; src-tauri/src/sources/freedb.rs hello_identity, request_url, parse_xmcd; sources/mod.rs query-before-read. Source/fixture evidence does not establish live registration. -->
