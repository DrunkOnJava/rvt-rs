# RE-388: a wall's saved mesh gives its NetVolume, its side areas and, where its sides agree, its gross side area

**Date:** 2026-11-09
**Issues:** #1046 (base quantities Revit computes from its joined geometry, W7), #1016 (quantities); follows RE-270 (the four wall quantities of a plain prism), RE-293 (walls joined to roofs and floors clear of them), RE-340 (walls drawn from a mesh because they embed a curtain wall) and RE-387 (the volume of a body drawn from a mesh of flat faces)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them) and puzzbobb's 24 Revit 2025 models (Revit 25.0.2's IFC4 exports)
**Runs:** Measure [42677569083](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42677569083) (`probes/examples/probe_1046_wall_mesh_quantities.rs` and `tools/re/wall_mesh_quantities_vs_ifc.py`, commit `89b210a6d0fd`, the office as built and at permit and the Revit 2025 models) and [42681675150](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42681675150) (the same with the shape of each wall's top and bottom, commit `c6285bc2bf64`, the office); the change against its base, section 3
**Status:** @@STATUS@@

## 1. What Revit writes, and what the wall's saved mesh is

Revit's export writes `Qto_WallBaseQuantities` on a wall: `Width`, `Length`, `Height`, `GrossFootprintArea`, `GrossSideArea`, `NetSideArea`, `GrossVolume` and `NetVolume`. rvt-rs writes `Width` from the wall's type, `Length` and `Height` by their own rules (RE-252, RE-266), and the four areas and volumes only on a wall that is a plain prism cut back at its joins with no step of its record that cuts it (RE-270, RE-293): 97 of the office's walls (49 as built, 48 at permit). About 2,500 wall quantities of the office as built stay missing, every one of walls whose solid Revit joins and cuts (#1046).

revit-ifc (CLEANROOM.md source 5, read and cited in RE-270, no code copied) takes `NetVolume` from the volume of the wall's solid, `GrossSideArea` and `NetSideArea` from the largest vertical face of that solid (the area inside its outer loop, and its area with the holes taken out) and `GrossVolume` as the wall type's width times the `GrossSideArea`. The mesh Revit saved for a wall is that solid, joined to the walls beside it and cut by what it hosts, tessellated, so it holds what the quantities are taken from: the body of a wall rvt-rs draws by extrusion is not the joined solid, but the saved mesh beside it is.

## 2. The mesh against Revit's quantities

`probe_1046_wall_mesh_quantities` meshes the saved graphics of every wall (`SWall`, `ArcWall`) at detail 3 and prints its volume, its heights, its vertical flat faces (area, outer-loop area, normal) and whether the mesh closes (every directed edge has its reverse once, corners welded to 1e-6 ft) and in how many pieces; `wall_mesh_quantities_vs_ifc.py` counts, for each wall both Revit's export and the probe hold, how many of Revit's values the mesh's equals (a relative 1e-5 or 1e-6 m, m2 or m3 where that is larger, as `qtos_vs_ifc.py` scores), by gate.

**Revit 2024, the office.** 1,058 of 1,084 walls mesh as built (1,135 of 1,139 at permit); Revit writes quantities on 581 of them as built (586 at permit), the rest being walls its export splits at storeys or writes none for:

| | As built | At permit |
|---|---:|---:|
| `NetVolume`, any mesh: equal; within 1e-4 but not 1e-5; further | 570; 11; 0 | 574; 12; 0 |
| `NetSideArea`, closed in one piece: equal; within 1e-4; further | 496; 3; 0 | 506; 3; 0 |
| `NetSideArea`, any mesh: equal; within 1e-4 or 1e-2; further | 542; 6; 7 | 556; 3; 3 |
| `GrossSideArea`, closed in one piece: equal; within 1e-4; further | 496; 3; 0 | 505; 3; 1 |
| `GrossSideArea`, closed in one piece, the two sides equal: equal; within 1e-4; further | 454; 3; 0 | 469; 3; 0 |
| `GrossVolume` (width times the gross), closed in one piece: equal; further | 499; 0 | 508; 1 |

The volume is Revit's on every wall, a mesh that does not close included (the open meshes are faces split by a vertex the other side does not share, whose surface is closed all the same); the 11 and 12 not within 1e-5 are walls of 6 faces under 4.2 ft3, whose thickness is a tenth of a foot and whose corners sit at an elevation of 190 ft, where the saved mesh's single-precision corners move a volume by that much. The side areas are Revit's only where the mesh closes in one piece: a face split across several primitives gives a fragment (323 ft2 for Revit's 1,513 on two walls with an open mesh), and a wall whose mesh is two pieces gives a smaller area (61.35 ft2 against Revit's 64.99 on one). **Which walls Revit gives the areas.** Revit's export writes the full five (`GrossSideArea`, `GrossVolume`, `Height`, `NetSideArea`, `NetVolume`) on 555 of the office's 581 walls with quantities as built (562 of 586 at permit) and only `NetVolume` on the other 26 (24). The 26 are all `Basic Wall`s, rvt-rs draws all of them as extrusions (20 plain, 6 cut by a boolean) like 544 of the 555, and their meshes are mostly closed in one piece too (19 of 26, against 499 of 555): the shape probe's count of each wall's levels facing up and down and of its sloped and curved faces does not separate them either (146 walls with the shape of 11 of them are given the full set). What does is the number of vertical flat faces: the 26 have five or more (5 to 13), and of the walls closed in one piece, 0 of the 19 volume-only walls as built (0 of 12 at permit) have four or fewer, where 302 of the 499 walls with the full set (318 of 509) do, a wall's two sides and two ends, which is also what 118 of the 138 Revit 2025 walls have. A wall with five or more vertical faces is given no area, whichever it is.

The two sides of 457 of the 499 walls closed in one piece as built have the same outer-loop area, and Revit's gross area is that area; on 42 they differ and Revit's is the larger in all of them (the larger in 36 of 37 at permit): the one exception, a stepped wall whose two sides each hold two faces at different depths, is Revit's smaller, and nothing in its mesh tells it from the 36.

**Revit 2025, the 24 models.** 23 hold walls, 138 in all, every one with a mesh that closes in one piece: the mesh gives `NetVolume`, `NetSideArea`, `GrossSideArea`, `GrossVolume` and `Height` equal to Revit's on all 138.

## 3. What rvt-rs writes

@@WRITES@@

@@MEASURE@@

## 4. Not shown

- **A wall with a curved face** (an `ArcWall`, a curved end): its mesh is a tessellation and its volume is not Revit's to 1e-5, so it gets none.
- **A mesh that does not close, or is two pieces.** The volume is right on the open meshes measured, and the side areas are not; none of them is given, so 47 and 45 volumes as built and at permit that the mesh gives equal are left out for the sake of the areas' gate.
- **The areas of a wall with five or more vertical faces.** 197 of the 499 walls closed in one piece as built that Revit gives the full set (191 of 509 at permit), whose mesh equals Revit's areas, are left out because 19 (12) like them have only a volume in Revit's export: the number of vertical faces is the one thing the mesh holds that separates them, in this project, and why Revit's export writes only a volume of those is not known.
- **A gross side area where the two sides differ.** 42 walls as built and 37 at permit: Revit's is the larger side's on all but one, and nothing tells the one.
- **`Height`.** The mesh's span of heights is within 1e-4 of Revit's on all 555 walls and within 1e-5 on 518, the rest by the single-precision noise above, so it is not written.
- **`GrossFootprintArea`, `Length`.** The mesh gives neither (Revit's `Length` is a curve's, RE-252).
- **Other projects.** Only the office (Revit 2024) and the Revit 2025 models have walls with Revit's quantities and a measured mesh.

## 5. Reproduce

```bash
# The probe and its scorer, from the scratch commit that holds them (the probe and its scorer come from -f ref):
gh workflow run measure.yml --ref main -f ref=89b210a6d0fd -f base=none -f nordic=true -f r2025=true \
    -f models="nordic-office-as-built nordic-office-permit r2025-*" -f scorers=none \
    -f probe=probe_1046_wall_mesh_quantities -f probe_scorer=wall_mesh_quantities_vs_ifc
@@REPRODUCE@@
```
