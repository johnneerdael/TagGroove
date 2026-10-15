#!/usr/bin/env bash
# Fixtures for tests/e2e/chapters.js, written into the album folder ($1): an
# M4B and an MP3 (ID3v2 CHAP) with two chapters each, and a FLAC image with a
# two-track cue sheet beside it.
set -euo pipefail
album="$1"
meta="$(mktemp)"; trap 'rm -f "$meta"' EXIT
printf ';FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=1000\ntitle=Opening\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=1000\nEND=3000\ntitle=Second part\n' >"$meta"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=330:duration=3" -i "$meta" -map 0 -map_chapters 1 \
  -c:a aac -metadata title="Book" -metadata artist="Narrator" "$album/book.m4b"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=440:duration=3" -i "$meta" -map 0 -map_chapters 1 \
  -c:a libmp3lame -metadata title="Episode" -metadata artist="Host" "$album/podcast.mp3"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=550:duration=4" -c:a flac "$album/image.flac"
printf 'PERFORMER "The Trio"\r\nTITLE "Live"\r\nFILE "image.flac" WAVE\r\n  TRACK 01 AUDIO\r\n    TITLE "Opening"\r\n    INDEX 01 00:00:00\r\n  TRACK 02 AUDIO\r\n    TITLE "Ballad"\r\n    INDEX 01 00:02:00\r\n' >"$album/live.cue"
