#!/usr/bin/env python3
"""Reports the user guide's writing progress: pages per navigation section,
which are still drafts (front matter `status: draft`), and any navigation
entry without a file or page without a navigation entry.
Usage: python3 scripts/docs-status.py [--drafts] [--coverage]
  --drafts    list every draft page
  --coverage  the coverage map in docs/user-guide/_meta/ia.md with each
              Mp3tag topic's status computed from its pages
Reads files only; needs no MkDocs installation."""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DOCS = os.path.join(ROOT, "docs", "user-guide")
EXCLUDED = ("_meta/", "_snippets/")


def nav_entries():
    """(top-level section, path) for every page in mkdocs.yml's nav."""
    entries, section, in_nav = [], "", False
    with open(os.path.join(ROOT, "mkdocs.yml"), encoding="utf-8") as f:
        for line in f:
            if line.startswith("nav:"):
                in_nav = True
                continue
            if not in_nav or not line.strip():
                continue
            if not line.startswith(" "):
                break
            indent = len(line) - len(line.lstrip())
            match = re.match(r"\s*- (?:(.+?): )?([\w./-]+\.md)\s*$", line)
            if indent == 2:
                section = re.match(r'\s*- "?(.+?)"?:', line).group(1)
            if match:
                entries.append((section, match.group(2)))
    return entries


def is_draft(path):
    with open(os.path.join(DOCS, path), encoding="utf-8") as f:
        head = f.read(400)
    return bool(re.match(r"---\n(?:.*\n)*?status:\s*draft\s*\n(?:.*\n)*?---", head))


def coverage():
    """Status of every coverage-map row: written, partial or planned."""
    counts = {}
    with open(os.path.join(DOCS, "_meta", "ia.md"), encoding="utf-8") as f:
        for line in f:
            cells = [c.strip() for c in line.split("|")]
            if len(cells) < 5 or not cells[1].startswith("`"):
                continue
            pages = re.findall(r"\]\(\.\./([^)]+\.md)\)", cells[3])
            if not pages:
                continue
            done = [p for p in pages if os.path.exists(os.path.join(DOCS, p)) and not is_draft(p)]
            state = "written" if len(done) == len(pages) else "partial" if done else "planned"
            counts[state] = counts.get(state, 0) + 1
            print(f"{state:8} {cells[1].strip('`'):45} {len(done)}/{len(pages)} pages")
    print(", ".join(f"{n} {state}" for state, n in sorted(counts.items())))


def main():
    if "--coverage" in sys.argv:
        coverage()
        return 0
    entries = nav_entries()
    listed = {path for _, path in entries}
    missing = [path for path in listed if not os.path.exists(os.path.join(DOCS, path))]
    unlisted = []
    for folder, _, files in os.walk(DOCS):
        for name in files:
            rel = os.path.relpath(os.path.join(folder, name), DOCS).replace(os.sep, "/")
            if rel.endswith(".md") and rel not in listed and not rel.startswith(EXCLUDED):
                unlisted.append(rel)
    sections = {}
    for section, path in entries:
        done, total, drafts = sections.setdefault(section, [0, 0, []])
        sections[section][1] += 1
        if path in missing or is_draft(path):
            drafts.append(path)
        else:
            sections[section][0] += 1
    width = max(len(s) for s in sections)
    print(f"{'Section'.ljust(width)}  written / pages")
    for section, (done, total, _) in sections.items():
        print(f"{section.ljust(width)}  {done:>7} / {total}")
    written = sum(v[0] for v in sections.values())
    print(f"{'Total'.ljust(width)}  {written:>7} / {len(entries)}")
    if "--drafts" in sys.argv:
        for section, (_, _, drafts) in sections.items():
            for path in drafts:
                print(f"draft  {section}: {path}")
    for path in missing:
        print(f"ERROR  in the navigation but missing: {path}")
    for path in sorted(unlisted):
        print(f"ERROR  not in the navigation: {path}")
    return 1 if missing or unlisted else 0


if __name__ == "__main__":
    sys.exit(main())
