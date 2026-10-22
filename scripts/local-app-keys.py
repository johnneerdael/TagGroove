#!/usr/bin/env python3
"""Run a command with TagGroove's application keys from local credential files.

Release builds compile the online-service keys in from `OPENTAG_*` variables that CI takes from
repository secrets. A local debug build needs the same variables. The credential files sit in the
repository root, are ignored by git and are never printed:

    .acoustid   the AcoustID application key on its own
    .fanart     the fanart.tv project key on its own
    .discogs    "Consumer Key <tab> ..." and "Consumer Secret <tab> ..." (Discogs application page)
    .lastfm     "API key <tab> ..." and "Shared secret <tab> ..." (Last.fm application page)

usage: local-app-keys.py [--dir DIRECTORY] --check
       local-app-keys.py [--dir DIRECTORY] -- COMMAND [ARGUMENT ...]

`--check` lists which variables the files provide (names only). The second form runs COMMAND with
those variables added to its environment. A missing file only leaves its variables unset: the build
then has no built-in key for that service.
"""
import argparse
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# (file, label in the file or None for a bare value, variable)
SOURCES = [
    (".acoustid", None, "OPENTAG_ACOUSTID_API_KEY"),
    (".fanart", None, "OPENTAG_FANARTTV_API_KEY"),
    (".discogs", "Consumer Key", "OPENTAG_DISCOGS_CONSUMER_KEY"),
    (".discogs", "Consumer Secret", "OPENTAG_DISCOGS_CONSUMER_SECRET"),
    (".lastfm", "API key", "OPENTAG_LASTFM_API_KEY"),
    (".lastfm", "Shared secret", "OPENTAG_LASTFM_SHARED_SECRET"),
]
TOKEN = re.compile(r"[A-Za-z0-9_\-]{8,}")


def read_value(path, label):
    """The key stored in `path`: the first token after `label`, or the whole first line."""
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError:
        return None
    for line in lines:
        line = line.strip()
        if not line:
            continue
        if label is None:
            return line if TOKEN.fullmatch(line) else None
        head, _, rest = line.partition("\t")
        if not rest:
            match = re.match(rf"{re.escape(label)}\s*[:=]?\s+(\S+)", line, re.I)
            if match and TOKEN.fullmatch(match[1]):
                return match[1]
            continue
        if head.strip().lower() == label.lower():
            value = rest.strip().split()[0] if rest.strip() else ""
            return value if TOKEN.fullmatch(value) else None
    return None


def load(directory):
    keys = {}
    for name, label, variable in SOURCES:
        value = read_value(Path(directory) / name, label)
        if value:
            keys[variable] = value
    return keys


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--dir", default=str(ROOT), help="directory holding the credential files")
    parser.add_argument("--check", action="store_true", help="list the variables the files provide")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    keys = load(args.dir)
    if args.check:
        for _, _, variable in SOURCES:
            print(f"{variable}: {'set' if variable in keys else 'missing'}")
        return 0
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("give --check, or -- followed by a command")
    os.execvpe(command[0], command, {**os.environ, **keys})


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
