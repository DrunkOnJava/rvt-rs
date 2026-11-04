# RE-302 — Ellipse profiles, curved ruled surfaces and trimmed parametric faces

**Date:** 2026-11-03
**Issues:** #1247, #255; follows RE-224, RE-300, RE-301 and RE-304
**Runs:** Measure [41585460305](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41585460305) (the readers at f4d82ed7 against `main` at 30756e97, `--saved-meshes`, `mesh_bodies_vs_ifc` and `schema`), the census [41585448251](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41585448251) at f4d82ed7, and the probe runs named below; for the second attempt (§6) Measure [41765801884](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41765801884), [41765805196](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41765805196) and [41766414367](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41766414367) on the office, and the face probes [41766357243](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41766357243) and [41767705968](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41767705968)
**Result:** negative, in two attempts. The readers work: they draw every element the office kept as a box for these gaps, and the 43 they free there are not Revit's body by the scorer's bars. The first attempt (f4d82ed7) drew them at 1.027 to 1.047 of Revit's volume and 3.7 times the office's triangles. The second (§6) clips each trimmed loop to the grid cells it covers and shares the pieces' corners: the office is 2.1 times `main`'s triangles instead of 3.7, IfcOpenShell validates it again, and every toilet is 1.006 of Revit's volume with 0.5% more area, so the extra surface the first attempt saw was not there. What is left is Revit's: 0.022 ft is its coarse faceting of the bowl's surfaces of revolution, and 66 of its 808 vertices lie 0.003 to 0.005 ft off our surface at junctions between two of our faces. The toilets still fail the 0.01 ft bar (0 of 43) and the "ref on ours" bar (0 of 43), so nothing lands but this report and its probes; the readers stay on `re/1247-per-cell-clip` (reader code at b5d5546f, branch tip d5d26fb4).

## 1. The gap

After RE-304, the census of what keeps a saved mesh as a box on the BIM4LCA office as built (`probe_re224_saved_mesh_status`, Measure [41572297689](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41572297689) on `main` at 89793841) leaves 124 elements, led by three gaps in `native_parametric_surface` and `native_parametric_mesh`, the readers of ruled surfaces (`RuledSurf`) and surfaces of revolution (`SurfRev`):

- `unsupported parametric profile curve GEllipse` (51 elements): a profile was read only as a `GLine` or a `GArc`;
- `nonrectangular parametric trim` (58): a face was meshed only where its trim loop runs round its UV box;
- `nonlinear ruled profiles need a curvature policy` (43): a ruled surface was meshed only between two lines.

## 2. The reading

`probe_re302_ellipse_profiles` (Measure [41559823434](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41559823434), at 6efd70b6) prints every `GEllipse` and what points to it. On the office it finds 188, 38 of them a `RuledSurf`'s profile; on Autodesk's 2024 samples, 932 on Snowdon Towers architectural (592 ruled, 318 revolved), 372 on structural and 72 on plumbing (all ruled), 36 on electrical (all revolved). Every one has the same fields: `m_center`, `m_xVec` and `m_yVec` (unit, orthogonal), `m_xLen` and `m_yLen` (the two semi-axes) and `m_endParams` (the parameter range, radians). A point is `centre + xLen·cos t·x + yLen·sin t·y`, as a `GArc` is with one radius.

- **Ellipse profiles.** `Curve::Ellipse` evaluates that, with its derivative. Its second derivative is never longer than `max(xLen, yLen)`, so a surface of revolution steps an ellipse's parameter as it would an arc of that radius, which keeps each chord within the 0.001 ft bound.
- **Curved ruled surfaces.** A ruled surface joins `C1(u)` to `C2(u)` with straight lines. Along U it takes the step its more curved profile needs; across V it is straight, but the rulings twist where the profiles' tangents differ, so V is divided until the twist, sampled at 65 points as `|C1'·span1 − C2'·span2|`, stays within the chord bound across one U step.
- **Trimmed faces.** A face whose loop is not its UV box has the loop earcut in UV, and each triangle, being convex, cut exactly at the grid's U and V values, so every piece lies in one cell of the grid the rectangular path would draw. The pieces must cover the loop's own area, or the loop is taken as self-intersecting and the face rejected.
- **The edge a trimmed face shares with a plane.** A plane face beside a curved face takes the curved face's points along their edge, so the two meet without a gap; that read only an edge along a U or a V line, and rejected any other, which kept these faces' elements as boxes. On a trimmed face an edge may cross the grid diagonally. `probe_re302_plane_edges` measures such edges: the curved surface at each of Revit's own samples is on the plane's point to 1e-13 ft on every surface class, but Revit's samples are sparse, and the surface along the straight UV line between two of them strays from the plane by up to 0.01 ft. So at f4d82ed7 the edge is traced onto the plane: each line between two samples is divided in eight, and each point moved across the line onto the plane, taking the sign change nearest the line among 32 steps within the line's length on either side and bisecting it. The trace is then cut where it crosses a grid line the curved face is cut at (U and V on a ruled surface or a surface of revolution, U on a cylinder, whose strips are cut across U only, RE-304), each crossing solved the same way along its grid line. The curved face is cut through those points and the plane takes the same ones. A point with no sign change near it rejects the face, as does a line between two points that still crosses a grid line, or a plane loop that crosses itself after refinement.

## 3. Measured

**What the readers free.** The census of the office as built at each step, elements kept as boxes for a reason other than a nested detail filter or a view component:

| commit | kept | what keeps the 43 WCs |
|---|---:|---|
| `main` 89793841 (run 41572297689) | 124 | ellipse profiles, non-rectangular trims, curved ruled profiles |
| ellipse profiles and trims, 464007e9 (run 41575783153) | 124 | a shared curved edge that is not isoparametric |
| plane takes the grid's cuts on the straight UV line, 23628b71 (run 41577045231) | 127 | a cut leaves the plane |
| cuts solved onto the plane, b913fe03 to dc14d724 (runs 41579849356, 41581370410) | 127 | a cut line crosses another grid line; the plane's triangulation drops a point |
| cuts one axis at a time, 2f92cd71 (run 41582058884) | 127 | a cut line crosses another grid line |
| the edge traced onto the plane, 869e31f5 (run 41584152383) | 127 | a probe off the surface's envelope |
| off-envelope probes skipped, f4d82ed7 (run 41585448251) | 84 | none |

All 43 are the office's `WC-istuin:WC-1` toilets, the only element class on the office that these gaps alone kept as boxes. From 23628b71 on, three elements more are counted (1414271, 2607843, 3076853): on `main` their saved graphics give no surface at all, so the census, which counts only elements with some surface, does not list them; with the readers they give a few faces and still meet another gap (`unsupported faceted topology`, `HermiteSurf`). They were boxes throughout, so the readers free 43, and 124 → 84 is that less those three. `probe_re302_plane_edges` (run [41578601400](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41578601400)) measured their non-isoparametric edges: the curved surface at each of Revit's samples is on the plane to 1e-13 ft, and between two samples it strays from the plane by up to 0.01 ft, on a surface of revolution and on a cylinder alike. That is why each step above was needed.

**Against Revit's own export** (run 41585460305, `--saved-meshes`): the office as built scores 1,650 → 1,693 saved-mesh bodies, and the 43 new ones are all the toilets. None is within 1% of Revit's volume or 0.01 ft of its surface: the scorer's lists of the farthest give volumes of 1.027 to 1.047 of Revit's, Revit's vertices lie within 0.0048 to 0.0050 ft of rvt-rs's surface, and rvt-rs's lie up to 0.0218 ft from Revit's. The first attempt read this as surface rvt-rs draws and Revit does not (§6 shows it was not). Of the four shells of each toilet the scorer lists by shell, two are within 0.4% and 1.4% of Revit's volume (0.8376 against 0.8340 cubic ft, 0.4705 against 0.4639); the third is 20% over (0.2569 against 0.2134) and the fourth thirteen times over (0.0052 against 0.0004). The office at building permit gives the same 43, 1,738 → 1,781. Core Interior and RE1 are unchanged, byte for byte.

**The cost.** The office as built goes from 568,965 to 2,097,501 triangles (about 35,000 a toilet) and its IFC from 82.1 to 181.8 MB, the permit file from 573,733 to 2,102,269 and 82.1 to 181.7 MB. Measure's schema step then skips IfcOpenShell's validation, which it runs only under 120 MB, so the rule findings it reported on the office (4) are no longer measured.

**Autodesk's 2024 samples** (no Revit export, counted, not scored) move in 8 of the 10 files (rme basic and advanced are unchanged), among them Snowdon Towers electrical's boxes 139 → 1 and structural's saved meshes 62 → 78. Since the one class Revit's export does score is not Revit's body, none of these counts is claimed.

**Two mechanisms the first attempt left for a next one** (both resolved in §6):

- **The triangle count is structural.** `face_grid` adds every trim vertex's U and V to the grid's knots, so the plane beside a face meets its cuts, and `tessellate_trimmed` cuts each earcut triangle at every knot in its range, on both axes. Earcut's triangles on a loop of hundreds of points are long slivers across the face, so each becomes as many pieces as the grid cells it spans. Clipping the loop to each grid cell instead (Sutherland–Hodgman against the cell's four lines, then a fan), as RE-304 clips a cylinder's loop to each strip across U, gives one piece per cell the loop covers and keeps the knots the plane needs.
- **The extra surface is local, and the trace shows no second branch.** It sits in two of the four shells. A trace that took the wrong one of two places where the surface meets the plane would draw it, so `probe_re302_plane_edges` (run [41587751357](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41587751357), at 827718ff) counts, for every point the trace moves on the office's shared edges, the sign changes its 32 steps see: one, on all 19,992 on cylinders and 1,008 of the 1,148 on surfaces of revolution, and none on the other 140 (a point with none rejects its face; no census message ties them to an element measured here). No point saw two at that resolution. What draws the extra surface is open.

## 4. Not claimed

- Any element drawn by these readers, on any file: the readers are not on `main`.
- That the readers are wrong where no Revit export scores them: the samples' new meshes may be right, but nothing measures them.
- That #1247's 341 beams are among what these readers would draw; no beam in the corpora measured here is.
- That the 66 vertices of §6 are Revit's error and not ours: the probe places them at junctions and cannot say which side of the junction is off the true edge.
- The second attempt on any file but the BIM4LCA office pair: Core Interior, RE1, the Revit 2025 models and the 2024 samples were not measured with it, since its readers do not land.
- That the radius of §6 is the surface's own: it is `edge² / (8 · distance)` from Revit's triangles, not read from the surface.

## 5. Reproduce

```bash
# the readers against main, scored
gh workflow run measure.yml --ref main -f ref=f4d82ed7 -f base=30756e97 -f ifc_flags=--saved-meshes \
  -f nordic=true -f samples=true -f sample_years="2024 2024" \
  -f models="core-interior re1-* nordic-office-* sample-2024-*" -f scorers="mesh_bodies_vs_ifc schema"
# the census of what keeps a saved mesh as a box
gh workflow run measure.yml --ref main -f ref=f4d82ed7 -f base=none -f nordic=true \
  -f probe=probe_re224_saved_mesh_status -f models=nordic-office-as-built -f scorers=none
# what a GEllipse holds and what points to it, on the office and the 2024 samples
gh workflow run measure.yml --ref main -f ref=6efd70b6 -f base=none -f nordic=true -f samples=true \
  -f sample_years="2024 2024" -f probe=probe_re302_ellipse_profiles -f scorers=none
# the shared edges' distances from their planes, and the trace's sign changes
gh workflow run measure.yml --ref main -f ref=827718ff -f base=none -f nordic=true \
  -f probe=probe_re302_plane_edges -f models=nordic-office-as-built -f scorers=none
# the second attempt (§6): one run per row of its table (301c209e, 406e4b59, b5d5546f)
gh workflow run measure.yml --ref main -f ref=b5d5546f -f base=8b8f1fee -f ifc_flags=--saved-meshes \
  -f nordic=true -f models="nordic-office-*" -f scorers="mesh_bodies_vs_ifc schema"
# each toilet face against Revit's mesh of the toilet; on main the toilets are boxes, so run it at the
# readers' commit (the probe and scorer are on main and at d5d26fb4)
gh workflow run measure.yml --ref main -f ref=d5d26fb4 -f base=none -f nordic=true \
  -f models=nordic-office-as-built -f scorers=none -f probe=probe_re302_toilet_faces \
  -f probe_scorer=toilet_faces_vs_ifc
```

## 6. The second attempt: clipping to grid cells, and what is left

Branch `re/1247-per-cell-clip` (reader code at b5d5546f, tip d5d26fb4, over `main` at 8b8f1fee). `tessellate_trimmed` clips the loop to each column of grid cells and each column to its cells (Sutherland–Hodgman against the four lines), fans a convex piece and earcuts a concave one, keeps the pieces' corners as one vertex per (u, v) so neighbouring cells share them, and refuses a loop whose pieces do not cover its UV area, as before. Its work is bounded (200 million clip steps) for a hostile loop.

**The cost, office as built / at permit** (`--saved-meshes`; the 4 rule findings of IfcOpenShell's validation are those `main` has, and it runs on every row but the second, which is over its 120 MB limit):

| readers | commit (run) | triangles | IFC |
|---|---|---:|---:|
| `main` | 8b8f1fee (41765801884, base) | 568,885 / 573,323 | 82.2 / 82.2 MB |
| slivers cut at every knot | f4d82ed7 (41585460305) | 2,097,501 / 2,102,269 | 181.8 / 181.7 MB |
| per-cell clip, every trim vertex a knot | 301c209e (41765801884) | 1,354,151 / 1,358,589 | 110.4 / 110.4 MB |
| per-cell clip, the grid from the chord bound alone | 406e4b59 (41765805196) | 1,187,075 / 1,191,513 | 104.4 / 104.4 MB |
| the same, with the knots of isoparametric edges kept | b5d5546f (41766414367) | 1,191,138 / 1,195,576 | 104.6 / 104.6 MB |

301c209e is archived at `refs/archive/2026-11-03/re302-per-cell-clip-all-knots`; the other two are on the branch. The last row is the branch's: a plane beside an isoparametric edge of a trimmed face takes that face's grid values along the edge (`refined_plane_edge`), so the face's own samples stay in the grid or the two meet in a T-junction; every other edge of a trimmed loop is traced from the same `face_grid` on both sides. All three variants free the same 43 toilets (84 elements keep their box, as at f4d82ed7), at permit too.

**The toilets** (the scorer, same runs): 43 of 43 are within 1% of Revit's volume, 1.0059 to 1.0060 on the 15 it lists. The first attempt's were 1.027 to 1.047, and not equal: 1.0334 to 1.0465 across instances of one type among the 15 it listed, the third shell 0.2565 to 0.2649 against Revit's 0.2134 on every one. The figure no longer varies by instance. Two changes arrived together, the loop clipped to cells and the corners shared, and were not measured apart; a mesh whose pieces do not share corners has cracks between them, and the volume of a mesh with cracks changes with the crack's area vector times its distance from the mesh's centre (RE-311), which shared corners close. Revit's vertices are still 0.0046 to 0.0050 ft from our surface and ours 0.0218 to 0.0222 ft from Revit's: 0 of 43 within 0.01 ft, 0 of 43 "ref on ours".

**Per face** (`probe_re302_toilet_faces` and `toilet_faces_vs_ifc.py`, run 41767705968 at d5d26fb4, three toilets, alike to the digit): ours is 14,471 triangles in 70 faces and 23.0667 sq ft, Revit's 1,668 triangles, 808 vertices and 22.9621 sq ft. The surface rvt-rs draws is 0.5% more, not 4%, and the two bodies have the same extent (2.306 by 1.216 by 2.723 ft).

- **0.022 ft is Revit's faceting.** 1,127 of our 14,471 triangles have their centre more than 0.01 ft from Revit's surface (0.85 sq ft); all are on four surfaces of revolution (faces 57, 58, 59 and 61), none on a plane, a cylinder or a ruled surface. On the two largest (59 and 61: 1.66 and 1.93 sq ft, the bowl) the nearest Revit triangle's longest edge is 0.52 to 0.54 ft, and `edge² / (8 · distance)` is 1.6 to 1.75 ft at the worst triangles (median 1.9 and 2.0 ft over the 490 and 558 beyond 0.01 ft): the sag of a chord that long on a surface of that curvature. Revit's own vertices there lie on our surface: 27 of the 29 nearest face 59 and all 34 nearest face 61 are within 0.001 ft. That fits a mesh truer than the polyhedron Revit inscribes in the bowl, as for RE-311's elbows.
- **Revit's 66 vertices more than 0.001 ft off ours** (of 808: 742 are within 0.001, 7 within 0.002, 58 within 0.005, one within 0.01). Nearest face: 5 (a cylinder) 7, 57 14, 58 11, 59 2, and 66 to 69 (two small revolutions and two small cylinders at one height) 7, 7, 9 and 9. Of the 66, 59 are 0.0027 to 0.0050 ft off, each within 0.0007 ft of the same distance from the next nearest of our faces (face 5's from face 3, a plane; 57's from 61 and 58's from 59, both revolutions; 66's from 68 and 67's from 69) and 0.0027 to 0.0051 ft from the nearest boundary edge of its face, at most 0.0006 ft farther than the face: they lie where two of our faces meet. The other 7 are 0.0010 to 0.0012 ft off and 0.006 to 0.27 ft from an edge.

Nothing in the toilet is drawn that Revit's export lacks, and the difference in volume is gone. What remains cannot be closed by the readers if the 0.01 ft bar is Revit's polyhedron: a mesh within 0.01 ft of the bowl's chords is not within 0.001 ft of its surface. The 66 vertices are open. A junction vertex 0.003 to 0.005 ft from both faces is Revit's triangulation off the true edge, as RE-311 found at tees and closet bends (0.006 to 0.020 ft), or our boundary off the true edge: between two curved faces our edge is the straight UV line between Revit's saved samples, which strays from the true edge by up to 0.01 ft where `probe_re302_plane_edges` measured it against a plane, and only edges beside a plane are traced. Face 5's 7 sit at a cylinder-plane junction whose edge the readers do trace onto the plane, and they are 0.0046 to 0.0050 ft from it, which points to Revit's side there; the revolution-to-revolution junctions (57 with 61, 58 with 59) are untraced and this run cannot tell the two apart.

**What a next attempt needs.** (1) To settle the 66, compare them with the true edge: solve `A(uA) = B(uB)` along the junction of two curved faces (or compare with Revit's saved edge samples, which are on it), and trace curved-to-curved edges as plane edges are. (2) The 0.01 ft bar is unreachable for this bowl by any mesh of its true surface; whether the toilets are kept on "ref on ours" as RE1's elbows were (#1280) is for the maintainers to decide, not a measurement. (3) Measure the second attempt on Core Interior, RE1, the Revit 2025 models and the 2024 samples before any landing; this report does not.
