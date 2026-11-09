# RE-380: Revit writes a profile-edited wall's openings as the pockets of its profile against its convex hull

**Date:** 2026-11-09
**Issues:** #1607 (found by #1600's Measure); follows RE-151 (the openings of a sketch's holes, and the one wall opening of Core Interior), RE-358 (the profile's line carried on past its end) and RE-378 (the profile's highest point as the wall's top)
**Artefacts:** the BIM4LCA office as built and at permit and its terrain files (Revit 2024, CC BY-SA 4.0, each with Revit's own IFC4 export), and Autodesk's `2024_Core_Interior.rvt` with Revit's export of it (`research/bim4lca-measure.tsv`, `research/project-corpus.tsv`)
**Probe:** `probes/examples/probe_1607_wall_profile_lines.rs` (commit `80808447`, archived under `refs/archive/2026-11-09/cited/80808447c8c5`)
**Scorers:** `tools/re/unfilled_openings_vs_ifc.py`, `tools/re/wall_heights_vs_ifc.py`, `tools/re/wall_quantities_vs_ifc.py`, `tools/re/wall_bodies_vs_ifc.py`
**Status:** positive, and resting on two walls: 18 walls have sketch lines in the five files, the rule predicts an opening on three, and Revit's export holds the opening on the two of them it holds.

## 1. The question

RE-151's rule (`elevation_profile_cuts`) reads a wall's edited elevation profile, the lines of its sketch in one vertical plane, as its record box less one region for each run of lines that leaves the loop's bounding rectangle, tagged with the run's highest ElementId. It was measured on one wall, Core Interior's 55840, whose one opening in Revit's export it equals in Tag and box. Measure run 42614629885 (all models, `8a5b2225` against `b969296e`) and its scorer run 42618842627 show it writing an opening Revit's export does not write on ten more walls, and missing the one Revit writes on a terrain wall:

| File | Wall | rvt-rs's opening (Tag) | Revit's export |
|---|---|---|---|
| office, as built and at permit | 2058501, 2058499, 2056770, 2056705, 2057670, 2057668, 2070689, 2070691 (RE-358's eight) | 2137992, 2138050, 2138402, 2138449, 2138640, 2138691, 2139036, 2139150 | none |
| terrain, concrete and timber | 1351524 | 1351630 | none |
| terrain, concrete and timber | 1387731 | 1389150 | Tag 1389018, another shape |

## 2. What Revit writes

The probe prints every wall's sketch lines with their ElementIds and ends (run 42619901187, five Revit 2024 files with Revit's export). Taking each wall's lines as one loop and computing the loop's convex hull and its pockets, the stretches of the loop between two corners of the hull that the hull's edge cuts off:

| Wall | Pockets | Highest line id of the pocket | Revit's export |
|---|---|---|---|
| Core Interior 55840 | 1, eight corners, and the profile rectangle's corner (29.917, 76.0) in the profile's own along-and-up feet | 55859 | opening Tag 55859, the same box, nine points, 301.649 ft2 |
| terrain 1387731 | 1, four corners (122.404, 196.522), (138.808, 195.699), (171.639, 192.408), (215.944, 192.408), closed by the hull's edge | 1389018 | opening Tag 1389018, the same four corners and no fifth |
| terrain 1351524 | none (the slope steepens to the right, so the profile is convex) | | none |
| office, RE-358's eight | none (each top is one straight sloped line) | | none |
| office, 3025796, 3026297, 3027380 | none | | none |

**What closes a pocket.** The two openings are not closed alike. Core Interior's opening 55859 is nine points in its own plane: the stretch's eight corners and the corner of the profile's bounding rectangle at the wall's far end and top, (29.917, 76.0) in the profile's own along-and-up feet. It holds 301.649 ft2, where the stretch closed by its hull edge holds 71.233. The terrain wall's opening has the stretch's four (x, z) corners and no fifth, where the rectangle's corner (215.944, 196.522) would be one: the hull's edge closes it. The hull edge's ends differ. On 55840 they stand on two adjacent sides of the profile's rectangle, the top at (3.583, 76.0) and the far end at (29.917, 58.5), neither at a corner, and the hull's edge cuts the rectangle's top and far-end corner off. On 1387731 one stands inside the rectangle, (215.944, 192.408), and the other is the rectangle's corner (122.404, 196.522). Why Revit closes the one through the corner and not the other is not shown; two walls give two shapes, and the rule reads exactly those and the notch, where both ends of the edge lie on a side in common, the edge runs along the side and closes the stretch under any reading (section 5). Both `unfilled_openings_vs_ifc`'s corner lists (sets of distinct coordinates) and its boxes are the same for the stretch closed either way, which is how the first reading of this report, every pocket closed by its hull edge, passed them: the CI of the first commit of the pull request failed on the test of #227 (`tests/wall_profile_openings.rs`), which reads the opening's area, with rvt-rs's 71.233 ft2 against Revit's 301.649. The scorer now compares the opening's volume (section 4).

Core Interior's wall 34362 has the same pocket as 55840 and is in neither export (run 42621689708 finds no `IfcWall` with its Tag in Revit's or rvt-rs's). None of Revit's 30 (as built) and 24 (at permit) openings of no cutter in an office wall is on a wall with sketch lines; they are RE-321's rectangular openings. So the pocket rule predicts an opening on three of the 18 walls with sketch lines and none on the other 15, and Revit's export holds exactly the two it can (in Tag and corners) and none on the 15; RE-151's rule is right on one wall.

Revit writes the body of all 11 exported profile walls (55840, the office's eight, the terrain's two) as a `Tessellation` (`IfcPolygonalFaceSet`): the final shape, with the pocket's opening beside it where it has one, outside the body and voiding nothing. In Autodesk's open-source exporter (`WallExporter.cs`, `TryToCreateAsExtrusion` and `ExportWall`) a wall is built as an extrusion of its footprint with clippings, and the native call `AddClippingsToBaseExtrusion` hands back the "cut pair" openings that `OpeningUtil.CreateOpeningsIfNecessary` writes; the rule inside it is in `RevitAPIIFC`, closed, so the hull reading is a measurement and not something read. Read that way, the hull's sloped edges are the clip planes, which run on to the wall's end (what #1473 sees as the slope Revit keeps past the profile's end), and a pocket is what no plane can cut.

## 3. What rvt-rs writes

`elevation_profile_pockets` finds the pockets and closes each as section 2 says (pure, with failure-first tests on the numbers of the three walls, a notch in a side, and a hull edge from a corner to a side, which it does not read). The regions RE-151 reads are not dropped: they make the wall's shape, the box less the regions, and the passes that move a wall's ends or carry its profile on (RE-348, RE-358) work on them. After every pass that reads a wall's body or openings (`cut_profiles_from_bodies`), a wall whose regions are all solids has them taken out of its body as one `IfcBooleanResult` difference (each region's outline extruded through the wall, 0.01 ft wider than the wall each side), none written as openings, and its pockets written as its openings, with the Tag the highest line id of the pocket and the name and GlobalId RE-151 gave the region. The wall carries `ProfileCutSource` `wall_profile_cut_in_body`, counted in `exported.wall_profile_cuts`. A wall the cut cannot be made for (no body of its own, a representation map, a region that is not a solid) keeps its regions as its openings and writes no pocket, as before.

UNFILTERED

## 5. Not shown

- **The closures not seen.** Three kinds of hull edge are read: one with an end off the rectangle's edge, closed by the edge (1387731's, whose other end is the rectangle's corner; an edge with the other end on a side has no wall in the five files); one whose ends lie on a side in common, a notch cut in it, where the edge runs along the side and the polygon is the same under any reading (no wall has one); and one whose ends lie on two adjacent sides, neither at a corner, closed through the rectangle's corner between them (55840's). A pocket whose hull edge ends otherwise, at a corner of the rectangle with the other end on a side it does not share, is not written, and the wall's body is cut all the same. None of the 18 walls has one.
- **Why the hull.** The rule is read from 18 walls, two of which have an opening in Revit's export. A pocket on a wall outside Revit 2024, with an arc in its profile (RE-151 leaves a sketch with an arc alone), or in a profile with several loops is not measured.
- **The Tag's order rule.** The pocket's highest line id and the last of its lines in the sketch's order coincide on the three walls that have a pocket (the sketch lists were ascending by id).
- **Wall 34362.** Core Interior's second wall with a pocket is in neither export.

## 6. Reproduce

```bash
# The probe on the five Revit 2024 files with Revit's export:
gh workflow run measure.yml --ref main -f ref=80808447c8c54bcc23ba925287db3a8551466b02 -f base=none \
    -f nordic=true -f models="core-interior nordic-office-as-built nordic-office-permit nordic-office-terrain-*" \
    -f scorers=none -f probe=probe_1607_wall_profile_lines -f probe_scorer=-
```

The scorer runs 42616460564 and 42618842627 (`unfilled_openings_vs_ifc` printing the openings' mesh corners) and 42621689708 (the walls' body forms) are from commits `c9de5fb8`, `4ccce43d` and `067eb7c2` of the branch `measure/1600-revit-openings`, archived under `refs/archive/2026-11-09/cited/`.
