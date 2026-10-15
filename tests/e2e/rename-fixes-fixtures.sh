#!/usr/bin/env bash
# Fixtures for tests/e2e/rename-fixes.js, given the album folder ($1). Linux CI
# volumes are case-sensitive, so when possible a small ext4 image with
# casefolding is mounted and linked as ../casefold. Without it the suite skips
# the case-insensitive steps; this script never fails the run.
set -uo pipefail
album="$1"
[[ "$(uname)" == "Linux" ]] || exit 0
command -v mkfs.ext4 >/dev/null && command -v chattr >/dev/null && sudo -n true 2>/dev/null || exit 0
image="$(mktemp)" mount="$(mktemp -d)"
truncate -s 64M "$image"
mkfs.ext4 -q -F -O casefold "$image" >/dev/null 2>&1 || exit 0
sudo -n mount -o loop "$image" "$mount" || exit 0
sudo -n chown "$(id -u):$(id -g)" "$mount"
mkdir "$mount/music" && chattr +F "$mount/music" || exit 0
touch "$mount/music/Probe" && [[ -e "$mount/music/probe" ]] || exit 0
rm "$mount/music/Probe"
mkdir "$mount/music/album"
for spec in "album/song:330" "album/Other:550"; do
  IFS=: read -r name freq <<<"$spec"
  ffmpeg -loglevel error -y -f lavfi -i "sine=frequency=$freq:duration=1" \
    -metadata title="${name#album/}" "$mount/music/$name.mp3" || exit 0
done
ln -s "$mount/music" "$album/../casefold"
