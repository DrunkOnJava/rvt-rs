#!/usr/bin/env bash
# Fetch Autodesk's rac_advanced_sample_project of the releases 2015 to 2020 and
# the IFC2X3 export of the project from the three.js repository for a Measure
# run, and verify them (#1354, RE-330).
#
# Usage:
#   tools/ci/fetch-rac-advanced-pair.sh TARGET_DIR
#
# Reads research/rac-advanced-export.tsv (the export) and the rac_advanced rows
# of research/autodesk-sample-projects.tsv for 2015 to 2020, and puts them in
# TARGET_DIR/racadv/ as <year>-rac_advanced_sample_project.rvt and
# rac_advanced_sample_project.ifc, each checked against its pinned sha256 and
# byte count. A file that does not match is deleted and the script exits 1; a
# download that fails exits 2 (NOT MEASURED). Nothing is stored: the files are
# fetched for the run (the export carries no licence of its own).
set -euo pipefail

TARGET="${1:?usage: fetch-rac-advanced-pair.sh TARGET_DIR}"
HERE="$(dirname "$0")/../.."
EXPORT_PINS="$HERE/research/rac-advanced-export.tsv"
SAMPLE_PINS="$HERE/research/autodesk-sample-projects.tsv"
FIRST=2015
LAST=2020
sha256() { if command -v sha256sum > /dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }
mkdir -p "$TARGET/racadv"

fetch() { # path url sha bytes
  local path="$1" url="$2" sha="$3" bytes="$4" got_sha got_bytes
  if ! curl -fsSL --retry 3 --retry-all-errors --retry-delay 5 --max-time 900 -o "$path" "$url"; then
    rm -f "$path"
    echo "could not download $url"
    exit 2
  fi
  got_sha=$(sha256 "$path")
  got_bytes=$(wc -c < "$path" | tr -d ' ')
  if [ "$got_sha" != "$sha" ] || [ "$got_bytes" != "$bytes" ]; then
    rm -f "$path"
    echo "$(basename "$path"): got $got_bytes bytes sha256 $got_sha, pinned $bytes bytes $sha"
    exit 1
  fi
  echo "$(basename "$path"): matches its pin ($got_bytes bytes)"
}

while IFS=$'\t' read -r kind name url sha bytes; do
  [ "$kind" = export ] || continue
  fetch "$TARGET/racadv/$name" "$url" "$sha" "$bytes"
done < <(grep -v '^#' "$EXPORT_PINS")

while IFS=$'\t' read -r year file url sha bytes; do
  case "$year" in ''|\#*) continue ;; esac
  [ "$file" = rac_advanced_sample_project.rvt ] || continue
  [ "$year" -ge "$FIRST" ] && [ "$year" -le "$LAST" ] || continue
  fetch "$TARGET/racadv/$year-$file" "$url" "$sha" "$bytes"
done < "$SAMPLE_PINS"
