#!/usr/bin/env bash
# Fail a pull request that breaks a stable surface without announcing it
# (S2, #873): the Rust API, the viewer's wasm bindings and the CLIs' JSON.
# For the Rust API two checks, because neither alone sees every break:
#
# - cargo-semver-checks lints the API against the base. Exit 100 is a
#   verdict (violations found), 101 a check that could not complete.
#   While the baseline is before 1.0 the release type is `patch`, so
#   anything not patch-compatible counts; from 1.0 it is `minor`, so a
#   change that only needs a minor release (adding `#[must_use]`, say)
#   does not. Additive breaks, such as a variant added to an exhaustive
#   enum, are this check's: the listing diff below sees only removals.
# - cargo-public-api lists every public item with its full signature, at
#   the base and at the head. A base line missing from the head is an item
#   removed or changed, which covers what cargo-semver-checks documents it
#   misses, such as a parameter's type. An added line is not breaking.
#
# A breaking change passes only when the pull request's title carries the
# Conventional Commits `!` or its body a `BREAKING CHANGE:` footer, read
# from the API so that re-running the job after a retitle sees it. A tool
# that cannot complete fails the job whatever the title says.
#
# Runs on the pull request's merge commit, as actions/checkout checks it
# out, and needs PR, REPO and GH_TOKEN, cargo-semver-checks and
# cargo-public-api, and the nightly named by PUBLIC_API_TOOLCHAIN.

set -uo pipefail

# The base is the merge commit's first parent: main as GitHub merged the
# pull request into it. The event's base.sha can be older (7c4e87f5 on
# #912 with main at 6af75c97), and comparing with it counts changes main
# took since as the pull request's.
BASE=$(git rev-parse HEAD^1) || exit 1

# The policy follows the baseline's version, not the head's: the pull
# request that takes the crate to 1.0 is still judged by pre-1.0 rules.
version=$(git show "$BASE:Cargo.toml" \
    | python3 -c 'import sys, tomllib; print(tomllib.loads(sys.stdin.read())["package"]["version"])')
case "$version" in
    0.*) release_type=patch ;;
    *) release_type=minor ;;
esac
echo "baseline rvt $version: cargo-semver-checks at release type $release_type"

breaking=0
cargo semver-checks -p rvt --baseline-rev "$BASE" --default-features --release-type "$release_type"
case $? in
    0) ;;
    100) breaking=1 ;;
    *)
        echo "::error::cargo-semver-checks could not complete; see the log above"
        exit 1
        ;;
esac

cargo public-api --help || exit 1
listing() {
    (cd "$1" && cargo +"$PUBLIC_API_TOOLCHAIN" public-api -p rvt) > "$2" \
        || { echo "::error::cargo public-api could not list the API at $1"; exit 1; }
}
base_dir="${RUNNER_TEMP:-/tmp}/semver-base"
git worktree add --detach "$base_dir" "$BASE" > /dev/null || exit 1
listing "$base_dir" base-api.txt
listing . head-api.txt
git worktree remove --force "$base_dir"
echo "public API: $(wc -l < base-api.txt) items at the base, $(wc -l < head-api.txt) at the head"
if grep -vxF -f head-api.txt base-api.txt > removed-api.txt; then
    echo "Public items at the base that the head removed or changed:"
    cat removed-api.txt
    breaking=1
fi

# The other stable surfaces (RELEASE.md, "What 1.0 keeps stable"), each
# against the base's: the wasm bindings the viewer calls, and the JSON the
# CLIs and the Python package print. Exit 100 is a verdict, anything else a
# check that could not run.
surface() {
    "$@"
    case $? in
        0) ;;
        100) breaking=1 ;;
        *)
            echo "::error::$* could not run"
            exit 1
            ;;
    esac
}
work="${RUNNER_TEMP:-/tmp}/semver-surfaces"
mkdir -p "$work/base"
git show "$BASE:rvt-wasm/src/lib.rs" > "$work/base-wasm.rs" || exit 1
git archive "$BASE" docs/schemas | tar -x -C "$work/base" || exit 1
surface python3 tools/ci/wasm-api.py compat "$work/base-wasm.rs" rvt-wasm/src/lib.rs
surface python3 tools/ci/schema-compat.py "$work/base/docs/schemas" docs/schemas

if [ "$breaking" -eq 0 ]; then
    echo "The stable surfaces have no breaking change against the base."
    exit 0
fi
title=$(gh api "repos/$REPO/pulls/$PR" --jq .title)
body=$(gh api "repos/$REPO/pulls/$PR" --jq '.body // ""')
if printf '%s\n' "$title" | grep -qE '^[a-z]+(\([^)]*\))?!:' \
    || printf '%s\n' "$body" | grep -qE '^BREAKING[ -]CHANGE:'; then
    echo "::notice::A stable surface has a breaking change (above), and the pull request announces it."
    exit 0
fi
echo "::error::A stable surface has a breaking change (above) that the pull request does not announce. Mark the title type(scope)!: or undo the change, then re-run this job."
exit 1
