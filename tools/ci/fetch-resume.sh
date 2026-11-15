#!/usr/bin/env bash
# fetch_resume URL OUT [USER_AGENT]: download URL to OUT, resuming a transfer
# that comes back short instead of starting it again (#1386: a CDN that keeps
# dropping a long download made every restart fail the same way). A server that
# refuses the range (curl exit 33, or 416) starts the file again. Returns
# non-zero when the attempts or the time run out. The caller still checks the
# file against its pin.
#
# It resumes for as long as FETCH_RESUME_BUDGET seconds allow (40 minutes, well
# inside a Measure job's 90), with up to `attempts` attempts, each capped at
# what is left of the budget and at 15 minutes. A connection that stays under
# FETCH_RESUME_MIN_SPEED bytes a second (1 KB/s) for FETCH_RESUME_SLOW_SECONDS
# (a minute) has stalled, and is dropped and resumed. A Measure run's held-out
# house (66.7 MB) came at 12 KB/s on every connection of one runner while the
# other side of the same run fetched it in three minutes: six attempts of 15
# minutes reached 66.4 MB and held the job until it was cancelled (run
# 43647353701). A host that slow now ends at the budget as NOT MEASURED, while
# one steady at 30 KB/s still brings that file in.
fetch_resume() {
  local url=$1 out=$2 agent=${3:-rvt-rs-ci (https://github.com/DrunkOnJava/rvt-rs)}
  local budget=${FETCH_RESUME_BUDGET:-2400} speed=${FETCH_RESUME_MIN_SPEED:-1024}
  local slow=${FETCH_RESUME_SLOW_SECONDS:-60} attempts=20 start=$SECONDS try status left pause
  for ((try = 1; try <= attempts; try++)); do
    left=$((budget - (SECONDS - start)))
    if [ "$left" -le 0 ]; then
      echo "  $(basename "$out"): the ${budget} s budget is spent after $((try - 1)) attempts" >&2
      return 1
    fi
    status=0
    curl -fsSL -C - --max-time $((left < 900 ? left : 900)) \
      --speed-limit "$speed" --speed-time "$slow" \
      -A "$agent" -o "$out" "$url" || status=$?
    [ "$status" -eq 0 ] && return 0
    case "$status" in
      33|36) rm -f "$out" ;;          # range refused or bad resume offset: start again
      22) [ -s "$out" ] || rm -f "$out" ;;
    esac
    echo "  attempt $try of $attempts for $(basename "$out") failed (curl $status); $( [ -s "$out" ] && echo "resuming from $(wc -c < "$out" | tr -d ' ') bytes" || echo "starting again")" >&2
    pause=$(( try * 10 < 60 ? try * 10 : 60 ))
    left=$((budget - (SECONDS - start)))
    [ "$try" -lt "$attempts" ] && sleep $(( pause < left ? pause : (left > 0 ? left : 0) ))
  done
  return 1
}
