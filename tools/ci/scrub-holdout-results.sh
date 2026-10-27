#!/usr/bin/env bash
# Reduce each held-out model's Measure results to summary counts before they
# are uploaded (research/holdout-models.tsv, RE-252).
#
# Usage:
#   tools/ci/scrub-holdout-results.sh RESULTS_DIR
#
# The held-out models carry no redistribution licence, and a public
# repository's artifacts can be downloaded by anyone, so nothing taken from
# them per element may be uploaded. For each RESULTS_DIR/holdout-*/ this keeps
# counts.txt, made of the summary lines of its probe scores (the wall length
# rule, the export's version and the census of its wall representations) and
# of its quantity census (the totals and each quantity's counts), and
# ifc.sha256, the hash of rvt-rs's own export; every other file is deleted.
set -euo pipefail

RESULTS="${1:?usage: scrub-holdout-results.sh RESULTS_DIR}"
for dir in "$RESULTS"/holdout-*/; do
  [ -d "$dir" ] || continue
  {
    for f in "$dir"probe_score*.txt; do
      [ -f "$f" ] || continue
      grep -E "^(rule [a-z ,=]+: [0-9]+ walls, span is Revit's Length on [0-9]+, not on \[[0-9, ]*\]|walls with Revit's Length and a GLine: [0-9]+|a choice at each end gives the Length: [0-9]+|walls by their number of Qto_WallBaseQuantities sets: [][0-9(), ]+|exit [0-9]+)$" "$f" || true
    done
    for f in "$dir"qtos_vs_ifc*.txt; do
      [ -f "$f" ] || continue
      grep -E '^(Revit quantities on objects rvt-rs writes: [0-9]+; equal: [0-9]+; different: [0-9]+; missing: [0-9]+; rvt-rs quantities Revit does not write: [0-9]+|  Qto_[A-Za-z]+\.[A-Za-z]+: [0-9]+, equal [0-9]+, different [0-9]+, missing [0-9]+|exit [0-9]+)$' "$f" || true
    done
  } > "$RUNNER_TEMP/counts.txt"
  find "$dir" -mindepth 1 ! -name ifc.sha256 -delete
  mv "$RUNNER_TEMP/counts.txt" "${dir}counts.txt"
  echo "$(basename "$dir"): $(wc -l < "${dir}counts.txt" | tr -d ' ') count lines kept"
done
