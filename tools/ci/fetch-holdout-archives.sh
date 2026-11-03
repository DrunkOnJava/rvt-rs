#!/usr/bin/env bash
# Fetch the held-out models that come in archives or need a name of their own
# (research/holdout-archive-models.tsv, #408, #1261) for a Measure run, and
# verify them.
#
# Usage:
#   tools/ci/fetch-holdout-archives.sh PINS TARGET_DIR
#
# Each `file` row is downloaded into TARGET_DIR/holdout/ under its name; each
# `archive` row (a .tar.gz) is downloaded and checked whole, and each `member`
# row extracted from it into TARGET_DIR/holdout/ under its own name, so that a
# model and Revit's export of it share a stem. A name under `spent/` goes to
# TARGET_DIR/spent/ instead: a pair no longer held out, measured in full. Every file
# is checked against its sha256 and byte count. Exits 1 when a file does not
# match its pin and 2 when one could not be downloaded (NOT MEASURED).
set -euo pipefail

PINS="${1:?usage: fetch-holdout-archives.sh PINS TARGET_DIR}"
TARGET="${2:?usage: fetch-holdout-archives.sh PINS TARGET_DIR}"
sha256() { if command -v sha256sum > /dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$TARGET/holdout" "$TARGET/spent"
dest() { case "$1" in spent/*) echo "$TARGET/$1" ;; *) echo "$TARGET/holdout/$1" ;; esac; }
check() { # file name sha bytes
  local got_sha got_bytes
  got_sha=$(sha256 "$1")
  got_bytes=$(wc -c < "$1" | tr -d ' ')
  if [ "$got_sha" != "$3" ] || [ "$got_bytes" != "$4" ]; then
    echo "$2: got $got_bytes bytes sha256 $got_sha, pinned $4 bytes $3"
    rm -f "$1"
    exit 1
  fi
  echo "$2: matches its pin"
}
fetch() { # url out
  curl -fsSL --retry 3 --retry-all-errors --retry-delay 5 --max-time 900 \
    -A "rvt-rs-ci (https://github.com/DrunkOnJava/rvt-rs)" -o "$2" "$1"
}
while IFS=$'\t' read -r kind name source sha bytes; do
  case "$kind" in
    file)
      fetch "$source" "$(dest "$name")" || { rm -f "$(dest "$name")"; echo "could not download $name"; exit 2; }
      check "$(dest "$name")" "$name" "$sha" "$bytes"
      ;;
    archive)
      fetch "$source" "$work/$name.tar.gz" || { echo "could not download $name"; exit 2; }
      check "$work/$name.tar.gz" "$name" "$sha" "$bytes"
      ;;
    member)
      archive="${source%%:*}" path="${source#*:}"
      tar -xzf "$work/$archive.tar.gz" -C "$work" "$path" || { echo "$name: $path not in $archive"; exit 1; }
      mv "$work/$path" "$(dest "$name")"
      check "$(dest "$name")" "$name" "$sha" "$bytes"
      ;;
  esac
done < <(grep -v '^#' "$PINS")
