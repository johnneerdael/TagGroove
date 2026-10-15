#!/usr/bin/env bash
# Copies an existing historical docs-screens artifact into the user guide.
# New capture is paused pending review-gated CI support. No workflow is
# discovered or dispatched; nothing is launched locally.
# Usage: scripts/docs-screens-fetch.sh <run-id> [scene ...]
#   run-id  an explicit existing run whose source and images were verified
#   scene   only copy these scenes, e.g. main-window tag-to-filename
# Requires the GitHub CLI (gh), signed in.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
dest="$root/docs/user-guide/assets/screens"
run="${1:-}"
if [[ ! "$run" =~ ^[0-9]+$ ]]; then
  echo "New screenshot capture is paused pending review-gated CI support. To fetch a verified historical artifact, pass an explicit numeric run ID: scripts/docs-screens-fetch.sh <run-id> [scene ...]" >&2
  exit 2
fi
shift
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
gh run download "$run" --name docs-screens --dir "$tmp"
mkdir -p "$dest"
count=0
for file in "$tmp"/*.png; do
  name="$(basename "$file" .png)"; scene="${name%-light}"; scene="${scene%-dark}"
  if [[ $# -gt 0 ]] && ! printf '%s\n' "$@" | grep -qxF "$scene"; then continue; fi
  cp "$file" "$dest/$name.png"; count=$((count + 1))
done
echo "Copied $count screenshots from run $run into ${dest#"$root/"}"
