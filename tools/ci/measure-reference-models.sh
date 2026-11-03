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
#   - revit_property_sets_vs_ifc.txt, where Revit's export with its own
#     property sets is beside the model as <model>.revit-sets.ifc: an export
#     with `--revit-property-sets` (its log rvt-ifc-revit-sets.log) scored
#     against it (RE-210);
#   - timings.log: each step's wall time in milliseconds, taken while other
#     steps run beside it (the export's is in rvt-ifc.log);
#   - info.json and info.exit: `rvt-info --redact --json` on it (the identity,
#     the document's Atom entry, the streams) and its exit status. The Atom
#     entry goes through the PartAtom parser, from the model's
#     ProjectInformation stream. The IFC export reads only a PartAtom stream,
#     which none of these models has (diagnostics.json says has_part_atom
#     false), so this, and the families below, are what measure that parser;
#   - probe.txt, when PROBE names a probe (the rvt-probes crate's
#     examples) already built in BIN_DIR/examples: its output on the model, ending with its exit status;
#   - probe_score.txt, when PROBE_SCORER also names a scorer in
#     PROBE_SCORER_DIR taking <probe output> <revit-export.ifc>.
# PROBE may name several probes and PROBE_SCORER a scorer for each, in the
# same order ("-" for none); each probe then writes probe-<probe>.txt and
# probe_score-<probe>.txt.
# MEASURE_MODELS, when set, keeps only the models whose names match one of
# its patterns (e.g. "nordic-office-* core-interior"). MEASURE_SCORERS, when
# set, runs on the models only the steps it names: tools/re scorers, and
# info, gltf, schema and witness ("none" for none of them; rvt-gltf runs
# when a .glb scorer or gltf is named). A tools/re script it names that is
# in neither scorer list runs as an IFC scorer, so a research lister needs no
# place in every run. Every model is still exported and hashed.
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
# These runs check what the exports hold; their times are not benchmarks
# (the workflow's `timing` job times both refs on one runner,
# tools/ci/measure-timing.sh). Every export runs first, MEASURE_JOBS at once,
# by default one per CPU; then every step of every model (each scorer, the
# schema check, the glTF export and its scorers, the witness exports) runs as
# a task of its own, MEASURE_JOBS at once, the heaviest first, while at least
# MEASURE_MIN_FREE_KB of memory is free (by default 3 GiB); then every probe,
# alone, one at a time.
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
nordic-office-terrain-concrete|ARK_NordicLCA_Office_Terrain_Concrete_BuildingPermit.rvt|ARK_NordicLCA_Office_Terrain_Concrete_BuildingPermit_Revit.ifc
nordic-office-terrain-timber|ARK_NordicLCA_Office_Terrain_Timber_BuildingPermit.rvt|ARK_NordicLCA_Office_Terrain_Timber_BuildingPermit_Revit.ifc
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
# The held-out models (research/holdout-models.tsv, RE-252, and the archived
# ones research/holdout-archive-models.tsv lists, #408, #1261), only with
# `-f holdout=true`, each scored against Revit's export beside it; the workflow
# keeps only their counts (tools/ci/scrub-holdout-results.sh).
if [ -d "$MODELS/holdout" ]; then
  for model in "$MODELS"/holdout/*.rvt; do
    [ -f "$model" ] || continue
    stem=$(basename "$model" .rvt)
    [ -f "$MODELS/holdout/$stem.ifc" ] || { echo "holdout-$stem: no Revit export beside it, not measured" >&2; continue; }
    MODELS_LIST="$MODELS_LIST
holdout-$stem|holdout/$stem.rvt|holdout/$stem.ifc"
  done
fi
# Pairs fetched with them that are no longer held out (research/holdout-archive-models.tsv
# names them under spent/), measured and uploaded in full.
if [ -d "$MODELS/spent" ]; then
  for model in "$MODELS"/spent/*.rvt; do
    [ -f "$model" ] || continue
    stem=$(basename "$model" .rvt)
    [ -f "$MODELS/spent/$stem.ifc" ] || { echo "spent-$stem: no Revit export beside it, not measured" >&2; continue; }
    MODELS_LIST="$MODELS_LIST
spent-$stem|spent/$stem.rvt|spent/$stem.ifc"
  done
fi
# Scorers taking <rvt-rs.ifc> <revit-export.ifc>, and <model.glb> <revit-export.ifc>.
IFC_SCORERS="class_sets_vs_ifc aggregates_vs_ifc global_ids_vs_ifc names_vs_ifc storeys_vs_ifc
room_names_vs_ifc type_objects_vs_ifc layer_sets_vs_ifc element_materials_vs_ifc
material_colours_vs_ifc openings_vs_ifc opening_hosts_vs_ifc opening_boxes_vs_ifc
door_bodies_vs_ifc door_window_sizes_vs_ifc oriented_boxes_vs_ifc plan_profiles_vs_ifc
pipe_bodies_vs_ifc duct_bodies_vs_ifc roof_bodies_vs_ifc stair_bodies_vs_ifc ports_vs_ifc psets_vs_ifc qtos_vs_ifc saved_meshes_vs_ifc tagged_spaces_vs_ifc port_places_vs_ifc predefined_types_vs_ifc box_bodies_vs_ifc column_bodies_vs_ifc mesh_bodies_vs_ifc wall_ends_list wall_heights_vs_ifc frames_vs_ifc space_profiles_vs_ifc space_overlaps_vs_ifc"
GLB_SCORERS="wall_bodies_vs_ifc wall_ends_cv_vs_ifc wall_layers_vs_ifc slab_layers_vs_ifc glb_material_colours_vs_ifc"

read -ra MODEL_PATTERNS <<< "${MEASURE_MODELS:-}"
read -ra STEPS <<< "${MEASURE_SCORERS:-}"
wanted_model() { # name
  local pattern
  [ "${#MODEL_PATTERNS[@]}" -gt 0 ] || return 0
  for pattern in "${MODEL_PATTERNS[@]}"; do
    # shellcheck disable=SC2254 # each is a pattern
    case "$1" in $pattern) return 0 ;; esac
  done
  return 1
}
wanted() { # step
  local step
  [ "${#STEPS[@]}" -gt 0 ] || return 0
  for step in "${STEPS[@]}"; do
    [ "$step" = "$1" ] && return 0
  done
  return 1
}
# Whether a step is in a scorer list or is one of the steps run by name.
listed() { # step
  local s
  for s in $IFC_SCORERS $GLB_SCORERS info gltf schema witness revit_property_sets_vs_ifc none; do
    [ "$s" = "$1" ] && return 0
  done
  return 1
}
wanted_glb() {
  local s
  wanted gltf && return 0
  for s in $GLB_SCORERS; do
    wanted "$s" && return 0
  done
  return 1
}

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

# A step's time limit. Steps share the runner's cores, and under that load the
# office's mesh_bodies_vs_ifc took 940 s against 418 s alone (#1285); runners
# differ up to 1.7 times in speed, so the limit leaves twice that and more.
STEP_TIMEOUT="${MEASURE_STEP_TIMEOUT:-2400}"
score() { # name args...
  local out="$1" dir step; shift
  dir=$(dirname "$out"); step=$(basename "$out" .txt)
  timed "$dir" "$step" timeout "$STEP_TIMEOUT" python "$@" > "$out" 2>&1
  echo "exit $?" >> "$out"
}

info() { # file dir
  "$BIN/rvt-info" "$1" --redact --json > "$2/info.json" 2> "$2/info.log"
  echo "exit $?" > "$2/info.exit"
}

# C9: the IFC4 schema's view of dir/model.ifc in dir/schema.txt: attribute
# counts always, IfcOpenShell's full validation and its check of the schema's
# EXPRESS rules up to max_bytes, otherwise marked as skipped, and the standard property sets against IFC4's
# definitions, fetched once into PSET_DEFINITIONS (#822).
PSET_DEFINITIONS="${RUNNER_TEMP:-/tmp}/ifc4-pset-definitions.csv"
schema() { # dir max_bytes
  {
    echo "arity:"
    timed "$1" schema-arity timeout 600 python "$(dirname "$0")/ifc_schema_arity.py" "$1/model.ifc" 2>&1 \
      | grep -v '^IFC schema conformance passed\|^  instances checked' || true
    if [ "$(stat -c %s "$1/model.ifc")" -le "$2" ]; then
      echo "validate:"
      timed "$1" schema-validate timeout "$STEP_TIMEOUT" python "$(dirname "$0")/ifc_validate_summary.py" "$1/model.ifc" "$1/validation.jsonl" 2>&1
      echo "rules:"
      timed "$1" schema-rules timeout "$STEP_TIMEOUT" python "$(dirname "$0")/ifc_validate_summary.py" --rules "$1/model.ifc" 2>&1
    else
      echo "validate: skipped, over $(( $2 / 1048576 )) MB"
    fi
    echo "property sets:"
    timed "$1" schema-psets timeout "$STEP_TIMEOUT" python "$(dirname "$0")/ifc4_property_sets.py" "$1/model.ifc" \
      --definitions "$PSET_DEFINITIONS" 2>&1
  } > "$1/schema.txt"
}

# Every export runs before the steps that read it, several at once; its wall
# time in rvt-ifc.log is taken beside the others, so it is not a benchmark.
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

# A model's steps, one task each, as `<weight factor> <task>` lines: the
# factor, times the model's size, orders the tasks, the heaviest first, so the
# longest ones do not start last (the BIM4LCA office's mesh_bodies_vs_ifc took
# 477 s of its 1,953 run one after another, run 41589642386).
model_tasks() { # name rvt ref
  local name="$1" rvt="$2" ref="$3" s meshes=
  echo "1 base"
  # ports_vs_ifc reads the findings the schema step's validation wrote, so it
  # runs after it, in the same task.
  wanted schema && echo "5 schema"
  wanted witness && [ -n "$(witness_artifacts "$name")" ] && echo "2 witness"
  wanted_glb && echo "4 glb"
  if [ "$ref" != "-" ]; then
    for s in $IFC_SCORERS; do
      wanted "$s" || continue
      [ "$s" = ports_vs_ifc ] && wanted schema && continue
      case "$s" in
        # These two mesh the same bodies of the same two files, so they run
        # one after the other as one task and the second reads what the
        # first meshed.
        mesh_bodies_vs_ifc | saved_meshes_vs_ifc) meshes=1 ;;
        layer_sets_vs_ifc | wall_heights_vs_ifc | door_bodies_vs_ifc) echo "2 ifc:$s" ;;
        *) echo "1 ifc:$s" ;;
      esac
    done
    for s in "${STEPS[@]}"; do
      listed "$s" && continue
      [ -f "$SCORERS/$s.py" ] || continue
      echo "1 ifc:$s"
    done
  fi
  [ -n "$meshes" ] && echo "18 meshes"
  [ -f "$MODELS/${rvt%.rvt}.revit-sets.ifc" ] && wanted revit_property_sets_vs_ifc && echo "2 sets"
  return 0
}

model_task() { # name rvt ref task
  local name="$1" rvt="$2" ref="$3" task="$4" dir s artifact sets flags extra
  dir=$(model_dir "$name")
  case "$task" in
    base)
      wanted info && timed "$dir" rvt-info info "$MODELS/$rvt" "$dir"
      grep -v -E '^FILE_NAME\(|IFCOWNERHISTORY' "$dir/model.ifc" | sha256sum | cut -d' ' -f1 > "$dir/ifc.sha256"
      ;;
    schema)
      # A writer path only these models reach, such as the office files'
      # thermal transmittance unit (RE-194), is checked here.
      [ -s "$dir/model.ifc" ] && schema "$dir" 125829120
      if [ "$ref" != "-" ] && wanted ports_vs_ifc; then
        extra=()
        [ -f "$dir/validation.jsonl" ] && extra=(--validation "$dir/validation.jsonl")
        score "$dir/ports_vs_ifc.txt" "$SCORERS/ports_vs_ifc.py" "$dir/model.ifc" "$MODELS/$ref" "${extra[@]}"
      fi
      ;;
    witness)
      # The witness observations committed for this model (research/witness/),
      # regenerated so a change to its IFC can commit them from the results.
      for artifact in $(witness_artifacts "$name"); do
        timed "$dir" "witness-$artifact" "$BIN/rvt-ifc" "$MODELS/$rvt" -o "$dir/witness-$artifact.ifc" --observation "$dir/witness-$artifact.json" --artifact-id "$artifact" > /dev/null 2>&1
        rm -f "$dir/witness-$artifact.ifc"
      done
      ;;
    glb)
      timed "$dir" rvt-gltf "$BIN/rvt-gltf" "$MODELS/$rvt" -o "$dir/model.glb" > "$dir/rvt-gltf.log" 2>&1
      echo "exit $?" >> "$dir/rvt-gltf.log"
      if [ "$ref" != "-" ]; then
        for s in $GLB_SCORERS; do
          wanted "$s" || continue
          score "$dir/$s.txt" "$SCORERS/$s.py" "$dir/model.glb" "$MODELS/$ref"
        done
      fi
      ;;
    ifc:*)
      s=${task#ifc:}
      score "$dir/$s.txt" "$SCORERS/$s.py" "$dir/model.ifc" "$MODELS/$ref"
      ;;
    meshes)
      for s in saved_meshes_vs_ifc mesh_bodies_vs_ifc; do
        wanted "$s" || continue
        RVT_MESH_CACHE="$dir/mesh-cache" score "$dir/$s.txt" "$SCORERS/$s.py" "$dir/model.ifc" "$MODELS/$ref"
      done
      echo "$name: mesh cache $(du -sh "$dir/mesh-cache" 2>/dev/null | cut -f1), removed"
      rm -rf "$dir/mesh-cache"
      ;;
    sets)
      # RE-210: where Revit's export with its own property sets is beside the
      # model (<model>.revit-sets.ifc, the R2025 models), an export with
      # `--revit-property-sets` scored against it.
      sets="${rvt%.rvt}.revit-sets.ifc"
      case " ${RVT_IFC_FLAGS:-} " in
        *" --revit-property-sets "*) flags=() ;;
        *) flags=(--revit-property-sets) ;;
      esac
      # shellcheck disable=SC2086 # RVT_IFC_FLAGS is a list of flags.
      timed "$dir" rvt-ifc-revit-sets "$BIN/rvt-ifc" "$MODELS/$rvt" -o "$dir/revit-sets.ifc" ${RVT_IFC_FLAGS:-} "${flags[@]}" \
        > "$dir/rvt-ifc-revit-sets.log" 2>&1
      echo "exit $?" >> "$dir/rvt-ifc-revit-sets.log"
      score "$dir/revit_property_sets_vs_ifc.txt" "$SCORERS/revit_property_sets_vs_ifc.py" "$dir/revit-sets.ifc" "$MODELS/$sets" --list
      rm -f "$dir/revit-sets.ifc"
      ;;
  esac
}

# After every task of a model: the bulky outputs stay out of the uploaded
# results, and the log gets the model's group with the sum of its steps.
model_done() { # name
  local dir ms
  dir=$(model_dir "$1")
  rm -rf "$dir/mesh-cache"
  rm -f "$dir/model.glb" "$dir/validation.jsonl"
  [ -n "${KEEP_IFC:-}" ] || rm -f "$dir/model.ifc"
  ms=$(awk '{ms += $2} END {print ms + 0}' "$dir/timings.log" 2>/dev/null || echo 0)
  printf '::group::%s\n%s ms in its steps\n::endgroup::\n' "$1" "$ms"
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
read -ra PROBES <<< "${PROBE:-}"
read -ra PROBE_SCORERS <<< "${PROBE_SCORER:-}"
# A run of one probe writes probe.txt (probe_score.txt, probe-pair.txt), a
# run of several probe-<probe>.txt (probe_score-<probe>.txt, ...).
probe_out() { # dir stem probe
  if [ "${#PROBES[@]}" -gt 1 ]; then echo "$1/$2-$3.txt"; else echo "$1/$2.txt"; fi
}
probe() { # probe dir out args...
  local name="$1" dir="$2" out="$3" step; shift 3
  step=$(basename "$out" .txt)
  timed "$dir" "$step" timeout 1200 "$BIN/examples/$name" "$@" > "$out" 2>&1
  echo "exit $?" >> "$out"
}
probe_model() { # probe scorer name rvt ref
  local dir scorer="${PROBE_SCORER_DIR:-$SCORERS}/$2.py"
  dir=$(model_dir "$3")
  probe "$1" "$dir" "$(probe_out "$dir" probe "$1")" "$MODELS/$4"
  if [ "$5" != "-" ] && [ "$2" != "-" ] && [ -f "$scorer" ]; then
    score "$(probe_out "$dir" probe_score "$1")" "$scorer" "$(probe_out "$dir" probe "$1")" "$MODELS/$5" --list
  fi
}
probe_family() { # probe family
  local dir
  dir=$(family_dir "$2")
  probe "$1" "$dir" "$(probe_out "$dir" probe "$1")" "$2"
}
probe_sample() { # probe sample
  local sample="$2" dir copy
  dir=$(sample_dir "$sample")
  probe "$1" "$dir" "$(probe_out "$dir" probe "$1")" "$sample" --records
  if [ -n "${PROBE_PAIR_RELEASE:-}" ] && [ "$(basename "$sample" | cut -c1-4)" != "$PROBE_PAIR_RELEASE" ]; then
    for copy in "$SAMPLE_DIR/$PROBE_PAIR_RELEASE"-*.rvt; do
      [ -f "$copy" ] && [ "$(pair_key "$copy")" = "$(pair_key "$sample")" ] || continue
      probe "$1" "$dir" "$(probe_out "$dir" probe-pair "$1")" "$sample" "$copy"
    done
  fi
}

# Every export runs first; then every model's steps, each a task of its own,
# and every family and sample item; MEASURE_JOBS at once (by default one per
# CPU), the heaviest first: the models scored against Revit's export, then the
# rest, by file size times the step's weight; then every probe, alone.
JOBS="${MEASURE_JOBS:-$(nproc)}"
MIN_FREE_KB="${MEASURE_MIN_FREE_KB:-3145728}"
free_kb() { awk '/^MemAvailable:/ {print $2}' /proc/meminfo; }
spawn() { # command...: run it once fewer than JOBS are running, and while
  # nothing runs or MIN_FREE_KB of memory is free: two office scorers
  # meshing an 82 MB IFC each beside two more could fill the runner's 16 GB.
  while [ "$(jobs -rp | wc -l)" -ge "$JOBS" ] \
    || { [ "$(jobs -rp | wc -l)" -gt 0 ] && [ "$(free_kb)" -lt "$MIN_FREE_KB" ]; }; do
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
  wanted_model "$name" || continue
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
    measure_model) spawn export_ifc "$MODELS/$second" "$(model_dir "$first")" ;;
    measure_sample) spawn export_ifc "$first" "$(sample_dir "$first")" ;;
  esac
done <<< "$items"
wait
tasks=""
while IFS='|' read -r rank run first second third; do
  case "$run" in
    measure_model)
      while read -r factor task; do
        tasks="$tasks
$(( rank * factor ))|model_task|$first|$second|$third|$task"
      done < <(model_tasks "$first" "$second" "$third")
      ;;
    measure_sample) tasks="$tasks
$(( rank * 3 ))|measure_sample|$first|||" ;;
    measure_family) tasks="$tasks
$rank|measure_family|$first|||" ;;
  esac
done <<< "$items"
tasks=$(printf '%s\n' "$tasks" | sort -t'|' -k1,1nr)
while IFS='|' read -r _ run first second third task; do
  case "$run" in
    model_task) spawn model_task "$first" "$second" "$third" "$task" ;;
    measure_sample | measure_family) spawn "$run" "$first" ;;
  esac
done <<< "$tasks"
wait
while IFS='|' read -r _ run first _; do
  [ "$run" = measure_model ] && model_done "$first"
done <<< "$items"
for i in "${!PROBES[@]}"; do
  name=${PROBES[$i]}
  scorer=${PROBE_SCORERS[$i]:--}
  [ -x "$BIN/examples/$name" ] || continue
  while IFS='|' read -r _ run first second third; do
    case "$run" in
      measure_model) probe_model "$name" "$scorer" "$first" "$second" "$third" ;;
      measure_family) probe_family "$name" "$first" ;;
      measure_sample) probe_sample "$name" "$first" ;;
    esac
  done <<< "$items"
done
