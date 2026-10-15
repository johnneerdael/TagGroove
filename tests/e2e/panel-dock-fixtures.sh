#!/usr/bin/env bash
# Fixtures for tests/e2e/panel-dock.js: a hand-written tag panel configuration in
# the key layout Mp3tag uses for usrfields.ini (UTF-16 with BOM), next to the
# album folder ($1). No Mp3tag file is used.
set -euo pipefail
album="$1"
{
  printf '\xff\xfe'
  printf '[#0]\r\nname=Tit&le\r\nvalue=TITLE\r\nsize=12\r\nenabled=1\r\nmultiline=0\r\ndefault=< keep >\r\nversion=1\r\n[#1]\r\nname=K&ünstler\r\nvalue=ARTIST\r\nsize=12\r\nversion=1\r\n[#2]\r\nname=\r\nvalue=_SEPARATOR\r\nversion=1\r\n[#3]\r\nname=&Year\r\nvalue=YEAR\r\nsize=6\r\nversion=1\r\n[#4]\r\nname=T&rack\r\nvalue=TRACK\r\nsize=6\r\nversion=1\r\n[#5]\r\nname=Mood\r\nvalue=MOOD\r\nsize=12\r\nmultiline=1\r\nrows=4\r\nversion=1\r\n[#6]\r\nname=Compilation\r\nvalue=COMPILATION\r\nsize=4\r\nmultiline=2\r\nenabled=0\r\nversion=1\r\n[#7]\r\nname=Path\r\nvalue=_PATH\r\nversion=1\r\n' | iconv -f UTF-8 -t UTF-16LE
} > "$album/../usrfields.ini"
