#!/usr/bin/env bash
# Score rvt-rs against Revit's own exports of the redistributable reference
# models, for the Measure workflow (.github/workflows/measure.yml).
#
# Usage:
#   tools/ci/measure-reference-models.sh BIN_DIR MODELS_DIR OUT_DIR
#
# BIN_DIR holds the rvt-ifc and rvt-gltf to measure; MODELS_DIR is where
# tools/fetch-reference-models.sh put the models. For each model this
# writes OUT_DIR/<model>/:
#   - model.ifc, when KEEP_IFC is set: the exported IFC itself;
#   - ifc.sha256: the exported IFC's hash without its FILE_NAME and
#     IFCOWNERHISTORY lines, which carry the export time, so two builds
#     that write the same model hash the same;
#   - witness-<artifact>.json: the rvt-rs observation committed under
#     research/witness/<artifact>/observations/rvt-rs.json;
#   - <scorer>.txt: each tools/re scorer's output against Revit's export,
#     ending with its exit status;
#   - probe.txt, when PROBE names an examples/ probe already built in
#     BIN_DIR/examples: its output on the model, ending with its exit status;
#   - probe_score.txt, when PROBE_SCORER also names a scorer in
#     PROBE_SCORER_DIR taking <probe output> <revit-export.ifc>.
# A scorer that fails is recorded, not fatal: its output is the evidence.
set -uo pipefail
BIN="$1"; MODELS="$2"; OUT="$3"
SCORERS="$(dirname "$0")/../re"
mkdir -p "$OUT"

# model name | .rvt | Revit's export ("-" when there is none)
MODELS_LIST="
core-interior|2024_Core_Interior.rvt|2024_Core_Interior_slim.ifc
einhoven|Revit_IFC5_Einhoven.rvt|-
re1-architecture|RE1-Architecture.rvt|RE1-Architecture.ifc
re1-electrical|RE1-Electrical.rvt|RE1-Electrical.ifc
re1-mechanical|RE1-Mechanical.rvt|RE1-Mechanical.ifc
re1-plumbing|RE1-Plumbing.rvt|RE1-Plumbing.ifc
"
# Scorers taking <rvt-rs.ifc> <revit-export.ifc>, and <model.glb> <revit-export.ifc>.
IFC_SCORERS="aggregates_vs_ifc global_ids_vs_ifc names_vs_ifc storeys_vs_ifc
room_names_vs_ifc type_objects_vs_ifc layer_sets_vs_ifc element_materials_vs_ifc
material_colours_vs_ifc openings_vs_ifc opening_hosts_vs_ifc opening_boxes_vs_ifc
door_bodies_vs_ifc oriented_boxes_vs_ifc plan_profiles_vs_ifc"
GLB_SCORERS="wall_bodies_vs_ifc wall_ends_cv_vs_ifc wall_layers_vs_ifc slab_layers_vs_ifc glb_material_colours_vs_ifc"

# Witness artifacts (research/witness/<id>/observations/rvt-rs.json) per model.
witness_artifacts() {
  case "$1" in
    core-interior) echo "magnetar-2024-core-interior magnetar-2024-core-interior-slim" ;;
  esac
}

score() { # name args...
  local out="$1"; shift
  timeout 1200 python "$@" > "$out" 2>&1
  echo "exit $?" >> "$out"
}

while IFS='|' read -r name rvt ref; do
  [ -n "$name" ] || continue
  dir="$OUT/$name"; mkdir -p "$dir"
  echo "::group::$name"
  "$BIN/rvt-ifc" "$MODELS/$rvt" -o "$dir/model.ifc" --diagnostics "$dir/diagnostics.json" > "$dir/rvt-ifc.log" 2>&1
  echo "exit $?" >> "$dir/rvt-ifc.log"
  "$BIN/rvt-gltf" "$MODELS/$rvt" -o "$dir/model.glb" > "$dir/rvt-gltf.log" 2>&1
  echo "exit $?" >> "$dir/rvt-gltf.log"
  grep -v -E '^FILE_NAME\(|IFCOWNERHISTORY' "$dir/model.ifc" | sha256sum | cut -d' ' -f1 > "$dir/ifc.sha256"
  # The witness observations committed for this model (research/witness/),
  # regenerated so a change to its IFC can commit them from the results.
  for artifact in $(witness_artifacts "$name"); do
    "$BIN/rvt-ifc" "$MODELS/$rvt" -o "$dir/witness.ifc" --observation "$dir/witness-$artifact.json" --artifact-id "$artifact" > /dev/null 2>&1
    rm -f "$dir/witness.ifc"
  done
  if [ -n "${PROBE:-}" ] && [ -x "$BIN/examples/$PROBE" ]; then
    timeout 1200 "$BIN/examples/$PROBE" "$MODELS/$rvt" > "$dir/probe.txt" 2>&1
    echo "exit $?" >> "$dir/probe.txt"
  fi
  if [ "$ref" != "-" ]; then
    if [ -f "$dir/probe.txt" ] && [ -n "${PROBE_SCORER:-}" ] && [ -f "${PROBE_SCORER_DIR:-$SCORERS}/$PROBE_SCORER.py" ]; then
      score "$dir/probe_score.txt" "${PROBE_SCORER_DIR:-$SCORERS}/$PROBE_SCORER.py" "$dir/probe.txt" "$MODELS/$ref" --list
    fi
    for s in $IFC_SCORERS; do score "$dir/$s.txt" "$SCORERS/$s.py" "$dir/model.ifc" "$MODELS/$ref"; done
    for s in $GLB_SCORERS; do score "$dir/$s.txt" "$SCORERS/$s.py" "$dir/model.glb" "$MODELS/$ref"; done
  fi
  # The bulky outputs stay out of the uploaded results.
  rm -f "$dir/model.glb"
  [ -n "${KEEP_IFC:-}" ] || rm -f "$dir/model.ifc"
  echo "::endgroup::"
done <<< "$MODELS_LIST"
