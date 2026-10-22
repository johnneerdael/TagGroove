#!/usr/bin/env python3
"""Insert user-guide screenshots into pages from tests/docs-screens/figures.json.

Each manifest entry names a scene, a page, where to place the figure and its
alt text and caption:

  {"scene": "find-dialog", "page": "tagging/find.md",
   "after": "## Choose a matching strategy",      # or "intro" (after the first paragraph)
   "alt": "...", "caption": "..."}

A figure is only written when both docs/user-guide/assets/screens/<scene>-light.png
and -dark.png exist, so the manifest can list scenes that are not captured yet
and the strict build stays green. Figures are fenced by
`<!-- figure:<scene> -->` ... `<!-- /figure -->` markers, so running this again
updates them in place instead of duplicating them.

Usage: python3 scripts/docs-screens-embed.py [--check]
  --check   report missing images and unknown pages, change nothing
"""
import html
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DOCS = os.path.join(ROOT, "docs", "user-guide")
SCREENS = os.path.join(DOCS, "assets", "screens")
MANIFEST = os.path.join(ROOT, "tests", "docs-screens", "figures.json")
BLOCK_START = re.compile(r"^(\s*)(```|~~~|\||[-*+] |\d+\. |!!! |\?\?\? |<|\{)")


def figure(entry, page, indent=""):
    rel = os.path.relpath(SCREENS, os.path.dirname(os.path.join(DOCS, page))).replace(os.sep, "/")
    alt = entry["alt"].replace("[", "(").replace("]", ")")
    scene = entry["scene"]
    lines = [
        f"<!-- figure:{scene} -->",
        "<figure markdown>",
        f"![{alt}]({rel}/{scene}-light.png#only-light)",
        f"![{alt}]({rel}/{scene}-dark.png#only-dark)",
    ]
    if entry.get("caption"):
        lines.append(f"<figcaption>{html.escape(entry['caption'], quote=False)}</figcaption>")
    lines += ["</figure>", "<!-- /figure -->"]
    return "\n".join(indent + line if line else line for line in lines)


def insertion_point(lines, after):
    """Line index at which to insert, after a heading's first paragraph."""
    if after == "intro":
        start = next((i for i, l in enumerate(lines) if l.startswith("# ")), -1)
    else:
        start = next((i for i, l in enumerate(lines) if l.strip() == after), -1)
    if start < 0:
        raise KeyError(after)
    i = start + 1
    while i < len(lines) and not lines[i].strip():
        i += 1
    if i >= len(lines) or BLOCK_START.match(lines[i]) or lines[i].startswith("#"):
        return start + 1  # next block is not prose: place the figure right after the heading
    while i < len(lines) and lines[i].strip():  # skip the whole paragraph
        i += 1
    return i


def main():
    check = "--check" in sys.argv
    with open(MANIFEST, encoding="utf-8") as f:
        entries = json.load(f)
    by_page = {}
    problems = []
    for entry in entries:
        scene = entry["scene"]
        missing = [m for m in ("light", "dark") if not os.path.exists(os.path.join(SCREENS, f"{scene}-{m}.png"))]
        if missing:
            problems.append(f"no image yet: {scene} ({', '.join(missing)})")
            continue
        if not os.path.exists(os.path.join(DOCS, entry["page"])):
            problems.append(f"unknown page for {scene}: {entry['page']}")
            continue
        by_page.setdefault(entry["page"], []).append(entry)
    written = 0
    for page, items in by_page.items():
        path = os.path.join(DOCS, page)
        with open(path, encoding="utf-8") as f:
            text = f.read()
        for entry in items:
            marker = re.compile(rf"<!-- figure:{re.escape(entry['scene'])} -->.*?<!-- /figure -->", re.S)
            block = figure(entry, page)
            if marker.search(text):
                text = marker.sub(lambda m: block, text, count=1)
                continue
            lines = text.split("\n")
            try:
                at = insertion_point(lines, entry.get("after", "intro"))
            except KeyError as error:
                problems.append(f"{page}: heading not found for {entry['scene']}: {error}")
                continue
            tail = [""] if at < len(lines) and lines[at].strip() else []
            lines[at:at] = ["", block] + tail
            text = "\n".join(lines)
            written += 1
        if not check:
            with open(path, "w", encoding="utf-8") as f:
                f.write(text)
    for line in problems:
        print(line)
    print(f"{written} figures inserted; {len(entries) - sum(len(v) for v in by_page.values())} manifest entries waiting for images or fixes")
    return 1 if any(p.startswith(("unknown", "heading")) or ": heading" in p for p in problems) else 0


if __name__ == "__main__":
    sys.exit(main())
