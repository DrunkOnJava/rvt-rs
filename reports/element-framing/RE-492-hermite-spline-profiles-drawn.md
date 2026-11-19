# RE-492: A Hermite spline profile is read and drawn, and a plane meets such a ruled face along the edge's saved points

**Date:** 2026-11-19
**Issues:** #2094 (this change), #2082, #1191; follows RE-311 §3, which read spline profiles and left them off `main` because no body Revit's export holds gained a mesh from them.
**Artefacts:** the BIM4LCA office (VTT, CC BY-SA 4.0; Revit 2024): its timber version as built and at permit, and its two terrain files, each with Revit's own IFC4 export. Autodesk's sample projects of Revit 2023 to 2027, Snowdon Towers Architectural among them, are counted only, since they have no Revit export.
**Probes:** `probe_2094_edge_bow` (branch `research/2094-saved-points`, `bd6f2305`), and the scorer `tools/re/far_vertices_vs_ifc.py` (branch `measure/2094-far-vertices`, `e8e26552`). Both are diagnostics of this change and are not landed.
**Status:** positive, on four elements in two saves each: the two stair flights of the timber office, and a ramp and its ramp flight on the terrain files. Every Revit vertex of the two flights is within 0.000109 ft of rvt-rs's surface, their volume within 0.0013 ft³ of Revit's, and their faces meet. Nothing else scored against a Revit export moves. The rows were registered in the pull request before each run was read.

## 1. What is read

A ruled surface or a surface of revolution whose profile is a `GHermiteSpline` (RE-311 §3: `m_NodeArray` of `m_iParametr`, `m_iPoint` and `m_iTangent`) is evaluated, and its grid is cut to the profile's curvature, a step at most 0.001 ft from its chord (`0130573d`). Before, such a face failed with `unsupported parametric profile curve GHermiteSpline`, and its body kept its record box.

## 2. Where a plane meets such a face

Where a plane shares an edge with one of these ruled faces, and the edge's samples cross both of the ruled face's parameters, the plane's side had been drawn through the points where the plane cuts the curved surface (`refined_plane_edge`, #1706). The curved face's side was drawn straight in its own parameters between the edge's saved points. The two parted by the edge's bow, and Revit's two vertices a flight that missed rvt-rs's surface, at 0.00121 to 0.00125 ft, lay in the plane on the straight line between two saved points (`far_vertices_vs_ifc`, Measure 44518424477).

Now both faces draw such an edge straight between its saved points (`115d224a`):
- The ruled face decides which edges these are (`FaceGrid::chord_edges`: its profile a spline, its neighbour's surface a `Plane`, the edge's samples changing both parameters), and the plane reads that set.
- The plane keeps the edge's saved points.
- The ruled face's grid vertices along each span between two saved points are put on the line between the span's ends.
- That is kept only where every triangle keeps its area. Otherwise the face's points stay on the surface, so the step refuses no face.

Two steps of evidence led there:
- With only the plane on the saved points (`b9e663b3`, Measure 44529253935), Revit's vertices on the flights came within 0.000109 ft. But 179 and 180 boundary vertices a flight were still more than 0.001 ft from another face, up to 0.00268 ft.
- With both faces on them (Measure 44534977433), no boundary vertex is more than 0.0001 ft from another face, and the largest gap is 0.000001 ft.

## 3. Measured

Measure [44536807887](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44536807887) (`115d224a` against `c22a87e4`: nordic, r2025, samples, holdout, exports kept), with [44486295483](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44486295483) (`0130573d`, the spline reader alone) and [44534977433](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44534977433) (the probe).

**The timber office's flights** (3558983 and 3561904, `Monolithic Run`, in both saves):

| | `main` | the reader alone | this change | Revit |
|---|---|---|---|---|
| body | record box, 3,376.5 ft³ | saved graphics | saved graphics | |
| Revit's vertices from rvt-rs's surface, at most | 4.4 ft | 0.00125 ft | 0.000109, 0.0000979 ft | |
| volume (ft³) | 3,376.5 | 111.837, 111.875 | 112.116, 112.117 | 112.115, 112.116 |
| boundary vertices over 0.001 ft from another face | | 196 a flight | 0 | |

`stair_bodies_vs_ifc` has every Revit vertex within 0.001 ft of rvt-rs's surface on 31 of 31 flights on both files (29 of 31 on `main`). The two stairs that hold the flights (3558492, 3561901) were boxes and are now 113.422 and 113.273 ft³, against Revit's 113.341 and 113.342.

**The terrain files' ramps.** On `main` the ramp flight 1386050 and the ramp 1386524 (`Ramp:AP-ajoramppi`) were boxes. Now:
- The ramp flight has rvt-rs's vertices within 0.00088 ft of Revit's body (0.01305 ft with the reader alone) and Revit's within 0.00088 ft of rvt-rs's.
- The ramp is within 0.000005 ft both ways.
- `mesh_bodies_vs_ifc`'s ramp flights within 0.01 ft and 0.001 ft go from 4 to 5 of 5, and `saved_meshes_vs_ifc` holds 6 of 6 ramps within 0.01 ft.
- The ramp flight's volume ratio of 0.5 is Revit's export: its body holds 1,028 triangles twice, and the volumes are equal once each face counts once.

**Nothing else scored moves.**
- No other `mesh_bodies_vs_ifc`, `saved_meshes_vs_ifc`, `roof_bodies_vs_ifc` or `stair_bodies_vs_ifc` count changes, and no `psets_vs_ifc` or `qtos_vs_ifc` line.
- Against the reader alone, 4 exports change and 8 bodies move, every one among the bodies the reader newly drew.
- No held-out export changes, and the same-project gates pass.

**Counted, not scored.** Bodies drawn from saved graphics against `main`, from each file's `diagnostics.json`: 25 more on 10 files.

| file | newly drawn | what |
|---|---:|---|
| BIM4LCA timber office, as built and at permit | 2 each | the flights |
| BIM4LCA terrain, concrete and timber | 2 each | the ramp and its ramp flight |
| `rac_advanced`, 2023 to 2027 | 1 each | a beam; on 2025 to 2027 held to its 2024 save (`bodies_across_saves`: beams compared 37 → 38) |
| Snowdon Towers Architectural | 12 | 4 record boxes and 8 turned family boxes |

Other bodies that held a Hermite profile stay boxes, now refused for the next part not read:
- the BIM4LCA office's toilets: 19 a file as built and at permit, and 17 a file in the timber version (`malformed ruled-surface edge is connected to face N`);
- the housing files' 6 a file (`unsupported parametric profile curve GNurbSpline`);
- RE1 Electrical's two panels (one with a malformed ruled-surface edge and a `GNurbSpline`, one with a `GNurbSpline`);
- `rac_basic`'s one body a save of 2023 to 2027 (`a trim loop does not close in the surface's parameters`);
- 2 bodies on Snowdon Towers Architectural (`shared curved edge leaves adjacent plane`).

## 4. Not claimed

- **Any body the samples hold.** The 17 newly drawn bodies on `rac_advanced` and Snowdon Towers have no Revit export to score them.
- **A fallback that leaves no trace.** Where putting a ruled face's vertices on the saved points' lines would leave a triangle with no area, the face keeps its points on the surface and the plane still takes the saved points. The two then part as they did with the plane alone moved. No diagnostic says so. On the two flights the probe shows that none fell back; elsewhere it is not measured.
- **Faces between two curved surfaces.** On Snowdon Towers Architectural and the BIM4LCA office and housing files, faces still part where two curved faces meet, a cylinder or a surface of revolution on each side. On #2098's experiment, which puts every plane-to-parametric crossing edge on the saved points, these are the only gaps left over 0.001 ft, up to 0.0024 ft (`probe_2098_face_cracks`, Measure 44542871619). This change touches only edges between a plane and a spline ruled face.
- **Why two scorers disagreed on the flights' volume** before this change (`stair_bodies_vs_ifc` 0.2% under Revit's, `mesh_bodies_vs_ifc` within 0.01%). They agree now, and the cause was not measured.
- **`GNurbSpline` profiles**, which stay unread.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=115d224a90b2106c7c28fcd6dca08c5799820dd7 \
  -f base=c22a87e447ba0a3009d7789a75a34b1430370544 -f nordic=true -f r2025=true -f samples=true \
  -f holdout=true -f keep_ifc=true
gh workflow run measure.yml --ref measure/2094-far-vertices -f ref=bd6f2305daceaf7d1fba41b0ed03ff3cf6145b8a \
  -f base=none -f nordic=true -f models="nordic-office-timber-as-built nordic-office-timber-permit" \
  -f scorers="mesh_bodies_vs_ifc stair_bodies_vs_ifc far_vertices_vs_ifc" -f probe=probe_2094_edge_bow
```

The moved bodies are read with `tools/re/ifc_body_diff.py` on the kept exports of the first run and of [44486295483](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44486295483).
