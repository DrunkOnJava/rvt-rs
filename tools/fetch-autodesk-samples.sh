#!/usr/bin/env bash
# Fetch Autodesk's published sample projects (#421, RE-140) and verify them.
#
# Usage:
#   tools/fetch-autodesk-samples.sh target-dir [first-year [last-year]]
#
# Reads research/autodesk-sample-projects.tsv and downloads each row from
# Autodesk's own download host into target-dir as <year>-<file>, checking the
# pinned sha256 and byte count. A mismatch is deleted and reported, and the
# script exits 1. Files already present with the right hash are left alone.
# The files are Autodesk's, on the host its sample families come from (which
# phi-ag/rvt lists under CC BY-NC-SA 3.0; these projects' own terms were not
# checked): they are fetched for a measurement run and never stored in this
# repository.

set -euo pipefail

TARGET_DIR="${1:?usage: fetch-autodesk-samples.sh target-dir [first-year [last-year]]}"
FIRST="${2:-0}"
LAST="${3:-9999}"
PINS="$(dirname "$0")/../research/autodesk-sample-projects.tsv"
mkdir -p "$TARGET_DIR"

failed=0
while IFS=$'\t' read -r year file url sha bytes; do
  case "$year" in ''|\#*) continue ;; esac
  [ "$year" -ge "$FIRST" ] && [ "$year" -le "$LAST" ] || continue
  path="$TARGET_DIR/$year-$file"
  if [ -f "$path" ] && [ "$(sha256sum "$path" | cut -d' ' -f1)" = "$sha" ]; then
    echo "  [ok]        $year-$file"
    continue
  fi
  echo "  [fetch]     $year-$file <- $url"
  curl -fsSL --retry 3 --retry-delay 5 --max-time 900 -o "$path" "$url"
  got_sha="$(sha256sum "$path" | cut -d' ' -f1)"
  got_bytes="$(wc -c < "$path" | tr -d ' ')"
  if [ "$got_sha" != "$sha" ] || [ "$got_bytes" != "$bytes" ]; then
    rm -f "$path"
    echo "  [MISMATCH]  $year-$file: got $got_bytes bytes sha256 $got_sha, pinned $bytes bytes $sha"
    failed=1
  else
    echo "  [verified]  $year-$file ($got_bytes bytes)"
  fi
done < "$PINS"
exit "$failed"
