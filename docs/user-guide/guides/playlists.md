---
status: draft
---

# Playlists and cue sheets

!!! abstract "This page is being written"

    It will cover:

    - Playlist all/selected/per folder, Generate playlists, extended info, relative paths, auto-create, loading m3u/m3u8/pls/cue

    Until then, the [reference section](../reference/index.md) has the details.

## Verified behavior for the completed guide

On macOS/Linux, `.m3u` output uses Windows-1252 only when it can be read back
unchanged. It falls back to UTF-8 for unrepresentable characters and for
Windows-1252 bytes that would be mistaken for different UTF-8 text, such as
a filename containing `Â©`. `.m3u8` stays UTF-8. The Windows system-codepage
path is unchanged. Check the playlist in its receiving player as well as
loading it back into OpenTag.
