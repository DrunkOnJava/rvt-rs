# RE-81 — Revit 2023 element records

**Date:** 2026-09-27
**Issue:** #421
**Artefacts (local only, no licence):** `255ribeiro/intro_ifc` `Exemplo_data.rvt` (Revit 2023, build 20221122) and `modelo_bim.rvt` (Revit 2023, build 20220503), each with Revit's own IFC4 export.
**Probe:** `examples/probe_re81_2023_element_records.rs`

**Result:** positive for element identity. Every element Tag of both exports is found, and on `modelo_bim` nothing else is. Not yet wired into the exporter.

## The 2023 prologue

2023 records sit behind the marker RE-80 derives from the schema: `[Outline][0xFF x 4][ElementParents]`, which is `3c 01 ff ff ff ff 82 05` on 2023. Relative to the marker:

| field | 2023 | 2024 and 2025 |
|---|---:|---:|
| ElementId | `u32` at -52 | `u64` at -80 (record `+0x00`) |
| BuiltInCategory | `i64` at -38 | `i64` at -62 (`+0x12`) |
| container reference | `u64` at -30 | same (`+0x32`) |
| placement kind | `u32` at -14, `0xffffef7f` placed | same (`+0x42`) |
| bounding box | six `f64` after the marker | same |
| ElementHeader tag | `u16` at -44 (`0x0576` = 1398) | |

The tail of the prologue is 2024's; only the id and category sit elsewhere. The ElementId offset was found by matching every `u32` before each marker against the export's Tags: -52 holds 44 of the 45 distinct Tags on `Exemplo_data` and 37 of 37 on `modelo_bim`. The 45th is in a record at the very start of its partition, which a first scan missed. The category offset holds a `BuiltInCategory` on every record whose marker is preceded by one.

The first reference list is not a list of `u64` ElementIds at the 2024 position (its values read as packed `u32` pairs), so references are not decoded yet.

## The instance rule

RE-21's rule (placed, no container, a box with volume), applied to the 2023 fields:

| file | category | instances | Tags of Revit's export | not in it |
|---|---|---:|---:|---:|
| `modelo_bim` | walls | 4 | 4 | 0 |
| | doors | 1 | 1 | 0 |
| | windows | 10 | 10 | 0 |
| | floors, roofs, foundations | 1, 1, 4 | 1, 1, 4 | 0 |
| | structural columns, framing | 8, 8 | 8, 8 | 0 |
| `Exemplo_data` | walls | 17 | 17 | 0 |
| | doors, windows | 10, 9 | 10, 9 | 0 |
| | floors | 1 | 1 | 0 |
| | generic models | 22 | 8 | 14 |
| | rooms | 9 | (9 `IfcSpace`, which carry no Tag) | |

Revit export Tags not found: 0 on both files. The 8 generic models Revit exports are window trims and cuts nested in the windows (`M_Moldura-janela-interior-plana`, `M_Recorte-Janela-Exterior-Plana`); the 14 it leaves out are other nested pieces of the same windows, with 13 references where the exported ones have 11. RE-79's mutual-reference rule does not separate them in 2023, because the reference lists are not decoded.

## Not claimed

- The exporter still reads element records on 2024 and 2025 only; admitting 2023 needs the second prologue (RE-30), names and types, storeys and references on this layout (#421).
- Two files of one author; other 2023 files may differ.
