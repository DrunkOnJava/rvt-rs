# RE-465 — A ceiling in pieces: Revit's GrossArea is the piece its first-drawn lines lie on

**Date:** 2026-11-14
**Issue:** #1209 (W7, coverings' GrossArea); follows RE-297, RE-355, RE-97
**Artefacts:** the BIM4LCA office (concrete and timber, as built and at building permit, Revit 2024) with Revit's own IFC4 exports.
**Result:** positive, on one building.
- A ceiling whose sketch is several separate loops is written, by rvt-rs as by Revit, as one `IfcCovering` with a body item per piece (#331). Revit's `Qto_CoveringBaseQuantities.GrossArea` is the area of one piece (RE-297).
- That piece is the one the sketch's lowest-ElementId line lies on, the first drawn (as RE-355 found for a void's first line), on 71 of 75 ceilings.
- Narrowed to the ceilings whose two lowest lines lie on one piece, it is Revit's piece on 62 of 62. The other 13 get no `GrossArea`.

## 1. The rules scored

Probe `probe_1209_ceiling_piece_lines` and scorer `ceiling_piece_lines_vs_ifc` (branch `research/1209-ceiling-pieces`) take each sketch line's current copy, as the export does (RE-216). They set beside Revit's `GrossArea` the piece holding:
- the first and the last curve of the Sketch's stored list (RE-97);
- the lowest and the highest line ElementId;
- the lowest ElementId, where the second-lowest lies on the same piece.

Measure [43564653950](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43564653950) and [43565505348](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43565505348):

| file | ceilings in pieces | GrossArea is one piece's | first listed = lowest id | last listed = highest id | narrowed: written, Revit's, left out |
|---|---:|---:|---:|---:|---|
| concrete office, as built | 16 | 16 | 15 | 2 | 15, 15, 1 |
| concrete office, permit | 20 | 20 | 19 | 2 | 16, 16, 4 |
| timber office, as built | 19 | 19 | 18 | 3 | 15, 15, 4 |
| timber office, permit | 20 | 20 | 19 | 3 | 16, 16, 4 |

- The first listed curve and the lowest ElementId pick the same piece on every ceiling: the stored list is in drawing order.
- Neither order nor size picked the piece (RE-297, Measure 43552644831).

## 2. The one exception

The lowest-ElementId rule misses only ceiling 2094020, on all four files. Its sketch was one rectangle, lines 2094022 to 2094025, later split in two by lines 2636081 and up, so two of the original lines lie on each piece. The lowest line, 2094022, is on the lower piece; Revit's `GrossArea` is the upper piece's on every file. Its two lowest lines lie on two pieces, so the narrowed rule leaves it out, as it does 12 other ceilings.

## 3. What is written

`PlanProfile::first_drawn_piece` holds the piece the two lowest lines both lie on, carried as `m_plan_profile_first_piece`. `add_ceiling_quantities` writes that piece's area as the ceiling's `GrossArea`, under the same conditions as a ceiling of one piece: drawn from its sketch, uncut, and its layers filling its body.

## 4. Where it rests

The four files are one building. No other model with a Revit export (the BIM4LCA housing, the Revit 2025 models, Core Interior, RE1) has a ceiling in pieces (Measure [43566205330](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43566205330): 32 to 45 ceilings per housing file, none in pieces). The rule is listed on `docs/one-project-rules.md`.
