# Redistributable project corpus

License-free synthetic `.rvt` fixtures used for corpus health (Lane Three)
and as stable inputs for later lanes (partition scanner, typed decoders).

**Autodesk-owned samples are never committed here** (see [`SECURITY.md`](../SECURITY.md)).
Real project corpora (for example `magnetar-io/revit-test-datasets`) stay
external and are pointed at with `RVT_PROJECT_CORPUS_DIR`.

Intake lanes (public / authorized private / local probes) and the “do not
solicit unsure files” rule: [`docs/corpus-intake.md`](../docs/corpus-intake.md).

## Layout

```text
corpus/
  README.md                 # this file
  LICENSE                   # Apache-2.0 for all generated fixtures
  tier1/                    # always-on, in-repo, redistributable
    generate.sh             # regenerate fixtures via gen-fixture
    architectural-2024/
      architectural-2024.rvt
      architectural-2024.license.json
      architectural-2024.fixture.json   # gen-fixture recipe + known counts
    structural-2023/
      …
    mep-2024/
      …
  tier2/
    README.md               # optional external corpus health
```

Known-count manifests consumed by `tests/project_count_fixtures.rs` live under
[`tests/fixtures/project-counts/`](../tests/fixtures/project-counts/) and
reference the relative `project_file` paths under `tier1/`.

## Tier one (always-on)

Synthetic CFB fixtures from `gen-fixture`. They are **not** byte-compatible
with Autodesk Revit, but they open through the rvt-rs reader, schema parser,
and walker. Element instance counts are deterministic from the recipe
(round-robin over `--classes`).

| Fixture | Year | Classes | Elements | levels | walls | floors | doors | windows |
|---------|------|---------|----------|--------|-------|--------|-------|---------|
| `architectural-2024` | 2024 | Level,Wall,Floor,Door,Window | 25 | 5 | 5 | 5 | 5 | 5 |
| `structural-2023` | 2023 | Level,Wall,Floor,Column,Beam | 20 | 4 | 4 | 4 | 0 | 0 |
| `mep-2024` | 2024 | Level,Wall,Door,Window,Duct | 20 | 4 | 4 | 0 | 4 | 4 |

Regenerate (requires a release `gen-fixture` binary):

```bash
cargo build --release --bin gen-fixture
bash corpus/tier1/generate.sh
```

Each directory must keep:

- `*.rvt` — the fixture bytes
- `*.license.json` — SPDX / provenance sidecar (docs/corpus.md shape)
- `*.fixture.json` — generator recipe and authoritative known counts

## Tier two (optional external)

See [`tier2/README.md`](tier2/README.md). Health checks skip when
`RVT_PROJECT_CORPUS_DIR` is unset. CI's Tier-two job clones the MIT
`magnetar-io/revit-test-datasets` corpus and runs smoke + known-count tests.

## CI wiring

| Tier | When | Job | What |
|------|------|-----|------|
| 1 | every PR / push | `corpus-tier1` | open/schema/inventory + known-count manifests against `corpus/tier1` |
| 2 | every PR / push | `corpus-tier2` | external project smoke + magnetar count manifests via `RVT_PROJECT_CORPUS_DIR` |

Local:

```bash
# Tier 1 (no env needed)
cargo test --release --test corpus_tier1_health -- --nocapture
RVT_CORPUS_TIER1_DIR="$PWD/corpus/tier1" \
  cargo test --release --test project_count_fixtures -- --nocapture

# Tier 2 (skips without corpus)
RVT_PROJECT_CORPUS_DIR=/path/to/magnetar/Revit \
  cargo test --release --test corpus_tier2_health -- --nocapture
```

## Sidecar schema (fixture.json)

```json
{
  "schema_version": 1,
  "generator": "gen-fixture",
  "name": "architectural-2024",
  "seed": 42,
  "year": 2024,
  "classes": ["Level", "Wall", "Floor", "Door", "Window"],
  "element_count": 25,
  "expected_counts": {
    "levels": 5,
    "walls": 5,
    "floors": 5,
    "doors": 5,
    "windows": 5
  }
}
```

Later lanes should treat `expected_counts` as the oracle for synthetic
class-instance density. Typed geometry still requires real project files
under Tier two.
