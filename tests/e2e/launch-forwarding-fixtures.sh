#!/usr/bin/env bash
# Fixtures for tests/e2e/launch-forwarding.js, written next to the album folder ($1):
# folders with spaces and '#' for Finder-style file URLs, and (Linux) a real second
# launch of the app with Mp3tag switches once the first instance owns its
# single-instance D-Bus name.
set -euo pipefail
album="$1"
work="$(dirname "$album")"
mkdir -p "$work/Second Instance" "$work/Open With" "$work/Opened Folder"
cp "$album/sine-a.mp3" "$work/Second Instance/forward-c.mp3"
cp "$album/sine-a.mp3" "$work/Open With/open-d.mp3"
cp "$album/sine-b.mp3" "$work/Open With/open #e.mp3"
cp "$album/sine-b.mp3" "$work/Opened Folder/folder-f.mp3"
root="$(cd "$(dirname "$0")/../.." && pwd)"
bin="$root/target/debug/taggroove"
name="${OPENTAG_E2E_ID:-org.opentag.editor.usertest}.SingleInstance"
if [[ "$(uname)" == "Linux" && -x "$bin" ]] && command -v dbus-send >/dev/null; then
  (
    for _ in $(seq 1 600); do
      if dbus-send --session --print-reply --dest=org.freedesktop.DBus /org/freedesktop/DBus \
        org.freedesktop.DBus.NameHasOwner "string:$name" 2>/dev/null | grep -q 'boolean true'; then
        # A relative /fn: value resolves against the second launch's working directory.
        cd "$work/Second Instance"
        env -u OPENTAG_E2E_SCRIPT timeout 60 "$bin" /add "/fn:forward-c.mp3"
        exit 0
      fi
      sleep 0.1
    done
  ) </dev/null >/dev/null 2>&1 &
fi
