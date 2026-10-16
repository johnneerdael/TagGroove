#!/usr/bin/env bash
# Fixtures for tests/e2e/gnudb.js, given the album folder ($1): a mock CDDB
# server on 127.0.0.1:18766 that answers like gnudb (gnudb.org/howtognudb.php)
# and enforces its access policy. Nothing here contacts gnudb.
#  - The hello must be "<user> <domain> OpenTagAudioTagEditor v<version>"
#    with a valid-looking domain, and proto=6; otherwise "500 Unknown
#    application" (as gnudb answers unregistered clients).
#  - "cddb read" is answered only for a category and ID that a query or
#    search listed before; anything else is "401".
#  - Entries put the request's hello into EXTD, so the suite sees it as the
#    COMMENT value in the review.
#  - A search for the artist "Busy" answers 429 once and matches afterwards:
#    a client that retried on its own would show results.
#  - Every other command (submit, write, …) is a syntax error.
set -euo pipefail
album="$1"
mock="$album/../cddb-mock"
mkdir -p "$mock"
cat >"$mock/server.py" <<'PY'
import http.server, re, sys, threading, urllib.parse

LISTED = set()
STATE = {"busy": 0}
HELLO = re.compile(r"^(\S+) (\S+) OpenTagAudioTagEditor v\d+\.\d+\.\d+\S*$")
COVER = "https://coverartarchive.org/release/{}/{}-500.jpg"
RELEASES = ["16efdbf1-47c9-4179-ad23-4c7bc7fda547", "2e796f5f-082e-427a-9a49-f3ed54e69406",
            "5c7ff4fc-bc24-4375-b587-c57ad158e361", "8763f781-57b1-4c84-90da-5cb7180873da"]


def listing(code, text, lines):
    return "\r\n".join([f"{code} {text}, list follows (until terminating `.')"] + lines + ["."]) + "\r\n"


def entry(category, disc, hello, covers):
    artist, album = {"folk/1a2b3c4d": ("Fixture Artist", "Layout Album"),
                     "rock/0f00aa02": ("Fixture Artist", "Text Album"),
                     "misc/0f00bb02": ("Other Artist", "Other Album")}[f"{category}/{disc}"]
    lines = ["# xmcd", "#", "# Track frame offsets:", "#\t150", "#\t375", "#", "# Disc length: 8 seconds", "#",
             "# Revision: 1", "# Submitted via: OpenTag e2e mock"]
    if covers:
        lines += ["# Cover: " + COVER.format(RELEASES[3], "38592060423"), "# Artid: " + RELEASES[3], "# Artid: " + RELEASES[2]]
    lines += ["#", "DISCID=0a000802", f"DTITLE={artist} / {album}", "DYEAR=2004", "DGENRE=Folk",
              "TTITLE0=Mock First", "TTITLE1=Mock Second", f"EXTD=hello: {hello}", "EXTT0=", "EXTT1=", "PLAYORDER="]
    return f"210 {category} {disc} CD database entry follows (until terminating `.')\r\n" + "\r\n".join(lines) + "\r\n.\r\n"


def answer(query):
    hello = query.get("hello", [""])[0]
    if query.get("proto", [""])[0] != "6":
        return "501 Illegal protocol level.\r\n"
    match = HELLO.match(hello)
    if not match or "." not in match.group(2) or match.group(2) == "localhost" or match.group(1) == "opentag":
        return "500 Unknown application, developer email for " + hello + "\r\n"
    cmd = query.get("cmd", [""])[0]
    words = cmd.split()
    if words[:2] == ["cddb", "query"] and len(words) >= 5:
        LISTED.add("folk/1a2b3c4d")
        return listing(211, "Found inexact matches", ["folk 1a2b3c4d Fixture Artist / Layout Album"])
    if cmd == "search":
        artist = query.get("artist", [""])[0]
        if artist == "Busy":
            STATE["busy"] += 1
            if STATE["busy"] == 1:
                return "429 Too Many Requests\r\n"
            LISTED.add("misc/0f00bb02")
            return listing(200, "Found 1 matches", ["misc 0f00bb02 Retried / Automatically"])
        if query.get("tracks", [""])[0] != "2":
            return "202 No match found\r\n"
        LISTED.update(["rock/0f00aa02", "misc/0f00bb02"])
        return listing(200, "Found 2 matches", ["rock 0f00aa02 Fixture Artist / Text Album", "misc 0f00bb02 Other Artist / Other Album"])
    if cmd == "artsearch":
        return listing(200, "Found 5 matches", [COVER.format(RELEASES[0], "32779693860"), COVER.format(RELEASES[1], "37994282199")] + RELEASES[1:] + ["f4371769-40d1-4ba7-8023-91024320bda8"])
    if words[:2] == ["cddb", "read"] and len(words) == 4:
        key = f"{words[2]}/{words[3]}"
        if key not in LISTED:
            return f"401 {words[2]} {words[3]} No such CD entry in database.\r\n"
        return entry(words[2], words[3], hello, key == "rock/0f00aa02")
    return "500 Command syntax error.\r\n"


class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        url = urllib.parse.urlsplit(self.path)
        if url.path == "/ready":
            body = "ok"
        elif url.path == "/~cddb/cddb.cgi":
            body = answer(urllib.parse.parse_qs(url.query, keep_blank_values=True))
        else:
            self.send_error(404)
            return
        data = body.encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *args):
        pass


server = http.server.ThreadingHTTPServer(("127.0.0.1", 18766), Handler)
threading.Timer(float(sys.argv[1]), server.shutdown).start()
server.serve_forever()
PY
# The server must not hold the run's output open or outlive the suite (the
# app itself is stopped after 180 s): it is detached and exits after 150 s.
for python in python3 python; do "$python" -c '' 2>/dev/null && break; done
if [[ "$(uname)" == "Linux" ]]; then
  setsid -f "$python" -I "$mock/server.py" 150 </dev/null >/dev/null 2>&1
else
  "$python" -I "$mock/server.py" 150 </dev/null >/dev/null 2>&1 &
fi
for _ in $(seq 1 50); do
  curl -fs -o /dev/null http://127.0.0.1:18766/ready && exit 0
  sleep 0.2
done
echo "The mock CDDB server did not start" >&2
exit 1
