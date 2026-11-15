# RE-473: a Revit 2015 to 2023 family type holds its geometry-material map in the older id shape, and the 70 windows of #1603 name no type for it to be read from because their types' records are not found

**Date:** 2026-11-15
**Issues:** #1603 (Revit 2015 to 2023: family instances' materials, 70 windows and 14 railings lack Revit's on the spent Revit 2018 project); follows RE-203 (`FamilySymbol.m_geomTag2MaterialId`), RE-113 (the 32-bit type material scan), RE-280 and RE-364 section 11 (the id shape before 2024).
**Artefacts:** every sample project of Revit 2015 to 2027 that Measure fetches with `samples` (59 files), the reference models, the spent Revit 2018 project, and the four held-out Revit 2018 to 2021 projects as counts only; never stored.
**Probes:** `probe_1603_type_maps_pre2024` (full run at `650584d8244e`, focused run at `73b75d0d2a7b`) and `probe_1603_instance_types_pre2024` (run at `af1b6805d012`), all three archived under `refs/archive/2026-11-15/cited/`; `tools/re/type_maps_across_saves.py` pairs a save with the same project's later one.
**Status:** positive for the map, negative for the issue's windows and railings. The map is on every Revit 2015 to 2023 save, in a shape the reader does not take, and where the same project is saved in 2024 it names the same materials for 1,326 of 1,350 types. It does not reach the issue's 70 windows and 14 railings: the decode gives those instances no type, because the windows' type records are not found in that file and railings' and stairs' types have no such record on any release before 2024. Nothing in the output changes.

## 1. The question

The 64-bit branch of the partition decode replaces the scanned material set of a family type with the native map `m_geomTag2MaterialId` (`refine_type_materials`); the 32-bit branch never calls it. `native_type_materials::geometry_maps` takes a map entry's material as `/second/m_id/m_id64`, the 2024 form of an ElementId, and a save before Revit 2024 holds `/second/m_id/m_id`. #1603 asks why the 70 windows and 14 railings of a Revit 2018 project get no material set where Revit's IFC2X3 export gives them one. This report establishes what the file holds, not yet the reader.

## 2. What the probe counts

For each file, the family types its decode's instances name (`TYPE_ID_FIELD`), read through `native_document::extract_graphs` with `Options::releases` widened to the native releases and either id shape accepted:

- per release: the types, the records found, the types whose record held the field, and the entries in each id shape;
- per class of instance: the types, those read, those the map names a material for, those the scan names one for, and the types named only by the map, only by the scan, or whose entries are all unset;
- per class: the instances of the decode and how many carry a type id at all (added for the focused run, after the full run showed Window and Railing rows missing on the spent project).

The `type map:` lines name ElementIds and exist to pair a save with the same project's later one; the held-out scrub drops them.

## 3. Measured

Measure runs 43678727089 (head `650584d8244e`, 66 models, `-f samples=true`, the rows registered in the pull request before it was read) and 43679458150 (head `73b75d0d2a7b`, the pre-2024 samples, the spent project and the four held-out projects, with the instance lines).

**The id shape (row 1).**

| Releases | Sample files | Entries in the 2015 to 2023 shape | Entries in the 2024 shape |
| --- | ---: | ---: | ---: |
| 2015 to 2023 | 34 | 6,696 | 0 |
| 2024 to 2027 | 25 | 0 | 6,192 |

The spent Revit 2018 project holds 62 entries in the older shape and none in the other, and the four held-out files (Revit 2018, 2020, 2020, 2021) hold 61, 33, 93 and 215, all in the older shape. No file of any release holds both. A reader taking `m_id64` only sees none of these 7,160 entries.

**The reach (row 2).** The share of the instances' types whose record held the field:

| Release | Types | Read | Share |
| --- | ---: | ---: | ---: |
| 2015 | 275 | 256 | 93.1% |
| 2016, 2017, 2018 (each) | 120 | 111 | 92.5% |
| 2019 | 283 | 264 | 93.3% |
| 2020, 2021, 2022 (each) | 120 | 111 | 92.5% |
| 2023 | 198 | 184 | 92.9% |
| 2024 | 743 | 632 | 85.1% |
| 2025, 2026, 2027 (each) | 325 | 288 | 88.6% |

The share before 2024 (1,370 of 1,476 types) is above the share from 2024 (1,496 of 1,718). Before 2024 the 106 types without the field are all of four system-family classes (ceilings 41, roofs 45, ducts 15, pipes 5); from 2024 the 222 also include railings, stairs, stair runs, landings, stringers, ramps and wall sweeps, and 7 foundation and generic model types.

**The same project, saved later (row 5).** 32 pairs of a sample save before 2024 with the same project's 2024 save (or latest save) have types in both; 1,350 types compared, 1,326 name the same materials, 24 other materials, none only in the earlier save:

| Class | Types in both | Same | Other |
| --- | ---: | ---: | ---: |
| Window | 27 | 27 | 0 |
| Door | 155 | 155 | 0 |
| Furniture | 90 | 90 | 0 |
| Casework | 18 | 18 | 0 |
| FurnitureSystem | 18 | 18 | 0 |
| LightingFixture | 70 | 70 | 0 |

(The 2015 save of the structural sample pairs with none of its 2024 save's types: the ids differ. Types counted once per pair and class.) The 24 are 3 types of the structural sample, one of each of `StructuralFraming`, `StructuralFoundation` and `StructuralColumn`, in each of its 8 pairs; the entry counts match, and the earlier save names a material the 2024 save does not. That is the opposite of a missed read. Where the 2024 save has a type the earlier one has no map for (up to 14 types per pair), the files cannot say whether the type was added between the saves or its record was not reached. Both saves are read by the build under test, so a read they share would not show.

**Beyond the scan, and scan only (rows 3 and 4).** On the 34 pre-2024 sample saves, types per class that the map names a material for and the scan does not, and the reverse (types of each class repeat across a project's releases):

| Class | Types | Only the map | Only the scan | All entries unset |
| --- | ---: | ---: | ---: | ---: |
| Mass | 42 | 32 | 0 | 5 |
| LightingFixture | 70 | 18 | 3 | 5 |
| FurnitureSystem | 18 | 18 | 0 | 0 |
| Door | 155 | 10 | 0 | 0 |
| GenericModel | 95 | 9 | 5 | 14 |
| SpecialtyEquipment | 64 | 9 | 0 | 0 |
| StructuralFraming | 117 | 9 | 11 | 31 |
| ElectricalEquipment | 54 | 9 | 29 | 45 |
| PlumbingFixture | 26 | 1 | 11 | 15 |
| Window, Furniture, Casework, StructuralColumn, StructuralFoundation | 27, 90, 18, 82, 58 | 0 | 0 | 0 |

The scan also names materials for types whose map entries are all unset or absent: 36 of `DuctFitting`'s 50 types (all 50 unset), 37 of `PipeFitting`'s 55, 27 of `DuctTerminal`'s 35, 29 of `ElectricalEquipment`'s 54, and 41 ceiling and 29 roof types that hold no map. The 2024 saves show the same (`DuctFitting` 20 of 93 types named by the scan, all 93 unset). The scan has to stay for those types, as it does on 2024 and later; in all, 19 classes have a type only the scan names.

**The issue's windows and railings (row 3).** Not met. The spent Revit 2018 project has no Window and no Railing row: its decode holds 70 windows and 14 railings and none carries a type id (`instances class Window total 70 naming a type 0`, `Railing total 14 naming a type 0`; doors 66 of 68, stairs and stair runs 0 of 6 each). The held-out Revit 2018 save of the same project prints the same 70 and 14. Two other held-out projects name a type for every window (21 of 21, 110 of 110), a third for 25 of 29, and the 12 sample projects with windows (22 saves of 2018 to 2023) for all of theirs (24 or 17 each). Railings name no type on any pre-2024 file of the run, sample or not; on the 2024 saves their types hold no map either (44 types, none read), so the railings of the issue do not come from this field at all.

**Where the type join stops (`probe_1603_instance_types_pre2024`, run 43681309215).** A Revit 2015 to 2023 instance gets a type when exactly one id in its reference list is a symbol record of its own category with a name entry (RE-109) and that symbol's family resolves (`type_and_family_names_2023`). The probe repeats those stages and counts where each instance stops:

| File | Category | Instances | Symbol records (named) | Joined | No candidate | Stop at the family |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Spent Revit 2018 project | Windows | 70 | 1 (1) | 0 | 68 | 2 |
| Spent Revit 2018 project | Doors | 68 | 4 (4) | 66 | 0 | 2 |
| Spent Revit 2018 project | Railings, Stairs | 14, 6 | 0 | 0 | 14, 6 | 0 |
| Its held-out Revit 2018 save | Windows | 72 | 1 (1) | 0 | 68 | 4 |
| A held-out Revit 2020 project | Windows | 29 | 46 (19) | 25 | 4 | 0 |
| A held-out Revit 2020 project | Doors | 27 | 35 (10) | 18 | 9 | 0 |
| Two other held-out projects | Windows | 21, 110 | 27 (13), 26 (9) | 21, 110 | 0 | 0 |
| 22 sample saves, 12 with windows | Windows | 246 | 108 (66) | 246 | 0 | 0 |
| 22 sample saves, 21 with doors | Doors | 1,080 | 570 (300) | 1,068 | 0 | 12 |

- **The Revit 2018 project's windows.** The file holds one symbol record of the window category, and 68 of its 70 windows never name it. All 68 name a named id that has no record of its own, so their types are named in the file but their type records are not among the records the scan finds. The two others name the one record and stop at its family.
- **Another cause, on a held-out Revit 2020 project.** 4 of 29 windows and 9 of 27 doors name a symbol record of their category that has no name entry; their types are found and have no name.
- **Railings and stairs.** Their types are system types with no symbol record, so no join candidate exists: none of the 248 railings and 84 stairs of the 22 sample saves and the spent project joins, nor do the held-out files' (7 to 14 railings and 3 to 6 stairs on the files that have them). A 2024 save names them through `railing_type_picks`, which the 32-bit branch does not call.

## 4. The registered rows

1. **Shape.** Met: 6,696 entries of 34 sample files in the older shape, none in the 2024 shape, and the reverse on 2024 to 2027 (6,192, none in the older).
2. **Reach.** Met: 92.8% of types before 2024 against 87.1% from 2024.
3. **Beyond the scan.** Met on the pre-2024 samples for the classes the issue says `rac_basic` 2023 lacks against its 2024 save that hold a map (system furniture 18 types, light fixtures 18, generic models 9, plumbing fixtures 1; furniture 0, the scan names all 90). Not met for the issue's windows and railings, whose instances carry no type.
4. **Scan only.** Non-zero for 19 classes; the scan stays for the types whose entries are unset, as on 2024 and later.
5. **Held to the 2024 save.** Met on the classes the issue names (Window, Door, Furniture, Casework, FurnitureSystem, LightingFixture: 0 other of 378 types); 24 other of 1,350 in all, three structural types of one project.

## 5. What this decides and does not

- **Decided.** A reader that took both id shapes would read the same map entries a Revit 2024 save holds for the same types, on the classes where there are Revit 2024 saves to hold it to. The 64-bit refinement reads none of them on a release before 2024 today.
- **Not decided.** Whether Revit's export writes those materials. The map was not scored against any export: `element_materials_vs_ifc.py` on a reader's own run is the measure.
- **The issue's windows and railings are another cause.** Their instances carry no type id, so no map or scan could be applied. For the windows the join stops because the type records are not found (68 of 70 name a named id with no record); for the railings and stairs because their types have no symbol record on any release before 2024. Whether a window's type can be recovered from such a list by another rule (the one named id without a record that other windows of the file share, with the family told apart from the type) is not measured: it is the next row.
- **The default material on 72 elements Revit gives none** (the issue's second item) is untouched here.

## 6. What would decide it

A reader change of its own: `native_type_materials` and `native_family_forms` take either id shape (`native_json::identifier_u64`), the map is read for the 32-bit branch, and the result is scored against Revit's exports with `holdout=true` and the same-project 2024 saves. Separately, a probe of the 68 windows' lists on the spent Revit 2018 project: how many named ids without a record each holds, how many windows share each, and which of them the file's other windows join as family or as type.

## 7. Reproduce

```bash
# run 43681309215 measured the commit below with the scrub patterns of this change; main holds both once it lands
gh workflow run measure.yml --ref main -f ref=af1b6805d0122c1285c7b241c1cff370a2c4c48b -f base=none \
  -f samples=true -f holdout=true -f scorers=none -f probe=probe_1603_type_maps_pre2024
# the same with -f probe=probe_1603_instance_types_pre2024 for the join
# each model's probe.txt (counts.txt for a held-out model) holds its `type maps pre2024:` and `type join pre2024:` lines
python3 tools/re/type_maps_across_saves.py DIR   # DIR holds the sample-*/probe.txt directories of a full run
```
