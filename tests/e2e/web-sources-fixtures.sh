#!/usr/bin/env bash
# Fixtures for tests/e2e/web-sources.js, given the album folder ($1): a small
# web site served on 127.0.0.1:18765 (search page, JSON albums, a cover image)
# and test tag source files in ../web-sources that the suite installs. All
# scripts here are written for TagGroove's tests; no third-party .src is used.
set -euo pipefail
album="$1"
site="$album/../web-site" sources="$album/../web-sources"
mkdir -p "$site/album" "$sources"

cat >"$site/search.html" <<'HTML'
<html><body><ul>
<li class="r"><a href="/album/7.json">Fixture Artist - Fixture Album</a> <span>1999</span> <img src="/cover.png"></li>
<li class="r"><a href="/album/8.json">Someone Else - Other Album</a> <span>2004</span></li>
</ul></body></html>
HTML
cat >"$site/album/7.json" <<'JSON'
{"title":"Fixture Album","artist":"Fixture Artist","year":"1999","genre":"Electronic","cover":"/cover.png",
 "tracks":[{"title":"Web First","ms":3000},{"title":"Web Second","ms":3000}]}
JSON
ffmpeg -loglevel error -y -f lavfi -i color=c=orange:s=64x64 -frames:v 1 "$site/cover.png"

cat >"$sources/&Fixture#&Web Test.src" <<'SRC'
# TagGroove web source test: an HTML index page and a JSON album page.
[Name]=Fixture Web Test
[BasedOn]=http://127.0.0.1:18765
[IndexUrl]=http://127.0.0.1:18765/search.html?q=%s
[AlbumUrl]=http://127.0.0.1:18765
[WordSeparator]=+
[IndexFormat]=%_url%|%Release%|%Year%|%_coverurl%
[SearchBy]=Artist||%artist%||%s$$Album||%album%||+%s
[Encoding]=url-utf-8
[Settings]=&Fixture#Settings.settings
[Include]=Fixture.inc

[ParserScriptIndex]=...
Do
FindLine "<li class=\"r\">" 1 1
IfNot "<li"
Else
FindInLine "href=\""
SayUntil "\""
Say "|"
FindInLine ">"
SayUntil "<"
Say "|"
FindInLine "<span>"
SayNextNumber
Say "|"
SayRegexp "(?<=src=\")[^\"]+"
SayNewline
MoveLine 1
Endif
While "<li"
SRC

cat >"$sources/Fixture.inc" <<'INC'
# Shared album script for the fixture sources.
[ParserScriptAlbum]=...
json "on"
OutputTo "ALBUM"
json_select "title"
SayRest
OutputTo "ARTIST"
json_select "artist"
SayRest
OutputTo "YEAR"
json_select "year"
SayRest
OutputTo "GENRE"
json_select "genre"
SayRest
IfVar "addComment" "true"
OutputTo "COMMENT"
Say "Imported from the fixture site"
Endif
OutputTo "COVERURL"
json_select "cover"
SayRest
json_foreach "tracks"
OutputTo "TITLE"
json_select "title"
SayRest
Say "|"
OutputTo "_LENGTH"
json_select "ms"
SayDuration "ms"
Say "|"
OutputTo "TRACK"
json_foreach_counter
SayRest
Say "|"
json_foreach_end
INC

cat >"$sources/&Fixture#&Direct lookup.src" <<'SRC'
[Name]=Fixture Direct
[AlbumUrl]=http://127.0.0.1:18765/album/%s.json
[SearchBy]=Release ID||7||%s
[Settings]=&Fixture#Settings.settings
[Include]=Fixture.inc
SRC

cat >"$sources/&Fixture#Broken.src" <<'SRC'
[Name]=Fixture Broken
[IndexUrl]=http://127.0.0.1:18765/search.html?q=%s
[IndexFormat]=%_url%|%Release%
[SearchBy]=Artist||%artist%||%s
[ParserScriptIndex]=...
FindLine "this text is not on the page"
[ParserScriptAlbum]=...
SayRest
SRC

cat >"$sources/&Fixture#Settings.settings" <<'JSON'
{"key":"fixture","title":"Fixture Settings","settings":[
  {"type":"heading","key":"importHeading","title":"Import"},
  {"type":"bool","key":"addComment","title":"Write a comment","description":"Adds a COMMENT field","default":false},
  {"type":"string","key":"language","title":"Language","choices":["en","de"],"default":"en"}]}
JSON

# The server must not hold the run's output open or outlive the suite (the
# app itself is stopped after 180 s): it is detached and exits after 150 s.
if [[ "$(uname)" == "Linux" ]]; then
  (cd "$site" && exec setsid -f timeout 150 python3 -m http.server 18765 --bind 127.0.0.1 </dev/null >/dev/null 2>&1)
else
  # macOS has neither setsid nor timeout, and Windows runners name the interpreter python.
  for python in python3 python; do "$python" -c '' 2>/dev/null && break; done
  serve='import functools, http.server, sys, threading
server = http.server.ThreadingHTTPServer(("127.0.0.1", 18765), functools.partial(http.server.SimpleHTTPRequestHandler, directory=sys.argv[1]))
threading.Timer(150, server.shutdown).start()
server.serve_forever()'
  "$python" -I -c "$serve" "$site" </dev/null >/dev/null 2>&1 &
fi
for _ in $(seq 1 50); do
  curl -fs -o /dev/null http://127.0.0.1:18765/search.html && exit 0
  sleep 0.2
done
echo "The fixture web server did not start" >&2
exit 1
