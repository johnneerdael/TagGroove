#!/usr/bin/env bash
# Fixtures for tests/e2e/chapter-fields.js, written into the album folder
# ($1): an MP3 (ID3v2 CHAP), an M4B and an MKA with two chapters each, and two
# FLAC files with a cue sheet whose second track starts with a pregap at the
# end of the first file and plays from the second.
set -euo pipefail
album="$1"
meta="$(mktemp)"; trap 'rm -f "$meta"' EXIT
printf ';FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=1000\ntitle=Opening\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=1000\nEND=3000\ntitle=Second part\n' >"$meta"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=330:duration=3" -i "$meta" -map 0 -map_chapters 1 \
  -c:a aac -metadata title="Book" -metadata artist="Narrator" "$album/book.m4b"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=440:duration=3" -i "$meta" -map 0 -map_chapters 1 \
  -c:a libmp3lame -metadata title="Episode" -metadata artist="Host" "$album/podcast.mp3"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=495:duration=3" -i "$meta" -map 0 -map_chapters 1 \
  -c:a flac -metadata title="Film" -metadata artist="Crew" "$album/film.mka"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=550:duration=2" -c:a flac "$album/disc-1.flac"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=660:duration=2" -c:a flac "$album/disc-2.flac"
printf 'PERFORMER "The Duo"\r\nTITLE "Gapless"\r\nFILE "disc-1.flac" WAVE\r\n  TRACK 01 AUDIO\r\n    TITLE "First"\r\n    INDEX 01 00:00:00\r\n  TRACK 02 AUDIO\r\n    TITLE "Second"\r\n    INDEX 00 00:01:50\r\nFILE "disc-2.flac" WAVE\r\n    INDEX 01 00:00:00\r\n' >"$album/gaps.cue"
