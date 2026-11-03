# RE-302 — Ellipse profiles, curved ruled surfaces and trimmed parametric faces

**Date:** 2026-11-03
**Issues:** #1247, #255; follows RE-224, RE-300, RE-301 and RE-304
**Runs:** Measure [41585460305](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41585460305) (the readers at f4d82ed7 against `main` at e65c9232, `--saved-meshes`, `mesh_bodies_vs_ifc` and `schema`), the census [41585448251](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41585448251) at f4d82ed7, and the probe runs named below
**Result:** negative. The readers work: they draw every element the office kept as a box for these gaps, and the 43 they free there are not Revit's body (1.027 to 1.047 of its volume, 0.022 ft off), at 3.7 times the office's triangles. Nothing here lands; the readers stay on the branch at f4d82ed7, and this report records why and what a next attempt needs.

## 1. The gap

After RE-304, the census of what keeps a saved mesh as a box on the BIM4LCA office as built (`probe_re224_saved_mesh_status`, Measure [41572297689](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41572297689) on `main` at 89793841) leaves 124 elements, led by three gaps in `native_parametric_surface` and `native_parametric_mesh`, the readers of ruled surfaces (`RuledSurf`) and surfaces of revolution (`SurfRev`):

- `unsupported parametric profile curve GEllipse` (51 elements): a profile was read only as a `GLine` or a `GArc`;
- `nonrectangular parametric trim` (58): a face was meshed only where its trim loop runs round its UV box;
- `nonlinear ruled profiles need a curvature policy` (43): a ruled surface was meshed only between two lines.

## 2. The reading

`probe_re302_ellipse_profiles` prints every `GEllipse` and what points to it. On the office it finds 188, 38 of them a `RuledSurf`'s profile; on Autodesk's 2024 samples, 932 on Snowdon Towers architectural (592 ruled, 318 revolved), 372 on structural and 72 on plumbing (all ruled), 36 on electrical (all revolved). Every one has the same fields: `m_center`, `m_xVec` and `m_yVec` (unit, orthogonal), `m_xLen` and `m_yLen` (the two semi-axes) and `m_endParams` (the parameter range, radians). A point is `centre + xLen·cos t·x + yLen·sin t·y`, as a `GArc` is with one radius.

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
| plane takes the grid's cuts on the straight UV line, 23628b71 | 127 | a cut leaves the plane |
| cuts solved onto the plane, b913fe03 to dc14d724 | 127 | a cut line crosses another grid line; the plane's triangulation drops a point |
| cuts one axis at a time, 2f92cd71 (run 41582058884) | 127 | a cut line crosses another grid line |
| the edge traced onto the plane, 869e31f5 (run 41584152383) | 127 | a probe off the surface's envelope |
| off-envelope probes skipped, f4d82ed7 (run 41585448251) | 84 | none |

All 43 are the office's `WC-istuin:WC-1` toilets, the only element class on the office that these gaps alone kept as boxes. `probe_re302_plane_edges` (run [41578601400](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41578601400)) measured their non-isoparametric edges: the curved surface at each of Revit's samples is on the plane to 1e-13 ft, and between two samples it strays from the plane by up to 0.01 ft on a surface of revolution and 0.001 ft on a cylinder. That is why each step above was needed.

**Against Revit's own export** (run 41585460305, `--saved-meshes`): the office as built scores 1,650 → 1,693 saved-mesh bodies, and the 43 new ones are all the toilets. None is within 1% of Revit's volume or 0.01 ft of its surface: their volume is 1.027 to 1.047 of Revit's, Revit's vertices lie within 0.0048 to 0.0050 ft of rvt-rs's surface, and rvt-rs's lie up to 0.0218 ft from Revit's. The error is surface rvt-rs draws and Revit does not. Of each toilet's four shells, two are within 0.4% and 1.4% of Revit's volume (0.8376 against 0.8340 cubic ft, 0.4705 against 0.4639); the third is 20% over (0.2569 against 0.2134) and the fourth thirteen times over (0.0052 against 0.0004). The office at building permit gives the same 43, 1,738 → 1,781. Core Interior and RE1 are unchanged, byte for byte.

**The cost.** The office as built goes from 568,965 to 2,097,501 triangles (about 35,000 a toilet) and its IFC from 82.1 to 181.8 MB, the permit file from 573,733 to 2,102,269 and 82.1 to 181.7 MB. Measure's schema step then skips IfcOpenShell's validation, which it runs only under 120 MB, so the rule findings it reported on the office (4) are no longer measured.

**Autodesk's 2024 samples** (no Revit export, counted, not scored) move in every file, among them Snowdon Towers electrical's boxes 139 → 1 and structural's saved meshes 62 → 78. The structural sample's 16 new footings fit their record boxes exactly. Since the one class Revit's export does score is not Revit's body, none of these counts is claimed.

**Two mechanisms a next attempt has to address:**

- **The triangle count is structural.** `face_grid` adds every trim vertex's U and V to the grid's knots, so the plane beside a face meets its cuts, and `tessellate_trimmed` cuts each earcut triangle at every knot in its range, on both axes. Earcut's triangles on a loop of hundreds of points are long slivers across the face, so each becomes as many pieces as the grid cells it spans. Clipping the loop to each grid cell instead (Sutherland–Hodgman against the cell's four lines, then a fan), as RE-304 clips a cylinder's loop to each strip across U, gives one piece per cell the loop covers and keeps the knots the plane needs.
- **The extra surface is local, and not a second branch.** It sits in two of the four shells. A trace that took the wrong one of two places where the surface meets the plane would draw it, so `probe_re302_plane_edges` (run [41587751357](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41587751357), at 827718ff) counts, for every point the trace moves on the office's shared edges, the sign changes its 32 steps see: one, on all 19,992 on cylinders and 1,008 of the 1,148 on surfaces of revolution, and none on the other 140, which the trace rejects. No point saw two. What draws the extra surface is open.

## 4. Not claimed

- Any element drawn by these readers, on any file: the readers are not on `main`.
- That the readers are wrong where no Revit export scores them: the samples' new meshes may be right, but nothing measures them.
- That #1247's 341 beams are among what these readers would draw; no beam in the corpora measured here is.

## 5. Reproduce

```bash
# the readers against main, scored
gh workflow run measure.yml --ref main -f ref=f4d82ed7 -f base=e65c9232 -f ifc_flags=--saved-meshes \
  -f nordic=true -f samples=true -f sample_years="2024 2024" \
  -f models="core-interior re1-* nordic-office-* sample-2024-*" -f scorers="mesh_bodies_vs_ifc schema"
# the census of what keeps a saved mesh as a box
gh workflow run measure.yml --ref main -f ref=f4d82ed7 -f base=none -f nordic=true \
  -f probe=probe_re224_saved_mesh_status -f models=nordic-office-as-built -f scorers=none
# the shared edges' distances from their planes, and the trace's sign changes
gh workflow run measure.yml --ref main -f ref=827718ff -f base=none -f nordic=true \
  -f probe=probe_re302_plane_edges -f models=nordic-office-as-built -f scorers=none
```
