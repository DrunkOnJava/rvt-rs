#!/usr/bin/env bash
# Seal a held-out pair for the final 1.0 check (W5, #408, #1565): download a
# Revit project and its release's own IFC export, hash and count both, delete
# them, and add their row to research/sealed-holdout-models.tsv. Nothing else is
# read from the files, so the pair stays unmeasured until that check.
#
# Usage, from the repository root:
#   tools/ci/seal-holdout-pair.sh RELEASE NAME RVT_URL IFC_URL LICENCE CONTRIBUTOR
#
# RELEASE is 2024 to 2027, LICENCE Apache-2.0, MIT or CC-BY-4.0, CONTRIBUTOR the
# GitHub login that offered the pair. Run tools/ci/check_sealed_holdout.py after
# it; the row it adds must pass.
set -euo pipefail

REGISTRY=research/sealed-holdout-models.tsv
release="${1:?usage: seal-holdout-pair.sh RELEASE NAME RVT_URL IFC_URL LICENCE CONTRIBUTOR}"
name="${2:?name}"
rvt_url="${3:?rvt url}"
ifc_url="${4:?ifc url}"
licence="${5:?licence}"
contributor="${6:?contributor}"

case "$release" in 2024|2025|2026|2027) ;; *) echo "release $release is not one of 2024 to 2027" >&2; exit 1 ;; esac
case "$licence" in Apache-2.0|MIT|CC-BY-4.0) ;; *) echo "licence $licence is not Apache-2.0, MIT or CC-BY-4.0" >&2; exit 1 ;; esac
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
for side in rvt ifc; do
  url=$rvt_url
  [ "$side" = ifc ] && url=$ifc_url
  if ! fetch_resume "$url" "$work/$side"; then
    echo "could not download the $side file" >&2
    exit 2
  fi
  fields+=("$url" "$(sha256 "$work/$side")" "$(wc -c < "$work/$side" | tr -d ' ')")
  rm -f "$work/$side"
done
row=$(printf '%s\t' "$release" "$name" "${fields[@]}" "$licence" "$contributor")
row="${row}$(date -u +%Y-%m-%d)"
printf '%s\n' "$row" >> "$REGISTRY"
echo "sealed $name ($release): $(printf '%s' "$row" | cut -f4,7 | tr '\t' ' ')"
