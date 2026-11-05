#!/usr/bin/env bash
# Fail when a Revit 2026 or 2027 sample's bodies are not its earlier save's (RE-341).
#
# Usage:
#   tools/ci/check-bodies-across-saves.sh RESULTS_DIR
#
# tools/re/bodies_across_saves.py holds the beams, tread landings, stair runs
# and sketch outlines of each 2026 and 2027 save of rac_advanced, rac_basic and
# rst_basic to the same project's 2024 and 2025 saves, and Measure's script
# writes its output beside each sample as bodies_across_saves.txt. This reads
# those files from RESULTS_DIR (the head side's results, merged or one directory
# for each shard) and fails on any
# that reads FAIL or does not end in exit status 0, and on a sample whose file
# was not written, so a check that did not run for want of a file is a failure.
#
# It is a job of its own in the Measure workflow, so that a failure turns the
# run red without removing Compare's report. A check that is skipped on purpose
# is never silent: it prints a notice, and writes a line to the run summary,
# saying why and what to set:
#   SCORERS      Measure's `scorers` input; a run that names scorers without
#                this one is not checked.
#   SAMPLES      Measure's `samples` input; without `true` no sample is exported.
#   SAMPLE_YEARS Measure's `sample_years` input, the first and last year of the
#                samples fetched; empty is all of them.
# A sample whose 2024 or 2025 save is not in the run is marked SKIPPED by the
# measuring script, and one with only some PARTIAL; both print a notice and
# neither is a pass.
set -euo pipefail

results="${1:?usage: check-bodies-across-saves.sh RESULTS_DIR}"
summary="${GITHUB_STEP_SUMMARY:-/dev/null}"

note() { # text: a notice in the log and a line in the run summary
  echo "::notice::$1"
  printf -- '- %s\n' "$1" >> "$summary"
}

printf '### Bodies across saves (RE-341)\n\n' >> "$summary"

if [ -n "${SCORERS:-}" ] && ! grep -qw bodies_across_saves <<< "$SCORERS"; then
  note "Not checked: the scorers asked for (${SCORERS}) exclude bodies_across_saves."
  exit 0
fi
if [ "${SAMPLES:-}" != true ]; then
  note "Not checked: samples=true was not set, so no 2026 or 2027 sample was held to its earlier save. A change to a reader of Revit 2024 to 2027 is measured with samples=true."
  exit 0
fi

read -ra range <<< "${SAMPLE_YEARS:-}"
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
  note "Not checked: sample_years (${SAMPLE_YEARS:-}) holds no 2026 or 2027 sample."
  exit 0
fi

failed=0
passed=0
partial=0
skipped=0
for name in "${expected[@]}"; do
  # RESULTS_DIR holds the shards' artifacts merged or one directory each.
  file=$(find "$results" -maxdepth 3 -type f -path "*/$name/bodies_across_saves.txt" | sort | sed -n '1p')
  if [ -z "$file" ] || [ ! -f "$file" ]; then
    echo "::error::$name: bodies_across_saves.txt was not written"
    printf -- '- %s: FAIL, no bodies_across_saves.txt was written\n' "$name" >> "$summary"
    failed=1
    continue
  fi
  if grep -qx 'exit 0' "$file" && grep -qx 'bodies across saves: PASS' "$file"; then
    passed=$((passed + 1))
    echo "$name: PASS"
    grep -E '^(beams and landings|stair runs|sketch outlines|named elements|named [0-9]+, [0-9]+ save): ' "$file" || true
    printf -- '- %s: PASS\n' "$name" >> "$summary"
  elif grep -qx 'exit 0' "$file" && grep -q '^bodies across saves: PARTIAL' "$file"; then
    partial=$((partial + 1))
    note "$name: $(grep '^bodies across saves: PARTIAL' "$file")"
  elif grep -qx 'exit 0' "$file" && grep -q '^bodies across saves: SKIPPED' "$file"; then
    skipped=$((skipped + 1))
    note "$name: $(grep '^bodies across saves: SKIPPED' "$file")"
  else
    echo "::error::$name: its bodies are not its earlier save's"
    grep -v '^exit' "$file" || true
    printf -- '- %s: FAIL, see its bodies_across_saves.txt\n' "$name" >> "$summary"
    failed=1
  fi
done
echo "samples expected: ${#expected[@]}, passed: $passed, partial: $partial, skipped: $skipped"
printf '\nsamples expected: %s, passed: %s, partial: %s, skipped: %s\n' "${#expected[@]}" "$passed" "$partial" "$skipped" >> "$summary"
exit "$failed"
