#!/usr/bin/env bash
# #2085 M rows 3 and 4: per kept model, ifc_body_diff (base vs head) and the newly drawn bodies against Revit's export.
# Usage: read2085M.sh <measure download dir> <revit ifc dir> <out dir>
# TOOLS (default the repository's tools/re) and RVT_PROJECT_CORPUS_DIR (for RE1 Architecture's IFC).
set -u
D=$1; RI=$2; OUT=$3; TOOLS=${TOOLS:-$(cd "$(dirname "$0")/../../tools/re" && pwd)}
S=$(dirname "$0")
mkdir -p "$OUT"
declare -A REVIT=(
  [nordic-housing-concrete-as-built]=ARK_NordicLCA_Housing_Concrete_As-Built_Revit.ifc
  [nordic-housing-concrete-permit]=ARK_NordicLCA_Housing_Concrete_BuildingPermit_Revit.ifc
  [nordic-housing-timber-as-built]=ARK_NordicLCA_Housing_Timber_As-Built_Revit.ifc
  [nordic-housing-timber-permit]=ARK_NordicLCA_Housing_Timber_BuildingPermit_Revit.ifc
  [nordic-office-as-built]=ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc
  [nordic-office-permit]=ARK_NordicLCA_Office_Concrete_BuildingPermit_Revit.ifc
  [nordic-office-timber-as-built]=ARK_NordicLCA_Office_Timber_As-Built_Revit.ifc
  [nordic-office-timber-permit]=ARK_NordicLCA_Office_Timber_BuildingPermit_Revit.ifc
)
for head in $(find "$D/measure-head" -name model.ifc); do
  m=$(basename "$(dirname "$head")")
  base=$(find "$D/measure-base" -path "*/$m/model.ifc" | head -1)
  [ -z "$base" ] && { echo "$m: no base export"; continue; }
  hs="$(dirname "$head")/ifc.sha256"; bs="$(dirname "$base")/ifc.sha256"
  [ -f "$hs" ] && [ -f "$bs" ] && cmp -s "$hs" "$bs" && continue
  python3 -u "$TOOLS/ifc_body_diff.py" "$base" "$head" "$OUT/$m.newly.txt" > "$OUT/$m.diff.txt" 2>&1
  (cd "$S" && uv run --no-project --quiet --with ifcopenshell==0.9.0.post1 python -u wallq2085.py "$base" "$head") > "$OUT/$m.walls.txt" 2>&1
  rv="${REVIT[$m]:-}"
  [ "$m" = re1-architecture ] && rv="${RVT_PROJECT_CORPUS_DIR:?set RVT_PROJECT_CORPUS_DIR}/RE1-Architecture.ifc"
  if [ -n "$rv" ]; then
    case "$rv" in /*) ;; *) rv="$RI/$rv" ;; esac
    (cd "$S" && uv run --no-project --quiet --with ifcopenshell==0.9.0.post1 --with numpy python -u newbodies2085.py "$TOOLS" "$head" "$rv" "$OUT/$m.newly.txt") > "$OUT/$m.newbodies.txt" 2>&1
  else
    echo "newly drawn: $(grep -c . "$OUT/$m.newly.txt" 2>/dev/null || echo 0) (no Revit export: not measured)" > "$OUT/$m.newbodies.txt"
  fi
  echo "$m done"
done
