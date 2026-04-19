#!/bin/bash
# Benchmark every rvt-rs CLI against the 11-version sample family corpus.
# Writes both a machine-readable CSV and a human-readable table.
set -e
cd /home/user/Developer/re/rvt-recon-2026-04-19/tools/rvt-rs

SAMPLE_2024="../../samples/_phiag/examples/Autodesk/racbasicsamplefamily-2024.rfa"
SAMPLES_GLOB="../../samples/_phiag/examples/Autodesk/rac*.rfa"

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
