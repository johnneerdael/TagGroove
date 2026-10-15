#!/usr/bin/env bash
# Fixtures for tests/e2e/library.js: one MP3 in a folder next to the album,
# outside the directory the suite includes in the library.
set -euo pipefail
album="$1"
mkdir -p "$album/../elsewhere"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=880:duration=2" \
  -metadata title="Elsewhere" -metadata artist="Test Artist" "$album/../elsewhere/other.mp3"
