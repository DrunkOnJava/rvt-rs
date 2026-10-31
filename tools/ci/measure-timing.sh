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
# and head medians in milliseconds and head / base, then the same for each
# export's peak resident memory in MiB (GNU time's maximum RSS; "-" where the
# runner has no GNU time). A file either build fails to export is reported as
# failed, not timed. FILEs that do not exist are skipped, so an unexpanded
# glob costs nothing.
set -uo pipefail
BASE="$1"; HEAD="$2"; OUT="$3"; shift 3
ROUNDS="${ROUNDS:-3}"
tmp=$(mktemp -d)
# GNU time's maximum resident set size, in KiB, where the runner has it.
timer=()
/usr/bin/time -f %M -o /dev/null true 2> /dev/null && timer=(/usr/bin/time -f %M -o "$tmp/peak")

run() { # bin file -> "milliseconds peak-KiB" (peak "-" without GNU time), or "failed"
  local started ms peak=-
  rm -f "$tmp/peak"
  started=$(date +%s%N)
  # shellcheck disable=SC2086 # RVT_IFC_FLAGS is a list of flags.
  # ${timer[@]+...}: an empty array is "unbound" to bash 3.2 under set -u.
  if ${timer[@]+"${timer[@]}"} "$1/rvt-ifc" "$2" -o "$tmp/model.ifc" --diagnostics "$tmp/diagnostics.json" ${RVT_IFC_FLAGS:-} > /dev/null 2>&1; then
    ms=$(( ($(date +%s%N) - started) / 1000000 ))
    [ -s "$tmp/peak" ] && peak=$(awk 'END { print $1 }' "$tmp/peak")
    echo "$ms $peak"
  else
    echo failed
  fi
}

median() { # values... -> median, "failed" when any failed, "-" when any is unknown
  case " $* " in *" failed "*) echo failed; return ;; *" - "*) echo -; return ;; esac
  printf '%s\n' "$@" | sort -n | awk '{ v[NR] = $1 } END { print v[int((NR + 1) / 2)] }'
}

ratio() { # base head -> head / base, or "-"
  case "$1 $2" in *failed*|*-*) echo -; return ;; esac
  [ "$1" -eq 0 ] && { echo -; return; }
  awk -v h="$2" -v b="$1" 'BEGIN { printf "%.2f", h / b }'
}

mib() { # KiB -> MiB, or the value as it is
  case "$1" in failed|-) echo "$1" ;; *) echo $(( ($1 + 512) / 1024 )) ;; esac
}

{
  echo "| file | base (ms) | head (ms) | head / base | base peak (MiB) | head peak (MiB) | peak head / base |"
  echo "|---|---:|---:|---:|---:|---:|---:|"
} > "$OUT"
for file in "$@"; do
  [ -f "$file" ] || continue
  name=$(basename "$file" .rvt)
  echo "::group::$name"
  base=(); head=()
  for round in $(seq "$ROUNDS"); do
    if [ $((round % 2)) -eq 1 ]; then
      base+=("$(run "$BASE" "$file")"); head+=("$(run "$HEAD" "$file")")
    else
      head+=("$(run "$HEAD" "$file")"); base+=("$(run "$BASE" "$file")")
    fi
  done
  base_ms=(); base_kib=(); head_ms=(); head_kib=()
  for v in "${base[@]}"; do base_ms+=("${v%% *}"); base_kib+=("${v##* }"); done
  for v in "${head[@]}"; do head_ms+=("${v%% *}"); head_kib+=("${v##* }"); done
  echo "base ${base_ms[*]}; head ${head_ms[*]}"
  echo "peak KiB base ${base_kib[*]}; head ${head_kib[*]}"
  b=$(median "${base_ms[@]}"); h=$(median "${head_ms[@]}")
  bp=$(median "${base_kib[@]}"); hp=$(median "${head_kib[@]}")
  echo "| $name | $b | $h | $(ratio "$b" "$h") | $(mib "$bp") | $(mib "$hp") | $(ratio "$bp" "$hp") |" >> "$OUT"
  echo "::endgroup::"
done
rm -rf "$tmp"
