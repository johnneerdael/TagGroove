#!/usr/bin/env bash
# Captures the user guide's app screenshots: launches the debug app under xvfb
# with the fixtures from tests/docs-screens/fixtures.sh, lets
# tests/docs-screens/screens.js drive it through named scenes, and saves one PNG
# per scene and color mode into the output folder.
# Usage (CI only): scripts/docs-screenshots.sh <output-dir> [scene ...]
# Needs a debug build (target/debug/opentag), ffmpeg, xvfb-run, xdotool and
# ImageMagick. Real app windows open here, so it refuses to run outside CI and
# outside Linux. This dormant runner has no active workflow caller; new
# capture must be routed through the review gate before CI execution.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
if [[ -z "${CI:-}" || "$(uname -s)" != Linux ]]; then
  echo "Refusing to open app windows here. New docs capture is paused pending review-gated CI support; see docs/PUBLISHING.md." >&2
  exit 2
fi
bin="$root/target/debug/opentag"
scale="${OPENTAG_SHOTS_SCALE:-2}"

# Saves the app window (the largest visible window titled OpenTag; GTK also
# creates helper windows) as a PNG.
capture() {
  local file="$1" id size best=0 window=root
  for id in $(xdotool search --onlyvisible --name 'OpenTag' 2>/dev/null); do
    size="$(xwininfo -id "$id" | awk '/Width:/{w=$2}/Height:/{h=$2}END{print w*h}')"
    if (( size > best )); then best=$size; window=$id; fi
  done
  import -window "$window" "$file"
  if command -v pngquant >/dev/null; then pngquant --force --skip-if-larger --quality=80-98 --output "$file" "$file" || true; fi
  echo "captured $(basename "$file") ($(identify -format '%wx%h' "$file"))"
}

# Second stage, inside xvfb: run the app and capture on each SHOT request.
if [[ "${1:-}" == --under-xvfb ]]; then
  out="$2" work="$3" album="$4" log="$3/app.log" status=1
  # WebKitGTK under xvfb has no GPU; GDK_SCALE renders at HiDPI for crisp images.
  WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1 GDK_SCALE="$scale" \
    LANG=en_US.UTF-8 LANGUAGE=en OPENTAG_E2E_OFFLINE=1 OPENTAG_E2E_SCRIPT="$work/screens.js" \
    timeout 600 "$bin" "/fp:$album" >"$log" 2>&1 &
  pid=$!
  while IFS= read -r line; do
    case "$line" in
      "[e2e] SHOT "*) capture "$out/${line#"[e2e] SHOT "}.png" ;;
      "[e2e] SUMMARY"*) echo "$line"; [[ "$line" == *" 0 failed" ]] && status=0 ;;
      "[e2e] "*) echo "$line" ;;
    esac
  done < <(tail -n +1 -f --pid="$pid" "$log")
  wait "$pid" || true
  if (( status != 0 )); then echo "--- app output ---" >&2; tail -n 80 "$log" >&2; fi
  exit "$status"
fi

out="$(mkdir -p "${1:?output directory}" && cd "$1" && pwd)"; shift
[[ -x "$bin" ]] || { echo "No debug build at $bin" >&2; exit 1; }
# The same test identifier as the native suites; its state is cleared first.
identifier=org.opentag.editor.usertest
rm -rf "${XDG_CONFIG_HOME:-$HOME/.config}/$identifier" "${XDG_DATA_HOME:-$HOME/.local/share}/$identifier"
# The album sits where a reader's would, so the location bar reads naturally.
work="$(mktemp -d)" library="$HOME/Music/Juniper Vale"; trap 'rm -rf "$work" "$library"' EXIT
album="$library/Harbour Lights"
"$root/tests/docs-screens/fixtures.sh" "$album"
# Scene names given on the command line restrict the run to those scenes.
scenes="$(node -e 'process.stdout.write(JSON.stringify(process.argv.slice(1)))' -- "$@")"
# Scene files (one per workstream) register themselves before the driver runs.
{ printf 'window.OPENTAG_DOCS_SCENES = %s;\n' "$scenes"
  for file in "$root"/tests/docs-screens/scenes/*.js; do [[ -e "$file" ]] && cat "$file"; done
  cat "$root/tests/docs-screens/screens.js"; } >"$work/screens.js"
# A display sized for the HiDPI window (1240x800 logical) plus room to spare;
# the session bus serves single-instance forwarding as in the native suites.
dbus-run-session -- xvfb-run -a -s "-screen 0 $((1300 * scale))x$((860 * scale))x24" \
  "$0" --under-xvfb "$out" "$work" "$album"
ls -1 "$out"
