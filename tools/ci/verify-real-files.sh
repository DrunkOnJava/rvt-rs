#!/usr/bin/env bash
# Run the real-file, CLI, contract and hardening targets of
# tools/ci/test-targets.txt.
#
# Usage: tools/ci/verify-real-files.sh [--check-only | cargo test options...]
#   e.g. tools/ci/verify-real-files.sh --profile ci
#   --check-only checks the manifest and runs nothing.
#
# Corpus-reading targets use RVT_SAMPLES_DIR / RVT_PROJECT_CORPUS_DIR when
# set and skip their corpus checks otherwise. Unit targets are never run.
# Fails when a tests/*.rs target is missing from the manifest or listed twice,
# or is not a module of tests/integration/main.rs.
#
# Every tests/*.rs file is a module of one test binary, `integration` (C6):
# 77 binaries took 118 s of build time where one takes 13 s. A target is its
# module, so its tests are named `<target>::<test>`.
#
# With cargo-nextest installed (CI installs it), every target runs in one
# `cargo nextest run`, each test in its own process as before, the manifest's
# skips as a filterset scoped to their module, and `--profile X` passed on as
# `--cargo-profile X`. Without it, or with RVT_NO_NEXTEST=1, `cargo test`
# runs the same tests, chosen by exact name.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
manifest="$root/tools/ci/test-targets.txt"
merged="$root/tests/integration/main.rs"

listed=$(awk '!/^#/ && NF >= 2 {print $2}' "$manifest" | sort)
present=$(cd "$root/tests" && ls ./*.rs | sed 's|^\./||; s|\.rs$||' | sort)
modules=$(sed -n 's/^mod \([a-z0-9_]*\);$/\1/p' "$merged" | sort)
dupes=$(printf '%s\n' "$listed" | uniq -d)
missing=$(comm -13 <(printf '%s\n' "$listed") <(printf '%s\n' "$present"))
stale=$(comm -23 <(printf '%s\n' "$listed") <(printf '%s\n' "$present"))
unmerged=$(comm -13 <(printf '%s\n' "$modules") <(printf '%s\n' "$present"))
gone=$(comm -23 <(printf '%s\n' "$modules") <(printf '%s\n' "$present"))
if [ -n "$dupes$missing$stale$unmerged$gone" ]; then
  [ -n "$missing" ] && echo "not classified in tools/ci/test-targets.txt: $missing" >&2
  [ -n "$stale" ] && echo "listed but not in tests/: $stale" >&2
  [ -n "$dupes" ] && echo "listed twice: $dupes" >&2
  [ -n "$unmerged" ] && echo "not a module of tests/integration/main.rs: $unmerged" >&2
  [ -n "$gone" ] && echo "a module of tests/integration/main.rs but not in tests/: $gone" >&2
  exit 1
fi
if [ "${1:-}" = "--check-only" ]; then
  echo "every tests/*.rs target is classified and merged"
  exit 0
fi

targets=()
skips=()
while read -r class target rest; do
  case "$class" in
    real | cli | contract | hardening) ;;
    *) continue ;;
  esac
  targets+=("$target")
  if [[ "${rest:-}" == skip=* ]]; then
    IFS=',' read -r -a names <<< "${rest#skip=}"
    for name in "${names[@]}"; do skips+=("$target $name"); done
  fi
done < <(grep -v '^#' "$manifest")

if command -v cargo-nextest > /dev/null 2>&1 && [ -z "${RVT_NO_NEXTEST:-}" ]; then
  args=()
  while [ $# -gt 0 ]; do
    case "$1" in
      --profile) args+=(--cargo-profile "$2"); shift 2 ;;
      --profile=*) args+=(--cargo-profile "${1#--profile=}"); shift ;;
      *) args+=("$1"); shift ;;
    esac
  done
  joined=$(printf '|%s' "${targets[@]}")
  filter="test(/^(${joined:1})::/)"
  if [ ${#skips[@]} -gt 0 ]; then
    excluded=""
    for skip in "${skips[@]}"; do
      excluded+=" | (test(/^${skip%% *}::/) & test(${skip#* }))"
    done
    filter="$filter & not (${excluded:3})"
  fi
  exec cargo nextest run ${args[@]+"${args[@]}"} --test integration -E "$filter"
fi

# libtest's filters match substrings, so `type_global_ids::` would also pick
# `door_type_global_ids::`; list the binary's tests and pass exact names.
listing=$(cargo test "$@" --test integration -- --list --format terse)
chosen=()
while IFS= read -r line; do
  name="${line%: test}"
  [ "$name" = "$line" ] && continue
  module="${name%%::*}"
  rest="${name#*::}"
  keep=""
  for target in "${targets[@]}"; do [ "$target" = "$module" ] && keep=1; done
  for skip in ${skips[@]+"${skips[@]}"}; do
    [ "${skip%% *}" = "$module" ] && [[ "$rest" == *"${skip#* }"* ]] && keep=""
  done
  [ -n "$keep" ] && chosen+=("$name")
done <<< "$listing"
# With no names, libtest would run every test, unit targets included.
if [ ${#chosen[@]} -eq 0 ]; then
  echo "no test of the manifest's targets is in the integration binary" >&2
  exit 1
fi
echo "${#chosen[@]} tests chosen from ${#targets[@]} targets"
exec cargo test "$@" --test integration -- --exact "${chosen[@]}"
