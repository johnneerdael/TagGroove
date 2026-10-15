#!/usr/bin/env bash
# Copies the screenshots captured by the Docs screenshots workflow into the
# user guide. Only downloads an artifact; nothing is launched locally.
# Usage: scripts/docs-screens-fetch.sh [run-id] [scene ...]
#   run-id  a Docs screenshots run (default: the latest successful run on the
#           current branch)
#   scene   only copy these scenes, e.g. main-window tag-to-filename
# Requires the GitHub CLI (gh), signed in.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
dest="$root/docs/user-guide/assets/screens"
run="${1:-}"; [[ $# -gt 0 ]] && shift
if [[ -z "$run" ]]; then
  branch="$(git -C "$root" branch --show-current)"
  run="$(gh run list --workflow docs-screenshots.yml --branch "$branch" --status success --limit 1 --json databaseId --jq '.[0].databaseId')"
  [[ -n "$run" ]] || { echo "No successful Docs screenshots run on $branch; start one with: gh workflow run docs-screenshots.yml --ref $branch" >&2; exit 1; }
fi
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
