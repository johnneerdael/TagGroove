#!/usr/bin/env bash
# Fixtures for tests/e2e/sources-4.js, given the album folder ($1): recorded
# answers of iTunes, Deezer, LRCLIB, Last.fm, MusicBrainz release groups and
# fanart.tv, served on 127.0.0.1:18766 where scripted offline runs send the
# built-in sources (src-tauri/src/sources/http.rs, `routed`). A request for
# https://host/path is answered from the file host_path; a file
# host_path@words answers only when the query contains the words. Unknown
# images get a small PNG, anything else 404. All answers were written for
# TagGroove's tests from the services' documented formats.
set -euo pipefail
album="$1"
site="$album/../source-answers"
mkdir -p "$site"
ffmpeg -loglevel error -y -f lavfi -i color=c=teal:s=64x64 -frames:v 1 "$site/cover.png"

# iTunes: an album search and the album lookup with two songs.
cat >"$site/itunes.apple.com_search" <<'JSON'
{"resultCount": 1, "results": [{"wrapperType": "collection", "collectionType": "Album", "artistId": 42, "collectionId": 900001,
 "artistName": "Fixture Apple Artist", "collectionName": "Fixture Apple Album", "country": "USA", "trackCount": 2,
 "collectionViewUrl": "https://music.apple.com/us/album/fixture/900001", "releaseDate": "2011-05-02T07:00:00Z", "primaryGenreName": "Electronic",
 "artworkUrl100": "https://is1-ssl.mzstatic.com/image/thumb/Fixture/cover.png/100x100bb.jpg", "copyright": "℗ 2011 Fixture"}]}
JSON
cat >"$site/itunes.apple.com_lookup" <<'JSON'
{"resultCount": 3, "results": [
 {"wrapperType": "collection", "collectionType": "Album", "artistId": 42, "collectionId": 900001, "artistName": "Fixture Apple Artist",
  "collectionName": "Fixture Apple Album", "collectionViewUrl": "https://music.apple.com/us/album/fixture/900001", "releaseDate": "2011-05-02T07:00:00Z",
  "primaryGenreName": "Electronic", "artworkUrl100": "https://is1-ssl.mzstatic.com/image/thumb/Fixture/cover.png/100x100bb.jpg", "copyright": "℗ 2011 Fixture", "trackCount": 2},
 {"wrapperType": "track", "kind": "song", "collectionId": 900001, "trackId": 900011, "artistName": "Fixture Apple Artist", "trackName": "Apple First", "discCount": 1, "discNumber": 1, "trackCount": 2, "trackNumber": 1, "trackTimeMillis": 3000},
 {"wrapperType": "track", "kind": "song", "collectionId": 900001, "trackId": 900012, "artistName": "Fixture Apple Artist", "trackName": "Apple Second", "discCount": 1, "discNumber": 1, "trackCount": 2, "trackNumber": 2, "trackTimeMillis": 3000, "trackExplicitness": "explicit"}]}
JSON

# Deezer: a search, the album and its tracks (ISRC, disc and position).
cat >"$site/api.deezer.com_search_album" <<'JSON'
{"data": [{"id": 700001, "title": "Fixture Deezer Album", "link": "https://www.deezer.com/album/700001", "record_type": "album", "nb_tracks": 2,
 "cover_medium": "https://cdn-images.dzcdn.net/images/cover/fixture/250x250-000000-80-0-0.jpg", "artist": {"id": 7, "name": "Fixture Deezer Artist"}}], "total": 1}
JSON
cat >"$site/api.deezer.com_album_700001" <<'JSON'
{"id": 700001, "title": "Fixture Deezer Album", "upc": "0123456789012", "link": "https://www.deezer.com/album/700001", "label": "Fixture Records",
 "release_date": "2015-09-01", "genres": {"data": [{"id": 1, "name": "Ambient"}]}, "artist": {"id": 7, "name": "Fixture Deezer Artist"},
 "cover_medium": "https://cdn-images.dzcdn.net/images/cover/fixture/250x250-000000-80-0-0.jpg",
 "cover_big": "https://cdn-images.dzcdn.net/images/cover/fixture/500x500-000000-80-0-0.jpg",
 "cover_xl": "https://cdn-images.dzcdn.net/images/cover/fixture/1000x1000-000000-80-0-0.jpg"}
JSON
cat >"$site/api.deezer.com_album_700001_tracks" <<'JSON'
{"data": [
 {"id": 1, "title": "Deezer First", "isrc": "NLA001500001", "duration": 3, "track_position": 1, "disk_number": 1, "artist": {"name": "Fixture Deezer Artist"}},
 {"id": 2, "title": "Deezer Second", "isrc": "NLA001500002", "duration": 3, "track_position": 2, "disk_number": 1, "artist": {"name": "Fixture Deezer Artist"}}], "total": 2}
JSON

# LRCLIB: lyrics for "Sine A" (exact signature); "Sine B" has none, and the
# search fallback finds nothing either.
cat >"$site/lrclib.net_api_get@track_name=Sine A" <<'JSON'
{"id": 555001, "trackName": "Sine A", "artistName": "Test Artist", "albumName": "Test Album", "duration": 3.0, "instrumental": false,
 "plainLyrics": "Fixture lyric line one\nFixture lyric line two", "syncedLyrics": "[00:00.50] Fixture lyric line one\n[00:01.50] Fixture lyric line two"}
JSON
echo '[]' >"$site/lrclib.net_api_search@track_name=Sine B"

# Last.fm album information (looked up by artist and album).
cat >"$site/ws.audioscrobbler.com_2.0@album.getinfo" <<'JSON'
{"album": {"name": "Fixture Last Album", "artist": "Fixture Last Artist", "url": "https://www.last.fm/music/Fixture/Fixture+Last+Album",
 "image": [{"size": "extralarge", "#text": "https://lastfm.freetls.fastly.net/i/u/300x300/fixture.png"}],
 "tags": {"tag": [{"name": "seen live"}, {"name": "electronic"}, {"name": "ambient"}]},
 "tracks": {"track": [{"name": "Last First", "duration": 3, "@attr": {"rank": 1}, "artist": {"name": "Fixture Last Artist"}},
                      {"name": "Last Second", "duration": 3, "@attr": {"rank": 2}, "artist": {"name": "Fixture Last Artist"}}]}}}
JSON

# fanart.tv: the album is found as a MusicBrainz release group.
cat >"$site/musicbrainz.org_ws_2_release-group" <<'JSON'
{"count": 1, "release-groups": [{"id": "9ba659df-5814-32f6-b95f-02b738698e7c", "title": "Fixture Fanart Album", "primary-type": "Album",
 "first-release-date": "2004-11-23", "artist-credit": [{"name": "Fixture Fanart Artist"}]}]}
JSON
cat >"$site/webservice.fanart.tv_v3_music_albums_9ba659df-5814-32f6-b95f-02b738698e7c" <<'JSON'
{"name": "Fixture Fanart Artist", "mbid_id": "f4a31f0a-51dd-4fa7-986d-3095c40c5ed9", "albums": {"9ba659df-5814-32f6-b95f-02b738698e7c": {
 "albumcover": [{"id": "1", "url": "http://assets.fanart.tv/fanart/music/x/albumcover/fixture-a.jpg", "likes": "1"},
                {"id": "2", "url": "http://assets.fanart.tv/fanart/music/x/albumcover/fixture-b.jpg", "likes": "5"}]}}}
JSON

# AcoustID: a stand-in fpcalc names each file's fingerprint after the file,
# and the lookups identify sine-a as the release's SECOND track and sine-b as
# its first, so the review must pair them by fingerprint, not by order.
cat >"$site/fpcalc" <<'SH'
#!/bin/sh
case "$2" in *sine-a*) fp=FPA ;; *) fp=FPB ;; esac
echo "{\"duration\": 3.0, \"fingerprint\": \"$fp\"}"
SH
chmod +x "$site/fpcalc"
cat >"$site/fpcalc.cmd" <<'CMD'
@echo off
echo %~nx2| findstr /i "sine-a" >nul && (echo {"duration": 3.0, "fingerprint": "FPA"}) || (echo {"duration": 3.0, "fingerprint": "FPB"})
CMD
release=5b11f4ce-a62d-471e-81fc-a69a8278c7da
for spec in "FPA:acoustid-a:6a0dd0f8-0b07-4f43-9b33-000000000002" "FPB:acoustid-b:6a0dd0f8-0b07-4f43-9b33-000000000001"; do
  IFS=: read -r fp id recording <<<"$spec"
  cat >"$site/api.acoustid.org_v2_lookup@fingerprint=$fp" <<JSON
{"status": "ok", "results": [{"id": "$id", "score": 0.97, "recordings": [{"id": "$recording", "artists": [{"name": "Fixture MB Artist"}],
 "releases": [{"id": "$release", "title": "Fixture MB Album", "country": "XW", "date": {"year": 2020}, "track_count": 2}]}]}]}
JSON
done
cat >"$site/musicbrainz.org_ws_2_release_$release" <<JSON
{"id": "$release", "title": "Fixture MB Album", "date": "2020-01-01", "country": "XW", "status": "Official",
 "artist-credit": [{"name": "Fixture MB Artist", "artist": {"id": "0e2b1f4a-6d1c-4f6e-8a8b-000000000010", "name": "Fixture MB Artist"}}],
 "release-group": {"id": "0e2b1f4a-6d1c-4f6e-8a8b-000000000020", "primary-type": "Album"},
 "media": [{"position": 1, "format": "Digital Media", "tracks": [
   {"position": 1, "number": "1", "title": "MB First", "length": 3000, "recording": {"id": "6a0dd0f8-0b07-4f43-9b33-000000000001", "title": "MB First"}},
   {"position": 2, "number": "2", "title": "MB Second", "length": 3000, "recording": {"id": "6a0dd0f8-0b07-4f43-9b33-000000000002", "title": "MB Second"}}]}]}
JSON

serve='import http.server, os, re, sys, threading, urllib.parse
root = sys.argv[1]
class Answers(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass
    def do_GET(self):
        self.answer("")
    def do_POST(self):
        self.answer(self.rfile.read(int(self.headers.get("content-length") or 0)).decode())
    def answer(self, body):
        url = urllib.parse.urlsplit(self.path)
        name = re.sub(r"[^A-Za-z0-9.-]", "_", url.path.strip("/"))
        query = urllib.parse.unquote_plus(url.query + "&" + body)
        files = sorted(os.listdir(root))
        chosen = [f for f in files if f.startswith(name + "@") and f.split("@", 1)[1] in query]
        chosen += [f for f in files if f == name]
        if not chosen and re.search(r"\.(jpg|png)$|/front(-[0-9]+)?$", url.path):
            chosen = ["cover.png"]
        if not chosen:
            self.send_response(404)
            self.send_header("content-length", "0")
            self.end_headers()
            return
        with open(os.path.join(root, chosen[0]), "rb") as f:
            data = f.read()
        self.send_response(200)
        self.send_header("content-type", "image/png" if chosen[0] == "cover.png" else "application/json")
        self.send_header("content-length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)
server = http.server.ThreadingHTTPServer(("127.0.0.1", 18766), Answers)
threading.Timer(175, server.shutdown).start()
server.serve_forever()'
# The server must not hold the run's output open or outlive the suite (the
# app itself is stopped after 180 s): it is detached and stops after 175 s.
for python in python3 python; do "$python" -c '' 2>/dev/null && break; done
if [[ "$(uname)" == "Linux" ]]; then
  setsid -f "$python" -I -c "$serve" "$site" </dev/null >/dev/null 2>&1
else
  "$python" -I -c "$serve" "$site" </dev/null >/dev/null 2>&1 &
fi
for _ in $(seq 1 50); do
  curl -fs -o /dev/null http://127.0.0.1:18766/itunes.apple.com/search && exit 0
  sleep 0.2
done
echo "The fixture answer server did not start" >&2
exit 1
