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
