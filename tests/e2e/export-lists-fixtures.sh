#!/usr/bin/env bash
# Fixtures for tests/e2e/export-lists.js, written into the album folder ($1):
# a subfolder disc2/ with a third MP3 that carries an embedded 16x16 front cover,
# so per-directory exports have two folders and the cover window has artwork.
set -euo pipefail
album="$1"
art="$(mktemp -d)"; trap 'rm -rf "$art"' EXIT
mkdir -p "$album/disc2"
ffmpeg -loglevel error -y -f lavfi -i "color=red:s=16x16" -frames:v 1 "$art/front.png"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=880:duration=3" -i "$art/front.png" \
  -map 0:a -map 1:v -c:v copy -id3v2_version 3 -disposition:v attached_pic \
  -metadata:s:v title="Album cover" -metadata:s:v comment="Cover (front)" \
  -metadata title="Sine C" -metadata artist="Test Artist" -metadata album="Test Album" -metadata track=3 \
  "$album/disc2/sine-c.mp3"
