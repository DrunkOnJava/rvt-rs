#!/usr/bin/env bash
# Fetch BIMcollab's example structural model (Revit 2015) and Revit's IFC export
# of it for a Measure run, and verify them (RE-198, #421).
#
# Usage:
#   tools/ci/fetch-bimcollab.sh PINS TARGET_DIR
#
# Downloads the archives PINS (research/bimcollab-models.tsv) names, extracts
# each pinned member into TARGET_DIR/bimcollab/ and checks its sha256 and byte
# count. The archives themselves are not pinned: BIMcollab repacks them. Exits 1
# when a member does not match its pin and 2 when an archive could not be
# downloaded (NOT MEASURED).
set -euo pipefail

PINS="${1:?usage: fetch-bimcollab.sh PINS TARGET_DIR}"
TARGET="${2:?usage: fetch-bimcollab.sh PINS TARGET_DIR}"
sha256() { if command -v sha256sum > /dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$TARGET/bimcollab"
while IFS=$'\t' read -r kind name source sha bytes; do
  case "$kind" in
    archive)
      if ! curl -fsSL --retry 3 --retry-delay 5 --max-time 900 -o "$work/$name.zip" "$source"; then
        echo "could not download $source"
        exit 2
      fi
      echo "$name: $(wc -c < "$work/$name.zip" | tr -d ' ') bytes, sha256 $(sha256 "$work/$name.zip") (pinned $bytes, $sha)"
      ;;
    member)
      archive="${source%%:*}" path="${source#*:}"
      outer="${path%%!*}" inner="${path#*!}"
      if [ "$inner" != "$path" ]; then
        unzip -q -o -j "$work/$archive.zip" "$outer" -d "$work/nested"
        unzip -q -o -j "$work/nested/$(basename "$outer")" "$inner" -d "$TARGET/bimcollab"
      else
        unzip -q -o -j "$work/$archive.zip" "$path" -d "$TARGET/bimcollab"
      fi
      got_sha=$(sha256 "$TARGET/bimcollab/$name")
      got_bytes=$(wc -c < "$TARGET/bimcollab/$name" | tr -d ' ')
      if [ "$got_sha" != "$sha" ] || [ "$got_bytes" != "$bytes" ]; then
        echo "$name: got $got_bytes bytes sha256 $got_sha, pinned $bytes bytes $sha"
        rm -f "$TARGET/bimcollab/$name"
        exit 1
      fi
      echo "$name: matches its pin"
      ;;
  esac
done < <(grep -v '^#' "$PINS")
