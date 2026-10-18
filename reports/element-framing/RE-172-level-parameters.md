# RE-172 — Level parameters from the per-element association record

**Date:** 2026-10-13
**Artefacts:** Autodesk's public Revit 2024 sample files (`racadvancedsampleproject`, `racbasicsampleproject`, `rmeadvancedsampleproject`, `rmebasicsampleproject`, `rstadvancedsampleproject`, `rstbasicsampleproject`, `Technicalschoolcurrentm`) each with Revit's own IFC4 export, as the oracle.
**Clean-room provenance:** public Autodesk Revit 2024 sample files, Revit's own IFC4 export as the oracle, and exported symbol names only. No SDK source and no decompiled or disassembled code.
**Result:**
- **The record.** Every element has one record in the inflated `Partitions/*` streams (checksum pages stripped, every gzip member inflated) that opens `d3 02 01 00 00 00 · u32 ElementId · u32 0 · u32 LevelId · u32 0 · ff ff ff ff × 6`. It is variable length.
- **`LevelId`** (at ElementId + 8) is the element's Level: a wall's `Base Constraint`, and for every other category the `Level`, `Base Level`, `Reference Level` or `Schedule Level` parameter. Where it is not `0xffffffff` it equalled Revit's parameter on every element tested in 7 files. `0xffffffff` means the level is stored elsewhere (every beam, stairs, some MEP terminals, braces): rvt-rs writes nothing for them.
- **Wall `Top Constraint`** is a second `0 · LevelId · 0 · ff..` group 100 to 320 bytes after the ElementId (150, 152, 156, 164 or 168 in the files seen). It is read at the first offset in that window where the group holds a recovered Level's id and lies inside the record. It is exact where it is located and incomplete: walls whose group is not found get no `Top Constraint`.
- **Fail closed.** An id is written only when it is a Level recovered by `partition_level_records`. Revit 2024 only.
- **Where it is written.** A `Constraints` property set (Revit's name for it) on the element, with the names Revit uses: walls and curtain walls `Base Constraint` and `Top Constraint` (`Up to level: <name>`), columns `Base Level` and `Level`, railings, roofs and stairs `Base Level`, duct and pipe segments `Reference Level`, everything else `Level`. Values are `IfcLabel`. Storey containment, `LevelElementId` and the level binding of RE-27, RE-59, RE-60 and RE-68 are not touched.

Measured against Revit's IFC4 (export of this branch against upstream main `120e2f9`, matched by `Tag`; "equal" is of the elements where Revit has the property). Storey containment and element counts are identical to main on all seven files.

| file | Level | Base Level | Base Constraint | Top Constraint | Reference Level |
|---|---:|---:|---:|---:|---:|
| racadvanced | 5214 / 5214 | 190 / 190 (+1) | 180 / 180 | 177 / 177 | |
| Technicalschool | 5214 / 5214 | 190 / 190 (+6) | 180 / 180 | 177 / 177 | |
| rmeadvanced | 4151 / 4151 | 141 / 141 | 166 / 166 | 165 / 165 | 1216 / 1216 |
| rmebasic | 4196 / 4196 | 141 / 141 | 166 / 166 | 165 / 165 | 1215 / 1215 |
| racbasic | 359 / 359 | 4 / 4 | 56 / 56 | 48 / 48 | |
| rstadvanced | 502 / 502 | 203 / 203 | 6 / 6 | 6 / 6 | |
| rstbasic | 67 / 67 | 30 / 30 | 9 / 9 | 9 / 9 | |

(+n) is the number written where Revit's export has no such property.

**Limits.**
- Beams, stairs, braces and some MEP terminals have `0xffffffff` in the field: no value is written.
- `Top Constraint` coverage is partial (the offset of the group varies with optional blocks not decoded).
- The DACH and Snowdon Architectural samples export no elements on main (the layout of their ElementId is not located, RE-30), so nothing is written for them.
- Revit 2023 and 2025 files are not covered: the record was checked on 2024 only.

Check: `tests/level_parameters.rs` (set `RVT_REVIT2024_SAMPLES_DIR` to a folder holding `racadvancedsampleproject.rvt` and Revit's export of it).
