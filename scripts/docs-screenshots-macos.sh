#!/usr/bin/env bash
# Owner-approved LOCAL capture of the user guide's screenshots on macOS.
#
# The reviewed CI job (scripts/docs-screenshots.sh) remains the default way to
# capture on Linux. This runner exists for the macOS look and for scenes that
# need a configured desktop (online tag sources with the owner's keys). It
# launches a debug build of TagGroove with the end-to-end hook, drives it
# through the named scenes in tests/docs-screens (the same scene files the CI
# runner uses) and saves one PNG per scene and colour mode.
#
# Usage:
#   TAGGROOVE_DOCS_CAPTURE_LOCAL=1 scripts/docs-screenshots-macos.sh <output-dir> [scene ...]
#
# Needs: macOS, a debug build with the e2e hook (npx tauri build --debug
# --bundles app, from main), ffmpeg (for the fixture album), Swift (to find the
# window), ImageMagick (to crop) and Screen Recording permission for the
# terminal that runs this. Quit TagGroove first: it is a single-instance app.
#
# Environment:
#   TAGGROOVE_APP     the .app to drive (default /Applications/TagGroove.app)
#   TAGGROOVE_ALBUM   folder for the fixture album (default /Users/Shared/Music/Juniper Vale/Harbour Lights,
#                     a neutral path that does not show a user name)
#   TAGGROOVE_ALBUM_FIXTURE  a script that fills the folder (default tests/docs-screens/fixtures.sh)
set -euo pipefail

if [[ "$(uname -s)" != Darwin || "${TAGGROOVE_DOCS_CAPTURE_LOCAL:-}" != 1 ]]; then
  echo "This opens real app windows on this Mac. Set TAGGROOVE_DOCS_CAPTURE_LOCAL=1 to run it (macOS only)." >&2
  exit 2
fi
root="$(cd "$(dirname "$0")/.." && pwd)"
app="${TAGGROOVE_APP:-/Applications/TagGroove.app}"
bin="$app/Contents/MacOS/taggroove"
album="${TAGGROOVE_ALBUM:-/Users/Shared/Music/Juniper Vale/Harbour Lights}"
fixture="${TAGGROOVE_ALBUM_FIXTURE:-$root/tests/docs-screens/fixtures.sh}"
[[ -x "$bin" ]] || { echo "No app binary at $bin" >&2; exit 1; }
for tool in ffmpeg swiftc magick screencapture; do
  command -v "$tool" >/dev/null || { echo "Missing tool: $tool" >&2; exit 1; }
done
if pgrep -x taggroove >/dev/null; then
  echo "TagGroove is already running. Quit it first; a second launch would only forward to the running window." >&2
  exit 1
fi

out="$(mkdir -p "${1:?output directory}" && cd "$1" && pwd)"; shift
work="$(mktemp -d)"
# Remove only what this run created: the fixture album, then its parent if that is now empty.
cleanup() {
  rm -rf "$work"
  if [[ -n "${made_album:-}" ]]; then rm -rf "$made_album"; rmdir "$(dirname "$made_album")" 2>/dev/null || true; fi
}
trap cleanup EXIT

# The window finder: the largest on-screen window owned by a process id.
cat >"$work/windowid.swift" <<'SWIFT'
import CoreGraphics
import Foundation
let pid = Int32(CommandLine.arguments[1])!
let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
var best = 0, bestArea = 0.0
for w in windows where (w[kCGWindowOwnerPID as String] as? Int32) == pid && (w[kCGWindowLayer as String] as? Int) == 0 {
  let b = w[kCGWindowBounds as String] as? [String: Double] ?? [:]
  let area = (b["Width"] ?? 0) * (b["Height"] ?? 0)
  if area > bestArea, let id = w[kCGWindowNumber as String] as? Int { best = id; bestArea = area }
}
print(best)
SWIFT
swiftc -O "$work/windowid.swift" -o "$work/windowid" 2>/dev/null

# The fixture album, written where a reader's would sit.
if [[ ! -e "$album" ]]; then made_album="$album"; fi
"$fixture" "$album"

# Scene names given on the command line restrict the run to those scenes.
scenes="$(node -e 'process.stdout.write(JSON.stringify(process.argv.slice(1)))' -- "$@")"
{ printf 'window.OPENTAG_DOCS_SCENES = %s;\n' "$scenes"
  for file in "$root"/tests/docs-screens/scenes/*.js; do [[ -e "$file" ]] && cat "$file"; done
  cat "$root/tests/docs-screens/screens.js"; } >"$work/screens.js"

# What the run may touch: remember the app's saved settings to report changes.
config="$HOME/Library/Application Support/org.opentag.editor"
[[ -d "$config" ]] && cp -R "$config" "$work/config-before"

log="$work/app.log"; : >"$log"; : >"$work/crop"
OPENTAG_E2E_SCRIPT="$work/screens.js" LANG=en_US.UTF-8 "$bin" "/fp:$album" >"$log" 2>&1 &
pid=$!

capture() {  # $1 = scene-shot name
  local name="$1" file="$out/$1.png" id crop
  id="$("$work/windowid" "$pid")"
  if [[ "$id" == 0 ]]; then echo "no window for $name" >&2; return 1; fi
  screencapture -x -o -l "$id" "$file"
  crop="$(cat "$work/crop")"; : >"$work/crop"
  if [[ -n "$crop" ]]; then
    local w h
    w="$(magick identify -format %w "$file")"; h="$(magick identify -format %h "$file")"
    # CROP x,y,w,h,innerW,innerH in CSS pixels; the window image also holds the title bar.
    IFS=, read -r cx cy cw ch iw ih <<<"$crop"
    local scale margin=14
    scale="$(awk -v w="$w" -v iw="$iw" 'BEGIN{printf "%.4f", w/iw}')"
    read -r gx gy gw gh < <(awk -v s="$scale" -v h="$h" -v ih="$ih" -v x="$cx" -v y="$cy" -v cw="$cw" -v ch="$ch" -v m="$margin" 'BEGIN{
      top=h-ih*s; x0=(x-m)*s; y0=top+(y-m)*s; if(x0<0)x0=0; if(y0<top)y0=top;
      printf "%d %d %d %d", x0, y0, (cw+2*m)*s, (ch+2*m)*s }')
    magick "$file" -crop "${gw}x${gh}+${gx}+${gy}" +repage "$file"
  fi
  echo "captured $name ($(magick identify -format '%wx%h' "$file"))"
}

seen=0; status=1
while kill -0 "$pid" 2>/dev/null; do
  total="$(wc -l <"$log" | tr -d ' ')"
  if (( total > seen )); then
    while IFS= read -r line; do
      case "$line" in
        "[e2e] CROP "*) printf '%s' "${line#"[e2e] CROP "}" >"$work/crop" ;;
        "[e2e] SHOT "*) capture "${line#"[e2e] SHOT "}" || true ;;
        "[e2e] SUMMARY"*) echo "$line"; [[ "$line" == *" 0 failed" ]] && status=0 ;;
        "[e2e] FAIL"*|"[e2e] FATAL"*) echo "$line" ;;
      esac
    done < <(sed -n "$((seen + 1)),${total}p" "$log")
    seen=$total
  fi
  sleep 0.15
done
wait "$pid" 2>/dev/null || true

if [[ -d "$work/config-before" ]]; then
  if ! diff -rq "$work/config-before" "$config" >"$work/config.diff"; then
    echo "Saved settings changed during the run (review; restore if unintended):" >&2
    cat "$work/config.diff" >&2
  fi
fi
ls -1 "$out"
exit "$status"
