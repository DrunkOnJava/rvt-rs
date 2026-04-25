#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'USAGE'
Usage: tools/corpus-health.sh [corpus-dir]

Inventories redistributable Revit corpus candidates and runs the project
corpus smoke test when .rvt files are present.

Default corpus-dir:
  $RVT_PROJECT_CORPUS_DIR, or _corpus_candidates when the variable is unset.

The script never downloads files. Use tools/fetch-corpus.sh first when you want
to clone the permissive upstream candidate repositories.
USAGE
}

case "${1:-}" in
    -h|--help)
        usage
        exit 0
        ;;
esac

corpus_dir="${1:-${RVT_PROJECT_CORPUS_DIR:-_corpus_candidates}}"

if [[ ! -d "$corpus_dir" ]]; then
    echo "corpus directory not found: $corpus_dir" >&2
    echo "Run tools/fetch-corpus.sh or pass an existing directory." >&2
    exit 1
fi

echo "Corpus directory: $corpus_dir"
echo

mapfile -t revit_files < <(find "$corpus_dir" -type f \( -name '*.rvt' -o -name '*.rfa' -o -name '*.rte' -o -name '*.rft' \) -not -path '*/.git/*' | sort)
mapfile -t project_files < <(printf '%s\n' "${revit_files[@]}" | grep -E '\.rvt$' || true)

printf 'Revit files: %d\n' "${#revit_files[@]}"
printf 'Project files (.rvt): %d\n' "${#project_files[@]}"

if [[ "${#revit_files[@]}" -gt 0 ]]; then
    echo
    printf '%s\n' "${revit_files[@]}"
fi

if [[ "${#project_files[@]}" -eq 0 ]]; then
    echo
    echo "No .rvt project files found; project_corpus_smoke skipped."
    exit 0
fi

echo
echo "Running project corpus smoke test..."
RVT_PROJECT_CORPUS_DIR="$corpus_dir" cargo test --test project_corpus_smoke -- --nocapture
