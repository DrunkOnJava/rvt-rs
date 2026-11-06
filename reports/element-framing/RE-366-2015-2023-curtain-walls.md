# RE-366: Revit 2015 to 2023 curtain walls, their parts and their axes

**Date:** 2026-11-06
**Issues:** #1190 (geometry beyond the box on 2015 to 2023, 2026 and 2027); follows RE-46 (curtain walls and their parts), RE-62, RE-72 (curtain grids), RE-81 and RE-85 (a 32-bit record's reference list), RE-106 (a part's own axes), RE-193 and RE-269 (a part's `IsExternal`), #439 (doors set in no curtain wall).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored.
**Scorer:** `tools/re/save_pair_bodies.py` (box and volume, not shape equality).
**Status:** positive against the same samples saved by a later release. **No curtain wall of a 2015 to 2023 file is scored here against a Revit export**, and a later save cannot catch an error it shares. Vitoria's `ALD26 Before` (Revit 2018), BIMcollab's example and the `racadv-*` pairs have such exports; they are for Measure, and none was opened for this work.

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

## 4. What else changes

Across the 33 sample saves of 2015 to 2023, against `main`:

- **Doors and windows.** 266 doors no longer fill an opening in a curtain wall, and 235 curtain walls lose that void. All 266 and all 235 now equal the 2024 save.
- **Storeys.** 225 members and panels change storey, to the curtain wall's: all 225 equal the 2024 save.
- **`IfcWall` to `IfcCurtainWall`:** 511 elements. Each gains `Pset_CurtainWallCommon.IsExternal` and loses `Pset_WallCommon.IsExternal` and `ExtendToStructure`.
- **Type names.** 4 curtain walls on `rac_basic` gain a family name.
- **Nothing else.** No element appears or goes, and no GlobalId changes.
- **Byte identity.** Of 206 local project files of 2008 to 2027, 193 export IFC and diagnostics byte for byte as on `main` (`SOURCE_DATE_EPOCH=1700000000`). The 13 that change are 2017 to 2023 files holding curtain walls. No file of 2024 or later, or before 2015, changes.
- **Cost.** No difference above the noise of three runs each (`rac_advanced` 2023, about 4.3 s; `rac_basic` 2019, about 2 s).

## 5. Not measured, not read

- Against a Revit export: see the status above. The "9 curtain walls" the docs record as walls on Vitoria's `ALD26 Before` were measured before this change, and are not measured again.
- A curtain wall's `Length`, which comes from a native read that does not run for these walls here.
- Family and type names on mullions and panels, and so their `ObjectType`, `Reference` and materials. A part's `NetVolume` and `NetSurfaceArea`.
- A corner mullion's curtain wall, a curtain grid's, a door or window's in a grid.

## 6. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2015 2024
for f in samples/*.rvt; do rvt-ifc "$f" -o "${f%.rvt}.ifc"; done
python3 tools/re/save_pair_bodies.py samples/2023-racadvancedsampleproject.ifc samples/2024-racadvancedsampleproject.ifc --sources partition_family_instance_oriented_box
# Curtain walls and aggregates: compare IfcCurtainWall and IfcRelAggregates by Tag across the two exports.
```
