# RE-495: Two cylinder faces take the points where the two cylinders meet

**Date:** 2026-11-20
**Issues:** #2104 (step 1), #2107 (this change); #1191 (W6), #2102 (RE-494), RE-492, RE-493.
**Artefacts:** the BIM4LCA office and housing files (VTT, CC BY-SA 4.0; Revit 2024), each with Revit's own IFC4 export; RE1 Architecture and Plumbing (Revit 2025, MIT); the held-out house; Autodesk's sample projects of Revit 2015 to 2027 and Snowdon Towers' 2024 models, which have no Revit export in Measure. Outside Measure, kept local: Revit's IFC4 export of Snowdon Towers Architectural (bldrs-ai/test-models, no licence).
**Probes:** `probe_2098_face_cracks`, printing every body's pairs of surfaces, zeros included, and a `cylpair` line for each body that reaches an edge between two cylinders whose saved samples run across both parameters on both sides: the same file on this change (`97a54e71`) and on `main` at `d089085b` (`2b57a4d9`); and `probe_2104_pair_edges` (`57e5c29a`), how far an edge's saved points lie from each of its two cylinders. Not landed; kept as `refs/archive/2026-11-20/measure-2104-cand`, `refs/archive/2026-11-20/measure-2104-main-base` and `refs/archive/2026-11-20/measure-2104-pair-edges`. The reads of Revit's export of Snowdon Towers Architectural are `row5.py` and its helpers, kept as `refs/archive/2026-11-20/measure-2107-reads` (`106bef91`, `research-tools/2107/`).
**Status:** positive. Recorded at landing with rows 5 and 6 failed; read since, in part (§5, #2111), row 5's move was the scorer's vertex sample, and on the bodies read row 6's count moved with the triangulation at a threshold equal to the chord bound. On the 77 files the probe reads, no boundary vertex between two cylinders is left over 0.001 ft, against 3,170 on `main`, and nothing else parts. Two of the eight rows registered in the pull request failed: row 5, as one toilet on RE1 Architecture leaves `mesh_bodies_vs_ifc`'s 0.01 ft column, and row 6, as 45 of the 62 bodies that move on Snowdon Towers Architectural fit Revit's export worse on at least one count; see §5. The triangles of the bodies the probe reads grow by up to 1.35 on the plumbing and HVAC files. Griffin chose to land it with the fit loss on record and its cause read next, and kept that choice once told the triangle cost (2026-11-20); landing with those rows failed is outside the outcomes registered in the pull request, by that decision. The rows were registered in the pull request before each run was read.

## 1. The rule

Where a cylinder face shares an edge with another cylinder and both faces' saved samples of that edge run across both surface parameters (neither is isoparametric, as RE-492 decides it at a plane), `refined_curved_edge` hands the edge to `refined_cylinder_pair_edge`:

- side 0 bisects between the edge's saved samples in its own parameters, solving each new point onto side 1's cylinder with `bisect_crossing_edge`, until the chord between neighbours is within `EDGE_SAGITTA` (0.00025 ft) of the edge, as `refine_crossing_edge_within` does where a cylinder meets a plane;
- side 1 takes the same points in its own parameters (`CylinderFrame::uv_of`, on the turn nearest its neighbour);
- the saved samples are kept as stored, and a point beyond either face's envelope is left out by both;
- an edge that does not qualify keeps the samples the graph stored, as on `main`, and so does a refinement that fails, apart from the cylinder's point budget, which refuses as it does at a plane. How many refinements fail is not counted.

Each face still puts its own strip points between the shared ones, so whether the faces meet within 0.001 ft is measured, not given. Before this, each face added its own steps between the saved points on its own surface, and on a tee's two-loop face and the cylinder beside it none of the points over 0.001 ft was held by the other face (#2104, run [44686787125](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44686787125)).

A first version moved boundary vertices onto the straight line between saved points. It also closed every crack, but on Snowdon Towers Architectural 23 of its 53 moved bodies fitted Revit's export worse, and it was rebuilt as above (Griffin, 2026-11-20).

The edge's saved points lie on both cylinders within 0.000001 ft on every 2024 sample and within 0.00009 ft on RE1 Architecture ([44727344384](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44727344384), `probe_2104_pair_edges` at `57e5c29a`): the saved points are where the two cylinders meet.

## 2. Measured

**The probe on every file** ([44730930391](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44730930391); 77 files of the reference models and samples). Rows 1 and 3 hold, and row 2's part read here holds.

| file | pair of surfaces | over 0.001 ft on `main` | with this change |
|---|---|---:|---:|
| Snowdon Towers HVAC | cylinder / two-loop cylinder | 1,152 | 0 |
| Snowdon Towers Architectural | cylinder / cylinder | 604 | 0 |
| Snowdon Towers Plumbing | cylinder / cylinder | 378 | 0 |
| `rac_basic`, 2023 to 2027 (each) | cylinder / cylinder | 144 | 0 |
| BIM4LCA housing, four files (each) | cylinder / cylinder | 72 | 0 |
| RE1 Architecture | cylinder / cylinder | 28 | 0 |

- No other pair of surfaces gains a vertex over 0.001 ft on any file, and no body gains one.
- The same 5,927 bodies are read on both sides, and none is newly refused, changes its refusal or changes its face count.

**The full Measure** ([44730938605](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44730938605), `ed2d342d` against `main` `d089085b`; nordic, r2025, samples, holdout, every scorer, exports kept). Rows 2, 4 and 7 hold; row 5 fails on one family row.

- 89 of 120 exports are byte-identical, and all 102 `diagnostics.json`, so on those files no refusal reason rises and no saved mesh is lost.
- On the 30 kept exports that differ, 2,818 bodies change (`ifc_body_diff.py`), every one a body the probe lists on a `cylpair` line (4,605 bodies on 41 files). The held-out house's export also differs and is not kept.
- No `psets_vs_ifc`, `qtos_vs_ifc`, `saved_meshes_vs_ifc` or `roof_bodies_vs_ifc` line moves. Of the 10 count rows in the diff, two move, both on RE1 Architecture's `IfcFlowTerminal`, whose class row stays level: `Toilet, Two Piece` leaves the 0.01 ft column (rvt-rs's farthest tested vertex from Revit's surface 0.0084 to 0.0130 ft, the scorer's sample, §5; Revit's vertices within 0.0040 ft of rvt-rs's surface on both), and `Shower Head, Hand Held Sprayer` enters it (0.0101 to 0.0091 ft).
- On the BIM4LCA housing, the railings' volume ratios grow (`Railing:KJO-2` 1.1985 to 1.2057, `KJO-1` 1.1989 to 1.2042, `KAI-7` 1.0886 to 1.0916, `KD-1` 1.0895 to 1.0901), the downspouts' farthest vertex comes in (0.0076 to between 0.0069 and 0.0073 ft), and on 7 stair bodies Revit's vertices lie at most 0.0081 ft from rvt-rs's surface, against 0.0079.
- The same-project gates pass on `ed2d342d`.

**Against Revit's export of Snowdon Towers Architectural** (run [44720380479](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44720380479)'s kept exports, read locally with `row5.py`). Row 6 fails. 62 bodies move. Revit's vertices beyond 0.001 ft of rvt-rs's surface go from 23,243 to 23,511, and beyond 0.01 ft from 8,806 to 8,809. 45 of the 62 are worse than on `main` on one count:

- 23 by Revit's vertices beyond 0.001 ft: 21 `Toilet-Domestic-3D`, from 308 to 320 each, and 1135932 (`Sink-3 Basin`) and 1135933, from 770 to 778 and from 524 to 532. The farthest Revit vertex is unchanged on each.
- 2 by Revit's vertices beyond 0.01 ft: two `Railing:Handrail - Pipe`, from 4 to 6 and from 8 to 9, the farthest from 0.01017 to 0.01033 ft.
- 20 by rvt-rs's farthest vertex, still within 0.001 ft: 19 `Crown Molding - Cabinets` bodies, from 0.00047 to 0.00072 ft, and a broiler, from 0.000002 to 0.00088 ft.

On the two bodies read, all 20 newly missed Revit vertices lie near a vertex `main` does not hold, at a median 0.0026 to 0.0030 ft; on the toilet, the head triangle nearest each of the four read has all three corners new and is 0.63 ft long and 0.0053 ft wide, where `main`'s is 0.016 ft wide.

**What the added points cost** (row 8). The triangle totals of the bodies the probe reads (243 on `rme_basic`) grow by more than 10 per cent on 13 files; whole-model triangles and the viewer's load were not measured: `rme_basic` 1.35 on each of five years, `rme_advanced` 1.29 on each of four, the two terrain housing files 1.30 (1,160 to 1,512), RE1 Plumbing 1.25 and Snowdon Towers HVAC 1.24. The exports grow by at most 6.6 MiB (`rme_basic` 2024, 106.0 to 112.7 MiB); none crosses the 120 MiB Measure validates, and Snowdon Towers Plumbing (129.9 to 134.2) and Architectural (292.2 to 293.7) were already over it on `main` (#2103).

## 3. Reproduce

```text
gh workflow run measure.yml --ref main -f ref=97a54e71c024e7ddbdfdcd359ba3cfdd8764205e -f base=2b57a4d9d23085775445edbe990f41e3bc9255e0 -f nordic=true -f samples=true -f probe=probe_2098_face_cracks -f scorers=none
gh workflow run measure.yml --ref main -f ref=ed2d342d09f66dc467ad330dcff32bdd69c29b65 -f base=d089085b5c953d55d260a920f561b11f30c88926 -f nordic=true -f r2025=true -f samples=true -f holdout=true -f keep_ifc=true
gh workflow run measure.yml --ref main -f ref=57e5c29a1e63040a2811fe54bc99dca98c6d19b2 -f base=none -f samples=true -f sample_years="2024" -f probe=probe_2104_pair_edges -f scorers=none
```

## 4. Not claimed

- **Why 45 bodies fit Revit's export worse** on Snowdon Towers Architectural, and the RE1 toilet: #2111, read in part in §5.
- **The pairs with a surface of revolution** (step 2 of #2104): cylinder beside a surface of revolution and two surfaces of revolution keep their counts.
- **How many refinements fail** and fall back to the stored samples.
- **Fewer triangles for the same closure:** the face's own strip points are not thinned where the shared edge adds points.

## 5. Read after landing (#2111, 2026-11-20)

Exports of `ed2d342d` and `d089085b` kept by [44779355089](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44779355089), read with every vertex; the tools are `research-tools/2111/` on `refs/archive/2026-11-20/measure-2111-reads` (`b060ca4b`). The reading is [#2111](https://github.com/DrunkOnJava/rvt-rs/issues/2111#issuecomment-7047304786), as corrected.

- **Row 5 was the scorer's sample.** `mesh_bodies_vs_ifc` tests at most 400 vertices of a body (`VERTEX_CAP`), an even stride through its sorted vertices, so added vertices move which are tested. RE1's `Toilet, Two Piece` has its farthest vertex from Revit's surface at 0.012989 ft on both sides, and the shower head at 0.010066 ft on both; only the tested sample moved (#2114).
- **Row 6's count moved with the triangulation.** On RE1's toilet, two Snowdon toilets and the sink, each Revit vertex newly beyond 0.001 ft lies off a new triangle's face or edge, by a median 0.00126 ft on the toilets. `main`'s triangles there are covered by the new mesh within 0.00081 ft at the 48 points tested on each of three toilets; the sink's 4 points 0.00159 ft off lie nearest an edge two triangles share. Which mesh lies nearer the true cylinder at those triangles is not read, so the count moved at a threshold equal to the 0.001 ft chord bound, and is shown neither as a fit loss nor as a finer mesh.
- **No cylinder face stands off its cylinder by more than before:** on the 1,492 graphs holding a cylinder pair ([44780039320](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44780039320)), the farthest a cylinder triangle stands off is 0.000997 ft on both sides and rises on none.
- **Which face, and which mesh is nearer the cylinder** ([#2111](https://github.com/DrunkOnJava/rvt-rs/issues/2111#issuecomment-7048394440), read after this section was written): every sliver read on the Snowdon Towers toilet and sink is on a `CylSurf` face, and stands off it by at most 0.000080 and 0.000098 ft, where the triangles `main` drew nearest the same Revit vertices stood off 0.000714 and 0.000950 ft. So at those triangles the new mesh lies nearer the cylinder, and the toilet's Revit vertices read stand at least 0.0010 ft off it. RE-496 halves the edge's refinement for fewer triangles.
- **Not read:** how far Revit's vertices lie from the true cylinder on the other bodies, why the welded mesh's open edges rise on those bodies (by 146 on each toilet), 19 of the 20 bodies counted by rvt-rs's farthest vertex, and the two railings counted beyond 0.01 ft.
