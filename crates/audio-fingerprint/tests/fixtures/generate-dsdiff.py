"""Reproduce the project-authored DSDIFF carrier; uses no media tools."""

import gzip
import hashlib
from pathlib import Path


def chunk(kind: bytes, data: bytes) -> bytes:
    return kind + len(data).to_bytes(8, "big") + data + bytes(len(data) % 2)


rate = 2_822_400
seconds = 12
properties = (
    b"SND "
    + chunk(b"FS  ", rate.to_bytes(4, "big"))
    + chunk(b"CHNL", (2).to_bytes(2, "big") + b"SLFTSRGT")
    + chunk(b"CMPR", b"DSD " + bytes([14]) + b"not compressed")
)
body = (
    b"DSD "
    + chunk(b"FVER", bytes([1, 5, 0, 0]))
    + chunk(b"PROP", properties)
    + chunk(b"DSD ", b"\x69\x69" * (rate * seconds // 8))
)
raw = b"FRM8" + len(body).to_bytes(8, "big") + body
carrier = bytearray(gzip.compress(raw, compresslevel=9, mtime=0))
# Pin the gzip OS header to "unknown", including on Python 3.11/3.12.
carrier[9] = 255
assert len(raw) == 8_467_330
assert hashlib.sha256(raw).hexdigest() == (
    "7f1992e6d162c53d6493595d8924ed6935dfb460397d782f284b3d4341089ee5"
)
assert hashlib.sha256(carrier).hexdigest() == (
    "7d9c7dea40f7d658fdc5b9864c3ab73eb87f0cc7d29da556384b73e444394630"
)
Path(__file__).with_name("dsdiff12.dff.gz").write_bytes(carrier)
