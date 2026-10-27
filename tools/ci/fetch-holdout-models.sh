#!/usr/bin/env bash
# Fetch the held-out models (research/holdout-models.tsv, RE-252) for a Measure
# run and verify them.
#
# Usage:
#   tools/ci/fetch-holdout-models.sh PINS TARGET_DIR
#
# Downloads each file PINS names into TARGET_DIR/holdout/ and checks its sha256
# and byte count. Exits 1 when a file does not match its pin and 2 when one
# could not be downloaded (NOT MEASURED).
set -euo pipefail

PINS="${1:?usage: fetch-holdout-models.sh PINS TARGET_DIR}"
TARGET="${2:?usage: fetch-holdout-models.sh PINS TARGET_DIR}"
sha256() { if command -v sha256sum > /dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }
mkdir -p "$TARGET/holdout"
while IFS=$'\t' read -r name url sha bytes; do
  file="$TARGET/holdout/$name"
  if ! curl -fsSL --retry 3 --retry-all-errors --retry-delay 5 --max-time 900 -o "$file" "$url"; then
    echo "could not download $name"
    rm -f "$file"
    exit 2
  fi
  got_sha=$(sha256 "$file")
  got_bytes=$(wc -c < "$file" | tr -d ' ')
  if [ "$got_sha" != "$sha" ] || [ "$got_bytes" != "$bytes" ]; then
    echo "$name: got $got_bytes bytes sha256 $got_sha, pinned $bytes bytes $sha"
    rm -f "$file"
    exit 1
  fi
  echo "$name: matches its pin"
done < <(grep -v '^#' "$PINS")
