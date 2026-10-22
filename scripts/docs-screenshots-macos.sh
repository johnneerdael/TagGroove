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
# --bundles app, from main), Swift (to find the window), ImageMagick (to crop) and Screen Recording permission for the
# terminal that runs this. Quit TagGroove first: it is a single-instance app.
#
# Environment:
#   TAGGROOVE_APP           the .app to drive (default /Applications/TagGroove.app)
#   TAGGROOVE_MUSIC_SOURCE  a folder holding albums/ and songs/ of real music (default ~/Music).
#                           Scenes never touch it: the run works on copies (APFS clones, so no
#                           extra disk) under TAGGROOVE_MUSIC_WORK, which is removed afterwards.
#   TAGGROOVE_MUSIC_WORK    where the copies live (default /Users/Shared/Music, a neutral path
#                           that does not show a user name in screenshots)
#   TAGGROOVE_FIRST_ALBUM   the album shown at start (default albums/R.E.M. - Automatic For The People)
set -euo pipefail

if [[ "$(uname -s)" != Darwin || "${TAGGROOVE_DOCS_CAPTURE_LOCAL:-}" != 1 ]]; then
  echo "This opens real app windows on this Mac. Set TAGGROOVE_DOCS_CAPTURE_LOCAL=1 to run it (macOS only)." >&2
  exit 2
fi
root="$(cd "$(dirname "$0")/.." && pwd)"
app="${TAGGROOVE_APP:-/Applications/TagGroove.app}"
bin="$app/Contents/MacOS/taggroove"
source="${TAGGROOVE_MUSIC_SOURCE:-$HOME/Music}"
music="${TAGGROOVE_MUSIC_WORK:-/Users/Shared/Music}"
first="${TAGGROOVE_FIRST_ALBUM:-albums/R.E.M. - Automatic For The People}"
[[ -x "$bin" ]] || { echo "No app binary at $bin" >&2; exit 1; }
for tool in swiftc magick screencapture; do
  command -v "$tool" >/dev/null || { echo "Missing tool: $tool" >&2; exit 1; }
done
if pgrep -x taggroove >/dev/null; then
  echo "TagGroove is already running. Quit it first; a second launch would only forward to the running window." >&2
  exit 1
fi

out="$(mkdir -p "${1:?output directory}" && cd "$1" && pwd)"; shift
work="$(mktemp -d)"
# Remove only what this run created, quit the app and put its saved state back as it was.
pid=""
state=()
made_folders=()
cleanup() {
  if [[ -n "${pid:-}" ]] && kill -0 "$pid" 2>/dev/null; then
    osascript -e 'tell application "TagGroove" to quit' >/dev/null 2>&1 || true
    for _ in $(seq 1 25); do kill -0 "$pid" 2>/dev/null || break; sleep 0.2; done
    kill "$pid" 2>/dev/null || true
  fi
  if [[ -d "$work/state" ]]; then
    for i in "${!state[@]}"; do
      if [[ -d "$work/state/$i" ]]; then
        if ! diff -rq "$work/state/$i" "${state[$i]}" >/dev/null 2>&1; then echo "Restoring app state: ${state[$i]}" >&2; fi
        rsync -a --delete "$work/state/$i/" "${state[$i]}/"
      fi
    done
  fi
  for folder in ${made_folders[@]+"${made_folders[@]}"}; do rm -rf "$folder"; done
  if [[ -n "${made_music:-}" ]]; then rmdir "$music" 2>/dev/null || true; fi
  rm -rf "$work"
}
trap cleanup EXIT

# The window finder: the largest on-screen window owned by a process id.
cat >"$work/windowid.swift" <<'SWIFT'
import CoreGraphics
import Foundation
let pid = Int32(CommandLine.arguments[1])!
let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
var best = 0, bestArea = 0.0, count = 0
for w in windows where (w[kCGWindowOwnerPID as String] as? Int32) == pid {
  count += 1
  guard (w[kCGWindowLayer as String] as? Int) == 0 else { continue }
  let b = w[kCGWindowBounds as String] as? [String: Double] ?? [:]
  let area = (b["Width"] ?? 0) * (b["Height"] ?? 0)
  if area > bestArea, let id = w[kCGWindowNumber as String] as? Int { best = id; bestArea = area }
}
print("\(best) \(count)")
SWIFT
swiftc -O "$work/windowid.swift" -o "$work/windowid" 2>/dev/null

# Work on copies of the real music so a scene can never change an original.
[[ -d "$source/albums" ]] || { echo "No albums folder in $source (set TAGGROOVE_MUSIC_SOURCE)" >&2; exit 1; }
[[ ! -e "$music" ]] && made_music="$music"
mkdir -p "$music"
for folder in albums songs; do
  [[ -d "$source/$folder" ]] || continue
  [[ -e "$music/$folder" ]] && { echo "$music/$folder already exists; set TAGGROOVE_MUSIC_WORK elsewhere" >&2; exit 1; }
  cp -cR "$source/$folder" "$music/$folder"
  made_folders+=("$music/$folder")
done
album="$music/$first"
[[ -e "$album" ]] || { echo "No such first album: $album" >&2; exit 1; }

# Scene names given on the command line restrict the run to those scenes.
scenes="$(node -e 'process.stdout.write(JSON.stringify(process.argv.slice(1)))' -- "$@")"
{ printf 'window.OPENTAG_DOCS_MUSIC = %s;\n' "$(node -e 'process.stdout.write(JSON.stringify(process.argv[1]))' -- "$music")"
  # What the working copy holds, so a scene can pick an album or song by a pattern (genre-appropriate material).
  printf 'window.OPENTAG_DOCS_LIBRARY = %s;\n' "$(node -e 'const fs=require("fs"),d=process.argv[1];const l=k=>{try{return fs.readdirSync(d+"/"+k).filter(n=>!n.startsWith("."))}catch{return[]}};process.stdout.write(JSON.stringify({albums:l("albums"),songs:l("songs")}))' -- "$music")"
  printf 'window.OPENTAG_DOCS_FIRST = %s;\n' "$(node -e 'process.stdout.write(JSON.stringify(process.argv[1]))' -- "$first")"
  # TAGGROOVE_DOCS_SKIP="scene-a scene-b" leaves scenes out (for example providers that are down).
  printf 'window.OPENTAG_DOCS_SKIP = %s;\n' "$(node -e 'process.stdout.write(JSON.stringify((process.argv[1]||"").split(/\s+/).filter(Boolean)))' -- "${TAGGROOVE_DOCS_SKIP:-}")"
  printf 'window.OPENTAG_DOCS_SCENES = %s;\n' "$scenes"
  for file in "$root"/tests/docs-screens/scenes/*.js; do [[ -e "$file" ]] && cat "$file"; done
  cat "$root/tests/docs-screens/screens.js"; } >"$work/screens.js"

# What the run may touch: the app keeps settings, its browser storage (colour mode, restored
# list) and undo journal under these folders. Snapshot them now; the cleanup trap puts them back
# even when the run is interrupted.
state=("$HOME/Library/Application Support/org.opentag.editor" "$HOME/Library/WebKit/org.opentag.editor" "$HOME/Library/Caches/org.opentag.editor")
mkdir -p "$work/state"
for i in "${!state[@]}"; do [[ -d "${state[$i]}" ]] && cp -R "${state[$i]}" "$work/state/$i"; done

# Turn off the "unsaved changes" confirmation for the run: it is a native sheet that blocks the page.
# The settings folder is restored from the snapshot above when the run ends.
python3 - "${state[0]}/settings.json" <<'PY'
import json, os, sys
path = sys.argv[1]
if os.path.exists(path):
    settings = json.load(open(path))
    settings.setdefault("pages", {}).setdefault("messages", {})["unsaved_changes"] = False
    json.dump(settings, open(path, "w"), indent=2)
PY

log="$work/app.log"; : >"$log"; : >"$work/crop"
# `open -g` starts the app without bringing it to the front, so the desktop stays usable.
open -g -n -a "$app" --env "OPENTAG_E2E_SCRIPT=$work/screens.js" --env LANG=en_US.UTF-8 \
  --stdout "$log" --stderr "$log" --args "/fp:$album"
pid=""
for _ in $(seq 1 100); do pid="$(pgrep -x taggroove | head -1 || true)"; [[ -n "$pid" ]] && break; sleep 0.2; done
[[ -n "$pid" ]] || { echo "TagGroove did not start" >&2; exit 1; }

baseline=""
capture() {  # $1 = scene-shot name
  local name="$1" file="$out/$1.png" id crop count
  read -r id count < <("$work/windowid" "$pid")
  if [[ "$id" == 0 ]]; then echo "no window for $name" >&2; return 1; fi
  # More windows than at the first capture means a native sheet or panel opened (a save panel,
  # a confirmation). It overlaps every later capture and cannot be closed from the page: stop.
  [[ -z "$baseline" ]] && baseline="$count"
  if (( count > baseline )); then
    echo "NATIVE WINDOW DETECTED before $name ($count app windows, expected $baseline): stopping" >&2
    rm -f "$file"; touch "$work/abort"; return 1
  fi
  screencapture -x -o -l "$id" "$file"
  crop="$(cat "$work/crop")"; : >"$work/crop"
  if [[ -n "$crop" ]]; then
    local w h
    w="$(magick identify -format %w "$file")"; h="$(magick identify -format %h "$file")"
    # CROP x,y,w,h,innerW,innerH in CSS pixels; the window image also holds the title bar.
    IFS=, read -r cx cy cw ch iw ih <<<"$crop"
    # A dialog can settle a few pixels after its rectangle is measured and has rounded corners; an inset keeps
    # the backdrop out of the crop (the guide's figure style draws its own border and radius).
    local scale margin=-3
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
        "[e2e] SHOT "*) capture "${line#"[e2e] SHOT "}" || true; [[ -e "$work/abort" ]] && break ;;
        "[e2e] SUMMARY"*) echo "$line"; [[ "$line" == *" 0 failed" ]] && status=0 ;;
        "[e2e] FAIL"*|"[e2e] FATAL"*|"[e2e] LOG"*) echo "$line" ;;
      esac
    done < <(sed -n "$((seen + 1)),${total}p" "$log")
    seen=$total
    [[ -e "$work/abort" ]] && break
  fi
  sleep 0.15
done
if [[ -e "$work/abort" ]]; then status=3; fi
wait "$pid" 2>/dev/null || true

ls -1 "$out"
exit "$status"
