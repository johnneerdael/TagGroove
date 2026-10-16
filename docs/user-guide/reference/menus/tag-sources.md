# Tag Sources menu

Tag sources propose release metadata and artwork for selected audio files.
Select the files of one album, save pending edits, then choose a provider.
A search result is a candidate: review the release, enabled fields and track
matching before importing it.

## Built-in source commands

| Item | Purpose |
| --- | --- |
| **MusicBrainz…** | Search or look up a MusicBrainz release and review its fields/tracks |
| **Discogs…** | Search or look up a Discogs release |
| **Discogs Release ID…** | Open a specific release using its ID/address rather than a broad search |
| **freedb…** | Query the configured online CDDB-compatible server using selected-file timing or an entered disc ID |
| **freedb (local)…** | Search a configured local freedb database/index; Primary+L |
| **Covers: MusicBrainz…** | Look up artwork without importing tag fields |
| **Covers: Discogs…** | Look up artwork without importing tag fields |
| **Discogs access token…** | Configure the token used by the Discogs connection |
| **Options…** | Open the Tag Sources preferences |

The baseline freedb dialog offers **Disc ID of the selected files** and
**freedb disc ID**. The local database can also expose **Artist and album
words** through its text index. This is a lookup from files/identifiers; the
current dialog does not expose a physical-CD-drive selector. Online freedb
free-text search is not one of this baseline dialog's modes.

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
provides further resource pointers. A listed script or provider is not a
compatibility or availability guarantee for OpenTag.

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

<!-- Source baseline main5f4112e. Pending application PRs can change provider, language or protocol behavior; reconcile after integration. No remote-service or native-runtime validation is claimed here. -->
