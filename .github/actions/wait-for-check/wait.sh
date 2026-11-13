#!/usr/bin/env bash
# Wait for a check run on a commit to finish, and fail unless it succeeded.
# Inputs come from the environment (action.yml): GH_TOKEN, GH_REPO, SHA,
# CHECK_NAME, ATTEMPTS (minutes to wait), DOCS_ONLY_SKIPS and SUPERSEDED_SKIPS
# (true or false), CI_WORKFLOW (the workflow that makes CHECK_NAME, default CI);
# POLL_SECONDS (default 60) is for trying the script by hand.
set -euo pipefail

: "${GH_TOKEN:?}" "${GH_REPO:?}" "${SHA:?}" "${CHECK_NAME:?}"
attempts="${ATTEMPTS:-80}"
poll="${POLL_SECONDS:-60}"
docs_only_skips="${DOCS_ONLY_SKIPS:-true}"
superseded_skips="${SUPERSEDED_SKIPS:-false}"
# jq reads it as $ENV.CI_WORKFLOW in ci_cancelled.
export CI_WORKFLOW="${CI_WORKFLOW:-CI}"

result() {
  if [ -n "${GITHUB_OUTPUT:-}" ]; then echo "result=$1" >> "$GITHUB_OUTPUT"; fi
}

# Each API call is retried with backoff: a GitHub 5xx (a 502 on 2026-11-04
# failed the v0.14.0 deploy) is not a verdict on the commit. A call that still
# fails ends the step, so nothing goes ahead without CI's answer.
api() {
  local try out
  for try in 1 2 3 4 5 6; do
    if out=$(gh api "$@"); then printf '%s\n' "$out"; return 0; fi
    [ "$try" -eq 6 ] && break
    echo "gh api $1 failed (try ${try}); retrying" >&2
    sleep $((try * 10))
  done
  echo "gh api $1 failed 6 times" >&2
  return 1
}

check_runs() {
  api "repos/${GH_REPO}/commits/${SHA}/check-runs" -X GET -f check_name="${CHECK_NAME}" --jq "$1"
}

# Whether the push's CI run on this commit was cancelled. CI's push runs share a
# group with no cancel-in-progress (ci.yml `concurrency`): a run still pending
# when a newer push arrives is cancelled, and a cancelled run never makes
# CHECK_NAME. Waiting for it ends only at the timeout, with the deploy of every
# later commit queued behind this one (13 November 2026: a viewer deploy waited
# 80 minutes for a commit whose CI had been cancelled in its first minute).
# An API failure is not an answer: the caller goes on waiting, and the next
# read of CHECK_NAME fails the step if the API is down for good.
ci_cancelled() {
  local state
  state=$(api "repos/${GH_REPO}/actions/runs" -X GET -f head_sha="${SHA}" -f event=push \
    --jq '[.workflow_runs[] | select(.name == $ENV.CI_WORKFLOW)] | sort_by(.run_number) | last // empty | "\(.status) \(.conclusion // "")"') || return 2
  [ "$state" = "completed cancelled" ]
}

# Every page of the commit's files (the endpoint returns 300 a page). A commit
# that lists no file gives one empty line, which the filter below does not
# ignore, so it waits for CI: closed. The filter is ci.yml's
# `on.push.paths-ignore`; keep the two in step.
files=$(api --paginate "repos/${GH_REPO}/commits/${SHA}" --jq '.files[].filename') || {
  echo "::error::could not list the files of ${SHA}"; exit 1; }
if ! grep -qvE '(\.md$|^docs/|^LICENSE|^\.gitignore$|^\.github/ISSUE_TEMPLATE/|^CITATION\.cff$)' <<<"$files"; then
  # A push of several commits runs CI once, on its last commit, when any of
  # them changes more than the ignored paths: a docs-only commit that ends
  # such a push has the check, and it still has to pass.
  seen=$(check_runs '.total_count') || { echo "::error::could not read ${CHECK_NAME} on ${SHA}"; exit 1; }
  if [ "$seen" = 0 ]; then
    if [ "$docs_only_skips" = true ]; then
      echo "CI's push filter skips this commit and it has no ${CHECK_NAME}; nothing to wait for"
      result skipped-no-ci
      exit 0
    fi
    # A release must not ride on an earlier commit's CI: this commit's tree
    # is that commit's plus docs, and CI says nothing about it. Failing here
    # also spares the wait for a check that will never come.
    echo "::error::every file of ${SHA} is one CI's push filter skips, so CI did not run on it and ${CHECK_NAME} will never come; go ahead only from a commit CI ran on"
    exit 1
  fi
  echo "${SHA} changes only files CI's push filter ignores, but ${CHECK_NAME} ran on it (earlier commits of its push); waiting for it"
fi

for _ in $(seq 1 "$attempts"); do
  conclusion=$(check_runs '[.check_runs[] | select(.status == "completed") | .conclusion] | first // ""') || {
    echo "::error::could not read ${CHECK_NAME} on ${SHA}"; exit 1; }
  case "$conclusion" in
    success) echo "${CHECK_NAME} passed on ${SHA}"; result success; exit 0 ;;
    "")
      if ci_cancelled; then
        if [ "$superseded_skips" = true ]; then
          echo "CI on ${SHA} was cancelled by a later push, so ${CHECK_NAME} will never come; this commit is not deployed, the later one is"
          result superseded
          exit 0
        fi
        echo "::error::CI on ${SHA} was cancelled by a later push, so ${CHECK_NAME} will never come; go ahead only from a commit CI ran on"
        exit 1
      fi
      sleep "$poll" ;;
    *) echo "::error::${CHECK_NAME} concluded ${conclusion} on ${SHA}; not going ahead"; exit 1 ;;
  esac
done
echo "::error::${CHECK_NAME} did not finish on ${SHA} within ${attempts} minutes"
exit 1
