#!/usr/bin/env bash
# Fixtures for tests/e2e/actions-polish.js, given the album folder ($1): a
# Windows-1252 text file without a BOM and a cover image in an Artwork folder.
set -euo pipefail
album="$1"
printf 'Caf\xe9 \x80 \x93quoted\x94' > "$album/ansi.txt"
mkdir -p "$album/Artwork"
ffmpeg -loglevel error -y -f lavfi -i "color=c=red:s=400x200" -frames:v 1 "$album/Artwork/front.png"
