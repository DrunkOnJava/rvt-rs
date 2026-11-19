#!/usr/bin/env bash
# Fetch the standalone families research/family-test-files.tsv pins and verify
# them.
#
# Usage:
#   tools/ci/fetch-family-files.sh PINS TARGET_DIR
#
# Each `file` row of PINS gives a URL, the name to save it under in TARGET_DIR,
# its sha256 and its byte count. A file already in TARGET_DIR that matches its
# pin is kept. Exits 1 when a downloaded file does not match its pin and 2 when
# one could not be downloaded (NOT MEASURED).
set -euo pipefail

PINS="${1:?usage: fetch-family-files.sh PINS TARGET_DIR}"
TARGET="${2:?usage: fetch-family-files.sh PINS TARGET_DIR}"
rows=$(awk -F'\t' '$1 == "file" { print $2 "\t" $3 "\t" $4 "\t" $5 }' "$PINS")
if [ -z "$rows" ]; then
  echo "no file row in $PINS"
  exit 1
fi
mkdir -p "$TARGET"

matches() {
  [ -f "$1" ] && [ "$(wc -c < "$1" | tr -d ' ')" = "$3" ] && [ "$(sha256sum "$1" | cut -d' ' -f1)" = "$2" ]
}

n=0
while IFS=$'\t' read -r url name sha bytes; do
  if [[ ! "$name" =~ ^[A-Za-z0-9._-]+$ || ! "$sha" =~ ^[0-9a-f]{64}$ || ! "$bytes" =~ ^[0-9]+$ ]]; then
    echo "$PINS: the row of '$name' is malformed"
    exit 1
  fi
  n=$((n + 1))
  file="$TARGET/$name"
  if matches "$file" "$sha" "$bytes"; then
    echo "$name: already there and matches its pin"
    continue
  fi
  rm -f "$file" "$file.part"
  if ! curl -fsSL --retry 3 --retry-all-errors --retry-delay 5 --max-time 300 -o "$file.part" "$url"; then
    echo "could not download $url"
    rm -f "$file.part"
    exit 2
  fi
  if ! matches "$file.part" "$sha" "$bytes"; then
    echo "$url: got $(wc -c < "$file.part" | tr -d ' ') bytes sha256 $(sha256sum "$file.part" | cut -d' ' -f1), pinned $bytes bytes $sha"
    rm -f "$file.part"
    exit 1
  fi
  mv "$file.part" "$file"
  echo "$name: fetched"
done <<< "$rows"
echo "family test files: $n in $TARGET"
