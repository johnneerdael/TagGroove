#!/usr/bin/env bash
# Fixtures for tests/e2e/sources-3.js. The album ($1) holds two WAV files of
# exactly three seconds, so the disc ID of the selection is 07000602; the
# local freedb folder beside it gains that disc and a Windows-layout bundle
# for the text index.
set -euo pipefail
album="$1"
freedb="$(dirname "$album")/freedb"
rm -f "$album"/*.mp3
for spec in "a:440:1" "b:660:2"; do
  IFS=: read -r name freq track <<<"$spec"
  ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=$freq:duration=3" -c:a pcm_s16le \
    -metadata title="Tone $name" -metadata artist="Test Artist" -metadata album="Test Album" \
    -metadata track="$track" "$album/tone-$name.wav"
done
mkdir -p "$freedb/misc" "$freedb/jazz"
cat >"$freedb/misc/07000602" <<'XMCD'
# xmcd
#
# Track frame offsets:
#	150
#	375
#
# Disc length: 8 seconds
#
DISCID=07000602
DTITLE=Quick Artist / Quick Album
DYEAR=2004
DGENRE=Ambient
TTITLE0=Quick One
TTITLE1=Quick Two
EXTD=
EXTT0=
EXTT1=
PLAYORDER=
XMCD
cat >"$freedb/jazz/10to1f" <<'XMCD'
#FILENAME=10000001
# xmcd
#
# Track frame offsets:
#	150
#	375
#
# Disc length: 8 seconds
#
DISCID=10000001
DTITLE=Café Quartet / Night Set
DYEAR=1999
DGENRE=Jazz
TTITLE0=Late Tune
TTITLE1=Early Tune
EXTD=
EXTT0=
EXTT1=
PLAYORDER=
XMCD
