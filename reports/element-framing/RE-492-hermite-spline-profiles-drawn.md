# RE-492: A Hermite spline profile is read and drawn, and a plane meets such a ruled face along the edge's saved points

**Date:** 2026-11-19
**Issues:** #2094 (this change), #2082, #1191; follows RE-311 §3, which read spline profiles and left them off `main` because no body Revit's export holds gained a mesh from them.
**Artefacts:** the BIM4LCA office (VTT, CC BY-SA 4.0; Revit 2024): its timber version as built and at permit, and its two terrain files, each with Revit's own IFC4 export. Autodesk's sample projects of Revit 2023 to 2027, Snowdon Towers Architectural among them, are counted only, since they have no Revit export.
**Probes:** `probe_2094_edge_bow` (branch `research/2094-saved-points`, `bd6f2305`), and the scorer `tools/re/far_vertices_vs_ifc.py` (branch `measure/2094-far-vertices`, `e8e26552`). Both are diagnostics of this change and are not landed.
**Status:** positive, on four elements in two saves each, all on one building, the BIM4LCA office: the two stair flights of its timber version, and a ramp and its ramp flight on its terrain files. Every Revit vertex of the two flights is within 0.000109 ft of rvt-rs's surface, their volume within 0.0013 ft³ of Revit's, and their faces meet. Nothing else scored against a Revit export moves. The rows were registered in the pull request before each run was read.

## 1. What is read

A ruled surface or a surface of revolution whose profile is a `GHermiteSpline` (RE-311 §3: `m_NodeArray` of `m_iParametr`, `m_iPoint` and `m_iTangent`) is evaluated, and its grid is cut to the profile's curvature, a step at most 0.001 ft from its chord (`0130573d`). Before, such a face failed with `unsupported parametric profile curve GHermiteSpline`, and its body kept its record box.

## 2. Where a plane meets such a face

Where a plane shares an edge with one of these ruled faces, and the edge's samples cross both of the ruled face's parameters, the plane's side had been drawn through the points where the plane cuts the curved surface (`refined_plane_edge`, #1706). The curved face's side was drawn straight in its own parameters between the edge's saved points. The two parted by the edge's bow, and Revit's two vertices a flight that missed rvt-rs's surface, at 0.00103 and 0.00125 ft, lay in the plane on the straight line between two saved points (`far_vertices_vs_ifc`, Measure 44518424477). That is measured on the plane's side: Revit's vertices on the ruled face's side of the edge were not read apart.

Now both faces draw such an edge straight between its saved points (`115d224a`):
- The ruled face decides which edges these are (`FaceGrid::chord_edges`: its profile a spline, its neighbour's surface a `Plane`, the edge's samples changing both parameters), and the plane reads that set.
- The plane keeps the edge's saved points.
- The ruled face's grid vertices along each span between two saved points are put on the line between the span's ends. The trimmed mesher tags each point with the span it is made on (`trimmed_cells_on`), so the step reads each point once and needs no work budget. It replaced a search for the points on each span, which every run below measured; the two give the same exports, byte for byte, on all 120 models (Measure [44549221722](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44549221722), `35d04301` against `115d224a`: `ifc.sha256` 120 of 120 and `diagnostics.json` 102 of 102 identical).
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
| boundary vertices over 0.001 ft from another face | | 196 a flight (as built) | 0 | |

`stair_bodies_vs_ifc` has every Revit vertex within 0.001 ft of rvt-rs's surface on 31 of 31 flights on both files (29 of 31 on `main`). The two stairs that hold the flights (3558492, 3561901) were boxes and are now 113.422 and 113.273 ft³, against Revit's 113.341 and 113.342.

**The terrain files' ramps.** On `main` the ramp flight 1386050 and the ramp 1386524 (`Ramp:AP-ajoramppi`) were boxes. Now:
- The ramp flight has rvt-rs's vertices within 0.00088 ft of Revit's body (0.01305 ft with the reader alone) and Revit's within 0.00088 ft of rvt-rs's.
- The ramp is within 0.000005 ft both ways.
- `mesh_bodies_vs_ifc`'s ramp flights within 0.01 ft and 0.001 ft are 5 of 5 (4 of 4 on `main`, where this one was a box, and 4 of 5 with the reader alone), and `saved_meshes_vs_ifc` holds 6 of 6 ramps within 0.01 ft.
- The ramp flight's volume ratio of 0.5 is Revit's export: its body holds 1,028 triangles twice, and the volumes are equal once each face counts once.

**Nothing else scored moves.**
- No other `mesh_bodies_vs_ifc`, `saved_meshes_vs_ifc`, `roof_bodies_vs_ifc` or `stair_bodies_vs_ifc` count changes, and no `psets_vs_ifc` or `qtos_vs_ifc` line.
- Against the reader alone, 4 exports change and 8 bodies move, every one among the bodies the reader newly drew.
- No held-out export changes against the reader alone. Against `main`, one does: the held-out single-dwelling house, whose kept counts do not move (19 bodies drawn from saved graphics scored, 28 record boxes), so no body is newly drawn there that a scorer sees. Its export is not kept, so what moved is not read, and row 3's figures leave it out.
- The same-project gates pass. Against `main` they are exercised by one beam, newly drawn on each `rac_advanced` save of 2025 to 2027 and held to its 2024 save; on the `rac_basic` saves only a refusal's text changes.

**Counted, not scored.** Bodies drawn from saved graphics against `main`, from `diagnostics.json` on the 102 models that keep one: 25 more on 10 files.

| file | newly drawn | what |
|---|---:|---|
| BIM4LCA timber office, as built and at permit | 2 each | the flights |
| BIM4LCA terrain, concrete and timber | 2 each | the ramp and its ramp flight |
| `rac_advanced`, 2023 to 2027 | 1 each | a beam; on 2025 to 2027 held to its 2024 save (`bodies_across_saves`: beams compared 37 → 38) |
| Snowdon Towers Architectural | 12 | 4 record boxes and 8 turned family boxes |

Other bodies that held a Hermite profile stay boxes, now refused for the next part not read:
- the BIM4LCA office's washbasins (`IfcSanitaryTerminal`, `IDO Glow washbasin 11461_11561:PA-4`): 19 a file as built and at permit, and 17 a file in the timber version (`malformed ruled-surface edge is connected to face N`);
- the housing files' wall-hung toilets (`seinäwc-istuin:WC-2`), 6 a file (`unsupported parametric profile curve GNurbSpline`);
- RE1 Electrical's BAS panel and fire alarm panel (872430 and 925565, RE-311), one with a malformed ruled-surface edge and a `GNurbSpline`, one with a `GNurbSpline`;
- `rac_basic`'s one body a save of 2023 to 2027 (`a trim loop does not close in the surface's parameters`);
- 2 bodies on Snowdon Towers Architectural (`shared curved edge leaves adjacent plane`).

## 4. Not claimed

- **Any body the samples hold.** The 17 newly drawn bodies on `rac_advanced` and Snowdon Towers have no Revit export to score them.
- **A fallback that leaves no trace.** Where putting a ruled face's vertices on the saved points' lines would leave a triangle with no area, the face keeps its points on the surface and the plane still takes the saved points. The two then part as they did with the plane alone moved. No diagnostic says so: this is not a work stop, and the saved meshes are meshed on worker threads, whose budget notes are dropped (`budget`), so a note would have to travel back in each face's mesh. On the two flights the probe shows that none fell back. On the ramps the body moved, which a fallback alone would not have done, since the plane's side does not move there: so the placement ran on at least one of their faces, and whether any other fell back is not measured. Elsewhere it is not measured either.
- **Any other edge.** This change touches only edges between a plane and a spline ruled face whose samples cross both of its parameters. Faces that part along other edges are #2098's.
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

The moved bodies are read with `tools/re/ifc_body_diff.py` on the kept exports of the first run and of [44486295483](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44486295483). The second command needs the branch `measure/2094-far-vertices` for its scorer; that branch and the probe's are not landed, and their tips are kept as `refs/archive/2026-11-19/measure-2094-far-vertices` (`e8e26552`) and `refs/archive/2026-11-19/research-2094-saved-points` (`1e65747c`, which holds `bd6f2305`).
