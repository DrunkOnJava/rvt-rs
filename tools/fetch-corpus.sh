#!/usr/bin/env bash
# Automated clone of the 7 MIT/Apache-licensed corpus sources
# identified in docs/corpus-hunt-2026-04-21.md (Q-01 work).
#
# Usage:
#   tools/fetch-corpus.sh [target-dir]
#
# Default target: _corpus_candidates/ (gitignored).
# Each repo is shallow-cloned to minimize bandwidth + disk.
#
# After cloning, run the corpus smoke test with:
#   RVT_PROJECT_CORPUS_DIR="$(realpath _corpus_candidates)" \
#     cargo test --test project_corpus_smoke
#
# Safe to re-run: existing clones get `git pull` instead of re-cloning.

set -euo pipefail

TARGET_DIR="${1:-_corpus_candidates}"
mkdir -p "$TARGET_DIR"

REPOS=(
    "DynamoDS/DynamoRevit"
    "DynamoDS/RevitTestFramework"
    "DynamoDS/DynamoWorkshops"
    "DynamoDS/RefineryToolkits"
    "DynamoDS/RefineryPrimer"
    "chuongmep/OpenMEP"
    "theseus-rs/file-type"
)

echo "Fetching ${#REPOS[@]} corpus sources to $TARGET_DIR/"
echo

for repo in "${REPOS[@]}"; do
    name="${repo##*/}"
    dir="$TARGET_DIR/$name"
    if [[ -d "$dir/.git" ]]; then
        echo "  [update] $repo"
        git -C "$dir" fetch --depth 1 origin 2>&1 | sed 's/^/    /' || {
            echo "    fetch failed — skipping"
            continue
        }
        git -C "$dir" reset --hard origin/HEAD 2>&1 | sed 's/^/    /' || true
    else
        echo "  [clone]  $repo"
        git clone --depth 1 "https://github.com/$repo.git" "$dir" 2>&1 | sed 's/^/    /' || {
            echo "    clone failed — skipping"
            continue
        }
    fi
done

echo
echo "Corpus fetched. File inventory:"
echo
find "$TARGET_DIR" -type f \( -name '*.rvt' -o -name '*.rfa' \) \
    -not -path '*/.git/*' | sort

echo
echo "To run the smoke test on every fetched .rvt:"
echo "  for rvt in \$(find $TARGET_DIR -name '*.rvt' -not -path '*/.git/*'); do"
echo "    RVT_PROJECT_CORPUS_DIR=\$(dirname \"\$rvt\") \\"
echo "      cargo test --test project_corpus_smoke 2>&1 | tail -5"
echo "  done"
