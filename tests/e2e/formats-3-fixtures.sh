#!/usr/bin/env bash
# Fixtures for tests/e2e/formats-3.js, written into the album folder ($1):
# an M4B and an MKA with two chapters each, an ffmpeg TTA and a TAK header
# stream (no TAK encoder exists for macOS; the stream is never decoded).
set -euo pipefail
album="$1"
meta="$(mktemp)"; trap 'rm -f "$meta"' EXIT
printf ';FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=1000\ntitle=Opening\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=1000\nEND=3000\ntitle=Second part\n' >"$meta"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=330:duration=3" -i "$meta" -map 0 -map_chapters 1 \
  -c:a aac -metadata title="Book" "$album/chapters.m4b"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=550:duration=3" -i "$meta" -map 0 -map_chapters 1 \
  -c:a flac -metadata title="Matroska" "$album/chapters.mka"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=770:duration=2" -c:a tta -metadata title="True Audio" "$album/stream.tta"
# Windows runners name the interpreter python; elsewhere python3.
for python in python3 python; do "$python" -c '' 2>/dev/null && break; done
"$python" -I - "$album/header.tak" <<'PY'
import sys
def crc24(data):
    crc = 0xB704CE
    for byte in data:
        crc ^= byte << 16
        for _ in range(8):
            crc <<= 1
            if crc & 0x1000000:
                crc ^= 0x864CFB
    return crc & 0xFFFFFF
value, at = 0, 0
for field, width in [(2, 6), (0, 4), (0, 4), (88200, 35), (0, 3), (44100 - 6000, 18), (16 - 8, 5), (1, 4), (0, 1)]:
    value |= field << at
    at += width
info = value.to_bytes(16, "little")[:10]
data = b"tBaK" + bytes([1]) + (len(info) + 3).to_bytes(3, "little") + info + crc24(info).to_bytes(3, "little") + bytes(4)
data += bytes((i * 37 % 89 + 1) for i in range(20000))
open(sys.argv[1], "wb").write(data)
PY
