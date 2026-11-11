# RE-430: a face whose trim is not the rectangle of its parameters, or has holes, is meshed in the cells of its grid, and 119 elements of Snowdon Towers Architectural, 43 of each BIM4LCA office and 71 of each `rac_advanced` save are drawn from their saved meshes

**Date:** 2026-11-11
**Issues:** #1706 (the saved-mesh refusals left after #1683: this is its second step); found here: #1732 (planar faces with holes) and #1737 (the triangle density); follows RE-426 (#1709, the first), RE-422 (#1683) and RE-302 (2026-11-03, the same attempt on the office's toilets, not landed)
**Artefacts:** Autodesk's Snowdon Towers Sample Architectural (Revit 2024, as Measure fetches the sample) and Revit's IFC4 export of it (local only: percentages, distances and triangle counts only here, no Revit-side volumes); rvt-rs's exports of the 2024 samples and the reference models by the base and by the change, which Measure kept (`keep_ifc`); RE1 Architecture and Revit's export of it (MIT)
**Probe and scorer:** `tools/re/mesh_bodies_vs_ifc.py`, with `tools/re/cut_ifc.py` (a model's cut for it) and `tools/re/ifc_body_diff.py` (two exports' bodies, object by object), both new here; the pure helpers `trimmed_cells` and `keep_triangle` are tested in `src/research/native_parametric_mesh.rs` (eight failure-first tests for the cells: either winding of a loop, a hole either way round, a diagonal, a bar pair in one strip, two loops sharing an edge, every knot, a bowtie, off-knot and non-finite input; three for the triangle: a needle and a hair, non-finite points, dust against a pole and a real triangle)
**Runs:** Measure, unfiltered with `keep_ifc`: [43041839803](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43041839803) (head `ea9a16d7`, the one this report is about, against `main` `8ec2ac98`) and its `timing` [43041844475](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43041844475); the heads before it, [43026283371](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43026283371) (`8434d29e`) and [43034275062](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43034275062) (`662dff5b`, its `timing` [43034280928](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43034280928)); the diagnostic builds of section 3, [43032254924](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43032254924) and [43032259387](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43032259387)
**Status:** Positive, and costly. 119 elements of Snowdon Towers Architectural and 626 element-exports of the other 23 moved exports are drawn from their saved meshes where they were boxes, none that was drawn changes, and the same-project gates pass. Six of the nine registered rows hold (1, 2, 5, 6, 7, 8); three miss and are reported as measured (section 7): the shares of volume and vertex agreement with Revit's export (row 3: the 102 chairs read 1.0164 and no body has every vertex of Revit's within 0.001 ft), and the IFC growth (rows 4 and 9: Snowdon Towers Architectural +17.3% against 15%, the office +27.8% to +31.2% against 27%). The lever for the growth is the triangle density of the revolved faces, #1737.

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

**The second round.** The run of that rule (Measure 43034275062, `662dff5b`) left 7 elements refused for `noncollapsed degenerate parametric triangle`: a railing in each of the five `rac_advanced` saves and two elements of Snowdon Towers Architectural. Their triangles have two vertices that are one point to the grid's own tolerance: `trimmed_cells` numbers a vertex by its parameters rounded to 1e-9, so two parameters less than a knot tolerance apart can fall either side of a rounding boundary and be two vertices, and on a surface of up to 10 ft to the unit they lie up to 1e-8 ft apart (the ones the 2023 and 2024 samples hold are 1.0e-10 and 6.6e-10 ft; the pole rule allowed 1e-10). A triangle without area whose shortest edge is 1e-8 ft or less is dropped as a pole's is (`COLLAPSED_EDGE`), and the helper's tests gain the 6.6e-10 ft pair with a third point 1.1e-3 ft away (still one point) and a pair 1e-7 ft apart with a foot-long edge (a needle, refused). The five railings are drawn now; the two Snowdon elements read the next reason, `surface coordinates outside envelope`. Between that head's run and the final one, 120 of 126 exports are byte for byte the same (row 8).

## 4. Against Revit's export

The elements the change draws that Revit's own export holds (same `Tag`), scored by `tools/re/mesh_bodies_vs_ifc.py` as Measure runs it (the office and the housing, from Measure's own results; Snowdon Towers Architectural, whose Revit export is local only, on a cut of the 118 elements made from the kept export of the run and from Revit's with `tools/re/cut_ifc.py`, then the same scorer). Volume is rvt-rs's over Revit's; "vertices" is the largest distance from a vertex of Revit's body to rvt-rs's surface.

| model | class, type | n | volume within 1% | median volume | every vertex of Revit's within 0.001 ft | farthest vertex of Revit's |
|---|---|---:|---:|---:|---:|---:|
| BIM4LCA office (4 exports) | `IfcSanitaryTerminal`, `WC-istuin:WC-1` | 43 | 43 | 1.0060 | 0 | 0.0048 ft |
| BIM4LCA housing, concrete at permit | `IfcSanitaryTerminal` | 6 | 6 | 1.0002 | 0 | |
| BIM4LCA housing, concrete at permit | `IfcCovering` | 9 | 9 | 1.0000 | 0 | |
| Snowdon Architectural | `IfcFurniture`, `Chair-Breuer` | 102 | 0 | 1.0164 | 0 | 0.0043 ft |
| Snowdon Architectural | `IfcFlowTerminal`, `Sink-Wall-Barrier Free-3D` | 15 | 15 | 1.0009 | 0 | |
| Snowdon Architectural | `IfcFlowTerminal`, `Faucet_Swing` | 1 | 0 | | 0 | |

The class is the one Revit's export gives (the scorer groups by it); rvt-rs writes the 16 sinks and faucets as `IfcSanitaryTerminal`, as `main` does for the boxes they replace. The scorer measures at most 400 distinct vertices of a body (`VERTEX_CAP`, every nth of the sorted ones), so the farthest vertex in the table is a sample's: a body that fails with the sample fails with all its vertices, which is how the 0 reads, and the bodies read in full below give the exact figures.

Of the 176 elements, 73 have Revit's volume within 1% (all but the 102 chairs and the faucet), and none has every vertex of Revit's within 0.001 ft of rvt-rs's surface; the 102 chairs are one type, one mesh 102 times. The 118 of Snowdon Towers Architectural are its newly drawn elements that carry a Tag, and Revit's export holds all 118; the 119th is an `IfcSpace` (a family instance named `Bandstand`), which the scorer does not read (it scores `IfcElement`s).

A toilet, the pair read in full (element 2634593, the office at building permit): rvt-rs's body is 17,494 triangles in the IFC, 16,325 triangles and 8,371 vertices as IfcOpenShell meshes it (which is what the scorer reads), against Revit's 1,668 triangles and 793 vertices (merged to 1e-6 ft); the bounding boxes agree to 0.0002 ft. 401 of Revit's 793 vertices are vertices of rvt-rs's body to 1e-5 ft. 67 are farther than 0.001 ft from its surface, by the scorer's own exact `surface_distance` over all of them (its sample of at most 400 reads 0.0048 ft as the farthest, all of them 0.0050 ft): 22 lie outside rvt-rs's surface and 45 inside (the largest 0.0050 and 0.0042 ft), 7 at the foot of the bowl (z at the bounding box's bottom, where the nearest triangles of rvt-rs's body are the vertical walls of the base), 28 along the ring between and 32 at the top. rvt-rs's 8,371 vertices lie up to 0.0222 ft from Revit's surface (median 0.0020 ft; `surface_distance` over all of them), as the coarser mesh's flat triangles lie off the curved surface they are inscribed in. Revit's vertices lie on both sides of rvt-rs's surface, so neither body is the other's inscribed mesh; which of the two is nearer the saved surface at these places is not decided here.

## 5. The chair read in full

The chairs are the shape RE-426 section 5 read the railings by. Element 738616: rvt-rs's body is 5 shells and Revit's 7; the two large shells agree to 1e-3 in volume (1.0004 and 1.0006 of Revit's) and the other three of rvt-rs's, round tubes, hold 1.126 of the volume of the five of Revit's together: a polygon inscribed in a circle holds less than the circle, and the tubes' excess is the 1.64% of the whole body (the ratio 1.0164). Revit's vertices farther than 0.001 ft from rvt-rs's surface are 81 of 1,106, the largest 0.00427 ft; they lie on both sides of rvt-rs's surface (37 above and 44 below along the nearest triangle's normal), one within 0.005 ft of an open edge of rvt-rs's mesh (the median distance to the nearest open edge is 1.25 ft), and the nearest triangles have edges of 0.011 to 0.027 ft: they are not holes in rvt-rs's body but the places where Revit's coarser tessellation differs from the surface by the chord error of its own edges. The box the change replaces is 13.49 times Revit's volume, the mesh 1.0164.

## 6. The other models, and what is still refused

Measure `keep_ifc`, 126 exports, `main` against the change (section 8): 101 identical and 25 moved, every one by elements drawn from their saved meshes where they were boxes, and the refusal reasons that moved with them:

| exports | elements drawn instead of boxed, in each | classes | was refused for |
|---|---:|---|---|
| BIM4LCA office (4) | 43 | `IfcSanitaryTerminal` (the toilets) | `nonrectangular parametric trim` |
| BIM4LCA housing, the four large | 15 | 9 `IfcCovering`, 6 `IfcSanitaryTerminal` | `nonrectangular parametric trim` |
| BIM4LCA housing, the two terrain | 9 | `IfcFurniture` | `nonrectangular parametric trim` |
| Autodesk `rac_advanced` 2023 to 2027 (5) | 71 | 70 `IfcFurniture`, 1 `IfcRailing` | `nonrectangular parametric trim` |
| Autodesk `rac_basic` 2023 to 2027 (5) | 1 | `IfcSanitaryTerminal` | `multiple parametric trim loops unsupported` |
| Snowdon Towers Architectural | 119 | 102 `IfcFurniture`, 16 `IfcSanitaryTerminal`, 1 `IfcSpace` | `nonrectangular parametric trim` |
| Snowdon Towers Plumbing | 13 | `IfcSanitaryTerminal` | `nonrectangular parametric trim` |
| Snowdon Towers Structural | 2 | `IfcBeam` | `nonrectangular parametric trim` |
| RE1 Architecture | 1 | `IfcSanitaryTerminal` | `nonrectangular parametric trim` |

One held-out file moves as well; its results hold counts only. Of Snowdon Architectural's 119, 91 were `partition_element_record_bbox` bodies (74 `IfcFurniture`, 16 `IfcSanitaryTerminal` and the `IfcSpace`) and 28 `partition_family_instance_turned_box` ones (`IfcFurniture`), so `body_sources` moves from 583 to 492 (`partition_element_record_bbox`), from 118 to 90 (`partition_family_instance_turned_box`) and from 4,163 to 4,282 (`saved_graphics_mesh`).

No element drawn at `main` changes. `tools/re/ifc_body_diff.py` on each of the 24 moved exports whose IFC both sides kept (text only: per GlobalId the class, BodySource, SavedMeshRefusal and a fingerprint of each face set's points and indices) finds 81,529 objects drawn from a saved mesh on both sides, which is `main`'s `body_sources.saved_graphics_mesh` summed over the 24, and 0 whose meshes differ; every BodySource that changes is a box becoming a saved mesh, so none that was drawn is refused; the objects are the same set on both sides (0 only in `main`'s, 0 only in the change's); and no other object changes geometry (a box is compared by its IFC type): only refusal texts change, 27 in Snowdon Architectural, 23 in each office and those listed below. Snowdon Architectural alone: 4,163 drawn in both, 0 differ, 119 changed.

Elements that keep their box with another refusal text than before (a face that now meshes lets the next one be read), by export: Snowdon Architectural 27, the four offices 23 each, the four large housing exports 7 each, `rac_basic` 8 per save, RE1 Architecture 1.

What the change leaves refused, by reason, in the `saved_mesh_refusals` of the final head (`main`'s count in brackets where it differs). The office at permit, 146 entries at `main` and 103 now: `multiple cylinder trim loops unsupported` 30 (a cylinder face with several loops, the next step of #1706), `unsupported parametric profile curve GHermiteSpline` 19, `a trim loop does not close in the surface's parameters` read beside `disconnected UV boundary` and a missing surface 15 (0; the revolution's seam or a pole, which the rule refuses as before), `scaled instance mode is not qualified` 10, `unsupported surface HermiteSurf` 9 (1). Snowdon Architectural, 388 and 269: 127 `no_saved_mesh` and 26 `whole_drawn_by_its_parts` (not candidates), `face regions need subdivision` 22 and 21 more with `multiple cylinder trim loops unsupported`, `triangulated area mismatch` 21 (0), `unsupported surface HermiteSurf` 19 (18), `GHermiteSpline` 14, `multiple cylinder trim loops unsupported` 12 (11), `surface coordinates outside envelope` 2 (0). The 21 are the elements that were `multiple parametric trim loops unsupported`, all `IfcSanitaryTerminal`: a plane face with holes fails the planar path's area check once the element's ruled faces mesh ([#1732](https://github.com/DrunkOnJava/rvt-rs/issues/1732)); they keep their boxes, as they did.

## 7. The registered rows

Rows 1 to 5 were registered on #1706 before the first run ([comment 6795159034](https://github.com/DrunkOnJava/rvt-rs/issues/1706#issuecomment-6795159034)) and rows 6 to 9 before the re-run of the sliver rule ([comment 6797828528](https://github.com/DrunkOnJava/rvt-rs/issues/1706#issuecomment-6797828528)). Each is read below on the run of section 8 (head `ea9a16d7` against `main` `8ec2ac98`, unfiltered, `keep_ifc`).

1. **Held.** At least 20 of the elements refused for `nonrectangular parametric trim` or `multiple parametric trim loops unsupported` are drawn. 119 of the 146 elements of Snowdon Towers Architectural that named one of the two (123 of the first, 21 of the second, 2 beside another reason) are drawn from their saved meshes; the other 27 are in sections 6 and 8.
2. **Held.** No element drawn at `main` changes, and none that was drawn is refused. Over the 24 moved exports whose IFC both sides kept, 81,529 objects are drawn from a saved mesh on both sides and 0 of their meshes differ; every BodySource that changes is a box becoming a saved mesh, and no box changes geometry (`tools/re/ifc_body_diff.py`).
3. **Missed on both shares; the classes are listed and the shapes read, as the row asks.** Of the 176 newly drawn elements that Revit's export holds, 73 (41%, 80% asked) have Revit's volume within 1%, and 0 (50% asked) have every vertex of Revit's within 0.001 ft of rvt-rs's surface. The volume share misses because of one type: 73 of the other 74 are within 1% (the office's 43 toilets, the housing's 15, Snowdon's 15 sinks) and the 102 chairs read 1.0164, the excess of three round tubes that Revit draws as inscribed polygons (section 5). The vertex share misses everywhere because Revit's own tessellation differs from rvt-rs's surface by up to 0.0043 ft (the chairs) and 0.0050 ft (a toilet) at the boundaries of faces, on both sides of it (section 4); the row's 0.001 ft assumed Revit's vertices lie on rvt-rs's surface, as they do on the bars of the railing RE-426 read in full, and these do not. Which mesh is nearer the saved surface there is not decided.
4. **Missed on the IFC, held on time and memory.** Snowdon Architectural's IFC grows from 229.7 to 269.4 MB, 17.3% (15% asked); its export time reads 1.02 and its peak memory 1.02 of `main`'s (`timing`, three rounds of both builds on one runner). RE-302's cost on the office (+27% for 43 toilets) was the one to beat; it is not beaten (row 9).
5. **Held.** 101 of 126 exports are byte for byte `main`'s (`ifc.sha256`). The 25 that move are listed by class and count in section 6, and by Tag in the comment on #1706 that reports this run (11 distinct sets for the 24 exports with an IFC, 745 element-exports; the `IfcSpace` by GlobalId).
6. **Held, on the results that can show it.** None of the 101 results that hold diagnostics lists `noncollapsed degenerate parametric triangle` among its `saved_mesh_refusals`, on either side. The other 25 (18 held-out, 6 `racadv` and 1 BIMcollab) hold counts only, by design, and show no refusal text.
7. **Held.** Every element whose only refusal was that triangle is drawn, or its next reason is listed. From the first head to the second, `body_sources.saved_graphics_mesh` rises by the 43 toilets of each office, 6 in each large housing export and 9 in each terrain one, 10 in each `rac_advanced` save, the Plumbing's 13, 1 in each `rac_basic` save and 15 in Snowdon Architectural; the final head draws 1 more in each `rac_advanced` save, a railing (11 in all, the 11 the registered text counts) and 2 elements of Snowdon Architectural read `surface coordinates outside envelope` (section 3).
8. **Held, read against the second head.** The first head's run (`8434d29e`) kept no IFC, so the row is read in two parts. In diagnostics, `body_sources.saved_graphics_mesh` rises or stays in all 101 results that hold them (22 rise, 79 stay, none falls) from the first head to the second. From the second head (`662dff5b`, whose IFC is kept) to the final one, 120 of 126 exports are byte for byte the same, and in the six that differ (the five `rac_advanced` saves and Snowdon Architectural) 30,462 objects are drawn from a saved mesh on both sides and 0 meshes differ (`tools/re/ifc_body_diff.py`); the differences are the 5 railings drawn and the refusal text of 2 elements of Snowdon's, which loses `noncollapsed degenerate parametric triangle` and keeps `surface coordinates outside envelope`.
9. **Missed on the IFC and on the vertices, held on the volume.** The office's 43 toilets read 1.0060 of Revit's volume (43 of 43 within 1%); 0 of 43 have every vertex of Revit's within 0.001 ft (row 3); the office's IFC grows from 95.9 to 122.5 MB, 27.8%, in the as-built and permit exports and from 85.9 to 112.5 MB and from 85.4 to 112.0 MB, 31.0% and 31.2%, in the timber ones (27% asked). Snowdon Architectural is row 4's. The lever the row names is the density of the grid on the revolved faces: rvt-rs writes 17,494 triangles per toilet where Revit's export has 1,668 ([#1737](https://github.com/DrunkOnJava/rvt-rs/issues/1737)).

## 8. The whole unfiltered run, and what the change costs

Measure [43041839803](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43041839803), unfiltered with `keep_ifc` and every model set (`samples`, `nordic`, `r2025`, `holdout`, `bimcollab`, `racadv`), `ea9a16d7` against `main` `8ec2ac98`: 126 exports, 101 identical and 25 moved (24 with an IFC, one held-out file with counts only). The same-project gates pass. `timing` [43041844475](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43041844475): both builds on one runner, three rounds, 74 files.

What the change costs is in triangles. Each element drawn from a revolved or ruled face carries thousands (rvt-rs's grid is 0.001 ft of chord; Revit's export of the same toilet has 1,668 triangles):

| exports | elements added, each | triangles added, each | per element | IFC, `main` to the change |
|---|---:|---:|---:|---|
| BIM4LCA office (4) | 43 | 752,242 | 17,494 | 95.9 to 122.5 MB (+27.8%) as built and at permit; 85.9 to 112.5 (+31.0%) and 85.4 to 112.0 (+31.2%) timber |
| BIM4LCA housing, the four large | 15 | 150,596 to 151,748 | 10,039 to 10,116 | 18.8 to 24.2 (+29.0%), 18.7 to 24.1 (+29.1%), 20.3 to 25.8 (+27.0%), 20.0 to 25.5 (+27.3%) |
| BIM4LCA housing, the two terrain | 9 | 122,281 | 13,586 | 8.7 to 14.1 MB (+61.0%) |
| `rac_advanced` 2023 to 2027 | 71 | 759,222 | 10,693 | 52.9 to 81.1 (+53.2%), 53.6 to 81.8, 50.8 to 79.0, 50.3 to 78.5, 50.3 to 78.5 (+56.1%) |
| `rac_basic` 2023 to 2027 | 1 | 72,321 | 72,321 | 29.8 to 32.4 MB (+8.6%) |
| Snowdon Architectural | 119 | 1,062,436 (+20.6%) | 8,928 | 229.7 to 269.4 MB (+17.3%) |
| Snowdon Plumbing | 13 | 60,528 | 4,656 | 122.3 to 124.7 MB (+2.0%) |
| Snowdon Structural | 2 | 9,304 | 4,652 | 14.3 to 14.6 MB (+2.2%) |
| RE1 Architecture | 1 | 6,094 | 6,094 | 1.2 to 1.4 MB (+21.7%) |

Triangles are the sums of `rvt-ifc.log`'s "drawn from their saved graphics" lines; the office at permit goes from 838,380 to 1,590,622. Export time and peak memory (`timing`, 74 files) read 0.98 to 1.09 and 0.95 to 1.06 of `main`'s; on the 69 other files they read 0.98 to 1.03 and 0.95 to 1.02, and the only ones above that are the `rac_advanced` saves (time 1.08, 1.09, 1.05, 1.06, 1.05 and peak 1.06, 1.05, 1.03, 1.05, 1.05 for 2023 to 2027), where 71 elements add 759,222 triangles to a pass of 6 to 8 seconds; Snowdon Architectural reads 1.02 and 1.02, the offices 1.00 to 1.02 and 1.00 to 1.01. The runners were AMD EPYC 9V45, 9V74 and 7763; each file's three rounds ran on one runner.

## 9. Reproduce

```bash
# Measure, unfiltered, with the exports kept (the run of section 8); the scorers come from main, the code from the archived head
gh workflow run measure.yml --ref main -f ref=ea9a16d7705ebb843ed9b817911872a5597fec4a -f base=8ec2ac98d3c982b83d0cdefe378d8c0e4a2df50d \
  -f samples=true -f nordic=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true -f keep_ifc=true
# the same without scorers, on one runner for both builds (the timing of section 8)
gh workflow run measure.yml --ref main -f ref=ea9a16d7705ebb843ed9b817911872a5597fec4a -f base=8ec2ac98d3c982b83d0cdefe378d8c0e4a2df50d \
  -f samples=true -f nordic=true -f timing=true -f scorers=none
```

`ea9a16d7705e` is archived as `refs/archive/2026-11-11/cited/ea9a16d7705e` (the PR's head when it was measured, which a squash leaves out of `main`; the commits after it change the report, the changelog, the reconnaissance addendum and two tools, and no source). The kept exports of both sides are in the run's `measure-head-*` and `measure-base-*` artifacts (`<model>/model.ifc`, with `rvt-ifc.log`, whose "drawn from their saved graphics" line gives the triangles of section 8, and `diagnostics.json`).

```bash
# sections 6 and 7: the bodies of one model's two exports, object by object (no geometry kernel; a 270 MB export takes about ten minutes)
python3 tools/re/ifc_body_diff.py <main model.ifc> <change model.ifc> [newly-drawn.txt]
# section 4, on a model whose Revit export is local only: cut the Tags of one type from both files, then score the cuts
python3 tools/re/cut_ifc.py <rvt-rs model.ifc> <rvt-rs cut.ifc> <Tag> [<Tag> ...]
python3 tools/re/cut_ifc.py <Revit model.ifc> <Revit cut.ifc> <Tag> [<Tag> ...]
python3 tools/re/mesh_bodies_vs_ifc.py <rvt-rs cut.ifc> <Revit cut.ifc>
```

The exact distances of section 4's toilet are the scorer's own `surface_distance(points, triangles)` over every distinct vertex of a body (`np.unique(triangles.reshape(-1, 3), axis=0)`) in place of `sample(triangles)`, which keeps at most 400 of them (`VERTEX_CAP`) and is why the table's farthest figure (0.0048 ft) is lower than the exact one (0.0050 ft); the counts of the vertices on each side and of the 67 are of the vertices merged to 1e-6 ft.
