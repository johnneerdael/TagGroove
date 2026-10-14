#!/usr/bin/env bash
# Fixtures for tests/e2e/general-options.js, written into the album folder ($1):
# a FLAC with ffmpeg's 8 KiB padding, an MP4 with a free box and moov after
# mdat, and a cover image next to the album.
set -euo pipefail
album="$1"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=330:duration=2" "$album/song.flac"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=550:duration=2" -c:a aac "$album/song.m4a"
ffmpeg -loglevel error -y -f lavfi -i "color=red:s=16x16" -frames:v 1 "$album/../cover.png"
