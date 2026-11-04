# RE-319: conduits, conduit fittings and gutters are Revit's own elements by `Tag` (measured on Revit 2018) and are recovered; cable trays and the centerline categories are not

**Date:** 2026-11-03
**Issues:** #1261 (a Revit 2018 project exports no element; RE-315 §9 left 178 conduits, conduit fittings and gutters as "the next category census"); follows RE-33 (the category census), RE-21 (the instance rule), RE-79 (nested MEP components), RE-131 (pipe cylinders), RE-315 (the 32-bit second prologue)
**Artefacts:** BIM-Speed's Vitoria `ALD26 Before.rvt` (Revit 2018, sha256 `d3e5b04631e67265927be98352021ed6290b33ae67d39b1b6727fd76f014ff6f`) and Revit's own IFC2X3 export of it (`a42c6f3b0c9b7575d4c37c675155ff53c02571ca81357a87023f9da5fbf45181`), CC BY 4.0, TU Berlin DepositOnce, doi:10.14279/depositonce-16441, spent for W5 (RE-315) and measured in full with `-f holdout=true`; RE1 Electrical (`9875e765c9f47893808c47b845a8379b8777b92510e3e0284f763cb457c9630b`, IFC `c7002a10c8e10ab2fca64ff61dcf38db04da652924f65ce53c2064827dfa9e16`), Mechanical (`d559720177aa0ec435133f804e7c45d60da2deb9d5585bb3d55f4830139c014b`, IFC `9e56cf7246ce86ef73646dd588a1efb8a4756e5e102b30e28b3b58577a454db0`) and Plumbing (`03689df2aa426031a216e65051b9f19d3265d6fe96884dd47373b17435f9e54c`, IFC `294490b222632509779910345f27a4eeda3645956138a593298c241495f7b8b6`), Revit 2025, MIT, IFC2X3 exports; Core Interior (`c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014`); the BIM4LCA office and its terrain variants, puzzbobb's 24 Revit 2025 models and Autodesk's sample projects of 2015 to 2027 (hashes in `research/bim4lca-measure.tsv`, `research/r2025-models.tsv` and `research/autodesk-sample-projects.tsv`); by counts only, the held-out `ALD26 Project`.
**Probe:** `probes/examples/probe_re319_category_census.rs`, scored by `tools/re/category_census_vs_ifc.py`.
**Status:** positive on Revit 2018's `ALD26 Before` against Revit's own export, the only file with an export that holds any of these categories; no other release's export is measured for them. Cable trays and cable tray fittings are **not recovered** (not measured: no file with a Revit export holds an instance). The centerline categories are never recovered.

## 1. The census

`probes/examples/probe_re319_category_census.rs` reads every file through the exporter's own record reader (the 2008 to 2023 `scan_records`, or the 2024 to 2027 product sweep over every category the file's marker frames carry, so a category outside `RECOVERED_CATEGORIES` shows too) and prints, per category, the ElementIds of any decoded frame, of the placed instances the RE-21 rule selects (no container reference, the placed-instance kind) and of those the exporter writes (a 3D body, in the export phase, outside a non-primary design option). `category_census_vs_ifc.py` scores each set against the `Tag`s of Revit's export. Measure run 41726416015 (`-f holdout=true -f samples=true -f nordic=true -f r2025=true -f bimcollab=true -f models="spent-* core-interior einhoven re1-* nordic-office-* r2025-* bimcollab-*"`, head `eba89c8e`).

Per category, frames with a decoded id / RE-21 instances / written (3D body, phase, option) / of those in Revit's export by `Tag`:

| file | conduits -2008132 | conduit fittings -2008128 | gutters -2001391 | cable trays -2008130 | cable tray fittings -2008126 |
|---|---|---|---|---|---|
| Vitoria `ALD26 Before` (2018, IFC2X3 export) | 102 / 102 / 102 / **102** | 135 / 74 / 74 / **74** | 2 / 2 / 2 / **2** | none | 5 / 0 / 0 / nothing to score |
| RE1 Electrical, Mechanical, Plumbing (2025); Core Interior (2024); the four BIM4LCA office files (2024); the 24 R2025 models; Einhoven (2023); every `rac` and `rst` sample | none of the five | none | none | none | none |
| Autodesk `rme_advanced` 2015, 2019, 2024 to 2027 (no export) | 19 / 19 / 19 | 32 to 34 / 14 / 14 | none | 17 / 17 / 17 | 48 to 50 / 14 / 14 |
| Autodesk `rme_basic` 2015, 2019, 2024 to 2027 (no export) | 20 / 20 / 20 | 32 to 34 / 14 / 14 | none | none | 14 / 0 / 0 |
| Snowdon Towers Electrical 2024 (no export) | 530 / 530 / 530 | 338 / 322 / 322 | none | none | 24 / 0 / 0 |
| Snowdon Towers HVAC 2024 and Plumbing 2024 (no export) | none | 6 and 15 / 0 / 0 | none | none | 7 and 24 / 0 / 0 |

RE1 Electrical, which a first reading would take for the file with conduits and trays, has none in its records, and its export holds 14 `IfcFlowTerminal` and no flow segment. On each export of the second row (RE1, Core Interior, the BIM4LCA files and the R2025 models) every element with a `Tag` is among the instances the probe selects ("export elements not written 0"), and none is in these categories, so none of those exports holds a conduit, conduit fitting, gutter, cable tray or cable tray fitting. The probe's "written" column is the rule, a 3D body, the phase and the design option, not every filter the exporter applies (empty curtain panels, components nested in doors and windows), so its "not in the export" counts on a category the exporter filters further are not the exporter's; for the three categories here the exporter's own output equals it (§6). The same ElementIds are in every release of `rme_advanced` and `rme_basic` (2015 to 2027), which no export of those samples checks.

On `ALD26 Before` the census agrees with RE-315 §9: every conduit, conduit fitting and gutter the rule selects is a `Tag` of Revit's export (178 of 178), in `IfcFlowSegment` (102 conduits, 2 gutters) or `IfcFlowFitting` (74 fittings), and none the rule selects is outside the export. The 61 other conduit fitting frames fail the rule (a container reference, or the type symbol's kind). The categories one step beside each, `-2008139` (102 frames) and `-2008141` (74), match the conduits and their fittings one for one, and are in Revit's export as nothing: they are `OST_ConduitCenterLine` and `OST_ConduitFittingCenterLine`, which revit-ifc's `ElementFilteringUtil` lists among the categories it leaves out of the export (cited, not copied), as `-2008045` and `-2008072` (`OST_PipeCurvesCenterLine`, `OST_PipeFittingCenterLine`) are for the 140 pipes and 128 fittings already written. All 444 of the centerline frames the rule selects there (102, 74, 140 and 128) are "not in the export" in the scorer's table, so none is recovered. The ids are Autodesk's published `BuiltInCategory` values: `OST_Conduit` -2008132, `OST_ConduitFitting` -2008128, `OST_CableTray` -2008130, `OST_CableTrayFitting` -2008126, `OST_Gutter` -2001391, `OST_ConduitCenterLine` -2008139, `OST_ConduitFittingCenterLine` -2008141.

## 2. What is recovered, and what is not

- **Conduits, conduit fittings, gutters:** recovered, under the unchanged RE-21 rule, with the record's box as the body, on every release the exporter reads (2008 to 2027; the rule and the record layout are those of every other category). Measured against Revit's export on 2018 only.
- **Cable trays and cable tray fittings:** not recovered. No file in the development set that has a Revit export holds an instance of either (the cable tray fittings the rule selects, 14 on each `rme` sample, are on files with no export, and on Vitoria 5 frames fail the rule), so nothing shows that Revit's export writes them, or in which class, or which it leaves out. They stay counted as unattributed frames where the file's layout counts them, and "not measured" here.
- **The centerline categories:** never recovered (above).

## 3. The class

Revit's IFC2X3 export of `ALD26 Before` writes conduits and gutters as `IfcFlowSegment` and conduit fittings as `IfcFlowFitting`. rvt-rs writes IFC4.

**Revit's own type objects** name the IFC4 entity. The same export types each element by a type object, and `class_sets_vs_ifc` and `category_census_vs_ifc` read it (runs 41730511780 and 41740219735): the 102 conduits by `IfcCableCarrierSegmentType` (`PredefinedType` `NOTDEFINED`), the 74 conduit fittings by `IfcCableCarrierFittingType` (`NOTDEFINED`) and the 2 gutters by `IfcPipeSegmentType` with `PredefinedType` `GUTTER`; its 140 pipes by `IfcPipeSegmentType` (`NOTDEFINED`) and 128 pipe fittings by `IfcPipeFittingType`, which rvt-rs already writes as `IfcPipeSegment` and `IfcPipeFitting`. So the IFC4 entities are `IfcCableCarrierSegment`, `IfcCableCarrierFitting` and `IfcPipeSegment`, each a subtype of the `IfcFlowSegment` or `IfcFlowFitting` Revit writes for the occurrence.

What revit-ifc says (Autodesk's open-source exporter, CLEANROOM source 5: read for what the export writes, cited by file and class, no code taken):

- `Source/Revit.IFC.Common/Utility/IFCCompatibilityType.cs`, the dictionary `SuperTypeCompatibility`, says which IFC4 subtypes Revit writes as their supertype before IFC4: `IfcCableCarrierSegment`, `IfcCableSegment`, `IfcDuctSegment` and `IfcPipeSegment` as `IfcFlowSegment`; `IfcCableCarrierFitting`, `IfcCableFitting`, `IfcDuctFitting`, `IfcJunctionBox` and `IfcPipeFitting` as `IfcFlowFitting`. It is the IFC4-to-IFC2X3 direction, and it picks no subtype for a category; it is why the occurrences are `IfcFlowSegment` and `IfcFlowFitting` in this export.
- `Source/Revit.IFC.Export/Exporter/FamilyInstanceExporter.cs`, `CanHaveSystemDefinition`, tests the types `IfcCableSegmentType`, `IfcCableCarrierSegmentType`, `IfcCableFittingType` and `IfcCableCarrierFittingType` against `OST_CableTrayFitting` and `OST_ConduitFitting`: a conduit fitting is exported as one of the cable types.
- `Source/Revit.IFC.Export/Toolkit/IFC4Enums.cs`: `IFCCableCarrierSegmentType` has `CONDUITSEGMENT`, `IFCPipeSegmentType` has `GUTTER`.
- The category-to-class table that decides the class of each category is Revit's own (`IFCCategoryTemplate`, `ExporterCacheManager.CategoryMappingTemplate`) and is not in revit-ifc's source, so **revit-ifc names the IFC4 class of none of `OST_Conduit`, `OST_ConduitFitting` and `OST_Gutter`**. The class of the gutter, `IfcPipeSegment`, therefore rests on the type objects of Revit's own export (2 of 2) and on IFC4's `GUTTER`, not on revit-ifc.

What is written: conduits as `IfcCableCarrierSegment`, their fittings as `IfcCableCarrierFitting` and gutters as `IfcPipeSegment` (as pipes are `IfcPipeSegment` and `IfcPipeFitting` for the same two IFC2X3 classes). `PredefinedType` stays unset on all three, as a pipe's does: no IFC4 export shows Revit's value on an occurrence, and the writer never invents an enumerator it did not decode. Revit's type objects carry `GUTTER` for the gutters (2 of 2) and `NOTDEFINED` for the conduits and fittings, so writing `.GUTTER.` on a gutter is measured on its type object only; it is left for a decision. No type object is written for the three: a conduit fitting's type name is read and Revit's `Reference` property is written for it, as for every typed element, and 74 of 74 equal Revit's.

## 4. Bodies

Boxes, with their `BodySource`, as every other category. RE-131's cylinder is not applied: `attach_pipe_axes` reads a pipe's two ends and radius from its connector entries for the class `Pipe` on Revit 2024 and 2025 only (`partition_pipe_axes::PIPE_AXIS_SUPPORTED_REVIT_VERSIONS`), `ALD26 Before`'s own pipes are boxes, and no file with a conduit and a Revit export is 2024 or 2025, so nothing shows that a conduit's entries read the same way or what Revit's cylinder is. On Revit's export the written conduit boxes are the right ones: of the 104 conduits and gutters the scorer pairs by `Tag`, 102 have a world box within 0.01 ft of Revit's (`duct_bodies_vs_ifc`, worst 3.0 ft; the scorer does not say which two), and Revit's own body fills a median 0.374 of a conduit's box, 0.309 of a conduit fitting's and 0.003 of a gutter's (`box_bodies_vs_ifc`): a conduit is a thin cylinder in its box, a gutter a profile along a roof edge, so the shape is not decoded and `rvt-ifc` counts them with the other bodies that are their record's box.

## 5. Storey

Conduits and their fittings sit on the Level their record's list names, as the other MEP categories (RE-107): all 176 on Revit's storey (the 21 elements on another storey are the 14 columns, 4 pipe segments and 3 pipe fittings of RE-315 §7, unchanged). A gutter's record names no Level; Revit's export puts both on `P5`. `Gutter` is in `ELEVATION_PLACED_CLASSES` (RE-68, the classes Revit places by elevation: slab edges, wall sweeps, light fixtures, generic models), and both land on `P5`. That is two elements on one file; the band is the same fail-closed band the other classes use.

## 6. Result on `ALD26 Before`

Run 41730511780 (main's side `c28ff053`, head `9ade4f78`, every scorer, no model but the spent pair) and run 41731699676 (head `37838a94`, with the gutters' elevation) measured the code before the gutters became `IfcPipeSegment`; every figure below was measured again at the final head, `46da221f`, in the unfiltered run of §7 (41741114199), and is the same.

| `ALD26 Before` | main | head | Revit's export |
|---|---:|---:|---:|
| elements paired by `Tag` | 1,136 | **1,314** | 1,321 |
| written that Revit's export lacks | 0 | **0** | |
| in Revit's class or IFC4's for its IFC2X3 one | 1,127 | **1,305** | |
| storeys | 7 | 7 | 7 |
| elements on Revit's storey | 1,073 | **1,251** | |
| elements on another storey, on none | 21, 16 | 21, 16 | |

By Revit's class: 1,314 of 1,321 written. The same class: beams 1, building element proxies 43, columns 114, doors 68, members 206, plates 67, railings 14, roofs 8, slabs 9, stairs 6, stair flights 6, walls (`IfcWall`) 12, windows 70 (624). In IFC4's class for Revit's IFC2X3 one: 128 `IfcFlowFitting` as `IfcPipeFitting`, 140 `IfcFlowSegment` as `IfcPipeSegment`, 235 `IfcWallStandardCase` as `IfcWall`, and now **74 `IfcFlowFitting` as `IfcCableCarrierFitting`, 102 `IfcFlowSegment` as `IfcCableCarrierSegment` and 2 `IfcFlowSegment` as `IfcPipeSegment`** (681). Another class: 9 `IfcCurtainWall` as `IfcWall` (unchanged). **Missing, 7** (185 before): 4 `IfcWallStandardCase` (the curtain-panel walls of RE-315 §5) and 3 `IfcElementAssembly` (not instances). `IfcFlowSegment` absent 0 (104), `IfcFlowFitting` absent 0 (74).

The scorers that move (`compare-scorer-diff` of run 41741114199, the same list as run 41730511780's):

- `class_sets_vs_ifc`: `IfcFlowFitting` 202: 128 `IfcPipeFitting`, 74 `IfcCableCarrierFitting`, absent 74 to 0; `IfcFlowSegment` 244: 140 `IfcPipeSegment`, 102 `IfcCableCarrierSegment`, 2 `IfcPipeSegment` (the gutters, whose Revit type object is `IfcPipeSegmentType` `GUTTER`), absent 104 to 0; `IfcSpace` extras 73 as before (spaces are paired by GlobalId, which are not Revit's, #878). No extra element.
- `storeys_vs_ifc`: 1,314 elements, 16 on no storey (the 9 proxies, 6 railings and 1 flight of before); 1,251 on Revit's storey, 26 where Revit contains it in an `IfcSpace`, 21 on another, 9 and 7 on none; the 21 are the same 14 columns, 4 pipe segments and 3 pipe fittings as before, none of them a conduit.
- `box_bodies_vs_ifc`: record-box bodies 880 to 1,058 (+102 conduits, +74 fittings, +2 gutters).
- `duct_bodies_vs_ifc`: 0 to 104 flow segments written with a body, 102 of 104 within 0.01 ft of Revit's world box.
- `names_vs_ifc`: 74 conduit fittings equal in name and ObjectType (`M_Codo de tubo - Aluminio:Estándar:<id>`, Revit's own); 102 conduits and 2 gutters differ, as the 140 pipes do (`Conduit-<id>` and `Gutter-<id>` where Revit's is `Tubo con uniones:Tubo:<id>` and `Canalón:Canalón:<id>`: the system family's name is Revit's, in the file's language, and the type name is not read for a system type on Revit 2018).
- `type_objects_vs_ifc`: Revit's types on the elements written 1,102 to 1,280; none of the 178 has a type object of ours (flow segments untyped 140 to 244, flow fittings 0 to 74); the TypeName carried on the occurrence is Revit's for 666 of 1,280 (592 before, the 74 conduit fittings added).
- `psets_vs_ifc`: Revit's values on the objects written 4,961 to 5,317; equal 2,383 to 2,457 (the 74 fittings' `Pset_EnvironmentalImpactIndicators.Reference`); **different 0 to 0; values Revit does not write 20 to 20**; missing 2,578 to 2,860, 282 more, of which `Pset_DistributionFlowElementCommon.Reference` on the 176 conduits and fittings (a set Revit writes on those two classes and not on the pipes, and that the Reference allow-list of #1120 does not name, so it is left out) `Pset_EnvironmentalImpactIndicators.Reference` on the 104 conduits and gutters (no type name read), and `Pset_PipeSegmentTypeCommon.Reference` on the 2 gutters: 176 + 104 + 2 = 282.
- `global_ids_vs_ifc`: none of the elements' GlobalIds is Revit's (#878), as before.
- `schema.txt`: arity, validation, EXPRESS rules and property sets 0 findings; sets 1,668 to 1,742.

No other scorer moves on this file (`ports_vs_ifc`, `pipe_bodies_vs_ifc`, `qtos_vs_ifc`, `predefined_types_vs_ifc`: unchanged).

## 7. Other files, and the held-out Project

Run 41741114199 (`-f holdout=true -f samples=true -f nordic=true -f r2025=true -f bimcollab=true`, no model filter, every scorer; main's side `4ad28023`, head `46da221f`, the merge of main into the code of this report; the commits after it change documents only): 112 models exported on both sides, **97 byte for byte main's IFC and 15 moved**. The 15 are the models that hold the three categories where the rule selects an instance, and in each the only change is the conduits, conduit fittings and gutters written:

| model | change |
|---|---|
| Vitoria `ALD26 Before` (spent) | +102 conduits, +74 conduit fittings, +2 gutters (§6) |
| `ALD26 Project` (held out) | counts only, below |
| Autodesk `rme_advanced` 2015, 2019, 2024, 2025, 2026, 2027 | +19 conduits, +14 conduit fittings each (swept solids +33 on 2024) |
| Autodesk `rme_basic` 2015, 2019, 2024, 2025, 2026, 2027 | +20 conduits, +14 conduit fittings each (+34 on 2024) |
| Snowdon Towers Electrical 2024 | +530 conduits, +322 conduit fittings (+852 swept solids) |

Those samples have no Revit export: they are counted, not scored; their schema checks have 0 findings (the 2024 `rme` files' validation is skipped over 30 MB, as before). Unchanged, byte for byte: Core Interior, Einhoven, RE1 Architecture, Electrical, Mechanical and Plumbing, the four BIM4LCA files, the 24 R2025 models, BIMcollab's 2015 example, the other 17 held-out pairs (Frigento, Gdynia, Warmond, BIM Whale and the rest, whose `counts.txt` are main's line for line), and 45 samples, among them Snowdon Towers Architectural, Structural, HVAC and Plumbing (the last two hold conduit fittings and cable tray fittings that fail the rule). No other scorer output moves on any model.

**Held out, counts only** (`ALD26 Project`, Revit's 1,520 elements and spaces: 1,447 elements and 73 spaces): main wrote 1,336 (1,263 elements and 73 spaces); the head writes **1,514 (1,441 elements and 73 spaces)**. `IfcFlowSegment` absent 104 to 0 and `IfcFlowFitting` absent 74 to 0, the two classes where any change should show; no element written that Revit's export lacks (the extra column is `IfcSpace` 73 as before, spaces paired by GlobalId); 1,433 of the 1,441 elements in Revit's class or IFC4's (the other 8 are curtain walls written as walls); 1,359 of 1,441 on Revit's storey (1,181 of 1,263), 14 on none and 21 on another, as before; the 6 elements missing are 3 `IfcElementAssembly` and 3 `IfcWallStandardCase`. Properties: Revit's values on the objects written 5,784 to 6,140, equal 2,919 to 2,993, **different 0 to 0, values Revit does not write 18 to 18**. The file was not opened.

## 8. Reproduce

```bash
# the census, on every development file (the samples run the probe alone)
gh workflow run measure.yml --ref main -f ref=3a81189c6dde0783004c41d3d2e4daf27930e1e8 -f base=none -f holdout=true -f samples=true -f nordic=true -f r2025=true -f bimcollab=true -f models="spent-* core-interior einhoven re1-* nordic-office-* r2025-* bimcollab-*" -f scorers=none -f probe=probe_re319_category_census -f probe_scorer=category_census_vs_ifc
# Vitoria Before, every scorer, against main, with the IFC kept
gh workflow run measure.yml --ref main -f ref=46da221fc89f259d7dd535e06e8f49a7ac0286a3 -f base=main -f holdout=true -f models="spent-*" -f keep_ifc=true
# the final head: every model, every scorer, every held-out pair (counts only)
gh workflow run measure.yml --ref main -f ref=46da221fc89f259d7dd535e06e8f49a7ac0286a3 -f base=main -f holdout=true -f samples=true -f nordic=true -f r2025=true -f bimcollab=true
```

Runs: census 41726416015 (head `eba89c8e`); the categories' names and types in Revit's export 41729789378 and 41740219735; `ALD26 Before` against main 41730511780 (head `9ade4f78`, before the gutters' elevation and the Level parameter) and 41731699676 (head `37838a94`, with the 2024 samples); the final run 41741114199 on `46da221f`. An earlier unfiltered run, 41733219416 on `4e9e3ef1`, wrote the gutters as `IfcFlowSegment` before the type objects of Revit's export (run 41740219735) showed `IfcPipeSegmentType`; it moved the same 15 models and is superseded. The census scorer is in `tools/re` at `3a81189c`; the scorer for the final run is the one of the head.

## 9. What stays missing, and what is not measured

- 4 curtain-panel walls and 3 element assemblies of `ALD26 Before` (RE-315 §9).
- Cable trays and cable tray fittings (above), on every file.
- The class and `PredefinedType` of a conduit, conduit fitting or gutter occurrence in an IFC4 export of Revit: no such export holds one. The IFC4 classes follow the type objects of Revit's IFC2X3 export of `ALD26 Before`; the gutters' `GUTTER` is on their type objects only (2 of 2).
- Conduits' and gutters' names, and their `Reference`: Revit 2018 writes `Tubo con uniones:Tubo:<id>`; the system type's name is not read there.
- A conduit's or fitting's `Constraints` Level parameter: `rvt-ifc` writes none for the three classes, since no export measures its name (Revit's own for a pipe segment is `Reference Level`, for a fitting `Level`).
- Ports and connections of conduit fittings: not written (`is_distribution_element` does not name `IfcCableCarrierFitting`), and the 2008 to 2023 path reads no joins at all.
- Conduit and gutter bodies are boxes (on Autodesk's 2024 and later `rme` samples 8 of the 33 new bodies each are turned boxes, RE-87). 102 of the 104 conduit and gutter boxes on `ALD26 Before` are within 0.01 ft of Revit's body; the other two are not attributed, and may be the two gutters.
- Type objects: Revit types all 178 (`IfcCableCarrierSegmentType`, `IfcCableCarrierFittingType`, `IfcPipeSegmentType` with `GUTTER`); rvt-rs writes no type object for them. A gutter, written as `IfcPipeSegment`, would be typed with `NOTDEFINED` if it ever carried a type name; none does on the files measured.
- Snowdon Towers Electrical's 530 conduits and 322 fittings are counted, not scored: it has no Revit IFC export here (its VIM export, #408, is held out).
- Systems: the 2024 and later path's MEP system membership is not class-gated, so a conduit may join a system there; that is not measured.
