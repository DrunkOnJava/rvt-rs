#!/usr/bin/env bash
# Time rvt-ifc at two refs on one runner, for the Measure workflow's `timing`
# input (.github/workflows/measure.yml).
#
# Usage:
#   tools/ci/measure-timing.sh BASE_BIN HEAD_BIN OUT_FILE FILE...
#
# Measure's base and head jobs run on different runners, whose speed alone
# moves an export's time by up to about 1.7x (B81), so their wall times
# compare machines as much as code. Here both builds export each FILE on the
# same machine, in turn, ROUNDS times (default 3), the order alternating
# each round so a drift in the runner's speed falls on both alike, and the
# medians are compared. OUT_FILE gets a Markdown table: each file, the base
# and head medians in milliseconds, and head / base. A file either build
# fails to export is reported as failed, not timed. FILEs that do not exist
# are skipped, so an unexpanded glob costs nothing.
set -uo pipefail
BASE="$1"; HEAD="$2"; OUT="$3"; shift 3
ROUNDS="${ROUNDS:-3}"
tmp=$(mktemp -d)

run() { # bin file -> milliseconds, or "failed"
  local started
  started=$(date +%s%N)
  # shellcheck disable=SC2086 # RVT_IFC_FLAGS is a list of flags.
  if "$1/rvt-ifc" "$2" -o "$tmp/model.ifc" --diagnostics "$tmp/diagnostics.json" ${RVT_IFC_FLAGS:-} > /dev/null 2>&1; then
    echo $(( ($(date +%s%N) - started) / 1000000 ))
  else
    echo failed
  fi
}

median() { # values... -> median, or "failed" when any failed
  case " $* " in *" failed "*) echo failed; return ;; esac
  printf '%s\n' "$@" | sort -n | awk '{ v[NR] = $1 } END { print v[int((NR + 1) / 2)] }'
}

{
  echo "| file | base (ms) | head (ms) | head / base |"
  echo "|---|---:|---:|---:|"
} > "$OUT"
for file in "$@"; do
  [ -f "$file" ] || continue
  name=$(basename "$file" .rvt)
  echo "::group::$name"
  base_ms=(); head_ms=()
  for round in $(seq "$ROUNDS"); do
    if [ $((round % 2)) -eq 1 ]; then
      base_ms+=("$(run "$BASE" "$file")"); head_ms+=("$(run "$HEAD" "$file")")
    else
      head_ms+=("$(run "$HEAD" "$file")"); base_ms+=("$(run "$BASE" "$file")")
    fi
  done
  echo "base ${base_ms[*]}; head ${head_ms[*]}"
  b=$(median "${base_ms[@]}"); h=$(median "${head_ms[@]}")
  if [ "$b" = failed ] || [ "$h" = failed ] || [ "$b" -eq 0 ]; then
    echo "| $name | $b | $h | - |" >> "$OUT"
  else
    echo "| $name | $b | $h | $(awk -v h="$h" -v b="$b" 'BEGIN { printf "%.2f", h / b }') |" >> "$OUT"
  fi
  echo "::endgroup::"
done
rm -rf "$tmp"
