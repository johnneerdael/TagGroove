#!/usr/bin/env python3
"""Generate the installer RTF from the canonical UTF-8 notice text.

Run with --check before publication. Unicode is represented as signed UTF-16
RTF escapes; literal braces/backslashes cannot become document controls.
"""
import argparse
from pathlib import Path


def render(text):
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    out = [r"{\rtf1\ansi\ansicpg1252\deff0{\fonttbl{\f0\fmodern Consolas;}}\uc1\pard\f0\fs18 "]
    for character in text:
        if character in "\\{}":
            out.append("\\" + character)
        elif character == "\n":
            out.append("\\par\n")
        elif character == "\t":
            out.append("\\tab ")
        elif character == "\f":
            # Upstream COPYING documents can retain form-feed page separators.
            out.append("\\u12?")
        elif ord(character) < 32:
            raise ValueError("Notice text contains an unsupported control character")
        elif ord(character) <= 126:
            out.append(character)
        else:
            units = character.encode("utf-16-le")
            for offset in range(0, len(units), 2):
                value = int.from_bytes(units[offset:offset + 2], "little", signed=True)
                out.append(f"\\u{value}?")
    out.append("}\n")
    return "".join(out).encode("ascii")


def main():
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, default=root / "THIRD_PARTY_NOTICES.md")
    parser.add_argument("--output", type=Path, default=root / "THIRD_PARTY_NOTICES.rtf")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = render(args.input.read_text(encoding="utf-8"))
    if args.check:
        if not args.output.is_file() or args.output.read_bytes() != data:
            raise SystemExit("Installer RTF is stale; run python3 scripts/installer-license.py")
    else:
        args.output.write_bytes(data)


if __name__ == "__main__":
    main()
