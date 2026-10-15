#!/usr/bin/env bash
# Fixtures for tests/e2e/remove-mapping.js, written into the album folder ($1):
# a FLAC whose Vorbis comments hold DATE, for the Options → Tags → Mapping rule.
set -euo pipefail
album="$1"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=330:duration=2" \
  -metadata title="Mapped" -metadata date="2024" "$album/song.flac"
