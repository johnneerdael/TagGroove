#!/usr/bin/env bash
# Drives the real desktop app (debug build) through scripted UI workflows
# against generated fixture files. macOS/Linux; requires ffmpeg.
# Usage: scripts/e2e-native.sh [tests/e2e/<script>.js] [--no-build]
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
script="${1:-$root/tests/e2e/core-workflows.js}"
# Parallel runs need distinct identifiers: single-instance forwarding and app
# config directories are keyed by it.
identifier="${OPENTAG_E2E_ID:-org.opentag.editor.usertest}"
if [[ "${2:-}" != "--no-build" ]]; then
  (cd "$root" && AR="${AR:-/usr/bin/ar}" npx tauri build --debug --bundles app \
    --config "{\"identifier\":\"$identifier\",\"productName\":\"OpenTag UserTest\"}" >/dev/null)
fi
if [[ "$(uname)" == "Darwin" ]]; then
  bin="$root/target/debug/bundle/macos/OpenTag UserTest.app/Contents/MacOS/opentag"
else
  bin="$root/target/debug/opentag"
fi
work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
album="$work/album"; mkdir -p "$album"
for spec in "a:440:1" "b:660:2"; do
  IFS=: read -r name freq track <<<"$spec"
  upper="$(printf '%s' "$name" | tr '[:lower:]' '[:upper:]')"
  ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=$freq:duration=3" \
    -metadata title="Sine $upper" -metadata artist="Test Artist" -metadata album="Test Album" \
    -metadata track="$track" "$album/sine-$name.mp3"
done
output="$(OPENTAG_E2E_SCRIPT="$script" timeout 180 "$bin" "/fp:$album" 2>&1 | grep '^\[e2e\]' || true)"
printf '%s\n' "$output"
grep -q '^\[e2e\] SUMMARY' <<<"$output" || { echo "e2e run did not finish" >&2; exit 1; }
! grep -q '^\[e2e\] FAIL\|^\[e2e\] FATAL' <<<"$output"
