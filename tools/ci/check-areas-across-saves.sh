#!/usr/bin/env bash
# Fail when rac_advanced's Areas differ across its saves (#1261, #1194, #1434).
#
# Usage:
#   tools/ci/check-areas-across-saves.sh RESULTS_DIR
#
# The gate of the Areas of Revit 2025 to 2027 (Griffin, 2026-11-05, "Same-project
# gate"): rac_advanced's Areas on its 2025, 2026 and 2027 saves equal its 2024
# save's (tools/re/areas_across_saves.py, run by tools/ci/measure-reference-models.sh,
# which writes areas_across_saves.txt at the top of a samples shard's results).
# Its 2021 save is held to its 2022 save the same way (#1434), in
# areas_across_saves_2021.txt. With every sample of a gate in the shard, a failed
# check, a missing result and a result that read other than 25 Areas, 25 of them
# with a body, on any save each fail; a pass needs the scorer's own `passed`
# line, naming the gate's four saves, or its two. A shard with fewer than a
# gate's samples (a narrower sample_years, or the other shard: the 2008 to 2023
# shard holds the 2021 and 2022 saves, the 2024 to 2099 shard the 2024 to 2027
# ones) checks nothing of that gate and says so in a notice.
#
# RESULTS_DIR holds the head side's shard artifacts, one directory each (the
# files of two shards share a name, so they are not merged). This was a step of
# the measure job; as a job of its own it turns the run red without removing
# Compare's report, which a failed measure job would skip.
#   SAMPLES  Measure's `samples` input: without `true` no sample is exported.
set -euo pipefail

results="${1:?usage: check-areas-across-saves.sh RESULTS_DIR}"
summary="${GITHUB_STEP_SUMMARY:-/dev/null}"

note() { # text: a notice in the log and a line in the run summary
  echo "::notice::$1"
  printf -- '- %s\n' "$1" >> "$summary"
}

printf '### Areas across rac_advanced saves (#1261)\n\n' >> "$summary"

if [ "${SAMPLES:-}" != true ]; then
  note "Not checked: samples=true was not set, so no rac_advanced sample was exported and the Areas were not held to its 2024 save."
  exit 0
fi

mapfile -t files < <(find "$results" -maxdepth 2 -type f \( -name areas_across_saves.txt -o -name areas_across_saves_2021.txt \) | sort)
if [ "${#files[@]}" -eq 0 ]; then
  echo "::error::the Areas of rac_advanced were not compared across its saves: no result, so nothing was checked"
  printf -- '- FAIL: no areas_across_saves.txt or areas_across_saves_2021.txt in any shard\n' >> "$summary"
  exit 1
fi

failed=0
passed=0
for file in "${files[@]}"; do
  cat "$file"
  if grep -q '^areas across saves: skipped:' "$file"; then
    note "$(sed -n '1p' "$file")"
    continue
  fi
  saves=4
  case "$file" in *_2021.txt) saves=2 ;; esac
  if [ "$(sed -n '$p' "$file")" != "exit 0" ] || ! grep -q "^areas across saves: passed: 25 Areas, 25 with a body, on each of $saves saves" "$file"; then
    echo "::error::the Areas of rac_advanced differ across its saves (2024 to 2027, or 2021 and 2022), or other than 25 Areas with a body were read on a save, or a save is missing ($file)"
    printf -- '- FAIL: %s\n' "$file" >> "$summary"
    failed=1
  else
    passed=$((passed + 1))
    printf -- '- PASS: %s\n' "$file" >> "$summary"
  fi
done
echo "results read: ${#files[@]}, passed: $passed"
exit "$failed"
