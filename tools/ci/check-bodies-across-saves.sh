#!/usr/bin/env bash
# Fail when a Revit 2026 or 2027 sample's bodies are not its earlier save's (RE-341).
#
# Usage:
#   tools/ci/check-bodies-across-saves.sh RESULTS_DIR
#
# tools/re/bodies_across_saves.py holds the beams, landings, stair runs and
# sketch outlines of each 2026 and 2027 save of rac_advanced, rac_basic and
# rst_basic to the same project's 2024 and 2025 saves, and Measure's script
# writes its output beside each sample as bodies_across_saves.txt. This reads
# those files from RESULTS_DIR and fails on any that does not end in PASS and
# exit status 0, and on a sample whose file was not written, so a check that
# did not run is a failure and never a pass.
#
# SAMPLE_YEARS (the shard's first and last year of samples, as Measure's
# matrix gives them) says which samples this shard holds: a shard with no 2026
# or 2027 sample has nothing to check. SCORERS (Measure's `scorers` input)
# keeps the check off a run that names other scorers only.
set -euo pipefail

results="${1:?usage: check-bodies-across-saves.sh RESULTS_DIR}"
years="${SAMPLE_YEARS:-}"
scorers="${SCORERS:-}"

if [ -n "$scorers" ] && ! grep -qw bodies_across_saves <<< "$scorers"; then
  echo "bodies_across_saves is not among the scorers asked for: not checked"
  exit 0
fi

read -ra range <<< "$years"
low="${range[0]:-0}"
high="${range[1]:-9999}"
expected=()
for year in 2026 2027; do
  { [ "$year" -ge "$low" ] && [ "$year" -le "$high" ]; } || continue
  for project in racadvancedsampleproject racbasicsampleproject rstbasicsampleproject; do
    expected+=("sample-$year-$project")
  done
done
if [ "${#expected[@]}" -eq 0 ]; then
  echo "no 2026 or 2027 sample among this shard's years (${years:-all}): nothing to check"
  exit 0
fi

failed=0
for name in "${expected[@]}"; do
  file="$results/$name/bodies_across_saves.txt"
  if [ ! -f "$file" ]; then
    echo "::error::$name: bodies_across_saves.txt was not written"
    failed=1
    continue
  fi
  if grep -qx 'bodies across saves: PASS' "$file" && grep -qx 'exit 0' "$file"; then
    echo "$name: PASS"
    grep -E '^(beams and landings|stair runs|sketch outlines|named elements|named [0-9]+, [0-9]+ save): ' "$file"
  else
    echo "::error::$name: its bodies are not its earlier save's"
    grep -v '^exit' "$file"
    failed=1
  fi
done
echo "samples checked: ${#expected[@]}"
exit "$failed"
