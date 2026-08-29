#!/usr/bin/env bash
# Developer-friendly local quality gate for rvt-rs.
#
# Default (no flags): fmt check, clippy -D warnings, rustdoc -D warnings,
# and the workspace test suite. Does not require network.
#
# Optional expensive / environment-dependent checks are opt-in via flags.
# Prefer this script for day-to-day local verification; use tools/quality.sh
# when you also want the pre-push supply-chain / bench compile path.
set -euo pipefail

usage() {
    cat <<'USAGE'
Usage: tools/check-local.sh [options]

Required gates (always run):
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --lib
  cargo test --workspace --all-targets --all-features

Optional gates (opt-in; fail clearly when prerequisites are missing):
  --viewer         viewer/ npm typecheck + build (no network install)
  --corpus         re-run tests with RVT_SAMPLES_DIR / RVT_PROJECT_CORPUS_DIR
  --ifcopenshell   verify the ifcopenshell Python module imports
  --deny           run cargo deny check (requires cargo-deny)
  --audit          run cargo audit (requires cargo-audit)
  --all-optional   enable every optional gate above

  -h, --help       show this help

Corpus directories (for --corpus):
  RVT_SAMPLES_DIR          default: $PWD/_corpus/examples/Autodesk
  RVT_PROJECT_CORPUS_DIR   default: $PWD/_project_corpus/Revit

Does not download corpora or install tools. Fetch corpora with
tools/fetch-corpus.sh or the AGENTS.md clone recipe when needed.
USAGE
}

run() {
    printf '\n==> %s\n' "$*"
    "$@"
}

require_cmd() {
    local name="$1"
    local hint="$2"
    if ! command -v "$name" >/dev/null 2>&1; then
        echo "error: '$name' is required for this check but is not installed." >&2
        echo "       $hint" >&2
        exit 1
    fi
}

run_viewer=0
run_corpus=0
run_ifcopenshell=0
run_deny=0
run_audit=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --viewer)
            run_viewer=1
            ;;
        --corpus)
            run_corpus=1
            ;;
        --ifcopenshell)
            run_ifcopenshell=1
            ;;
        --deny)
            run_deny=1
            ;;
        --audit)
            run_audit=1
            ;;
        --all-optional)
            run_viewer=1
            run_corpus=1
            run_ifcopenshell=1
            run_deny=1
            run_audit=1
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
    shift
done

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# --- required gates ---------------------------------------------------------

run cargo fmt --all -- --check
run cargo clippy --workspace --all-targets --all-features -- -D warnings
run env RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --lib
run cargo test --workspace --all-targets --all-features

# --- optional gates ---------------------------------------------------------

if [[ "$run_viewer" -eq 1 ]]; then
    if [[ ! -d viewer/node_modules ]]; then
        echo "error: viewer/node_modules is missing." >&2
        echo "       From viewer/: run 'npm ci' once (network), then re-run with --viewer." >&2
        exit 1
    fi
    run npm --prefix viewer run typecheck
    run npm --prefix viewer run build
    if command -v wasm-objdump >/dev/null 2>&1 && [[ -f viewer/pkg/rvt_bg.wasm ]]; then
        printf '\n==> wasm network-import audit\n'
        if wasm-objdump -j Import -x viewer/pkg/rvt_bg.wasm \
            | grep -iE '"(fetch|XMLHttpRequest|WebSocket|EventSource|sendBeacon)"' >/dev/null; then
            echo "VIOLATION: compiled WASM imports a network primitive" >&2
            exit 1
        fi
        echo "PASS: no network imports"
    else
        echo "==> wasm network-import audit skipped (wasm-objdump or viewer/pkg missing)"
    fi
fi

if [[ "$run_corpus" -eq 1 ]]; then
    samples_dir="${RVT_SAMPLES_DIR:-$root/_corpus/examples/Autodesk}"
    project_dir="${RVT_PROJECT_CORPUS_DIR:-$root/_project_corpus/Revit}"
    missing=0
    if [[ ! -d "$samples_dir" ]]; then
        echo "error: sample corpus directory not found: $samples_dir" >&2
        missing=1
    fi
    if [[ ! -d "$project_dir" ]]; then
        echo "error: project corpus directory not found: $project_dir" >&2
        missing=1
    fi
    if [[ "$missing" -eq 1 ]]; then
        echo "       Clone corpora first (see AGENTS.md), or set RVT_SAMPLES_DIR /" >&2
        echo "       RVT_PROJECT_CORPUS_DIR to existing trees. --corpus does not download." >&2
        exit 1
    fi
    run env \
        RVT_SAMPLES_DIR="$samples_dir" \
        RVT_PROJECT_CORPUS_DIR="$project_dir" \
        cargo test --workspace --all-targets --all-features
fi

if [[ "$run_ifcopenshell" -eq 1 ]]; then
    require_cmd python3 "Install Python 3 and the ifcopenshell package."
    printf '\n==> python3 -c "import ifcopenshell"\n'
    if ! python3 -c "import ifcopenshell; print(ifcopenshell.version)" 2>/dev/null; then
        echo "error: Python module 'ifcopenshell' is not importable." >&2
        echo "       Install with: python3 -m pip install 'ifcopenshell>=0.8.0,<0.9.0'" >&2
        exit 1
    fi
fi

if [[ "$run_deny" -eq 1 ]]; then
    require_cmd cargo-deny "Install with: cargo install cargo-deny --locked"
    run cargo deny check
fi

if [[ "$run_audit" -eq 1 ]]; then
    require_cmd cargo-audit "Install with: cargo install cargo-audit --locked"
    run cargo audit
fi

printf '\ncheck-local: all requested gates passed\n'
