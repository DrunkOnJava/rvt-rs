#!/usr/bin/env bash
# Seal a held-out pair for the final 1.0 check (W5, #408, #1565): download a
# Revit project and its release's own IFC export, hash and count both, upload
# both unopened to the private repository DrunkOnJava/rvt-rs-sealed, delete the
# local copies, and add their row to research/sealed-holdout-models.tsv. Only the first
# 64 KiB of each are looked at, and only to refuse what is not a pair: a project
# that is not an OLE2 file, an IFC that is not ISO 10303-21 of IFC4 or IFC4X3,
# or one that does not name Autodesk Revit RELEASE there (a share page, a zip,
# another release's export). The project's own release is the contributor's
# word: only its OLE2 magic is checked. Nothing is printed from them, so the pair stays
# unmeasured until that check. The row is judged by tools/ci/check_sealed_holdout.py
# before it is added, so a duplicate or a bad field never reaches the registry.
#
# Usage, from the repository root:
#   tools/ci/seal-holdout-pair.sh RELEASE NAME RVT_URL IFC_URL LICENCE CONTRIBUTOR
#
# RELEASE is 2024 to 2027, LICENCE Apache-2.0, MIT, CC-BY-4.0 or CC0-1.0, CONTRIBUTOR the
# GitHub login that offered the pair. Run tools/ci/check_sealed_holdout.py after
# it; the row it adds must pass.
#
# The copy (Griffin, 2026-11-08): the final check fetches the pair again from its
# links, and a contributor can still delete a repository behind a pinned link. Both
# files go to BACKUP's `sealed` release as <sha256>.rvt and <sha256>.ifc, the
# hashes the row holds, before the row is added, so a pair is never listed without
# its copy. It needs `gh` signed in as the maintainer, who alone can write there.
set -euo pipefail

REGISTRY=research/sealed-holdout-models.tsv
BACKUP=DrunkOnJava/rvt-rs-sealed
release="${1:?usage: seal-holdout-pair.sh RELEASE NAME RVT_URL IFC_URL LICENCE CONTRIBUTOR}"
name="${2:?name}"
rvt_url="${3:?rvt url}"
ifc_url="${4:?ifc url}"
licence="${5:?licence}"
contributor="${6:?contributor}"

case "$release" in 2024|2025|2026|2027) ;; *) echo "release $release is not one of 2024 to 2027" >&2; exit 1 ;; esac
case "$licence" in Apache-2.0|MIT|CC-BY-4.0|CC0-1.0) ;; *) echo "licence $licence is not Apache-2.0, MIT, CC-BY-4.0 or CC0-1.0" >&2; exit 1 ;; esac
[[ "$name" =~ ^[A-Za-z0-9._-]+$ ]] || { echo "name $name is not [A-Za-z0-9._-]" >&2; exit 1; }
for url in "$rvt_url" "$ifc_url"; do
  [[ "$url" =~ ^https://[^[:space:]]+$ ]] || { echo "$url is not an https URL" >&2; exit 1; }
done
[ -f "$REGISTRY" ] || { echo "$REGISTRY is missing; run from the repository root" >&2; exit 1; }
if cut -f2 "$REGISTRY" | grep -qxF -- "$name"; then
  echo "$name is already sealed" >&2
  exit 1
fi

# shellcheck source=tools/ci/fetch-resume.sh
. "$(dirname "$0")/fetch-resume.sh"
sha256() { if command -v sha256sum > /dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fields=()
copies=()
for side in rvt ifc; do
  url=$rvt_url
  [ "$side" = ifc ] && url=$ifc_url
  if ! fetch_resume "$url" "$work/$side"; then
    echo "could not download the $side file" >&2
    exit 2
  fi
  sum=$(sha256 "$work/$side")
  size=$(wc -c < "$work/$side" | tr -d ' ')
  fields+=("$url" "$sum" "$size")
  head -c 65536 "$work/$side" > "$work/$side.head"
  mv "$work/$side" "$work/$sum.$side"
  copies+=("$work/$sum.$side:$size")
done
if ! python3 "$(dirname "$0")/check_sealed_holdout.py" --pair "$work/rvt.head" "$work/ifc.head" "$release" >&2; then
  echo "not sealed: the files are not a Revit $release project and its own IFC4 export" >&2
  exit 1
fi
rm -f "$work/rvt.head" "$work/ifc.head"
row=$(printf '%s\t' "$release" "$name" "${fields[@]}" "$licence" "$contributor")
row="${row}$(date -u +%Y-%m-%d)"
{ cat "$REGISTRY"; printf '%s\n' "$row"; } > "$work/candidate.tsv"
if ! python3 "$(dirname "$0")/check_sealed_holdout.py" --rows "$work/candidate.tsv" >&2; then
  echo "not sealed: the row would leave the registry failing its check" >&2
  exit 1
fi
for copy in "${copies[@]}"; do
  file=${copy%:*}
  size=${copy##*:}
  asset=$(basename "$file")
  # An asset is named by its own hash, so replacing one only repeats a run that
  # stopped before the row was added.
  if ! gh release upload sealed "$file" -R "$BACKUP" --clobber > /dev/null; then
    echo "not sealed: the copy $asset could not be uploaded to $BACKUP" >&2
    exit 1
  fi
  uploaded=$(gh release view sealed -R "$BACKUP" --json assets --jq ".assets[] | select(.name == \"$asset\") | .size")
  if [ "$uploaded" != "$size" ]; then
    echo "not sealed: $BACKUP holds $asset as ${uploaded:-nothing}, not $size bytes" >&2
    exit 1
  fi
  rm -f "$file"
done
printf '%s\n' "$row" >> "$REGISTRY"
echo "sealed $name ($release): $(printf '%s' "$row" | cut -f4,7 | tr '\t' ' ')"
