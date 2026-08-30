# Compound ArcWall (`variant 0x0821`) framing harness notes

Credit research posture: [@STE1200](https://github.com/STE1200) /
[Discussion #112](https://github.com/DrunkOnJava/rvt-rs/discussions/112).
Lane C framing only — **no compound-opening decoder**.

## Status

| Claim | Status |
|-------|--------|
| Standard ArcWall `0x07fa` (2023) | Validated decoder (`arc_wall_record`) |
| Compound envelope marker `0x0821` | Observed on Einhoven (RE-14.3); **not decoded** |
| Sub-marker `0x0870` | Observed in compound bodies; meaning unknown |
| Door/Window fill semantics | **Unsupported** (RE-19 negative) |

## Harness

Library module: `rvt::compound_framing`

- `tokenize_compound_markers` / `tokenize_all_08xx` — LE u16 stamp scan
- `CompoundStampKind` / `CompoundStampSummary` — classification helpers
- `adversarial_f64_collision_seed` — documents that `21 08` can appear inside unrelated f64 bytes; treat hits as candidates only

API names deliberately avoid `decode_compound_openings`.

## Synthetic fixture law

Unit tests use owned hex/synthetic buffers only. Production corpora are
regression oracles, not discovery sources for inventing opening layouts.

## Next (human / Revit-gated)

1. Owned compound-wall fixture with API truth for embedded openings.
2. Align marker spans to ElementIds only under evidence gates.
3. Do not promote IFC void/fill from stamp hits alone.
