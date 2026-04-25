# Project Count Fixtures

These manifests record count targets for curated real project files. They are
used by `tests/project_count_fixtures.rs`.

Count statuses are intentionally explicit:

- `known` means the count comes from a redistributable source such as a paired
  Revit IFC export or an owner-supplied schedule.
- `known_gap` means the source count is known, but the current decoder is
  expected to miss it. The manifest must name the tracking issue and, when
  applicable, the unsupported feature surfaced by export diagnostics.
- `decoder_baseline` means the count is not an authoritative model count; it
  pins current decoder output so regressions are visible until an authoritative
  schedule or reference export is available.
- `unknown` means no authoritative count is available yet. The manifest must
  include a reason, so missing data is deliberate and reviewable.

Do not mark a category as `known` unless the source is recorded in the manifest.
