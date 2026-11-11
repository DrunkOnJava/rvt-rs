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
# tools/ci/fetch-bimcollab.sh fetched it, and Autodesk's rac_advanced samples
# of 2015 to 2020 with their IFC2X3 export in its racadv/ folder when
# tools/ci/fetch-rac-advanced-pair.sh fetched them. For
# each model this writes OUT_DIR/<model>/:
#   - model.ifc, when KEEP_IFC is set: the exported IFC itself;
#   - ifc.sha256: the exported IFC's hash without its FILE_NAME and
#     IFCOWNERHISTORY lines, which carry the export time, so two builds
#     that write the same model hash the same;
#   - schema.txt: the IFC4 schema's findings on the export (C9), its
#     attribute counts and IfcOpenShell's validation up to 120 MB, and its
#     standard property sets against IFC4's definitions (#822); a run whose
#     scorers leave `schema` out writes none, for a sample as for a model;
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
#   - mcp_answers_vs_ifc.txt and mcp-dump.json, when BIN holds rvt-rs-mcp (#1327):
#     what the MCP server answers about the model, scored against the model's
#     export (a difference is a failure, exit 1) and against Revit's export
#     (tools/re/mcp_answers_vs_ifc.py); mcp-dump.json is the answers as the
#     deterministic semantic dump (tools/re/mcp_client.py), which the semantic
#     merge check (#1334) reads. Of a model that is kept as counts only, the
#     lines starting `mcp answers:` leave the runner. With
#     MCP_REQUIRE_FOLDED_FIELD set (the workflow sets it on the head side), the
#     scorer is run with --require-folded-field: diagnostics without
#     `exported.body_pieces_folded` (#1424) are a difference. With
#     MCP_REQUIRE_SPACE_KIND set (the same side) it is run with
#     --require-space-kind: a room with no `space_kind` in the answer or the IFC,
#     and a summary with no `space_kinds` (#1437), are a difference;
#   - space_kinds_vs_ifc.txt, on every model and every sample whose export is not
#     empty (#1437): the check (tools/re/space_kinds_vs_ifc.py) that each
#     `IfcSpace` carries `RvtElementRecordGeometry.SpaceKind`, that `area` holds
#     exactly where `SpaceOutlineSource` is an Area value, and how many spaces
#     hold each kind, ending with its exit status, 1 on any space without a
#     kind or any disagreement. Needs no Revit export. The workflow's compare
#     job sums these over the run (`--total`) and fails on a model with no
#     result, with a failure count, or whose spaces are not the number its own
#     export counted, and on a geometry export that lost its outline source; of
#     a model kept as counts only, the lines starting `space kinds:` leave the
#     runner;
#   - probe.txt, when PROBE names a probe (the rvt-probes crate's
#     examples) already built in BIN_DIR/examples: its output on the model, ending with its exit status;
#   - probe_score.txt, when PROBE_SCORER also names a scorer in
#     PROBE_SCORER_DIR taking <probe output> <revit-export.ifc>.
# PROBE may name several probes and PROBE_SCORER a scorer for each, in the
# same order ("-" for none); each probe then writes probe-<probe>.txt and
# probe_score-<probe>.txt.
# MEASURE_MODELS, when set, keeps only the models whose names match one of
# its patterns (e.g. "nordic-office-* core-interior"); MEASURE_SKIP, when set,
# leaves out the models whose names match one of its patterns, which is how
# the workflow's shards split the models between runners. MEASURE_SCORERS, when
# set, runs on the models only the steps it names: tools/re scorers, and
# info, gltf, schema and witness ("none" for none of them; rvt-gltf runs
# when a .glb scorer or gltf is named). A tools/re script it names that is
# in neither scorer list runs as an IFC scorer, so a research lister needs no
# place in every run. Every model is still exported and hashed.
# When FAMILY_DIR names the Autodesk family corpus, each of its .rfa files gets
# OUT_DIR/family-<file>/ with the same info.json and info.exit (a family
# carries its Atom entry in a PartAtom stream) and, when PROBE is built, the
# same probe.txt, and its default export hashed as a model's is (ifc.sha256,
# beside the export's diagnostics.json and rvt-ifc.log), so a change that moves
# what a family writes shows in the comparison of the two sides. A family has no
# Revit export, so nothing else is measured on it.
# When AREAS_ACROSS_SAVES is set and SAMPLE_DIR holds Autodesk's rac_advanced
# samples of 2024, 2025, 2026 and 2027, OUT_DIR/areas_across_saves.txt is the
# gate of #1261 and #1194 (Griffin, 2026-11-05, "Same-project gate"): the 25
# Areas of its 2025, 2026 and 2027 saves must equal its 2024 save's in
# GlobalId, number, name, storey, outline source, body area, top surface,
# height and lowest vertex (tools/re/areas_across_saves.py), ending with its
# exit status, which the workflow turns into a failed job. With its 2021 and
# 2022 samples, OUT_DIR/areas_across_saves_2021.txt holds the 2021 save to the
# 2022 save the same way (#1434; a shard with neither save writes no such file).
# Each runs once every export is done and
# before any sample's IFC is deleted. When a run holds fewer than a gate's
# saves (a narrower sample_years, or the other shard) its file says `skipped:`,
# names the years missing, and the workflow prints a notice: nothing was
# checked. A run with ifc_flags is skipped the same way, since the gate is about
# the default export.
# SAMPLE_DIR does the same for Autodesk's sample projects (.rvt), adding the
# export's diagnostics.json, rvt-ifc.log and ifc.sha256 (hashed as a model's
# is) under OUT_DIR/sample-<file>/, and element_stage_counts.txt, the check
# (tools/re/element_stage_counts.py, #1389) that a Revit 2008 to 2023 export's
# element-record stage counts add up, ending with its exit status. When
# PROBE_PAIR_RELEASE names a release, the probe also runs on each sample of
# another release and the same project saved by that one, the sample first,
# into probe-pair.txt, ending with its exit status: the cross-release
# comparisons of #740 and #742.
# When MEASURE_CACHE_DIR names a directory, a scorer or schema step whose every
# input is as it was in an earlier run takes that run's output
# (tools/ci/measure-cache.py, docs/maintaining.md); its `<step> cached` line
# stands for its time in timings.log, and OUT_DIR/score-cache-<shard>.log (written
# by the workflow) lists each hit and miss. Exports, rvt-info, rvt-gltf, the
# witness exports and probes always run, and the held-out models, BIMcollab's
# and the pairs once held out (holdout-*, bimcollab-*, spent-*), of which only
# counts leave the runner, are never looked up or stored: the Actions cache is
# not private to the workflows that may change it.
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
nordic-office-timber-as-built|ARK_NordicLCA_Office_Timber_As-Built.rvt|ARK_NordicLCA_Office_Timber_As-Built_Revit.ifc
nordic-office-timber-permit|ARK_NordicLCA_Office_Timber_BuildingPermit.rvt|ARK_NordicLCA_Office_Timber_BuildingPermit_Revit.ifc
nordic-housing-concrete-as-built|ARK_NordicLCA_Housing_Concrete_As-Built.rvt|ARK_NordicLCA_Housing_Concrete_As-Built_Revit.ifc
nordic-housing-concrete-permit|ARK_NordicLCA_Housing_Concrete_BuildingPermit.rvt|ARK_NordicLCA_Housing_Concrete_BuildingPermit_Revit.ifc
nordic-housing-timber-as-built|ARK_NordicLCA_Housing_Timber_As-Built.rvt|ARK_NordicLCA_Housing_Timber_As-Built_Revit.ifc
nordic-housing-timber-permit|ARK_NordicLCA_Housing_Timber_BuildingPermit.rvt|ARK_NordicLCA_Housing_Timber_BuildingPermit_Revit.ifc
nordic-housing-terrain-concrete|ARK_NordicLCA_Housing_Terrain-Concrete_BuildingPermit.rvt|ARK_NordicLCA_Housing_Terrain-Concrete_BuildingPermit_Revit.ifc
nordic-housing-terrain-timber|ARK_NordicLCA_Housing_Terrain-Timber_BuildingPermit.rvt|ARK_NordicLCA_Housing_Terrain-Timber_BuildingPermit_Revit.ifc
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
# Autodesk's rac_advanced sample of 2015 to 2020, each scored against the one
# IFC2X3 export of the project (research/rac-advanced-export.tsv, #1354), only
# with `-f racadv=true`. The export is counted apart as IFC2X3 everywhere, these
# models are never looked up in or stored to the score cache, and only their
# summary counts leave the runner (tools/ci/scrub-holdout-results.sh).
if [ -d "$MODELS/racadv" ]; then
  for model in "$MODELS"/racadv/*.rvt; do
    [ -f "$model" ] || continue
    stem=$(basename "$model" .rvt)
    [ -f "$MODELS/racadv/rac_advanced_sample_project.ifc" ] || { echo "racadv: no export beside the models, not measured" >&2; break; }
    MODELS_LIST="$MODELS_LIST
racadv-${stem%%-*}|racadv/$stem.rvt|racadv/rac_advanced_sample_project.ifc"
  done
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
door_bodies_vs_ifc door_window_sizes_vs_ifc door_axes_vs_ifc oriented_boxes_vs_ifc plan_profiles_vs_ifc
pipe_bodies_vs_ifc duct_bodies_vs_ifc roof_bodies_vs_ifc stair_bodies_vs_ifc ports_vs_ifc psets_vs_ifc qtos_vs_ifc saved_meshes_vs_ifc tagged_spaces_vs_ifc port_places_vs_ifc predefined_types_vs_ifc box_bodies_vs_ifc column_bodies_vs_ifc mesh_bodies_vs_ifc wall_ends_list wall_heights_vs_ifc frames_vs_ifc space_profiles_vs_ifc space_overlaps_vs_ifc
unfilled_openings_vs_ifc"
GLB_SCORERS="wall_bodies_vs_ifc wall_ends_cv_vs_ifc wall_layers_vs_ifc slab_layers_vs_ifc glb_material_colours_vs_ifc"

read -ra MODEL_PATTERNS <<< "${MEASURE_MODELS:-}"
read -ra SKIP_PATTERNS <<< "${MEASURE_SKIP:-}"
read -ra STEPS <<< "${MEASURE_SCORERS:-}"
wanted_model() { # name
  local pattern
  for pattern in "${SKIP_PATTERNS[@]}"; do
    # shellcheck disable=SC2254 # each is a pattern
    case "$1" in $pattern) return 1 ;; esac
  done
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
  for s in $IFC_SCORERS $GLB_SCORERS info gltf schema witness revit_property_sets_vs_ifc mcp_answers_vs_ifc space_kinds_vs_ifc bodies_across_saves none; do
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
  local out="$1" dir step status; shift
  dir=$(dirname "$out"); step=$(basename "$out" .txt)
  timed "$dir" "$step" timeout "$STEP_TIMEOUT" python "$@" > "$out" 2>&1
  status=$?
  echo "exit $status" >> "$out"
  # A run that names its scorers is run for their answer, and a crash left in
  # the output file reads as a green run (#1541). In the default set a scorer
  # may exit 1 for a finding, so only the named ones are raised, and a probe's
  # scorer, which is always asked for by name.
  if [ "$status" != 0 ] && { [[ "$step" == probe_score* ]] || { [ "${#STEPS[@]}" -gt 0 ] && wanted "$step"; }; }; then
    echo "::error::$(basename "$dir"): the scorer $step, named in this run, exited $status (see its $step.txt)"
  fi
}

# The score cache (tools/ci/measure-cache.py), on when MEASURE_CACHE_DIR names
# its directory: a step whose every input is as it was in an earlier run takes
# that run's output, and `<step> cached` stands for its time in timings.log. A
# step is kept only if it finished with status 0, and the models whose results
# only leave the runner as counts (holdout-*, bimcollab-*, racadv-*), and the
# pairs that were once held out (spent-*), are never cached.
CACHE_TOOL="$(dirname "$0")/measure-cache.py"
cscore() { # name args...: `score`, from the cache when the scorer and its inputs are unchanged
  local out="$1" dir step found=2; shift
  dir=$(dirname "$out"); step=$(basename "$out" .txt)
  if [ -n "${MEASURE_CACHE_DIR:-}" ]; then
    python "$CACHE_TOOL" lookup --step "$step" --dir "$dir" --dest "$out" -- "$@"
    found=$?
    [ "$found" = 0 ] && return 0
  fi
  score "$out" "$@"
  [ "$found" = 1 ] && python "$CACHE_TOOL" store --step "$step" --dir "$dir" --dest "$out"
  return 0
}

# A schema step's output on stdout, the cache's when the script and the file
# are unchanged, else the step's own under its time limit. The files the step
# also writes are named by CACHE_EXTRA, and kept with the output.
cached() { # dir step limit script args...
  local dir="$1" step="$2" limit="$3" script="$4" tmp status found=2 extra=() path; shift 4
  tmp="${RUNNER_TEMP:-/tmp}/cached-$$-$(basename "$dir")-$step"
  for path in ${CACHE_EXTRA:-}; do extra+=(--extra "$path"); done
  if [ -n "${MEASURE_CACHE_DIR:-}" ]; then
    python "$CACHE_TOOL" lookup --step "$step" --dir "$dir" --dest "$tmp" ${extra[@]+"${extra[@]}"} -- "$script" "$@"
    found=$?
    if [ "$found" = 0 ]; then
      cat "$tmp"
      rm -f "$tmp"
      return 0
    fi
  fi
  timed "$dir" "$step" timeout "$limit" python "$script" "$@" > "$tmp" 2>&1
  status=$?
  cat "$tmp"
  [ "$found" = 1 ] && python "$CACHE_TOOL" store --step "$step" --dir "$dir" --dest "$tmp" --status "$status" ${extra[@]+"${extra[@]}"}
  rm -f "$tmp"
  return "$status"
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
    cached "$1" schema-arity 600 "$(dirname "$0")/ifc_schema_arity.py" "$1/model.ifc" \
      | grep -v '^IFC schema conformance passed\|^  instances checked' || true
    if [ "$(stat -c %s "$1/model.ifc")" -le "$2" ]; then
      # The validation and the EXPRESS rules read one parse of the file; the
      # rules' summary comes back in a file, to follow the validation's.
      # (validation.jsonl is kept for ports_vs_ifc, which a sample has none of,
      # and which measure_sample deletes.)
      local extras="$1/validation.jsonl $1/schema-rules.part"
      case "$(basename "$1")" in sample-*) extras="$1/schema-rules.part" ;; esac
      echo "validate:"
      CACHE_EXTRA="$extras" cached "$1" schema-validate-rules $(( 2 * STEP_TIMEOUT )) \
        "$(dirname "$0")/ifc_validate_summary.py" --both "$1/model.ifc" "$1/validation.jsonl" "$1/schema-rules.part"
      echo "rules:"
      cat "$1/schema-rules.part" 2> /dev/null || true
      rm -f "$1/schema-rules.part"
    else
      echo "validate: skipped, over $(( $2 / 1048576 )) MB"
    fi
    echo "property sets:"
    cached "$1" schema-psets "$STEP_TIMEOUT" "$(dirname "$0")/ifc4_property_sets.py" "$1/model.ifc" \
      --definitions "$PSET_DEFINITIONS"
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
  # #1327: the MCP server's answers need the model's export and its .rvt, not
  # Revit's export, so a model with none (Einhoven) is scored for consistency.
  if wanted mcp_answers_vs_ifc; then
    if [ -x "$BIN/rvt-rs-mcp" ]; then
      echo "6 mcp"
    else
      echo "$name: no rvt-rs-mcp in $BIN (a ref older than the crate), mcp_answers_vs_ifc not measured" >&2
    fi
  fi
  # #1437: each IfcSpace says what it is. Reads the export alone, so a model
  # with no Revit export (Einhoven) is checked too.
  wanted space_kinds_vs_ifc && echo "1 kinds"
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
  local name="$1" rvt="$2" ref="$3" task="$4" dir s artifact sets flags extra dump
  dir=$(model_dir "$name")
  case "$task" in
    base)
      wanted info && timed "$dir" rvt-info info "$MODELS/$rvt" "$dir"
      grep -v -E '^FILE_NAME\(|IFCOWNERHISTORY' "$dir/model.ifc" | sha256sum | cut -d' ' -f1 > "$dir/ifc.sha256"
      # RE-348, RE-349: how many walls take the wall base and end rules, and
      # why the others do not, as counts (the diagnostics are not uploaded for
      # a model without a licence to redistribute it; these lines are).
      [ -s "$dir/diagnostics.json" ] \
        && python "$SCORERS/wall_rule_counts.py" "$dir/diagnostics.json" > "$dir/wall_rules.txt" 2>&1 || true
      ;;
    schema)
      # A writer path only these models reach, such as the office files'
      # thermal transmittance unit (RE-194), is checked here.
      [ -s "$dir/model.ifc" ] && schema "$dir" 125829120
      if [ "$ref" != "-" ] && wanted ports_vs_ifc; then
        extra=()
        [ -f "$dir/validation.jsonl" ] && extra=(--validation "$dir/validation.jsonl")
        cscore "$dir/ports_vs_ifc.txt" "$SCORERS/ports_vs_ifc.py" "$dir/model.ifc" "$MODELS/$ref" "${extra[@]}"
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
          cscore "$dir/$s.txt" "$SCORERS/$s.py" "$dir/model.glb" "$MODELS/$ref"
        done
      fi
      ;;
    ifc:*)
      s=${task#ifc:}
      # A research lister named in MEASURE_SCORERS, in neither list, is not cached.
      if listed "$s"; then
        cscore "$dir/$s.txt" "$SCORERS/$s.py" "$dir/model.ifc" "$MODELS/$ref"
      else
        score "$dir/$s.txt" "$SCORERS/$s.py" "$dir/model.ifc" "$MODELS/$ref"
      fi
      ;;
    kinds)
      score "$dir/space_kinds_vs_ifc.txt" "$SCORERS/space_kinds_vs_ifc.py" "$dir/model.ifc"
      ;;
    mcp)
      # The server decodes the model again and answers the fixed query set from
      # it (tools/re/mcp_client.py): a program run, so never from the score cache.
      extra=()
      [ "$ref" != "-" ] && extra=("$MODELS/$ref")
      # The dump (GlobalIds, names, values) is for the semantic merge check
      # (#1334), which reads openly licensed models only: it is written for
      # the models named here, and for no other (not the held-out, spent,
      # BIMcollab or rac_advanced ones, nor a model added later).
      dump=()
      case "$name" in
        core-interior | einhoven | re1-* | nordic-housing-* | nordic-office-as-built | nordic-office-permit | nordic-office-terrain-* | nordic-office-timber-* | r2025-*)
          dump=(--dump "$dir/mcp-dump.json")
          ;;
      esac
      require=()
      [ -n "${MCP_REQUIRE_FOLDED_FIELD:-}" ] && require=(--require-folded-field)
      [ -n "${MCP_REQUIRE_SPACE_KIND:-}" ] && require+=(--require-space-kind)
      score "$dir/mcp_answers_vs_ifc.txt" "$SCORERS/mcp_answers_vs_ifc.py" \
        --server "$BIN/rvt-rs-mcp" --rvt "$MODELS/$rvt" \
        --diagnostics "$dir/diagnostics.json" ${dump[@]+"${dump[@]}"} \
        ${require[@]+"${require[@]}"} \
        "$dir/model.ifc" ${extra[@]+"${extra[@]}"}
      ;;
    meshes)
      for s in saved_meshes_vs_ifc mesh_bodies_vs_ifc; do
        wanted "$s" || continue
        RVT_MESH_CACHE="$dir/mesh-cache" cscore "$dir/$s.txt" "$SCORERS/$s.py" "$dir/model.ifc" "$MODELS/$ref"
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
      cscore "$dir/revit_property_sets_vs_ifc.txt" "$SCORERS/revit_property_sets_vs_ifc.py" "$dir/revit-sets.ifc" "$MODELS/$sets" --list
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
  # The default export of the family (a standalone revolution's body, a family's
  # type table), hashed as a model's is, so the comparison of the two sides lists
  # a family whose export moved.
  timed "$dir" rvt-ifc export_ifc "$1" "$dir"
  if [ -s "$dir/model.ifc" ]; then
    grep -v -E '^FILE_NAME\(|IFCOWNERHISTORY' "$dir/model.ifc" | sha256sum | cut -d' ' -f1 > "$dir/ifc.sha256"
  fi
  [ -n "${KEEP_IFC:-}" ] || rm -f "$dir/model.ifc"
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
# RE-341, RE-345, RE-346: bodies_across_saves.py holds the 2025, 2026 and 2027 saves
# of the five projects below to their earlier 2024 and 2025 saves, so those saves
# keep their exports until it has run, after every sample item.
# Whether this run's exports take a flag. A value of only spaces is no flag:
# export_ifc passes it unquoted, so rvt-ifc gets none.
flagged_export() {
  [[ ${RVT_IFC_FLAGS:-} == *[![:space:]]* ]]
}
saves_gate_keeps() { # sample
  wanted bodies_across_saves || return 1
  flagged_export && return 1
  case "$(basename "$1" | cut -c1-4)" in 2024 | 2025 | 2026 | 2027) ;; *) return 1 ;; esac
  case "$(pair_key "$1")" in racadvanced | racbasic | rmeadvanced | rmebasic | rstbasic) return 0 ;; esac
  return 1
}
measure_sample() { # sample
  local sample="$1" dir item_started copy
  item_started=$(date +%s%N)
  dir=$(sample_dir "$sample"); mkdir -p "$dir"
  timed "$dir" rvt-info info "$sample" "$dir"
  # #1389: the stage counts of a Revit 2008 to 2023 export add up, and an export
  # that wrote no record says where its records stopped. Reads the diagnostics
  # alone, so it runs for every sample, whatever its release.
  if [ -s "$dir/diagnostics.json" ]; then
    score "$dir/element_stage_counts.txt" "$SCORERS/element_stage_counts.py" "$dir/diagnostics.json"
    # RE-348, RE-349: how many walls take the wall base and end rules.
    python "$SCORERS/wall_rule_counts.py" "$dir/diagnostics.json" > "$dir/wall_rules.txt" 2>&1 || true
  fi
  # C9: the IFC4 schema's view of the export, so a writer path only the
  # samples reach (I-shape profiles and profile set usages on the
  # structural samples) is checked somewhere. Attribute counts on every
  # export; IfcOpenShell's full validation on exports up to 120 MB, as
  # for a reference model, the rest marked as skipped. A run that names its
  # scorers without `schema` (a probe run, `scorers=none`) skips it, as a
  # reference model does: the EXPRESS rules alone took 170 to 220 s of a large
  # MEP sample and set the length of such a run's longest shard (run
  # 42987216721).
  if [ -s "$dir/model.ifc" ]; then
    wanted schema && schema "$dir" 125829120
    grep -v -E '^FILE_NAME\(|IFCOWNERHISTORY' "$dir/model.ifc" | sha256sum | cut -d' ' -f1 > "$dir/ifc.sha256"
    # #1437: every space of the sample's export says what it is.
    wanted space_kinds_vs_ifc && score "$dir/space_kinds_vs_ifc.txt" "$SCORERS/space_kinds_vs_ifc.py" "$dir/model.ifc"
  fi
  if [ -z "${KEEP_IFC:-}" ] && ! saves_gate_keeps "$sample"; then rm -f "$dir/model.ifc"; fi
  rm -f "$dir/validation.jsonl"
  finished "$dir" "$(basename "$sample")" "$item_started"
}

# RE-341, RE-345, RE-346: each 2025, 2026 and 2027 save of rac_advanced, rac_basic,
# rme_advanced, rme_basic and rst_basic is held to the same project's earlier 2024
# and 2025 saves, whose exports the samples kept (tools/re/bodies_across_saves.py).
# It writes bodies_across_saves.txt
# beside the sample, ending with its exit status; the Measure workflow turns a
# failure on the head side into a red run (tools/ci/check-bodies-across-saves.sh).
bodies_across_saves() {
  local candidate earlier sample dir references
  if ! wanted bodies_across_saves; then
    echo "::notice::bodies_across_saves skipped: the scorers asked for exclude it"
    return 0
  fi
  for candidate in "$SAMPLE_DIR"/202[567]-*.rvt; do
    [ -f "$candidate" ] || continue
    case "$(pair_key "$candidate")" in racadvanced | racbasic | rmeadvanced | rmebasic | rstbasic) ;; *) continue ;; esac
    references=()
    # A save is held to the project's saves of earlier years only.
    for earlier in "$SAMPLE_DIR"/202[45]-*.rvt; do
      [ -f "$earlier" ] && [ "$(pair_key "$earlier")" = "$(pair_key "$candidate")" ] || continue
      [ "$(basename "$earlier" | cut -c1-4)" -lt "$(basename "$candidate" | cut -c1-4)" ] || continue
      references+=("$(basename "$earlier" | cut -c1-4)=$(sample_dir "$earlier")/model.ifc")
    done
    dir=$(sample_dir "$candidate")
    # The check is about the default export: a flag such as --phase changes
    # what a save writes, or refuses a save that has no such phase.
    if flagged_export; then
      echo "::notice::$(basename "$dir"): this run's exports used ifc_flags ($RVT_IFC_FLAGS), so bodies_across_saves compared nothing"
      printf 'bodies across saves: SKIPPED (the exports of this run used ifc_flags: %s)\nexit 0\n' "$RVT_IFC_FLAGS" > "$dir/bodies_across_saves.txt"
      continue
    fi
    if [ "${#references[@]}" -eq 0 ]; then
      echo "::notice::$(basename "$dir"): no earlier 2024 or 2025 save of its project is in this run, so bodies_across_saves compared nothing"
      printf 'bodies across saves: SKIPPED (no earlier 2024 or 2025 save of the project is in this run)\nexit 0\n' > "$dir/bodies_across_saves.txt"
      continue
    fi
    # One scorer for each save at a time, up to the jobs the runner holds: the
    # MEP samples mesh some 6,600 bodies each, and a save's pipes, cuts, ports
    # and properties are read beside the bodies (RE-346).
    spawn score "$dir/bodies_across_saves.txt" "$SCORERS/bodies_across_saves.py" "$dir/model.ifc" "${references[@]}"
  done
  wait
  for sample in "$SAMPLE_DIR"/*.rvt; do
    [ -f "$sample" ] || continue
    if [ -z "${KEEP_IFC:-}" ] && saves_gate_keeps "$sample"; then rm -f "$(sample_dir "$sample")/model.ifc"; fi
  done
}

# The Areas of Autodesk's rac_advanced across its saves (the gates of the Areas
# of Revit 2025 to 2027, RE-333 section 9, and of its 2021 save, #1434): the
# later saves of a gate are held to its first. With every sample of a gate in
# the run, a save with no IFC is a failure, not a skipped check: the scorer is
# given every path and says which is missing. With fewer samples than that the
# gate is skipped, and says so.
areas_gate() { # result_file year... (the first year is the reference)
  local result="$1" year file missing="" args=()
  shift
  for year in "$@"; do
    file="$SAMPLE_DIR/$year-racadvancedsampleproject.rvt"
    [ -f "$file" ] || missing="$missing $year"
    args+=("$year=$(sample_dir "$file")/model.ifc")
  done
  if [ -n "$missing" ]; then
    echo "areas across saves: skipped: no rac_advanced sample of$missing in this run, so nothing was compared" > "$result"
    return 0
  fi
  {
    timeout "$STEP_TIMEOUT" python "$SCORERS/areas_across_saves.py" "${args[@]}"
    echo "exit $?"
  } > "$result" 2>&1
}
areas_across_saves() {
  # The gates are about the default export: a flag such as --phase changes what
  # a save writes, or refuses a save that has no such phase.
  if flagged_export; then
    local skipped="areas across saves: skipped: the exports of this run used ifc_flags ($RVT_IFC_FLAGS), so nothing was compared"
    echo "$skipped" > "$OUT/areas_across_saves.txt"
    echo "$skipped" > "$OUT/areas_across_saves_2021.txt"
    return 0
  fi
  areas_gate "$OUT/areas_across_saves.txt" 2024 2025 2026 2027
  # A shard that holds neither the 2021 nor the 2022 save writes no file for the
  # 2021 gate: two shards write one name, and the artifact the shards are merged
  # into keeps one of them, which must not be a placeholder over a result.
  if [ -f "$SAMPLE_DIR/2021-racadvancedsampleproject.rvt" ] || [ -f "$SAMPLE_DIR/2022-racadvancedsampleproject.rvt" ]; then
    areas_gate "$OUT/areas_across_saves_2021.txt" 2022 2021
  fi
}

# Every probe runs after the items, alone, one at a time: a probe is cut off
# after 20 minutes, and items running beside it would slow it by up to about
# 2x, so a probe would stop sooner than it always has.
read -ra PROBES <<< "${PROBE:-}"
read -ra PROBE_SCORERS <<< "${PROBE_SCORER:-}"
# A scorer that reads a probe's output, named in MEASURE_SCORERS, would be given
# the IFC as that output, find nothing in it and report zeros with success (#1505).
for s in "${STEPS[@]:-}"; do
  [ -n "$s" ] && [ -f "$SCORERS/$s.py" ] || continue
  listed "$s" && continue
  if grep -qE '\.py +<probe[- ._a-z0-9]*>' "$SCORERS/$s.py"; then
    echo "::error::$s reads a probe's output: dispatch it with -f probe=<probe> -f probe_scorer=$s, not in scorers" >&2
    exit 2
  fi
done
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
# Before any sample's IFC is deleted at the end of its measure_sample.
if [ -n "${AREAS_ACROSS_SAVES:-}" ] && [ -d "${SAMPLE_DIR:-}" ]; then
  areas_across_saves
fi
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
[ -d "${SAMPLE_DIR:-}" ] && bodies_across_saves
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
