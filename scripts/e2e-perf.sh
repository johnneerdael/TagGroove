#!/usr/bin/env bash
# Measures interaction timings of a release-optimized profiling build against a
# real folder. The folder is never written: each invocation makes a fresh APFS
# copy-on-write clone (`cp -c`, no extra disk) and launches the app on it with
# `/fp:<clone>`. A fresh clone also has no cached file pages, so the first run
# approximates a cold read; later runs in the same invocation are warm.
# Usage: scripts/e2e-perf.sh <folder> [runs] [--no-build] [--read-only]
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
folder="${1:?folder to measure}"; runs="${2:-2}"; shift $(( $# < 2 ? $# : 2 ))
build=1; write=true
for arg in "$@"; do
  case "$arg" in --no-build) build=0 ;; --read-only) write=false ;; esac
done
identifier="org.opentag.editor.e2e.perf"
if (( build )); then
  (cd "$root" && AR="${AR:-/usr/bin/ar}" npx tauri build --bundles app --features e2e \
    --config "{\"identifier\":\"$identifier\",\"productName\":\"OpenTag Perf\"}" >/dev/null)
fi
bin="$root/target/release/bundle/macos/OpenTag Perf.app/Contents/MacOS/opentag"
# App Nap throttles timers, animation frames and disk I/O of windows that are
# not visible, which would turn background runs into multi-second outliers.
defaults write "$identifier" NSAppSleepDisabled -bool YES
scratch="${OPENTAG_PERF_SCRATCH:-${TMPDIR:-/tmp}}"
work="$(mktemp -d "$scratch/opentag-perf.XXXXXX")"; trap 'rm -rf "$work"' EXIT
clone="$work/$(basename "$folder")"
cp -cR "$folder" "$clone"
for run in $(seq 1 "$runs"); do
  script="$work/perf-$run.js"
  launch="$(perl -MTime::HiRes=time -e 'printf "%d", time*1000')"
  printf 'window.__OPENTAG_PERF__={launch:%s,write:%s};\n' "$launch" "$write" >"$script"
  [[ "$write" == true ]] && write=false # only the first run writes; later runs stay comparable
  cat "$root/tests/e2e/performance.js" >>"$script"
  echo "[e2e] RUN $run of $runs: $clone"
  OPENTAG_E2E_SCRIPT="$script" timeout 3600 "$bin" "/fp:$clone" 2>&1 | grep --line-buffered '^\[e2e\]' || true
done
