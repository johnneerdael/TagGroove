# Tag Sources → freedb

Open **File → Options… → Tag Sources → freedb** to configure a CDDB server for album metadata. The historical freedb service ended; the saved defaults point to gnudb, and a compatible alternative server can be entered.

!!! tip "New to this?"
    A server address, application acceptance, and an album match are three separate conditions. If the server refuses OpenTag, changing the disc ID will not fix that refusal. A [local snapshot](tag-sources-local-freedb.md) avoids dependence on a remote CDDB server.

## Server list

Select the server to use from **FREEDB SERVER**. **New** adds a configuration; **Delete** removes the selected one while leaving at least one entry. Changing the selected entry changes which server is used after Options **OK**.

OpenTag ships HTTPS and HTTP gnudb entries, with HTTPS selected initially. **Current list of freedb servers** restores that shipped list and selects its first entry, replacing edits in the dialog. Unlike the remote-list retrieval described for Mp3tag, this button does not fetch a fresh list from a server. **Restore page defaults** also restores the page's saved-default choices for later **OK**.

## Server fields

| Field | Meaning and default |
|---|---|
| **NAME** | Label used in the server selector; the initial HTTPS entry is `gnudb (HTTPS)`. It is not the network address. |
| **PROTOCOL** | HTTPS or HTTP. Default HTTPS encrypts the request in transit when the server supports it. |
| **ADDRESS** | Hostname only, initially `gnudb.gnudb.org`. Do not paste a whole URL here. |
| **PATH** | Server endpoint, initially `/~cddb/cddb.cgi`. Keep it separate from the hostname. |
| **PORT** | Optional number from 1 to 65535. Empty uses the protocol's default port. |
| **Email address** | Optional configured identity sent in the CDDB handshake, not a login password. Defaults to empty. |

The handshake contains a user/host identity plus OpenTag's application identity and version. On the guide's baseline source, leaving email empty uses the fallback `opentag` / `localhost` identity. Newer development code changes that fallback and validates the address; do not assume a newer service-registration fix has shipped merely because the options are visible.

gnudb may accept only applications registered by its operator. A refusal message is a service/application registration issue, not evidence that the endpoint settings were saved incorrectly. Choose a compatible server or local database when appropriate. Ordinary timeout/proxy behavior is configured under [Networking](networking.md).

## Disc ID fields

Both switches default to off:

- **Copy disc ID to COMMENT field** includes the source disc ID in the imported comment data. Review the comment in the import before applying it if it already holds notes worth keeping.
- **Copy disc ID to DISCID field** imports the disc ID into its dedicated `DISCID` field, keeping that identity separate from human-readable comments.

These govern metadata imported from freedb; they do not change the audio or make a server accept the client. The disc ID is not a MusicBrainz release identifier, a barcode, or a filesystem path.

## Related

- [Local freedb](tag-sources-local-freedb.md)
- [Tag Sources cover settings](tag-sources.md)
- [Networking](networking.md)

<!-- verified: baseline007a806 src/tag-sources.ts FREEDB_SERVERS, freedbServerList, bindServers, tagSourcesOptionsPages; src-tauri/src/sources/freedb.rs settings_from, command_url, hello_identity. PR20 103019c changes client/email protocol; baseline wording intentional. -->
