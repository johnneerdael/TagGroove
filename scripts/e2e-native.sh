#!/usr/bin/env bash
# Drives the real desktop app (debug build) through scripted UI workflows
# against generated fixture files. macOS/Linux; requires ffmpeg.
# Usage: scripts/e2e-native.sh [tests/e2e/<script>.js] [--no-build]
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
# Native runs open real app windows. They belong in CI (headless, xvfb-run on
# Linux), never on someone's working desktop unless explicitly allowed.
if [[ -z "${CI:-}" && -z "${OPENTAG_E2E_ALLOW_LOCAL:-}" ]]; then
  echo "Refusing to open test windows on this desktop. Native e2e runs in CI; set OPENTAG_E2E_ALLOW_LOCAL=1 to override." >&2
  exit 2
fi
script="${1:-$root/tests/e2e/core-workflows.js}"
# Parallel runs need distinct identifiers: single-instance forwarding and app
# config directories are keyed by it.
identifier="${OPENTAG_E2E_ID:-org.opentag.editor.usertest}"
# Concurrent runs cover each other's windows; WebKit would otherwise suspend
# timers in an occluded test window (macOS 14+ honours this policy).
window='"app":{"windows":[{"title":"OpenTag","width":1240,"height":800,"minWidth":800,"minHeight":560,"backgroundThrottling":"disabled"}]}'
if [[ "${2:-}" != "--no-build" ]]; then
  if [[ "$(uname)" == "Darwin" ]]; then bundle=(--bundles app); else bundle=(--no-bundle); fi
  (cd "$root" && AR="${AR:-/usr/bin/ar}" npx tauri build --debug "${bundle[@]}" \
    --config "{\"identifier\":\"$identifier\",\"productName\":\"OpenTag UserTest\",$window}" >/dev/null)
fi
if [[ "$(uname)" == "Darwin" ]]; then
  bin="$root/target/debug/bundle/macos/OpenTag UserTest.app/Contents/MacOS/opentag"
else
  bin="$root/target/debug/opentag"
fi
# --no-build may reuse a bundle built for another identifier; trust the bundle.
if [[ "$(uname)" == "Darwin" ]]; then
  identifier="$(/usr/libexec/PlistBuddy -c 'Print CFBundleIdentifier' "${bin%/MacOS/opentag}/Info.plist")"
fi
# Start every run from empty app state; only test identifiers are ever cleared.
case "$identifier" in
  *usertest*|*.e2e.*)
    if [[ "$(uname)" == "Darwin" ]]; then
      rm -rf "$HOME/Library/Application Support/$identifier" "$HOME/Library/WebKit/$identifier" "$HOME/Library/Caches/$identifier"
    else
      rm -rf "${XDG_CONFIG_HOME:-$HOME/.config}/$identifier" "${XDG_DATA_HOME:-$HOME/.local/share}/$identifier"
    fi ;;
esac
work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
album="$work/album"; mkdir -p "$album"
for spec in "a:440:1" "b:660:2"; do
  IFS=: read -r name freq track <<<"$spec"
  upper="$(printf '%s' "$name" | tr '[:lower:]' '[:upper:]')"
  ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=$freq:duration=3" \
    -metadata title="Sine $upper" -metadata artist="Test Artist" -metadata album="Test Album" \
    -metadata track="$track" "$album/sine-$name.mp3"
done
# Local tag source data (a freedb folder) sits next to the album.
if [[ -d "$root/tests/fixtures/freedb" ]]; then cp -R "$root/tests/fixtures/freedb" "$work/freedb"; fi
# A script may add its own fixtures: OPENTAG_E2E_FIXTURES=<executable> gets the album folder.
if [[ -n "${OPENTAG_E2E_FIXTURES:-}" ]]; then "$OPENTAG_E2E_FIXTURES" "$album"; fi
# WebKitGTK under xvfb has no GPU; use software rendering.
export WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1
output="$(OPENTAG_E2E_SCRIPT="$script" timeout 180 "$bin" "/fp:$album" 2>&1 | grep '^\[e2e\]' || true)"
printf '%s\n' "$output"
grep -q '^\[e2e\] SUMMARY' <<<"$output" || { echo "e2e run did not finish" >&2; exit 1; }
! grep -q '^\[e2e\] FAIL\|^\[e2e\] FATAL' <<<"$output"
