#!/usr/bin/env bash
# Fetch the makers' public Revit families (#152) and verify them.
#
# Usage:
#   tools/fetch-public-families.sh target-dir
#
# Reads research/public-families.tsv. Each row is a zip on the maker's own
# site: it is downloaded, checked against its pinned sha256 and size, and the
# .rfa inside it whose sha256 and size are pinned is written to target-dir
# under the row's name. The files are fetched for a measurement run and never
# stored in this repository.
#
# A host that cannot be reached after the retries is NOT MEASURED: a warning,
# and the row is skipped. A file whose hash or size differs from the pin is a
# failure (exit 1): the evidence the pin names is no longer what the host
# serves.

set -euo pipefail

TARGET_DIR="${1:?usage: fetch-public-families.sh target-dir}"
PINS="$(dirname "$0")/../research/public-families.tsv"
mkdir -p "$TARGET_DIR"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

failed=0
while IFS=$'\t' read -r year name url zip_sha zip_bytes rfa_sha rfa_bytes _source; do
  case "$year" in ''|\#*) continue ;; esac
  path="$TARGET_DIR/$name"
  if [ -f "$path" ] && [ "$(sha256sum "$path" | cut -d' ' -f1)" = "$rfa_sha" ]; then
    echo "  [ok]        $name"
    continue
  fi
  zip="$WORK/$name.zip"
  echo "  [fetch]     $name <- $url"
  if ! curl -fsSL --retry 3 --retry-delay 5 --retry-all-errors --max-time 600 -o "$zip" "$url"; then
    echo "::warning::$name NOT MEASURED: $url could not be fetched"
    continue
  fi
  got_sha="$(sha256sum "$zip" | cut -d' ' -f1)"
  got_bytes="$(wc -c < "$zip" | tr -d ' ')"
  if [ "$got_sha" != "$zip_sha" ] || [ "$got_bytes" != "$zip_bytes" ]; then
    echo "  [MISMATCH]  $name zip: got $got_bytes bytes sha256 $got_sha, pinned $zip_bytes bytes $zip_sha"
    failed=1
    continue
  fi
  out="$WORK/$name.d"
  mkdir -p "$out"
  # A zip without any .rfa is reported below as a mismatch.
  unzip -q -o -j "$zip" '*.rfa' -d "$out" || true
  found=""
  for rfa in "$out"/*.rfa; do
    [ -f "$rfa" ] || continue
    if [ "$(sha256sum "$rfa" | cut -d' ' -f1)" = "$rfa_sha" ] \
      && [ "$(wc -c < "$rfa" | tr -d ' ')" = "$rfa_bytes" ]; then
      found="$rfa"
      break
    fi
  done
  if [ -z "$found" ]; then
    echo "  [MISMATCH]  $name: no .rfa in the zip has sha256 $rfa_sha and $rfa_bytes bytes"
    failed=1
    continue
  fi
  mv "$found" "$path"
  echo "  [verified]  $name ($rfa_bytes bytes, Revit $year)"
done < "$PINS"
exit "$failed"
