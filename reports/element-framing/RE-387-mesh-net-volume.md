# RE-387: the volume of a body drawn from a mesh of flat faces is Revit's NetVolume

**Date:** 2026-11-09
**Issues:** #1046 (base quantities Revit computes from its joined geometry, W7), #1016 (quantities); follows RE-243 (a mullion's `NetVolume` from its extruded section), RE-268 (its `NetSurfaceArea` from the ends its joins keep) and #1191 (every element that would be a box is drawn from the mesh Revit saved for it)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them) and puzzbobb's 24 Revit 2025 models (Revit 25.0.2's IFC4 exports)
**Runs:** Measure [42669397638](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42669397638) (`tools/re/mesh_quantities_vs_ifc.py` on `main` at `cbeba4c7`, the office as built and at permit and the 24 Revit 2025 models) and [42680955225](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42680955225) (unfiltered: this change's code `a82047ab` against `main` at `5247e7ba`, section 3)
**Status:** Positive for the volume of a body whose faces are all flat and which closes: it is Revit's `NetVolume` for every member, panel and column drawn from a mesh in a model whose Revit export writes one, and rvt-rs now writes it where it wrote none (the office's equal quantities go from 29,761 to 31,904 as built and from 30,037 to 32,196 at permit; different stays 1 and the quantities written that Revit's export does not stay 0, section 3). Negative for the mesh's surface area (Revit leaves out the end faces its joins hide) and for a column's `GrossVolume` (Revit omits it on columns that nothing tells apart): neither is written. The volume of a body with a curved face is its tessellation's, not Revit's, and such a body is refused.

## 1. What Revit writes, and what the saved mesh is

The office's export writes `Qto_MemberBaseQuantities.NetVolume` on all 10,003 curtain mullions, `Qto_PlateBaseQuantities.NetVolume` on all 260 curtain panels (269 at permit), `Qto_ColumnBaseQuantities.NetVolume` and `GrossVolume` on its columns and `Qto_WallBaseQuantities.NetVolume` on every wall. revit-ifc's `NetVolumeCalculator` takes the volume of the solid it writes (CLEANROOM.md source 5, RE-243), and the mesh Revit saved for an element is that solid, tessellated. rvt-rs draws a mullion, a panel or a column whose body would be its record's box from that mesh by default (#1191), so it holds the body the quantity is the volume of, and wrote the volume of none of the 2,208 mullions it has no extrusion of nor of the panels.

## 2. The mesh's volume against Revit's

`mesh_quantities_vs_ifc.py` takes rvt-rs's `IfcTriangulatedFaceSet` bodies, works their volume and surface area out with IfcOpenShell and counts, for each object both files hold, how many of Revit's values it equals (a relative 1e-5, or 1e-6 where that is larger, as `qtos_vs_ifc.py` scores). The office as built draws 11,084 bodies from a mesh, 11,201 at permit:

| Entity and quantity | Revit writes, on a mesh-drawn object | The mesh's volume equals it | rvt-rs wrote it (`main` at `cbeba4c7`) | Not written, the mesh's volume equals Revit's on |
|---|---:|---:|---:|---:|
| mullion `NetVolume`, as built | 9,678 | 9,678 | 7,795 | 1,883 of 1,883 |
| mullion `NetVolume`, at permit | 9,706 | 9,706 | 7,823 | 1,883 of 1,883 |
| panel `NetVolume`, as built (at permit) | 260 (269) | 260 (269) | 0 | 260 of 260 (269 of 269) |
| column `NetVolume`, as built (at permit) | 199 (289) | 199 (289) | 199 (282) | 0 (7 of 7) |
| column `GrossVolume`, as built (at permit) | 199 (271) | 199 (271) | 199 (265) | 0 (6 of 6) |
| wall `NetVolume` (the 6 and 12 drawn from a mesh) | 6 (12) | 6 (12) | 0 | 6 of 6 (12 of 12) |
| ramp flight `NetVolume` | 1 | 1 | 0 | 1 of 1 |

The mesh's volume is not Revit's where the body has a curved face, whose saved mesh is a tessellation: 4 of the office's 12 `IfcBuildingElementProxy` `NetVolume` differ by 1.7e-5 to 4.5e-5 of the value, and 42 of its 44 spaces drawn from a mesh by 4e-4. A volume taken of a body whose faces are all flat is a polyhedron's, which is exact.

The 24 Revit 2025 models draw 6 members from a mesh (their only mesh-drawn quantities), and the mesh's volume and surface area equal Revit's `NetVolume`, `GrossVolume`, `NetSurfaceArea` and `GrossSurfaceArea` on all 6, none written before this change.

**The surface area is not the mesh's.** Revit's `NetSurfaceArea` of a mullion equals the mesh's on 4,087 of the office's 9,678 as built (4,099 of 9,706 at permit) and on none of the others, which are 0.0075 less in the first examples measured (0.8475 against 0.8550): Revit's quantity leaves out the end face its joins hide, which RE-268 reads from the boxes, where the saved mesh keeps it. rvt-rs's value (RE-268) is Revit's on all 6,606 it writes, so the mesh gives no surface area.

## 3. What rvt-rs writes

`saved_meshes::attach_scoped_with_control` takes the volume of each body it draws from a mesh (`mesh_volume::closed_mesh_volume`) where every face of the mesh is flat (`all_faces_flat`: one normal on all the vertices of each face the mesh was cut from, which a saved planar face has and a curved one does not) and the triangles, their vertices welded, close a surface wound one way, and gives it to the element as `NetVolume` in the base quantity set of its entity: `Qto_MemberBaseQuantities` for an `IfcMember`, `Qto_PlateBaseQuantities` for an `IfcPlate`, `Qto_ColumnBaseQuantities` for an `IfcColumn`; only where the element holds none, so RE-243's values and RE-291's stay. `--no-base-quantities` leaves them out with the rest.

### The change against `main`

Measure [42680955225](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42680955225), unfiltered (`nordic`, `r2025`, `samples`, `racadv`, `bimcollab` and `holdout`), this change's code `a82047ab` against `main` at `5247e7ba`, 118 exports. 37 move: the office as built and at permit, the three Revit 2025 `Rooms_Stairs` models, RE1 Architecture, five held-out models, Einhoven and 25 Autodesk samples; the other 81 are byte-identical. Of every scorer's output only the quantity census (`qtos_vs_ifc`, 6 models), the held-out counts (5) and the MCP answers that quote the same quantities (7) change, and of the diagnostics only `exported.total_entities` does: `decoded.parameter_value_count`, the property census, every body, box, mesh, opening and material scorer are byte-identical, so no body and no property moves. `same-project gates` passes on `a82047ab` (it compares bodies and Areas across saves, which a quantity does not move).

| Export | Revit's quantities on objects rvt-rs writes | Equal, before to after | Different | Written that Revit's export does not |
|---|---:|---:|---:|---:|
| office as built | 39,754 | 29,761 to 31,904 | 1 to 1 | 0 to 0 |
| office at permit | 40,006 | 30,037 to 32,196 | 1 to 1 | 0 to 0 |
| Revit 2025 `Rooms_Stairs`, `_endwithriserOFF` and `_NoRisers` (each) | 106 | 43 to 45 | 0 to 0 | 0 to 0 |
| held-out Revit 2024 pair | 313 | 87 to 89 | 1 to 1 | 0 to 0 |
| held-out Revit 2025 pair | 313 | 89 to 91 | 1 to 1 | 0 to 0 |
| held-out Revit 2026 pair | 313 | 89 to 91 | 1 to 1 | 0 to 0 |
| held-out Revit 2027 pair | 357 | 74 to 76 | 1 to 1 | 0 to 0 |
| held-out `modelo_bim` (Revit 2023, a later Revit's export) | 211 | 8 to 16 | 0 to 0 | 0 to 0 |

What the values are:

- **Mullions, `Qto_MemberBaseQuantities.NetVolume`:** 7,795 to 9,678 of 10,003 equal as built and 7,823 to 9,706 of 10,031 at permit. The 1,883 written are every mesh-drawn mullion that had none, each equal; the 325 still without one are drawn as extrusions, not from a mesh (the bodies of the office as built on `main` that lack a volume: 1,883 `IfcTriangulatedFaceSet` and 325 `IfcExtrudedAreaSolid`, read from its kept export; the census has 325 missing at permit too).
- **Panels, `Qto_PlateBaseQuantities.NetVolume`:** 0 to 260 of 260 as built, 0 to 269 of 269 at permit.
- **Columns, `Qto_ColumnBaseQuantities.NetVolume`:** unchanged as built (the 80 without one are extrusions), 282 to 289 of 345 equal at permit: the 7 mesh-drawn columns that had none. A column's `GrossVolume` is not written (section 4).
- **The three `Rooms_Stairs` models:** the 2 members of each, `NetVolume` 0 to 2 of 2 equal.
- **Held-out models (counts only):** the 2 members of each of the four pairs, 0 to 2 of 2 equal, and the 8 columns of `modelo_bim`, 0 to 8 of 8.
- **RE1 Architecture:** Revit's export writes no base quantities there (its setup's "Export base quantities" off, RE-242), so the census counts what rvt-rs writes beside them apart, 96 to 98 (its 2 panels' `NetVolume`), and `rvt-ifc --no-base-quantities` leaves them out.
- **Einhoven and the 25 Autodesk samples** have no Revit export to score; they gain volumes and nothing else (`exported.total_entities` is recounted: 90,983 to 91,261 on the office as built).

The test that refuses a mesh is exercised on nothing the census can score: every mesh-drawn member, panel and column of the office passes it (2,143 given a volume as built, 2,159 at permit, none refused), so what a curved or open mesh would get rests on the proxies and spaces of section 2, whose classes are not given a volume.

## 4. Not shown

- **A wall's `NetVolume`.** The 6 (12) walls drawn from a mesh because they embed a curtain wall (RE-340) have a mesh volume equal to Revit's, 6 of 6 (12 of 12), and are not given it here. Every other wall is drawn as an extrusion.
- **A body with a curved face.** Its mesh is a tessellation (section 2) and nothing in the file says how close, so the test refuses it and it gets no volume. No member, panel or column of the measured models is refused: the office's 325 mullions and 80 columns without a `NetVolume` are extrusions, and every mesh-drawn one holds one, so the refusal is shown on no Revit export.
- **Other entities.** Only an `IfcMember`, an `IfcPlate` and an `IfcColumn` are given a volume. The walls and the ramp flight of section 2 have a mesh volume equal to Revit's and are not given it here (the walls', with their side areas, are RE-388); a box drawn from a mesh of an entity Revit writes no base quantity set on (furniture, fixtures) gets none.
- **`NetSurfaceArea` of the 3,397 mullions without one**, and every quantity that is not a volume (section 2), stay as RE-268 left them.
- **A column's `GrossVolume`.** The mesh's volume equals it where Revit writes it, but at permit Revit's export omits it (with `Length` and `OuterSurfaceArea`) on the 18 columns it draws as a cut mesh, which are 18 of the 24 columns rvt-rs has no `GrossVolume` for: 289 columns have a mesh, 271 carry the quantity, 265 have it written. Nothing in the file tells the 6 it writes from the 18 it omits, so a column gets its `NetVolume` only (Revit writes that on all 289).
- **Revit 2025's `GrossVolume`, `GrossSurfaceArea` and `NetSurfaceArea` of a member** (the 6 above) are not written.

## 5. Reproduce

```bash
# The scorer (on main from this change on; the scorers come from --ref), on the code of main before it:
gh workflow run measure.yml --ref main -f ref=cbeba4c73e619962d9ad93c7a50949ec6049083e \
    -f base=none -f nordic=true -f r2025=true -f models="nordic-office-as-built nordic-office-permit r2025-*" \
    -f scorers=mesh_quantities_vs_ifc
# The change against main, unfiltered (every scorer, the same-project gates), at the code and base of the run above:
gh workflow run measure.yml --ref main -f ref=a82047aba5850c4ced3c6240c8aac4af307cdbed -f base=5247e7ba107add935c3cc3ed60bb9ee95dbeeb8e \
    -f nordic=true -f r2025=true -f samples=true -f racadv=true -f bimcollab=true -f holdout=true
```
