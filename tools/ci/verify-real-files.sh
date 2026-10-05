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
# Fails when a tests/*.rs target is missing from the manifest or listed twice.
#
# With cargo-nextest installed (CI installs it), every target runs in one
# `cargo nextest run`, test binaries in parallel, the manifest's skips as a
# filterset scoped to their binary, and `--profile X` passed on as
# `--cargo-profile X` (C6). Without it, or with RVT_NO_NEXTEST=1, each target
# runs in turn through `cargo test`.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
manifest="$root/tools/ci/test-targets.txt"

listed=$(awk '!/^#/ && NF >= 2 {print $2}' "$manifest" | sort)
present=$(cd "$root/tests" && ls ./*.rs | sed 's|^\./||; s|\.rs$||' | sort)
dupes=$(printf '%s\n' "$listed" | uniq -d)
missing=$(comm -13 <(printf '%s\n' "$listed") <(printf '%s\n' "$present"))
stale=$(comm -23 <(printf '%s\n' "$listed") <(printf '%s\n' "$present"))
if [ -n "$dupes$missing$stale" ]; then
  [ -n "$missing" ] && echo "not classified in tools/ci/test-targets.txt: $missing" >&2
  [ -n "$stale" ] && echo "listed but not in tests/: $stale" >&2
  [ -n "$dupes" ] && echo "listed twice: $dupes" >&2
  exit 1
fi
if [ "${1:-}" = "--check-only" ]; then
  echo "every tests/*.rs target is classified"
  exit 0
fi

if command -v cargo-nextest > /dev/null 2>&1 && [ -z "${RVT_NO_NEXTEST:-}" ]; then
  args=()
  while [ $# -gt 0 ]; do
    case "$1" in
      --profile) args+=(--cargo-profile "$2"); shift 2 ;;
      --profile=*) args+=(--cargo-profile "${1#--profile=}"); shift ;;
      *) args+=("$1"); shift ;;
    esac
  done
  targets=()
  skips=()
  while read -r class target rest; do
    case "$class" in
      real | cli | contract | hardening) ;;
      *) continue ;;
    esac
    targets+=(--test "$target")
    if [[ "${rest:-}" == skip=* ]]; then
      IFS=',' read -r -a names <<< "${rest#skip=}"
      for name in "${names[@]}"; do skips+=("(binary(=$target) & test($name))"); done
    fi
  done < <(grep -v '^#' "$manifest")
  filter=()
  if [ ${#skips[@]} -gt 0 ]; then
    joined=$(printf ' | %s' "${skips[@]}")
    filter=(-E "not (${joined:3})")
  fi
  exec cargo nextest run ${args[@]+"${args[@]}"} "${targets[@]}" ${filter[@]+"${filter[@]}"}
fi

status=0
while read -r class target rest; do
  case "$class" in
    real | cli | contract | hardening) ;;
    *) continue ;;
  esac
  skips=()
  if [[ "${rest:-}" == skip=* ]]; then
    IFS=',' read -r -a names <<< "${rest#skip=}"
    for name in "${names[@]}"; do skips+=(--skip "$name"); done
  fi
  echo "::group::$class $target"
  if ! cargo test "$@" --test "$target" -- ${skips[@]+"${skips[@]}"}; then
    echo "::error::$class target $target failed"
    status=1
  fi
  echo "::endgroup::"
done < <(grep -v '^#' "$manifest")
exit "$status"
