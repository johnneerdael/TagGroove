#!/usr/bin/env bash
# Fixtures for tests/e2e/keyboard-ui.js, written into the album folder ($1): a
# subfolder with one MP3 (per-folder playlists, drops and the Directory field),
# a file for the permanent delete, and a hand-written Mp3tag-style columns.ini
# (UTF-16 with BOM, as Mp3tag writes it) next to the album. No Mp3tag file is used.
set -euo pipefail
album="$1"
mkdir -p "$album/sub"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=550:duration=3" \
  -metadata title="Sine Sub" -metadata artist="Sub Artist" -metadata album="Sub Album" -metadata track=1 "$album/sub/sine-sub.mp3"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=330:duration=3" \
  -metadata title="Delete Me" -metadata artist="Test Artist" -metadata album="Test Album" -metadata track=9 "$album/zz-delete.mp3"
{
  printf '\xff\xfe'
  printf '[#0]\r\nName=File\r\nValue=%%_filename_ext%%\r\nField=%%_filename_ext%%\r\nWidth=240\r\nPos=0\r\n[#1]\r\nName=Künstler\r\nValue=%%artist%%\r\nField=ARTIST\r\nWidth=150\r\nPos=1\r\n[#2]\r\nName=Track\r\nValue=$num(%%track%%,2)\r\nSort=%%track%%\r\nAlign=1\r\nNum=1\r\nShow=0\r\nPos=2\r\n' | iconv -f UTF-8 -t UTF-16LE
} > "$album/../mp3tag-columns.ini"
