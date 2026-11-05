# RE-349: a wall whose base join names an element that covers its plan in part is cut over the covered part

**Date:** 2026-11-05
**Issues:** #1411 (a wall base covered in part: cut over the covered part); follows RE-336 (#1388, the base cut), RE-337 (#1396, #1398, the wall ends)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`), as Measure fetches them
**Tools:** `tools/re/wall_heights_vs_ifc.py`, `tools/re/wall_quantities_vs_ifc.py`, `tools/re/wall_top_cover_vs_ifc.py`, `tools/re/wall_layers_vs_ifc.py`
**Runs:** Measure [42099826920](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42099826920) (both offices, filtered to the four wall scorers, `keep_ifc`) at `c413f382` against `main` at `44c055db`. The commits after `c413f382` on this branch are tests, documents and this report; none changes `src/`. No other model has been measured on this head.
**Status:** positive for the two walls it reaches (2058499 and 2058501 as built); every other model not yet measured on this head.

## 1. What the bytes say

RE-336 cuts a wall's base at the top of the footing its base join names (`JoinToRoofGStep` with `m_baseOrTop` 0) when that element covers the whole of the wall's plan. The office's Basement walls 2058499 and 2058501 as built name the footing 2058504, which covers their plan only in part: it begins at y 75.9678 ft, and the walls' plan (y 75.7285 to 80.3806 ft, 75.4183 ft once RE-337 carries their ends on to Revit's) begins before it. Revit's export gives the body that results. In the frame of rvt-rs's export (Revit's export turned 40.33 degrees about the origin, which fits the vertices of the two footings to 1e-3 ft), the vertices of Revit's wall 2058499 over the end are:

- from the footing's low edge (y 75.9678 ft) towards the far end, the base follows the footing's sloped top: 174.8688 ft at the edge to 176.8373 ft at y 80.3806 ft;
- over the 0.55 ft of plan before the edge, the base is the wall's own, 174.2126 ft, and a vertical face of 0.66 ft joins the two at the edge (174.2126 to 174.8688 ft).

That is the wall minus the footing: where the two share plan the wall starts at the footing's top, and where they do not it starts at its own base. Revit's lowest point for both walls is therefore the wall's own base (174.2126 ft), which `main` already had because it kept the base whole, and its volume is 30.805 ft3, which `main`'s whole prism (35.302 ft3) is 4.497 ft3 over.

## 2. The rule

`decide_base` (`wall_tops.rs`) takes an element named by the base join that covers the wall in part as well as one that covers it whole: its outline over the wall's plan (`reach` over `STEP_TOLERANCE_SQUARE_FEET`) gives the convex pieces of the plan inside it (`covered_pieces`: the wall's convex pieces clipped by the triangles of the outline, joined where two make a convex piece). The element's top over the covered pieces only must lie between the wall's base and top, as for a whole cover (a top that crosses either is refused as before), and only one element may qualify. `base_cut_solid` then makes, for each piece of the wall (its whole plan, or the pieces of a wall whose ends RE-337 carried on), the part inside the covered pieces cut at the element's top (along the plane of a shape-edited top, or at its level) and the rest of the piece as a column from the wall's own base. A wall carries `BodyBaseSource` `wall_base_sloped_at_floor_covering_in_part` or `wall_base_cut_at_floor_covering_in_part` (a level top), and `BaseCoverElementId`. Where the covered part is not read (the wall's plan is not made of convex pieces, a hole or opening of the element lies over the wall, or the part falls in more than 32 pieces) the wall keeps its base and says `partly_covered_plan_not_read` as its `BaseCoverRefusal`.

RE-337's end rule asks whether the cut of the base still resolves over the longer plan. It now does for these two walls, so they take Revit's end with the base cut.

## 3. Measured

Run 42099826920, head `c413f382` against `main` at `44c055db`, the office as built and at permit.

- **Walls changed.** Comparing the body of every wall (volume, bounds and the base, end and cover markers, 607 walls a side) between the two exports of the office as built: 2 differ, 2058499 and 2058501. At building permit the export is byte for byte `main`'s (`ifc.sha256` equal on both sides), because the footing covers the walls there.
- **The two walls, as built.** Lowest point Revit's before and after (174.2126 ft, `wall_heights_vs_ifc`). Plan end y 75.7285 ft before, 75.4183 ft after, Revit's. Volume (`wall_quantities_vs_ifc`) 35.3019 ft3 before, 30.6585 ft3 after, against Revit's 30.8054 ft3 (and 30.8054 for 2058501): 4.497 ft3 over before, 0.147 ft3 under after. That 0.147 ft3 is the upper step of the end that RE-337 does not draw (the same figure as wall 2057670's, which ends at the same walls), `BodyEndSource` `wall_end_to_base_wall_upper_step_omitted`, tracked on #1410. Plan rows of `wall_top_cover_vs_ifc`: `equal 85 taller 10 lower 0` of 95 points before, `equal 90 taller 10 lower 0` of 100 after, as wall 2057670; the area where the top is higher than Revit's (the far-end stub, 0.3363 ft2) is unchanged.
- **Other scorer output.** `wall_layers_vs_ifc` is identical on both offices. In `wall_heights_vs_ifc` and `wall_top_cover_vs_ifc` the scorer fits one turn and shift to all walls, and the two walls' longer plan moves that fit by 0.0005 degree, so the `over` rows and the taller and lower areas of walls whose bodies are unchanged move in the fourth decimal (about 600 walls' `wall_top_cover_vs_ifc` rows differ); the comparison of the bodies above is what says no other wall changed.
- **Diagnostics.** `exported.wall_base_refusals` as built goes from `{no_element_covers_the_wall_plan: 2}` to empty, and `exported.wall_end_refusals` from `{base_not_cut_over_the_extended_plan: 2}` to empty; at permit both are as on `main` (`wall_end_refusals` `{wall_beneath_not_one_rectangle: 2}`, #1409). `tests/export_decisions_counts.rs` listed both reasons as witnessed by the office as built; neither is now, and the table no longer requires them.

## 4. Not measured

Nothing but the office has been measured on this head. These are exercised by no measured wall, so no Revit export has checked them:

- the level top of a partial cover (`wall_base_cut_at_floor_covering_in_part`; the two walls are cut along a sloped top);
- an element with a hole or opening under the wall (refused, not scored), and a wall whose plan is not convex (refused);
- a wall covered in part by an element in a model other than the office as built: Core Interior, RE1 Architecture, the 24 Revit 2025 models, Autodesk's sample projects and the held-out files are to be compared with `main` before this lands, and a wall they move that Revit's export does not confirm is a reason to narrow the rule;
- a footing that covers a wall's two ends and not its middle (the covered pieces would be two).
