# RE-426: a plane that meets a cylinder along an edge across both its parameters is meshed: 126 more elements of Snowdon Towers Architectural and 49 of the other 2024 samples are drawn from their saved meshes

**Date:** 2026-11-11
**Issues:** #1706 (the saved-mesh refusals left after #1683: this is its first step); follows RE-422 (#1683, the four ruled-surface roofs), RE-407 (multi-loop cylinder faces) and RE-420
**Artefacts:** Autodesk's Snowdon Towers Sample Architectural (Revit 2024, as Measure fetches the sample) and Revit's IFC4 export of it (local only: percentages, distances and triangle counts only here, no Revit-side volumes); rvt-rs's exports of the 2024 samples and the reference models by `main` and by the change, which Measure kept (`keep_ifc`)
**Probe and scorer:** `probes/examples/probe_1706_shared_edges.rs`, `tools/re/mesh_bodies_vs_ifc.py`
**Runs:** Measure [42998375045](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42998375045) (the probe, Autodesk's 2024 samples), [43000891419](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43000891419) (`keep_ifc`, the rule on its own) and [43002827683](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43002827683) (`keep_ifc`, with the envelope guard of section 3); the baseline is [42993080184](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42993080184) (`keep_ifc` of `main` 16411282)
**Status:** Positive on 102 of the 126 elements of Snowdon Towers Architectural, which Revit's export lets be read: 100 building element proxies, a slab and a roof agree with Revit's body (volume within 1%, every vertex within 0.001 ft). The 24 railings are drawn and 22 of them read 1.21 to 1.26 times Revit's volume (two glass-panel railings are within 1%), as the 101 railings drawn before this change do, for the polygons Revit's export draws round bars with (section 5); the volume bar registered for them misses and they stay drawn (section 7). 49 elements of five other 2024 samples are drawn too, none of them scored (section 6). No element that was drawn changes, and 19 stay boxes for the next reason.

## 1. The question

After #1683, Snowdon Towers Architectural still had 559 products with a `SavedMeshRefusal` (#1706), and 141 of them were refused for a shared edge that `refined_plane_edge` would not follow: 116 `shared curved edge is not an isoparametric interval` and 25 `shared curved edge changes its fixed parameter`. A plane face that meets a curved face refines the edge it shares with it in the curved face's parameters so that its polygon passes through the curved face's grid lines, and for that it needs the edge to run along one parameter, `u` or `v`, the other being constant. A slanted plane through a cylinder meets it along an ellipse in its parameters, and a mitred rail along a curve that is neither: no grid line to follow. What do those edges look like, and can the plane's polygon follow them without one?

## 2. What the edges are

`probe_1706_shared_edges` (Measure 42998375045) reads every record the saved-mesh pass reads and classifies each edge that has a plane on one side and a `CylSurf`, `SurfRev`, `RuledSurf` or `ConeSurf` on the other. In Snowdon Towers Architectural:

| curved face | plane-to-curved edges | along `u` | along `v` | along neither | curved faces of those | that mesh through the pass |
|---|---:|---:|---:|---:|---:|---:|
| `ConeSurf` | 114 | 98 | 16 | 0 | | |
| `CylSurf` | 14,421 | 8,492 | 5,281 | **648** | 501 | 608 (30 have no trim loop, 10 several) |
| `RuledSurf` | 990 | 468 | 433 | 89 | 54 | 0 (74 `nonrectangular parametric trim`, 6 several loops) |
| `SurfRev` | 1,862 | 735 | 946 | 181 | 93 | 0 (155 `nonrectangular parametric trim`, 26 several loops) |

The 648 cylinder edges hold 3 to 17 samples (median 7). The chord between two neighbouring samples, against the surface point at the middle of their parameters, is within 0.001 ft on 484 of the 648 edges (351 within 1e-4, 113 within 5e-4, 20 within 1e-3) and beyond it on 164 (160 within 5e-3, 4 beyond); that surface point is within 1e-3 ft of the plane on 638 of 648 (518 within 1e-4, 90 within 5e-4, 30 within 1e-3) and within 5e-3 on all. So following the edge's own samples is within the bar for three edges in four, the interpolation in the cylinder's parameters stays near the plane everywhere, and the rest need points between the samples. The ruled and revolved faces are refused on their own trims; they are the next step. Snowdon Structural has 8 such cylinder edges and 16 on revolved faces.

## 3. The rule

An edge across both parameters takes, between each pair of its samples, the points of the edge itself that keep the chord between neighbours within 2.5e-4 ft (a quarter of the 0.001 ft bar) of the edge. The parameter that changes more between two samples is held at its middle value and the other is solved by Newton's method until the point of the surface is within 1e-9 ft of the plane, which is a point of the edge between them; where it is more than 2.5e-4 ft from the chord it goes between them, and each half is looked at in turn, to 32 halvings (a halving takes off a half to three quarters of a chord's deviation, and a smooth edge settles in a dozen). Two samples closer than the bound are left as they are. The edge is refused as before when a point does not reach the plane, when it lies farther from the middle of the chord than the chord is long (the other branch of the intersection is a cylinder's diameter away), when a span is still above the bound after the 32nd halving (an edge that never settles), or when one edge asks for more than 100,000 points.

The first version judged each chord against the surface at the middle of the two samples' parameters, which is how far the chord cuts the cylinder and not how far it is from the edge, and refused a point more than four sagittas from the chord's middle, which refused cylinders of radius 20 at quarter turns (a standalone copy of the helper under `rustc --test`, not kept, fails them with `left its span`) and stopped at the sixth halving without telling. The review of this PR (rvtDevelop) asked that a span above the bound at the limit be an error, and the change that answered it tested the solved parameter against the two samples' own values instead, which refused every closed ellipse: an interval that holds an extremum of the cut has the edge past both its samples (rvtDevelop's simulation, and Measure 43012868588 on `f59e822d`, where the 24 railings were boxes again). The chord's length is the bound now, and the tests include a full turn sampled off its extrema (8, 12, 24 and 48 samples), samples that straddle an extremum, and a steeper plane whose cut has to be solved for `u`.

`refine_crossing_edge` is pure and has thirteen failure-first tests (a full turn off its extrema, an interval that straddles one, a steep cut, a quarter turn of a wide cylinder in two samples, samples already within the bound, a plane far from the span, a span above the bound at the depth limit and an edge over the point budget (each pinned to its own message by passing the limits in), a surface that gives no point or no number, one sample, the isoparametric test, and the list both sides take); each checks the refined chords against the true edge from the formula, not the solver. Both faces take the list, the plane's polygon through the projection of each point onto the plane and the cylinder's loop as the parameters themselves, and both leave out a point of the edge beyond the cylinder's envelope, the samples the graph gave staying wherever they are: the envelope is made from the samples, the edge between two of them can reach a sagitta past it, and `cylinder_face` refuses a loop outside its envelope, so their vertices along the edge are the same. The first run of the rule (43000891419) drew 102 elements and turned the 24 railings that had `changes its fixed parameter` into `cylinder trim outside envelope`; the guard on the cylinder's loop is the second run's change, and the same guard on the plane's polygon is the review's.

An edge along `u` or `v` is as before, so no element that was drawn changes.

## 4. Against Revit's export

Measure 43002827683 kept the exports of its 16 models. The 126 elements it newly draws are read against Revit's IFC4 export of Snowdon Towers Architectural (local only) with `tools/re/mesh_bodies_vs_ifc.py`, which pairs elements by `Tag` and measures each body's volume and how far each body's vertices lie from the other's surface. "Ref on ours" is every tested vertex of Revit's body within 0.001 ft of rvt-rs's surface, which tells a mesh of the same surface from a mesh of another where Revit's export is the coarser (RE-311).

| class | drawn now | volume within 1% | every vertex of each within 0.001 ft of the other's surface | ref on ours | median volume ratio |
|---|---:|---:|---:|---:|---:|
| `IfcBuildingElementProxy` (99 `Wall Sweep:Base_Simple`, 1 `Motion_Towel_Dispenser`) | 100 | 100 | 99 | 100 | 1.0005 |
| `IfcSlab` (`Floor:Hardscape - Brick Walk`) | 1 | 1 | 1 | 1 | 1.0000 |
| `IfcRoof` (1916596, `Basic Roof:Insulation on Metal Deck - EPDM`) | 1 | 1 | 1 | 1 | 0.9997 |
| `IfcRailing` | 24 | 2 | 0 | 3 | 1.2132 |

The 102 that are not railings agree with Revit's body: the towel dispenser is within 0.01 ft and not 0.001 ft, the slab's volume is within 0.01%, and roof 1916596, the second of the two roofs RE-422 left as blocks (a plane's edge across a curved face that is not isoparametric), reads 0.9997 of Revit's volume with every vertex of each body within 0.001 ft. The railings are section 5.

## 5. The 24 railings

**What they read.** Two are within 1% (the two `Handrail - Glass Panel w Brackets`); the other 22 read 1.21 to 1.26 times Revit's volume, and Revit's vertices lie on rvt-rs's surface for 3 of the 24. The railings that `main` 16411282 already draws from their saved meshes read the same way on the same scorer: 101 of them in this model, 34 within 1%, 29 with Revit's vertices on rvt-rs's surface, median 1.1758. The pipe railings (`Pipe-Wall Mount` 31, `_IBC-Pipe-Plate Stringer` 23) are all outside 1% and `Road Striping` (17) and `Handrail - Balcony` (8) are all inside it. The 24 are not a new kind of disagreement.

**Why: the shells.** Revit's export draws a round section as a polygon inscribed in the circle that rvt-rs's mesh draws with more sides (RE-397 section 5.2, #1658): a hexagon where the mesh has a 24-gon, a quadrilateral on thinner bars. The volumes then differ by the polygons' area ratio, 1.1954 for a 24-gon over a hexagon, which a volume bar of 1% reads as a difference. `tools/re/volume_readings.py` reads the 22 railings shell by shell: it pairs 1,937 shells, reads 1,792 of them as explained (volume ratio 1.19 to 1.21, the area ratio of the round end faces) and does not read 145, and 10 shells of rvt-rs's and 37 of Revit's have no partner. The 145 give two reasons: 72 have no round end face in rvt-rs's shell, and 73 have a round end face with no inscribed polygon in Revit's. The railings drawn before the change are read the same way (365 shells explained and 120 not on the 27 the scorer prints).

**One railing read in full.** Railing 622056 (`_IBC-Pipe-Plate Stringer (Extension None)`) has 101 shells in each file, paired one to one; 95 are explained. The 6 that are not are round bars 3.09 ft long cut at a slant at both ends: Revit's export writes each as a prism of 8 vertices and 12 triangles, rvt-rs's mesh as 130 vertices and 236 triangles, and the volume ratio is 1.4970. The scorer cannot read them because the end face of a slanted cut is an ellipse and `circle_of` takes only circles. All 8 of Revit's vertices are vertices of rvt-rs's mesh (the distance to the nearest is 0). The three largest shells, 19 ft across, have Revit's vertices up to 0.01728 ft from rvt-rs's surface and rvt-rs's up to 0.01268 ft from Revit's; at a rounded corner of a flat face in one of them the two meshes share the ends of an arc, rvt-rs's 8 vertices lie on a circle of radius 0.2296 ft to 0.00004 ft, and Revit draws the arc's 44.9 degrees as one chord with a vertex at its middle, 0.01741 ft from the arc. The miss of "ref on ours" there is Revit's chord.

**What this shows and what it does not.** On the railing read in full the differences are Revit's coarser polygons and chords: wherever Revit draws a bar its vertices lie on rvt-rs's mesh, and where it draws a corner as a chord rvt-rs's mesh follows the arc. It does not show that for the other 23: there the scorer's own reading is the evidence, and the 145 shells it does not read are named by the two reasons above. A reading of bars with slanted ends is a follow-up (the scorer, #1712).

**The call.** The 24 stay drawn. A railing the pass refused was written as its record box, whose median fill of Revit's body is 0.001 on the 109 railings #1635 measured (about a thousand times Revit's volume); each is now a body that follows Revit's own surfaces to the chord bounds the pass holds (0.001 ft on a surface, 0.00025 ft on these edges), whose volume differs from Revit's by the polygons Revit's export writes. That is the state of the 101 railings drawn since #1191 and shipped in 0.18.0, whose status already says a mesh is not always Revit's.

## 6. The other samples, and what is still refused

| model (Revit 2024) | was a box, drawn now | what | refused with a shared-edge reason, before then after | drawn before, differing now |
|---|---:|---|---:|---:|
| Snowdon Towers Architectural | 126 | 100 proxies, 24 railings, a slab, a roof | 145, 0 | 0 of 3,837 |
| Snowdon Towers Structural | 0 | | 0, 0 | 0 of 83 |
| `rac_advanced` | 21 | 13 railings, 8 proxies | 21, 0 | 0 of 5,021 |
| `rac_basic` | 3 | 3 railings | 4, 0 | 0 of 318 |
| `rme_advanced` | 8 | 8 railings | 8, 0 | 0 of 4,873 |
| `rme_basic` | 8 | 8 railings | 8, 0 | 0 of 4,958 |
| `rst_basic` | 9 | 9 beams (`partition_beam_box_along_axis`) | 9, 0 | 0 of 67 |

175 elements are drawn from their saved meshes that were boxes, 126 of them scored (section 4) and 49 not: no Revit export of those five samples exists, so their bodies rest on the same rule, the same-project gates (`tools/re/bodies_across_saves.py`) and the scorer's reading of the 24 railings above. 8 of the run's 16 exports are byte-identical (Core Interior, Einhoven, three RE1 files, three Snowdon files). RE1 Architecture is unchanged but for the refusal text of 2 furniture boxes.

Snowdon Towers Architectural has 437 products with a `SavedMeshRefusal` now, from 563. The 19 elements that lost a shared-edge reason and stay boxes are refused for the next one: 16 sanitary terminals and 2 furniture elements for `nonrectangular parametric trim` (a revolved or ruled face whose trim is not a rectangle; 122 elements of the model are refused for it alone), and a sanitary terminal for `disconnected UV boundary; multiple cylinder trim loops unsupported`. The shared-edge refusals of the ruled and revolved faces (89 and 181 edges, section 2) are that reason's: the next step of #1706 is a mesher for a nonrectangular trim, not another edge rule.

## 7. The registered rows

Registered on #1706 before the first run (https://github.com/DrunkOnJava/rvt-rs/issues/1706#issuecomment-6792974651) and read here for both: the rule on its own (Measure 43000891419, head `547f2f27`) and the rule with the envelope guard of section 3 (43002827683, head `ea1f01fd`, the head measured and the one this report describes).

| row | the bar | 43000891419 | 43002827683 |
|---|---|---|---|
| 1 | the 141 shared-edge refusals fall to 100 or fewer, at least 20 drawn | held: 102 drawn; the 24 railings refused for `cylinder trim outside envelope` | held: 126 drawn; no element names a shared-edge reason now (145 did: the 141 of the probe and 4 that name it after another), 19 stay boxes for the next reason |
| 2 | no element drawn at `main` changes | held | held: 0 of 19,207 drawn elements in eight models |
| 3 | at least 80% of the newly drawn that Revit's export holds have Revit's vertices on rvt-rs's surface, and each has its volume within 1% | held for the 102 | **missed for the volume bar**: Revit's vertices on rvt-rs's surface for 105 of 126 (83.3%); volume within 1% for 104 of 126 (82.5%), the 22 railings outside it |
| 4 | no element that was drawn is refused now | held | held |

Row 3 said that on a miss the step is narrowed to the edges whose chords the graph's samples already hold (484 of 648) and the refinement looked at for the rest. The refinement is looked at: it is not what the railings miss. Their chords are within 0.00025 ft by construction, the same 1.18 holds for the 101 railings drawn without any crossing edge, and the shells that differ differ by Revit's polygons (section 5). Narrowing would return the 24 railings to boxes about a thousand times Revit's volume, so it was not done; this is the maintainer's call and is on the record here for review.

## 8. Reproduce

```bash
# the exports (a commit, not a branch; the tooling comes from --ref)
gh workflow run measure.yml --ref main -f ref=ea1f01fd073acf21020fd25245f713af426f59ea -f base=none -f samples=true -f sample_years="2024 2024" -f keep_ifc=true -f scorers=none
gh run download <run> -n measure-head
# the baseline is the same run at 16411282 (Measure 42993080184)
# the scorer, on the exports cut to the changed Tags and the local Revit IFC4 export
python3 tools/re/mesh_bodies_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
# the probe of section 2 (Measure 42998375045 ran it on the 2024 samples)
cargo run --profile ci -p rvt-probes --example probe_1706_shared_edges -- MODEL.rvt
```

`ea1f01fd` is kept under `refs/archive/2026-11-11/cited/ea1f01fd073a`.
