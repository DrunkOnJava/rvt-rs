#!/usr/bin/env bash
# fetch_resume URL OUT [USER_AGENT]: download URL to OUT, resuming a transfer
# that comes back short instead of starting it again (#1386: a CDN that keeps
# dropping a long download made every restart fail the same way). Twelve
# attempts with a pause of 10, 20 ... 60 s; a server that refuses the range
# (curl exit 33, or 416) starts the file again. Returns non-zero when every
# attempt failed. The caller still checks the file against its pin.
#
# A connection that stays under FETCH_RESUME_MIN_SPEED bytes a second for
# FETCH_RESUME_SLOW_SECONDS (a minute) is dropped and the transfer resumed on a new one (curl exit 28). A
# Measure run's held-out house came at 12 KB/s on one runner while the other
# side of the same run fetched it in three minutes: six attempts of 15 minutes
# took 66.4 of its 66.7 MB, and the job ran into its timeout (run 43647353701).
# A host slow on every connection now fails here in minutes, as NOT MEASURED,
# rather than holding the job until it is cancelled.
fetch_resume() {
  local url=$1 out=$2 agent=${3:-rvt-rs-ci (https://github.com/DrunkOnJava/rvt-rs)} try status
  local speed=${FETCH_RESUME_MIN_SPEED:-51200} slow=${FETCH_RESUME_SLOW_SECONDS:-60}
  for try in 1 2 3 4 5 6 7 8 9 10 11 12; do
    status=0
    curl -fsSL -C - --max-time 900 --speed-limit "$speed" --speed-time "$slow" \
      -A "$agent" -o "$out" "$url" || status=$?
    [ "$status" -eq 0 ] && return 0
    case "$status" in
      33|36) rm -f "$out" ;;          # range refused or bad resume offset: start again
      22) [ -s "$out" ] || rm -f "$out" ;;
    esac
    echo "  attempt $try of 12 for $(basename "$out") failed (curl $status); $( [ -s "$out" ] && echo "resuming from $(wc -c < "$out" | tr -d ' ') bytes" || echo "starting again")" >&2
    [ "$try" -lt 12 ] && sleep $(( try * 10 < 60 ? try * 10 : 60 ))
  done
  return 1
}
