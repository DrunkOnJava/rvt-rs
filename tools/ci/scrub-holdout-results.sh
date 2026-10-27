#!/usr/bin/env bash
# Reduce the Measure results of each model fetched without a licence to
# redistribute it, the held-out models (research/holdout-models.tsv, RE-252)
# and BIMcollab's example model (research/bimcollab-models.tsv, RE-198), to
# summary counts before they are uploaded.
#
# Usage:
#   tools/ci/scrub-holdout-results.sh RESULTS_DIR
#
# A public repository's artifacts can be downloaded by anyone, so nothing taken
# from those models per element may be uploaded (Griffin, 2026-10-26). For
# each RESULTS_DIR/holdout-*/ and RESULTS_DIR/bimcollab-*/ this keeps
# counts.txt, made of the summary lines of its probe scores (the wall length
# rule, with the walls it is wrong on as a count, the export's version, the
# census of its wall representations and the hours between its save and its
# export), the count lines a probe prints for
# such a model (`type refs:` and `type copies:`, RE-253) and
# of its quantity census (the totals and each quantity's counts, the walls it
# counts apart for their two lengths (RE-252), and for each value that
# differs only the quantity and rvt-rs's value over Revit's), of
# its names census (counts by category, entity and agreement) and of its layer
# set census (its counts, not its worst elements), and
# ifc.sha256, the hash of rvt-rs's own export; every other file is deleted.
set -euo pipefail

RESULTS="${1:?usage: scrub-holdout-results.sh RESULTS_DIR}"
for dir in "$RESULTS"/holdout-*/ "$RESULTS"/bimcollab-*/; do
  [ -d "$dir" ] || continue
  {
    for f in "$dir"probe_score*.txt; do
      [ -f "$f" ] || continue
      grep -E "^(rule [a-z ,=]+: [0-9]+ walls, span is Revit's Length on [0-9]+, not on \[[0-9, ]*\]|walls with Revit's Length and a GLine: [0-9]+|a choice at each end gives the Length: [0-9]+|walls by their number of Qto_WallBaseQuantities sets: [][0-9(), ]+|walls Revit gives a Width the partition reader gives no centreline: [0-9]+; with layers [0-9]+, Width = their total on [0-9]+|export time vs save: (export less save in hours: -?[0-9]+\.[0-9]|not read \((no save time|no export time)\))|exit [0-9]+)$" "$f" \
        | awk '{ if (match($0, /not on \[[0-9, ]*\]$/)) { ids = substr($0, RSTART + 8, RLENGTH - 9); n = ids == "" ? 0 : split(ids, a, ","); $0 = substr($0, 1, RSTART - 1) "not on " n " walls" } print }' || true
    done
    for f in "$dir"probe.txt "$dir"probe-*.txt; do
      [ -f "$f" ] || continue
      grep -E '^type (refs|copies): [A-Za-z ,;:()-]+[0-9][0-9A-Za-z ,;:()-]*$' "$f" || true
    done
    for f in "$dir"qtos_vs_ifc*.txt; do
      [ -f "$f" ] || continue
      grep -E "^(Revit quantities on objects rvt-rs writes: [0-9]+; equal: [0-9]+; different: [0-9]+; missing: [0-9]+; rvt-rs quantities Revit does not write: [0-9]+|  Qto_[A-Za-z]+\.[A-Za-z]+: [0-9]+, equal [0-9]+, different [0-9]+, missing [0-9]+|walls whose curve and its cut-back curve differ, either of which Revit's export may write as their Length, counted apart \(RE-252\): [0-9]+|walls whose Length is rvt-rs's curve's: [0-9]+; Revit's Axis's: [0-9]+; neither: [0-9]+( \(none counted apart\))?|exit [0-9]+)$" "$f" || true
      awk '/^largest differences/ { on = 1; next }
           on && $2 ~ /^Qto_[A-Za-z]+\.[A-Za-z]+:$/ && $6 + 0 != 0 {
             q = $2; sub(/:$/, "", q); a = $4; sub(/,$/, "", a)
             printf "different %s: rvt-rs over Revit %.6f\n", q, a / $6
           }' "$f"
    done
    for f in "$dir"names_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E '^ +[0-9]+  (system family|family instance|[a-z ]+) \| Ifc[A-Za-z]+ \| Name (equal|differs) \| ObjectType (equal|differs)$' "$f" | sed 's/^/names: /' || true
    done
    for f in "$dir"layer_sets_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E "^( +[0-9]+  [a-z][a-z ]*(: [A-Za-z',: ]+| = Revit's Family:Type)?|  [a-z ]+ within [0-9.]+ ft of Revit's: [0-9]+ of [0-9]+)$" "$f" | sed 's/^/layer sets: /' || true
    done
  } > "$RUNNER_TEMP/counts.txt"
  find "$dir" -mindepth 1 ! -name ifc.sha256 -delete
  mv "$RUNNER_TEMP/counts.txt" "${dir}counts.txt"
  echo "$(basename "$dir"): $(wc -l < "${dir}counts.txt" | tr -d ' ') count lines kept"
done
