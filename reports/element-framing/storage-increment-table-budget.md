# The storage increment table's value budget

**Date:** 2026-10-14
**Issues:** RE-78 (saved display meshes)
**Result:** positive. On two Revit 2024 houses the native preamble stopped in
`Global/DocumentIncrementTable` on a resource budget, not on the format, so
no saved graphics were read and every family instance stayed a box.

## Corpus

BIM4LCA / Nordic LCA architectural models (Gravicon, 2024), published by
Nordic Sustainable Construction under CC BY-SA 4.0 (`ARCH.zip`, linked from
VTT's "BIM files to download"). Twelve `.rvt` files (Revit 2024, build
20230411_1515) with Revit's IFC4 export of each. Kept local, like the other
corpora; nothing from them is committed. The publisher's site footer also
says "all rights reserved", which the page does not reconcile with the
CC BY-SA statement for the models.

## Observation

```text
rvt-ifc ARK_NordicLCA_Housing_Timber_BuildingPermit.rvt -o out.ifc --saved-meshes
rvt-ifc: saved graphics not read, bodies unchanged:
  DocumentIncrementTable.m_localIncrements at 418669:
  DocumentIncrement.m_incrementVersions at 418669:
  native field value budget exceeded (100000)
```

`native_index::storage_increments` decodes the table with
`decode_object_fields`, whose budget is `GraphLimits::default()`
(100,000 values). Decoded with a larger budget, the table has the same
fields as in every other file, and `storage_increments` accepts it. Values
used and the stream's size, on every project file at hand:

| file | values | bytes | bytes per value |
|---|---:|---:|---:|
| Housing_Timber_As-Built | 109,543 | 458,972 | 4.19 |
| Housing_Timber_BuildingPermit | 106,289 | 445,362 | 4.19 |
| Housing_Concrete_As-Built | 87,767 | 368,186 | 4.20 |
| Housing_Concrete_BuildingPermit | 83,491 | 350,240 | 4.19 |
| Office_Timber_As-Built | 32,757 | 129,352 | 3.95 |
| Office_Timber_BuildingPermit | 31,801 | 125,892 | 3.96 |
| Office_Concrete_As-Built | 29,579 | 117,024 | 3.96 |
| Office_Concrete_BuildingPermit | 27,811 | 110,524 | 3.97 |
| Housing_Terrain-Timber_BuildingPermit | 11,965 | 50,540 | 4.22 |
| Housing_Terrain-Concrete_BuildingPermit | 11,265 | 47,566 | 4.22 |
| Office_Terrain_Timber_BuildingPermit | 5,969 | 24,490 | 4.10 |
| Office_Terrain_Concrete_BuildingPermit | 5,721 | 23,438 | 4.10 |
| AA-SingleDwellingHouse (Revit 2026, flowbim.ee, RE-124) | 23,393 | 93,186 | 3.98 |
| 2024_Core_Interior | 4,101 | 17,854 | 4.35 |
| Revit_IFC5_Einhoven | 407 | 1,828 | 4.49 |

Falsifiable: a table that needs more values than it has bytes.

## Change

`native_parameters::decode_object_fields_with_limits` takes explicit
budgets (`decode_object_fields` calls it with the defaults).
`storage_increments` uses the default budget raised to the stream's byte
length: one value per byte, about four times what any file above uses.
Every other decode keeps the default.

## Measured

| file | bodies from saved meshes, main | after |
|---|---:|---:|
| Housing_Timber_BuildingPermit | 0 of 1,083 | 1,022 |
| Housing_Timber_As-Built | 0 of 1,087 | 1,026 |

Housing_Timber_BuildingPermit against Revit's own IFC export with
`tools/re/saved_meshes_vs_ifc.py` (IfcOpenShell 0.8.5), as in
`RE-78-saved-graphics-meshes.md`. Revit's export is turned -60.6317° by its
site placement; with that turn the residual offset is zero:

```text
python3 tools/re/saved_meshes_vs_ifc.py Housing_Timber_BuildingPermit.ifc     ARK_NordicLCA_Housing_Timber_BuildingPermit_Revit.ifc     --source saved_graphics_mesh --rotate-deg -60.631748527001605
```

| class | n | <=0.01 ft | <=0.1 | <=1 | >1 | no ref |
|---|---:|---:|---:|---:|---:|---:|
| IfcMember | 452 | 452 | 0 | 0 | 0 | 0 |
| IfcFurniture | 102 | 79 | 23 | 0 | 0 | 0 |
| IfcBuildingElementProxy | 95 | 52 | 0 | 0 | 0 | 43 |
| IfcPlate | 78 | 78 | 0 | 0 | 0 | 0 |
| IfcDoor | 67 | 12 | 4 | 51 | 0 | 0 |
| IfcWindow | 65 | 65 | 0 | 0 | 0 | 0 |
| IfcRailing | 44 | 23 | 17 | 4 | 0 | 0 |
| IfcSanitaryTerminal | 35 | 22 | 0 | 4 | 9 | 0 |
| IfcBeam | 28 | 28 | 0 | 0 | 0 | 0 |
| IfcSlab | 26 | 26 | 0 | 0 | 0 | 0 |
| IfcCovering | 14 | 14 | 0 | 0 | 0 | 0 |
| IfcStairFlight | 11 | 11 | 0 | 0 | 0 | 0 |
| IfcColumn | 4 | 4 | 0 | 0 | 0 | 0 |
| IfcRamp | 1 | 1 | 0 | 0 | 0 | 0 |
| total | 1,022 | 867 | 44 | 59 | 9 | 43 |

The 112 bodies outside 0.01 ft are saved meshes that the reader decodes only
in part (most doors: their frame Geometry is dropped by one rejected face);
those are separate gaps in `native_saved_mesh`, not this budget. Every other
file above exports the same IFC as before (header timestamp aside); Core
Interior and Einhoven give the same glb and diagnostics byte for byte, and
the CI witness verdicts for Core Interior (full and slim) pass and replay
(`match` for rvt-rs, IfcOpenShell and IFClite).

## Not claimed

- No test is registered: the change needs the BIM4LCA files, which are not
  among CI's corpora; on the corpora CI has, the output is unchanged.
- `creation_episodes` and the other preamble decodes keep the default budget
  and were not observed to hit it.

## Provenance

Byte observations on the BIM4LCA files and comparison with Revit's own IFC
export of the same files. No Autodesk or ODA source, leaked documentation or
decompiled code.
