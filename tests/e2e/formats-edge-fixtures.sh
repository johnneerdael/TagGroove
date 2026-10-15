#!/usr/bin/env bash
# Fixtures for tests/e2e/formats-edge.js, written into the album folder ($1):
# an ADTS AAC stream with an APEv2 tag, and a WavPack file with a prepended
# ID3v2.3 tag and an APEv2 tag.
set -euo pipefail
album="$1"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=440:duration=2" -c:a aac "$album/stream.aac"
ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=660:duration=2" -c:a wavpack "$album/prefixed.wv"
python3 -I - "$album/stream.aac" "$album/prefixed.wv" <<'PY'
import struct, sys
def ape(items):
    body = b"".join(struct.pack("<II", len(v.encode()), 0) + k.encode() + b"\0" + v.encode() for k, v in items)
    def block(flags):
        return b"APETAGEX" + struct.pack("<IIII", 2000, len(body) + 32, len(items), flags) + bytes(8)
    return block(0xA0000000) + body + block(0x80000000)
def id3(title):
    frame = b"\x00" + title.encode("latin-1")
    frame = b"TIT2" + struct.pack(">I", len(frame)) + b"\0\0" + frame
    size = len(frame)
    return b"ID3\x03\x00\x00" + bytes((size >> s) & 0x7F for s in (21, 14, 7, 0)) + frame
aac, wv = sys.argv[1], sys.argv[2]
open(aac, "ab").write(ape([("Title", "From APE"), ("Artist", "Band")]))
data = open(wv, "rb").read()
open(wv, "wb").write(id3("Hidden ID3v2") + data + ape([("Title", "From APE wv")]))
PY
