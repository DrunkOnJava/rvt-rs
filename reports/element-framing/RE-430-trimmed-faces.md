# RE-430: a face whose trim is not the rectangle of its parameters, or has holes, is meshed in the cells of its grid: TODO elements of Snowdon Towers Architectural and TODO of the other samples are drawn from their saved meshes

**Date:** 2026-11-11
**Issues:** #1706 (the saved-mesh refusals left after #1683: this is its second step); follows RE-426 (#1709, the first), RE-422 (#1683) and RE-302 (2026-11-03, the same attempt on the office's toilets, not landed)
**Artefacts:** Autodesk's Snowdon Towers Sample Architectural (Revit 2024, as Measure fetches the sample) and Revit's IFC4 export of it (local only: percentages, distances and triangle counts only here, no Revit-side volumes); rvt-rs's exports of the 2024 samples and the reference models by the base and by the change, which Measure kept (`keep_ifc`); RE1 Architecture and Revit's export of it (MIT)
**Probe and scorer:** `tools/re/mesh_bodies_vs_ifc.py`; the pure helpers `trimmed_cells` and `keep_triangle` are tested in `src/research/native_parametric_mesh.rs` (eight failure-first tests for the cells: either winding of a loop, a hole either way round, a diagonal, a bar pair in one strip, two loops sharing an edge, every knot, a bowtie, off-knot and non-finite input; three for the triangle: a needle and a hair, non-finite points, dust against a pole and a real triangle)
**Runs:** TODO
**Status:** TODO

## 1. The question

After RE-426 the saved-mesh refusals of Snowdon Towers Architectural were 122 elements for `nonrectangular parametric trim`, 21 for `multiple parametric trim loops unsupported` and the multi-reason ones. `face_grid` meshed a face only when its one trim loop ran round the rectangle of the surface's parameters; any other loop (the outline of a revolved bowl, a hole in a plate, two loops of one face) was refused. Can the region the loops enclose be cut into the grid's cells?

## 2. The rule

Every point of every trim loop is already a knot of the face's grid (`face_grid` adds the trim to the axes), so within a strip between two `u` knots each trim segment runs from one side of the strip to the other. `trimmed_cells` takes the segments that cross a strip, orders them at the strip's middle and pairs them (even-odd: the region between the first and second, between the third and fourth ...), which makes a set of trapezoids; it cuts each across at the `v` knots it holds (Sutherland-Hodgman against one line, so every piece is convex) and fans each piece into triangles turning counter-clockwise in `(u, v)`. Vertices are shared by the rounded value of their parameters (1e-9). The triangles must add up to the area the loops enclose, found independently by even-odd nesting of the loops whichever way each turns; otherwise the loops cross each other or themselves and the face is refused. The budgets are 64 loops a face, 20 million strip-segment pairs and 4 million triangles. A loop that does not close in the surface's parameters (the seam of a revolution, a pole: the edges meet in model coordinates but not in the parameters) is refused, as before. `tessellate_trimmed` then evaluates the surface at each point, turns the triangles the way `tessellate_uv` turns the grid's for the same orientation, and keeps or drops each by the rule of section 3.

The rule applies to the faces `face_grid` meshes, which are ruled, revolved and cone faces; a cylinder face with several trim loops is still refused (`multiple cylinder trim loops unsupported`, the next step). A face whose one loop is the rectangle takes the path it always took, so no element that was drawn changes.

## 3. The slivers between two near knots

The first run of the change (Measure 43026283371, `8434d29e` against `a6c41159`) refused faces that `main` never refuses, for `noncollapsed degenerate parametric triangle`: 43 on each of the four BIM4LCA office exports, 6 to 9 on the housing, 11 on each `rac_advanced`, 13 on Snowdon Plumbing, and more inside multi-reason texts. The rule it came from, meant for a pole, drops a triangle without area (a cross product of 1e-12 ft² or less) only if one of its edges is 1e-10 ft long or less, and refuses the face otherwise.

A diagnostic build (a scratch branch, not for `main`; Measure 43032254924 and 43032259387, the office, the housing, and the 2023 and 2024 samples) printed every refused triangle. There are 14 distinct ones, all of this kind:

| surface | cross product, ft² | edges, ft | parameter span (u, v) | in |
|---|---:|---|---|---|
| SurfRev | 3.987e-16 | 1.18e-09, 3.37e-07, 3.37e-07 | 2.00e-09, 9.67e-06 | Snowdon Architectural |
| SurfRev | 3.958e-15 | 3.62e-08, 1.09e-07, 1.15e-07 | 3.00e-09, 1.67e-06 | rac_advanced |
| ConeSurf | 1.485e-14 | 1.77e-08, 8.40e-07, 8.40e-07 | 4.00e-08, 8.40e-07 | Snowdon Plumbing |
| SurfRev | 3.765e-14 | 1.93e-07, 1.95e-07, 2.74e-07 | 2.87e-07, 1.32e-06 | rac_basic |
| SurfRev | 1.014e-13 | 5.56e-08, 1.82e-06, 1.82e-06 | 6.54e-06, 5.60e-08 | BIM4LCA housing, BIM4LCA office |
| SurfRev | 1.176e-13 | 2.26e-08, 5.21e-06, 5.21e-06 | 9.30e-08, 1.25e-04 | Snowdon Architectural |
| SurfRev | 1.314e-13 | 3.87e-08, 3.39e-06, 3.39e-06 | 3.10e-08, 5.43e-05 | rac_advanced |
| RuledSurf | 2.671e-13 | 1.39e-08, 2.95e-05, 2.95e-05 | 3.80e-08, 4.85e-05 | rac_basic |
| SurfRev | 3.567e-13 | 1.53e-07, 2.33e-06, 2.34e-06 | 5.67e-07, 5.60e-05 | Snowdon Architectural |
| SurfRev | 3.990e-13 | 1.82e-08, 2.19e-05, 2.19e-05 | 3.33e-05, 4.00e-08 | rac_basic |
| SurfRev | 7.237e-13 | 1.23e-07, 5.87e-06, 5.87e-06 | 9.60e-08, 9.39e-05 | rac_advanced |
| SurfRev | 7.490e-13 | 1.59e-07, 4.72e-06, 4.72e-06 | 1.50e-05, 1.59e-07 | Snowdon Architectural |
| SurfRev | 7.962e-13 | 2.16e-07, 3.68e-06, 3.69e-06 | 1.74e-05, 5.62e-04 | rac_basic |
| RuledSurf | 8.943e-13 | 3.43e-08, 2.60e-05, 2.60e-05 | 9.31e-06, 3.60e-08 | rac_basic |

(The 2023 and 2024 saves of a sample read alike.) Each is a cell between a trim loop's point and a grid line that nearly meet: two knots a few millionths of a parameter unit apart, so a sliver of the surface at most 3e-5 ft long and a hair wide. None has an edge over 2.95e-5 ft or a cross product over 8.9e-13 ft², and the shortest edge of each is under 2.2e-7 ft: far above the 1e-10 ft the pole rule allows and far below anything a scorer or a viewer can tell. The office's 43 toilets, the housing's 6 and the 11 of each `rac_advanced` are one triangle each of this table.

**The rule.** `keep_triangle` keeps a triangle with area, drops one without area whose edges are all 1e-4 ft or shorter, as it drops one with a collapsed edge, and still refuses a triangle without area and a longer edge: a needle is not a sliver, and a face whose trim leaves one is not sound. 1e-4 ft is a tenth of the 0.001 ft the scorers compare vertices at, so the crack a dropped triangle leaves is narrower than that. Three failure-first tests (a needle three points on a line, a hair 0.01 ft long and 1e-11 ft wide, non-finite points; the office's sliver, a pole, and a triangle that has area). The same helper serves the rectangular grid (`tessellate_uv`), where the corpus holds no such triangle, so it is byte for byte as before there.

## 4. Against Revit's export

TODO

## 5. The chair read in full

TODO

## 6. The other models, and what is still refused

TODO

## 7. The registered rows

Registered on #1706 before the run (https://github.com/DrunkOnJava/rvt-rs/issues/1706#issuecomment-6795159034).

TODO

## 8. The whole unfiltered run, and what the change costs

TODO

## 9. Reproduce

TODO
