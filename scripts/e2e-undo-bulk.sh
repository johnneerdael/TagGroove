#!/usr/bin/env bash
# Saves one field across many generated FLAC files in the real desktop app and
# undoes it. The files are copy-on-write clones of one noise template (APFS
# `cp -c`, reflinks elsewhere), so the folder exceeds the undo cap in size while
# using little disk. Requires ffmpeg and a cloning volume for the scratch dir.
# Usage: scripts/e2e-undo-bulk.sh [files] [--no-build]
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
files="${1:-100}"
identifier="${OPENTAG_E2E_ID:-org.opentag.editor.e2e.history}"
if [[ "${2:-}" != "--no-build" ]]; then
  (cd "$root" && AR="${AR:-/usr/bin/ar}" npx tauri build --debug --bundles app \
    --config "{\"identifier\":\"$identifier\",\"productName\":\"OpenTag UserTest\"}" >/dev/null)
fi
if [[ "$(uname)" == "Darwin" ]]; then
  bin="$root/target/debug/bundle/macos/OpenTag UserTest.app/Contents/MacOS/opentag"
  clone=(cp -c)
else
  bin="$root/target/debug/opentag"
  clone=(cp --reflink=always)
fi
work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
album="$work/album"; mkdir -p "$album"
# About 23 MB of incompressible FLAC with ffmpeg's 8 KiB padding block.
ffmpeg -loglevel error -y -f lavfi -i "anoisesrc=d=180:r=44100:a=0.5" -ac 2 \
  -metadata album="Original Album" "$work/template.flac"
for i in $(seq -w 1 "$files"); do
  "${clone[@]}" "$work/template.flac" "$album/track-$i.flac"
done
bytes=$(( $(stat -f %z "$work/template.flac" 2>/dev/null || stat -c %s "$work/template.flac") * 10#$files ))
free() { df -k "$work" | awk 'NR==2 {print $4 * 1024}'; }
before="$(free)"
script="$work/undo-bulk.js"
printf 'window.__OPENTAG_BULK__={files:%d,bytes:%d};\n' "$((10#$files))" "$bytes" >"$script"
cat "$root/tests/e2e/undo-bulk.js" >>"$script"
output="$(OPENTAG_E2E_SCRIPT="$script" timeout 900 "$bin" "/fp:$album" 2>&1 | grep '^\[e2e\]' || true)"
printf '%s\n' "$output"
echo "[e2e] DISK files=$((10#$files)) logical_bytes=$bytes free_before=$before free_after=$(free)"
grep -q '^\[e2e\] SUMMARY' <<<"$output" || { echo "e2e run did not finish" >&2; exit 1; }
! grep -q '^\[e2e\] FAIL\|^\[e2e\] FATAL' <<<"$output"
