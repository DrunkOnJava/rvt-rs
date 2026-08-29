# Project Count Fixtures

These manifests record count targets for curated project files. They are used
by `tests/project_count_fixtures.rs`, `tests/corpus_tier1_health.rs`, and
`tests/corpus_tier2_health.rs`.

## Tiers

| Prefix / id | Corpus root | Availability |
|-------------|-------------|--------------|
| `tier1-*` | in-repo [`corpus/tier1/`](../../../corpus/tier1/) | always (PR CI Tier one) |
| `magnetar-*` (no `tier` field or `tier: 2`) | `$RVT_PROJECT_CORPUS_DIR` | optional / CI Tier two |

Tier-one manifests use `fixture_metric` (`class_instances.<ClassName>`) so
tests can assert synthetic gen-fixture instance density without relying on
typed IFC export (scaffold-only on synthetic CFBs).

## Count statuses

Count statuses are intentionally explicit:

- `known` means the count comes from a redistributable source such as a
  gen-fixture recipe, a paired Revit IFC export, or an owner-supplied schedule.
- `known_gap` means the source count is known, but the current decoder is
  expected to miss it. The manifest must name the tracking issue and, when
  applicable, the unsupported feature surfaced by export diagnostics.
- `decoder_baseline` means the count is not an authoritative model count; it
  pins current decoder output so regressions are visible until an authoritative
  schedule or reference export is available.
- `unknown` means no authoritative count is available yet. The manifest must
  include a reason, so missing data is deliberate and reviewable.

Do not mark a category as `known` unless the source is recorded in the manifest.
