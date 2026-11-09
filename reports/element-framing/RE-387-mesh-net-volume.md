# RE-387: the volume of a body drawn from a mesh of flat faces is Revit's NetVolume

**Date:** 2026-11-09
**Issues:** #1046 (base quantities Revit computes from its joined geometry, W7), #1016 (quantities); follows RE-243 (a mullion's `NetVolume` from its extruded section), RE-268 (its `NetSurfaceArea` from the ends its joins keep) and #1191 (every element that would be a box is drawn from the mesh Revit saved for it)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them) and puzzbobb's 24 Revit 2025 models (Revit 25.0.2's IFC4 exports)
**Runs:** Measure [42669397638](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42669397638) (`tools/re/mesh_quantities_vs_ifc.py` on `main` at `cbeba4c7`, the office as built and at permit and the 24 Revit 2025 models); the change against `main`, section 3
**Status:** @@STATUS@@

## 1. What Revit writes, and what the saved mesh is

The office's export writes `Qto_MemberBaseQuantities.NetVolume` on all 10,003 curtain mullions, `Qto_PlateBaseQuantities.NetVolume` on all 260 curtain panels (269 at permit), `Qto_ColumnBaseQuantities.NetVolume` and `GrossVolume` on its columns and `Qto_WallBaseQuantities.NetVolume` on every wall. revit-ifc's `NetVolumeCalculator` takes the volume of the solid it writes (CLEANROOM.md source 5, RE-243), and the mesh Revit saved for an element is that solid, tessellated. rvt-rs draws a mullion, a panel or a column whose body would be its record's box from that mesh by default (#1191), so it holds the body the quantity is the volume of, and wrote the volume of none of the 2,208 mullions it has no extrusion of nor of the panels.

## 2. The mesh's volume against Revit's

`mesh_quantities_vs_ifc.py` takes rvt-rs's `IfcTriangulatedFaceSet` bodies, works their volume and surface area out with IfcOpenShell and counts, for each object both files hold, how many of Revit's values it equals (a relative 1e-5, or 1e-6 where that is larger, as `qtos_vs_ifc.py` scores). The office as built draws 11,084 bodies from a mesh, 11,201 at permit:

| Entity and quantity | Revit writes, on a mesh-drawn object | The mesh's volume equals it | rvt-rs writes it now | None written, the mesh's volume equals Revit's on |
|---|---:|---:|---:|---:|
| mullion `NetVolume`, as built | 9,678 | 9,678 | 7,795 | 1,883 of 1,883 |
| mullion `NetVolume`, at permit | 9,706 | 9,706 | 7,823 | 1,883 of 1,883 |
| panel `NetVolume`, as built (at permit) | 260 (269) | 260 (269) | 0 | 260 of 260 (269 of 269) |
| column `NetVolume`, as built (at permit) | 199 (289) | 199 (289) | 199 (282) | 0 (7 of 7) |
| column `GrossVolume`, as built (at permit) | 199 (271) | 199 (271) | 199 (265) | 0 (6 of 6) |
| wall `NetVolume` (the 6 and 12 drawn from a mesh) | 6 (12) | 6 (12) | 0 | 6 of 6 (12 of 12) |
| ramp flight `NetVolume` | 1 | 1 | 0 | 1 of 1 |

The mesh's volume is not Revit's where the body has a curved face, whose saved mesh is a tessellation: 4 of the office's 12 `IfcBuildingElementProxy` `NetVolume` differ by 1.7e-5 to 4.5e-5 of the value, and 42 of its 44 spaces drawn from a mesh by 4e-4. A volume taken of a body whose faces are all flat is a polyhedron's, which is exact.

The 24 Revit 2025 models draw 6 members from a mesh (their only mesh-drawn quantities), and the mesh's volume and surface area equal Revit's `NetVolume`, `GrossVolume`, `NetSurfaceArea` and `GrossSurfaceArea` on all 6, none written now.

**The surface area is not the mesh's.** Revit's `NetSurfaceArea` of a mullion equals the mesh's on 4,087 of the office's 9,678 as built (4,099 of 9,706 at permit) and on none of the others, which are 0.0075 less in the first examples measured (0.8475 against 0.8550): Revit's quantity leaves out the end face its joins hide, which RE-268 reads from the boxes, where the saved mesh keeps it. rvt-rs's value (RE-268) is Revit's on all 6,606 it writes, so the mesh gives no surface area.

## 3. What rvt-rs writes

`saved_meshes::attach_scoped_with_control` takes the volume of each body it draws from a mesh (`mesh_volume::closed_mesh_volume`) where every face of the mesh is flat (`all_faces_flat`: one normal on all the vertices of each face the mesh was cut from, which a saved planar face has and a curved one does not) and the triangles, their vertices welded, close a surface wound one way, and gives it to the element as `NetVolume` in the base quantity set of its entity: `Qto_MemberBaseQuantities` for an `IfcMember`, `Qto_PlateBaseQuantities` for an `IfcPlate`, `Qto_ColumnBaseQuantities` for an `IfcColumn`; only where the element holds none, so RE-243's values and RE-291's stay. `--no-base-quantities` leaves them out with the rest.

@@MEASURE@@

## 4. Not shown

- **A wall's `NetVolume`.** The 6 (12) walls drawn from a mesh because they embed a curtain wall (RE-340) have a mesh volume equal to Revit's, 6 of 6 (12 of 12); they are raised by another pass and are not given it here. Every other wall is drawn as an extrusion.
- **A body with a curved face.** Its mesh is a tessellation (section 2), and nothing in the file says how close; it gets no volume, and a mullion on a curved curtain wall whose body is such a mesh stays without one.
- **Other entities.** A box drawn from a mesh of an entity the quantity census has no value for (furniture, fixtures) gets none: Revit writes no base quantity set on them. Only the entities of section 2 are given a volume.
- **`NetSurfaceArea` of the 3,397 mullions without one**, and every quantity that is not a volume (section 2), stay as RE-268 left them.
- **A column's `GrossVolume`.** The mesh's volume equals it where Revit writes it, but at permit Revit's export omits it (with `Length` and `OuterSurfaceArea`) on the 18 columns it draws as a cut mesh, which are 18 of the 24 columns rvt-rs has no `GrossVolume` for: 289 columns have a mesh, 271 carry the quantity, 265 have it written. Nothing in the file tells the 6 it writes from the 18 it omits, so a column gets its `NetVolume` only (Revit writes that on all 289).
- **Revit 2025's `GrossVolume`, `GrossSurfaceArea` and `NetSurfaceArea` of a member** (the 6 above) are not written.

## 5. Reproduce

```bash
# The scorer, from this change's branch (the scorers come from --ref), on the code of main:
gh workflow run measure.yml --ref <branch holding tools/re/mesh_quantities_vs_ifc.py> -f ref=cbeba4c73e619962d9ad93c7a50949ec6049083e \
    -f base=none -f nordic=true -f r2025=true -f models="nordic-office-as-built nordic-office-permit r2025-*" \
    -f scorers=mesh_quantities_vs_ifc
@@REPRODUCE@@
```
