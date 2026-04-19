#!/bin/bash
# Benchmark every rvt-rs CLI against the 11-version sample family corpus.
# Writes both a machine-readable CSV and a human-readable table.
#
# Run from the crate root (i.e. the directory containing Cargo.toml).
# Samples dir is configurable via RVT_SAMPLES_DIR (defaults to
# ../../samples/_phiag/examples/Autodesk — the layout the phi-ag/rvt
# git submodule uses when dropped under samples/).
set -e

if [ ! -f "Cargo.toml" ] || ! grep -q '^name = "rvt"' Cargo.toml 2>/dev/null; then
  echo "error: run this from the rvt-rs crate root (where Cargo.toml lives)" >&2
  exit 1
fi

SAMPLES_DIR="${RVT_SAMPLES_DIR:-../../samples/_phiag/examples/Autodesk}"
SAMPLE_2024="${SAMPLES_DIR}/racbasicsamplefamily-2024.rfa"
SAMPLES_GLOB="${SAMPLES_DIR}/rac*.rfa"

if [ ! -f "$SAMPLE_2024" ]; then
  echo "error: sample file not found: $SAMPLE_2024" >&2
  echo "       set RVT_SAMPLES_DIR to override the sample location" >&2
  exit 1
fi

echo "=== rvt-rs CLI benchmarks (2024 sample) ==="
echo
hyperfine --warmup 2 --style color \
  --export-markdown docs/data/bench-2024.md \
  --export-csv docs/data/bench-2024.csv \
  --command-name "rvt-info (text)"      "./target/release/rvt-info  $SAMPLE_2024 > /dev/null" \
  --command-name "rvt-info (json)"      "./target/release/rvt-info  -f json $SAMPLE_2024 > /dev/null" \
  --command-name "rvt-schema"           "./target/release/rvt-schema $SAMPLE_2024 > /dev/null" \
  --command-name "rvt-history"          "./target/release/rvt-history $SAMPLE_2024 > /dev/null" \
  --command-name "rvt-history --partitions" "./target/release/rvt-history --partitions $SAMPLE_2024 > /dev/null" \
  --command-name "rvt-analyze (text)"   "./target/release/rvt-analyze --redact --no-color --quiet $SAMPLE_2024 > /dev/null" \
  --command-name "rvt-analyze (json)"   "./target/release/rvt-analyze --redact --json $SAMPLE_2024 > /dev/null"

echo
echo "=== cross-version sweep: rvt-analyze over all 11 releases ==="
echo
hyperfine --warmup 1 --style color \
  --export-csv docs/data/bench-versions.csv \
  "for f in $SAMPLES_GLOB; do ./target/release/rvt-analyze --redact --json --quiet \"\$f\" > /dev/null; done"
