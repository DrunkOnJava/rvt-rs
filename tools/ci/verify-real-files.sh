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
