#!/usr/bin/env bash
# Fetch the Revit 2018 export of Autodesk's rac_basic_sample_project that a user attached to the IFC for
# Revit wiki (research/rac-basic-2018-export.tsv, RE-326 section 5) for a Measure run, and verify it
# (#2148).
#
# Usage:
#   tools/ci/fetch-rac-basic-2018-export.sh TARGET_DIR
#
# Puts it in TARGET_DIR/ as rac_basic_sample_project.ifc, checked against its pinned sha256 and byte
# count. A file that does not match is deleted and the script exits 1; a download that fails exits 2
# (NOT MEASURED). Nothing is stored: the file is fetched for the run (it carries no licence).
set -euo pipefail

TARGET="${1:?usage: fetch-rac-basic-2018-export.sh TARGET_DIR}"
PINS="$(dirname "$0")/../../research/rac-basic-2018-export.tsv"
sha256() { if command -v sha256sum > /dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }
mkdir -p "$TARGET"

while IFS=$'\t' read -r kind name url sha bytes; do
  [ "$kind" = export ] || continue
  path="$TARGET/$name"
  if ! curl -fsSL --retry 3 --retry-all-errors --retry-delay 5 --max-time 600 -o "$path" "$url"; then
    rm -f "$path"
    echo "could not download $url"
    exit 2
  fi
  got_sha=$(sha256 "$path")
  got_bytes=$(wc -c < "$path" | tr -d ' ')
  if [ "$got_sha" != "$sha" ] || [ "$got_bytes" != "$bytes" ]; then
    rm -f "$path"
    echo "$name: got $got_bytes bytes sha256 $got_sha, pinned $bytes bytes $sha"
    exit 1
  fi
  echo "$name: matches its pin ($got_bytes bytes)"
done < <(grep -v '^#' "$PINS")
