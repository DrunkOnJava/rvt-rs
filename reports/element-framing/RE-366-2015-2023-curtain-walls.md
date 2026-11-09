# RE-366: Revit 2015 to 2023 curtain walls, their parts and their axes

**Date:** 2026-11-06
**Issues:** #1190 (geometry beyond the box on 2015 to 2023, 2026 and 2027); follows RE-46 (curtain walls and their parts), RE-62, RE-72 (curtain grids), RE-81 and RE-85 (a 32-bit record's reference list), RE-106 (a part's own axes), RE-193 and RE-269 (a part's `IsExternal`), #439 (doors set in no curtain wall).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored.
**Scorers:** `tools/re/save_pair_bodies.py` (box and volume, not shape equality), `tools/re/curtain_classes_across_saves.py` (class and aggregation by `Tag`), and the Measure scorers against Revit's exports (section 3a).
**Status:** positive against the same samples saved by a later release, and against Revit's own exports of `rac_advanced` 2015 to 2020, Vitoria's `ALD26 Before` and the held-out `ALD26 Project`, **every one of them an IFC2X3 export**: this repository's standing rule counts a disagreement with an IFC2X3 view apart, so it cannot remove a value, and the same holds for the agreement in section 3a, which is a result against IFC2X3 too. **No curtain wall of a 2015 to 2023 file is scored against a Revit IFC4 export.** A later save cannot catch an error it shares.

## 1. Before

The 2015 to 2023 branch of `recover_partition_schema_mvp` never called the curtain readers. So a curtain wall was an `IfcWall`, its mullions and panels stood alone, and those off the model's axes were boxes along them. `rac_advanced` 2023 against its 2024 save, on `main`:

| | 2023 | 2024 |
|---|---:|---:|
| `IfcCurtainWall` | 0 | 34 |
| elements aggregated under another | 0 | 4,515 |
| `partition_family_instance_oriented_box` | 0 | 148 |
| members with `Span` and `Length` | 0 | 3,252 |

## 2. The change

- The branch calls `attach_curtain_walls` and `attach_curtain_axes` on `CURTAIN_32BIT_REVIT_VERSIONS` (2015 to 2023). 2008 to 2014 stay as they were.
- `attach_curtain_walls` takes a part's reference list as the 32-bit record read it, and `recover_2023_records` hands those lists over.
- `scan_transforms_in_layout` reads a part's transform on any release with an element-data layout. `scan_instance_transforms`, which door and family readers call, is unchanged.
- `bind_opening_hosts` runs after them, as on 2024, so that a door is set in no curtain wall (#439).
- A curtain wall takes `attach_record_is_external`, and its parts take its value through `attach_curtain_member_is_external`, as on 2024.

No new byte layout is decoded.

## 3. Against a later save

Each element paired by `Tag`, against the 2024 save, on `rac_advanced` (34 curtain walls, 4,515 aggregated) and `rac_basic` (9, 192):

| save | curtain walls | aggregated as 2024 | under another wall | not aggregated |
|---|---:|---:|---:|---:|
| `rac_advanced` 2015 to 2017 (each) | 34 of 34 | 4,384 | 0 | 80 |
| `rac_advanced` 2018, 2019 (each) | 34 of 34 | 4,463 | 0 | 1 |
| `rac_advanced` 2020 | 34 of 34 | 4,454 | 0 | 10 |
| `rac_advanced` 2021 | 34 of 34 | 4,444 | 0 | 71 |
| `rac_advanced` 2022 | 34 of 34 | 4,463 | 0 | 52 |
| `rac_advanced` 2023 | 34 of 34 | 4,454 | 0 | 61 |
| `rac_basic` 2015, 2016, 2019, 2022, 2023 (each) | 9 of 9 | 188 | 0 | 4 |

**Class and parts, over every save.** `tools/re/curtain_classes_across_saves.py`, on the exports of Measure run 42579926185 (head `6f71f93a`), pairs each wall of the 31 saves of 2015 to 2023 that have a 2024 save of the same project (`rac_advanced` and `rac_basic` of 2015 to 2023, `rme_advanced` and `rme_basic` of 2015 and 2019) with the 2024 save's by `Tag`: 511 walls are `IfcCurtainWall` here and in the 2024 save, none is `IfcWall` there, none that is `IfcCurtainWall` there is `IfcWall` here, and 4 walls (of `rst_basic` 2015) have no counterpart. No mullion or plate sits under a wall that is not a curtain wall, here or in the 2024 save; 49,903 sit under a curtain wall here. The 511 are 34 on each of 9 `rac_advanced` saves, 9 on each of 9 `rac_basic` saves and 31 on each of 4 `rme_*` saves, so the tables above, which cover the first two, hold 387 of them.

**Those not aggregated.** On `rac_advanced` 2023's 61, 45 are mullions at a corner whose record names two curtain walls, which the rule declines, 15 are stair flights and landings under their stairs, which this change does not read, and 1 is an opening. A curtain grid's data (RE-72) was tried in the 32-bit layout, where it holds its wall as a `u32` 53 bytes past the ElementId's eight: it reached none of these, so it is not read.

**Bodies,** to 3 mm on the world box and 0.1% on the volume:

| | `main` | this change |
|---|---:|---:|
| `rac_advanced` 2023, members | 3,206 of 3,312 | 3,312 of 3,312 |
| `rac_advanced` 2023, panels | 1,307 of 1,349 | 1,349 of 1,349 |
| `rac_advanced` 2015, 2019, members (each) | 2,779 of 3,276 | 2,885 of 3,276 |
| `rac_advanced` 2015, 2019, panels (each) | 1,145 of 1,349 | 1,187 of 1,349 |
| `rac_basic` 2023, members and panels | 188 of 188 | 188 of 188 |

The 148 oriented boxes are the 2024 save's on all four of `rac_advanced` 2015, 2019, 2021 and 2023. On `rac_advanced` 2019 no body that matched on `main` stops matching, and the 553 that still differ have the box they have on `main`, at worst 15 mm off. RE-180 records that `rac_advanced` 2019 and 2020 are an earlier edit in which curtain walls moved.

**Values,** on the walls and parts both saves hold them for:

| property | `rac_advanced` 2023 | `rac_advanced` 2015 |
|---|---:|---:|
| `Pset_MemberCommon.Span`, `Qto_MemberBaseQuantities.Length` | 3,252 of 3,252 | 3,241 of 3,252 |
| `Pset_MemberCommon.IsExternal` | 3,147 equal, 45 not read | 3,077 equal, 79 not read |
| `Pset_PlateCommon.IsExternal` | 1,307 of 1,307 | 1,307 of 1,307 |
| `Pset_CurtainWallCommon.IsExternal` | 34 of 34 | 34 of 34 |

The 11 on 2015 to 2020 are about 7.5 mm off, as their record boxes are. A part with no curtain wall takes no `IsExternal`.

## 3a. Against Revit's own exports

Measure run [42581222440](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42581222440) (head `6f71f93a` against `main` at `58fd08f2`, 118 exports, every scorer, `holdout`, `racadv`, `bimcollab`, `nordic`, `r2025` and `samples`). All of these Revit exports are IFC2X3. The held-out and `racadv` models give counts only.

| model | what moved |
|---|---|
| `rac_advanced` 2015 to 2020 (six pairs, the same counts on each) | `IfcCurtainWall` 34 of 34 (0 before); `Pset_CurtainWallCommon.IsExternal` 32 of 32 equal; `Pset_MemberCommon.Span` 3,241 equal, 11 different and 24 missing of 3,276 (0, 0 and 3,276 before); property values equal 2,503 to 5,776 of 16,108, different 0 to 11, missing 13,605 to 10,321, values Revit does not write 392 to 326 |
| Vitoria `ALD26 Before` (Revit 2018, open) | `IfcCurtainWall` 5 of 9 (0 before), the other 4 stay `IfcWall`; `Span` 206 of 206 (0 before), none different; `IsExternal` of curtain walls 5 of 9 equal; equal 2,737 to 2,948 of 5,317, different 0, missing 2,580 to 2,369, values Revit does not write 65 to 55 |
| Vitoria `ALD26 Project` (held out) | `IfcCurtainWall` 5 of 8 (0 before), the other 3 stay `IfcWall`; `Span` 206 of 206, none different; equal 3,291 to 3,502 of 6,140, different 0, missing 2,849 to 2,638, values Revit does not write 62 to 52 |

**The 11 different `Span` values** on each `rac_advanced` pair are the 3,241 of 3,252 of section 3: about 7.5 mm off Revit's, as their record boxes are. They are counted apart, against an IFC2X3 export, and they remove nothing. What they share is not established. **The agreement is against IFC2X3 as well:** Vitoria's `Span` 206 of 206 is a result against Revit's IFC2X3 export, not an IFC4 one.

**The walls that stay walls.** By the rules (RE-46, RE-72), a wall is a curtain wall where a placed mullion's reference list names it, or where a part (a mullion, a panel, a door or a window) names a curtain grid whose element data names the wall (`scan_curtain_grid_walls`). The 4 of `ALD26 Before` and the 3 of `ALD26 Project` that Revit writes as `IfcCurtainWall` stay `IfcWall`, and their `IsExternal` stays the wall's, so neither route names them: no mullion's list does, and no part names a grid whose data does. Whether they are curtain walls whose parts name no grid and no mullion names them, or walls whose parts' lists or grid data the reader does not read, is not established. The 7 elements `ALD26 Before` still lacks (4 curtain-panel walls and 3 assemblies) are not these: they are not written at all.

No export of 2024 or later, none of the office, the Revit 2025 models, Core Interior, RE1 or BIMcollab, and no other held-out model moves. The same-project gates pass.

## 4. What else changes

Across the 33 sample saves of 2015 to 2023, against `main`:

- **Doors and windows.** 266 doors no longer fill an opening in a curtain wall, and 235 curtain walls lose that void. All 266 and all 235 now equal the 2024 save.
- **Storeys.** 225 members and panels change storey, to the curtain wall's: all 225 equal the 2024 save.
- **`IfcWall` to `IfcCurtainWall`:** 511 elements. Each gains `Pset_CurtainWallCommon.IsExternal` and loses `Pset_WallCommon.IsExternal` and `ExtendToStructure`.
- **Type names.** 4 curtain walls on `rac_basic` gain a family name.
- **Nothing else.** No element appears or goes, and no GlobalId changes.
- **Byte identity.** Measure run 42581222440 hashes the IFC of 118 exports, this change against `main`: 88 are identical, and the 30 that differ are the 22 samples that hold curtain walls (`rac_advanced` and `rac_basic` of 2015 to 2023, `rme_advanced` and `rme_basic` of 2015 and 2019), the six `racadv` pairs, and the two Vitoria models. A first count from a local run of 206 files gave 13, all of 2017 to 2023, which is not the set of saves that hold curtain walls. No export of 2024 or later moves.
- **Cost.** No difference above the noise of three runs each (`rac_advanced` 2023, about 4.3 s; `rac_basic` 2019, about 2 s).

## 5. Not measured, not read

- Against a Revit IFC4 export: see the status above. On Vitoria's `ALD26 Before` the docs' "9 curtain walls come out as walls" is now 4 (section 3a).
- A curtain wall's `Length`, which comes from a native read that does not run for these walls here.
- Family and type names on mullions and panels, and so their `ObjectType`, `Reference` and materials. A part's `NetVolume` and `NetSurfaceArea`.
- A corner mullion's curtain wall, a curtain grid's, a door or window's in a grid.

## 6. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2015 2024
for f in samples/*.rvt; do rvt-ifc "$f" -o "${f%.rvt}.ifc"; done
python3 tools/re/save_pair_bodies.py samples/2023-racadvancedsampleproject.ifc samples/2024-racadvancedsampleproject.ifc --sources partition_family_instance_oriented_box
# Curtain walls and aggregates by Tag across the saves of each project:
python3 tools/re/curtain_classes_across_saves.py samples/   # samples/sample-<year>-<name>/model.ifc, from a Measure run with keep_ifc
```
