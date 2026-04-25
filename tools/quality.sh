#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'USAGE'
Usage: tools/quality.sh [--quick|--full]

Runs the local pre-push quality gate.

Modes:
  --quick   fmt, clippy, rustdoc, and the normal workspace test suite.
  --full    quick gate plus benchmark compile checks.

Optional tools:
  cargo-audit is run when installed. Set RVT_REQUIRE_AUDIT=1 to fail if it is
  missing.
  cargo-deny is run when installed. Set RVT_REQUIRE_DENY=1 to fail if it is
  missing.

Corpus-heavy tests remain opt-in through RVT_SAMPLES_DIR and
RVT_PROJECT_CORPUS_DIR; without those directories they skip gracefully.
USAGE
}

mode="quick"
case "${1:-}" in
    "")
        ;;
    --quick)
        mode="quick"
        ;;
    --full)
        mode="full"
        ;;
    -h|--help)
        usage
        exit 0
        ;;
    *)
        usage >&2
        exit 2
        ;;
esac

run() {
    printf '\n==> %s\n' "$*"
    "$@"
}

run cargo fmt --all -- --check
run cargo clippy --workspace --all-targets --all-features -- -D warnings
run env RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --lib
run cargo test --workspace --all-targets --all-features

run_optional() {
    local name="$1"
    local require_var="$2"
    local install_hint="$3"
    shift 3

    if ! command -v "$name" >/dev/null 2>&1; then
        if [[ "${!require_var:-0}" == "1" ]]; then
            echo "$name is required but not installed. Install with: $install_hint" >&2
            exit 1
        fi
        echo "==> $name skipped: command is not installed"
        echo "    Install with: $install_hint"
        return
    fi

    printf '\n==> %s\n' "$*"
    set +e
    local output
    output="$("$@" 2>&1)"
    local status=$?
    set -e

    if [[ "$status" -eq 0 ]]; then
        printf '%s\n' "$output"
        return
    fi

    if grep -qi "read-only path" <<<"$output"; then
        if [[ "${!require_var:-0}" == "1" ]]; then
            printf '%s\n' "$output" >&2
            exit "$status"
        fi
        echo "==> $name skipped: advisory database is on a read-only path in this environment"
        echo "    Re-run outside the sandbox, or set $require_var=1 to make this failure strict."
        return
    fi

    printf '%s\n' "$output" >&2
    exit "$status"
}

if command -v cargo-audit >/dev/null 2>&1; then
    run_optional cargo-audit RVT_REQUIRE_AUDIT "cargo install cargo-audit" cargo audit
elif [[ "${RVT_REQUIRE_AUDIT:-0}" == "1" ]]; then
    echo "cargo-audit is required but not installed. Install with: cargo install cargo-audit" >&2
    exit 1
else
    echo "==> cargo audit skipped: cargo-audit is not installed"
    echo "    Install with: cargo install cargo-audit"
fi

if command -v cargo-deny >/dev/null 2>&1; then
    run_optional cargo-deny RVT_REQUIRE_DENY "cargo install cargo-deny" cargo deny check
elif [[ "${RVT_REQUIRE_DENY:-0}" == "1" ]]; then
    echo "cargo-deny is required but not installed. Install with: cargo install cargo-deny" >&2
    exit 1
else
    echo "==> cargo deny skipped: cargo-deny is not installed"
    echo "    Install with: cargo install cargo-deny"
fi

if [[ "$mode" == "full" ]]; then
    run cargo bench --no-run
fi

printf '\nquality gate passed (%s)\n' "$mode"
