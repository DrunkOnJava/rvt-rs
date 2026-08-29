#!/usr/bin/env bash
# Regenerate Tier-one synthetic fixtures with gen-fixture.
# Run from the repository root after: cargo build --release --bin gen-fixture
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
bin="${GEN_FIXTURE_BIN:-$root/target/release/gen-fixture}"

if [[ ! -x "$bin" ]]; then
  echo "gen-fixture not found at $bin" >&2
  echo "Build with: cargo build --release --bin gen-fixture" >&2
  exit 1
fi

gen() {
  local name="$1" year="$2" seed="$3" count="$4" classes="$5"
  local dir="$root/corpus/tier1/$name"
  mkdir -p "$dir"
  "$bin" "$name" \
    --output "$dir/$name.rvt" \
    --year "$year" \
    --seed "$seed" \
    --element-count "$count" \
    --classes "$classes"
  echo "  ok $name ($year, seed=$seed, n=$count, classes=$classes)"
}

echo "Regenerating corpus/tier1 fixtures via $bin"
gen architectural-2024 2024 42 25 "Level,Wall,Floor,Door,Window"
gen structural-2023    2023  7 20 "Level,Wall,Floor,Column,Beam"
gen mep-2024           2024 99 20 "Level,Wall,Door,Window,Duct"
echo "Done. Recompute sha256 in *.license.json if bytes changed."
