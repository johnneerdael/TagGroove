# Last.fm

Last.fm supplies album and track metadata, cover art and listener tags that
TagGroove can write as `GENRE`. It needs no key from you: TagGroove has its own
Last.fm application key, so the source works as soon as you open it. You can
also [sign in to your Last.fm account](api-keys.md#lastfm-account) to make
requests as that account; nothing else changes.

Choose **Tag Sources → Last.fm…**. **Artist and album** looks up that album
directly; **Album title words** searches by album title. **Artist and track title**
searches recordings and loads track details, including available album context.
Track artists and album artists remain separate. **MusicBrainz release
ID** accepts a release MBID for a direct lookup.

The review shows available album and track metadata, cover art and filtered,
title-cased listener tags as `GENRE`. Choose the genre count in **Options →
Tag Sources → Online services**. Personal collection tags and bare numbers
are filtered, but review the remaining genres. Artwork is limited to the
images Last.fm supplies, typically up to 300 pixels.
