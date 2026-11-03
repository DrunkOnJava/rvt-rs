# RE-311 — Why saved pipe fittings measure 1.2 times Revit's volume, cone faces, and spline profiles

**Date:** 2026-11-03
**Issues:** #1144, #96 (pipe fittings); follows RE-78, RE-224, RE-265, RE-302, RE-304 and RE-305
**Artefacts:** RE1 Plumbing, Mechanical, Architecture and Electrical (Revit 2025, MIT, `magnetar-io/revit-test-datasets` and `Drshelden/IFC-ECS`) with Revit's own IFC exports, as Measure fetches them
**Runs:** Measure [41585460305](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41585460305) (the figures this report starts from, `main` at e65c9232), [41640991781](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41640991781) (`probes/examples/probe_re311_fitting_faces.rs` at 5acfcaed), [41642258924](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41642258924) (cone faces, 8c1637a3), [41643460069](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41643460069) (cone faces and spline profiles, 64b2cff0), [41644455294](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41644455294) and [41644460456](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41644460456) (both at 64b2cff0, unfiltered), and [41651021342](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41651021342) and [41651025810](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41651025810) (both at 8c1637a3, unfiltered: this change)
**Result:** three parts. (1) Negative for a defect in the mesher: the fittings drawn from saved meshes are the right bodies, meshed finer than Revit's export, whose volume is 16 to 26% short of the true surface on the elbows; the "wrong" volumes of the closet bends were the scorer's. (2) Positive: a `ConeSurf` face is meshed, which draws 8 more pipe transitions on RE1 Plumbing. (3) Negative, left off `main`: spline profile curves are read, and no body Revit's export holds gains a mesh from it; the two panels #1144 names keep their box for trims RE-302 and RE-305 measured as not readable.

## 1. Pipe fittings: Revit's export is the coarse one

RE-304 left the volume of RE1 Plumbing's fittings as "RE-78's open question": every vertex within 0.01 ft of Revit's body, a volume 1.2 to 1.5 times Revit's, nothing missing. Measure 41585460305 had RE1 Plumbing's 46 saved fittings at a median 1.216 and RE1 Mechanical's three `Closet Bend Reducing` at 0.28, 0.35 and 2.59 with a negative signed volume for two of them.

**Elbows.** Revit's export of a `PIPE-Elbow - Welded` is an `IfcFacetedBrep` of 14 to 22 faces: a polygon of 6, 8 or 10 sides round the pipe (by its radius) and two segments along the 90° bend, three rings in all. Its vertices are on the true surface (the ring is at the pipe's radius, the centre of the middle ring at 45° of the bend radius), so the polyhedron is inscribed in it. The torus section it approximates has `π r² · R · π/2`:

| Type (Tag) | sides | r (ft) | R (ft) | Revit's volume | true torus | true / Revit | rvt-rs / Revit | rvt-rs / true |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 444370 | 6 | 0.0335 | 0.0439 | 0.000181 | 0.000243 | 1.3431 | 1.3170 | 0.981 |
| 443813 | 8 | 0.0417 | 0.0521 | 0.000362 | 0.000447 | 1.2337 | 1.2160 | 0.986 |
| 447179 | 8 | 0.0499 | 0.0620 | 0.000618 | 0.000762 | 1.2337 | 1.2166 | 0.986 |
| 445111 | 10 | 0.0827 | 0.1290 | 0.003667 | 0.004354 | 1.1873 | 1.1758 | 0.990 |
| 445080 | 10 | 0.0991 | 0.1624 | 0.006632 | 0.007874 | 1.1873 | 1.1705 | 0.986 |

(volumes in cubic feet, from the vertices of Revit's `IfcFacetedBrep` in `RE1-Plumbing.ifc`; the ratio of the torus to the polyhedron depends only on the number of sides.) rvt-rs's mesh is 98.1 to 99.0% of the true torus (its chord bound is 0.001 ft), so the 1.17 to 1.32 is Revit's polyhedron being 16 to 26% short of the surface, and nothing in rvt-rs's body. The faces agree: for each elbow the probe's `SurfRev` and plane faces are wound the way the nearest triangle of Revit's body is (all 336, 320 and 216 triangles of the curved faces; a plane's, 87% or more, the rest nearest to a neighbouring face's triangle at the rim), the mesh is one closed shell with every edge shared once in each direction (445080: 1,074 consistent edges, none open or doubled), and Revit's vertices all lie within 0.001 ft of it (30 of 30 elbows). Revit's farthest-vertex distance is the polyhedron's sag: the outer equator of a 45° segment is `(R + r)(1 − cos 22.5°)` = 0.0199 ft from a chord for 445080, against 0.0232 measured.

**Transitions and tees.** `PIPE-Transition - Welded` has the same cause (below). A `PIPE-Tee - Welded` is a 6-sided run and branch whose saddle Revit triangulates between polygons: 46 of Revit's 50 vertices lie within 0.001 ft of rvt-rs's surface, and four, at the saddle, 0.006 to 0.007 ft off.

**Closet bends and the 1.44 to 1.47 of RE-304.** The volumes in Measure 41585460305 were taken about the origin of the file's frame, as the scorer always had. A saved mesh meshes each curved face on its own, so the faces meet along cracks (the closet bend 442037: 1,698 edges shared once each way, 86 open, 284 that do not pair, none wound the same way twice), and the signed volume of a mesh with cracks changes with its distance from the origin by the crack's area vector times that distance. `volume` in `tools/re/mesh_bodies_vs_ifc.py` is now taken about the body's own centre. Of the same bodies:

| Body | before | after |
|---|---:|---:|
| `Closet Bend Reducing`, RE1 Mechanical (3, one symbol, three poses) | 0.2799, 0.3495, 2.5937 | 1.1431, 1.1431, 1.1431 |
| `Closet Bend Reducing`, RE1 Plumbing (4) | 1.1144, 0.6755, 1.4127, 0.4582 | 1.1144, 1.1725, 1.1725, 1.1197 |
| `PIPE-Tee - Welded`, RE1 Plumbing (10) | 1.4423 to 1.4718 | 1.1415 to 1.1433 |

The three Mechanical bends are one saved mesh (1,532 triangles, 0.002066 cubic ft, the same for all three) under three poses; Revit's is 0.001807, a 10-sided collar and bend of 172 triangles. The bend's torus is `π r² R π/2` = 0.001229 plus the two hubs and necks 0.000862, 0.00209 cubic ft: rvt-rs's mesh is 98.9% of the true body and Revit's 86.5%. The closet bends' farthest vertices (0.02 ft) are the sag of Revit's coarse bend and, for six of the 88 Revit vertices of 442037, a point of Revit's triangulation 0.006 to 0.020 ft from the surface where the bend meets the hub: left as measured.

RE-304 (§3) gave the tees as 1.44 to 1.47 of Revit's volume; that was the origin artefact, and the figure in `docs/status.md` for 0.12.2 stands as the release recorded it.

**What is not changed.** No fitting that was drawn is failed closed or re-meshed. A mesh at Revit's faceting would need Revit's export setting ("level of detail for some geometry") and say less about the body than the surface does; #1144 measured the boxes of 19 RE1 Plumbing fittings at 0.028 to 0.099 ft from Revit's body, and the saved meshes of the 54 here are 0.0003 to 0.035 ft from it.

`mesh_bodies_vs_ifc.py` has a new column, "ref on ours": the bodies whose every Revit vertex lies within 0.001 ft of rvt-rs's surface. It separates a mesh of the same surface from a mesh of another one: RE1 Plumbing 39 of 54 fittings, with the elbows 30 of 30 and the transitions 8 of 8, none of the tees, closet bends or the tee vent (Revit's triangulated junctions).

## 2. `ConeSurf`

The 24 `ConeSurf` faces of RE1 Plumbing (in 12 graphs: the 8 pipe transitions Revit's export holds and 4 elements it does not, 53311, 444375, 447176 and 447863) and 4 of RE1 Architecture's (a shower head) failed with `unsupported surface ConeSurf`. The probe prints the fields: `m_center` (the apex), `m_halfAngle` (0.0471, 0.0315 and 0.0303 rad here), a unit orthogonal basis `m_xVec`, `m_yVec`, `m_zVec`, `m_Envelope` (U a half turn, V from 0.8055 to 0.9303) and `m_orientFlag`. The edge each cone face shares with a plane carries the cone's `(u, v)` and the plane's `(x, y)` of the same point: at `v` = 0.93032 the radius is 0.0437992, which is `v · sin α` (0.0470797) and not `v · tan α` (0.0471319). So `P(u, v) = apex + v (cos α · z + sin α (cos u · x + sin u · y))`, `v` the distance from the apex along the generator, and the normal `∂P/∂u × ∂P/∂v ∝ cos α · e_r − sin α · z`, outward for an increasing radius. Read as a surface that is straight in V, the grid needs only the angular chord bound; a plane that shares an edge with a cone is refined on the cone's grid as one that shares it with a revolution is.

Measure 41642258924, `--saved-meshes`, this change against `main` (`mesh_bodies_vs_ifc`):

| file | scored, before → after | `PIPE-Transition - Welded` | within 0.01 ft | Revit's vertices on rvt-rs's surface (0.001 ft) |
|---|---|---:|---:|---:|
| RE1 Plumbing | 52 → 60 | 8 new | 8 of 8 | 8 of 8 |
| RE1 Architecture, Electrical, Mechanical | unchanged | | | |

Per transition (the probe's `fitting_faces_vs_ifc` scorer): 60 to 76 triangles against Revit's 120 or 24, vertices 0.0032 to 0.0045 ft from Revit's polyhedron, every Revit vertex (62 of 62, 14 of 14) within 0.001 ft of the mesh, every cone and plane triangle wound as the nearest Revit triangle, volume 1.08 to 1.10 of Revit's (a 6- or 8-sided band: the same faceting as §1). The box was 0.028 to 0.099 ft from Revit's body. IFC hashes of every other RE1 model are identical to `main`'s.

The shower head still keeps its box: its cone faces decode, and a `SurfRev` of the same primitive fails with `nonrectangular parametric trim`, which RE-302 and RE-305 measured.

## 3. Spline profiles: measured, not landed

`GNurbSpline` (a rational B-spline: `m_Degree`, `m_Knots`, `m_ctrlPoints` of `m_Point` and `m_w`, `m_endParams`) and `GHermiteSpline` (`m_NodeArray` of `m_iParametr`, `m_iPoint` and `m_iTangent`, the tangent per unit of parameter, as a node pair whose tangents run along the chord shows: 5.4267 · 0.0033 is the chord 0.0178) profile curves were evaluated at 64b2cff0 (archived as `refs/archive/2026-11-03/re311-spline-profiles`, which adds this report's docs on top), a profile's chord error held to 0.001 ft by cutting the curve into equal parameter steps from its second differences (at most 128), and a ruled surface's shared grid taking the largest. A line and an arc were cut as before, a ruled surface between arcs still failed, and a ruled grid between lines was computed by the same arithmetic.

Measure 41643460069, RE1 Electrical's faces by outcome, before → after (the counts are the probe's, over every graph of the file): `RuledSurf` ok 0 → 608 of 702 (the 702 failed for a spline profile; 94 now fail for other reasons), `SurfRev` ok 8 → 268 of 284, `Plane` ok 2,853 → 3,143 (the planes sharing an edge with a spline-profile face, each checked against the plane's stored coordinates at the edge's samples by `refined_plane_edge`). No element gains a mesh: the BAS panel (926984 and 872430, 857 faces decoded) keeps 83 faces that fail and the fire alarm panel (930176 and 925565, 799 decoded) 44, for `nonrectangular parametric trim` (18 each), `multiple parametric trim loops unsupported` (12 each), `disconnected parametric trim in model coordinates` (14 each) and, on the BAS panel alone, `malformed ruled-surface edge is connected to face N` (39): the trims RE-302 and RE-305 measured as not readable. What decoded is 48,418 triangles for the BAS panel (#1144 measured 221,630 for its grid against 29,610 in Revit's export) and 4,954 for the fire alarm panel.

Across the unfiltered Measure 41644455294 (the office, Core Interior, Einhoven, RE1 and the 24 Revit 2025 models, all scorers) no body scored against Revit's export changed. The only bodies that gained a mesh beyond the cones are on Autodesk's 2024 samples, which have no Revit export, so none of them is scored: Snowdon Towers architectural 2,337 → 2,340 and rac advanced 4,971 → 4,972 (4 bodies, the difference between Measure 41644455294 at 64b2cff0 and 41651021342 at 8c1637a3). A reader that draws nothing Revit's export can be set against is a reader nothing measures, and RE-302 and RE-305 left off `main` readers of this kind; this one is left off too, to be taken with the trims that keep the panels as boxes.

## 4. Elsewhere

Measure [41651021342](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41651021342), this change (cone faces, 8c1637a3) against `main` with `--saved-meshes`, every scorer, `-f nordic=true -f r2025=true -f samples=true -f sample_years="2024 2024"`. Bodies drawn from saved meshes, before → after:

| file | before → after |
|---|---|
| Core Interior, Einhoven, the BIM4LCA office as built (11,079) and at permit (11,196), the 24 Revit 2025 models | unchanged |
| RE1 Architecture (53), Electrical (47), Mechanical (68) | unchanged |
| RE1 Plumbing | 52 → 60 |
| Autodesk 2024 samples, **counted, not scored** (no Revit export): rme advanced | 5,633 → 5,754 |
| rme basic | 5,669 → 5,833 |
| Snowdon Towers plumbing | 2,781 → 2,957 |
| Snowdon Towers HVAC | 2,064 → 2,073 |
| rac basic | 309 → 310 |
| rac advanced, rst basic, Snowdon Towers architectural, electrical and structural | unchanged |

No body scored against Revit's export moves but RE1 Plumbing's: `mesh_bodies_vs_ifc` there 52 → 60 and `box_bodies_vs_ifc` 9 → 1 record-box bodies (the 8 transitions were boxes whose volume is 0.597 of Revit's body). On the samples the swept solids that carry their own Position fall as the bodies are drawn from saved meshes (rme basic 1,170 → 1,006, Snowdon plumbing 3,472 → 3,296, rme advanced 1,162 → 1,041, HVAC 532 → 523, rac basic 175 → 174, RE1 Plumbing 72 → 64). The samples' gains are 471 bodies no export can score; the cone reading is verified on the 8 RE1 transitions, and these stay unverified until a file with Revit's export covers them, as RE-304's did.

Default exports ([41651025810](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41651025810), the same models without `--saved-meshes`): `ifc.sha256` of base and head are identical on all 42 models (the office at both versions, Core Interior, Einhoven, RE1, the 24 Revit 2025 models, the ten 2024 samples) and the scorer diff is empty. No door or window moves.

## 5. Not claimed

- That a mesh at Revit's faceting would be exact: no rule for Revit's sides-by-radius (6, 8, 10 sides at 0.034, 0.042 to 0.050 and 0.083 to 0.099 ft) or its two segments along a bend was fitted, and the export setting that produces it is not in the file.
- Anything about the six of 88 Revit vertices on the Mechanical closet bend that lie 0.006 to 0.020 ft from the mesh, or the four at the tee's saddle.
- Spline profiles on any body: none is scored, and the code is not on `main`.
- Cone faces other than the 8 transitions, and the shower head.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=8c1637a3 -f base=main \
  -f ifc_flags=--saved-meshes -f keep_ifc=true -f models="re1-*" -f scorers="mesh_bodies_vs_ifc" \
  -f probe=probe_re311_fitting_faces -f probe_scorer=fitting_faces_vs_ifc
gh workflow run measure.yml --ref main -f ref=8c1637a3 -f base=main -f ifc_flags=--saved-meshes \
  -f nordic=true -f r2025=true -f samples=true -f sample_years="2024 2024"
```

(the second without `ifc_flags` for the default exports; `--ref` is the ref whose `tools/re` scorers run, which carries the scorer changes once this lands). The probe at 5acfcaed prints each fitting's primitives, faces and surfaces. The elbow table is Revit's side parsed from `RE1-Plumbing.ifc`: the `IfcFacetedBrep` of each `IfcPipeFittingType`'s representation map, the ring radius from its two end caps and the bend radius from their centres. The spline readers are at 64b2cff0.
