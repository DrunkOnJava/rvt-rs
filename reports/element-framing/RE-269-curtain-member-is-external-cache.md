# RE-269 — Why Revit's export writes IsExternal true on 528 curtain members: a cache in the exporter

**Date:** 2026-10-30
**Issues:** #35 (parameters), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); closes the question RE-193 §6, RE-226 §2 and RE-241 left open
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024 files, build 20230411, with Revit's IFC4 exports, exporter 24.0.5.432), RE1 Architecture (Revit 2025, IFC2X3 export, exporter 26.2.0.20), Core Interior (exporter 24.0.20.20) and the R2025 models (exporter 25.0.2.419), as Measure fetches them
**Probe and scorer:** `probes/examples/probe_is_external_members.rs` and `tools/re/member_is_external_cache_vs_ifc.py`
**Run:** Measure [41106058055](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41106058055) (`-f nordic=true -f r2025=true`, every model)
**Result:** positive for the explanation, with no change to what rvt-rs writes. Revit's value for a mullion or panel is not in the model: it is the host rule of revit-ifc's `IsElementExternal`, applied through an ElementId cache that the exporter that made the office exports filled with its element types. Replaying that cache in the exporter's order gives Revit's value on all 10,263 members and panels of the office as built (0 different) and on all 10,300 at building permit (0 different). The plain host rule gives it on 1,651 of the as-built 10,263 (1,691 of 10,300 at permit).

## 1. The values

Revit's export of the office writes `IsExternal` on every curtain mullion (`Pset_MemberCommon`, 10,003 as built and 10,031 at permit, true on 498 of each) and every curtain panel (`Pset_PlateCommon`, 260 and 269, true on 30). The mullions' 10,003 are 10,003 of the 12,853 property values the export writes on objects rvt-rs writes that rvt-rs does not (W7). RE-241 showed that each curtain wall's members share one value (10,002 of 10,003 mullions are pure by wall, 220 walls) and that no field of the member, its type or its wall says which walls: 156 of the 220 walls are exterior by type Function and by `Pset_CurtainWallCommon.IsExternal`, and only four of them have true members.

## 2. Revit's rule, read in revit-ifc

Read in revit-ifc (CLEANROOM.md source 5; read and cited, no code copied) at `IFC_v24.3.40`, `IFC_v24.2.0` and `IFC_v24.1.0`:

- `IsExternalCalculator` calls `CategoryUtil.IsElementExternal` on the element the property set is built for. `IsElementExternal` takes, in order: a value already cached under the element's ElementId; a Yes/No parameter named `IsExternal` on the element or its type; a value cached under the element's type's ElementId; the type's Function (`FUNCTION_PARAM`; the type's id is then cached with the result); a roof is external; a family instance takes its host's value; otherwise false. One dictionary, `ExporterCacheManager.IsExternalParameterValueCache`, holds element ids and type ids alike, and every step but the host step caches the element's own value under its id.
- A mullion or panel is a family instance, and on these files its type has no Function, so it reaches the host step: the value of the curtain wall. That is the host rule RE-193 measured.
- `ProductWrapper.Dispose` creates the property sets of the elements a wrapper holds first (`PropertyUtil.CreateInternalRevitPropertySets`) and those of their element types after (`PropertyUtil.CreateElementTypeProperties`, which passes the type itself to the property set entries, so `IsExternalCalculator` runs on the type). A curtain wall's members are exported inside the wall's own wrapper (`CurtainSystemExporter.ExportCurtainObjectCommonAsContainer`), so their sets are created when the wall's wrapper is disposed, instances first.
- At `IFC_v24.3.40` (and from `IFC_v24.2.20` on) `IsElementExternal` returns null for an element type: nothing is cached for it. At `IFC_v24.1.0` and `IFC_v24.2.0` it does not: a type with no Function, no parameter and not a roof, and which is no family instance, falls to the last step, `CacheIsElementExternal(typeId, false)`.

So in the older exporter, the members of the first curtain wall (in export order) that uses a member type get the wall's value, the type then caches not external, and every member of that type on a later wall reads the cached false at the third step, before the host step is reached.

## 3. Measured

The office's exports are made by `Autodesk Revit 24.0.5.432 - IFC 24.0.5.432` (the FILE_NAME header `originating_system`). That version is not among revit-ifc's tags; `IFC_v24.1.0` and `IFC_v24.2.0` are the nearest, and the replay below is the test that it behaves as they do. The scorer replays the cache over the probe's list of members (type = `m_masterSymbolId`, host = `m_hostId`, the host's native Function from RE-185): walls in ascending ElementId, each wall's members take the cache if their type is in it, else the wall's Function (external unless 0), and after each wall its member types enter the cache as not external. The 220 `IfcCurtainWall` entities of Revit's exports are in ElementId order (0 inversions), so the order is the same by the exports' own entity order.

| | Office as built | Office at permit |
|---|---:|---:|
| `IfcMember` Revit writes true / false | 498 / 9,505 | 498 / 9,533 |
| `IfcMember`: cache replay equal / different | 10,003 / 0 | 10,031 / 0 |
| `IfcMember`: host rule equal / different | 1,424 / 8,579 | 1,454 / 8,577 |
| `IfcPlate` Revit writes true / false | 30 / 230 | 30 / 239 |
| `IfcPlate`: cache replay equal / different | 260 / 0 | 269 / 0 |
| `IfcPlate`: host rule equal / different | 227 / 33 | 237 / 32 |

By member type (as built; the permit file has the same types and the same true counts):

| member | members | walls using it | first wall in export order | its Function | Revit true | Revit false |
|---|---:|---:|---|---|---:|---:|
| mullion type A (RE-193's 1907093) | 8,857 | 145 | 1906414 | exterior | 397 | 8,460 |
| mullion type B | 218 | 11 | 1417009 | exterior | 100 | 118 |
| mullion type C | 2 | 2 | 1417025 | exterior | 1 | 1 |
| mullion type D | 926 | 64 | an interior wall | interior | 0 | 926 |
| panel type E | 258 | 71 | 1417009 | exterior | 28 | 230 |
| panel type F | 2 | 1 | an exterior wall | exterior | 2 | 0 |

Two walls hold a split: one has 1 true of its 5 members (the one mullion of the rarely used type C) and one 2 true of its 29 (its two panels of type F). Type D is false throughout because the first wall that uses it is interior. Every member of the first wall of a type gets the wall's value, because the type enters the cache only after the wall's instances are done. Revit's export also writes `IsExternal` false in the common set of the type object of each member type (the scorer reads it on the IFC types of all 10,263 members), which the source of `IFC_v24.3.40` does not produce: the type's value comes from the same calculator in the older exporter. RE1 Architecture's export, made by exporter 26.2.0.20 in IFC2X3, writes none on types.

## 4. What is not shown

- **A later exporter.** No reference export with curtain members that share a type across walls was made by an exporter after `IFC_v24.2.0`. RE1 Architecture (exporter 26.2.0.20) has 10 mullions and 2 panels on one interior wall, false by either rule; the R2025 models (exporter 25.0.2.419) hold no curtain member (the 2 members of three of them are stair stringers); Core Interior's export writes no property values. That a later exporter writes the host rule's values (about 9,140 of the office's members true, on exterior walls) is read from its source, not measured.
- **The exporter version is not in the model.** The office's files were saved by build 20230411 and exported by 24.0.5.432; RE1 Architecture was saved by build 20250515 and exported by 26.2.0.20; Core Interior (build 20230509) by 24.0.20.20. Nothing in a `.rvt` says which exporter will export it, so the file does not say which of the two values Revit would write.
- **Other classes.** Walls, doors and the rest decide `IsExternal` by a parameter or a Function (RE-193) and take the cache only where a type repeats without one; the office's other classes measured equal without it.

## 5. What rvt-rs does

Nothing is written: rvt-rs writes no `IsExternal` on curtain mullions and panels, as before (RE-193), and none on the type objects of their types, where Revit's export writes false (§3). The values are a function of the exporter's order and a defect Autodesk removed from later versions, not of the model. The maintainer decided on 2026-10-30, between three choices, to leave them out and count them apart as known-wrong exporter output, with `WalkingLineOffset` and the mullions' `CrossSectionArea` (RE-235 §5): the census reports the 10,263 members' values (10,300 at permit) and their types' values as values of an exporter defect, not as missing.

The choices he did not take:

- **Write the replay on Revit 2024 files**: exact on both office exports, which would close the 10,003 mullion values in W7's 12,853 missing (and the 260 panels'), and wrong for a 2024 file that a later exporter exports, which the model cannot tell.
- **Write the host rule**: Revit's on 1,651 of the as-built 10,263 against this export, so not Revit's value here.

### Update, 2026-10-30: the host rule is written

The maintainer then took the choice §5 lists second, the host rule, because it is the value exporters from IFC_v24.2.20 on give (the replay stays unwritten): each mullion and panel takes its curtain wall's value, from the wall's own `IsExternal` parameter or type Function, as `IsElementExternal` gives a family instance its host's (RE-288, [#1187](https://github.com/DrunkOnJava/rvt-rs/issues/1187)). Nothing is written on the member types, which `IsElementExternal` at IFC_v24.2.20 and IFC_v24.3.40 returns null for. The office's values stay counted apart in the census, so the 8,579 mullions and 33 panels where the host rule differs from this exporter's export are not scored as different. The gap that remains: no model of a newer exporter has an exterior curtain wall, so the exterior members' values are checked against revit-ifc's source only.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main \
  -f ref=7476ca1ebb4d1fe38d018803cec23f5f47af52d6 -f base=none -f nordic=true -f r2025=true -f scorers=none \
  -f probe=probe_is_external_members -f probe_scorer=member_is_external_cache_vs_ifc
```
