#!/usr/bin/env bash
# fetch_resume URL OUT [USER_AGENT]: download URL to OUT, resuming a transfer
# that comes back short instead of starting it again (#1386: a CDN that keeps
# dropping a long download made every restart fail the same way). Six attempts
# with a pause of 10, 20 ... 60 s; a server that refuses the range (curl exit
# 33, or 416) starts the file again. Returns non-zero when every attempt failed.
# The caller still checks the file against its pin.
fetch_resume() {
  local url=$1 out=$2 agent=${3:-rvt-rs-ci (https://github.com/DrunkOnJava/rvt-rs)} try status
  for try in 1 2 3 4 5 6; do
    status=0
    curl -fsSL -C - --max-time 900 -A "$agent" -o "$out" "$url" || status=$?
    [ "$status" -eq 0 ] && return 0
    case "$status" in
      33|36) rm -f "$out" ;;          # range refused or bad resume offset: start again
      22) [ -s "$out" ] || rm -f "$out" ;;
    esac
    echo "  attempt $try of 6 for $(basename "$out") failed (curl $status); $( [ -s "$out" ] && echo "resuming from $(wc -c < "$out" | tr -d ' ') bytes" || echo "starting again")" >&2
    [ "$try" -lt 6 ] && sleep $(( try * 10 < 60 ? try * 10 : 60 ))
  done
  return 1
}
