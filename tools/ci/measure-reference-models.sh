#!/usr/bin/env bash
# Score rvt-rs against Revit's own exports of the redistributable reference
# models, for the Measure workflow (.github/workflows/measure.yml).
#
# Usage:
#   tools/ci/measure-reference-models.sh BIN_DIR MODELS_DIR OUT_DIR
#
# BIN_DIR holds the rvt-ifc, rvt-gltf and rvt-info to measure; MODELS_DIR is
# where tools/fetch-reference-models.sh put the models, with the R2025 models
# in its r2025/ folder when tools/ci/fetch-r2025-models.sh fetched them, and
# BIMcollab's Revit 2015 model in its bimcollab/ folder when
# tools/ci/fetch-bimcollab.sh fetched it. For
# each model this writes OUT_DIR/<model>/:
#   - model.ifc, when KEEP_IFC is set: the exported IFC itself;
#   - ifc.sha256: the exported IFC's hash without its FILE_NAME and
#     IFCOWNERHISTORY lines, which carry the export time, so two builds
#     that write the same model hash the same;
#   - schema.txt: the IFC4 schema's findings on the export (C9), its
#     attribute counts and IfcOpenShell's validation up to 120 MB, and its
#     standard property sets against IFC4's definitions (#822);
#   - witness-<artifact>.json: the rvt-rs observation committed under
#     research/witness/<artifact>/observations/rvt-rs.json;
#   - <scorer>.txt: each tools/re scorer's output against Revit's export,
#     ending with its exit status;
#   - timings.log: each step's wall time in milliseconds, and the item's
#     after its export (the export's is in rvt-ifc.log);
#   - info.json and info.exit: `rvt-info --redact --json` on it (the identity,
#     the document's Atom entry, the streams) and its exit status. The Atom
#     entry goes through the PartAtom parser, from the model's
#     ProjectInformation stream. The IFC export reads only a PartAtom stream,
#     which none of these models has (diagnostics.json says has_part_atom
#     false), so this, and the families below, are what measure that parser;
#   - probe.txt, when PROBE names an examples/ probe already built in
#     BIN_DIR/examples: its output on the model, ending with its exit status;
#   - probe_score.txt, when PROBE_SCORER also names a scorer in
#     PROBE_SCORER_DIR taking <probe output> <revit-export.ifc>.
# When FAMILY_DIR names the Autodesk family corpus, each of its .rfa files gets
# OUT_DIR/family-<file>/ with the same info.json and info.exit (a family
# carries its Atom entry in a PartAtom stream) and, when PROBE is built, the
# same probe.txt. A family has no Revit export, so nothing else is measured on it.
# SAMPLE_DIR does the same for Autodesk's sample projects (.rvt), adding the
# export's diagnostics.json, rvt-ifc.log and ifc.sha256 (hashed as a model's
# is) under OUT_DIR/sample-<file>/. When
# PROBE_PAIR_RELEASE names a release, the probe also runs on each sample of
# another release and the same project saved by that one, the sample first,
# into probe-pair.txt, ending with its exit status: the cross-release
# comparisons of #740 and #742.
# A scorer that fails is recorded, not fatal: its output is the evidence.
# Every export runs first, alone, one at a time; then MEASURE_JOBS items run
# at once, by default one per CPU; then every probe, alone, one at a time.
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
nordic-office-as-built|ARK_NordicLCA_Office_Concrete_As-Built.rvt|ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc
nordic-office-permit|ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt|ARK_NordicLCA_Office_Concrete_BuildingPermit_Revit.ifc
"
# puzzbobb's Revit 2025 models (research/r2025-models.tsv), in MODELS_DIR/r2025
# only with `-f r2025=true`, each scored against Revit's export beside it.
if [ -d "$MODELS/r2025" ]; then
  for model in "$MODELS"/r2025/*.rvt; do
    [ -f "$model" ] || continue
    stem=$(basename "$model" .rvt)
    export_ifc="r2025/$stem.ifc"
    [ -f "$MODELS/$export_ifc" ] || export_ifc="-"
    MODELS_LIST="$MODELS_LIST
r2025-$stem|r2025/$stem.rvt|$export_ifc"
  done
fi
# BIMcollab's example structural model (Revit 2015) with Revit 2015's export of
# it (research/bimcollab-models.tsv, RE-198), only with `-f bimcollab=true`.
if [ -f "$MODELS/bimcollab/02_BIMcollab_Example_STR.rvt" ]; then
  MODELS_LIST="$MODELS_LIST
bimcollab-str-2015|bimcollab/02_BIMcollab_Example_STR.rvt|bimcollab/02_BIMcollab_Example_STR_optimized.ifc"
fi
# Scorers taking <rvt-rs.ifc> <revit-export.ifc>, and <model.glb> <revit-export.ifc>.
IFC_SCORERS="class_sets_vs_ifc aggregates_vs_ifc global_ids_vs_ifc names_vs_ifc storeys_vs_ifc
room_names_vs_ifc type_objects_vs_ifc layer_sets_vs_ifc element_materials_vs_ifc
material_colours_vs_ifc openings_vs_ifc opening_hosts_vs_ifc opening_boxes_vs_ifc
door_bodies_vs_ifc door_window_sizes_vs_ifc oriented_boxes_vs_ifc plan_profiles_vs_ifc
pipe_bodies_vs_ifc duct_bodies_vs_ifc roof_bodies_vs_ifc stair_bodies_vs_ifc ports_vs_ifc psets_vs_ifc saved_meshes_vs_ifc tagged_spaces_vs_ifc port_places_vs_ifc predefined_types_vs_ifc"
GLB_SCORERS="wall_bodies_vs_ifc wall_ends_cv_vs_ifc wall_layers_vs_ifc slab_layers_vs_ifc glb_material_colours_vs_ifc"

# Witness artifacts (research/witness/<id>/observations/rvt-rs.json) per model.
witness_artifacts() {
  case "$1" in
    core-interior) echo "magnetar-2024-core-interior magnetar-2024-core-interior-slim" ;;
  esac
}

# Each step's wall time in milliseconds, one `<step> <ms>` line per step in
# the item's timings.log, which the compare job's diff leaves out (*.log).
timed() { # dir step command...
  local dir="$1" step="$2" started status
  shift 2
  started=$(date +%s%N)
  "$@"
  status=$?
  echo "$step $(( ($(date +%s%N) - started) / 1000000 ))" >> "$dir/timings.log"
  return "$status"
}

score() { # name args...
  local out="$1" dir step; shift
  dir=$(dirname "$out"); step=$(basename "$out" .txt)
  timed "$dir" "$step" timeout 1200 python "$@" > "$out" 2>&1
  echo "exit $?" >> "$out"
}

info() { # file dir
  "$BIN/rvt-info" "$1" --redact --json > "$2/info.json" 2> "$2/info.log"
  echo "exit $?" > "$2/info.exit"
}

# C9: the IFC4 schema's view of dir/model.ifc in dir/schema.txt: attribute
# counts always, IfcOpenShell's full validation up to max_bytes, otherwise
# marked as skipped, and the standard property sets against IFC4's
# definitions, fetched once into PSET_DEFINITIONS (#822).
PSET_DEFINITIONS="${RUNNER_TEMP:-/tmp}/ifc4-pset-definitions.csv"
schema() { # dir max_bytes
  {
    echo "arity:"
    timed "$1" schema-arity timeout 600 python "$(dirname "$0")/ifc_schema_arity.py" "$1/model.ifc" 2>&1 \
      | grep -v '^IFC schema conformance passed\|^  instances checked' || true
    if [ "$(stat -c %s "$1/model.ifc")" -le "$2" ]; then
      echo "validate:"
      timed "$1" schema-validate timeout 1200 python "$(dirname "$0")/ifc_validate_summary.py" "$1/model.ifc" "$1/validation.jsonl" 2>&1
    else
      echo "validate: skipped, over $(( $2 / 1048576 )) MB"
    fi
    echo "property sets:"
    timed "$1" schema-psets timeout 1200 python "$(dirname "$0")/ifc4_property_sets.py" "$1/model.ifc" \
      --definitions "$PSET_DEFINITIONS" 2>&1
  } > "$1/schema.txt"
}

# Each export runs alone, before the items start, so its wall time in
# rvt-ifc.log is taken as it always was: the summary compares those times
# across commits, and items running beside an export would move its time
# by up to about 2x.
model_dir() { echo "$OUT/$1"; }
sample_dir() { echo "$OUT/sample-$(basename "$1" .rvt)"; }
export_ifc() { # file dir
  local started
  mkdir -p "$2"
  started=$(date +%s%N)
  # shellcheck disable=SC2086 # RVT_IFC_FLAGS is a list of flags.
  "$BIN/rvt-ifc" "$1" -o "$2/model.ifc" --diagnostics "$2/diagnostics.json" ${RVT_IFC_FLAGS:-} > "$2/rvt-ifc.log" 2>&1
  echo "exit $?" >> "$2/rvt-ifc.log"
  echo "milliseconds $(( ($(date +%s%N) - started) / 1000000 ))" >> "$2/rvt-ifc.log"
}

# Each item ends with its group in the log, written at once as items run
# together, and its wall time in its timings.log.
finished() { # dir name started
  local ms=$(( ($(date +%s%N) - $3) / 1000000 ))
  echo "item $ms" >> "$1/timings.log"
  printf '::group::%s\n%s ms\n::endgroup::\n' "$2" "$ms"
}

measure_model() { # name rvt ref
  local name="$1" rvt="$2" ref="$3" dir item_started artifact s
  item_started=$(date +%s%N)
  dir=$(model_dir "$name"); mkdir -p "$dir"
  timed "$dir" rvt-gltf "$BIN/rvt-gltf" "$MODELS/$rvt" -o "$dir/model.glb" > "$dir/rvt-gltf.log" 2>&1
  echo "exit $?" >> "$dir/rvt-gltf.log"
  timed "$dir" rvt-info info "$MODELS/$rvt" "$dir"
  grep -v -E '^FILE_NAME\(|IFCOWNERHISTORY' "$dir/model.ifc" | sha256sum | cut -d' ' -f1 > "$dir/ifc.sha256"
  # A writer path only these models reach, such as the office files'
  # thermal transmittance unit (RE-194), is checked here.
  [ -s "$dir/model.ifc" ] && schema "$dir" 125829120
  # The witness observations committed for this model (research/witness/),
  # regenerated so a change to its IFC can commit them from the results.
  for artifact in $(witness_artifacts "$name"); do
    timed "$dir" "witness-$artifact" "$BIN/rvt-ifc" "$MODELS/$rvt" -o "$dir/witness.ifc" --observation "$dir/witness-$artifact.json" --artifact-id "$artifact" > /dev/null 2>&1
    rm -f "$dir/witness.ifc"
  done
  if [ "$ref" != "-" ]; then
    for s in $IFC_SCORERS; do
      # ports_vs_ifc reads the findings the schema step's validation wrote,
      # rather than validating the export again.
      extra=()
      [ "$s" = ports_vs_ifc ] && [ -f "$dir/validation.jsonl" ] && extra=(--validation "$dir/validation.jsonl")
      score "$dir/$s.txt" "$SCORERS/$s.py" "$dir/model.ifc" "$MODELS/$ref" "${extra[@]}"
    done
    for s in $GLB_SCORERS; do score "$dir/$s.txt" "$SCORERS/$s.py" "$dir/model.glb" "$MODELS/$ref"; done
  fi
  # The bulky outputs stay out of the uploaded results.
  rm -f "$dir/model.glb" "$dir/validation.jsonl"
  [ -n "${KEEP_IFC:-}" ] || rm -f "$dir/model.ifc"
  finished "$dir" "$name" "$item_started"
}

family_dir() { echo "$OUT/family-$(basename "$1" .rfa)"; }
measure_family() { # family
  local dir item_started
  item_started=$(date +%s%N)
  dir=$(family_dir "$1"); mkdir -p "$dir"
  timed "$dir" rvt-info info "$1" "$dir"
  finished "$dir" "$(basename "$1")" "$item_started"
}

# Autodesk's sample projects (research/autodesk-sample-projects.tsv), one per
# release and model: each gets OUT_DIR/sample-<year>-<file>/ with info.json and
# info.exit, the export's diagnostics (what rvt-rs reads of it today) and its
# exit status, schema.txt (the IFC4 schema's findings on the export), and the
# probe's output (called with --records, which a probe may take to list what it
# read). There is no Revit export of them, so nothing else is scored.
# A sample's project, the same across releases: its file name without the
# year, the underscores and "sampleproject" (2016's rac_basic_sample_project
# is 2020's racbasicsampleproject).
pair_key() {
  local name
  name=$(basename "$1" .rvt)
  name=${name#????-}
  name=${name//_/}
  echo "${name%sampleproject}"
}
measure_sample() { # sample
  local sample="$1" dir item_started copy
  item_started=$(date +%s%N)
  dir=$(sample_dir "$sample"); mkdir -p "$dir"
  timed "$dir" rvt-info info "$sample" "$dir"
  # C9: the IFC4 schema's view of the export, so a writer path only the
  # samples reach (I-shape profiles and profile set usages on the
  # structural samples) is checked somewhere. Attribute counts on every
  # export; IfcOpenShell's full validation on exports up to 30 MB, the
  # rest marked as skipped.
  if [ -s "$dir/model.ifc" ]; then
    schema "$dir" 31457280
    grep -v -E '^FILE_NAME\(|IFCOWNERHISTORY' "$dir/model.ifc" | sha256sum | cut -d' ' -f1 > "$dir/ifc.sha256"
  fi
  [ -n "${KEEP_IFC:-}" ] || rm -f "$dir/model.ifc"
  rm -f "$dir/validation.jsonl"
  finished "$dir" "$(basename "$sample")" "$item_started"
}

# Every probe runs after the items, alone, one at a time: a probe is cut off
# after 20 minutes, and items running beside it would slow it by up to about
# 2x, so a probe would stop sooner than it always has.
probe() { # dir out args...
  local dir="$1" out="$2" step; shift 2
  step=$(basename "$out" .txt)
  timed "$dir" "$step" timeout 1200 "$BIN/examples/$PROBE" "$@" > "$out" 2>&1
  echo "exit $?" >> "$out"
}
probe_model() { # name rvt ref
  local dir
  dir=$(model_dir "$1")
  probe "$dir" "$dir/probe.txt" "$MODELS/$2"
  if [ "$3" != "-" ] && [ -n "${PROBE_SCORER:-}" ] && [ -f "${PROBE_SCORER_DIR:-$SCORERS}/$PROBE_SCORER.py" ]; then
    score "$dir/probe_score.txt" "${PROBE_SCORER_DIR:-$SCORERS}/$PROBE_SCORER.py" "$dir/probe.txt" "$MODELS/$3" --list
  fi
}
probe_family() { # family
  probe "$(family_dir "$1")" "$(family_dir "$1")/probe.txt" "$1"
}
probe_sample() { # sample
  local sample="$1" dir copy
  dir=$(sample_dir "$sample")
  probe "$dir" "$dir/probe.txt" "$sample" --records
  if [ -n "${PROBE_PAIR_RELEASE:-}" ] && [ "$(basename "$sample" | cut -c1-4)" != "$PROBE_PAIR_RELEASE" ]; then
    for copy in "$SAMPLE_DIR/$PROBE_PAIR_RELEASE"-*.rvt; do
      [ -f "$copy" ] && [ "$(pair_key "$copy")" = "$(pair_key "$sample")" ] || continue
      probe "$dir" "$dir/probe-pair.txt" "$sample" "$copy"
    done
  fi
}

# Every export runs first, alone; then every item runs in its own folder,
# MEASURE_JOBS at once (by default one per CPU), the longest first: the
# models scored against Revit's export, then the rest, each by file size;
# then every probe, alone.
JOBS="${MEASURE_JOBS:-$(nproc)}"
spawn() { # command...: run it once fewer than JOBS items are running
  while [ "$(jobs -rp | wc -l)" -ge "$JOBS" ]; do
    wait -n
  done
  "$@" &
}

# The definitions are fetched once here, before the items read them at once.
python -c 'import sys; sys.path.insert(0, sys.argv[1]); import ifc4_property_sets; ifc4_property_sets.definitions(sys.argv[2])' \
  "$(dirname "$0")" "$PSET_DEFINITIONS"

items=""
while IFS='|' read -r name rvt ref; do
  [ -n "$name" ] || continue
  # The BIM4LCA office models are fetched only with `-f nordic=true`, and the
  # R2025 models only with `-f r2025=true`.
  if [ ! -f "$MODELS/$rvt" ]; then
    echo "skipping $name: $rvt absent"
    continue
  fi
  rank=$(stat -c %s "$MODELS/$rvt")
  [ "$ref" = "-" ] || rank=$(( rank + 1000000000000 ))
  items="$items
$rank|measure_model|$name|$rvt|$ref"
done <<< "$MODELS_LIST"
if [ -d "${FAMILY_DIR:-}" ]; then
  for family in "$FAMILY_DIR"/*.rfa; do
    [ -f "$family" ] || continue
    items="$items
$(stat -c %s "$family")|measure_family|$family||"
  done
fi
if [ -d "${SAMPLE_DIR:-}" ]; then
  for sample in "$SAMPLE_DIR"/*.rvt; do
    [ -f "$sample" ] || continue
    items="$items
$(stat -c %s "$sample")|measure_sample|$sample||"
  done
fi
items=$(printf '%s\n' "$items" | sort -t'|' -k1,1nr)
while IFS='|' read -r _ run first second _; do
  case "$run" in
    measure_model) export_ifc "$MODELS/$second" "$(model_dir "$first")" ;;
    measure_sample) export_ifc "$first" "$(sample_dir "$first")" ;;
  esac
done <<< "$items"
while IFS='|' read -r _ run first second third; do
  [ -n "$run" ] || continue
  spawn "$run" "$first" "$second" "$third"
done <<< "$items"
wait
if [ -n "${PROBE:-}" ] && [ -x "$BIN/examples/$PROBE" ]; then
  while IFS='|' read -r _ run first second third; do
    case "$run" in
      measure_model) probe_model "$first" "$second" "$third" ;;
      measure_family) probe_family "$first" ;;
      measure_sample) probe_sample "$first" ;;
    esac
  done <<< "$items"
fi
