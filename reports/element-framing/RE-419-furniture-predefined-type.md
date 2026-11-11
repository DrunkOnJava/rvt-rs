# RE-419: a furniture element's `PredefinedType` is its own "IFC Predefined Type" where that is an enumerator, and `NOTDEFINED` where it has none, on the releases whose export writes it

**Date:** 2026-11-11
**Issues:** #1681 (furniture: rvt-rs writes no `PredefinedType`), #1636 (second-project measurement); follows RE-417 (the Export to IFC As values) and RE-45 (the export parameters)
**Artefacts:** the BIM4LCA housing's four large files (concrete and timber, as built and at permit) and the office's four (Revit 2024, Revit's IFC4 exports), and the held-out house and office pairs of Revit 2022 to 2027 (counts only), as Measure fetches them. Never stored.
**Runs:** Measure [42945616762](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42945616762) (`probe_re45_ifc_export_parameters` with `tools/re/export_parameters_vs_ifc.py`, on `main` at `134db9761fc48eaf4eceefb3f37e7591e7ec5e29`) and [42949444153](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42949444153) (`tools/re/predefined_types_vs_ifc.py` on the held-out pairs, whose summary lines `tools/ci/scrub-holdout-results.sh` now keeps as counts) for section 1; [42951039566](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42951039566) (unfiltered) for section 3; [42956695754](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42956695754) (the Autodesk samples with both sides' exports kept) and [42959714727](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42959714727) (the furniture type objects) for section 4
**Status:** Positive on Revit 2024, 2025 and 2026. 872 of the 884 furniture the 19 scored models hold read `PredefinedType` equal to Revit's, none different, and the other 12 are the held-out 2022 pair's, which a 32-bit release's missing export parameters keep out; 25 of 126 exports move.

## 1. What Revit writes

`tools/re/predefined_types_vs_ifc.py` (which read `IfcFurniture` since RE-417) pairs each `IfcFurniture` of rvt-rs's export with Revit's by `Tag`. Before this change rvt-rs wrote no `PredefinedType` on 104 of the 107 furniture of the concrete housing files and 103 of the 106 of the timber ones, where Revit's export writes:

| Revit writes | housing, concrete | housing, timber | office (each of four files) |
|---|---:|---:|---:|
| `NOTDEFINED` | 89 | 90 | 36 |
| `CHAIR` | 9 | 9 | 0 |
| `BED` | 6 | 4 | 0 |
| `USERDEFINED` | 3 (written equal since RE-417) | 3 | 62 |

The probe lists each element's own export parameters: the 15 furniture of the concrete housing (13 of the timber one) that carry "Export to IFC As" `IfcFurnitureType` carry "IFC Predefined Type" `CHAIR` (9) or `BED` (6 and 4), exactly the elements and values Revit's export writes them with; the office's 62 `USERDEFINED` carry "Export to IFC As" `IfcFurniture` with "IFC Predefined Type" `USERDEFINED`. The other 89 or 90 and 36 carry no predefined type, and Revit writes `NOTDEFINED` on them. revit-ifc's `IFCExportInfoPair` (CLEANROOM.md source 5) reads the same way: on its `release/2024.1` branch a value that is an enumerator of the entity is kept and any other, an empty one included, becomes `NOTDEFINED`; on `master` an empty one is left to the same default, and a value that is no enumerator becomes `USERDEFINED` with the text as the user-defined type. The two agree on the cases measured here (an enumerator, or none) and not on a value that is no enumerator.

**The held-out pairs** (run 42949444153, counts only): the house and the office of Revit 2022, 2024, 2025 and 2026 and the single-dwelling house of the CIADD pair hold 4, 8 and 6 furniture each, and Revit writes `NOTDEFINED` on every one. The two Revit 2027 pairs write **no** `PredefinedType` on any furniture (4 and 8), stair, stair flight, slab or member: `Revit's: {'-': n}` on every class that has one.

## 2. The rule

`IFCFURNITURE`'s enumerators (`IfcFurnitureTypeEnum`) are in `category_map::PREDEFINED_TYPE_ENUMERATORS`, so any furniture or casework element takes its own or its type's "IFC Predefined Type" where it is one, as every other entity there does; RE-417 kept them out because the own predefined type of the furniture a category maps was not measured. An element that has none takes `NOTDEFINED` on the releases of `FURNITURE_DEFAULT_REVIT_VERSIONS`, 2024, 2025 and 2026, the releases whose files rvt-rs reads the export parameters of and the held-out pairs and the BIM4LCA files show it on: the default is the mapping's, which for furniture and casework is none, so on any other release such an element carries no `PredefinedType`, as before. Revit 2027 is left out because its pairs write none; Revit 2022 is left out because the 32-bit releases (2023 and earlier) do not read the export parameters at all (`attach_ifc_export_overrides` runs on the 64-bit releases only), so a 2022 file cannot tell a furniture with an "IFC Predefined Type" of `CHAIR` from one without, and a default there would be a wrong `NOTDEFINED` where Revit writes `CHAIR`: the 2022 pair's 4 and 8 furniture stay unset (Measure shows both sides identical), where Revit writes `NOTDEFINED`. Each furniture or casework element carries the Revit release of its file (`m_furniture_release`) for that. An element whose value is no enumerator keeps none on every release, since the two revit-ifc versions disagree on it and no file measured here has one.

On the Revit 2027 pairs rvt-rs's `PredefinedType` of a stair, a flight, a slab and a member differs from Revit's none (the scorer counts 1, 1, 3 and 2 across the two), where it is Revit's on the 2026 pairs: not changed here, and recorded for a change of its own (#1692).

## 3. Measured: the change

Unfiltered Measure [42951039566](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42951039566), every flag, 126 models on each side, head `a066df723875bf8ef3e6be6e57339791b5ccde9e` (kept under `refs/archive/2026-11-11/cited/a066df723875`) against `0649e224b96285c2bced628c5379bdd4be924727`, its merge base with `main` (the later commit `1d1262b7`, which takes 2022 out of the release list, changes no export: no 2022 file reaches the list, section 2): 101 exports are byte-identical and 25 move, the housing's six files and the office's four, the held-out pairs of 2024, 2025 and 2026 and the CIADD house, RE1 Architecture, and seven Autodesk samples (section 4). `same-project gates` passed on the head.

| `IfcFurniture` `PredefinedType` equal to Revit's | before | after |
|---|---:|---:|
| housing, concrete (as built and at permit) | 3 of 107 | 107 of 107 |
| housing, timber (as built and at permit) | 3 of 106 | 106 of 106 |
| office, each of its four files | 0 of 98 | 98 of 98 |
| held-out house of 2024, 2025 and 2026 | 0 of 4 each | 4 of 4 each |
| held-out office of 2024, 2025 and 2026 | 0 of 8 each | 8 of 8 each |
| CIADD house | 0 of 6 | 6 of 6 |
| held-out house and office of 2022 | 0 of 4 and 0 of 8 | 0 of 4 and 0 of 8: not changed (section 2) |
| held-out house and office of 2027 | 4 of 4 and 8 of 8 | 4 of 4 and 8 of 8: both write none |

Over the 19 models that hold furniture a Revit export scores, 872 of 884 read equal, and the 12 that do not are the 2022 pair's; `differs` is 0 and `NOTDEFINED` where Revit writes none is 0 on every class of every model. The only other scored line of any model that is not 0 is the 2027 pairs' `differs` on stairs (1), stair flights (1), slabs (3) and members (2), where rvt-rs writes a value and Revit none: identical on both sides of this change and recorded in section 2 (#1692). No other line of any census (`qtos_vs_ifc`, `psets_vs_ifc`, `class_sets_vs_ifc` and the others) changes on any model with a Revit export.

RE1 Architecture moves (its 23 furniture take `NOTDEFINED`): Revit's export of it is IFC2X3, which has no `PredefinedType` on the class, so it scores nothing, and, as the IFC2X3 views are, it is counted apart and never removes a value.

## 4. The Autodesk samples and the type objects

**The seven Autodesk samples that move.** Measure [42956695754](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42956695754) kept both sides' exports of the 58 samples (`samples=true keep_ifc=true scorers=none`, head `6792103992c19691e28c61f5aa736c7c7d770811`, kept under `refs/archive/2026-11-11/cited/6792103992c1`, against `0649e224b96285c2bced628c5379bdd4be924727`): 51 are byte-identical and 7 move, `rac_advanced` and `rac_basic` of 2024, 2025 and 2026 and Snowdon Towers Architectural of 2024. No Revit export of them exists, so each is read: `tools/re/furniture_predefined_types_diff.py <before> <after>` pairs every `IfcFurniture` and `IfcFurnitureType` by GlobalId, and on all seven the only change is `PredefinedType` of the furniture elements from none to `NOTDEFINED`: 104 in each `rac_advanced` save, 32 in each `rac_basic` save and 345 in Snowdon, 753 in all, every furniture of those files; no type changes, no name or tag differs and no entity is added or removed. RE1 Architecture, the one other file that moves, has the same change on its 23 furniture. `tools/re/ifc_element_diff.py`, which reads property and quantity sets, finds nothing to report on any of the seven: the attribute is not in a set.

**The furniture type objects.** In run 42951039566 each of the four large offices gains 2 type objects (1,405 to 1,407 concrete as built, 1,341 to 1,343 at permit, 1,430 to 1,432 timber as built, 1,397 to 1,399 at permit) and the housing's files and every other model gain none: a furniture type whose instances carry `NOTDEFINED` and `USERDEFINED` is written once for each, as Revit's export writes it. Measure 42959714727 (the final head `6792103992c19691e28c61f5aa736c7c7d770811`, kept under `refs/archive/2026-11-11/cited/6792103992c1`; `tools/re/predefined_types_vs_ifc.py` now tallies the `PredefinedType` of the `IfcFurnitureType` objects of both files, and `tools/ci/scrub-holdout-results.sh` keeps that tally as counts) reads rvt-rs's tally equal to Revit's on all 17 models it covers, the eight large BIM4LCA files (the housing's `BED` 6 or 4, `CHAIR` 9, `NOTDEFINED` 36 to 40 and `USERDEFINED` 1; the offices' `NOTDEFINED` 8 and `USERDEFINED` 36 to 42) and the held-out pairs of 2022 to 2026 and the CIADD house (`NOTDEFINED` 2 each). The exports of all 17 are byte-identical to those of run 42951039566, the head that run measured: taking 2022 out of the release list changes none.

## 5. Sources

RE-45, RE-417. revit-ifc's `IFCExportInfoPair` (`Source/Revit.IFC.Export/Utility/IFCExportInfoPair.cs`, branches `release/2024.1` and `master`; CLEANROOM.md source 5). No Autodesk source was read.
