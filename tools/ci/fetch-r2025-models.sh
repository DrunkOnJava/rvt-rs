#!/usr/bin/env bash
# Fetch puzzbobb's Revit 2025 model archives (#356, #885) for a Measure run
# and verify them.
#
# Usage:
#   tools/ci/fetch-r2025-models.sh PINS TARGET_DIR
#
# Reads each archive row of PINS (research/r2025-models.tsv), downloads the
# archive, checks its sha256 and byte count and extracts into TARGET_DIR/r2025/
# either its .rvt and .ifc files or, where the row's fifth column names
# `member=name` pairs (comma-separated), those members alone under those
# names. Exits 1 when an archive does not match its pin and 2 when one could
# not be downloaded (NOT MEASURED).
set -euo pipefail

PINS="${1:?usage: fetch-r2025-models.sh PINS TARGET_DIR}"
TARGET="${2:?usage: fetch-r2025-models.sh PINS TARGET_DIR}"
rows=$(awk -F'\t' '$1 == "archive" { print $2 "\t" $3 "\t" $4 "\t" $5 }' "$PINS")
if [ -z "$rows" ]; then
  echo "no archive row in $PINS"
  exit 1
fi
mkdir -p "$TARGET/r2025"
n=0
while IFS=$'\t' read -r url sha bytes members; do
  n=$((n + 1))
  zip="$TARGET/r2025-$n.zip"
  if ! curl -fsSL --retry 3 --retry-all-errors --retry-delay 5 --max-time 900 -o "$zip" "$url"; then
    echo "could not download $url"
    exit 2
  fi
  got_sha=$(sha256sum "$zip" | cut -d' ' -f1)
  got_bytes=$(wc -c < "$zip" | tr -d ' ')
  if [ "$got_sha" != "$sha" ] || [ "$got_bytes" != "$bytes" ]; then
    echo "$url: got $got_bytes bytes sha256 $got_sha, pinned $bytes bytes $sha"
    rm -f "$zip"
    exit 1
  fi
  if [ -n "$members" ]; then
    IFS=',' read -ra pairs <<< "$members"
    for pair in "${pairs[@]}"; do
      unzip -q -o -j "$zip" "${pair%%=*}" -d "$TARGET/r2025"
      mv "$TARGET/r2025/$(basename "${pair%%=*}")" "$TARGET/r2025/${pair#*=}"
    done
  else
    unzip -q -o -j "$zip" '*.rvt' '*.ifc' -d "$TARGET/r2025"
  fi
  rm -f "$zip"
done <<< "$rows"
echo "R2025 models: $(ls "$TARGET"/r2025/*.rvt | wc -l) .rvt, $(ls "$TARGET"/r2025/*.ifc | wc -l) .ifc"
