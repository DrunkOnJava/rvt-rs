# RE-256 — A wall's, floor's, ceiling's and roof's type parameters

**Date:** 2026-10-27
**Issues:** #35 (parameters); follows RE-207 (an element's saved parameters), RE-28 and RE-253 (the type a wall's records name), RE-240 (the type a pipe, wall sweep or ramp names in its own record)
**Artefacts:** Snowdon Towers Architectural (Revit 2025) with Revit's IFC4 export made with "Export Revit property sets" (puzzbobb's, local only: the export cannot be redistributed); puzzbobb's R2025 models (Revit 2025, Revit's IFC4 exports); the BIM4LCA office as built and at building permit and the housing on terrain (Revit 2024)
**Result:** positive: each wall, floor, ceiling and roof lists its type's saved parameters after its own, and on Snowdon every type value Revit's export writes that rvt-rs lists is Revit's.

## 1. Before

RE-207 lists a family instance's type parameters through `FamilyInstance.m_masterSymbolId`. A wall, floor, ceiling or roof is a system-family element, has no such member, and so listed none of its type's: on Snowdon 1 of 1,380 such elements had any value at type level (an in-place wall, a family instance). Revit's export, with "Export Revit property sets", writes those types' values on each element's type object (`Identity Data`, `Construction`, ...).

## 2. Each record names its type

`examples/probe_35_system_type_links.rs` lists every ElementId-valued member of each element's native record root and the class of the record it names. On Snowdon's 1,380 walls, floors, ceilings and roofs that Revit's export types, exactly one member names a type record:

| element record | member | type record | Snowdon |
|---|---|---|---:|
| `SWall` | `m_WallAttributesId` | `BasicWallType` | 1,037 |
| `ArcWall` | `m_WallAttributesId` | `BasicWallType` | 24 |
| `FaceWall` | `m_typeId` | `BasicWallType` | 16 |
| `Floor` | `m_floorAttributesId` | `FloorAttributes` | 176 |
| `Ceiling` | `m_ceilingAttributesId` | `CompoundCeilingType` | 68 |
| `ProfileRoof` | `m_roofAttributesId` | `RoofAttributes` | 20 |

The R2025 roof by extrusion is a `Roof` record with the same `m_roofAttributesId`. RE-253 found the `SWall` member names the type rvt-rs gives every wall. Stair landings (`StairsLanding`, 17 of Snowdon's slabs) name no type record this way and are left out.

## 3. What rvt-rs does

`native_element_parameters::read` also reads the records of the four type classes, and gives each element of the classes above its type's values after its own, at level `type`, as it does a family instance's: a parameter the element holds itself is not repeated. A ceiling's and a roof's type record now has its `SymbolInfo.m_name` read as its Type Name (-1002001), as a wall's and a floor's type already had.

## 4. Measured

Snowdon, each property Revit writes on the type object of a wall, slab, covering or roof, against the type-level values `rvt-elements` lists for the element (`tools/re/system_type_parameters_vs_ifc.py`: a text equal, a number within 1e-6 relative):

| property | `main` | this change | parameter |
|---|---:|---:|---|
| `Identity Data.Type Name` | 1 | 1,363 of 1,380 | -1002001 |
| `Identity Data.Assembly Code` | 0 | 1,362 of 1,362 | -1002500 |
| `Identity Data.Type Mark` | 0 | 889 of 889 | -1001405 |
| `Identity Data.Fire Rating` (walls) | 0 | 683 of 683 | -1001206 |
| `Materials and Finishes.Parapet Cap Material` | 1 | 1 of 1 | 1996136, a project parameter |

The 17 type names left are the stair landings (§2). Revit's other type properties are not saved values of the type: a wall's `Width` is its layers' sum, `Function` and the wrapping settings are written as their labels, `Assembly Description` is looked up from the assembly code, and the analytical properties are computed from the materials. Two further value coincidences are not counted: a slab type's `Coarse Scale Fill Color` 0 equals its `FUNCTION_PARAM` 0, and two wall types' names equal their structural material's name.

On every model, `rvt-elements`' output differs from `main`'s only by the added type-level values; nothing else in any element changes and no element is added or lost:

| model | type-level values added (elements) |
|---|---|
| Snowdon Towers Architectural | walls 3,714 (1,095), floors 569 (182), ceilings 310 (68), roofs 48 (20) |
| office as built | walls 2,224 (628), floors 124 (31), ceilings 390 (78), roofs 42 (10) |
| office at permit | walls 2,093 (589), floors 124 (31), ceilings 375 (75), roofs 42 (10) |
| housing on terrain | walls 82 (14), floors 20 (4) |
| R2025 models | the type's name on each of their 5 to 9 walls and on their roof |

`rvt-ifc`'s default output is byte for byte `main`'s on all of them, apart from the timestamp. With `--revit-property-sets` it is too, except on the two office files, where 5 roof types also carry their project parameter `U_value` (1.0) in `Identity Data`: the type's record is read now, and RE-210 writes a type's named parameters. The office's exports were made without Revit's property sets, so whether Revit writes it there is not measured.

Built-in parameters still have no name (#255); the ids above are what the type's values are listed under.

## 5. Reproduce

```bash
cargo run --release --example probe_35_system_type_links -- MODEL.rvt IDS.txt
cargo run --release --bin rvt-elements -- --no-typed MODEL.rvt > elements.json
python3 tools/re/system_type_parameters_vs_ifc.py elements.json REVIT.ifc
RVT_PROJECT_CORPUS_DIR=... cargo test --release --test integration element_parameters
```
