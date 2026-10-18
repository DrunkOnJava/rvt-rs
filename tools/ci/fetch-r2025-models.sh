#!/usr/bin/env bash
# Fetch puzzbobb's R2025_Models.zip (#356) for a Measure run and verify it.
#
# Usage:
#   tools/ci/fetch-r2025-models.sh PINS TARGET_DIR
#
# Reads the archive row of PINS (research/r2025-models.tsv), downloads the
# archive, checks its sha256 and byte count and extracts its .rvt and .ifc
# files into TARGET_DIR/r2025/. Exits 1 when the archive does not match its
# pin and 2 when it could not be downloaded (NOT MEASURED).
set -euo pipefail

PINS="${1:?usage: fetch-r2025-models.sh PINS TARGET_DIR}"
TARGET="${2:?usage: fetch-r2025-models.sh PINS TARGET_DIR}"
row=$(awk -F'\t' '$1 == "archive" { print $2, $3, $4 }' "$PINS")
read -r url sha bytes <<< "$row"
if [ -z "${url:-}" ]; then
  echo "no archive row in $PINS"
  exit 1
fi
mkdir -p "$TARGET/r2025"
zip="$TARGET/r2025.zip"
if ! curl -fsSL --retry 3 --retry-delay 5 --max-time 900 -o "$zip" "$url"; then
  echo "could not download $url"
  exit 2
fi
got_sha=$(sha256sum "$zip" | cut -d' ' -f1)
got_bytes=$(wc -c < "$zip" | tr -d ' ')
if [ "$got_sha" != "$sha" ] || [ "$got_bytes" != "$bytes" ]; then
  echo "R2025_Models.zip: got $got_bytes bytes sha256 $got_sha, pinned $bytes bytes $sha"
  rm -f "$zip"
  exit 1
fi
unzip -q -o -j "$zip" '*.rvt' '*.ifc' -d "$TARGET/r2025"
rm -f "$zip"
echo "R2025 models: $(ls "$TARGET"/r2025/*.rvt | wc -l) .rvt, $(ls "$TARGET"/r2025/*.ifc | wc -l) .ifc"
