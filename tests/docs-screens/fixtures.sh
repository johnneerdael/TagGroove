#!/usr/bin/env bash
# Fixtures for the user guide's screenshots (tests/docs-screens/screens.js),
# written into the album folder ($1): an invented eight-track album with an
# embedded front cover, plausible lengths and deliberately untidy filenames so
# converters have something to show. Every name and image is made up here.
set -euo pipefail
album="$1"
art="$(mktemp -d)"; trap 'rm -rf "$art"' EXIT
mkdir -p "$album"
ffmpeg -loglevel error -y -f lavfi \
  -i "gradients=s=600x600:c0=0xFFC72C:c1=0xFF7800:c2=0xF23D0B:c3=0x101922:nb_colors=4:x0=0:y0=0:x1=600:y1=600" \
  -frames:v 1 "$art/front.jpg"
i=0
while IFS='|' read -r file title seconds; do
  i=$((i + 1))
  ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=$((220 + 55 * i)):duration=$seconds" -i "$art/front.jpg" \
    -map 0:a -map 1:v -c:a libmp3lame -b:a 192k -c:v copy -id3v2_version 3 -disposition:v attached_pic \
    -metadata:s:v title="Album cover" -metadata:s:v comment="Cover (front)" \
    -metadata title="$title" -metadata artist="Juniper Vale" -metadata album_artist="Juniper Vale" \
    -metadata album="Harbour Lights" -metadata date=2024 -metadata genre="Indie Folk" \
    -metadata track="$i/8" -metadata composer="M. Okafor" \
    "$album/$file.mp3"
done <<'TRACKS'
01_low_tide|Low Tide|201
02 - Lanterns On The Water|Lanterns on the Water|244
03_paper boats|Paper Boats|187
track04|North Pier|229
05 - juniper vale - salt & cedar|Salt & Cedar|212
06_the_keeper's_song|The Keeper's Song|268
07 Fog Horn (Interlude)|Fog Horn (Interlude)|96
08 - Harbour Lights|Harbour Lights|301
TRACKS
