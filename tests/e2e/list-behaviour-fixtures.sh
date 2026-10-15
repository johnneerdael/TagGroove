#!/usr/bin/env bash
# Fixtures for tests/e2e/list-behaviour.js, written into the album folder ($1):
# a folder image (folder.png) and a third MP3 with its own embedded front cover.
set -euo pipefail
album="$1"
art="$(mktemp -d)"; trap 'rm -rf "$art"' EXIT
ffmpeg -loglevel error -y -f lavfi -i "color=blue:s=8x8" -frames:v 1 "$album/folder.png"
ffmpeg -loglevel error -y -f lavfi -i "color=red:s=16x16" -frames:v 1 "$art/front.png"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=880:duration=3" -i "$art/front.png" \
  -map 0:a -map 1:v -c:v copy -id3v2_version 3 -disposition:v attached_pic \
  -metadata:s:v title="Album cover" -metadata:s:v comment="Cover (front)" \
  -metadata title="Sine C" -metadata artist="Test Artist" -metadata album="Test Album" -metadata track=3 \
  "$album/sine-c.mp3"
