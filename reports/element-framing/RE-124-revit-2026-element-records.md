# RE-124 — Revit 2026 element records: the 2024/2025 frame holds, with the schema's marker

**Date:** 2026-09-28
**Issues:** #421 (element records beyond 2024/2025); follows RE-32 (the per-release marker) and RE-80 (the marker is schema class tags)
**Artefact:** `AA-SingleDwellingHouse-RVT-CIADD.rvt` (Revit 2026, 66.7 MB, sha256 `eeff78f3d07dc23397d276f870f42240cb453cfdd39d965531a9d0ccc7d0a654`) and Revit's own IFC4X3 export of it (Revit 26.4.0.32, CoordinationView + QuantityTakeOffAddOnView, sha256 `c9e2986c0f349b3aee0973a15d598c9fc82955e0b4d75b4e8ff99808be767342`). Public downloads from flowbim.ee, pointed to by STE1200 (Discussion #112); no licence stated, so local only.
**Result:** positive on the record frame, not yet shipped. Revit 2026 records carry the marker RE-32 predicted, and the 2024/2025 decode with the RE-21 instance rule selects only elements Revit's export holds. Admitting 2026 into the export is held back by three measured problems, listed in section 3.

## 1. The marker

The file's own schema gives `Outline` tag `0x0161` and `ElementParents` tag `0x05f1`, so the marker is `61 01 ff ff ff ff f1 05`. RE-32 predicted the tail `0x05f1` (the ElemTable header constant 1481 plus 40) from 2024 and 2025. The `ElementHeader` tag is `0x5e5` (1509), as STE1200 reported. `partition_element_records::schema_bbox_marker` reads it (#506).

## 2. The instance rule against Revit's export

`examples/probe_re124_revit_2026_records.rs` scans every partition for the recovered categories with that marker and keeps placed instances (declared ElementId, no container, placement kind placed). `tools/re/instances_vs_ifc_tags.py` compares them with the export's element Tags (openings left out):

| | elements |
|---|---:|
| export elements with a numeric Tag | 115 |
| placed instances found (first prologue only) | 88 |
| of them in the export | **88** (0 not in it) |
| missed | 27: 24 walls, 2 slabs, 1 geographic element |

Every found category lands on Revit's class: 35 walls, 13 doors, 16 windows, 6 floors, 6 furniture, 6 plumbing fixtures (`IfcSanitaryTerminal`), 1 roof, and 5 equipment and generic models (`IfcBuildingElementProxy`).

With 2026 admitted to the record pipeline as a trial (the marker above added to `bbox_marker`), the record chain (RE-35) attributes the missed frames too: the export has all 59 walls and 8 slabs, 85 elements in all with a body, and none that is not in Revit's export.

## 3. What holds admission back

Measured in the same trial export:

- **Doors and windows are written as bare openings.** All 29 export as `IfcOpeningElement` with no host wall, not as `IfcDoor` and `IfcWindow`. The RE-84 check for a type that draws no geometry and the RE-85 host binding read 2024/2025 layouts.
- **Storeys are not the model's.** The export has four storeys (`-01 Plinth` −800 mm, `00 Ground` 0, `01 Floor` 400, `02 Roof` 3,400). rvt-rs writes `Ground floor`, `Level 1` and `Roof` at 0, from partition strings, because Level records are gated to 2023 to 2025. Released 0.4.0 does the same on this file: that fallback is a separate defect (issue to follow).
- **Rooms are unnamed.** All 13 rooms are found, but their numbers and names (RE-117) are not read on 2026.

Family and type names and type parameters (Type Mark) do read on 2026.

## 4. Reproduce

```bash
cargo run --profile ci --example probe_re124_revit_2026_records -- AA-SingleDwellingHouse-RVT-CIADD.rvt > ids.txt
python3 tools/re/instances_vs_ifc_tags.py ids.txt AA-SingleDwellingHouse-RVT-CIADD.ifc
```

## 5. Addendum: admitted (2026-09-28)

The three blockers of section 3 are resolved, and Revit 2026 is admitted as experimental.

**Every per-release constant is a schema class tag.** Read from each file's own `Formats/Latest`, the constants rvt-rs held per release are:

| constant | class | 2023 | 2024 | 2025 | 2026 |
|---|---|---|---|---|---|
| record marker head, tail | `Outline`, `ElementParents` | 0x013c, 0x0582 | 0x0146, 0x05ab | 0x0159, 0x05d3 | 0x0161, 0x05f1 |
| Level elevation marker | `Plane` | 0x0235 | 0x0248 | 0x025d | 0x0265 |
| element-data header | `CellList` | 0x02c0 | 0x02d3 | 0x02ef | 0x02fd |
| layer frame | `VerticalRegionsStructure` | 0x106f | 0x10a6 | 0x110e | 0x1165 |
| material object | `Material` | 0x09fb | 0x0a28 | 0x0a6b | 0x0a9c |
| material name frame | `PatternHelper` | 0x013f | 0x0149 | 0x015c | 0x0164 |
| material name end | `PhysicalParamSet` | 0x0beb | 0x0c17 | 0x0c6b | 0x0cac |

A 2026 layer record is 41 bytes: the 2025 fields at their offsets, then a `u32` repeating the function.

**Storeys and rooms.** The 2026 elevation marker and element-data header give the house's 4 Levels with Revit's names and elevations, and its 13 rooms with Revit's number and name (RE-117).

**Doors and windows.** RE-84 exported them as bare openings for two reasons.
- Their types' material maps hold unset entries (`ff`×8), which RE-82's map reader refused.
- A file whose material names were not read made every map unreadable.

A map with unset entries now proves the type draws geometry, but its materials are not taken as the element's. Taken as its set, it gave 22 more right and 46 more wrong sets on Snowdon Towers. A file without material names decides nothing. Every 2023 to 2025 file is byte-identical.

**Measured against Revit's export** (`tools/re/instances_vs_ifc_tags.py` for the ElementIds, IfcOpenShell for the rest):

| | result |
|---|---|
| elements | 113 of 115, none outside it; missed 1 window, 1 geographic element |
| GlobalIds | 113 of 113 Revit's |
| storeys | 4 of 4, Revit's names and elevations |
| elements in Revit's storey | 97; the other 16 are in their room's `IfcSpace` in Revit's export, whose storey is theirs |
| rooms | 13 of 13 Revit's number and name |
| doors, windows | 13 of 13 and 15 of 16 as `IfcDoor`/`IfcWindow` in their host |
| wall names | `Family:Type:ElementId` as Revit's (type names from the element data, family from the layers) |
| layer sets | 26, named and as thick as Revit's to 0.001 ft; layer materials mostly unnamed (32 of 148 material objects unnamed) |
| element materials | 2 sets Revit's, 26 incomplete (unnamed layers), 4 with a material Revit does not give (3 windows Revit gives none, 1 nested glass part) |
| wall bodies (oriented) | thickness 59 of 59 within 1 mm, length 35 of 59, height 37 of 59 |

The 2026 family file, which has no elements, now lists its 24 materials as the 2025 family does. Not read on 2026: wall join lists, roof slopes, stairs, beam axes, IFC export overrides and the opening index.
