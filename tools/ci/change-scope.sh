#!/usr/bin/env bash
# Prints skip=true|false and prose=true|false for GITHUB_OUTPUT (ci.yml's scope
# job). It says true only on proof: a failed or empty answer leaves both false,
# so everything runs, and only a line on stderr says why.
set -uo pipefail

say() { echo "scope: $*" >&2; }
answer() { echo "skip=$1"; echo "prose=$2"; exit 0; }

case "${EVENT:-}" in
  pull_request)
    files=$(gh api --paginate "repos/${GH_REPO}/pulls/${PR}/files" --jq '.[].filename') || files=
    [ -n "$files" ] || { say "the pull request's files could not be listed"; answer false false; }
    # A name a test or a check under tests/ mentions is read by it.
    read_by_tests=$(grep -rhoE '[A-Za-z0-9_.-]+\.md' tests | sort -u) || read_by_tests=
    while IFS= read -r file; do
      case "$file" in
        reports/*|changelog.d/*) ;;
        docs/*.md) ;;
        *) say "$file is not prose"; answer false false ;;
      esac
      if grep -qxF "$(basename "$file")" <<<"$read_by_tests"; then
        say "$file is read by a test"; answer false false
      fi
    done <<<"$files"
    say "only prose changed: the Rust jobs are skipped"
    answer true true
    ;;
  push)
    [ "${REF:-}" = refs/heads/main ] || answer false false
    pr=$(gh api "repos/${GH_REPO}/commits/${SHA}/pulls" --jq '.[0].number // empty') || pr=
    [ -n "$pr" ] || { say "no pull request landed this commit"; answer false false; }
    head=$(gh api "repos/${GH_REPO}/pulls/${pr}" --jq .head.sha) || head=
    [ -n "$head" ] || { say "pull request ${pr} has no head"; answer false false; }
    ours=$(gh api "repos/${GH_REPO}/git/commits/${SHA}" --jq .tree.sha) || ours=
    theirs=$(gh api "repos/${GH_REPO}/git/commits/${head}" --jq .tree.sha) || theirs=
    if [ -z "$ours" ] || [ "$ours" != "$theirs" ]; then
      say "the merged tree (${ours:-unknown}) is not pull request ${pr}'s (${theirs:-unknown})"
      answer false false
    fi
    passed=$(gh api "repos/${GH_REPO}/commits/${head}/check-runs?check_name=full%20%2F%20CI%20passed&filter=latest" \
      --jq '.check_runs[0].conclusion // empty') || passed=
    [ "$passed" = success ] || { say "full / CI passed is '${passed}' on ${head}"; answer false false; }
    # The caches of Rust dependencies are saved only from main.
    changed=$(gh api "repos/${GH_REPO}/commits/${SHA}" --jq '.files[].filename') || changed=
    [ -n "$changed" ] || { say "the commit's files could not be listed"; answer false false; }
    if grep -qE '(^|/)Cargo\.(lock|toml)$|^rust-toolchain|^\.github/|^tools/ci/' <<<"$changed"; then
      say "the commit changes dependencies or CI: everything runs"
      answer false false
    fi
    say "the tree pull request ${pr} passed on: its tests are skipped"
    answer true false
    ;;
  *) answer false false ;;
esac
